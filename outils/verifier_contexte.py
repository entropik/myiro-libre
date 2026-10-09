"""Contrôle local du worktree, de la branche et des consignes d'isolement."""
import argparse
from pathlib import Path
import subprocess
import sys


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True,
                            text=True, encoding="utf-8", errors="replace", shell=False)
    if result.returncode:
        raise ValueError("Lecture git impossible : vérifier le compte d'exécution et le dépôt, sans modifier globalement safe.directory.")
    return result.stdout.strip()


def isolation(text):
    title = "## Isolement des agents"
    if title not in text:
        raise ValueError("Section d'isolement absente.")
    return text.split(title, 1)[1].split("\n## ", 1)[0].strip().replace("\r\n", "\n")


def verify(root, expected_path, expected_branch, reference="dev", read_git=git):
    actual = Path(read_git(root, "rev-parse", "--show-toplevel")).resolve()
    if actual != Path(expected_path).resolve():
        raise ValueError("Mauvais worktree : arrêter avant toute écriture.")
    branch = read_git(root, "branch", "--show-current")
    if branch != expected_branch:
        raise ValueError("Mauvaise branche : arrêter avant toute écriture.")
    upstream = read_git(root, "show", f"{reference}:AGENTS.md")
    local = (actual / "AGENTS.md").read_text(encoding="utf-8-sig")
    if isolation(local) != isolation(upstream):
        raise ValueError("Les consignes d'isolement divergent de la référence : les comparer avant toute écriture.")
    current = read_git(root, "rev-parse", "--short", "HEAD")
    public = read_git(root, "rev-parse", "--short", reference)
    return {"worktree": str(actual), "branch": branch, "commit": current,
            "reference": reference, "reference_commit": public,
            "different": current != public}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree", type=Path, required=True)
    parser.add_argument("--branch", required=True)
    parser.add_argument("--reference", default="dev")
    args = parser.parse_args()
    try:
        result = verify(Path.cwd(), args.worktree, args.branch, args.reference)
        print(f"Worktree vérifié : {result['worktree']} ({result['branch']} @ {result['commit']}).")
        print(f"Référence : {result['reference']} @ {result['reference_commit']}.")
        if result["different"]:
            print("Versions différentes : préciser la version examinée. Aucune synchronisation automatique.")
        return 0
    except (ValueError, OSError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
