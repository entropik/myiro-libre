# Fiches d'ABI : FDXSDK (MYIRO-1)

Une fiche par fonction de `FDXSDK.dll`, établie par lecture statique des DLL et de leurs appelants (MY-CT1, EIZO ColorNavigator, Ergosoft). Aucune DLL n'a été chargée et aucun instrument n'a été interrogé pour écrire ces fiches.

Chaque affirmation porte un niveau :

- **confirmé** : lu directement dans le code du SDK et, sauf mention, recoupé par un appelant ou une seconde version de la DLL ;
- **supposé** : déduction plausible, non démontrée ;
- **à vérifier sur l'instrument** : seule une exécution réelle peut trancher.

Une valeur passe de « supposé » à « confirmé » dans la provenance d'une mesure (ADR 0005) quand sa fiche l'est.

Les preuves citées (`Audit-MYIRO/retroanalyse/…`) sont des désassemblages de binaires propriétaires : elles restent sur le poste et ne sont pas publiées. Les fiches ne recopient pas de code du fabricant ; les adresses données sont des adresses préférées de module (`VA`), utiles pour retrouver un passage.

## Versions étudiées

| Repère | Fichier | Version | Architecture |
|---|---|---|---|
| `fdx-x86` | MY-CT1 `FDXSDK.dll` | 1.0.1.0 | x86 |
| `fdx-x64-101` | Ergosoft `FDXSDK.dll` | 1.0.1.0 | x64 |
| `fdx-x64-103` | EIZO `FDXSDK.dll` | 1.0.3.0 | x64 |
| MYIRO tools 1.5.0 Windows | `FDXSDK.dll` | 1.0.3.0 | x64 (identique à EIZO, signature mise à part) |
| MYIRO tools 1.5.0 Mac | `libFDXSDK.dylib` | 1.0.5 | x86_64 + arm64, **symboles conservés** |
| MY-CT1 1.1.0 Mac | `libFDXSDK.dylib` | 1.1.0 | x86_64, **symboles conservés** |

Les bibliothèques Mac ont gardé les noms de leurs fonctions internes, avec les types des arguments, et les messages de journal nomment les champs des structures : elles confirment la plupart des signatures reconstituées sur Windows. Les mêmes 91 exports existent dans toutes les versions.

## Règles communes à toutes les fonctions

- **Convention d'appel x86 : `__stdcall`** (confirmé : chaque export dépile ses arguments, `ret N`). Le callback d'événements est en `__cdecl`, lui.
- **Retour : un code signé de 32 bits**, pas un booléen (confirmé). Négatif : erreur (MY-CT1 teste le signe). Positif ou nul : succès, éventuellement avec un avertissement.
- **Mécanisme du code** (confirmé) : chaque export remet à zéro une erreur globale en entrant, la renseigne en cas d'échec, puis renvoie cette erreur si elle existe, sinon une seconde valeur globale (statut ou avertissement, sens non établi). `FDX_GetError` relit le même état.
- **Session globale** (confirmé) : aucune fonction ne reçoit de « poignée » d'instrument ; la DLL gère un seul instrument à la fois. Accès concurrents et réentrance : non établis, donc interdits par le pont (un appel à la fois).
- **État interne** (confirmé) : la DLL tient un numéro d'état. Plusieurs fonctions refusent d'agir selon cet état. Le sens de chaque numéro n'est pas établi ; l'état 1 est le seul où `GetDeviceInfo` répond (supposé : « connecté, au repos »).

## Codes d'erreur rencontrés

| Code | Hexadécimal | Sens | Niveau |
|---|---|---|---|
| -9992 | `0xFFFFD8F8` | argument invalide (pointeur nul, valeur hors bornes) | valeur confirmée ; sens confirmé par les contrôles d'arguments et les journaux du SDK Mac |
| -9986 | `0xFFFFD8FE` | opération interdite dans l'état actuel (« device is connected. », « can not use. now status:%d ») | valeur confirmée ; sens tiré des journaux |
| -9983 | `0xFFFFD901` | étalonnage requis (renvoyé par `SetMeasureCondition` en 1.0.5) | valeur confirmée ; sens tiré des journaux |

Les codes existent par plages : -9999 à -9981 (général), -9899 à -9892 (reconnaissance de bande), -9793 à -9789. Leur inventaire complet, avec libellés supposés, est dans `retroanalyse/logiciels/my-ct1.md` et `myiro-tools.md`.

## Fiches

1. [FDX_GetSDKVersion](FDX_GetSDKVersion.md) : version de la DLL (palier Version)
2. [FDX_GetDevicePortList](FDX_GetDevicePortList.md) : liste des instruments visibles (palier Détection)
3. [FDX_Connect](FDX_Connect.md) : ouverture de la session (palier Connexion)
4. [FDX_GetDeviceInfo](FDX_GetDeviceInfo.md) : identité de l'instrument (palier Connexion)
5. [FDX_RegisterDeviceEventHandler](FDX_RegisterDeviceEventHandler.md) : événements de l'instrument (tous les paliers à partir de la connexion)
6. [FDX_Calibration](FDX_Calibration.md) : étalonnage sur le blanc (palier Étalonnage)
7. [FDX_SetMeasureCondition](FDX_SetMeasureCondition.md) : armement d'une mesure ponctuelle ou d'une bande (palier Mesure ponctuelle)
8. [FDX_StopMeasurement](FDX_StopMeasurement.md) : désarmement, retour au repos
9. [FDX_GetMeasureData](FDX_GetMeasureData.md) : lecture des spectres M0/M1/M2 et des données brutes
