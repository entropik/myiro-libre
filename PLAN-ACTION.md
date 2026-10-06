# Plan d'action : application autonome de mesure et de profilage ICC (MYIRO-1 et FD-9)

Date : 6 octobre 2026. Document de travail, à relire avec `Audit-MYIRO/LIRE-MOI-AUDIT.txt`.

Convention : **[confirmé]** = vérifié dans les fichiers du dossier (chemin cité) ; **[supposé]** = déduction à valider.

---

## 1. Objectif

Remplacer Myiro Tool par une application autonome qui :

1. pilote le **MYIRO-1** (`FDXSDK.dll`) et le **FD-9** (`FD9SDK.dll`) ;
2. mesure des chartes (bandes au MYIRO-1, feuilles au FD-9), des couleurs ponctuelles et des densités ;
3. calcule des profils ICC imprimante à partir des mesures ;
4. n'appelle jamais les surfaces de maintenance usine des instruments.

---

## 2. Audit général : où en est le projet

### 2.1 Ce qui existe

| Élément | État |
|---|---|
| Collecte des installations (MY-CT1, FD-S2w, Ergosoft, EIZO, résidus MYIROtools) | Fait, 871 fichiers, 634 Mo, contrôle SHA-256 |
| Analyse PE statique (exports, imports, versions, chaînes) | Fait, 84 binaires, `Audit-MYIRO/analyse/` |
| Désassemblage Capstone des exports FDX et des appelants | **Commencé**, `Audit-MYIRO/retroanalyse/` (non mentionné dans `CLAUDE.md`) |
| Script de désassemblage | `Audit-MYIRO/outils/desassemble.py` (Capstone 5.0.9 local dans `python-libs/`, non mentionné dans `CLAUDE.md`) |
| Désassemblage FD9SDK | Absent |
| En-têtes C reconstitués, signatures documentées | Absents |
| Code applicatif, dépôt git, tests | Absents |
| Moteur de profilage ICC | Absent (aucun export de profilage dans FDXSDK ni FD9SDK) |

### 2.2 Constats nouveaux de cet audit

**MYIRO-1 / FDXSDK**

- **[confirmé]** Les exports x86 sont en `__stdcall` : nettoyage de pile par l'appelé (`ret N`) dans `retroanalyse/fdx-x86/exports/` : `FDX_GetSDKVersion` (`ret 4`, 1 argument), `FDX_Calibration` (`ret 4`), `FDX_Connect` (`ret 8`, 2 arguments), `FDX_GetDevicePortList` (`ret 0xc`, 3 arguments), `FDX_GetMeasureData` (`ret 0x14`, 5 arguments). `FDX_StartMeasurement` se termine par `ret` sans opérande : 0 argument ou `__cdecl`, à trancher.
- **[confirmé]** `FDX_GetSDKVersion` remplit une structure de sortie (écritures à `+0`, `+4`, `+8`), retourne un code négatif si le pointeur est nul (`0xFFFFD8F8`) et refuse de s'exécuter « device is connected » dans certains états. Les codes d'erreur semblent être des entiers négatifs autour de -10000 **[supposé]**.
- **[confirmé]** `libmeasurementdevice*.dll` d'EIZO charge FDXSDK dynamiquement (`GetProcAddress`) et référence toute la chaîne de mesure : `Connect`, `Calibration`, `Get/SetMeasureCondition`, `StartMeasurement`, `StopMeasurement`, `GetMeasureData`, `GetRAWData`, `CalculateMeasureData`, `RegisterDeviceEventHandler`, `GetError`. **C'est l'appelant de référence pour reconstituer la séquence de mesure.**
- **[confirmé]** `SpectrophotometerConfigurationToolMY-CT1.exe` importe seulement la configuration (`GetDeviceInfo`, `Get/SetNetworkInfo`, `Get/SetShutdownTime`…) **et** des écritures usine `FDX_JIG_SetFactoryCalib_CALREFDATA` et `FDX_JIG_SetFactoryCalib_WriteSector`. MY-CT1 n'est donc pas une référence pour la mesure, et ne doit pas être utilisé pour « essayer » des fonctions sur l'instrument.
- **[confirmé]** Export de référence MYIROtools : `collecte/MYIRO-ProgramData/MYIROtools/Data/Tool/Measure/SpotColor/SpotColor_00001.txt`, CGATS.17, MYIRO-1 identifié par son n° de série, 45/0, D50, 2°, deux mesures Lab. Il servira de point de comparaison si cet instrument est celui du poste.

