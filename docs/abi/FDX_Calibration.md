# FDX_Calibration

**En clair** : lance l'étalonnage de l'instrument. Pour mesurer du papier, c'est l'**étalonnage sur le blanc** : l'instrument, posé sur son capuchon, mesure la tuile blanche du capuchon et s'en sert de référence. C'est le palier Étalonnage, à refaire avant chaque série de mesures.

## Signature

```c
int32_t __stdcall FDX_Calibration(int32_t type);   /* FDX_eCalibType ; x86 : ret 4 */
```

## Ce qui est confirmé

- Un seul argument, le type d'étalonnage (`CCalibration::ExecCalibration`, SDK Mac) :

  | Type | Étalonnage | Usage |
  |---|---|---|
  | **0** | **blanc** (`CalibrationWhite`) | mesure de réflexion : papier, ponctuelle et bande |
  | 1 | irradiance | lumière ambiante ; exige l'adaptateur, sinon erreur -9986 |
  | 2 | radiance (zéro) | mesure d'écran |

- L'étalonnage est **asynchrone** : son résultat arrive par les événements de `FDX_RegisterDeviceEventHandler`, 7 (commencé) puis **8 (réussi)** ou **9 (échoué)**. EIZO attend ces événements par pas de 300 ms, environ 10 s au plus.
- Les commandes envoyées à l'instrument sont celles de l'étalonnage utilisateur (`CCommand_0x10` `SetUserCalib`, `CCommand_0x12`). Aucune écriture des données d'étalonnage d'usine n'a été trouvée sur ce chemin : celles-ci ne passent que par les `FDX_JIG_*`, interdits.
- MYIRO tools, avant d'étalonner, appelle `FDX_StopMeasurement`, puis `FDX_Calibration(0)` pour la réflexion.
- Erreur -9987 : délai de réponse dépassé. Avertissement +1 dans `FDX_GetError` : étalonnage blanc à refaire.

## Ce que disent les manuels

- La tuile blanche est dans le **capuchon MY-A01**, appairé à l'instrument par son n° de série ; ses valeurs sont données à 23 °C. Le capuchon sert aussi à un étalonnage du noir, très probablement enchaîné par le même appel.
- Un logiciel connecté est obligatoire. Avec un logiciel connecté et le voyant **jaune** (non étalonné), le bouton de l'instrument lance aussi l'étalonnage.
- Voyant : **bleu clignotant** pendant l'étalonnage, **vert 1 s** s'il a réussi, **rouge clignotant 1 s** s'il a échoué, puis **bleu fixe** : prêt à mesurer.
- À refaire après la mise sous tension, après un nettoyage, après un certain temps ou un changement de température. **L'arrêt automatique (15 min) efface l'étalonnage** ; il ne se déclenche pas en USB.

## Pour le pont

- **Type 0 seulement.** Les types 1 et 2 (lumière ambiante, écran) restent hors du périmètre de la v1 et sont refusés par le pont.
- Avant l'appel : rappel d'événements enregistré, instrument connecté et au repos, **capuchon en place** (geste de l'opérateur, demandé par l'application).
- Après l'appel : attendre l'événement 8 ou 9, avec un délai maximal ; journaliser les événements et `FDX_GetError`.
- Archiver la date et l'heure de l'étalonnage dans la provenance des mesures qui suivent.

## Ce qui est supposé

- L'appel rend la main avant la fin de l'étalonnage (asynchrone côté appelant) : déduit des attentes d'EIZO et de MYIRO tools, à confirmer.
- L'étalonnage du noir est inclus dans le type 0.

## À vérifier sur l'instrument

- Les événements reçus (7 puis 8), la durée réelle de l'étalonnage et le voyant.
- Le comportement sans capuchon (événement 9 attendu, aucun dommage).

## Preuves (locales)

- `fdx-x86/exports/FDX_Calibration.asm.txt` (`ret 4`, type 0..2) ; `preuves/eizo-x64/calibration.asm.txt` (appel à `0x1800422a4` et attente).
- `retroanalyse/logiciels/myiro-tools.md` § 11 et § 13 : types d'étalonnage, commandes, séquence MYIRO tools.
- `retroanalyse/logiciels/manuels.md` § 2 : procédure, capuchon, voyant.
