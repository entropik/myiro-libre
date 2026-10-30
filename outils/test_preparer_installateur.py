"""ArgyllCMS dans l'installateur : embarqué seulement complet, licencié et en 3.5.0.

Les dossiers sont factices et l'aide de colprof est simulée : aucun binaire
d'ArgyllCMS n'est lancé, aucun installateur n'est compilé.
"""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

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

    def test_argyll_complet_donne_ses_six_fichiers(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        self.assertEqual({
            "../argyll/bin/colprof.exe": "argyll/bin/colprof.exe",
            "../argyll/bin/profcheck.exe": "argyll/bin/profcheck.exe",
            "../argyll/bin/targen.exe": "argyll/bin/targen.exe",
            "../argyll/bin/iccgamut.exe": "argyll/bin/iccgamut.exe",
            "../argyll/ref/sRGB.icm": "argyll/ref/sRGB.icm",
            "../argyll/License.txt": "argyll/License.txt",
        }, self.embarquer())

    def test_un_autre_nom_de_licence_est_accepte(self):
        self.poser(*pi.ARGYLL_FICHIERS, "AGPLv3.txt")
        self.assertEqual("argyll/AGPLv3.txt", self.embarquer()["../argyll/AGPLv3.txt"])

    def test_toutes_les_licences_presentes_sont_embarquees(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt", "AGPLv3.txt", "COPYING")
        ressources = self.embarquer()
        for nom in ("License.txt", "AGPLv3.txt", "COPYING"):
            self.assertEqual(f"argyll/{nom}", ressources[f"../argyll/{nom}"])
        self.assertEqual(8, len(ressources))

    def test_une_licence_garde_son_nom_reel(self):
        # Sous Windows, « LICENSE.TXT » répond aussi à « License.txt » : il n'est
        # embarqué qu'une fois, sous son nom réel.
        self.poser(*pi.ARGYLL_FICHIERS, "LICENSE.TXT")
        ressources = self.embarquer()
        self.assertEqual({"argyll/LICENSE.TXT"}, {c for c in ressources.values() if "bin/" not in c and "ref/" not in c})

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

    def test_sans_iccgamut_l_installateur_s_arrete(self):
        # Sans iccgamut, la comparaison des profils perdrait la limite du gamut.
        self.poser("bin/colprof.exe", "bin/profcheck.exe", "ref/sRGB.icm", "License.txt")
        with self.assertRaises(SystemExit) as arret:
            self.embarquer()
        self.assertIn("bin/iccgamut.exe", str(arret.exception.code))

    def test_une_autre_version_est_refusee(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        with self.assertRaises(SystemExit) as arret:
            self.embarquer(aide="Create ICC profile, Version 3.4.1\n")
        self.assertIn("3.4.1", str(arret.exception.code))

    def test_une_version_illisible_est_refusee(self):
        self.poser(*pi.ARGYLL_FICHIERS, "License.txt")
        with self.assertRaises(SystemExit):
            self.embarquer(aide="colprof: Error - usage\n")


class MireFd9Embarquee(unittest.TestCase):
    """Mire FD-9 RGB-2520 : embarquée telle quelle, seulement avec son empreinte."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.racine = Path(self.tmp.name)

    def poser(self, octets):
        chemin = self.racine / "mire-fd9" / pi.MIRE_FD9_FICHIER
        chemin.parent.mkdir(parents=True, exist_ok=True)
        chemin.write_bytes(octets)

    def test_sans_dossier_mire_fd9_rien_n_est_embarque(self):
        self.assertEqual({}, pi.mire_fd9_embarquee(self.racine))

    def test_la_mire_d_origine_est_embarquee_sous_mire_fd9(self):
        self.poser(b"mire factice")
        empreinte = pi.hashlib.sha256(b"mire factice").hexdigest()
        self.assertEqual(
            {"../mire-fd9/RGB-2520 A3 - 1p.tif": "mire-fd9/RGB-2520 A3 - 1p.tif"},
            pi.mire_fd9_embarquee(self.racine, empreinte=empreinte))

    def test_une_autre_empreinte_arrete(self):
        self.poser(b"mire modifiee")
        with self.assertRaises(SystemExit) as arret:
            pi.mire_fd9_embarquee(self.racine)
        self.assertIn("empreinte", str(arret.exception.code))

    def test_un_dossier_sans_la_mire_arrete(self):
        (self.racine / "mire-fd9").mkdir()
        with self.assertRaises(SystemExit) as arret:
            pi.mire_fd9_embarquee(self.racine)
        self.assertIn(pi.MIRE_FD9_FICHIER, str(arret.exception.code))

    def test_l_annonce_dit_la_regle_de_sdk(self):
        # Embarquée, la mire suit la même règle que SDK/ : installateur local, jamais publié tel quel.
        annonce = pi.annonce_mire_fd9(True)
        self.assertNotIn("\n", annonce)
        self.assertIn("SDK/", annonce)
        self.assertIn("jamais publié", annonce)
        # Sans mire-fd9/, une ligne aussi, et le script continue.
        absente = pi.annonce_mire_fd9(False)
        self.assertNotIn("\n", absente)
        self.assertIn("mire-fd9/ absent", absente)

    def test_l_empreinte_est_celle_de_la_crate_mires(self):
        rgb2520 = (pi.RACINE / "crates" / "mires" / "src" / "rgb2520.rs").read_text(encoding="utf-8")
        self.assertIn(pi.MIRE_FD9_EMPREINTE, rgb2520)
        self.assertIn(pi.MIRE_FD9_FICHIER, rgb2520)


class AideColprof(unittest.TestCase):
    def test_un_colprof_qui_ne_repond_pas_arrete_clairement(self):
        attente = subprocess.TimeoutExpired(cmd="colprof", timeout=30)
        with mock.patch.object(pi.subprocess, "run", side_effect=attente):
            with self.assertRaises(SystemExit) as arret:
                pi.aide_colprof(Path("argyll/bin/colprof.exe"))
        self.assertIn("n'a pas répondu", str(arret.exception.code))
        self.assertIn("colprof.exe", str(arret.exception.code))


class LectureVersion(unittest.TestCase):
    def test_agpl_version_3_n_est_pas_un_numero(self):
        self.assertEqual("3.5.0", pi.lire_version_argyll(AIDE_350))

    def test_deux_numeros_differents_ne_donnent_rien(self):
        self.assertIsNone(pi.lire_version_argyll("Version 3.5.0\nVersion 3.4.1\n"))


if __name__ == "__main__":
    unittest.main()
