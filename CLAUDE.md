# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Le travail courant se fait sur `dev`, un agent à la fois dans le dossier principal,
selon `AGENTS.md`. Pour le périmètre fonctionnel validé et l'ordre de livraison,
consulter [la spécification couleur](docs/specifications/poste-travail-couleur.md).

## Objectif du projet

Construire un outil de mesure et de profilage ICC pour deux spectrophotomètres Konica Minolta dont le support logiciel est arrêté : le **MYIRO-1** (piloté par `FDXSDK.dll`) et le **FD-9** (piloté par `FD9SDK.dll`). Le dépôt contient un audit statique des installations du poste (Windows) et le code Rust du pont MYIRO-1, qui pilote l'instrument réel de la version du SDK jusqu'à la lecture en bande (7 octobre 2026). `.gitignore` exclut `collecte/`, `SDK/`, `argyll/` (ArgyllCMS 3.5.0 posé à la main à la racine), `mire-fd9/` (mire RGB-2520 du FD-9, posée à la main) et tout binaire ou manuel Konica Minolta. Le plan d'action est dans `PLAN-ACTION.md` (application Rust libre GPL-3.0, MYIRO-1 et FD-9 dans la première livraison RVB vérifiée ; le FD-9 du poste est piloté par FD-S2w en réseau). `Audit-MYIRO/retroanalyse/` contient les désassemblages Capstone produits par `Audit-MYIRO/outils/desassemble.py`.

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