**FD-9 / FD9SDK**

- **[confirmé]** `FD-S2w.exe` 1.6.0.8 (x86) importe 29 fonctions de FD9SDK, dont `FD9_Connect`, `FD9_RegisterJob`, `FD9_StartPatchRecognition`, `FD9_GetPatchRecognitionResult2`, `FD9_StartMeasurement2`, `FD9_GetData`, `FD9_CalculateData`. C'est l'appelant de référence pour le FD-9. Il appelle aussi deux exports `JIG_` (`JIG_TileMeasurement`, `JIG_CalculateDeltaE2000`) : le préfixe JIG ne signifie donc pas toujours écriture usine, mais la règle reste de ne pas les appeler sans contrat établi.
- **[confirmé]** FD9SDK dépend de `DIColor.dll` (calculs couleur KM : `_SpectrumToColor`, `_SpectrumToDensity`, `_CalcDeltaE`…), `FD9BarcodeManager.dll`, OpenCV 2.4.7 et, en x64, `VCOMP110.DLL`. La FD9SDK x64 archivée (Ergosoft 1.3.1.5) **n'a pas ses dépendances x64 dans `collecte/`** ; elles existent dans `C:/Program Files/Ergosoft 16` (`DIColor.dll`, `FD9BarcodeManager.dll`, `opencv_core247.dll` vus), à archiver.
- **[confirmé]** `collecte/FD-S2w/Module/FD-SDK.lic` (48 octets) contient une clé de licence de la forme `FD-S2w:<empreinte>`. **[supposé]** FD9SDK exige une clé d'application à la connexion ; à vérifier dans le désassemblage de `FD9_Connect`.
- **[confirmé]** Identifiants USB : FD9SDK contient `VID_132B&PID_210D\A8AN` et `\9C1A` ; FDXSDK contient `PID_210D`/`PID_210F` avec `\9C1D` et `\ACJ1`. **[supposé]** Le PID 210D est commun à plusieurs instruments KM et le suffixe distingue le modèle.
- **[confirmé]** FD-S2w n'est déclaré que pour FD-9 et FD-5 BT/BT2 (`Module/Instrument.lst`).

**Mires et densité**

- **[confirmé]** Les définitions de mires FD-S2w sont du XML lisible avec valeurs et mise en page : `collecte/FD-S2w/Standard Chart Defs/ECI2002.xml` : 1485 plages, `ColorFormat="CMYK"`, valeurs ×100 (`10000` = 100 %), feuille 20800 × 28000 et plages de 600 (**[supposé]** centièmes de mm : 208 × 280 mm, plages de 6 mm). Mires disponibles : ECI2002, IT8.7/4, P2P25X, P2P51, TC1617x (H/V), Test Chart No69, Ugra/Fogra MW3, Universal LFP IT874+. Elles sont importables directement.
- **[confirmé]** `FD-S2w/InitialData/Density` contient 344 fichiers `.s2chart` et `ColorDens/PreDefined` des mires de contrôle de densité par format de papier (FD-5 et FD-9). Le format `.s2chart` reste à étudier.
- **[confirmé]** `libcalcolor.dll` (FD-S2w) exporte `SpectrumToDensity`, `CalcDotArea`, `CalcGraybalanceByG7`, `CalcTrapValueByPreucil`… : liste utile des fonctions densitométriques attendues par les utilisateurs de FD-S2w, à réimplémenter plutôt qu'à appeler (licence, x86 uniquement).

### 2.3 Écarts à combler

