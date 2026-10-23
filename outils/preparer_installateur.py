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
   `profcheck`, `ref/sRGB.icm` et le texte de la licence sous `argyll/`.
   Sans `argyll/`, l'installateur se construit sans ArgyllCMS : l'application
   dira qu'il est introuvable au moment de calculer un profil.

ArgyllCMS est sous AGPL-3.0 : chaque publication d'un installateur qui
l'embarque fournit les sources de la version incluse, ou leur lien précis.

Un installateur qui embarque `SDK/` est STRICTEMENT LOCAL : les DLL du fabricant
n'ont aucun droit de redistribution. Ne jamais le publier (Releases GitHub
comprises). Sans `SDK/`, l'installateur est publiable.
"""
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
ARGYLL_FICHIERS = ("bin/colprof.exe", "bin/profcheck.exe", "ref/sRGB.icm")
# Nom du texte de licence à la racine de la distribution : non confirmé pour
# la 3.5.0, le premier présent de cette liste est retenu.
ARGYLL_LICENCES = ("License.txt", "AGPLv3.txt", "License3.txt", "LICENSE.txt", "COPYING.txt", "COPYING")


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


def aide_colprof(colprof):
    """Aide de colprof lancé sans argument (il rend un code d'erreur : seul le texte compte)."""
    sortie = subprocess.run([str(colprof)], capture_output=True, timeout=30)
    return (sortie.stdout + sortie.stderr).decode("utf-8", errors="replace")


def argyll_embarque(racine=RACINE, aide=aide_colprof):
    """Ressources Tauri d'ArgyllCMS sous `argyll/`, ou {} sans dossier `argyll/`.
    Un ArgyllCMS incomplet, sans licence ou d'une autre version arrête tout."""
    argyll = racine / "argyll"
    if not argyll.is_dir():
        return {}
    manquants = [f for f in ARGYLL_FICHIERS if not (argyll / f).is_file()]
    if manquants:
        sys.exit(f"ArgyllCMS incomplet dans {argyll} : il manque {', '.join(manquants)}")
    licence = next((nom for nom in ARGYLL_LICENCES if (argyll / nom).is_file()), None)
    if licence is None:
        sys.exit(f"Texte de licence d'ArgyllCMS introuvable dans {argyll} (cherché : {', '.join(ARGYLL_LICENCES)}) : "
                 "l'AGPL interdit de l'embarquer sans lui.")
    version = lire_version_argyll(aide(argyll / "bin" / "colprof.exe"))
    if version != VERSION_ARGYLL:
        sys.exit(f"ArgyllCMS de {argyll} en version {version or 'illisible'} : seule la {VERSION_ARGYLL} est validée (ADR 0002).")
    return {f"../argyll/{f}": f"argyll/{f}" for f in (*ARGYLL_FICHIERS, licence)}


def main():
    # ArgyllCMS est contrôlé avant la compilation des ponts, qui est longue.
    argyll = argyll_embarque()
    compiler_ponts(hote())
    ressources = dll_embarquees()
    ressources.update(argyll)
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
        print("Dossier SDK/ absent : aucune DLL embarquée, l'installateur cherchera le logiciel du fabricant.")
    if argyll:
        print(f"ArgyllCMS {VERSION_ARGYLL} embarqué (AGPL-3.0) : toute publication de cet installateur fournit "
              f"les sources d'ArgyllCMS {VERSION_ARGYLL} ou leur lien précis.")
    else:
        print("Dossier argyll/ absent : ArgyllCMS non embarqué, l'application ne pourra pas calculer de profil.")
    print("Suite : cargo tauri build --config app/tauri.installateur.json")


if __name__ == "__main__":
    main()
