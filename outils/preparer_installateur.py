#!/usr/bin/env python3
"""Prépare la construction de l'installateur Windows de myiro-libre.

Usage (à la racine du dépôt) :
    python outils/preparer_installateur.py
    cargo tauri build --config app/tauri.installateur.json

1. Compile les ponts `pont-myiro1` et `pont-fd9` en 64 bits et en 32 bits
   (i686-pc-windows-msvc ; la DLL du FD-9 de FD-S2w est 32 bits) et les dépose
   dans `app/binaries/` sous les noms attendus par Tauri (`bundle.externalBin`) ;
   installés, ils deviennent `pont-myiro1-x64.exe`, `pont-fd9-x86.exe`, etc.
   à côté de l'application.
2. Écrit `app/tauri.installateur.json`, la configuration complémentaire de
   l'installateur. Si le dossier `SDK/` du dépôt existe (ignoré par git), chacun
   de ses sous-dossiers qui contient FDXSDK.dll est embarqué entier dans
   `sdk/x64/` ou `sdk/x86/` selon l'architecture de la DLL.
3. Si le dossier `argyll/` du dépôt existe (ignoré par git), vérifie qu'il
   contient ArgyllCMS 3.5.0 (ADR 0002 : version figée) et embarque `colprof`,
   `profcheck`, `iccgamut` (limite du gamut, comparaison des profils), `targen`,
   `ref/sRGB.icm` et tous les textes de licence présents sous
   `argyll/`.
   Sans `argyll/`, l'installateur se construit sans ArgyllCMS : l'application
   dira qu'il est introuvable au moment de calculer un profil ou de préparer
   une mire MYIRO-1.
4. Si le dossier `mire-fd9/` du dépôt existe (ignoré par git), vérifie
   l'empreinte de la mire RGB-2520 et l'embarque telle quelle sous
   `mire-fd9/`. Une autre empreinte arrête tout. Sans `mire-fd9/`,
   l'application dira la mire FD-9 introuvable ; le MYIRO-1 reste utilisable.
   Embarquée, la mire suit la même règle que `SDK/` : installateur local,
   jamais publié tel quel.
5. Cherche `zxing.dll` (ZXing.Net 0.14.0, Apache-2.0) dans le dossier `Module`
   de FD-S2w, puis dans `SDK/` et ses sous-dossiers ; le premier trouvé est
   copié dans `app/binaries/zxing/` (avec `zxing.presentation.dll` s'il est
   à côté) et installé à côté de `pont-fd9-x86.exe`, avec sa licence
   (`outils/licences/zxing/`). La DLL du FD-9 le charge depuis le dossier du
   pont pour lire le QR des mires (ticket #87). Sans lui, une ligne le dit et
   le pont FD-9 refusera de lancer une feuille.

ArgyllCMS est sous AGPL-3.0 : chaque publication d'un installateur qui
l'embarque fournit les sources de la version incluse, ou leur lien précis.

Un installateur qui embarque `SDK/` est STRICTEMENT LOCAL : les DLL du fabricant
n'ont aucun droit de redistribution. Ne jamais le publier (Releases GitHub
comprises). Sans `SDK/`, l'installateur est publiable.
"""
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys

