# Mesurer une feuille au FD-9 — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'imprimeur glisse la mire RGB-2520 imprimée dans le FD-9, et l'application la mesure. Elle pilote elle-même l'instrument, sans FD-S2w ni MYIROtools : connexion, scan de la feuille, lecture du QR code, reconnaissance des 2 520 plages, mesure, puis spectres M0, M1 et M2 de chaque plage. La mesure entre dans la bibliothèque comme une mesure importée de FD-S2w, avec sa provenance « pont FD-9 » : le calcul de profil du plan 3 s'en sert sans changement.

**Architecture:** le pont `pont-fd9` (32 bits, `FD9SDK.dll` de FD-S2w) reçoit trois nouveaux paliers : **Connexion**, **Prénumérisation** (scan, QR et reconnaissance, sans mesure) et **Feuille**. Ce sont les seuls que le FD-9 déclare au-delà de la détection. Il n'a ni étalonnage par l'utilisateur ni mesure ponctuelle (aucun export pour cela dans le SDK). La session suit la séquence de MYIROtools, le logiciel du fabricant qui imprime et mesure cette mire : rappel d'événements, connexion, capture, reconnaissance « Combine », mesure, lecture plage par plage, fin. Les événements de la DLL sont traduits en états explicites. La crate `fd9-sys` reçoit les structures, avec leur disposition en 32 et 64 bits, et la liste blanche élargie. `pont-protocole` reçoit la description d'une feuille (`DefinitionFeuille`) et deux requêtes. L'application envoie la définition de la RGB-2520, lit les valeurs RVB de la mire (crate `mires`, plan 4), écrit un CGATS myiro-libre avec RVB et spectres, puis l'importe dans la résolution choisie.

**Tech Stack:** Rust 2021, Tauri 2, `libloading` (déjà là), crates `fd9-sys`, `pont-protocole`, `cgats`, `bibliotheque`, `mires` (plan 4). Aucune nouvelle dépendance externe.

**Spec:** ticket #34 (critères d'acceptation) ; `docs/specifications/poste-travail-couleur.md` ; fiches `docs/abi/FD9_*` ; manuel FD-S2w de la collecte (`Audit-MYIRO/collecte/FD-S2w/Document/Manual/manual_fra.pdf`, local, « Conditions de la charte », p. 35–37) ; étude statique du 28 octobre 2026, notes locales `Audit-MYIRO/retroanalyse/logiciels/fd9-feuille.md` (dossier ignoré par git, adresses et pseudo-code).

**Dépend de :** plan 4 (`docs/superpowers/plans/2026-10-23-mires-de-profilage.md`) **fusionné dans `dev`**. Ce plan reprend sa crate `mires` (`crates/mires/src/rgb2520.rs` : `RGB2520_FICHIER`, `RGB2520_EMPREINTE`, `RGB2520_NOM`, `GeometrieFd9`, `RGB2520_GEOMETRIE`, `MireFd9 { octets, empreinte, geometrie, valeurs }`, `lire_mire_fd9`, `lire_mire_fd9_sans_empreinte`, `ErreurMire`) et `chercher_mire_fd9(exe)` de l'application. **Ce plan ne modifie pas `crates/mires`.**

## Feuille de route (28 octobre 2026)