- `crates/colorimetrie` : spectre → XYZ → Lab (D50, 2°), LCH, ΔE00, ΔC, ΔH ; contrat partagé avec le futur RIP (ADR 0004), sans Windows ni ponts. Ses tests `sharma.rs` et `mesures_archivees.rs` lisent des données locales non versionnées : ils sont `#[ignore]` et se lancent par `cargo test -p colorimetrie -- --ignored` (variables `SHARMA_CIEDE2000`, `MESURES_ARCHIVEES`).
- `crates/fdx-sys` : liste blanche des exports de `FDXSDK.dll` et formes binaires ; ne charge jamais la DLL.
- `crates/fd9-sys` : même rôle pour `FD9SDK.dll` (six exports : version, détection, connexion ; fiches `docs/abi/FD9_*`).
- `crates/jeu-validation` : jeu « données brutes → spectres » tiré des mesures archivées (outil `extraire-jeu`, identifiants retirés) et banc de comparaison d'un calcul candidat (ADR 0006) ; indépendant de Windows.
- `crates/pont-protocole` : messages JSON entre l'application et les ponts (requêtes, réponses, provenance) ; indépendant de Windows.
- `crates/pont-myiro1` : session (paliers, plafond, journal), adapter `FdxDll`, boucle du protocole et exécutable `pont-myiro1 --dll <FDXSDK.dll> [--plafond <palier>]`.
- `crates/pont-fd9` : même rôle pour le FD-9 (trait `SdkFd9`, adapter `Fd9Dll`, exécutable `pont-fd9 --dll <FD9SDK.dll> [--plafond version|detection]`) ; paliers version et détection seulement. La DLL de FD-S2w est 32 bits : `cargo build -p pont-fd9 --target i686-pc-windows-msvc`.
- `crates/profilage` : calcul d'un profil ICC à partir d'une mesure (ADR 0002) : plages contrôlées (`preparer_plages`, rien de deviné), fichier `.ti3`, trait `MoteurProfil` avec `ArgyllCms` (`colprof` puis `profcheck -k` ; `targen` pour les plages d'une mire MYIRO-1 ; en processus externe, version figée `VERSION_ARGYLL` 3.5.0, toute autre refusée) et `MoteurSimule` pour les tests, trait `MoteurGamut` (limite du gamut d'un profil par `iccgamut -v -ia -pl`, `MoteurGamutSimule` pour les tests) ; indépendante de Tauri, de Windows et des ponts. ArgyllCMS est cherché dans `argyll/` à côté de l'exécutable, puis deux niveaux plus haut (racine du dépôt depuis `target/debug/`) ; un dossier sans `bin/colprof`, `bin/profcheck` et `ref/sRGB.icm` est dit introuvable. Ses tests `tests/argyll.rs` et le test ignoré de `tests/limite.rs` lancent le vrai ArgyllCMS : `#[ignore]`, variable `ARGYLL` ou dossier `argyll/` du dépôt (le dossier principal pour un worktree). Trait `MoteurVerification` (`ArgyllCms` par `xicclu`, `MoteurVerificationSimule`) : interroge un profil pour sa vérification ; sans `xicclu`, `ErreurMoteur::OutilManquant`.
- `crates/gamut` : comparaison de profils ICC RVB (ticket #71, contrat dans l'ADR 0004) : lecture de la table A2B1 `mft2` en colorimétrie absolue (`ProfilIcc`, refus nommés `ErreurIcc`), surface du cube RVB et volume, chroma par teinte et coupes (approchées), couleurs de base, chemins, écarts ΔE00 par zone entre deux profils, fidélité, répétabilité, « mêmes mesures », lecture du `.gam` d'`iccgamut` ; `profil_essai` fabrique un profil synthétique pour les tests. Pour une image (ticket #80) : table inverse B2A1 et aller-retour colorimétrique relatif (`ProfilIcc::reproduire`), profil de l'image (`ProfilSource` : matrice et courbes, gris, `mft2` A2B1 puis A2B0, sRVB sans profil), écarts de chaque pixel (`analyser_pixels`), familles de teinte (`familles_teinte`). Ne dépend que de `colorimetrie` et de `serde`. Ses tests `tests/reference.rs` et `tests/inverse.rs` (concordance avec `xicclu`) lisent des profils de l'atelier non versionnés : `#[ignore]`, variable `PROFILS_COMPARAISON` ; ceux de `tests/source.rs` lisent `ref/sRGB.icm` et `ref/DisplayP3.icm` d'ArgyllCMS : `#[ignore]`, variable `ARGYLL` ou dossier `argyll/` du dépôt.
- `crates/photo` : ouvrir une photo RVB (JPEG, PNG, TIFF) en mémoire, sans rien écrire sur le disque (ticket #80) : profil intégré tel quel (`profil_illisible` s'il est présent mais non lu, jamais pris pour du sRVB), orientation Exif, image d'analyse d'au plus 2 048 pixels de côté (un pixel sur N, sans mélange) ; CMJN et images de plus de 120 millions de pixels refusés (`RefusPhoto`). Ne dépend que de `image` 0.25 (JPEG, PNG, TIFF).
- `crates/mires` : mires de profilage RVB (ticket #33), indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS. FD-9 : la mire RGB-2520 reprise telle quelle (`lire_mire_fd9` : empreinte, géométrie 42 × 60 plages, valeurs lues au centre des plages ; jamais redessinée). MYIRO-1 : gabarit des bandes tiré du manuel (`Gabarit`), lecture du `.ti1` de `targen`, mise en page avec contraste entre voisines (`mettre_en_page`, `verifier`, empreinte), pages dessinées avec texte et QR code et écrites en TIFF RVB PackBits sans profil ICC. Son test `tests/rgb2520.rs` lit la vraie mire : `#[ignore]`, variable `MIRE_FD9` ou dossier `mire-fd9/` du dépôt (le dossier principal pour un worktree).
- `crates/verification` : vérification d'un profil sur une seconde mire (ticket #41, contrat dans l'ADR 0004) : écarts de fidélité et de prédiction plage par plage, hors gamut, statistiques (moyenne, 95e centile, maximum, pires plages), tolérances et conclusion ; mire de vérification FD-9 écrite en XML de FD-S2w. Dépend seulement de `colorimetrie` et `serde`. Son test `tests/mire_fd9.rs` `--ignored` compare à un export local de la RGB-2520 (variable `MIRE_RGB2520`).
- `app` : application de bureau Tauri 2 (seule crate qui dépend de Tauri), catalogue de textes `textes.rs`, page dans `app/interface/` ; lancer avec `cargo run -p app` (`-- --langue en` pour l'anglais). Modules `instrument` et `pont` (sans Tauri) : l'application trouve seule `FDXSDK.dll` (dans l'ordre : dossier choisi, DLL embarquées `sdk/x64` et `sdk/x86` à côté de l'exécutable, `SDK/` du dépôt en développement, DLL retenue, emplacements connus Ergosoft, EIZO, MY-CT1), lit son architecture et lance le pont de la même architecture. Ponts cherchés à côté de l'exécutable (`pont-myiro1-x64.exe`, `pont-myiro1-x86.exe`), puis en développement dans `target/debug/` et `target/i686-pc-windows-msvc/debug/` : compiler avant `cargo run -p app` avec `cargo build -p pont-myiro1`, `cargo build -p pont-myiro1 --target i686-pc-windows-msvc` et, pour le FD-9 (DLL de FD-S2w en 32 bits), `cargo build -p pont-fd9 --target i686-pc-windows-msvc`. À la première détection FD-9 vide, l'application demande (fenêtre de contrôle de compte) sa règle de pare-feu « myiro-libre - FD-9 - detection (UDP 49152) » par le trait `PareFeu` (`app/src/instrument/parefeu.rs`) ; les tests passent par `PareFeuSimule` et ne lancent jamais `netsh` ni d'élévation (ADR 0005, ticket #47). Module `parc` (arbre imprimante → papier → résolution, ranger et exporter un profil ICC ; méthodes de `BibliothequeOuverte` testées sans Tauri dans `app/tests/parc.rs`) et module `installation` (trait `InstallationProfils` : `InstallationWindows` par `mscms`, `InstallationSimulee` pour les tests, `app/tests/dossier_couleurs.rs` ; aucun test n'installe dans le vrai Windows). Module `profiler` (calcul d'un profil RVB depuis une mesure importée, rangé « non vérifié » avec sa recette, sa précision, sa commande et son journal ; testé avec `MoteurSimule` dans `app/tests/profiler.rs`, sans ArgyllCMS). Module `comparer` (un à quatre profils de la bibliothèque ou fichiers `.icc` ouverts, gardés en mémoire et jamais rangés ; mesures embarquées par `cgats::lire_sans_condition` ; limite du gamut par `MoteurGamut` ; testé avec `MoteurGamutSimule` dans `app/tests/comparer.rs` ; sous-module `comparer/image` : une photo ouverte pour la séance, jamais rangée, analysée face à chaque profil hors des verrous, une seule analyse vivante et annulable, pixels et codes d'écart en binaire, testé dans `app/tests/comparer_image.rs` ; pipette `image_pixel` : un clic ramené au pixel de l'image d'analyse, sa couleur imprimée par chaque profil, testée dans `app/tests/pipette_image.rs`). Module `verifier` (vérifier un profil sur une seconde mire importée, jeux de tolérances versionnés, recalcul sans remesure, mire de vérification FD-9 rangée une fois dans la table `mires`, organisation 7 ; `app/tests/verifier.rs`) et module `rapport_verification` (rapport HTML autonome de la vérification, régénéré depuis les données ; `app/tests/rapport_verification.rs` ; à reprendre par la crate `rapport` du ticket #89), testés avec `MoteurVerificationSimule`, banc commun `app/tests/banc_verification/`. Module `mires` (préparer une mire : RGB-2520 copiée octet pour octet, ou pages MYIRO-1 en TIFF, dans `Documents\myiro-libre\Mires` ; mire et tirage rangés dans la bibliothèque, organisation 6 ; testé avec `MoteurSimule` dans `app/tests/mires.rs`) ; la mire FD-9 est cherchée dans `mire-fd9/` à côté de l'exécutable, puis deux niveaux plus haut. Page : `parc.js` (arbre), `arbre.js` (pièces de l'arbre), `reglage.js` (vue d'une résolution), `calcul-profil.js` (section « Calculer un profil » de la vue d'une résolution et provenance d'un profil calculé), `mire.js` (section « Préparer une mire »), `verification.js`, `resultat-verification.js` et `tolerances.js` (section « Vérifier le profil », son résultat et l'éditeur des tolérances), `bibliotheque.js` (feuille des mesures), `comparer.js` (vue « Comparer », grandes parties repliables par `accordeon.js`, testé dans `app/tests/page_accordeon.rs`), `comparer-image.js` (section « Image » de la comparaison : surimpression hors gamut, seuil d'alerte, chiffres par profil), `nuage-image.js` (nuage de couleurs de l'image, testé dans `app/tests/page_nuage.rs`), `pastilles-image.js` (pastilles des couleurs hors gamut et pipette, dans le panneau à droite de l'image, testé dans `app/tests/page_pastilles.rs`), `description-image.js` (nom et profil de l'image), `verdict-image.js` (part de l'image imprimable, au-dessus de la photo, testé dans `app/tests/page_verdict.rs`), `reglages-image.js` (réglages de la surimpression, dont la case « Compensation du point noir », et l'état de l'analyse à sa place gardée) et `image-agrandie.js` (image en grand dans une fenêtre modale), ces cinq chargés avant `comparer-image.js` et testés aussi dans `app/tests/page_image_disposition.rs`, `vue3d.js` (gamut en volume, Canvas 2D maison, triangles triés par profondeur), `vue3d-axes.js` (axes et blancs du papier, chargé avant `vue3d.js`) et `graphiques.js` (graphiques SVG de chroma, de coupe et de chemin). Un exécutable de test 32 bits dont le nom contient « install » exige des droits d'administrateur sous Windows : ne pas nommer ainsi un fichier de `tests/`.

Installateur Windows (NSIS, ponts 64 et 32 bits inclus) :

```
python outils/preparer_installateur.py                  # compile les deux ponts, écrit app/tauri.installateur.json
cargo tauri build --config app/tauri.installateur.json  # tauri-cli 2
python outils/test_preparer_installateur.py             # embarquement d'ArgyllCMS, sur dossiers factices
```

**Si `SDK/` existe, l'installateur embarque les DLL du fabricant : il est strictement local, à ne jamais publier (Releases GitHub comprises).** Sans `SDK/`, il n'en contient aucune. `verifier_docs.py` échoue si un fichier `.dll` est suivi ou non ignoré par git.

Si `argyll/` existe, `preparer_installateur.py` lit la version dans l'aide de `colprof` (seule la 3.5.0 passe) et embarque sous `argyll/` `bin/colprof.exe`, `bin/profcheck.exe`, `bin/xicclu.exe`, `bin/targen.exe`, `bin/iccgamut.exe`, `ref/sRGB.icm` et tous les textes de licence présents parmi `ARGYLL_LICENCES` (sous leur nom réel, casse ignorée) ; un ArgyllCMS incomplet, sans licence ou d'une autre version arrête le script. Sans `argyll/`, il le dit en une ligne et continue. ArgyllCMS est sous AGPL-3.0 : chaque publication d'un installateur qui l'embarque fournit les sources de la version incluse, ou leur lien précis (ADR 0002). Si `mire-fd9/` existe, la mire RGB-2520 y est contrôlée par son empreinte et embarquée telle quelle sous `mire-fd9/` ; une autre empreinte arrête le script, et sans le dossier il le dit en une ligne et continue.

```
cargo test                                     # tous les tests, contre un instrument simulé
cargo clippy --all-targets -- -D warnings
cargo test --target i686-pc-windows-msvc       # même chose en 32 bits (DLL de MY-CT1)
cargo test -p pont-myiro1 --test dll -- --ignored palier_version   # appels réels à la DLL du poste
cargo test -p pont-fd9 --target i686-pc-windows-msvc --test dll -- --ignored palier_version --nocapture    # FD-9 : charge la DLL de FD-S2w
cargo test -p pont-fd9 --target i686-pc-windows-msvc --test dll -- --ignored palier_detection --nocapture  # FD-9 : détection USB et réseau
cargo test -p profilage --test argyll -- --ignored --nocapture   # profil calculé par le vrai ArgyllCMS 3.5.0, mire synthétique ; plages choisies par targen
cargo test -p profilage --test limite -- --ignored --nocapture   # limite du gamut par le vrai iccgamut, profil d'essai linéaire
cargo test -p mires --test rgb2520 -- --ignored --nocapture      # lit la vraie mire RGB-2520 (MIRE_FD9 ou mire-fd9/)
```

Les tests `--ignored` de `crates/pont-myiro1/tests/dll.rs` parlent au vrai MYIRO-1 et, à partir de l'étalonnage, demandent des gestes à l'opérateur : ne les lancer qu'avec son accord, palier par palier. Leurs sorties (mesures) vont dans `Archivage/donnees/`, local et non versionné. Ceux de `crates/pont-fd9/tests/dll.rs` aussi, FD-S2w fermé ; `FD9SDK_DLL` désigne une autre DLL, et leurs sorties (adresse, identifiant réels) restent à l'écran.

Intégration continue (`.github/workflows/ci.yml`, GitHub Actions) à chaque PR et à chaque poussée sur `main` et `dev`. Job `windows` : `cargo fmt --all --check`, clippy, `cargo test` en 64 puis 32 bits. Job `linux` : fmt, clippy et tests limités aux crates qui doivent rester indépendantes de Windows (liste explicite `CRATES` dans le workflow, aujourd'hui `pont-protocole`, `jeu-validation`, `colorimetrie`, `bibliotheque`, `cgats`, `gamut`, `photo`, `profilage`, `mires` et `verification`). Les deux jobs lancent `python outils/test_preparer_installateur.py`, `python outils/test_verifier_docs.py` et `python outils/verifier_docs.py`. `main` exige ces deux contrôles verts : ne pas renommer les jobs. La CI ne couvre pas la DLL ni l'instrument (tests `--ignored`, aucun fichier Konica Minolta sur GitHub), ni les écrans, ni les mots interdits (la liste `.mots-interdits.local` reste sur le poste ; sans elle, le contrôle est sauté et annoncé).

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
- `python outils/verifier_docs.py` contrôle les mots interdits (liste locale `.mots-interdits.local`, non versionnée), les liens cassés, l'absence de tout fichier `.dll` suivi, et rappelle le journal du jour. `python outils/verifier_docs.py --installer` le branche sur les commits (hooks locaux).
- `docs/sources/` est local et ignoré par git (documents de tiers) : ne jamais le versionner, ne jamais le citer par nom dans un fichier suivi. Les analyses de logiciels concurrents n'y nomment pas le produit dans les fichiers publiés : écrire « un logiciel concurrent ».
- Les références externes (normes, projets libres) vont dans `docs/references/` sous forme de liens et de résumés, jamais de copies de contenu protégé.