RACINE = pathlib.Path(__file__).resolve().parent.parent
APP = RACINE / "app"
CIBLE_32 = "i686-pc-windows-msvc"
PONTS = ("pont-myiro1", "pont-fd9")
VERSION_ARGYLL = "3.5.0"
ARGYLL_FICHIERS = (
    "bin/colprof.exe",
    "bin/profcheck.exe",
    "bin/targen.exe",
    "bin/iccgamut.exe",
    "ref/sRGB.icm",
)
# Noms possibles des textes de licence à la racine de la distribution (non
# confirmés pour la 3.5.0) : tous ceux qui sont présents sont embarqués, sous
# leur nom réel, sans tenir compte de la casse.
ARGYLL_LICENCES = ("License.txt", "AGPLv3.txt", "License3.txt", "LICENSE.txt", "COPYING.txt", "COPYING")
# Mire FD-9 RGB-2520, reprise telle quelle (crate `mires`, `rgb2520.rs`).
MIRE_FD9_FICHIER = "RGB-2520 A3 - 1p.tif"
MIRE_FD9_EMPREINTE = "7550f4a38725ad9260982152bb6bcbe077922289adb48dd954f0bd7d2c7fc713"
# Composant de lecture du QR du FD-9 (ticket #87) : chargé par FD9SDK.dll
# depuis le dossier du pont. Apache-2.0 : redistribuable avec sa licence.
MODULE_FD_S2W = "C:/Program Files (x86)/KONICA MINOLTA/FD-S2w/Module"
ZXING = "zxing.dll"
ZXING_VOISINS = ("zxing.presentation.dll",)
ZXING_LICENCES = ("LICENSE-2.0.txt", "NOTICE.txt")
# SHA-256 du texte de l'Apache License 2.0 publié par la fondation Apache
# (https://www.apache.org/licenses/LICENSE-2.0.txt), fins de ligne LF.
EMPREINTE_APACHE_2_0 = "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"


def architecture(fichier):
    """« x64 », « x86 » ou None, d'après l'en-tête PE (le fichier n'est pas chargé)."""
    octets = fichier.read_bytes()[:4096]
    if octets[:2] != b"MZ" or len(octets) < 0x40:
        return None
    pe = int.from_bytes(octets[0x3C:0x40], "little")
    if octets[pe:pe + 4] != b"PE\0\0":
        return None
    return {0x8664: "x64", 0x014C: "x86"}.get(int.from_bytes(octets[pe + 4:pe + 6], "little"))


def hote():
    sortie = subprocess.run(["rustc", "-vV"], capture_output=True, text=True, check=True).stdout
    return next(l.split(": ")[1] for l in sortie.splitlines() if l.startswith("host: "))


def compiler_ponts(triple_hote):
    target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", RACINE / "target"))
    binaires = APP / "binaries"
    binaires.mkdir(exist_ok=True)
    for pont in PONTS:
        for arch, cible in (("x64", None), ("x86", CIBLE_32)):
            commande = ["cargo", "build", "--release", "-p", pont]
            if cible:
                commande += ["--target", cible]
            subprocess.run(commande, cwd=RACINE, check=True)
            source = target / (cible or "") / "release" / f"{pont}.exe"
            if architecture(source) != arch:
                sys.exit(f"{source} n'est pas un exécutable {arch}")
            shutil.copy2(source, binaires / f"{pont}-{arch}-{triple_hote}.exe")
            print(f"Pont {pont} {arch} : {source}")


def dll_embarquees():
    """Ressources Tauri : chaque dossier de SDK/ qui contient FDXSDK.dll, par architecture."""
    sdk = RACINE / "SDK"
    ressources, vues = {}, set()
    if not sdk.is_dir():
        return ressources
    for dossier in sorted(p for p in sdk.iterdir() if p.is_dir()):
        dll = dossier / "FDXSDK.dll"
        arch = architecture(dll) if dll.is_file() else None
        if arch is None or arch in vues:
            continue
        vues.add(arch)
        ressources[f"../SDK/{dossier.name}/*"] = f"sdk/{arch}/"
    return ressources


def lire_version_argyll(texte):
    """« Version 3.5.0 » dans l'aide de colprof → « 3.5.0 ». Seul un numéro en
    trois parties compte (« AGPL Version 3 » n'en est pas un) ; deux numéros
    différents ne donnent rien, comme dans la crate `profilage`."""
    mots = texte.split()
    versions = set()
    for mot, suivant in zip(mots, mots[1:]):
        v = suivant.rstrip(".,;:)")
        parties = v.split(".")
        if mot == "Version" and len(parties) == 3 and all(p.isdigit() for p in parties):
            versions.add(v)
    return versions.pop() if len(versions) == 1 else None


