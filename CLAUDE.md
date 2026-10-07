# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Objectif du projet

Construire un outil de mesure et de profilage ICC pour deux spectrophotomètres Konica Minolta dont le support logiciel est arrêté : le **MYIRO-1** (piloté par `FDXSDK.dll`) et le **FD-9** (piloté par `FD9SDK.dll`). Le dépôt contient un audit statique des installations du poste (Windows) et le code Rust du pont MYIRO-1, qui pilote l'instrument réel de la version du SDK jusqu'à la lecture en bande (7 octobre 2026). `.gitignore` exclut `collecte/`, `SDK/` et tout binaire ou manuel Konica Minolta. Le plan d'action est dans `PLAN-ACTION.md` (application Rust libre GPL-3.0, MYIRO-1 d'abord, FD-9 en parallèle ; le FD-9 du poste est piloté par FD-S2w en réseau). `Audit-MYIRO/retroanalyse/` contient les désassemblages Capstone produits par `Audit-MYIRO/outils/desassemble.py`.

La documentation de l'audit et les scripts sont en français ; garder cette langue pour les rapports et notes.

## Structure

- `SDK/` : DLL de travail, à utiliser comme cibles de développement.
  - `SDK/MY-CT1-x86/FDXSDK.dll` : 1.0.1.0, **x86** (impose un processus 32 bits pour un chargement direct).
  - `SDK/Ergosoft-x64/FDXSDK.dll` : 1.0.1.0, **x64**.
- `Audit-MYIRO/collecte/` : copies brutes (contrôlées SHA-256) des installations trouvées : MY-CT1, FD-S2w (application, pilote USB, mires XML et `.s2chart`, manuels), Ergosoft, EIZO ColorNavigator 7, résidus `ProgramData/MYIRO`. Archive en lecture seule : ne pas modifier, ne pas exécuter les installateurs ni les `.bat`.
- `Audit-MYIRO/analyse/` : résultats générés (exports/imports/versions/PDB par binaire en `.json`, chaînes avec offsets en `.strings.tsv`, `manifest.json`/`inventaire.csv`, `comparaison-FDX.json`, `indices-techniques.txt`).
- `Audit-MYIRO/LIRE-MOI-AUDIT.txt` : synthèse des découvertes, limites et prochaines étapes. À lire en premier.
- `Audit-MYIRO/outils/` : scripts d'audit ; `python-libs/` contient une copie locale de `pefile` (aucune installation globale).

## Commandes

Python 3 sur Windows, sans dépendance externe (pefile est importé depuis `outils/python-libs` via `sys.path`) :

```
python Audit-MYIRO/outils/audit.py     # recollecte depuis C:/Program Files..., C:/ProgramData... puis analyse PE
python Audit-MYIRO/outils/synthese.py  # régénère comparaison, indices-techniques.txt, LIRE-MOI-AUDIT.txt, et revérifie tous les SHA-256
```

- `audit.py` lit des chemins absolus du poste (`C:/Program Files (x86)/Configuration Tool MY-CT1`, `.../KONICA MINOLTA/FD-S2w`, `C:/ProgramData/MYIRO`, `C:/Program Files/Ergosoft 16`, `C:/ProgramData/EIZO/ColorNavigator 7`) ; il échoue sur collision de hash dans `collecte/`.
- `synthese.py` **réécrit entièrement** `LIRE-MOI-AUDIT.txt` à partir de texte codé en dur dans le script : modifier le script, pas le fichier texte. Il dépend aussi de `analyse/installateur-inventaire.csv` (produit hors de ces scripts).

Code Rust (espace de travail Cargo à la racine) :

- `crates/fdx-sys` : liste blanche des exports de `FDXSDK.dll` et formes binaires ; ne charge jamais la DLL.
- `crates/pont-protocole` : messages JSON entre l'application et les ponts (requêtes, réponses, provenance) ; indépendant de Windows.
- `crates/pont-myiro1` : session (paliers, plafond, journal), adapter `FdxDll`, boucle du protocole et exécutable `pont-myiro1 --dll <FDXSDK.dll> [--plafond <palier>]`.

```
cargo test                                     # tous les tests, contre un instrument simulé
cargo clippy --all-targets -- -D warnings
cargo test --target i686-pc-windows-msvc       # même chose en 32 bits (DLL de MY-CT1)
cargo test -p pont-myiro1 --test dll -- --ignored palier_version   # appels réels à la DLL du poste
```

