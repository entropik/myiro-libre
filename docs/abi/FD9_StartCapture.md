# FD9_StartCapture

**En clair** : une fois la feuille engagée dans le FD-9 (événement 3, `WAIT_FEED_COMPLETION`), demande à l'instrument de **prendre l'image** de la feuille. La fin de la prise d'image est annoncée par l'événement 5 (`WAIT_CAPTURE_COMPLETION`) ; on lit alors l'image et le QR avec [FD9_GetCaptureResult](FD9_GetCaptureResult.md).

## Signature

```c
int32_t __stdcall FD9_StartCapture(void);  /* x86 : ret nu, aucun argument */
```

## Ce qui est confirmé

- Aucun argument (`fd9-x86` `0x100b1d80`, `ret` nu).
- Avant tout, l'export **remet à zéro la cause d'erreur de préparation** lue par [FD9_GetPremeasurementError](FD9_GetPremeasurementError.md) (`0x100b1de8..0x100b1e0f`).
- Fonction de travail `0x1008a560` :
  - hors connexion : code interne 8013, public **1102** ;
  - **état du SDK autre que 3** : code interne 8014, public **1104** (`0x1008a57a..0x1008a583`) ;
  - sinon, appel d'une fonction d'échange avec une attente de **30 secondes** (`0x1008a5dc..0x1008a5f0`, `0x7530` ms), puis lancement d'un fil de travail de la DLL (`0x1008a5fd`), et **retour 0 dans tous les cas** : le résultat de l'échange n'est pas testé (`0x1008a5f5..0x1008a600`).
- Le retour 0 dit seulement que l'état était 3 : la prise d'image est **lancée**, pas finie, et peut-être pas acceptée. La fin vient par le rappel.

## Propriété de la mémoire

- Aucun argument : rien à libérer, rien de l'appelant n'est gardé (confirmé).

## Ce qui est supposé

- La fonction d'échange envoie à l'instrument la commande de prise d'image. Un instrument muet donnerait donc 0, puis aucun événement 5.
- Les événements 4 (`RUN_CAPTURE`) puis 5 suivent.

## Pour le pont

- Ne l'appeler **que dans le fil de la session**, après avoir reçu l'événement 3 ; jamais depuis le rappel. Exception : une feuille glissée avant la requête, que le pont reprend sur l'état 3 lu par [FD9_GetStatus](FD9_GetStatus.md) (supposé, essai E3) ; le contrôle d'état ci-dessus rend alors 1104 si l'état n'était pas 3, et le pont le rend tel quel.
- Attendre ensuite l'événement 5 avec un délai ; sans lui, rendre `Delai`, puis arrêter avec [FD9_StopMeasurement](FD9_StopMeasurement.md).

## À vérifier sur l'instrument

- La durée entre l'événement 3 et l'événement 5 pour une feuille A3 (essai E3).
- Les événements reçus entre les deux.

## Preuves (locales)

- `fd9-x86/exports/FD9_StartCapture.asm.txt` ; `fd9-x64/exports/FD9_StartCapture.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008a560`.
- Codes publics : table de regroupement `0x10108ca0` (8013 → 1102, 8014 → 1104).
- Séquences : `retroanalyse/logiciels/fd-s2w.md` § 5 ; `fd9-feuille.md` § 3 (MYIROtools lance l'appel depuis un autre fil que son gestionnaire d'événements).
