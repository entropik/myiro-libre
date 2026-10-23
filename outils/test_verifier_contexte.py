import pathlib
import tempfile
import unittest
from verifier_contexte import verify


class Context(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)
        self.instructions = "## Isolement des agents\nRègle courante.\n\n## Archive\nArchives.\n"
        (self.root / "AGENTS.md").write_text(self.instructions, encoding="utf-8")
        self.branch = "dev"

    def git(self, root, *args):
        return {("rev-parse", "--show-toplevel"): str(self.root),
                ("branch", "--show-current"): self.branch,
                ("show", "dev:AGENTS.md"): self.instructions,
                ("rev-parse", "--short", "HEAD"): "abc",
                ("rev-parse", "--short", "dev"): "def"}[args]

    def test_correct_worktree_reports_drift_without_mutation(self):
        self.assertTrue(verify(self.root, self.root, "dev", read_git=self.git)["different"])

    def test_wrong_worktree_stops(self):
        with self.assertRaisesRegex(ValueError, "worktree"):
            verify(self.root, self.root / "autre", "dev", read_git=self.git)

    def test_wrong_branch_stops(self):
        self.branch = "main"
        with self.assertRaisesRegex(ValueError, "branche"):
            verify(self.root, self.root, "dev", read_git=self.git)

    def test_stale_isolation_stops(self):
        (self.root / "AGENTS.md").write_text("## Isolement des agents\nAncienne consigne de fusion.\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "divergent"):
            verify(self.root, self.root, "dev", read_git=self.git)


if __name__ == "__main__":
    unittest.main()