def aide_colprof(colprof, delai=30):
    """Aide de colprof lancé sans argument (il rend un code d'erreur : seul le texte compte)."""
    try:
        sortie = subprocess.run([str(colprof)], capture_output=True, timeout=delai)
    except subprocess.TimeoutExpired:
        sys.exit(f"{colprof} n'a pas répondu en {delai} s : impossible de lire la version d'ArgyllCMS. "
                 "Vérifiez que le dossier argyll/ contient une copie complète d'ArgyllCMS 3.5.0, puis relancez.")
    return (sortie.stdout + sortie.stderr).decode("utf-8", errors="replace")


def licences_argyll(argyll):
    """Textes de licence présents à la racine d'`argyll/`, sous leur nom réel."""
    noms = {nom.lower() for nom in ARGYLL_LICENCES}
    return sorted(p.name for p in argyll.iterdir() if p.is_file() and p.name.lower() in noms)


def argyll_embarque(racine=RACINE, aide=aide_colprof):
    """Ressources Tauri d'ArgyllCMS sous `argyll/`, ou {} sans dossier `argyll/`.
    Un ArgyllCMS incomplet, sans licence ou d'une autre version arrête tout."""
    argyll = racine / "argyll"
    if not argyll.is_dir():
        return {}
    manquants = [f for f in ARGYLL_FICHIERS if not (argyll / f).is_file()]
    if manquants:
        sys.exit(f"ArgyllCMS incomplet dans {argyll} : il manque {', '.join(manquants)}")
    licences = licences_argyll(argyll)
    if not licences:
        sys.exit(f"Texte de licence d'ArgyllCMS introuvable dans {argyll} (cherché : {', '.join(ARGYLL_LICENCES)}) : "
                 "l'AGPL interdit de l'embarquer sans lui.")
    version = lire_version_argyll(aide(argyll / "bin" / "colprof.exe"))
    if version != VERSION_ARGYLL:
        sys.exit(f"ArgyllCMS de {argyll} en version {version or 'illisible'} : seule la {VERSION_ARGYLL} est validée (ADR 0002).")
    return {f"../argyll/{f}": f"argyll/{f}" for f in (*ARGYLL_FICHIERS, *licences)}


def mire_fd9_embarquee(racine=RACINE, empreinte=MIRE_FD9_EMPREINTE):
    """Ressource Tauri de la mire RGB-2520 sous `mire-fd9/`, ou {} sans dossier
    `mire-fd9/`. Une mire absente du dossier ou d'une autre empreinte arrête tout."""
    dossier = racine / "mire-fd9"
    if not dossier.is_dir():
        return {}
    mire = dossier / MIRE_FD9_FICHIER
    if not mire.is_file():
        sys.exit(f"Mire FD-9 introuvable : {mire} (dossier mire-fd9/ présent, fichier {MIRE_FD9_FICHIER} absent).")
    trouvee = hashlib.sha256(mire.read_bytes()).hexdigest()
    if trouvee != empreinte:
        sys.exit(f"{mire} n'est pas la mire RGB-2520 retenue : empreinte {trouvee}, attendue {empreinte}. "
                 "Reprenez le fichier d'origine, sans l'ouvrir ni l'enregistrer dans un autre logiciel.")
    return {f"../mire-fd9/{MIRE_FD9_FICHIER}": f"mire-fd9/{MIRE_FD9_FICHIER}"}


def annonce_mire_fd9(embarquee):
    """Une ligne sur la mire FD-9. Embarquée, elle suit la même règle que SDK/."""
    if embarquee:
        return ("ATTENTION : cet installateur embarque aussi la mire RGB-2520 (empreinte vérifiée) : même règle "
                "que SDK/, installateur local, jamais publié tel quel.")
    return "Dossier mire-fd9/ absent : mire FD-9 non embarquée, seule la mire MYIRO-1 pourra être préparée."


def dossiers_zxing(racine=RACINE):
    """Où chercher zxing.dll, dans l'ordre : FD-S2w, puis SDK/ et ses sous-dossiers."""
    sdk = racine / "SDK"
    dossiers = [pathlib.Path(MODULE_FD_S2W), sdk]
    if sdk.is_dir():
        dossiers += sorted(p for p in sdk.iterdir() if p.is_dir())
    return dossiers


