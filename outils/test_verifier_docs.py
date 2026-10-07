#!/usr/bin/env python3
"""Tests de outils/verifier_docs.py, lancé comme en CI : dans un dépôt neuf.

Usage : python outils/test_verifier_docs.py
Chaque test copie le script dans un dépôt git temporaire et l'exécute là,
sans liste locale de mots interdits (cas de la CI) ou avec une liste fictive.
"""
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parent / "verifier_docs.py"


class DepotNeuf:
    """Dépôt git temporaire contenant une copie de verifier_docs.py."""

    def __init__(self):
        self.dossier = pathlib.Path(tempfile.mkdtemp(prefix="verifier-docs-"))
        (self.dossier / "outils").mkdir()
        shutil.copy(SCRIPT, self.dossier / "outils" / "verifier_docs.py")
        (self.dossier / "docs" / "blog").mkdir(parents=True)
        subprocess.run(["git", "init", "-q"], cwd=self.dossier, check=True)

    def ecrire(self, nom, texte):
        chemin = self.dossier / nom
        chemin.parent.mkdir(parents=True, exist_ok=True)
        chemin.write_text(texte, encoding="utf-8")

    def verifier(self):
        r = subprocess.run([sys.executable, "outils/verifier_docs.py"], cwd=self.dossier,
                           capture_output=True, text=True, encoding="utf-8")
        return r.returncode, r.stdout

    def effacer(self):
        shutil.rmtree(self.dossier, ignore_errors=True)


class SansListeDeMotsInterdits(unittest.TestCase):
    def setUp(self):
        self.depot = DepotNeuf()
        self.addCleanup(self.depot.effacer)

    def test_annonce_que_le_controle_des_mots_est_saute(self):
        self.depot.ecrire("README.md", "Rien à signaler.\n")
        code, sortie = self.depot.verifier()
        self.assertEqual(code, 0, sortie)
        self.assertIn("mots interdits absente", sortie)

    def test_verifie_quand_meme_les_liens(self):
        self.depot.ecrire("README.md", "Voir [la suite](docs/absent.md).\n")
        code, sortie = self.depot.verifier()
        self.assertEqual(code, 1, sortie)
        self.assertIn("lien cassé vers docs/absent.md", sortie)


class AvecListeDeMotsInterdits(unittest.TestCase):
    def test_un_mot_de_la_liste_bloque(self):
        depot = DepotNeuf()
        self.addCleanup(depot.effacer)
        depot.ecrire(".mots-interdits.local", "motfictif\n")
        depot.ecrire("README.md", "Ici un MotFictif.\n")
        code, sortie = depot.verifier()
        self.assertEqual(code, 1, sortie)
        self.assertIn("README.md:1 : mot interdit", sortie)
        self.assertNotIn("mots interdits absente", sortie)


if __name__ == "__main__":
    unittest.main()
