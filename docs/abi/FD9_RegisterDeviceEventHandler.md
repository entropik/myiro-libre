# FD9_RegisterDeviceEventHandler

**En clair** : donne à la DLL la fonction du pont qu'elle appellera pour annoncer ce qui se passe dans l'instrument (papier engagé, mesure en cours, capot ouvert…). **Il faut le faire avant la connexion** : une fois connecté, la DLL refuse.

## Signature

```c
typedef void (__cdecl *FD9_Rappel)(uint32_t etat,      /* FD9_eSDKStatus */
                                   uint32_t travail,   /* n° de travail */
                                   uint32_t page,
                                   uint32_t mesurees); /* plages mesurées */
int32_t __stdcall FD9_RegisterDeviceEventHandler(FD9_Rappel rappel);  /* x86 : ret 4 */
```

## Ce qui est confirmé

- Un argument : l'adresse du rappel, gardée telle quelle. **Un pointeur nul désinscrit** : FD-S2w le fait après un échec de connexion.
- **Refus pendant une connexion** : si une session est ouverte, rien n'est enregistré et le code interne 8055 est renvoyé, public **1103** (`fd9-x86` `0x100b153a..0x100b1549`).
- Le rappel est appelé en **`__cdecl`** avec quatre entiers de 32 bits : la DLL dépile elle-même après l'appel (`fd9-x86` `0x100880a6..0x100880a8`). En Rust : `extern "C" fn(u32, u32, u32, u32)` (`GestionnaireEvenements` dans `crates/fd9-sys`).
- Les noms des quatre arguments viennent du symbole de la bibliothèque Mac. Les valeurs de `etat` et leurs noms sont les mêmes dans sa table de noms et dans celle de la DLL x86 (`fd9-x86` `0x101086d0`, 22 entrées) : voir la table ci-dessous.
- Supposé : `etat` est la valeur de la variable d'état du SDK (`0x1022235c`) au moment de l'appel, celle que les fonctions de la mesure d'une feuille testent avant d'agir et que [FD9_GetStatus](FD9_GetStatus.md) rend à `+0x10` (mêmes valeurs, même liste de noms ; le chemin qui pose l'état puis appelle le rappel n'a pas été suivi).

## États annoncés et action du pont

Le détail des appels, et ce que fait MYIROtools à chaque état, est dans [FD9SDK](FD9SDK.md), section « Mesurer une feuille ». Les noms complets portent le préfixe `SDK_` ou `SDK_WAIT_`.

| État | Nom | En clair | Action du pont (dans le fil de la session, jamais dans le rappel) |
|---|---|---|---|
| 0 | `WAIT_NO_AUTHORITY` | un autre logiciel a la main | journaliser ; attendre jusqu'au délai |
| 1 | `WAIT_AUTHORITY` | main obtenue, attente d'une feuille | attendre la feuille |
| 2 | `RUN_FEED` | feuille en cours d'engagement | attendre |
| 3 | `WAIT_FEED_COMPLETION` | feuille engagée | prise d'image ([FD9_StartCapture](FD9_StartCapture.md)) |
| 4 | `RUN_CAPTURE` | image en cours | attendre |
| 5 | `WAIT_CAPTURE_COMPLETION` | image prise | QR et image ([FD9_GetCaptureResult](FD9_GetCaptureResult.md)), puis reconnaissance ([FD9_StartPatchRecognition](FD9_StartPatchRecognition.md)) |
| 6 | `RUN_PATCHRECOG` | reconnaissance en cours | attendre |
| 7 | `WAIT_PATCHRECOG_COMPLETION` | plages reconnues | comparaison à la mire ([FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md)) ; Prénumérisation : arrêt ; Feuille : lancement de la mesure ([FD9_StartMeasurement2](FD9_StartMeasurement2.md)) |
| 8 | `RUN_MEASURE` | mesure en cours | attendre |
| 9 | `WAIT_MEASURE_COMPLETION` | mesure finie | lire les plages ([FD9_GetData](FD9_GetData.md)), puis clôture ([FD9_ExitMeasurement](FD9_ExitMeasurement.md)) |
| 10 | `WAIT_CANCEL_COMPLETION` | arrêt fini | clôture, puis rendre la raison de l'arrêt |
| 11 | `NOTYFY_PATCHCOUNT` | plages déjà mesurées (`mesurees`) | journaliser le compte, avancement |
| 101 | `ERROR_COVEROPEN` | capot ouvert | clôture, erreur nommée |
| 102 | `ERROR_PAPERJAM` | bourrage | clôture, erreur nommée |
| 103 | `ERROR_COMMUNICATION` | liaison perdue | clôture, instrument perdu |
| 201 | `ERROR_PREMEASURE` | échec de la préparation | cause ([FD9_GetPremeasurementError](FD9_GetPremeasurementError.md)), clôture, erreur nommée |
| 202 à 206 | `ERROR_MEASURE`, `ERROR_SENSOR`, `ERROR_CALCULATION`, `ERROR_CHARTTOOLONG`, `ERROR_MEMORY` | échec de mesure, capteur, calcul, feuille trop longue, mémoire | clôture, erreur nommée |
| 301 | `ERROR_FATAL` | erreur fatale | clôture, erreur nommée |
| autre | — | inconnu | arrêt sans rien inventer (`ReponseInattendue`) |

