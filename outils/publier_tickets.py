"""Publication GitHub reprenable, sans shell ni nouvelle tentative automatique.

Usage : python outils/publier_tickets.py MANIFESTE [--apply] [--native]
Sans --apply : validation locale uniquement. Les fichiers et l'état vivent
près du manifeste, dans un dossier durable ignoré par git.
"""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


class StopPublication(RuntimeError):
    pass


class GhFailure(StopPublication):
    def __init__(self, kind, status=None, request_id=None):
        self.kind, self.status, self.request_id = kind, status, request_id
        super().__init__(f"GitHub : {kind}; HTTP={status}; requête={request_id}. Arrêt sans nouvel essai.")


def decode_reply(result):
    # Ne jamais afficher stderr brut : il peut contenir des données privées.
    output = result.stdout.replace("\r\n", "\n")
    status, request_id = None, None
    if output.startswith("HTTP/"):
        header, _, output = output.partition("\n\n")
        status = int(header.splitlines()[0].split()[1])
        for line in header.splitlines()[1:]:
            if line.lower().startswith("x-github-request-id:"):
                request_id = line.split(":", 1)[1].strip()
    if result.returncode or (status and status >= 400):
        error = result.stderr.lower()
        kind = ("serveur" if status and status >= 500 else
                "droits" if status in (401, 403) else
                "requête" if status and status >= 400 else
                "réseau" if any(s in error for s in ("connectex", "dial tcp", "timeout", "resolve")) else
                "réponse indéterminée")
        raise GhFailure(kind, status, request_id)
    try:
        return json.loads(output) if output.strip() else None
    except ValueError as error:
        raise GhFailure("réponse JSON invalide", status, request_id) from error


class GitHub:
    def __init__(self, repo, run=subprocess.run):
        self.root, self.run = f"repos/{repo}/issues", run

    def request(self, method, path, data=None):
        args = ["gh", "api", "--include", "--method", method, path,
                "-H", "X-GitHub-Api-Version: 2022-11-28"]
        payload = None
        if data is not None:
            args += ["--input", "-"]
            payload = json.dumps(data, ensure_ascii=False)
        try:
            result = self.run(args, input=payload, capture_output=True, text=True,
                              encoding="utf-8", errors="replace", shell=False, timeout=45)
        except subprocess.TimeoutExpired as error:
            raise GhFailure("délai expiré, résultat incertain") from error
        except OSError as error:
            raise GhFailure("exécution locale impossible") from error
        return decode_reply(result)

    def view(self, number):
        return self.request("GET", f"{self.root}/{number}")

    def create(self, title, body):
        return self.request("POST", self.root,
                            {"title": title, "body": body, "labels": ["ready-for-agent"]})

    def edit(self, number, body):
        return self.request("PATCH", f"{self.root}/{number}", {"body": body})

    def find(self, marker):
        # Lecture directe, sans dépendre de l'index de recherche GitHub.
        matches = []
        for page in range(1, 101):
            rows = self.request("GET", f"{self.root}?state=all&per_page=100&page={page}")
            matches += [r for r in rows if marker in (r.get("body") or "") and "pull_request" not in r]
            if len(rows) < 100:
                return matches
        raise StopPublication("Lecture incomplète : réconciliation manuelle nécessaire.")

    def relation(self, owner, child, kind):
        suffix = "sub_issues" if kind == "parent" else "dependencies/blocked_by"
        path = f"{self.root}/{owner}/{suffix}"
        current = self.request("GET", path)
        if any(row["number"] == child for row in current):
            return
        child_id = self.view(child)["id"]
        field = "sub_issue_id" if kind == "parent" else "issue_id"
        try:
            self.request("POST", path, {field: child_id})
        except GhFailure:
            # Une seule lecture pour déterminer si une écriture a été appliquée.
            self.request("GET", path)
            raise
        if not any(row["number"] == child for row in self.request("GET", path)):
            raise StopPublication("Relation non confirmée par GitHub.")

    def remove_blocker(self, owner, child):
        path = f"{self.root}/{owner}/dependencies/blocked_by"
        rows = self.request("GET", path)
        target = next((row for row in rows if row["number"] == child), None)
        if target is None:
            return
        try:
            self.request("DELETE", f"{path}/{target['id']}")
        except GhFailure:
            self.request("GET", path)
            raise
        if any(row["number"] == child for row in self.request("GET", path)):
            raise StopPublication("Retrait de dépendance non confirmé par GitHub.")


def atomic_json(path, value):
    temp = path.with_suffix(path.suffix + ".new")
    temp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(temp, path)


