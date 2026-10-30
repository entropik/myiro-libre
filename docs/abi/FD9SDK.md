# Fiches d'ABI : FD9SDK (FD-9)

Une fiche par fonction de `FD9SDK.dll` utilisée par le pont FD-9 : paliers Version, Détection et Connexion, puis Prénumérisation et Feuille (mesure d'une feuille, ticket #34). Elles sont établies par lecture statique de la DLL Windows (x86 et x64), de la bibliothèque Mac de FD-S2w, qui a gardé les noms de ses fonctions internes et leurs types, et des appels de FD-S2w et de MYIROtools. Aucune DLL n'a été chargée et aucun instrument n'a été interrogé pour les écrire.

Les niveaux sont ceux des fiches du MYIRO-1 ([README](README.md)) :

- **confirmé** : lu directement dans le code du SDK, avec l'adresse citée ;
- **supposé** : déduction plausible, non démontrée ;
- **à vérifier sur l'instrument** : seule une exécution réelle peut trancher.

Les preuves citées (`Audit-MYIRO/retroanalyse/…`) sont des désassemblages de binaires propriétaires : elles restent sur le poste et ne sont pas publiées. Les fiches ne recopient pas de code du fabricant. Les adresses sont des adresses préférées de module (`VA`).

Les désassemblages des exports se régénèrent avec `python -I Audit-MYIRO/outils/desassemble.py fd9-x86 fd9-x64` (les exports `JIG_*` sont sautés), et une fonction interne avec `--address`.

## Versions étudiées

| Repère | Fichier | Version | Architecture |
|---|---|---|---|
| `fd9-x86` | FD-S2w 1.6.0 `Module/FD9SDK.dll` | 1.3.2.3 | x86 |
| `fd9-x64` | Ergosoft `FD9SDK.dll` | 1.3.1.5 | x64 |
| FD-S2w 1.6.1 Mac | `libFD9SDK.dylib` | 1.3.2.3 | x86_64, **symboles conservés** |
| `mit-x64` | MYIROtools 1.5.0.2 (appelant seulement) | — | x64 |

Les deux DLL Windows ont les mêmes 125 exports : 38 fonctions `FD9_*`, 86 fonctions `JIG_*` (maintenance usine et tests de l'appareil), jamais appelées, et une donnée, `gSDKLog` (variable du journal du SDK, rôle supposé d'après son nom), qui n'est pas une fonction et n'est jamais résolue par le pont.

FD9SDK est un SDK distinct de FDXSDK : noms, codes d'erreur, structures et transport réseau diffèrent. Le nom de code interne du FD-9 est « Jungfrau ». `FD9SDK.dll` importe `DIColor.dll`, posée à côté par FD-S2w, pour ses calculs de couleur.

## Règles communes

- **Convention d'appel x86 : `__stdcall`** pour les exports de ces fiches (confirmé : `ret N` égal à 4 octets par argument), **sauf [FD9_GetPremeasurementError](FD9_GetPremeasurementError.md), en `__cdecl`**. `FD9_WriteBmpFile`, hors liste, est aussi en `__cdecl`. Le rappel d'événements est appelé en `__cdecl`.
- **Les scalaires d'entrée passent par pointeur** (confirmé) : par exemple, la capacité de `FD9_GetDeviceList` est un `const uint32_t *`. Le code C++ d'origine utilise des références. **Beaucoup de ces pointeurs ne sont pas testés contre le pointeur nul** (`FD9_SetOption`, `FD9_StartMeasurement2`, `FD9_StopMeasurement`, `FD9_ExitMeasurement`, le mode de `FD9_StartPatchRecognition`…) : le pont passe toujours des pointeurs valides.
- **Session globale** (confirmé) : aucune poignée d'instrument ; un seul instrument à la fois par processus. Un drapeau interne vaut 0 hors connexion, 1 connecté en réseau, 2 connecté en USB (`fd9-x86` `0x1008aa3c..0x1008aa44`).
- **État du SDK** (confirmé) : une variable de la DLL (`fd9-x86` `0x1022235c`) garde l'état courant, de la même liste que les événements du rappel (`FD9_eSDKStatus`, table de noms `0x101086d0`). Chaque fonction de la mesure d'une feuille n'est admise que dans certains états (table ci-dessous) ; ailleurs elle rend 1104 sans rien faire.
- **Retour : un code public regroupé**, 0 = succès (confirmé). Chaque export passe son code interne à une fonction de regroupement (`fd9-x86` `0x100b43b0`) qui le cherche dans une table de 32 groupes (`0x10108ca0`, identique dans la bibliothèque Mac) et renvoie le numéro du groupe ; un code absent de la table devient **1999**. Deux exceptions : `FD9_GetPremeasurementError` rend sa propre valeur, et `FD9_GetData` réussi rend 0 sans passer par le regroupement.
- **Propriété de la mémoire** : chaque fiche a sa rubrique. En résumé, la DLL ne garde aucun pointeur de l'appelant après le retour, **sauf le rappel d'événements**, relu à chaque événement ; elle recopie les structures qu'on lui donne (`FD9_StartPatchRecognition` comprise). À l'inverse, **le texte rendu par `FD9_GetPremeasurementError` appartient à la DLL**.
- **`FD9_GetLastError` rend le code interne d'origine**, pas le code regroupé (confirmé en x86 et en x64). C'est le seul moyen de distinguer deux échecs du même groupe.

## Codes publics rencontrés dans ces fiches

| Code | Sens | Codes internes d'origine relevés | Niveau |
|---|---|---|---|
| 0 | succès | — | confirmé |
| 1001 | paramètre invalide | 1001 (contrôles des exports, et tout refus de `FD9_SetOption`), 2001 (liaison inconnue), 2002 (adresse vide), 8021 (calcul impossible depuis un spectre) | valeurs confirmées |
| 1002 | échec réseau pendant la détection | 12051 à 12057 (sockets de la détection) | valeurs confirmées, sens supposé |
| 1003 | instrument injoignable | 4001 (échec de la connexion TCP), 4010 (échec de la résolution du nom) | valeurs confirmées, sens supposé d'après l'appel qui précède |
| 1004 | aucun instrument connecté (`FD9_Disconnect`) | 8054 | valeurs confirmées, sens tiré du code de `FD9_Disconnect` |
| 1005 | tableau de sortie trop court | 8012 (`FD9_GetData`), 8022 (`FD9_CalculateData`) | valeurs confirmées, sens tiré des contrôles |
| 1102 | aucun instrument connecté (fonctions de la feuille) | 8009, 8013, 8015, 8017, 8023, 8065, 8077, 8082, 8084 | valeurs confirmées ; sens confirmé : chacun est rendu quand le drapeau de session vaut 0 |
| 1103 | appel interdit pendant une connexion | 8055 | valeurs confirmées, sens tiré du code |
| 1104 | **appel refusé dans l'état actuel du SDK** | 8014, 8016, 8018, 8024, 8050, 8062, 8064, 8066, 8085 | valeurs confirmées ; sens confirmé par les contrôles d'état de la mesure d'une feuille |
| 1901 | **connecté, recertification due** | 8011 | confirmé : MYIROtools et FD-S2w le traitent comme un succès ; MYIROtools journalise « Instrument should be recertified » |
| 1999 | code interne absent de la table | par exemple 10002108, 10002109 (voir `FD9_Connect`), 2056 (échec du calcul de `DIColor.dll`) | confirmé |

**1901 n'est pas un étalonnage à faire par l'opérateur** : c'est l'échéance de la recertification périodique de l'instrument (supposé : vérification par le fabricant). La connexion est faite et la mesure reste possible. Les autres groupes (1006 à 1010, 1101, 1105 à 1113, 1201 à 1209, 1902) existent mais leur sens n'est pas établi.

## Transport

- **USB** (confirmé) : port série virtuel (`COMn`). L'instrument est retenu si son identifiant Windows commence par `USB\VID_132B&PID_210D\` suivi de `9C1A` ou `A8AN`. Supposé : les deux préfixes distinguent FD-9 et MYIRO-9.
- **Réseau** (confirmé) : détection par diffusion UDP, commandes en TCP. Les deux utilisent le **port 49152**. Voir [FD9_GetDeviceList](FD9_GetDeviceList.md) et [FD9_Connect](FD9_Connect.md).
- **Journal du SDK** (confirmé sur Mac et dans la DLL x86 1.3.2.3, `0x1008dfe0`) : si un dossier `./DebugImage` ou `./DebugSDK` (textes `0x10118f5c`, `0x10118f6c`) existe dans le **dossier courant** du pont, le SDK y crée un sous-dossier horodaté `AAAAMMJJ_HHMMSS` et y écrit `$SDKLog_AAAAMMJJ_HHMMSS.txt`, qui trace les appels, leurs arguments et le travail interne (par exemple `CPatchRecognitionThread::thread start`, `CJungfrauCore::PatchRecognition(...)`). Ouverture : une fonction interne appelée de façon indirecte (`0x1008e8d0`), `FD9_StartPreMeasurement` et **`FD9_StartCapture`** (`0x100b1db5`) : une feuille glissée l'ouvre donc au plus tard à la capture. Supposé : chaque ligne est vidée aussitôt (`0x100b6191` après chaque écriture), donc le journal survit à un plantage. Préférer `DebugSDK` : `DebugImage` laisse supposer des images enregistrées. Ces dossiers sont ignorés par git : le journal contient l'adresse et l'identifiant de l'instrument. Le pont ne les crée jamais lui-même.
- **Diagnostic du pont** : un plantage est toujours rapporté sur la sortie d'erreur (code de l'exception, module et décalage de l'adresse fautive, accès, fil), sans fenêtre de Windows ; `--trace` écrit chaque appel à la DLL avant et après, chaque entrée et sortie du rappel, et les exceptions graves dès leur premier passage ; `--vidage <fichier.dmp>` écrit un vidage mémoire au plantage (local, jamais versionné). Une adresse `FD9SDK.dll+0x…` se retrouve dans les désassemblages en ajoutant 0x10000000.

## Mesurer une feuille

### Séquence

Référence : **MYIROtools**, le logiciel du fabricant qui imprime la mire RGB-2520 et la mesure au FD-9 (confirmé, gestionnaire d'événements `mit-x64` `0x1403eaca0`). Il ne crée pas de travail (`FD9_RegisterJob`) et n'appelle ni `FD9_MatchingChart` ni `FD9_ClippingChart` (confirmé, table d'import). FD-S2w suit la même trame, avec un travail.

1. [FD9_RegisterDeviceEventHandler](FD9_RegisterDeviceEventHandler.md), puis [FD9_Connect](FD9_Connect.md) : succès si 0 ou 1901 ; puis [FD9_GetSystemInfo](FD9_GetSystemInfo.md) et [FD9_GetStatus](FD9_GetStatus.md).
2. L'opérateur glisse la feuille. Événement 3 : [FD9_StartCapture](FD9_StartCapture.md).
3. Événement 5 : [FD9_GetCaptureResult](FD9_GetCaptureResult.md) deux fois (taille, puis image et QR) ; contrôle du QR.
4. [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md) en mode **COMBINE** (4), avec la définition de la mire **et les couleurs RVB de ses plages** (0 à 255), comme MYIROtools et FD-S2w : la feuille est retrouvée sur l'image. Sans couleurs, COMBINE fait planter la DLL (essai E3) ; COORDINATE mesure à côté d'une feuille décalée (essai E4 bis) : ni l'un ni l'autre.
5. Événement 7 : [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md) (fiabilité non nulle : refus, comme FD-S2w), **comparaison à la définition** (zones, plages et pas), puis [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md) : les points visés, trouvés sur l'image, chaque plage une fois, décalage de la feuille ≤ 21 mm, écart à la grille ≤ 3 mm. Au palier Prénumérisation, la feuille s'arrête là : [FD9_StopMeasurement](FD9_StopMeasurement.md)`(faux)`, puis attente de l'événement 10.
6. Palier Feuille : [FD9_StartMeasurement2](FD9_StartMeasurement2.md)`(1, 1)`.
7. Événements 11 : nombre de plages mesurées. Événement 9 : mesure finie.
8. Lecture de chaque plage : [FD9_SetOption](FD9_SetOption.md)`(3, 0x0c)` et [FD9_GetData](FD9_GetData.md) (152 données brutes), **dans l'ordre de mesure de l'instrument** (serpentin à l'essai E4), puis rangement par la position rendue dans l'ordre de la mire (chaque position une fois), puis M0, M1, M2 par [FD9_CalculateData](FD9_CalculateData.md), puis **contrôle de vraisemblance** : les L* M1 doivent suivre la clarté des couleurs imprimées (corrélation ≥ 0,5, sinon `mesure_invraisemblable`, rien n'est rendu).
9. [FD9_ExitMeasurement](FD9_ExitMeasurement.md)`(0, NULL)`. À la fermeture de la session : [FD9_Disconnect](FD9_Disconnect.md), puis désinscription du rappel.

### États admis par chaque fonction (confirmé)

| Fonction | États admis | Sinon |
|---|---|---|
| `FD9_StartCapture` | 3 | 1104 |
| `FD9_GetCaptureResult` | 5 | 1104 |
| `FD9_StartPatchRecognition` | 5 ou 7 | 1104 |
| `FD9_GetPatchRecognitionResult2`, `FD9_GetMeasurePoint`, `FD9_StartMeasurement2` | 7 | 1104 |
| `FD9_StopMeasurement` | 1 à 8 | 1104 |
| `FD9_ExitMeasurement` | 9, 10, 101 à 103, 201 à 206, 301 | 1104 |
| `FD9_GetStatus` | tout état connecté | — |
| `FD9_SetOption`, `FD9_CalculateData`, `FD9_GetPremeasurementError` | aucun contrôle d'état ni de connexion | — |
| `FD9_GetData` | aucun contrôle dans l'export ; lecture des données non suivie | — |

Conséquence : **une annulation se fait en deux temps** (`FD9_StopMeasurement` dans les états 1 à 8, puis `FD9_ExitMeasurement` après l'événement 10), et **à l'événement 9, `FD9_StopMeasurement` est refusé** si l'état est encore 9. MYIROtools l'appelle quand même à cet endroit, après une seconde d'attente, puis appelle `FD9_ExitMeasurement` ; le pont, lui, passe directement à `FD9_ExitMeasurement` (supposé équivalent, à vérifier à l'essai E4).

### États du rappel et action du pont

Noms confirmés dans la DLL x86 (table `0x101086d0`, 22 entrées) et dans la bibliothèque Mac. Actions de MYIROtools confirmées dans son gestionnaire d'événements ; celles du pont sont les choix du plan de la mesure d'une feuille.

| État | Nom | En clair | MYIROtools | Pont |
|---|---|---|---|---|
| 0 | `WAIT_NO_AUTHORITY` | main pas encore obtenue : passage normal juste après la connexion (essai E3 : 0 puis 1) ; s'il dure, un autre logiciel tient peut-être l'instrument | rien | journaliser ; continuer d'attendre jusqu'au délai de la feuille |
| 1 | `WAIT_AUTHORITY` | main obtenue, attente d'une feuille | remise à zéro de ses drapeaux | attendre la feuille |
| 2 | `RUN_FEED` | feuille en cours d'engagement | rien | attendre |
| 3 | `WAIT_FEED_COMPLETION` | feuille engagée | `FD9_StartCapture` (depuis un autre fil) | `FD9_StartCapture` |
| 4 | `RUN_CAPTURE` | image en cours | — | attendre |
| 5 | `WAIT_CAPTURE_COMPLETION` | image prise | `FD9_GetCaptureResult` ×2, puis reconnaissance | `FD9_GetCaptureResult` ×2, contrôle du QR, `FD9_StartPatchRecognition` (COORDINATE) ; autre QR : arrêt (`AutreMire`) |
| 6 | `RUN_PATCHRECOG` | reconnaissance en cours | — | attendre |
| 7 | `WAIT_PATCHRECOG_COMPLETION` | plages reconnues | résultat, comparaison, `FD9_GetMeasurePoint`, `FD9_StartMeasurement2(1, 1)` | résultat et comparaison (sinon arrêt, `MireNonReconnue`) ; Prénumérisation : `FD9_StopMeasurement(faux)` ; Feuille : `FD9_GetMeasurePoint`, `FD9_StartMeasurement2(1, 1)` |
| 8 | `RUN_MEASURE` | mesure en cours | — | attendre |
| 9 | `WAIT_MEASURE_COMPLETION` | mesure finie | 1 s, `FD9_StopMeasurement(vrai)`, `FD9_ExitMeasurement(0, NULL)` | lire et calculer toutes les plages, puis `FD9_ExitMeasurement(0, NULL)` |
| 10 | `WAIT_CANCEL_COMPLETION` | arrêt fini | `FD9_ExitMeasurement(0, NULL)` | `FD9_ExitMeasurement(0, NULL)`, puis rendre la raison de l'arrêt (ou la lecture de la feuille au palier Prénumérisation) |
| 11 | `NOTYFY_PATCHCOUNT` | nombre de plages déjà mesurées (4ᵉ argument) | garde le compte | compte journalisé, avancement à l'écran |
| 101 | `ERROR_COVEROPEN` | capot ouvert | `FD9_ExitMeasurement(0, NULL)` | `FD9_ExitMeasurement(0, NULL)`, `IncidentFeuille { etat: 101 }` |
| 102 | `ERROR_PAPERJAM` | bourrage | idem | idem, `etat: 102` |
| 103 | `ERROR_COMMUNICATION` | liaison perdue | idem | `FD9_ExitMeasurement(0, NULL)`, `InstrumentPerdu` |
| 201 | `ERROR_PREMEASURE` | échec de la préparation (image, reconnaissance) | `FD9_GetPremeasurementError(0, 0)` journalisé, puis `FD9_ExitMeasurement` | `FD9_GetPremeasurementError`, `FD9_ExitMeasurement(0, NULL)`, `IncidentFeuille { etat: 201, premesure }` |
| 202 | `ERROR_MEASURE` | échec de la mesure | `FD9_ExitMeasurement(0, NULL)` | `FD9_ExitMeasurement(0, NULL)`, `IncidentFeuille` |
| 203 | `ERROR_SENSOR` | erreur du capteur | idem | idem |
| 204 | `ERROR_CALCULATION` | erreur de calcul | idem | idem |
| 205 | `ERROR_CHARTTOOLONG` | feuille trop longue | idem | idem |
| 206 | `ERROR_MEMORY` | mémoire insuffisante | idem | idem |
| 301 | `ERROR_FATAL` | erreur fatale | idem | idem |
| autre | — | inconnu | — | arrêt sans rien inventer : `ReponseInattendue`, puis `FD9_StopMeasurement(faux)` ou `FD9_ExitMeasurement` selon ce que l'état admet |

Les noms complets portent le préfixe `SDK_` ou `SDK_WAIT_` (par exemple `SDK_WAIT_ERROR_PAPERJAM`). Un appel refusé en cours de route amène MYIROtools à `FD9_StopMeasurement(faux)`.

## Fiches

Session et identité :

1. [FD9_GetLastError](FD9_GetLastError.md) : dernier code interne ; seul appel du palier Version
2. [FD9_GetDeviceList](FD9_GetDeviceList.md) : instruments visibles en USB et sur le réseau (palier Détection)
3. [FD9_RegisterDeviceEventHandler](FD9_RegisterDeviceEventHandler.md) : rappel d'événements, à poser avant la connexion
4. [FD9_Connect](FD9_Connect.md) : ouverture de la session et **paramètres de connexion réseau** (palier Connexion)
5. [FD9_GetSystemInfo](FD9_GetSystemInfo.md) : identité de l'instrument et version du SDK (palier Connexion)
6. [FD9_GetStatus](FD9_GetStatus.md) : état courant, sans question à l'instrument
7. [FD9_Disconnect](FD9_Disconnect.md) : fermeture de la session

Mesure d'une feuille (paliers Prénumérisation et Feuille) :

8. [FD9_StartCapture](FD9_StartCapture.md) : prise d'image de la feuille engagée
9. [FD9_GetCaptureResult](FD9_GetCaptureResult.md) : QR code et image de la feuille
10. [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md) : reconnaissance des plages, description de la mire
11. [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md) : géométrie reconnue, à comparer à la mire
12. [FD9_GetPremeasurementError](FD9_GetPremeasurementError.md) : cause d'un échec de préparation (`__cdecl`)
13. [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md) : points de mesure, avant le lancement
14. [FD9_StartMeasurement2](FD9_StartMeasurement2.md) : lancement de la mesure
15. [FD9_StopMeasurement](FD9_StopMeasurement.md) : demande d'arrêt (annulation)
16. [FD9_ExitMeasurement](FD9_ExitMeasurement.md) : clôture d'une feuille finie, annulée ou en erreur
17. [FD9_SetOption](FD9_SetOption.md) : réglages du calcul, **aucun échange avec l'instrument**
18. [FD9_GetData](FD9_GetData.md) : données brutes d'une plage (152 valeurs)
19. [FD9_CalculateData](FD9_CalculateData.md) : spectres et L\*a\*b\* M0, M1, M2, calculés dans la DLL

Les fiches 1 à 5 et 7 couvrent la liste blanche actuelle de `crates/fd9-sys`. Les autres préparent la mesure d'une feuille : leurs exports n'entrent dans la liste blanche qu'avec elle (ticket #34), chacun avec sa fiche. Restent exclus : tous les `JIG_*`, `FD9_TAConnect` (connexion prioritaire), `FD9_SetNetworkSetting`, `FD9_RegisterUserIlluminant`, `FD9_RegisterJob`, `FD9_DeleteJob`, `FD9_GetJobList`, `FD9_MatchingChart`, `FD9_ClippingChart`, `FD9_GetChartImage`, `FD9_RegisterDisplayImage`, `FD9_CreateBarcodeImage`, `FD9_CreateChartImage`, `FD9_DeleteChartImage`, `FD9_WriteBmpFile`, `FD9_StartMeasurement` (première version), `FD9_StartPreMeasurement`, `FD9_GetPatchRecogResult`, `FD9_GetNetworkSetting` et `FD9_GetOption`.

## Preuves (locales)

- `retroanalyse/fd9-x86/` et `retroanalyse/fd9-x64/` : `metadata.json` (exports, imports) et `exports/*.asm.txt`.
- `retroanalyse/logiciels/fd-s2w.md` : étude de FD-S2w 1.6.0 et 1.6.1, signatures tirées des symboles Mac (§ 3 et 4), séquence de FD-S2w (§ 5), table des codes (§ 6), transport (§ 7).
- `retroanalyse/logiciels/fd9-feuille.md` : étude de la mesure d'une feuille (28 octobre 2026) : séquence de MYIROtools (§ 3), définition de la RGB-2520 (§ 2, nombres seulement), contrôles de la DLL (§ 4).
- Codes publics : table de regroupement `fd9-x86` `0x10108ca0` (groupes de 100 mots : code public, puis codes internes jusqu'à −1), lue dans le fichier.

## Hors liste blanche

Exports étudiés pour la fonction « Measured Point » de FD-S2w, **jamais appelés par le pont** (étude statique du 30 octobre 2026) :

- `FD9_GetChartImage(tFD9_Image *image, tFD9_PatchMeasurePoint *points, const uint32_t *nombre)` (`fd9-x86` `0x100b17d0`) : l'image de la feuille prise, refusée aux états 1 à 6 (0x1f74). FD-S2w l'appelle à l'événement 7, sans points, pour l'onglet « Measured Point » (`s2w-x86` `0x41ed00`). Candidat pour montrer la feuille dans l'application.
- `FD9_MatchingChart(tFD9_ChartCoordinate *sortie, tFD9_PatchMeasurePoint *points, const uint32_t *nombre, const tFD9_ChartCoordinate *entree)` (`0x100b1730`, travail `0x1009d390`) : à l'état 7 seulement, recale une définition sur l'image prise et rend coordonnées et points. FD-S2w ne l'importe pas.
- FD-S2w renvoie des points corrigés à la main en relançant `FD9_StartPatchRecognition` (mode 1 ou 2) avec une définition corrigée (`ReStartPatchRecognition`, `s2w-x86` `0x41f4c0`).
