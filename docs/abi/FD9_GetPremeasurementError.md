# FD9_GetPremeasurementError

**En clair** : donne la **cause** d'un échec de la préparation de la feuille (image, reconnaissance des plages), quand le rappel annonce l'erreur 201 (`ERROR_PREMEASURE`) : définition qui ne correspond pas, trop de zones ou de plages, feuille trop longue, image illisible… Elle ne parle pas à l'instrument : elle relit ce que la DLL a noté.

## Signature

```c
/* Attention : __cdecl, pas __stdcall. */
int32_t __cdecl FD9_GetPremeasurementError(int32_t *erreur_systeme,   /* NULL admis */
                                           const char **texte);        /* NULL admis */
```

## Ce qui est confirmé

- **Convention `__cdecl`** : l'export (`fd9-x86` `0x100b2960`) finit par un `ret` nu ; l'appelant dépile ses deux arguments. C'est un des deux exports `__cdecl` de la DLL (l'autre, `FD9_WriteBmpFile`, reste exclu).
- **Pas de code d'erreur regroupé** : l'export rend directement la valeur de la fonction de travail `0x1008e610`, sans passer par la fonction de regroupement ; `FD9_GetLastError` n'est pas modifié.
- **Valeur rendue** : la cause gardée par la DLL (`0x1008e6ee`), une valeur de `FD9_ePremeasurementError` (table de noms `0x101084a0`, 12 entrées, lue dans le fichier) :

| Valeur | Nom | En clair |
|---|---|---|
| 0 | `PREMEASURE_OK` | pas d'erreur |
| 1 | `W_DEFDATA_DENSITY_MISMATCH` | avertissement : densité différente de la définition |
| 2 | `E_DEFDATA_SIZE_MISMATCH` | taille différente de la définition |
| 3 | `E_DEFDATA_OTHER_MISMATCH` | autre différence avec la définition |
| 4 | `E_DEFDATA_TOO_MANY_AREAS` | plus de 50 zones |
| 5 | `E_DEFDATA_TOO_MANY_PATCHES` | plus de 15 000 plages, ou rangées et colonnes hors bornes |
| 6 | `E_CHARTPAPER_TOO_LONG` | feuille trop longue |
| 7 | `E_INVALID_INPUT_IMAGE` | image d'entrée invalide |
| 8 | `E_IMGPROC` | échec du traitement d'image |
| 9 | `E_FILE_WRITE_ERROR` | échec d'écriture d'un fichier |
| 10 | `E_OUT_OF_MEMORY` | mémoire insuffisante |
| 11 | `E_UNEXPECTED` | autre |

- **`erreur_systeme`**, si non nul, reçoit un numéro d'erreur système gardé à côté (`0x1008e6cf`). **`texte`**, si non nul, reçoit **l'adresse d'un texte de la DLL** (`0x1008e6db..0x1008e6ec`). Les deux pointeurs nuls sont admis : MYIROtools l'appelle avec `(0, 0)`.
- **Aucun test de connexion** : la fonction relit des champs de l'objet interne.
- La cause est **remise à zéro** par [FD9_StartCapture](FD9_StartCapture.md) (`0x100b1de8..0x100b1e0f`), et posée par [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md) quand ses contrôles échouent (4 et 5, `0x100b20e6`, `0x100b222b`, `0x100b2470`).

## Propriété de la mémoire

- `erreur_systeme` appartient à l'appelant ; la DLL y écrit un entier (confirmé).
- **Le texte appartient à la DLL** : l'appelant reçoit une adresse dans l'objet interne, pas une copie (confirmé). Il ne doit ni le libérer ni le garder : le texte change au prochain `FD9_StartCapture` ou à la prochaine erreur (supposé pour la durée exacte). Le pont le recopie aussitôt, ou ne le demande pas.

## Ce qui est supposé

- Les codes 1 à 3 correspondent à une feuille qui n'est pas celle de la définition (cas d'une autre mire de même taille).
- Le texte est en anglais et sert au journal, pas à l'opérateur.

## Pour le pont

- Déclarer l'export en `extern "C"`, jamais `extern "system"` : une mauvaise convention déséquilibrerait la pile en 32 bits.
- L'appeler à l'événement 201, avec `erreur_systeme` vers un entier du pont et `texte` nul ; ranger la valeur dans l'erreur rendue (`IncidentFeuille { etat: 201, premesure }`), puis appeler [FD9_ExitMeasurement](FD9_ExitMeasurement.md).

## À vérifier sur l'instrument

- La cause rendue pour une page blanche ou une autre mire (essai E3 bis).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetPremeasurementError.asm.txt` (`ret` nu, pas de regroupement) ; `fd9-x64/exports/FD9_GetPremeasurementError.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008e610`.
- Noms : table `0x101084a0` de `fd9-x86` ; `retroanalyse/logiciels/fd-s2w.md` § 4.1 (convention) et § 4.4 (énumération) ; appel de MYIROtools : `fd9-feuille.md` § 3.