1. Aucune signature confirmée de structure de données : `MeasureCondition`, format de `MeasureData` (spectre ? Lab ? combien de longueurs d'onde ?), `DeviceInfo`, callback d'événements.
2. FD9SDK non désassemblé ; clé de licence et séquence « job → reconnaissance des plages → mesure » inconnues.
3. Aucune mesure observée sur l'instrument réel, aucune capture de session de référence.
4. Pas de chaîne spectre → Lab/densité ni de moteur ICC.
5. `CLAUDE.md` n'est plus à jour (dossier `retroanalyse/`, `desassemble.py`, Capstone).
6. Pas de gestion de version : rien n'empêche de perdre le travail de rétro-ingénierie.

---

## 3. Architecture cible proposée

```
┌─────────────────────────────── Application (64 bits) ───────────────────────────────┐
│ Interface : mesure de charte · couleur ponctuelle · densité · profil · rapports      │
│ Cœur : mires (XML FD-S2w, CGATS) · colorimétrie (spectre→XYZ/Lab, M0/M1/M2,          │
│        densités ISO 5-3, ΔE00) · stockage des mesures · export CGATS.17 / .ti3       │
│ Profilage : ArgyllCMS colprof (processus externe) → profil ICC · contrôle profcheck   │
└──────────────┬──────────────────────────────────────────────┬──────────────────────┘
               │ JSON sur stdin/stdout (ou tube nommé)        │
     ┌─────────▼─────────┐                          ┌─────────▼─────────┐
     │ Pont MYIRO-1       │                          │ Pont FD-9          │
     │ processus séparé   │                          │ processus séparé   │
     │ FDXSDK x86 ou x64  │                          │ FD9SDK x86 ou x64  │
     │ liste blanche      │                          │ liste blanche      │
     │ d'exports          │                          │ d'exports          │
     └────────────────────┘                          └────────────────────┘
```

Choix et raisons :

- **Ponts dans des processus séparés.** Un plantage de DLL ou un ABI mal reconstitué ne fait pas tomber l'application ; le pont peut être 32 bits si seule la DLL x86 fonctionne ; le pont est le seul endroit où une **liste blanche d'exports** est appliquée (refus codé en dur de `FDX_JIG_*`, `JIG_*`, `FDX_Set*` d'écriture et `FD9_SetNetworkSetting` tant que leur contrat n'est pas établi).
- **Langage : Rust — tranché le 6 octobre 2026.** Un espace de travail Cargo unique :
  - `fdx-sys` et `fd9-sys` : déclarations `extern "stdcall"` / `extern "system"` reconstituées et structures `#[repr(C)]`, chargement dynamique par `libloading` (aucune liaison statique aux DLL KM) ;
  - `pont-myiro1` et `pont-fd9` : exécutables ponts, compilables pour `i686-pc-windows-msvc` (DLL x86) et `x86_64-pc-windows-msvc` (DLL x64), avec la liste blanche ; protocole JSON (`serde`) sur stdin/stdout ;
  - `colorimetrie` : spectre → XYZ/Lab, densités, ΔE, sans dépendance native, testable sur toutes plateformes ;
  - `mires` : lecture XML FD-S2w (`quick-xml`) et CGATS.17 / `.ti3`, génération de mises en page ;
  - `profilage` : pilotage d'ArgyllCMS en processus externe ;
  - `app` : interface moderne. Recommandation : **Tauri 2** (interface web, cœur Rust, paquets Windows/macOS/Linux) ; alternatives natives : Slint ou egui. À trancher en phase 8, le cœur n'en dépend pas.
  - Les ponts Windows sont la seule partie liée à Windows ; tout le reste peut tourner ailleurs (utile pour exploiter des fichiers de mesure sur Mac/Linux).
  - Outillage d'audit et de désassemblage : les scripts Python existants restent tels quels, ils ne font pas partie de l'application.
- **Profilage par ArgyllCMS `colprof`** en processus externe, alimenté en `.ti3` (CGATS) : moteur éprouvé, contrôle de GCR/TAC/intentions. Licence AGPL : sans contrainte pour un usage personnel, à revoir si l'application doit être distribuée. Un moteur maison basé sur LittleCMS reste possible plus tard.
- **Aucune redistribution des DLL KM** : l'application les cherche dans les installations du poste ou dans un dossier désigné par l'utilisateur.

---

## 4. Plan d'action par phases

Chaque phase a un critère de sortie. Les phases 1 à 3 se font **sans instrument branché**.

### Phase 0 — Fondations (court)

- Choisir la licence libre (GPL-3.0 recommandée), ajouter `LICENSE`, et archiver la réponse écrite de Konica Minolta sur les SDK.
- Initialiser un dépôt git ; exclure `Audit-MYIRO/collecte/` et toutes les DLL/PDF du suivi (licences, 634 Mo) ; suivre seulement scripts, analyses textuelles et documentation.
- Mettre à jour `CLAUDE.md` : `retroanalyse/`, `desassemble.py`, Capstone, ce plan.
- Archiver les dépendances x64 de FD9SDK depuis `C:/Program Files/Ergosoft 16` (DIColor, FD9BarcodeManager, OpenCV 2.4.7, VCOMP110) avec empreintes, via `audit.py`.
- Espace de travail Cargo : `crates/fdx-sys`, `crates/fd9-sys`, `crates/pont-myiro1`, `crates/pont-fd9`, `crates/colorimetrie`, `crates/mires`, `crates/profilage`, `app/` ; plus `docs/abi/` (fiches par fonction). Installer la cible `i686-pc-windows-msvc` ; intégration continue `cargo fmt`, `clippy`, `cargo test` (crates indépendantes de Windows).

*Sortie : dépôt versionné, documentation à jour, archive FD9 x64 complète.*

### Phase 1 — Reconstitution de l'ABI FDXSDK (MYIRO-1)

- Désassembler les sites d'appel dans `libmeasurementdevice*.dll` (EIZO) : ordre des appels, tailles des tampons alloués, champs lus dans les structures retournées, gestion des callbacks.
- Établir, pour chaque fonction prioritaire, une fiche `docs/abi/FDX_<nom>.md` : convention d'appel, arguments, structures (taille, décalages), codes d'erreur, propriété mémoire, avec preuve (adresse d'instruction) et statut confirmé/supposé.
- Comparer x86 1.0.1, x64 1.0.1 et x64 1.0.3 sur ces fonctions (tailles de structures, constantes).
- Traduire les fiches dans `fdx-sys` (types `#[repr(C)]`, tests de taille `size_of` par architecture), limité à la liste blanche.
- Points clés à trancher : format de `GetMeasureData` (spectre 380–730 nm par pas de 10 nm attendu **[supposé]**), conditions M0/M1/M2 dans `MeasureCondition`, déroulé d'une mesure en balayage (`scanMeasurement`, `CPatchRecog`) et événements associés.

*Sortie : fiches et en-tête couvrant `GetSDKVersion`, `GetDevicePortList`, `Connect`, `Disconnect`, `GetDeviceInfo`, `GetError`, `RegisterDeviceEventHandler`, `Calibration`, `Get/SetMeasureCondition`, `Start/StopMeasurement`, `CancelScanMeasurement`, `GetMeasureData`, `GetRAWData`, `CalculateMeasureData`.*

### Phase 2 — Reconstitution de l'ABI FD9SDK (FD-9)

- Ajouter FD9SDK x86 (1.3.2.3) et `FD-S2w.exe` à `desassemble.py`.
- Reconstituer le cycle observé dans FD-S2w : `GetDeviceList` → `Connect` (clé de licence ?) → `GetSystemInfo` → `RegisterJob` → `StartPatchRecognition` / `GetPatchRecognitionResult2` → `StartMeasurement2` → `GetData` / `CalculateData` → `DeleteJob` → `Disconnect`.
- Déterminer comment une mire XML FD-S2w devient un « job » (structures attendues par `RegisterJob`).
- Étudier le format `.s2chart`.
- **Le FD-9 est connecté en réseau** : capturer passivement (Wireshark, poste en lecture seule) le trafic entre FD-S2w et le FD-9 pendant une session normale (connexion, reconnaissance de mire, mesure). Cela révèle le protocole applicatif réel et permet de vérifier l'ABI reconstituée ; à terme, c'est la piste pour un pilote FD-9 en Rust **sans FD9SDK**, donc multiplateforme. Ne rejouer aucune trame vers l'instrument tant que leur rôle n'est pas établi.

*Sortie : fiches et crate `fd9-sys`, avec la question de la clé de licence tranchée.*

La phase 2 se mène **en parallèle** des phases 1, 3 et 4 : le MYIRO-1 est livré en premier, mais le FD-9 est étudié et développé dans le même projet (phase 5).

### Phase 3 — Ponts et banc d'essai sans instrument

- Écrire le pont MYIRO-1 en Rust avec liste blanche ; ne charger la DLL que dans le processus pont.
- Premier appel réel : `FDX_GetSDKVersion` seul, instrument débranché ; puis `FDX_GetDevicePortList` instrument débranché (liste vide attendue).
- Même chose pour le pont FD-9 (`FD9_GetDeviceList`).
- Tests automatisés : refus des exports interdits, encodage/décodage des structures, gestion des erreurs.

*Sortie : les deux ponts répondent sans instrument, sans plantage, en 32 et/ou 64 bits.*

### Phase 4 — MYIRO-1 sur instrument réel (progression imposée)

Une étape à la fois, avec journal de chaque appel et accord avant de passer à la suivante :

1. détection (`GetDevicePortList`) ;
2. connexion et informations (`Connect`, `GetDeviceInfo`) : vérifier le n° de série ;
3. calibration blanc (`Calibration`) ;
4. mesure ponctuelle : comparer à l'export MYIROtools de référence ou à une mesure faite avec un autre logiciel sur la même plage (écart ΔE00 attendu < 0,3 en répétabilité **[supposé]**) ;
5. lecture de bande seulement ensuite.

Si un comportement reste ambigu, observer une session d'un logiciel fonctionnel (EIZO ColorNavigator ou Ergosoft) avec la journalisation SDK (`/FDX_SDKLog`) et une capture USB, sans écrire dans l'instrument.

*Sortie : une bande mesurée de bout en bout et exportée en CGATS.17.*

### Phase 5 — FD-9 sur instrument réel

Même progression : détection → connexion/infos → reconnaissance d'une mire standard (ECI2002 ou IT8.7/4) → mesure d'une feuille → export CGATS.17, comparé à une mesure FD-S2w de la même feuille.

En attendant, **FD-S2w reste le chemin de secours pour le FD-9** : ses exports CGATS peuvent déjà alimenter le profilage (phase 7). C'est pourquoi le MYIRO-1, qui n'a plus de logiciel de mesure sur le poste, passe en premier.

*Sortie : feuille FD-9 mesurée par l'application, écarts avec FD-S2w documentés.*

### Phase 6 — Cœur colorimétrique et densitométrique

- Spectre → XYZ → Lab (D50, 2°, et autres illuminants/observateurs), M0/M1/M2 selon ce que l'instrument fournit.
- Densités ISO 5-3 (statut T, E, I), engraissement / taux de couverture apparent (Murray-Davies, Yule-Nielsen), trapping (Preucil), gris G7 : la liste des fonctions de `libcalcolor.dll` sert de cahier des charges. Les tables de pondération ISO 5-3 sont à sourcer proprement.
- ΔE76, ΔE94, ΔE00 ; moyennage de plusieurs feuilles ; détection de plages aberrantes.
- Validation croisée : mêmes spectres traités par l'application et par FD-S2w / DIColor, écarts tolérés documentés.

*Sortie : bibliothèque testée, résultats concordants avec la référence KM.*

### Phase 7 — Mires et profilage ICC

- Import des mires XML FD-S2w (valeurs CMJN + géométrie) et des fichiers CGATS ; génération de mires nouvelles (ArgyllCMS `targen`) mises en page pour le FD-9 (XML FD-S2w) et pour les bandes du MYIRO-1 (taille minimale de plage et longueur de bande à établir en phase 4).
- Export `.ti3` puis `colprof` : réglages exposés (TAC, GCR/génération du noir, intentions, qualité) ; contrôle `profcheck` et rapport ΔE de l'auto-cohérence.
- Validation : profil comparé à un profil de référence (par exemple FOGRA39 sur une impression certifiée, ou un profil produit par un autre outil à partir des mêmes mesures).

*Sortie : profil ICC CMJN produit de bout en bout à partir de mesures MYIRO-1 et FD-9.*

### Phase 8 — Application et livraison

- Interface : assistant de mesure de charte, mesure ponctuelle, densitométrie, création de profil, historique et rapports.
- Configuration : emplacement des DLL KM du poste, choix x86/x64 du pont.
- Documentation utilisateur en français ; procédure de sauvegarde des mesures.

*Sortie : application utilisable sans Myiro Tool pour le flux complet charte → profil.*

---

## 5. Risques et parades

| Risque | Parade |
|---|---|
| Altération ou blocage de l'instrument | Liste blanche dans les ponts ; aucun `JIG_*`/`FDX_JIG_*`/écriture `Set*` ; progression par étapes avec accord ; ne jamais utiliser MY-CT1 pour des essais |
| ABI mal reconstitué (plantage, données fausses) | Processus séparé ; fiches avec preuves ; comparaison systématique avec un logiciel KM ou tiers |
| Clé de licence exigée par FD9SDK | Trancher en phase 2 ; à défaut, rester sur FD-S2w pour le FD-9 |
| Dépendances x64 FD9 absentes de l'archive | Archivage en phase 0 |
| Licences KM (aucun droit de redistribution) | Utiliser les DLL installées du poste ; ne rien publier qui les contienne |
| Licence AGPL d'ArgyllCMS | Processus externe ; usage personnel ; revoir si distribution |
| Calibration blanc et état de l'instrument inconnus | Vérifier l'état de la céramique et l'historique ; comparer avec la mesure MYIROtools de référence |

---

## 6. Décisions à prendre

1. **Priorité d'instrument — tranché** : MYIRO-1 d'abord ; FD-9 étudié et développé en parallèle.
2. **Langage — tranché** : Rust ; reste à choisir le cadre d'interface (Tauri 2 recommandé).
3. **Usage — tranché le 6 octobre 2026** : projet **libre, open source, non commercial**, destiné aux imprimeurs et utilisateurs privés d'outil par l'arrêt du support Konica. Conséquences :
   - licence recommandée : **GPL-3.0** (ou AGPL-3.0), compatible avec ArgyllCMS appelé en processus externe ou même intégré ;
   - le code public ne contient que du travail original : ponts, en-têtes reconstitués, documentation d'ABI, cœur colorimétrique ;
   - l'utilisateur indique que Konica Minolta ne fait plus valoir de licence sur ces SDK (réponse obtenue par l'utilisateur, non vue dans ce dossier). Tant que cette réponse n'est pas **écrite et archivée** dans le dépôt, l'application ne redistribue pas les DLL ni les manuels KM et les trouve dans les installations existantes de chaque utilisateur (FD-S2w, Ergosoft, EIZO…) ; avec un accord écrit, un paquet incluant les DLL devient envisageable ;
   - les mires XML FD-S2w et les tables ISO 5-3 suivent la même règle : on les lit chez l'utilisateur, on ne les copie pas dans le dépôt sans droit établi.
4. **FD-S2w — tranché** : FD-S2w pilote encore le FD-9 sur ce poste, **en réseau** (FD-9 sur le réseau local, adresse MAC au préfixe Konica Minolta 00:20:6B), avec mesure M0/M1/M2, données spectrales, densité statut E, D50/2°. Il sert de référence et de secours.

---

## 7. Prochaines actions immédiates

1. Phase 0 : `git init`, `.gitignore`, mise à jour de `CLAUDE.md`, archivage des dépendances x64 FD9.
2. Phase 1 : désassembler les sites d'appel FDX dans `libmeasurementdevice_x64.dll` et rédiger les six premières fiches (`GetSDKVersion`, `GetDevicePortList`, `Connect`, `GetDeviceInfo`, `GetError`, `Disconnect`).
3. Phase 2 : ajouter FD9SDK et FD-S2w.exe à `desassemble.py` et trancher la question de `FD-SDK.lic`.
