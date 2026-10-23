"""ArgyllCMS dans l'installateur : embarqué seulement complet, licencié et en 3.5.0.

Les dossiers sont factices et l'aide de colprof est simulée : aucun binaire
d'ArgyllCMS n'est lancé, aucun installateur n'est compilé.
"""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import preparer_installateur as pi  # noqa: E402

AIDE_350 = "Create ICC profile, Version 3.5.0\nLicensed under the AGPL Version 3\n"


class ArgyllEmbarque(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.racine = Path(self.tmp.name)

    def poser(self, *fichiers):
        for f in fichiers:
            chemin = self.racine / "argyll" / f
            chemin.parent.mkdir(parents=True, exist_ok=True)
            chemin.write_bytes(b"factice")

    def embarquer(self, aide=AIDE_350):
        return pi.argyll_embarque(self.racine, aide=lambda colprof: aide)

    def test_sans_dossier_argyll_rien_n_est_embarque(self):
        self.assertEqual({}, pi.argyll_embarque(self.racine, aide=lambda colprof: self.fail("colprof lancé")))

    def test_argyll_complet_donne_ses_quatre_fichiers(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        self.assertEqual({
            "../argyll/bin/colprof.exe": "argyll/bin/colprof.exe",
            "../argyll/bin/profcheck.exe": "argyll/bin/profcheck.exe",
            "../argyll/ref/sRGB.icm": "argyll/ref/sRGB.icm",
            "../argyll/License.txt": "argyll/License.txt",
        }, self.embarquer())

    def test_un_autre_nom_de_licence_est_accepte(self):
        self.poser(*pi.ARGYLL_FICHIERS, "AGPLv3.txt")
        self.assertEqual("argyll/AGPLv3.txt", self.embarquer()["../argyll/AGPLv3.txt"])

    def test_sans_licence_rien_n_est_embarque(self):
        self.poser(*pi.ARGYLL_FICHIERS)
        with self.assertRaises(SystemExit) as arret:
            self.embarquer()
        self.assertIn("licence", str(arret.exception.code))

    def test_un_argyll_incomplet_arrete(self):
        self.poser("bin/colprof.exe", "License.txt")
        with self.assertRaises(SystemExit) as arret:
            self.embarquer()
        self.assertIn("bin/profcheck.exe", str(arret.exception.code))

    def test_une_autre_version_est_refusee(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        with self.assertRaises(SystemExit) as arret:
            self.embarquer(aide="Create ICC profile, Version 3.4.1\n")
        self.assertIn("3.4.1", str(arret.exception.code))

    def test_une_version_illisible_est_refusee(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        with self.assertRaises(SystemExit):
            self.embarquer(aide="colprof: Error - usage\n")


class LectureVersion(unittest.TestCase):
    def test_agpl_version_3_n_est_pas_un_numero(self):
        self.assertEqual("3.5.0", pi.lire_version_argyll(AIDE_350))

    def test_deux_numeros_differents_ne_donnent_rien(self):
        self.assertIsNone(pi.lire_version_argyll("Version 3.5.0\nVersion 3.4.1\n"))


if __name__ == "__main__":
    unittest.main()