« Clôture » = `FD9_ExitMeasurement(0, NULL)`, admise dans les états 9, 10 et d'erreur seulement.

## Propriété de la mémoire

- **La DLL garde l'adresse du rappel** dans une variable globale (`0x100b1549`) et ne la copie nulle part ailleurs : à la connexion, l'objet d'événements reçoit l'adresse de cette variable (`0x1008aa17`) et **la relit à chaque événement** avant d'appeler (`0x1008806a..0x10088070`) (confirmé).
- **`FD9_Disconnect` n'efface pas cette variable** (aucune écriture dans `0x1008af80..0x1008b006`) (confirmé). Seul un nouvel enregistrement la change, et il est refusé pendant une connexion.
- Conséquence : **le rappel doit rester valide de l'enregistrement jusqu'à la désinscription** (pointeur nul), elle-même possible seulement après `FD9_Disconnect`, ou jusqu'au déchargement de la DLL. Il ne peut pas être désinscrit pendant une session. En Rust : une fonction `extern "C"` statique, jamais une fermeture.
- Les quatre nombres passés au rappel sont des valeurs, pas des pointeurs : rien à libérer (confirmé, `0x1008809a..0x100880a3`).
- `FD9_Disconnect` arrête puis détruit l'objet d'événements créé à la connexion (`0x1008afa3..0x1008afc2`, même objet qu'en `0x1008aa31`) (confirmé). Supposé : l'arrêt attend la fin d'un appel du rappel en cours, si bien qu'aucun appel n'arrive après le retour de `FD9_Disconnect`.

## Ce qui est supposé

- Le rappel est appelé depuis un fil d'exécution de la DLL, pas celui du pont : la connexion crée un objet qui reçoit l'adresse du rappel, et la DLL importe `CreateThread`. Le rappel doit donc être court, ne jamais rappeler la DLL, et transmettre l'événement au fil principal.
- Les états 0 (`WAIT_NO_AUTHORITY`) et 1 (`WAIT_AUTHORITY`) décrivent la **prise de main** : un seul logiciel à la fois pilote l'instrument réseau.

## Pour le pont

- Enregistrer le rappel avant `FD9_Connect`, comme FD-S2w ; le désinscrire (pointeur nul) après un échec de connexion ou après `FD9_Disconnect`, jamais avant.
- Le rappel ne fait que copier les quatre nombres dans une file ; le journal de session les garde bruts. Un rappel reçu sans session active est compté et journalisé.
- La session n'avance que sur un événement reçu ou un code de retour lu ; un état inconnu arrête la feuille en le disant.

## À vérifier sur l'instrument

- Les événements reçus pendant une connexion seule, sans travail ni mesure.
- La suite des événements pour une feuille glissée sans travail (`FD9_RegisterJob` non appelé) : 1, puis 2 et 3 (essai E3).
- Les valeurs de `travail` et `page` sans travail.

## Preuves (locales)

- `fd9-x86/exports/FD9_RegisterDeviceEventHandler.asm.txt` (refus, enregistrement, `ret 4`).
- Appel du rappel : `0x10088097..0x100880ae` (balayage `desassemble.py fd9-x86 --address 0x10088097 --end 0x100880b1`).
- Table de noms des états : `fd9-x86` `0x101086d0` (22 paires valeur, nom), lue dans le fichier.
- `retroanalyse/logiciels/fd-s2w.md` § 3 (symbole Mac), § 4.4 (`FD9_eSDKStatus`), § 5 (ordre des appels de FD-S2w) ; `fd9-feuille.md` § 3 (gestionnaire d'événements de MYIROtools).
