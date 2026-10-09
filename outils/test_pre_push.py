"""Le hook protège les archives même après retrait des fichiers privés."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

HOOK = Path(__file__).resolve().parents[1] / ".githooks" / "pre-push"


class PublicationBranches(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.env = dict(os.environ, GIT_AUTHOR_NAME="Test", GIT_AUTHOR_EMAIL="test@example.invalid",
                        GIT_COMMITTER_NAME="Test", GIT_COMMITTER_EMAIL="test@example.invalid")
        self.git("init", "--quiet")
        self.empty_tree = self.git("mktree", input="")
        self.public = self.git("commit-tree", self.empty_tree, input="Public\n")
        blob = self.git("hash-object", "-w", "--stdin", input="Donnée synthétique\n")
        subtree = self.git("mktree", input=f"100644 blob {blob}\tfixture.txt\n")
        private_tree = self.git("mktree", input=f"040000 tree {subtree}\tAudit-ColorGPS\n")
        self.private = self.git("commit-tree", private_tree, "-p", self.public, input="Privé\n")
        self.cleaned = self.git("commit-tree", self.empty_tree, "-p", self.private, input="Retrait\n")
        self.shell = shutil.which("sh")
        if not self.shell and os.name == "nt":
            git_exe = Path(shutil.which("git")).resolve()
            candidate = git_exe.parent.parent / "bin" / "sh.exe"
            if candidate.is_file():
                self.shell = str(candidate)
        if not self.shell:
            self.fail("Shell Git requis pour tester le hook de publication.")

    def git(self, *args, input=None):
        result = subprocess.run(["git", *args], cwd=self.root, env=self.env,
                                input=input.encode("utf-8") if input is not None else None,
                                capture_output=True, check=True)
        return result.stdout.decode("utf-8").strip()

    def push(self, commit, source="ticket/test", target="dev"):
        line = f"refs/heads/{source} {commit} refs/heads/{target} {'0' * 40}\n"
        return subprocess.run([self.shell, str(HOOK)], cwd=self.root, env=self.env,
                              input=line, text=True, encoding="utf-8", capture_output=True).returncode

    def test_public_branches_allowed(self):
        for branch in ("dev", "main", "ticket/test"):
            with self.subTest(branch=branch):
                self.assertEqual(0, self.push(self.public, target=branch))

    def test_archived_source_refused_even_if_tree_is_public(self):
        self.assertNotEqual(0, self.push(self.public, source="archive/codex-initial"))

    def test_private_history_refused_without_private_ref(self):
        self.assertNotEqual(0, self.push(self.cleaned))

    def test_private_ancestor_refused(self):
        self.git("update-ref", "refs/heads/prive", self.public)
        self.assertNotEqual(0, self.push(self.public))

    def test_other_remote_branch_and_deletion_refused(self):
        self.assertNotEqual(0, self.push(self.public, target="archive/test"))
        self.assertNotEqual(0, self.push("0" * 40))


if __name__ == "__main__":
    unittest.main()
