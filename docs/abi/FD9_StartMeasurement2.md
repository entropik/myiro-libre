# FD9_StartMeasurement2

**En clair** : une fois les plages reconnues (événement 7) et la reconnaissance jugée bonne, **lance la mesure** de toutes les plages de la feuille. Le FD-9 mesure alors seul ; il annonce le nombre de plages déjà mesurées (événement 11) puis la fin (événement 9, `WAIT_MEASURE_COMPLETION`).

## Signature

```c
int32_t __stdcall FD9_StartMeasurement2(const uint32_t *mode,   /* FD9_eMeasurementMode */
                                        const uint32_t *avis);  /* x86 : ret 8 */
```

## Ce qui est confirmé

- L'export (`fd9-x86` `0x100b27a0`) lit les deux nombres **sans test du pointeur nul** et les passe par valeur à la fonction de travail `0x1008b320`.
- Fonction de travail :
  - remet à zéro le drapeau d'arrêt de [FD9_StopMeasurement](FD9_StopMeasurement.md) (`0x1008b361`) ;
  - hors connexion : code interne 8017, public **1102** ;
  - **état autre que 7** (`WAIT_PATCHRECOG_COMPLETION`) : code interne 8018, public **1104** (`0x1008b377..0x1008b380`) ;
  - garde le mode entier (`0x1008b429`), puis passe **`mode & 7`** à la préparation de la mesure (`0x1008b42f..0x1008b439`) ;
  - garde `avis` pour le fil de mesure (`0x1008b513..0x1008b51b`), puis lance ce fil (`0x1008b526`) et rend 0.
- Le retour 0 dit que la mesure est **lancée**, pas finie.

## Propriété de la mémoire

- Deux nombres lus pendant l'appel ; aucune adresse de l'appelant n'est gardée (confirmé).

## Ce qui est supposé

- `mode` : méthode de mesure, de 0 à 4 ; le bit 8 ajouté par FD-S2w commande l'alimentation continue (FD-S2w passe la méthode, ou la méthode + 8). Seuls les 3 bits bas servent ici.
- `avis` = 1 : la DLL envoie l'événement 11 (nombre de plages mesurées) pendant la mesure.
- MYIROtools passe `mode = 1` et `avis = 1` pour la RGB-2520 (confirmé dans `fd9-feuille.md` § 3) ; FD-S2w passe `avis = 1` et sa propre méthode.

## Pour le pont

- Appeler avec `mode = 1` et `avis = 1`, comme MYIROtools, **seulement** après avoir comparé la reconnaissance à la définition et appelé [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md).
- Ensuite, attendre l'événement 9 ; les événements 11 font avancer un compte, journalisé ; un événement d'erreur arrête (voir [FD9SDK](FD9SDK.md), « Mesurer une feuille »).
- Ne jamais l'appeler au palier Prénumérisation.

## À vérifier sur l'instrument

- La durée de la mesure d'une RGB-2520 et la cadence des événements 11 (essai E4).
- Le sens de la méthode 1.

## Preuves (locales)

- `fd9-x86/exports/FD9_StartMeasurement2.asm.txt` (`ret 8`) ; `fd9-x64/exports/FD9_StartMeasurement2.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008b320`.
- Valeurs passées : `retroanalyse/logiciels/fd9-feuille.md` § 3 et 4 ; `fd-s2w.md` § 4.4 (`FD9_eMeasurementMode`) et § 5.