1. Bibliothèque : parc et profils (fait, #66).
2. Nouveau cadre d'interface (fait, #68).
3. Profil RVB depuis une mesure importée (fait, #40 première partie, PR #72).
4. Mires de profilage (#33) : plan 4, en cours d'exécution (branche `ticket/33-mires`).
5. Comparer des profils (#71) : plan 5, en cours d'exécution.
6. **Mesurer une feuille au FD-9 (#34)** : ce plan.
7. Guidage, reprise et moyenne (#35), vérification du profil et parcours en 4 étapes (#41).
8. Séchage ; 9. reprise de l'existant.

Reportés au plan 7 : plusieurs feuilles par tirage, reprise d'une feuille mal lue, moyenne de deux lectures, guidage pas à pas, mires FD-S2w multi-zones et définitions `.cdx` quelconques. Ce plan ne mesure **que la RGB-2520**, d'une seule feuille.

## Ce que l'étude statique a établi

Niveaux : **confirmé** (lu dans le code), **supposé**, **à vérifier sur l'instrument**. Détails et adresses : notes locales `fd9-feuille.md` ; fiches publiques écrites à la tâche 1.

- **La référence est MYIROtools**, pas FD-S2w (confirmé) : c'est lui qui imprime la RGB-2520 (QR `FD-9:011M:RGB-2520 - 1P MY9*1+`) et qui la mesure. Il n'utilise ni travail (`FD9_RegisterJob`), ni `FD9_MatchingChart`, ni `FD9_ClippingChart`.
- **Séquence** (confirmé, gestionnaire d'événements de MYIROtools) :
  1. `RegisterDeviceEventHandler` puis `Connect` (succès : 0 ou 1901) ;
  2. événement 3 (alimentation finie) : `StartCapture` ;
  3. événement 5 (capture finie) : `GetCaptureResult` deux fois, pour la taille puis l'image et le QR ;
  4. `StartPatchRecognition` en mode 4, **COMBINE** (reconnaissance automatique et comparaison aux coordonnées de la définition) ;
  5. événement 7 : `GetPatchRecognitionResult2`, puis **comparaison du nombre de zones et de plages à la définition**, puis `GetMeasurePoint` et `StartMeasurement2(mode 1, avis 1)` ;
  6. événement 11 : nombre de plages déjà mesurées ;
  7. événement 9 : `StopMeasurement(vrai)` (refusé par le SDK à cet état, retour ignoré par MYIROtools), puis `ExitMeasurement(0, NULL)` ;
  8. erreurs 101–103, 201–206, 301 : `ExitMeasurement(0, NULL)` ; un appel refusé en cours de route : `StopMeasurement(faux)`.
- **1901 = recertification due** (confirmé : MYIROtools journalise « Instrument should be recertified »). La connexion est faite. Ce n'est pas un étalonnage demandé à l'utilisateur. Le nom `CODE_CALIBRATION_PERIODIQUE_DUE` de `fd9-sys` est trompeur : à renommer à la tâche 2.
- **QR** (confirmé dans `FD9SDK.dll`) : le texte est découpé sur `:`. Avec une version qui commence par `011`, on obtient `deviceName` (+0, 4 octets), `version` (+4, 4 octets) et un **texte libre à +0x294** (80 caractères au plus). `tFD9_BarcodeData` fait 1 172 octets. Pour la RGB-2520, le texte attendu est `RGB-2520 - 1P MY9*1+` : supposé, à vérifier à l'essai E3. C'est l'application qui le relit, pas le SDK.
- **Définition de la RGB-2520** (confirmé, définition de mire de MYIROtools) : feuille 27 940 × 41 660, une zone de 60 rangées × 42 colonnes, origine (1 349 ; 2 300), plage 601 × 601, pas 601. Unité : 1/100 mm. Ordre : rangées de haut en bas, de gauche à droite dans chaque rangée. Le SDK contrôle `gauche + pas × colonnes ≤ largeur` (même unité, confirmé). FD-S2w recopie les définitions champ par champ, sans conversion (confirmé). **Le fichier de définition (brouillé) et son contenu décodé ne sont jamais versionnés** : seuls ces nombres le sont, dans une constante.
- **Disposition x86 de `tFD9_ChartCoordinate`** (confirmé) :
  - taille 0xd70 (0xd78 en x64) ;
  - en-tête : largeur, hauteur, zones (≤ 50), plages (≤ 15 000) ;
  - zones à +0x10 : `{rangées, colonnes, plages}` × 50 ;
  - géométries à +0x268 : `{gauche, haut, largeur, hauteur, pas X, pas Y}` × 50 ;
  - modèle présent à +0x718 ; image modèle à +0xd60.
- **Couleurs des plages** (`tFD9_PatchColors`) : FD-S2w convertit les RVB avec un réglage couleur dont l'échelle n'est pas établie. **Ce plan n'en passe pas** (pointeur nul). COMBINE ne les exige pas : contrôle du SDK confirmé ; efficacité à vérifier à l'essai E3.
- **Lecture d'une plage** (confirmé) :
  - `GetData(&position_sortie, données, &longueur, &{index, −1, −1})` ;
  - les données brutes (option 3 = 0x0c) font 152 valeurs, soit **4 blocs de 38** ;
  - les autres types se calculent depuis ces 152 valeurs, avec le même calcul que `CalculateData` (entrée de 36 ou 152 valeurs).
- **`FD9_SetOption` n'écrit rien dans l'instrument** (confirmé) : elle range quatre réglages de calcul dans la DLL (clé 0 < 16, clé 1 < 16, clé 2 < 2, clé 3 < 15). C'est la seule `FD9_Set*` admise par ce plan ; sa fiche le prouve.

**Révision du 28 octobre 2026 (fiches de la tâche 1)** : la lecture des fonctions de travail du SDK a établi les états où `FD9_StopMeasurement` (1 à 8) et `FD9_ExitMeasurement` (9, 10, erreurs) sont admis. Les tâches 5 et 6 en tiennent compte : un arrêt attend l'événement 10 avant de sortir, et une mesure finie sort sans arrêter. Autres faits des fiches : état du SDK à +0x10 de `FD9_GetStatus`, spectre de 36 valeurs confirmé, `FD9_SetOption` sans contrôle de pointeur nul.

## Global Constraints

- Travail dans le worktree `.claude/worktrees/ticket-34`, branche `ticket/34-feuille-fd9` issue de `dev`. PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/34-feuille-fd9 --reference dev`. **Rebaser sur `dev` une fois le plan 4 fusionné**, avant la tâche 8.
- **Paliers et plafond** : le pont FD-9 déclare Version, Détection, Connexion, Prénumérisation, Feuille, dans cet ordre. Le plafond est fixé au lancement (`--plafond`). Un palier au-delà est refusé **sans appel à la DLL**. L'application ne lance le pont au plafond Feuille que pour une mesure demandée par l'opérateur ; sinon, son plafond FD-9 reste Connexion.
- **Liste blanche** (`fd9_sys::EXPORTS_AUTORISES`). Après ce plan, elle contient : `FD9_GetLastError`, `FD9_GetDeviceList`, `FD9_RegisterDeviceEventHandler`, `FD9_Connect`, `FD9_GetSystemInfo`, `FD9_GetStatus`, `FD9_Disconnect`, `FD9_StartCapture`, `FD9_GetCaptureResult`, `FD9_StartPatchRecognition`, `FD9_GetPatchRecognitionResult2`, `FD9_GetMeasurePoint`, `FD9_StartMeasurement2`, `FD9_GetData`, `FD9_SetOption`, `FD9_CalculateData`, `FD9_StopMeasurement`, `FD9_ExitMeasurement`, `FD9_GetPremeasurementError`.
  - Restent **exclus** : tous les `JIG_*` ; `FD9_SetNetworkSetting`, `FD9_RegisterUserIlluminant`, `FD9_TAConnect`, `FD9_RegisterJob`, `FD9_DeleteJob`, `FD9_GetJobList`, `FD9_MatchingChart`, `FD9_ClippingChart`, `FD9_GetChartImage`, `FD9_RegisterDisplayImage`, `FD9_CreateBarcodeImage`, `FD9_CreateChartImage`, `FD9_DeleteChartImage`, `FD9_WriteBmpFile`, `FD9_StartMeasurement` (v1), `FD9_StartPreMeasurement`, `FD9_GetPatchRecogResult`, `FD9_GetNetworkSetting`, `FD9_GetOption`.
  - On n'ajoute un export qu'avec sa fiche `docs/abi/`.
- **Conventions d'appel** : `extern "system"` (`__stdcall` en x86) pour tous les exports, sauf `FD9_GetPremeasurementError`, en `extern "C"`. Le rappel d'événements est appelé par la DLL en `__cdecl` : `unsafe extern "C" fn(u32, u32, u32, u32)`. **Les scalaires d'entrée passent par pointeur.**
- **Le rappel ne parle jamais à la DLL** : il pose l'événement dans un canal et rend la main. Toute la séquence se joue dans le fil de la session. Un rappel reçu sans session active est compté et journalisé, jamais perdu en silence.
- **Aucun état prêt inventé** : la session n'avance que sur un événement reçu ou un code de retour lu. Un délai dépassé, une perte de liaison ou une fermeture non confirmée sont rendus comme tels (`Delai`, `InstrumentPerdu`, `FermetureIncertaine`).
- FD-S2w et MYIROtools **fermés** pour tout essai réel : le SDK arbitre l'accès à l'instrument entre plusieurs clients (`WAIT_NO_AUTHORITY`).
- **Jamais d'adresse IP, de MAC ni de n° de série réels** dans un fichier versionné (tests, fiches, journal). Les sorties des tests `--ignored` restent à l'écran ; les mesures réelles vont dans `Archivage/donnees/` (local).
- Ne jamais versionner la définition de mire de MYIROtools (décodée ou non), ni sa clé de brouillage. Ne pas citer `docs/sources/`. Ne pas nommer le logiciel concurrent.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57). Fichiers source nouveaux sous 400 lignes. Tout texte de l'interface passe par `app/src/textes.rs` (français et anglais).
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- **Séquence de MYIROtools, sans travail.** `FD9_RegisterJob` sert aux mires FD-S2w de plusieurs pages ; MYIROtools mesure la RGB-2520 sans lui. Moins d'appels, et aucun travail à supprimer si la liaison se perd. Supposé : la feuille part seule à l'insertion (état 1 → 2). C'est vérifié à l'essai E3. Si ce n'est pas le cas, un plan suivant ajoute `RegisterJob` et `DeleteJob` avec leurs fiches.
- **COMBINE sans couleurs, repli COORDINATE.** Si l'essai E3 montre que COMBINE sans `tFD9_PatchColors` ne reconnaît pas la mire, le pont essaie COORDINATE (mode 2, coordonnées seules), selon le réglage `--reconnaissance coordinate` du pont. Il ne devine jamais une échelle de couleurs.
- **Reconnaissance contrôlée deux fois** :
  - zones et plages reconnues égales à la définition (comme MYIROtools) ;
  - **origine et pas reconnus à 3 mm près** de la définition (300 en 1/100 mm), pour attraper une autre mire de même taille.
  Un écart est refusé en le disant (`MireNonReconnue`), et rien n'est mesuré.
- **QR contrôlé, mais pas obligatoire.** Un QR lu doit avoir `deviceName = "FD-9"` et un texte qui commence par `RGB-2520 - 1P MY9`. Un autre texte refuse la feuille (`AutreMire { texte }`). Un QR absent ou illisible n'arrête pas : la reconnaissance tranche, et la provenance dit « QR non lu ».
- **Plages lues en fin de mesure.** MYIROtools lit les plages au fil des avis (événement 11). Ce plan les lit **toutes à l'événement 9**, avant `ExitMeasurement` : une seule boucle, sans course entre deux fils. Si l'essai E4 montre que les données ne sont plus là à l'événement 9, on passe à la lecture au fil des avis (tâche 6, variante prévue).
- **Spectres M0, M1, M2 calculés depuis les 152 données brutes.** On lit une fois les données brutes (`SetOption(3, 0x0c)`, `GetData` sur 152 valeurs). Puis, pour chaque condition, on pose `SetOption(0, …)` et on appelle `CalculateData` :
  - conditions : M0 = 0, M1 = 2, M2 = 15 (valeurs relevées dans FD-S2w ; supposé) ;
  - spectre (`SetOption(3, 0x0b)`) : 36 valeurs (longueur confirmée par la fiche `FD9_CalculateData`), 380–730 nm au pas de 10 (supposé) ;
  - Lab : `SetOption(3, 0)`, avec D50 (`SetOption(1, 2)`) et 2° (`SetOption(2, 0)`).

  Les conditions restent « supposées » dans la provenance tant que l'essai E4 ne les a pas comparées à l'export de MYIROtools du 26 octobre (`Archivage/donnees/mesures-importees/RGB-2520_00025.txt`, local).
- **Association aux valeurs nominales vérifiée.** La plage n° i de la mesure est la plage n° i de la mire : rangée `i / 42`, colonne `i % 42`, comptées depuis 0. On le vérifie par la position que rend `GetData` (zone 0, rangée, colonne), plage par plage. Un écart refuse la mesure entière.
- **La mesure entre dans la bibliothèque comme un CGATS myiro-libre importé.** Elle a ses colonnes RVB de 0 à 100, avec une échelle confirmée pour un fichier écrit par myiro-libre, et sa provenance « pont FD-9 » dans l'en-tête. Ainsi, le calcul du plan 3 et la page des mesures marchent sans changement. Le JSON complet de la mesure (`myiro-libre/mesure/1`, données brutes comprises) est gardé à côté, dans le dossier de la bibliothèque. Une table « mesures de pont en feuille » attendra le plan 7.
- **N° de série** : `Identite.numero_serie` est un `u32`. Le FD-9 rend 8 chiffres écrits en hexadécimal, supposés en BCD. Le pont les lit en décimal ; un texte non numérique refuse la connexion (`ReponseInattendue`), sans rien inventer. Le texte brut reste dans `brute_hex`.
- **Délai d'attente de la feuille : 120 s** après l'armement. Au-delà, on arrête (`StopMeasurement(faux)`, événement 10, `ExitMeasurement` : voir la tâche 5) et on rend `FeuilleAbsente`. Le délai se règle par `--attente-feuille <s>`, pour les essais.
- **Journal du SDK** : le pont n'écrit jamais `DebugSDK/` de lui-même. L'opérateur peut créer ce dossier dans le dossier courant du pont pour un essai (fiche `FD9SDK.md`).

## Review Focus

1. **Plafond respecté** : pont lancé au plafond Connexion et requête `mesurer_feuille` ; refus `palier_non_autorise`, **aucun appel** à la DLL (journal de la session vide après la connexion). → tests `la_feuille_est_refusee_sous_le_plafond_sans_appel` (tâche 5) et `le_plafond_par_defaut_de_l_application_est_connexion` (tâche 8).
2. **Événement d'erreur pendant la mesure** (bourrage 102, liaison 103, couvercle 101) : la session sort par `ExitMeasurement`, rend l'erreur nommée, ne rend aucune plage, et reste fermable. → tests `un_bourrage_rend_une_erreur_et_aucune_plage`, `une_perte_de_liaison_rend_instrument_perdu` (tâche 7).
3. **Autre mire, ou mire de même taille décalée** : refus avant mesure, `StartMeasurement2` jamais appelé. → tests `un_autre_qr_refuse_la_feuille`, `une_geometrie_decalee_refuse_la_feuille` (tâche 7).
4. **Rappel d'événements tardif** (après la fermeture, ou pendant une autre requête) : compté, journalisé, sans effet. → test `un_rappel_apres_fermeture_est_ignore_et_compte` (tâche 4).
5. **Positions rendues par `GetData` dans le désordre** : la mesure est refusée, rien n'est rangé. → test `une_position_inattendue_refuse_la_mesure` (tâche 6).
6. **Disposition des structures en 32 bits** : tailles et positions vérifiées par des tests compilés en x86 et en x64. → tests de la tâche 2, lancés aussi avec `--target i686-pc-windows-msvc`.

---

### Tâche 0 : Préparer le chantier

**Files:** aucun.

- [ ] **Step 1 : Contexte** — Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork/.claude/worktrees/ticket-34 --branch ticket/34-feuille-fd9 --reference dev` puis `git status --short`. Expected: contexte conforme, rien d'autre que ce plan.
- [ ] **Step 2 : Plan 4** — Run: `git log --oneline dev -- crates/mires/src/rgb2520.rs`. Si rien : les tâches 1 à 7 peuvent avancer (elles ne touchent pas `mires`) ; la tâche 8 attend la fusion du plan 4 et un `git rebase dev`.
- [ ] **Step 3 : Ponts compilés** — Run: `cargo build -p pont-fd9 --target i686-pc-windows-msvc` puis `cargo test -p pont-fd9 --target i686-pc-windows-msvc`. Expected: vert (état de départ).

### Tâche 1 : Fiches d'ABI de la mesure d'une feuille

**Files:**
- Create: `docs/abi/FD9_GetStatus.md`, `FD9_StartCapture.md`, `FD9_GetCaptureResult.md`, `FD9_StartPatchRecognition.md`, `FD9_GetPatchRecognitionResult2.md`, `FD9_GetMeasurePoint.md`, `FD9_StartMeasurement2.md`, `FD9_GetData.md`, `FD9_SetOption.md`, `FD9_CalculateData.md`, `FD9_StopMeasurement.md`, `FD9_ExitMeasurement.md`, `FD9_GetPremeasurementError.md`
- Modify: `docs/abi/FD9SDK.md` (liste des fiches, codes, 1901 = recertification, séquence d'une feuille, états du rappel), `docs/abi/FD9_Connect.md` (1901), `docs/abi/FD9_RegisterDeviceEventHandler.md` (table des états et action du pont)

Chaque fiche suit le modèle de `FD9_GetSystemInfo.md` : En clair, Signature, Ce qui est confirmé (adresses VA `fd9-x86`), Contenu des structures, Propriété de la mémoire, Ce qui est supposé, Pour le pont, À vérifier sur l'instrument, Preuves (locales). Elles reprennent les faits de `Audit-MYIRO/retroanalyse/logiciels/fd9-feuille.md` et de `fd-s2w.md` § 4, **sans recopier de code du fabricant**, sans adresse IP, MAC ni n° de série. La définition de mire de MYIROtools y est citée par ses nombres, jamais par son contenu brut ni sa clé.

- [ ] **Step 1 : Écrire les 13 fiches.** Points obligatoires :
  - `FD9_GetCaptureResult` : 1er appel avec image nulle, qui rend la taille ; 2e avec l'image. Résultats 0, 1 et 2 du QR. Disposition de `tFD9_BarcodeData` (1 172 octets), format `011` et ancien format. `bpp` forcé à 8.
  - `FD9_StartPatchRecognition` : disposition x86 et x64 de `tFD9_ChartCoordinate`, contrôles des zones, exigences de chaque mode, pointeur de couleurs nul admis en COMBINE.
  - `FD9_GetData` : `{index, −1, −1}`, bornes, données brutes en 4 × 38, position rendue.
  - `FD9_SetOption` : **aucun échange avec l'instrument**, les quatre clés et leurs bornes.
  - `FD9_StartMeasurement2` : exige l'état 7, `mode & 7`, sens supposé.
  - `FD9_StopMeasurement` et `FD9_ExitMeasurement` : quand MYIROtools appelle l'un ou l'autre.
- [ ] **Step 2 : `FD9SDK.md`** : section « Mesurer une feuille », avec la séquence, la table des états du rappel (0–11, 101–103, 201–206, 301) et l'action du pont pour chacun. Rendre à 1901 son sens : « recertification due ».
- [ ] **Step 3 : Vérifier** — Run: `python outils/verifier_docs.py`. Expected: OK (liens, mots interdits).
- [ ] **Step 4 : Commit** — `git commit -m "Fiches d'ABI FD-9 : mesure d'une feuille, établies par lecture statique"`

### Tâche 2 : `fd9-sys` — structures de la feuille et liste blanche

**Files:**
- Create: `crates/fd9-sys/src/feuille.rs` (structures, constantes, conversions ; sous 400 lignes)
- Modify: `crates/fd9-sys/src/lib.rs` (`mod feuille; pub use feuille::*;`, liste blanche, renommage du code 1901)
- Test: `crates/fd9-sys/tests/dispositions.rs`

**Interfaces :**

```rust
/// tFD9_ChartCoordinate : 0xd70 octets en x86, 0xd78 en x64.
#[repr(C)]
pub struct CoordonneesMire {
    pub largeur: u32,            // feuille, 1/100 mm
    pub hauteur: u32,
    pub zones: u32,              // ≤ ZONES_MAX
    pub plages: u32,             // ≤ PLAGES_MAX
    pub compte_zones: [CompteZone; 50],   // +0x10
    pub geometries: [GeometrieZone; 50],  // +0x268
    pub modele_present: u32,             // +0x718, toujours 0 ici
    pub reserve: [u8; 0xd60 - 0x71c],    // champs non décodés, à zéro
    pub modele: ImageFd9,                // +0xd60
}
#[repr(C)] pub struct CompteZone { pub rangees: u32, pub colonnes: u32, pub plages: u32 }
#[repr(C)] pub struct GeometrieZone { pub gauche: u32, pub haut: u32, pub largeur: u32, pub hauteur: u32, pub pas_x: u32, pub pas_y: u32 }
/// tFD9_Image : 16 octets en x86, 24 en x64.
#[repr(C)] pub struct ImageFd9 { pub largeur: u32, pub hauteur: u32, pub bits: u32, pub donnees: *mut u8 }
/// tFD9_BarcodeData : 1 172 octets.
#[repr(C)] pub struct DonneesQr { pub appareil: [u8; 4], pub version: [u8; 4], pub ancien: [u8; 0x294 - 8], pub texte: [u8; 0x200] }
/// tFD9_PatchPosition : 12 octets ; (index, -1, -1) désigne la plage n° index.
#[repr(C)] pub struct PositionPlage { pub zone_ou_index: u32, pub rangee: i32, pub colonne: i32 }
/// tFD9_PatchMeasurePoint : 20 octets, contenu non décodé.
#[repr(C)] pub struct PointMesure(pub [u8; 20]);

pub const ZONES_MAX: usize = 50;
pub const PLAGES_MAX: usize = 15_000;
pub const RANGEES_MAX: u32 = 250;
pub const COLONNES_MAX: u32 = 60;
pub const DONNEES_BRUTES: usize = 152;
pub const CODE_RECERTIFICATION_DUE: i32 = 1901;   // remplace CODE_CALIBRATION_PERIODIQUE_DUE

pub enum EtatSdk { AttenteSansMain, Attente, Alimentation, AlimentationFinie, Capture, CaptureFinie,
    Reconnaissance, ReconnaissanceFinie, Mesure, MesureFinie, AnnulationFinie, AvisPlages,
    CouvercleOuvert, Bourrage, Communication, Premesure, ErreurMesure, Capteur, Calcul,
    FeuilleTropLongue, Memoire, Fatale, Inconnu(u32) }
impl EtatSdk { pub fn depuis(code: u32) -> Self; pub fn est_erreur(&self) -> bool; }

pub enum ModeReconnaissance { Combine = 4, Coordonnees = 2 }   // seuls modes admis
pub enum ResultatQr { Trouve, Absent, Illisible, Inconnu(u32) }
pub enum CleOption { Condition = 0, Illuminant = 1, Observateur = 2, TypeDonnees = 3 }
pub const DONNEES_BRUTES_TYPE: u32 = 0x0c; pub const SPECTRE_TYPE: u32 = 0x0b; pub const LAB_TYPE: u32 = 0;

impl CoordonneesMire {
    /// Zéro partout, puis la définition ; refuse > 50 zones, > 15 000 plages,
    /// rangées ou colonnes hors bornes, zone qui déborde de la feuille.
    pub fn depuis_definition(d: &pont_protocole::DefinitionFeuille) -> Result<Box<Self>, ErreurParametre>;
}
impl DonneesQr { pub fn vide() -> Box<Self>; pub fn appareil(&self) -> String; pub fn texte(&self) -> String; }
```

`fd9-sys` dépend de `pont-protocole` pour `DefinitionFeuille` (tâche 3) ; si la dépendance gêne, la conversion va dans `pont-fd9`.

- [ ] **Step 1 : Tests d'abord** (`tests/dispositions.rs`), en 32 et 64 bits avec `cfg(target_pointer_width)` :

```rust
#[test] fn coordonnees_mire_fait_0xd70_en_32_bits_et_0xd78_en_64() {}
#[test] fn les_geometries_commencent_a_0x268_et_le_modele_a_0xd60() {}   // offset_of!
#[test] fn donnees_qr_fait_1172_octets_et_le_texte_commence_a_0x294() {}
#[test] fn position_plage_fait_12_octets_et_point_mesure_20() {}
#[test] fn la_rgb2520_tient_dans_la_feuille() {}                 // 1349 + 601 × 42 ≤ 27940
#[test] fn une_zone_qui_deborde_est_refusee() {}
#[test] fn plus_de_50_zones_sont_refusees() {}
#[test] fn les_etats_inconnus_restent_inconnus() {}             // EtatSdk::depuis(42) == Inconnu(42)
#[test] fn la_liste_blanche_n_a_ni_jig_ni_reglage_reseau() {}   // aucun JIG_, ni SetNetworkSetting, RegisterJob, MatchingChart…
#[test] fn set_option_est_la_seule_fonction_set_admise() {}
```

- [ ] **Step 2 : Lancer** — `cargo test -p fd9-sys` puis `cargo test -p fd9-sys --target i686-pc-windows-msvc`. Expected: échec (types absents).
- [ ] **Step 3 : Implémenter** `feuille.rs` et la liste blanche (dans l'ordre des Global Constraints, un commentaire par export qui renvoie à sa fiche).
- [ ] **Step 4 : Vert en 32 et 64 bits**, clippy, fmt.
- [ ] **Step 5 : Commit** — `git commit -m "fd9-sys : structures de la feuille FD-9 et liste blanche de la mesure"`

### Tâche 3 : `pont-protocole` — définition d'une feuille, deux paliers, deux requêtes

**Files:**
- Create: `crates/pont-protocole/src/feuille.rs`
- Modify: `crates/pont-protocole/src/lib.rs` (`Palier`, `Requete`, `Reponse`, `ErreurPont`, `pub use`)
- Test: `crates/pont-protocole/tests/feuille.rs`

**Interfaces :**

```rust
// Palier : ajoutés APRÈS Bande, l'ordre sert aux plafonds.
pub enum Palier { Version, Detection, Connexion, Etalonnage, MesurePonctuelle, Bande, Prenumerisation, Feuille }

/// Une feuille à mesurer au FD-9 : unité 1/100 mm, origine en haut à gauche.
pub struct DefinitionFeuille {
    pub nom: String,                 // « RGB-2520 - 1P MY9 », comparé au début du texte du QR
    pub largeur: u32, pub hauteur: u32,
    pub zones: Vec<ZoneFeuille>,     // 1 à 50
}
pub struct ZoneFeuille { pub gauche: u32, pub haut: u32, pub largeur_plage: u32, pub hauteur_plage: u32,
    pub pas_x: u32, pub pas_y: u32, pub rangees: u32, pub colonnes: u32 }
impl DefinitionFeuille { pub fn verifier(&self) -> Result<(), ErreurMesure>; pub fn plages(&self) -> usize; }

// Requete
Prenumeriser { mire: DefinitionFeuille },
MesurerFeuille { mire: DefinitionFeuille },

// Reponse
Prenumerisee { lecture: LectureFeuille },
// la mesure d'une feuille répond `Mesure` (Geometrie::Feuille {}) ; la lecture du QR et la
// reconnaissance vont dans la provenance (champ `feuille`, ci-dessous)

pub struct LectureFeuille {
    pub qr: Info<String>,            // texte libre du QR, inconnu si absent
    pub zones: u32, pub plages: u32, // reconnues
    pub fiabilite: u32,              // brute, sens non établi
    pub ecart_max: u32,              // plus grand écart d'origine ou de pas, 1/100 mm
}

// ErreurPont, nouveaux cas
MireNonReconnue { zones: u32, plages: u32, ecart_max: u32 },
AutreMire { texte: String },
FeuilleAbsente {},                  // délai d'attente de la feuille dépassé
IncidentFeuille { etat: u32, premesure: Option<i32> },   // 101–103, 201–206, 301 ; premesure : FD9_GetPremeasurementError
```

`Provenance` reçoit `feuille: Option<LectureFeuille>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), présent pour une mesure en feuille. Le format reste `myiro-libre/mesure/1` : le champ est facultatif et les anciennes lignes se relisent. **Vérifier à l'implémentation** que `Mesure::new` refuse une feuille sans `feuille`, et une `feuille` sur une autre géométrie. `MODELE_FD9: &str = "FD-9"`.

- [ ] **Step 1 : Tests d'abord**

```rust
#[test] fn prenumerisation_et_feuille_viennent_apres_bande() {}           // Palier::Feuille > Palier::Bande
#[test] fn une_requete_mesurer_feuille_se_relit() {}                      // aller-retour JSON
#[test] fn une_definition_sans_zone_est_refusee() {}
#[test] fn une_zone_qui_deborde_de_la_feuille_est_refusee() {}
#[test] fn la_definition_rgb2520_compte_2520_plages() {}                  // nombres de « Ce que l'étude a établi »
#[test] fn une_mesure_en_feuille_sans_lecture_est_refusee() {}
#[test] fn une_ancienne_mesure_bande_se_relit_sans_feuille() {}
#[test] fn un_champ_inconnu_dans_la_definition_est_refuse() {}            // deny_unknown_fields
```

- [ ] **Step 2 : Rouge, puis implémenter, puis vert** (`cargo test -p pont-protocole`), et les crates qui font un `match` sur `Palier` ou `Requete` (`pont-myiro1`, `pont-fd9`, `app`) compilent : les nouvelles requêtes y sont refusées (`requete_invalide` ou `palier_non_autorise`), sans effet.
- [ ] **Step 3 : Commit** — `git commit -m "Protocole : définition d'une feuille, paliers Prénumérisation et Feuille"`

### Tâche 4 : `pont-fd9` — palier Connexion et rappel d'événements

**Files:**
- Create: `crates/pont-fd9/src/evenements.rs` (canal du rappel, `extern "C"`)
- Modify: `crates/pont-fd9/src/lib.rs` (trait `SdkFd9`, `PALIER_MAX`, `Session::connecter`, `connecter_adresse`, `fermer`), `crates/pont-fd9/src/serveur.rs`, `crates/pont-fd9/src/dll.rs`, `crates/pont-fd9/src/main.rs` (`--plafond connexion|prenumerisation|feuille`)
- Test: `crates/pont-fd9/tests/session.rs`, `crates/pont-fd9/tests/serveur.rs`, `crates/pont-fd9/tests/commun/` (simulé)

**Trait élargi** (une méthode par export ; le type interdit tout autre appel) :

```rust
pub trait SdkFd9 {
    // existants : dernier_code, lister, version_fichier, empreinte
    fn inscrire_rappel(&mut self, actif: bool) -> i32;                 // FD9_RegisterDeviceEventHandler(rappel | NULL)
    fn connecter(&mut self, appareil: &Appareil, nom: &NomApplication) -> i32;
    fn infos(&mut self, tampon: &mut TamponInfosSysteme) -> i32;       // FD9_GetSystemInfo
    fn statut(&mut self, tampon: &mut [u32; 8]) -> i32;                // FD9_GetStatus
    fn deconnecter(&mut self) -> i32;
    /// Événements reçus par le rappel depuis le dernier appel, dans l'ordre.
    fn evenements(&mut self, attente: std::time::Duration) -> Vec<Evenement>;
    // tâche 5 et 6 : capturer, resultat_capture, reconnaitre, resultat_reconnaissance,
    // points_mesure, mesurer, donnees, regler, calculer, arreter, sortir, erreur_premesure
}
pub struct Evenement { pub etat: EtatSdk, pub travail: u32, pub page: u32, pub compte: u32 }
```

Le rappel (`evenements.rs`) est une fonction `unsafe extern "C" fn(u32, u32, u32, u32)`. Il pousse un `Evenement` dans un `std::sync::mpsc::Sender` rangé dans un `static` (`Mutex<Option<Sender<Evenement>>>`), puis rend la main. Sans session : un compteur `AtomicU64` d'appels orphelins, relu dans le journal.

Ordre de la connexion :
1. `inscrire_rappel(vrai)` ;
2. `connecter` (0 ou 1901 = connecté ; 1901 noté « recertification due » dans l'identité) ;
3. `infos` : tampon à zéro, `est_rempli` exigé ;
4. `statut`.

En cas d'échec : `inscrire_rappel(faux)`. Fermeture : `deconnecter`, puis `inscrire_rappel(faux)`. Un échec de `deconnecter` donne `FermetureIncertaine`, sans nouvel appel.

- [ ] **Step 1 : Tests d'abord** (instrument simulé)

```rust
#[test] fn la_connexion_inscrit_le_rappel_avant_connect() {}            // journal : RegisterDeviceEventHandler, Connect, GetSystemInfo, GetStatus
#[test] fn le_code_1901_connecte_avec_recertification_due() {}
#[test] fn un_echec_de_connexion_desinscrit_le_rappel() {}
#[test] fn des_infos_non_remplies_refusent_la_connexion() {}
#[test] fn un_numero_de_serie_non_numerique_refuse_la_connexion() {}
#[test] fn la_connexion_par_adresse_ne_detecte_pas() {}                 // ConnecterAdresse : pas de GetDeviceList
#[test] fn la_fermeture_deconnecte_puis_desinscrit() {}
#[test] fn une_deconnexion_refusee_rend_fermeture_incertaine() {}
#[test] fn un_rappel_apres_fermeture_est_ignore_et_compte() {}
#[test] fn le_plafond_detection_refuse_la_connexion_sans_appel() {}
```

- [ ] **Step 2 : Rouge, implémenter, vert** (64 et 32 bits).
- [ ] **Step 3 : Test réel préparé, pas lancé** — `tests/dll.rs` : `palier_connexion` (`#[ignore]`). Il prend l'adresse dans `FD9_ADRESSE` (jamais en dur), affiche l'identité à l'écran et se déconnecte. Il n'est lancé qu'à l'essai E1, avec l'opérateur.
- [ ] **Step 4 : Commit** — `git commit -m "Pont FD-9 : palier Connexion et rappel d'événements sans appel à la DLL"`

### Tâche 5 : `pont-fd9` — palier Prénumérisation

**Files:**
- Create: `crates/pont-fd9/src/feuille.rs` (déroulé d'une feuille, commun aux deux paliers ; sous 400 lignes)
- Modify: `crates/pont-fd9/src/lib.rs`, `crates/pont-fd9/src/serveur.rs`, `crates/pont-fd9/src/dll.rs`
- Test: `crates/pont-fd9/tests/feuille.rs`

Déroulé de `Session::prenumeriser(&DefinitionFeuille) -> Result<LectureFeuille, ErreurPont>` :

1. `CoordonneesMire::depuis_definition`, sinon `ParametreRefuse` (aucun appel).
2. Attendre les événements jusqu'à 3 (`AlimentationFinie`), pendant `attente_feuille`. Sinon : **arrêt** (ci-dessous), puis `FeuilleAbsente`.
3. `capturer` (StartCapture) ; attendre l'événement 5.
4. `resultat_capture` deux fois (taille, puis image dans un `Vec<u8>` de la taille rendue). Le QR est rangé dans `DonneesQr::vide()` ; un QR introuvable laisse ce tampon intact, et un `deviceName` `FAIL` rend « trouvé » : seul `deviceName == "FD-9"` compte comme un QR lu.
5. QR trouvé avec un autre texte que `nom` : **arrêt**, puis `AutreMire { texte }`.
6. `reconnaitre(mode, &coord, couleurs = nul, &0, &0)` : `mode`, `rangees` et `colonnes` ne sont **jamais des pointeurs nuls** (fiche `FD9_StartPatchRecognition`). Attendre l'événement 7.
7. `resultat_reconnaissance(&fiabilité, nul, &coord_lue)`. Comparer zones, plages, origines et pas (écart ≤ 300). Sinon **arrêt**, puis `MireNonReconnue`.
8. Prénumérisation seule : **arrêt**, puis rendre `LectureFeuille`.

**Arrêt** (fiches `FD9_StopMeasurement` et `FD9_ExitMeasurement`, confirmé) : `arreter(faux)`, admis seulement dans les états 1 à 8 ; puis **attendre l'événement 10** (`AnnulationFinie`) ; puis `sortir(0, nul)`, admis seulement aux états 9, 10 et d'erreur. Sans événement 10 dans le délai : `Delai`, sans appeler `sortir` et sans inventer d'état.

Pendant toute attente :
- un événement d'erreur (101–103, 201–206, 301) donne `sortir` directement (sans `arreter`), puis `IncidentFeuille` (avec `erreur_premesure` pour 201) ;
- une requête `annuler` lue par le serveur donne **arrêt**, puis `MesureAnnulee`, comme le pont MYIRO-1 (#26) ;
- l'image capturée n'est gardée que le temps de la requête : elle n'est ni écrite ni renvoyée (plus tard, #35).

- [ ] **Step 1 : Tests d'abord** (scénarios simulés)

```rust
#[test] fn une_rgb2520_est_reconnue_sans_mesurer() {}           // StartMeasurement2 absent du journal
#[test] fn sans_feuille_le_delai_rend_feuille_absente() {}
#[test] fn un_autre_qr_refuse_la_feuille() {}
#[test] fn un_qr_absent_n_empeche_pas_la_reconnaissance() {}    // qr: Inconnue
#[test] fn une_geometrie_decalee_refuse_la_feuille() {}         // écart 400 > 300
#[test] fn un_nombre_de_plages_different_refuse_la_feuille() {} // 2478 au lieu de 2520
#[test] fn annuler_pendant_l_attente_arrete_et_sort() {}         // StopMeasurement(false), événement 10, puis ExitMeasurement
#[test] fn sans_evenement_10_l_arret_rend_delai_sans_sortir() {}
#[test] fn une_erreur_sort_sans_arreter() {}                     // état 102 : ExitMeasurement seul
#[test] fn la_feuille_est_refusee_sous_le_plafond_sans_appel() {}
#[test] fn la_capture_est_lue_en_deux_appels() {}               // image nulle, puis tampon de la taille rendue
```

- [ ] **Step 2 : Rouge, implémenter, vert** (64 et 32 bits).
- [ ] **Step 3 : Test réel préparé** — `palier_prenumerisation` (`#[ignore]`, `FD9_ADRESSE`, délai `FD9_ATTENTE` en secondes, 120 par défaut) : affiche le texte du QR, les zones, les plages, la fiabilité et l'écart. Lancé à l'essai E3.
- [ ] **Step 4 : Commit** — `git commit -m "Pont FD-9 : prénumérisation d'une feuille, QR et reconnaissance contrôlés"`

### Tâche 6 : `pont-fd9` — palier Feuille

**Files:**
- Modify: `crates/pont-fd9/src/feuille.rs`, `crates/pont-fd9/src/lib.rs`, `crates/pont-fd9/src/serveur.rs`, `crates/pont-fd9/src/dll.rs`
- Create: `crates/pont-fd9/src/plages.rs` (lecture et calcul des plages)
- Test: `crates/pont-fd9/tests/feuille.rs`, `crates/pont-fd9/tests/plages.rs`

`Session::mesurer_feuille(&DefinitionFeuille) -> Result<Mesure, ErreurPont>` :

1. Mêmes étapes 1 à 7 que la prénumérisation.
2. `points_mesure(&mut Vec<PointMesure>, &plages)`, sur la taille reconnue.
3. `mesurer(&1, &1)` (StartMeasurement2).
4. Attendre l'événement 9. Les événements 11 font avancer un compte, journalisé. Les erreurs sortent comme à la tâche 5.
5. Pour i de 0 à plages − 1 :
   - `regler(TypeDonnees, 0x0c)` puis `donnees(&pos, brutes[152], &152, &{i, −1, −1})` ;
   - `pos` doit valoir `{0, i / colonnes, i % colonnes}` pour une zone (règle générale : zones dans l'ordre, rangées puis colonnes) ; sinon refus de toute la mesure (`ReponseInattendue`).
6. Puis, pour M0 (0), M1 (2) et M2 (15) :
   - `regler(Condition, c)`, `regler(TypeDonnees, 0x0b)`, `calculer(spectre, &n, brutes, &152)` ;
   - puis `regler(TypeDonnees, 0)`, `regler(Illuminant, 2)`, `regler(Observateur, 0)`, `calculer(lab, &3, brutes, &152)`.
7. `sortir(0, nul)`, admis à l'état 9. **Pas de `arreter(vrai)`** : il serait refusé (1104) à l'état 9 (fiche `FD9_StopMeasurement`, confirmé). MYIROtools l'appelle quand même et ignore son retour : ce plan ne le reprend pas.
8. `Mesure::new(plages, provenance)` :
   - modèle `FD-9`, version du SDK, empreinte de la DLL ;
   - `etalonnage: Info::Inconnue` (pas d'étalonnage utilisateur ; libellé : « étalonnage automatique de l'instrument ») ;
   - `Geometrie::Feuille {}`, `feuille: Some(lecture)` ;
   - conditions M0, M1, M2 `Supposee`, longueurs d'onde `Supposee { debut_nm: 380, pas_nm: 10 }`, D50 et 2° `Supposee` : les valeurs 2 et 0 viennent d'une table de FD-S2w, et la DLL ne permet pas de relire un réglage (aucun `FD9_GetOption` dans la liste blanche).

Variante prévue (Choix de l'assistant) : si l'essai E4 montre que les données ne sont plus lisibles à l'événement 9, l'étape 5 se fait au fil des événements 11, plage par plage.

- [ ] **Step 1 : Tests d'abord**

```rust
#[test] fn une_feuille_rgb2520_rend_2520_plages_dans_l_ordre() {}
#[test] fn chaque_plage_a_trois_spectres_et_ses_donnees_brutes() {}
#[test] fn une_position_inattendue_refuse_la_mesure() {}
#[test] fn un_spectre_de_longueur_inattendue_refuse_la_mesure() {}    // autre que 36 : refus, en le disant
#[test] fn la_fin_lit_les_plages_puis_sort_sans_arreter() {}          // ExitMeasurement(0, NULL) après la dernière plage, aucun StopMeasurement
#[test] fn la_provenance_dit_feuille_et_qr_lu() {}
#[test] fn les_conditions_restent_supposees_tant_que_non_verifiees() {}
#[test] fn mesurer_sans_prenumerisation_prealable_est_permis() {}     // Feuille exige Connexion, pas Prénumérisation
```

- [ ] **Step 2 : Rouge, implémenter, vert** (64 et 32 bits).
- [ ] **Step 3 : Test réel préparé** — `palier_feuille` (`#[ignore]`, `FD9_ADRESSE`, `FD9_SORTIE`). Il écrit la mesure (JSON) dans `FD9_SORTIE` (sous `Archivage/donnees/`, local) et affiche le Lab M1 des plages 1, 42, 2479 et 2520. Lancé à l'essai E4.
- [ ] **Step 4 : Commit** — `git commit -m "Pont FD-9 : mesure d'une feuille, 2 520 plages lues et associées à la mire"`

### Tâche 7 : Incidents et simulation complète

**Files:**
- Modify: `crates/pont-fd9/tests/commun/` (scénarios), `crates/pont-fd9/src/feuille.rs`
- Test: `crates/pont-fd9/tests/incidents.rs`, `crates/pont-fd9/tests/executable.rs`

Scénarios du ticket #34 : feuille valide, plage non reconnue, bourrage, perte de liaison, fermeture incertaine ; et en plus, couvercle ouvert, erreur de prémesure, feuille trop longue, état inconnu.

- [ ] **Step 1 : Tests**

```rust
#[test] fn un_bourrage_rend_une_erreur_et_aucune_plage() {}           // état 102 → IncidentFeuille { etat: 102 }
#[test] fn une_perte_de_liaison_rend_instrument_perdu() {}            // état 103, puis Disconnect refusé
#[test] fn un_couvercle_ouvert_rend_une_erreur_nommee() {}            // 101
#[test] fn une_erreur_de_premesure_donne_sa_cause() {}                // 201 + FD9_GetPremeasurementError
#[test] fn un_etat_inconnu_arrete_sans_inventer() {}                  // 77 → ReponseInattendue, sortir
#[test] fn apres_un_incident_la_session_reste_fermable() {}
#[test] fn l_executable_refuse_feuille_au_plafond_connexion() {}      // pont lancé en vrai processus, instrument simulé
```

- [ ] **Step 2 : Vert** en 64 et 32 bits ; `cargo clippy --all-targets -- -D warnings`.
- [ ] **Step 3 : Commit** — `git commit -m "Pont FD-9 : incidents d'une feuille traduits en erreurs explicites"`

### Tâche 8 : Application — mesurer la RGB-2520 et la ranger

**Prérequis :** plan 4 fusionné, branche rebasée sur `dev`.

**Files:**
- Create: `app/src/instrument/feuille.rs` (définition RGB-2520, requête, suivi ; sans Tauri), `app/src/mesurer_feuille.rs` (méthodes de `BibliothequeOuverte` : CGATS, import, JSON)
- Modify: `app/src/instrument/fd9.rs` (`PLAFOND_FD9` → `Palier::Connexion` ; connexion par adresse), `app/src/pont.rs` (palier de chaque requête), `app/src/lib.rs` (commandes et état seulement), `crates/cgats/src/ecriture.rs` et `crates/cgats/src/lecture.rs` (colonnes RVB d'un fichier myiro-libre), `app/src/textes.rs`
- Test: `app/tests/mesurer_feuille.rs`, `crates/cgats/tests/rvb.rs`

**Définition RGB-2520** (`feuille.rs`) : la constante `DEFINITION_RGB2520` porte les nombres de « Ce que l'étude a établi », et `nom = mires::RGB2520_NOM`. Un test la confronte à `mires::RGB2520_GEOMETRIE` : 42 colonnes, 60 rangées, et pas de 71 px à 300 ppp = 601 à 1/100 mm près (71 / 300 × 2 540 = 601,1).

Déroulé de `mesurer_feuille_fd9(condition)` :
1. Trouver la mire (`chercher_mire_fd9`, puis `lire_mire_fd9`) ; sinon, refus clair (texte de `ErreurMire`).
2. Relancer `pont-fd9` au plafond **Feuille**, pour cette mesure seulement.
3. `MesurerFeuille { mire: DEFINITION_RGB2520 }`, avec l'avancement (« glissez la feuille », « lecture de la feuille », « mesure »).
4. Mesure reçue :
   - écrire le CGATS avec `cgats::ecrire_avec_rvb(&mesure, &mire.valeurs)` (RVB de 0 à 100) ;
   - `importer_mesure(condition, "FD-9 RGB-2520 <date>.txt", texte)` ;
   - garder le JSON (`ecrire_mesure`) dans `<bibliothèque>/mesures-fd9/<id>.json`.
5. Fermer le pont, puis revenir au plafond Connexion.

**CGATS** : `ecrire_avec_rvb` ajoute `RGB_R RGB_G RGB_B` à chaque tableau. Le nombre de valeurs RVB doit être égal au nombre de plages, sinon refus. `lire` donne une échelle RVB `Confirmee` (0 à 100) pour un fichier écrit par myiro-libre. Les autres règles d'échelle ne changent pas.

- [ ] **Step 1 : Tests d'abord**

```rust
// crates/cgats/tests/rvb.rs
#[test] fn un_cgats_myiro_libre_avec_rvb_se_relit_avec_une_echelle_confirmee() {}
#[test] fn des_rvb_en_nombre_different_des_plages_sont_refuses() {}
// app/tests/mesurer_feuille.rs (pont et mire simulés, MoteurSimule)
#[test] fn la_definition_rgb2520_correspond_a_la_geometrie_de_la_mire() {}
#[test] fn une_feuille_mesuree_entre_dans_la_resolution_comme_mesure_importee() {}
#[test] fn la_mesure_importee_sert_au_calcul_du_profil() {}           // profiler, plan 3, sans changement
#[test] fn le_json_complet_est_garde_a_cote() {}
#[test] fn sans_mire_rgb2520_la_mesure_est_refusee_en_clair() {}
#[test] fn le_plafond_par_defaut_de_l_application_est_connexion() {}
#[test] fn apres_la_mesure_le_pont_revient_au_plafond_connexion() {}
```

- [ ] **Step 2 : Rouge, implémenter, vert** ; `cargo test -p app -p cgats` en 64 et 32 bits.
- [ ] **Step 3 : Commit** — `git commit -m "Application : mesurer la RGB-2520 au FD-9 et la ranger dans la résolution"`

### Tâche 9 : Écran « Mesurer une feuille au FD-9 »

**Files:**
- Modify: `app/interface/reglage.js` (section des mesures de la vue d'une résolution) ou un nouveau `app/interface/feuille-fd9.js` si `reglage.js` dépasse 400 lignes ; `app/src/textes.rs` ; `app/interface/index.html` (script)
- Test: `app/tests/textes.rs` (clés présentes dans les deux langues)

La vue d'une résolution reçoit un bouton **« Mesurer une feuille au FD-9 »**. Il est actif quand un FD-9 est connecté et la mire RGB-2520 trouvée ; sinon, il est grisé et dit pourquoi. Pendant la mesure, la page montre l'état en clair :
1. « Glissez la feuille RGB-2520 dans le FD-9 » ;
2. « Lecture de la feuille » ;
3. « Mesure : n / 2 520 » ;
4. « Mesure rangée » ou l'erreur avec sa cause et l'action à faire (`<code>.cause`, `<code>.action`).

Un bouton **« Annuler »** reste présent pendant l'attente et la lecture. Thèmes clair et sombre, jetons de `design-system/tokens.css`, aucun texte en dur, utilisable au clavier.

- [ ] **Step 1 : Clés de texte** `feuille_fd9.*` (français et anglais) : libellés, états, et chaque `ErreurPont` de la feuille (`feuille_absente`, `autre_mire`, `mire_non_reconnue`, `incident_feuille.101|102|103|201|…|301`, `delai`, `instrument_perdu`).
- [ ] **Step 2 : Page**, puis `cargo test -p app` (catalogue complet).
- [ ] **Step 3 : Vérification visuelle** avec `cargo run -p app` et un pont simulé (`--demonstration` si présent) : capture d'écran des quatre états, dans les deux thèmes.
- [ ] **Step 4 : Commit** — `git commit -m "Écran : mesurer une feuille au FD-9, état en clair et annulation"`

### Tâche 10 : Documentation et fin

**Files:**
- Modify: `CLAUDE.md` (paliers du pont FD-9, liste blanche, tests `--ignored` `palier_connexion`, `palier_prenumerisation`, `palier_feuille`, variables `FD9_ADRESSE`, `FD9_SORTIE`), `GLOSSARY.md` (Prénumérisation, Reconnaissance Combine, Recertification, Feuille), `docs/adr/0005-*.md` (paliers propres au FD-9), `docs/blog/<jour>.md`, `PLAN-ACTION.md` si l'étape y figure

- [ ] **Step 1 : Mettre à jour**, puis `python outils/verifier_docs.py`.
- [ ] **Step 2 : Contrôles complets** — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/test_preparer_installateur.py`, `python outils/test_verifier_docs.py`.
- [ ] **Step 3 : Commit**, puis PR vers `dev` avec « Choix de l'assistant » et « Review Focus ». Les essais réels E1–E5 sont listés dans la PR et cochés au fur et à mesure par l'opérateur.

---

## Essais sur l'instrument, palier par palier (opérateur)

À faire **avec l'accord de l'opérateur, un essai à la fois**, dans l'ordre. Préalables : FD-S2w et MYIROtools fermés, FD-9 allumé et en réseau, pare-feu réglé (#47), pont 32 bits compilé. `FD9_ADRESSE` est donnée par l'opérateur au moment de l'essai et n'est écrite nulle part. Option de diagnostic : créer un dossier `DebugSDK` dans le dossier courant pour avoir le journal du SDK ; le supprimer ensuite (il contient le n° de série). Chaque essai est consigné dans le journal du jour, sans adresse, MAC ni n° de série.

| Essai | Palier | Commande | Geste de l'opérateur | Ce qu'on attend | Ce qu'on note |
|---|---|---|---|---|---|
| **E1** | Connexion | `cargo test -p pont-fd9 --target i686-pc-windows-msvc --test dll -- --ignored palier_connexion --nocapture` | aucun | code 0 ou 1901 ; identité remplie ; déconnexion sans erreur | code de connexion (1901 = recertification due) ; micrologiciel ; contenu de `GetStatus` (8 nombres) |
| **E2** | Prénumérisation, sans feuille | `… --ignored palier_prenumerisation --nocapture` avec `FD9_ATTENTE=20` | ne rien glisser | `FeuilleAbsente` après 20 s ; instrument revenu au repos ; reconnexion possible | événements reçus (état 1 ?) ; effet de `StopMeasurement(faux)` puis `ExitMeasurement` à vide |
| **E3** | Prénumérisation, RGB-2520 | même commande, délai par défaut | glisser une RGB-2520 imprimée | QR « `RGB-2520 - 1P MY9*1+` » ; 1 zone, 2 520 plages ; écart ≤ 300 ; feuille rendue sans mesure | la feuille part-elle seule (sans travail) ? texte exact du QR ; géométrie rendue (unités, écart au TIFF) ; COMBINE sans couleurs suffit-il (sinon refaire avec `--reconnaissance coordinate`) ; feuille éjectée ou retenue après `StopMeasurement(faux)` ; **variante : glisser la feuille avant d'envoyer la requête** (état 3 lu par `FD9_GetStatus`, la capture part aussitôt) |
| **E3 bis** | Prénumérisation, autre feuille | même commande | glisser une page blanche, puis une autre mire FD-S2w | refus `MireNonReconnue` ou `AutreMire`, rien de mesuré | message rendu ; instrument au repos |
| **E4** | Feuille, RGB-2520 | `cargo test -p pont-fd9 --target i686-pc-windows-msvc --test dll -- --ignored palier_feuille --nocapture`, `FD9_SORTIE=Archivage/donnees/fd9/` | glisser la RGB-2520 **déjà mesurée le 26 octobre** | 2 520 plages ; spectres de 36 valeurs ; positions dans l'ordre ; durée | comparaison du Lab M1 et des spectres avec `Archivage/donnees/mesures-importees/RGB-2520_00025.txt` : ΔE00 moyen et maximal (seuil proposé : moyen ≤ 0,5, maximal ≤ 1,5, à confirmer par le mainteneur) ; données encore là à l'événement 9 (sinon variante de la tâche 6) ; valeurs des conditions M0 / M1 / M2 confirmées ou corrigées |
| **E5** | Application, de bout en bout | `cargo run -p app` | connecter le FD-9, choisir une résolution, « Mesurer une feuille au FD-9 », glisser la feuille | mesure rangée dans la résolution ; profil calculable (plan 3) | durée totale ; textes de l'écran ; profil calculé comparé à celui de la mesure du 26 octobre (comparaison du plan 5) |
| **E6** (facultatif, accord explicite) | Feuille, incident | `palier_feuille` | ouvrir le capot pendant la mesure | `IncidentFeuille { etat: 101 }` ; session fermable ; FD-9 réutilisable sans redémarrage | message ; geste de remise en état. **On ne provoque jamais de bourrage.** |

Après chaque essai, les fiches `docs/abi/` concernées passent les points vérifiés de « supposé » à « confirmé » (« vérifié sur l'instrument le … »), sans valeur identifiante.

## Questions ouvertes (tranchées par les essais)

1. La feuille part-elle seule sans `FD9_RegisterJob` (E3) ? Sinon : `RegisterJob` / `DeleteJob` au plan 7, avec leurs fiches.
2. COMBINE sans `tFD9_PatchColors` reconnaît-il la RGB-2520 (E3) ? Sinon : COORDINATE ; et si COORDINATE échoue aussi, étude de l'échelle des couleurs de FD-S2w (`0x4da710`, notes locales § 5).
3. Texte exact du QR et sens de `011M` et `*1+` (E3).
4. Unités et écart de la géométrie rendue par `GetPatchRecognitionResult2` (E3).
5. Longueur d'un spectre et valeurs M0 / M1 / M2 de la clé 0 (E4).
6. Données lisibles à l'événement 9 (E4).
7. Effet de `StopMeasurement(faux)` sur la feuille (E2, E3).
