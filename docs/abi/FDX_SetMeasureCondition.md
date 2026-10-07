# FDX_SetMeasureCondition

**En clair** : malgré son nom, cette fonction ne règle rien de permanent : elle **arme** la mesure, comme on arme un appareil photo. Après elle, l'instrument attend qu'on appuie sur son bouton. Le type demandé dit s'il s'agit d'une mesure ponctuelle ou d'une bande.

## Signature

```c
typedef struct { int32_t MeasureType; uint32_t option; } FDX_MeasureCondition; /* 8 octets */
int32_t __stdcall FDX_SetMeasureCondition(const FDX_MeasureCondition *condition); /* x86 : ret 4 */
```

## Ce qui est confirmé

- `MeasureType` (nom du fabricant, `CMeasurement::execMeasurement`) :

  | Valeur | Mesure |
  |---|---|
  | **0** | **ponctuelle**, en réflexion |
  | **1** | **bande**, en réflexion (reconnaissance des plages et du sens de passage) |
  | 2 | irradiance (lumière ambiante) |
  | 3 | radiance (écran) |

- `option` doit être nul sauf pour la bande, où il donne très probablement le nombre de plages attendu (0 = pas de contrôle) ; MYIRO tools passe toujours 0.
- Déroulé, lu dans la DLL Windows 1.0.1 x86 (`CFDX::SetMeasureCondition`, `0x1001cb90`) et identique dans le SDK Mac 1.0.5 :
  1. refus si l'état interne n'est pas 1 (repos) : -9986 ;
  2. refus si l'instrument n'est pas étalonné : **-9983** « is not calibrated. » ;
  3. contrôle du type (0 à 3), sinon « MeasureType is unknown » ;
  4. armement de la mesure, puis attente de l'état 2 (« attente de mesure »), signalé par l'événement **1**.
- Commande envoyée à l'instrument : `0x02` (conditions de mesure) puis démarrage (`0x11`). Aucune écriture en mémoire permanente n'a été trouvée sur ce chemin.
- MYIRO tools appelle `FDX_StopMeasurement` juste avant, puis laisse **le bouton de l'instrument** déclencher la mesure ; il n'appelle `FDX_StartMeasurement` que pour la lumière ambiante.

## Pour le pont

- **Types 0 (ponctuelle) et 1 (bande) seulement**, option 0 ; les types 2 et 3 restent hors de la v1.
- Le type 1 n'est autorisé qu'au palier Bande.
- Après l'appel : attendre l'événement 1, puis l'appui sur le bouton (événements 2 et 3, voir `FDX_GetMeasureData`).

## Classement

Export `FDX_Set*` à contrat désormais établi : **armement de la mesure, sans écriture persistante**. Il entre dans la liste blanche au palier Mesure ponctuelle.

## Vérifié sur l'instrument

Le 7 octobre 2026 (MYIRO-1 en USB, DLL 1.0.1.0 x64) : l'armement rend 0, puis l'événement **1** arrive ; un appui sur le bouton donne les événements **2** (répétés à chaque donnée brute) puis **3**. Juste après, l'instrument **se réarme seul** (nouvel événement 1).

**Piège observé** : dans l'état « mesure réussie » qui suit l'événement 3, `FDX_StopMeasurement` **et** un nouvel armement sont refusés (-9986). Il faut attendre le réarmement automatique (événement 1), puis désarmer (accepté, suivi de l'événement 0, retour au repos), avant de réarmer. Une session fermée sans ce désarmement laisse l'instrument bloqué en « mesure en cours » (voyant blanc fixe), sourd aux connexions suivantes (-9987) jusqu'à son redémarrage.

## Reste à vérifier

- Le refus -9983 si l'on arme sans étalonnage.

## Preuves (locales)

- DLL x86 1.0.1 : `fdx-x86/exports/FDX_SetMeasureCondition.asm.txt` (contrôle des bornes `0x10035ec9..0x10035eec`, appel interne `0x1001cb90`) ; routine interne : état 1 à `0x1001cbc2`, « is not calibrated. » et -9983 à `0x1001cc99..0x1001cca7`, type ≤ 3 à `0x1001ccd6`, attente de l'état 2 à `0x1001cd18`.
- `retroanalyse/logiciels/myiro-tools.md` § 6 et § 13 ; `retroanalyse/logiciels/my-ct1.md` § 7.2 (commande 0x02, non persistante).