def validate(plan, folder):
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", plan.get("repo", "")):
        raise StopPublication("Dépôt invalide.")
    if not re.fullmatch(r"[\w-]+", plan.get("id", "")):
        raise StopPublication("Identifiant de publication invalide.")
    parent = plan.get("parent")
    if parent is not None and (type(parent) is not int or parent <= 0):
        raise StopPublication("Parent invalide.")
    tickets = plan.get("tickets", [])
    keys = [t["key"] for t in tickets]
    if not keys or len(keys) != len(set(keys)):
        raise StopPublication("Tickets absents ou identifiants répétés.")
    numbers = [t["number"] for t in tickets if "number" in t]
    if any(type(n) is not int or n <= 0 or n == parent for n in numbers) or len(numbers) != len(set(numbers)):
        raise StopPublication("Numéro importé invalide, répété ou identique au parent.")
    pending = dict(zip(keys, tickets))
    ordered = []
    for t in tickets:
        if not re.fullmatch(r"[\w-]+", t["key"]) or not t.get("title", "").strip():
            raise StopPublication("Identifiant ou titre vide/invalide.")
        path = (folder / t["body"]).resolve()
        if not path.is_relative_to(folder.resolve()) or not path.is_file():
            raise StopPublication("Corps absent ou extérieur au dossier durable.")
        if t.get("replace_body"):
            if "number" not in t or not re.fullmatch(r"[a-f0-9]{64}", t.get("expected_body_sha256", "")):
                raise StopPublication("Remplacement : numéro et empreinte du corps initial requis.")
        removals = t.get("remove_blocked_by", [])
        if removals and "number" not in t:
            raise StopPublication("Retrait réservé à un ticket existant.")
        resolved = {next(v["number"] for v in tickets if v["key"] == d and "number" in v)
                    if type(d) is str and any(v["key"] == d and "number" in v for v in tickets)
                    else d for d in t.get("blocked_by", [])}
        if any(type(d) is not int or d <= 0 or d == t.get("number") or d in resolved for d in removals):
            raise StopPublication("Retrait de dépendance invalide ou encore requise.")
        for dep in t.get("blocked_by", []):
            if type(dep) is int and dep in numbers:
                raise StopPublication("Utiliser la clé pour une dépendance interne au lot, pas son numéro.")
            if not ((type(dep) is int and dep > 0 and dep != t.get("number")) or
                    (type(dep) is str and dep in pending and dep != t["key"])):
                raise StopPublication("Dépendance absente ou invalide.")
    while pending:
        ready = [k for k, t in pending.items()
                 if all(type(d) is int or d not in pending for d in t.get("blocked_by", []))]
        if not ready:
            raise StopPublication("Cycle de dépendances.")
        for k in ready:
            ordered.append(pending.pop(k))
    return ordered


def marker(plan, ticket):
    return f"<!-- publication:{plan['id']}:{ticket['key']} -->"


def dependency_block(plan, ticket, records):
    deps = [d if type(d) is int else records[d]["number"] for d in ticket.get("blocked_by", [])]
    lines = [marker(plan, ticket), "## Parent", f"Spécification : #{plan['parent']}." ] if plan.get("parent") else [marker(plan, ticket)]
    lines += ["## Blocked by", "\n".join(f"- #{n}" for n in deps) if deps else "Aucun : peut commencer immédiatement.",
              "<!-- fin-publication -->"]
    return "\n\n".join(lines)


def render_body(original, block):
    text = re.sub(r"<!-- publication:[^>]+ -->.*?<!-- fin-publication -->", "", original, flags=re.S)
    # Remplace uniquement les sections de liens ; préserve tous les autres contenus.
    text = re.sub(r"(?m)^## (?:Parent|Blocked by)\s*\n.*?(?=^## |\Z)", "", text, flags=re.S)
    return text.rstrip() + "\n\n" + block + "\n"


