#!/usr/bin/env python3
"""Vérifications automatiques de la documentation et des fichiers versionnés.

Sans dépendance externe (Python 3 seul). Usage :
    python outils/verifier_docs.py              vérifie tous les fichiers suivis par git
    python outils/verifier_docs.py --staged     vérifie seulement ce qui va être commité
    python outils/verifier_docs.py --message F  vérifie le message de commit contenu dans le fichier F
    python outils/verifier_docs.py --installer  installe les hooks git locaux (pre-commit, commit-msg)

Contrôles :
  1. mots interdits : la liste est dans `.mots-interdits.local` (un mot par ligne, ignorée par git,
     donc le mot lui-même n'apparaît jamais dans un fichier versionné) ; sans cette liste (en CI),
     le contrôle est sauté, annoncé, et les autres contrôles continuent ;
  2. liens relatifs cassés dans les fichiers Markdown ;
  3. rappel (non bloquant) si le journal du jour manque dans docs/blog/.
Code de sortie 1 si un contrôle bloquant échoue.
"""
import datetime
import pathlib
import re
import subprocess
import sys

RACINE = pathlib.Path(__file__).resolve().parent.parent
MOTS = RACINE / ".mots-interdits.local"
TEXTE = {".md", ".txt", ".rs", ".toml", ".py", ".html", ".css", ".json", ".yml", ".yaml", ".csv", ".tsv", ".sh", ".lock"}
IGNORES = {"outils/verifier_docs.py"}


def git(*args):
    r = subprocess.run(["git", *args], cwd=RACINE, capture_output=True, text=True, encoding="utf-8")
    return r.stdout.splitlines()


def fichiers(staged):
    if staged:
        noms = git("diff", "--cached", "--name-only", "--diff-filter=ACM")
    else:  # fichiers suivis, plus fichiers neufs pas encore ajoutés (hors fichiers ignorés)
        noms = git("ls-files") + git("ls-files", "--others", "--exclude-standard")
    return [RACINE / n for n in noms if n not in IGNORES]


def mots_interdits():
    if not MOTS.exists():
        return []
    return [m.strip().lower() for m in MOTS.read_text(encoding="utf-8").splitlines()
            if m.strip() and not m.startswith("#")]


def contenu(chemin, staged):
    """Texte à contrôler : version indexée si --staged, sinon version du disque."""
    rel = chemin.relative_to(RACINE).as_posix()
    if staged:
        r = subprocess.run(["git", "show", f":{rel}"], cwd=RACINE, capture_output=True)
        return r.stdout.decode("utf-8", errors="replace")
    try:
        return chemin.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def controle_mots(liste, staged):
    erreurs = []
    mots = mots_interdits()
    if not mots:
        return erreurs
    for f in liste:
        if f.suffix.lower() not in TEXTE and f.name not in {".gitignore", "LICENSE"}:
            continue
        rel = f.relative_to(RACINE).as_posix()
        for mot in mots:  # le chemin lui-même compte
            if mot in rel.lower():
                erreurs.append(f"{rel} : mot interdit dans le nom du fichier")
        for i, ligne in enumerate(contenu(f, staged).splitlines(), 1):
            bas = ligne.lower()
            for mot in mots:
                if mot in bas:
                    erreurs.append(f"{rel}:{i} : mot interdit")
    return erreurs


LIEN = re.compile(r"\]\(([^)\s#]+)(?:#[^)]*)?\)")


def controle_liens(liste):
    erreurs = []
    for f in liste:
        if f.suffix.lower() != ".md":
            continue
        for i, ligne in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            for cible in LIEN.findall(ligne):
                if re.match(r"^[a-z]+:", cible) or cible.startswith("/"):
                    continue
                if not (f.parent / cible).resolve().exists():
                    erreurs.append(f"{f.relative_to(RACINE).as_posix()}:{i} : lien cassé vers {cible}")
    return erreurs


def controle_message(chemin):
    mots = mots_interdits()
    texte = pathlib.Path(chemin).read_text(encoding="utf-8", errors="replace").lower()
    return [f"message de commit : mot interdit « {m} »" for m in mots if m in texte]


def installer():
    hooks = RACINE / ".git" / "hooks"
    cmd = 'python "$(git rev-parse --show-toplevel)/outils/verifier_docs.py"'
    (hooks / "pre-commit").write_text(f"#!/bin/sh\nexec {cmd} --staged\n", encoding="utf-8", newline="\n")
    (hooks / "commit-msg").write_text(f'#!/bin/sh\nexec {cmd} --message "$1"\n', encoding="utf-8", newline="\n")
    print("Hooks installés : .git/hooks/pre-commit et commit-msg (le hook pre-push existant n'est pas touché).")


def main(argv):
    for flux in (sys.stdout, sys.stderr):
        flux.reconfigure(encoding="utf-8", errors="replace")
    if "--installer" in argv:
        return installer() or 0
    if "--message" in argv:
        erreurs = controle_message(argv[argv.index("--message") + 1])
    else:
        staged = "--staged" in argv
        liste = fichiers(staged)
        if not MOTS.exists():  # cas de la CI : la liste reste sur le poste
            print("Liste locale de mots interdits absente (.mots-interdits.local) : contrôle des mots sauté.")
        erreurs = controle_mots(liste, staged) + controle_liens(liste)
        journal = RACINE / "docs" / "blog" / f"{datetime.date.today().isoformat()}.md"
        if not journal.exists() and "--staged" not in argv:
            print(f"Rappel : pas de journal du jour ({journal.relative_to(RACINE).as_posix()}). Commande : /fin-de-journee")
    for e in erreurs:
        print("ERREUR", e)
    if erreurs:
        print(f"{len(erreurs)} problème(s) bloquant(s).")
        return 1
    print("Vérifications documentaires : OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
