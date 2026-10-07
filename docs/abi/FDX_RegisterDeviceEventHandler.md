# FDX_RegisterDeviceEventHandler

**En clair** : donne à la DLL une fonction à appeler quand quelque chose se passe sur l'instrument (étalonnage terminé, mesure prête, bouton pressé, câble débranché). C'est ainsi que le logiciel apprend qu'un étalonnage a réussi ou qu'une bande vient d'être lue.

## Signature

```c
typedef void (__cdecl *FDX_EventCallback)(int32_t evenement,        /* FDX_eEventCode */
                                          uint32_t nb_donnees_brutes,
                                          int32_t erreur);           /* FDX_GetError() à cet instant */
int32_t __stdcall FDX_RegisterDeviceEventHandler(FDX_EventCallback rappel); /* x86 : ret 4 */
```

## Ce qui est confirmé

- Un seul argument : le pointeur de la fonction de rappel. **Aucun contexte utilisateur** n'est transmis : le rappel doit retrouver ses données par une variable globale.
- Le rappel est en **`__cdecl`** (pas en `__stdcall` comme les exports) : signature lisible dans EIZO (`void __cdecl EventCallback(enum FDX_eEventCode, unsigned int, enum FDXSDK_ERROR_TYPES)`).
- Le rappel est appelé **depuis un fil d'exécution de la DLL** (`CEventNotice::threadEntry`), pas depuis celui qui a lancé la commande.
- Le pointeur est conservé par la DLL : il doit rester valide toute la session. MYIRO tools le retire par `FDX_RegisterDeviceEventHandler(NULL)` après `FDX_Disconnect`.
- L'événement est tiré d'une table de transitions entre états internes (`CEventNotice::s_StatusMatrix`, 12 × 12) : `événement = table[ancien état][nouvel état]`.

## Événements

| Code | Transition d'état | Sens | Niveau |
|---|---|---|---|
| 0 | 0→1, 2→1, 3→1 | connecté au repos, ou attente annulée | confirmé (nommé `eEventCode_Standby` par MY-CT1) |
| 1 | 1→2, 4→2, 5→2 | mesure armée, l'instrument attend le bouton | confirmé (`eEventCode_MeasureWait`, MYIRO tools) |
| 2 | 2→3 | mesure commencée, données brutes en flux | transition confirmée |
| 3 | 3→4 | **mesure terminée, données prêtes** ; 2ᵉ argument = nombre de données brutes | confirmé (`eEventCode_MeasureComplete`) |
| 4 | 3→5 | mesure échouée | confirmé (`eEventCode_MeasureFailed`) |
| 5 | vers 8 | état 8 : sens divergent selon les sources (sans réponse, ou inconnu) | transition confirmée |
| 6 | vers 11 | **déconnexion** ou perte de l'instrument | confirmé (nommé `eEventCode_DisConnect` par MY-CT1) |
| 7 | 1→7 | **étalonnage commencé** | confirmé (journal « Starting calibration ») |
| 8 | 7→9 | **étalonnage réussi** | confirmé (« Calibration Ok ») |
| 9 | 7→10 | **étalonnage échoué** | confirmé (« Calibration failed ») |

États internes (numéros confirmés) : 0 déconnecté, 1 repos, 2 attente de mesure, 3 mesure, 4 et 5 mesure réussie ou échouée, 6 réglage de l'appareil, 7 étalonnage, 9 et 10 étalonnage réussi ou échoué ; 8 et 11 : absence de réponse et déconnexion, attribution divergente entre les deux SDK Mac étudiés.

## Pour le pont

- Enregistrer le rappel **avant** `FDX_Connect`, comme EIZO et MYIRO tools.
- Le rappel ne fait que **déposer** l'événement dans une file protégée et rendre la main aussitôt ; le pont lit la file. Jamais d'appel à la DLL depuis le rappel (réentrance non établie).
- Retirer le rappel (`NULL`) après `FDX_Disconnect`, avant de décharger la DLL.
- Journaliser chaque événement avec ses trois valeurs.

## Vérifié sur l'instrument

Le 7 octobre 2026 (DLL 1.0.1.0 x64, MYIRO-1 en USB) : le rappel `__cdecl`, enregistré avant la connexion, reçoit l'événement **0** à la connexion, puis **7** et **8** pendant un étalonnage réussi, avec un code d'erreur nul. Le retrait du rappel (`NULL`) à la fermeture ne provoque aucune erreur ; même comportement de connexion en x86.

Mesure ponctuelle, même jour : **1** à l'armement, **2** répété à chaque donnée brute pendant la mesure, **3** à la fin, puis **1** (réarmement automatique) ; après `FDX_StopMeasurement`, **0** (retour au repos).

## Reste à vérifier

- L'événement reçu quand on presse le bouton de l'instrument, et quand on débranche le câble.

## Preuves (locales)

- `fdx-x86/exports/FDX_RegisterDeviceEventHandler.asm.txt` (`ret 4`) ; `preuves/eizo-x64/callback.asm.txt` et `preuves/eizo-x86/callback-ret.asm.txt` (rappel `__cdecl`).
- `retroanalyse/logiciels/myiro-tools.md` § 10 et `retroanalyse/logiciels/my-ct1.md` § 5.8 : table des transitions, fil d'exécution, noms d'événements.