Les tests `--ignored` de `crates/pont-myiro1/tests/dll.rs` parlent au vrai MYIRO-1 et, à partir de l'étalonnage, demandent des gestes à l'opérateur : ne les lancer qu'avec son accord, palier par palier. Leurs sorties (mesures) vont dans `Archivage/donnees/`, local et non versionné.

## Points techniques établis par l'audit

- Trois `FDXSDK.dll` (MY-CT1 x86 1.0.1, Ergosoft x64 1.0.1, EIZO x64 1.0.3) exportent les mêmes 91 noms ; cela ne garantit pas le même ABI (structures, conventions d'appel x86, codes d'erreur).
- `FD9SDK.dll` (125 exports : 1.3.2.3 x86 dans FD-S2w, 1.3.1.5 x64 dans Ergosoft) est un SDK distinct de `FDXSDK` : ne pas les confondre.
- Aucun en-tête ni PDB disponible : les signatures sont à reconstituer par désassemblage des exports et des appelants (MY-CT1, FD-S2w, `libmeasurementdevice*.dll` d'EIZO). Toujours distinguer « confirmé » et « supposé » dans la documentation produite.
- Interfaces prioritaires : `FDX_GetSDKVersion`, `FDX_GetDevicePortList`, `FDX_Connect`, `FDX_GetDeviceInfo`, `FDX_Calibration`, `FDX_Get/SetMeasureCondition`, `FDX_StartMeasurement`, `FDX_StopMeasurement`, `FDX_GetMeasureData`, `FDX_GetRAWData`, `FDX_CalculateMeasureData`.
- USB : VID `132B`, PID `210D`/`210F` (correspondance PID/modèle non confirmée). Imports `WS2_32` : un mode réseau existe aussi.
- FDXSDK n'expose aucune création de profil ICC : la chaîne prévue est mesure → export CGATS.17 → moteur de profilage séparé. Un export CGATS MYIRO-1 de référence (45/0, D50, 2°) est dans `collecte/MYIRO-ProgramData/MYIROtools`.

## Règles de sécurité matérielle

- Ne jamais appeler les exports `FDX_JIG_*` (`SetFactoryCalib_*`, `UpdateProgram`, `SendProgramData`, `WriteSector`, `InitializeEEProm`…) : ce sont des surfaces de maintenance usine qui peuvent altérer ou bloquer l'instrument. Traiter aussi les autres `FDX_Set*` qui écrivent dans l'appareil (`SetNetworkInfo`, `SetCalibration`, `SetUserRefData`…) comme à risque tant que leur contrat n'est pas établi.
- Progression attendue sur appareil réel : version SDK → détection → connexion/infos → calibration → mesure ponctuelle, avant toute lecture de bandes. Ne pas installer de pilote ni lancer les installateurs archivés sans accord explicite.
- Les DLL et manuels sont sous licence du fabricant : aucun droit de redistribution n'est présumé.

## Agent skills

### Issue tracker

Les tickets vivent dans les GitHub Issues du dépôt public `entropik/myiro-libre` (CLI `gh`). See `docs/agents/issue-tracker.md`.

### Triage labels

Étiquettes par défaut : `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context : `GLOSSARY.md` et `docs/adr/` à la racine, créés au besoin. See `docs/agents/domain.md`.

## Documentation

- La documentation vit dans `docs/` : journal (`docs/blog/AAAA-MM-JJ.md`), décisions (`docs/adr/`), références ouvertes (`docs/references/`), fiches d'ABI (`docs/abi/`), système graphique de l'interface (`design-system/`, à ouvrir dans un navigateur), plus `GLOSSARY.md` et `PLAN-ACTION.md`. Elle est rédigée en français, avec des mots simples (l'utilisateur est imprimeur, pas développeur).
- En fin de tâche importante, mettre à jour le journal du jour, l'ADR concerné et le glossaire ; la commande `/fin-de-journee` fait le tour complet.
- `python outils/verifier_docs.py` contrôle les mots interdits (liste locale `.mots-interdits.local`, non versionnée), les liens cassés et rappelle le journal du jour. `python outils/verifier_docs.py --installer` le branche sur les commits (hooks locaux).
- `docs/sources/` est local et ignoré par git (documents de tiers) : ne jamais le versionner, ne jamais le citer par nom dans un fichier suivi. Les analyses de logiciels concurrents n'y nomment pas le produit dans les fichiers publiés : écrire « un logiciel concurrent ».
- Les références externes (normes, projets libres) vont dans `docs/references/` sous forme de liens et de résumés, jamais de copies de contenu protégé.
