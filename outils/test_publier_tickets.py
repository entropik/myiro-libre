"""Scénarios de publication entièrement simulés : aucun réseau ni instrument."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from publier_tickets import (GitHub, GhFailure, Publisher, StopPublication,
                             decode_reply, durable_folder, publication_lock, validate)


class FakeGitHub:
    def __init__(self):
        self.issues, self.writes, self.relations = {}, [], set()
        self.failure = None

    def view(self, n):
        return copy.deepcopy(self.issues[n])

    def create(self, title, body):
        n = len(self.issues) + 101
        self.issues[n] = {"number": n, "title": title, "body": body,
                          "labels": [{"name": "ready-for-agent"}]}
        self.writes.append(("create", n))
        if self.failure == "create_saved":
            raise GhFailure("serveur", 500)
        return self.view(n)

    def edit(self, n, body):
        self.writes.append(("edit", n))
        if self.failure == "edit":
            raise GhFailure("serveur", 500)
        self.issues[n]["body"] = body
        return self.view(n)

    def find(self, marker):
        return [self.view(n) for n, v in self.issues.items() if marker in v["body"]]

    def relation(self, parent, child, kind):
        if self.failure == "relation":
            raise GhFailure("serveur", 500)
        self.relations.add((parent, child, kind))

    def remove_blocker(self, owner, child):
        self.writes.append(("remove", owner, child))
        if self.failure == "remove":
            raise GhFailure("serveur", 500)
        self.relations.discard((owner, child, "blocker"))


class Publication(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.folder = Path(self.tmp.name)
        self.plan = {"repo": "owner/repo", "id": "test", "parent": 21,
                     "tickets": [{"key": "a", "title": "L’état d’un spectre : été", "body": "a.md"},
                                 {"key": "b", "title": "Fermer", "body": "b.md", "blocked_by": ["a", 3]}]}
        for t in self.plan["tickets"]:
            (self.folder / t["body"]).write_text("## Critères\n\n- [ ] Mesure valide.\n", encoding="utf-8")
        self.manifest = self.folder / "manifest.json"
        self.api = FakeGitHub()
        self.save_plan()

    def save_plan(self):
        self.manifest.write_text(json.dumps(self.plan, ensure_ascii=False), encoding="utf-8")

    def publisher(self):
        return Publisher(self.manifest, self.api)

    def test_dependencies_present_at_creation_and_resume_without_writes(self):
        self.publisher().publish(True)
        self.assertIn("- #101\n- #3", self.api.issues[102]["body"])
        self.assertIn("#21", self.api.issues[101]["body"])
        self.assertEqual(self.api.issues[101]["title"], "L’état d’un spectre : été")
        writes = list(self.api.writes)
        self.publisher().publish(True)
        self.assertEqual(writes, self.api.writes)
        self.assertIn((102, 101, "blocker"), self.api.relations)

    def test_parent_failure_keeps_complete_text_and_resume_no_duplicate(self):
        self.api.failure = "relation"
        with self.assertRaises(GhFailure):
            self.publisher().publish(True)
        self.assertIn("- #101", self.api.issues[102]["body"])
        self.api.failure = None
        self.publisher().publish(True)
        self.assertEqual(2, sum(w[0] == "create" for w in self.api.writes))

    def test_ambiguous_create_reconciles_without_creating_again(self):
        self.api.failure = "create_saved"
        with self.assertRaises(GhFailure):
            self.publisher().publish()
        self.api.failure = None
        self.publisher().publish()
        self.assertEqual(2, len(self.api.issues))
        self.assertEqual(2, sum(w[0] == "create" for w in self.api.writes))

    def test_pending_create_absent_stops_without_retry(self):
        p = self.publisher()
        p.state["records"]["a"]["pending_create"] = True
        p.save()
        with self.assertRaisesRegex(StopPublication, "incertaine"):
            self.publisher().publish()
        self.assertEqual([], self.api.writes)

    def test_pending_create_multiple_matches_stops(self):
        self.publisher().publish()
        p = self.publisher()
        p.state["records"]["a"].pop("number")
        p.state["records"]["a"]["pending_create"] = True
        p.save()
        self.api.issues[999] = copy.deepcopy(self.api.issues[101])
        self.api.issues[999]["number"] = 999
        with self.assertRaisesRegex(StopPublication, "incertaine"):
            self.publisher().publish()

    def test_edit_server_failure_stops_before_other_mutations(self):
        self.publisher().publish()
        self.api.issues[101]["body"] = "Travail ajouté par une autre personne.\n"
        self.api.failure = "edit"
        before = len(self.api.writes)
        with self.assertRaises(GhFailure):
            self.publisher().publish()
        self.assertEqual(1, len(self.api.writes) - before)
        self.assertEqual("Travail ajouté par une autre personne.\n", self.api.issues[101]["body"])

    def test_import_preserves_current_body(self):
        self.api.issues[23] = {"number": 23, "title": self.plan["tickets"][0]["title"],
                               "body": "Contenu actualisé.\n", "labels": [{"name": "ready-for-agent"}]}
        self.plan["tickets"][0]["number"] = 23
        self.save_plan()
        self.publisher().publish()
        self.assertIn("Contenu actualisé.", self.api.issues[23]["body"])

    def test_cycle_rejected_before_network(self):
        self.plan["tickets"][0]["blocked_by"] = ["b"]
        with self.assertRaisesRegex(StopPublication, "Cycle"):
            validate(self.plan, self.folder)

    def prepare_replacement(self):
        self.publisher().publish(True)
        for t, n in zip(self.plan["tickets"], (101, 102)):
            t["number"] = n
            t["replace_body"] = True
            t["expected_body_sha256"] = hashlib.sha256(self.api.issues[n]["body"].encode()).hexdigest()
            (self.folder / t["body"]).write_text("## Critères\n\nNouveau cadrage validé.\n", encoding="utf-8")
        self.save_plan()

    def test_explicit_replacement_is_verified_and_idempotent(self):
        self.prepare_replacement()
        self.publisher().publish(True)
        self.assertIn("Nouveau cadrage validé.", self.api.issues[101]["body"])
        before = list(self.api.writes)
        self.publisher().publish(True)
        self.assertEqual(before, self.api.writes)

    def test_replacement_refuses_concurrent_body_change(self):
        self.prepare_replacement()
        self.api.issues[101]["body"] += "Autre contribution.\n"
        before = list(self.api.writes)
        with self.assertRaisesRegex(StopPublication, "réconciliation"):
            self.publisher().publish(True)
        self.assertEqual(before, self.api.writes)

    def test_replacement_requires_original_hash(self):
        self.plan["tickets"][0].update(number=23, replace_body=True)
        with self.assertRaisesRegex(StopPublication, "empreinte"):
            validate(self.plan, self.folder)

    def test_remove_only_explicit_obsolete_dependency(self):
        self.prepare_replacement()
        self.plan["tickets"][1]["blocked_by"] = ["a"]
        self.plan["tickets"][1]["remove_blocked_by"] = [3]
        self.save_plan()
        self.publisher().publish(True)
        self.assertNotIn((102, 3, "blocker"), self.api.relations)
        self.assertIn((102, 101, "blocker"), self.api.relations)
        self.assertIn((21, 102, "parent"), self.api.relations)
        self.assertEqual([3], self.publisher().state["records"]["b"]["removed_blockers"])

    def test_failed_removal_stops_and_keeps_pending_state(self):
        self.prepare_replacement()
        self.plan["tickets"][0]["remove_blocked_by"] = [9]
        self.plan["tickets"][1]["remove_blocked_by"] = [8]
        self.save_plan()
        self.api.failure = "remove"
        with self.assertRaises(GhFailure):
            self.publisher().publish(True)
        self.assertEqual([("remove", 101, 9)], [w for w in self.api.writes if w[0] == "remove"])
        self.assertEqual(9, self.publisher().state["records"]["a"]["pending_remove_blocker"])

    def test_required_dependency_cannot_be_removed(self):
        self.prepare_replacement()
        self.plan["tickets"][1]["remove_blocked_by"] = [101]
        with self.assertRaisesRegex(StopPublication, "encore requise"):
            validate(self.plan, self.folder)

    def test_missing_dependency_rejected(self):
        self.plan["tickets"][0]["blocked_by"] = ["absent"]
        with self.assertRaises(StopPublication):
            validate(self.plan, self.folder)

    def test_duplicate_number_rejected(self):
        for t in self.plan["tickets"]:
            t["number"] = 23
        with self.assertRaises(StopPublication):
            validate(self.plan, self.folder)

    def test_numeric_internal_dependency_cannot_hide_a_cycle(self):
        self.plan["tickets"][0]["number"] = 23
        self.plan["tickets"][1]["number"] = 24
        self.plan["tickets"][0]["blocked_by"] = [24]
        self.plan["tickets"][1]["blocked_by"] = [23]
        with self.assertRaisesRegex(StopPublication, "clé"):
            validate(self.plan, self.folder)

    def test_parent_cannot_be_imported_as_child(self):
        self.plan["tickets"][0]["number"] = 21
        with self.assertRaises(StopPublication):
            validate(self.plan, self.folder)

    def test_state_repo_mismatch_rejected(self):
        self.publisher().publish()
        self.plan["repo"] = "another/repo"
        self.save_plan()
        with self.assertRaisesRegex(StopPublication, "autre publication"):
            self.publisher()

    def test_concurrent_publication_rejected(self):
        with publication_lock(self.folder):
            with self.assertRaisesRegex(StopPublication, "verrouillée"):
                with publication_lock(self.folder):
                    self.fail("Deux publications simultanées")

    def test_temp_folder_rejected_for_live_publication(self):
        with self.assertRaises(StopPublication):
            durable_folder(self.folder)


class Transport(unittest.TestCase):
    def test_remove_uses_issue_id_and_verifies_absence(self):
        calls = []
        replies = ['[{"number":38,"id":5748082141}]', '{}', '[]']
        def run(args, **kw):
            calls.append(args)
            return subprocess.CompletedProcess(args, 0, replies.pop(0), "")
        GitHub("owner/repo", run).remove_blocker(40, 38)
        self.assertIn("DELETE", calls[1])
        self.assertIn("repos/owner/repo/issues/40/dependencies/blocked_by/5748082141", calls[1])
        self.assertIn("GET", calls[2])

    def test_unicode_payload_is_stdin_and_arguments_are_not_shell(self):
        calls = []
        def run(args, **kw):
            calls.append((args, kw))
            return subprocess.CompletedProcess(args, 0, 'HTTP/2.0 201 Created\r\n\r\n{"number":23}', "")
        GitHub("owner/repo", run).create("L’état $(commande) `texte`", "é\n'\"\\")
        args, kw = calls[0]
        self.assertFalse(kw["shell"])
        self.assertNotIn("L’état", " ".join(args))
        self.assertEqual("L’état $(commande) `texte`", json.loads(kw["input"])["title"])
        self.assertEqual("é\n'\"\\", json.loads(kw["input"])["body"])

    def test_empty_500_is_server_not_json_syntax(self):
        result = subprocess.CompletedProcess([], 1, "HTTP/2.0 500 Internal Server Error\nX-GitHub-Request-Id: proof\n\n", "unexpected end of JSON input")
        with self.assertRaises(GhFailure) as ctx:
            decode_reply(result)
        self.assertEqual(("serveur", 500, "proof"), (ctx.exception.kind, ctx.exception.status, ctx.exception.request_id))

    def test_network_failure_classified(self):
        with self.assertRaises(GhFailure) as ctx:
            decode_reply(subprocess.CompletedProcess([], 1, "", "dial tcp connectex denied"))
        self.assertEqual("réseau", ctx.exception.kind)

    def test_malformed_json_stops(self):
        with self.assertRaises(GhFailure):
            decode_reply(subprocess.CompletedProcess([], 0, "HTTP/2.0 200 OK\n\n{", ""))

    def test_timeout_never_retries(self):
        calls = []
        def run(args, **kw):
            calls.append(args)
            raise subprocess.TimeoutExpired(args, 45)
        with self.assertRaises(GhFailure):
            GitHub("owner/repo", run).create("titre", "texte")
        self.assertEqual(1, len(calls))


if __name__ == "__main__":
    unittest.main()