def source_zxing(dossiers):
    """Premier zxing.dll trouvé dans `dossiers`, ou None."""
    return next((d / ZXING for d in dossiers if (d / ZXING).is_file()), None)


def zxing_embarque(racine=RACINE, dossiers=None):
    """Ressources Tauri de zxing.dll, installé à côté des ponts, et de sa licence ;
    {} si aucun n'est trouvé. Le fichier est copié dans app/binaries/zxing/ (ignoré
    par git). Un zxing.dll qui n'est pas un exécutable Windows arrête tout."""
    source = source_zxing(dossiers_zxing(racine) if dossiers is None else dossiers)
    if source is None:
        return {}
    if architecture(source) is None:
        sys.exit(f"{source} n'est pas un exécutable Windows : reprenez le zxing.dll de FD-S2w (ZXing.Net 0.14.0).")
    copie = racine / "app" / "binaries" / "zxing"
    copie.mkdir(parents=True, exist_ok=True)
    ressources = {}
    for nom in (ZXING, *ZXING_VOISINS):
        if (source.parent / nom).is_file():
            shutil.copy2(source.parent / nom, copie / nom)
            ressources[f"binaries/zxing/{nom}"] = nom
    for nom in ZXING_LICENCES:
        ressources[f"../outils/licences/zxing/{nom}"] = f"licences/zxing/{nom}"
    return ressources


def annonce_zxing(source):
    """Une ligne sur le composant de lecture du QR du FD-9."""
    if source is None:
        return (f"zxing.dll introuvable ({MODULE_FD_S2W}, SDK/) : composant de lecture du QR non embarqué, "
                "le pont FD-9 refusera de lancer une feuille.")
    return (f"zxing.dll (ZXing.Net 0.14.0, Apache-2.0) embarqué à côté de pont-fd9-x86.exe depuis {source}, "
            "licence jointe sous licences/zxing/.")


def main():
    # ArgyllCMS et la mire FD-9 sont contrôlés avant la compilation des ponts, qui est longue.
    argyll = argyll_embarque()
    mire_fd9 = mire_fd9_embarquee()
    zxing = zxing_embarque()
    compiler_ponts(hote())
    ressources = dll_embarquees()
    ressources.update(argyll)
    ressources.update(mire_fd9)
    ressources.update(zxing)
    bundle = {
        "active": True,
        "targets": ["nsis"],
        "externalBin": [f"binaries/{pont}-{arch}" for pont in PONTS for arch in ("x64", "x86")],
    }
    if ressources:
        bundle["resources"] = ressources
    (APP / "tauri.installateur.json").write_text(
        json.dumps({"bundle": bundle}, indent=2) + "\n", encoding="utf-8")
    print("Configuration écrite : app/tauri.installateur.json")
    dll = [cible for cible in ressources.values() if cible.startswith("sdk/")]
    if dll:
        print("ATTENTION : cet installateur embarque les DLL du fabricant (" + ", ".join(dll)
              + "). Il est STRICTEMENT LOCAL : ne jamais le publier, ni sur GitHub ni ailleurs.")
    else:
        print("Dossier SDK/ absent : aucune DLL du fabricant embarquée, l'installateur cherchera le logiciel du fabricant.")
    if argyll:
        print(f"ArgyllCMS {VERSION_ARGYLL} embarqué (AGPL-3.0) : toute publication de cet installateur fournit "
              f"les sources d'ArgyllCMS {VERSION_ARGYLL} ou leur lien précis.")
    else:
        print("Dossier argyll/ absent : ArgyllCMS non embarqué, l'application ne pourra pas calculer de profil.")
    print(annonce_mire_fd9(bool(mire_fd9)))
    print(annonce_zxing(source_zxing(dossiers_zxing()) if zxing else None))
    print("Suite : cargo tauri build --config app/tauri.installateur.json")


if __name__ == "__main__":
    main()