class Publisher:
    def __init__(self, manifest, api):
        self.path, self.api = Path(manifest), api
        self.folder = self.path.parent
        self.plan = json.loads(self.path.read_text(encoding="utf-8-sig"))
        self.ordered = validate(self.plan, self.folder)
        self.state_path = self.folder / "etat.json"
        self.state = json.loads(self.state_path.read_text(encoding="utf-8")) if self.state_path.exists() else {"records": {}}
        identity = {"repo": self.plan["repo"], "id": self.plan["id"]}
        if self.state.get("identity", identity) != identity:
            raise StopPublication("État appartenant à une autre publication.")
        self.state["identity"] = identity
        for t in self.ordered:
            r = self.state["records"].setdefault(t["key"], {})
            if "number" in t:
                if r.get("number", t["number"]) != t["number"]:
                    raise StopPublication("Numéro importé différent de l’état enregistré.")
                r["number"] = t["number"]

    def save(self):
        atomic_json(self.state_path, self.state)

    def publish(self, native=False):
        self.state["native_verified"] = False
        records = self.state["records"]
        try:
            for t in self.ordered:
                r = records[t["key"]]
                block = dependency_block(self.plan, t, records)
                current = None
                if "number" not in r:
                    if r.get("pending_create"):
                        matches = self.api.find(marker(self.plan, t))
                        if len(matches) != 1:
                            raise StopPublication("Création incertaine : aucun nouvel essai. Réconcilier le numéro dans le manifeste.")
                        r["number"] = matches[0]["number"]
                        self.save()
                    else:
                        body = render_body((self.folder / t["body"]).read_text(encoding="utf-8-sig"), block)
                        r["pending_create"] = True
                        self.save()  # Avant l'effet externe, y compris si le processus est interrompu.
                        try:
                            created = self.api.create(t["title"], body)
                        except GhFailure:
                            matches = self.api.find(marker(self.plan, t))
                            if len(matches) == 1:
                                r["number"] = matches[0]["number"]
                                self.save()
                            raise
                        r["number"] = created["number"]
                        self.save()
                current = self.api.view(r["number"])
                if current["title"] != t["title"]:
                    raise StopPublication(f"Titre modifié sur #{r['number']} : revoir avant écriture.")
                source = ((self.folder / t["body"]).read_text(encoding="utf-8-sig")
                          if t.get("replace_body") else current.get("body") or "")
                body = render_body(source, block)
                if t.get("replace_body") and body != (current.get("body") or ""):
                    digest = hashlib.sha256((current.get("body") or "").encode()).hexdigest()
                    if digest != t["expected_body_sha256"]:
                        raise StopPublication(f"Corps modifié sur #{r['number']} : réconciliation requise.")
                if body != (current.get("body") or ""):
                    r["pending_edit"] = True
                    self.save()
                    try:
                        self.api.edit(r["number"], body)
                    except GhFailure:
                        confirmed = self.api.view(r["number"])
                        r["body_verified"] = confirmed.get("body") == body
                        self.save()
                        raise
                    current = self.api.view(r["number"])
                if current.get("body") != body:
                    raise StopPublication(f"Corps de #{r['number']} non confirmé.")
                if "ready-for-agent" not in [v["name"] for v in current.get("labels", [])]:
                    raise StopPublication(f"Étiquette ready-for-agent absente sur #{r['number']}.")
                r.update(body_verified=True, pending_create=False, pending_edit=False,
                         body_sha256=hashlib.sha256(body.encode()).hexdigest())
                self.save()
                print(f"#{r['number']} : texte et dépendances vérifiés.")
            if native:
                for t in self.ordered:
                    number = records[t["key"]]["number"]
                    if self.plan.get("parent"):
                        self.api.relation(self.plan["parent"], number, "parent")
                    for dep in t.get("blocked_by", []):
                        other = dep if type(dep) is int else records[dep]["number"]
                        self.api.relation(number, other, "blocker")
                    for other in t.get("remove_blocked_by", []):
                        r = records[t["key"]]
                        r["pending_remove_blocker"] = other
                        self.save()
                        self.api.remove_blocker(number, other)
                        r.pop("pending_remove_blocker", None)
                        r.setdefault("removed_blockers", [])
                        if other not in r["removed_blockers"]:
                            r["removed_blockers"].append(other)
                        self.save()
                self.state["native_verified"] = True
            self.state.pop("last_error", None)
            self.save()
        except StopPublication as error:
            self.state["last_error"] = str(error)
            self.save()
            raise


@contextlib.contextmanager
def publication_lock(folder):
    lock = folder / "publication.lock"
    try:
        fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
    except FileExistsError as error:
        raise StopPublication("Publication verrouillée ; vérifier le processus avant de retirer le verrou.") from error
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write(str(os.getpid()))
        yield
    finally:
        lock.unlink()


def durable_folder(folder):
    if folder.resolve().is_relative_to(Path(tempfile.gettempdir()).resolve()):
        raise StopPublication("Déplacer le manifeste hors du dossier temporaire avant publication.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--native", action="store_true")
    args = parser.parse_args()
    try:
        plan = json.loads(args.manifest.read_text(encoding="utf-8-sig"))
        ordered = validate(plan, args.manifest.parent)
        print("Ordre validé : " + ", ".join(t["key"] for t in ordered))
        if args.native and not args.apply:
            raise StopPublication("--native exige --apply.")
        if args.apply:
            durable_folder(args.manifest.parent)
            with publication_lock(args.manifest.parent):
                Publisher(args.manifest, GitHub(plan["repo"])).publish(args.native)
        return 0
    except (StopPublication, OSError, ValueError, KeyError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
