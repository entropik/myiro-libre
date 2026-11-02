# FD9_GetCaptureResult

**En clair** : après la prise d'image (événement 5), rend le **QR code** lu sur la feuille et **l'image** de la feuille. On l'appelle deux fois : une première fois sans image, pour connaître la taille à prévoir ; une seconde avec un tampon de cette taille. Le QR dit quelle mire a été glissée ; l'image n'est pas nécessaire à la mesure.

## Signature

```c
typedef struct { uint32_t largeur, hauteur, bits_par_pixel; uint8_t *donnees; } tFD9_Image;  /* 16 octets en x86, 24 en x64 */

int32_t __stdcall FD9_GetCaptureResult(uint32_t *resultat_qr,        /* FD9_eBarcodeResult */
                                       tFD9_BarcodeData *qr,         /* 1 172 octets */
                                       uint32_t *taille_image,
                                       tFD9_Image *image);           /* NULL : taille seule ; x86 : ret 0x10 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b1e90`) : `resultat_qr`, `qr` et `taille_image` non nuls, sinon **1001**. `image` peut être nul.
- Fonction de travail `0x1008a610` :
  - hors connexion : code interne 8015, public **1102** ;
  - **état autre que 5** (`WAIT_CAPTURE_COMPLETION`) : code interne 8016, public **1104** (`0x1008a66d..0x1008a676`).
- **Résultat du QR**, d'après les 4 premiers octets du QR gardé par la DLL (`deviceName`, `0x1008a6de..0x1008a7ab`) :

| `deviceName` | `resultat_qr` | QR recopié dans `qr` |
|---|---|---|
| `FAIL` | 0 (`FOUND`) | oui |
| `FD-9`, contrôle réussi | 0 (`FOUND`) | oui |
| `FD-9`, contrôle échoué | 2 (`DATA_ERROR`) | oui |
| autre (rien de lu) | 1 (`NOTFOUND`) | **non : `qr` n'est pas touché** |

  Le contrôle dépend de la version écrite dans le QR (`0x1008a782..0x1008a7a2`) : version qui commence par `011`, contrôle des trois textes seulement (`0x1008d7c0` : `deviceName`, version, texte libre de 512 octets) ; autre version, contrôle de l'ancien format (`0x1008d6b0`, bornes).
- Recopie du QR : **1 172 octets** (`0x125` mots, `0x1008a7b1..0x1008a7be`).
- **`image` nul** : la DLL calcule la taille de l'image et l'écrit dans `*taille_image` (`0x1008a7e8..0x1008a85c`) ; rien d'autre n'est écrit dans l'image.
- **`image` donné** : la DLL remplit `donnees` (lu à `+0x0c` en x86), `largeur` et `hauteur` (`0x1008a7c7..0x1008a7d8`), puis **force `bits_par_pixel` à 8** (`0x1008a7df`). Le code rendu est celui de cette copie.
- Le QR gardé par la DLL est effacé par [FD9_ExitMeasurement](FD9_ExitMeasurement.md) (`0x1008c543..0x1008c560`).

## Contenu de `tFD9_BarcodeData` (1 172 octets)

Rempli par la DLL au moment de la lecture du QR (`CJungfrauCore::ReadBarcodeData`, `0x10095990`). Le texte du QR est découpé sur `:`.

| Position | Taille | Format `011` (RGB-2520) | Ancien format |
|---|---|---|---|
| +0x000 | 4 | `deviceName` : 1ᵉʳ morceau, `FD-9` | idem |
| +0x004 | 4 | `version` : 2ᵉ morceau, par exemple `011M` | idem |
| +0x008 | 0x28c | non lu pour ce format (supposé : reste à zéro) | nom de mire (16), pages, page, plages, zones, 50 × {rangées, colonnes, plages}, cinq champs hexadécimaux |
| +0x294 | 512 | **texte libre** : 3ᵉ morceau, 80 caractères au plus (`0x10095b3c..0x10095b66`) | texte |

Pour la RGB-2520, le QR imprimé par MYIROtools est `FD-9:011M:RGB-2520 - 1P MY9*1+` : `deviceName = FD-9`, `version = 011M`, texte libre `RGB-2520 - 1P MY9*1+`. **Le SDK ne lit pas le nom de mire dans ce texte** : c'est à l'application de le relire.

## Propriété de la mémoire

- Toutes les structures appartiennent à l'appelant ; la DLL y recopie ses données pendant l'appel et ne garde aucune adresse (confirmé pour la partie lue).
- L'image : l'appelant alloue `donnees` à la taille rendue par le premier appel ; la DLL la remplit et ne la libère pas (supposé : elle ne vérifie pas la taille réelle du tampon, elle se fie à `*taille_image`).
- Sur `NOTFOUND`, `qr` garde son contenu d'avant l'appel : le pont le met à zéro avant.

## Ce qui est supposé

- `FAIL` désigne un QR spécial (« feuille d'échec »), sans usage pour le pont.
- `*1+` : page 1 de la mire (MYIROtools) ; le sens de `M` dans `011M` n'est pas établi.
- Les codes 3 et 4 (`ERROR_B`, `ERROR_C`) de l'énumération ne sont pas rendus par cette fonction (aucune écriture vue).

## Pour le pont

- 1ᵉʳ appel : `image` nul, `qr` mis à zéro avant ; 2ᵉ appel : tampon de `*taille_image` octets. L'image n'est gardée que le temps de la requête (ni écrite, ni renvoyée).
- **Juger le QR à trois conditions** : `resultat_qr = 0`, `deviceName = "FD-9"`, texte libre lu jusqu'au premier zéro et au plus 512 octets. `FAIL`, 1 ou 2 : « QR non lu », la reconnaissance tranche.
- Un texte libre qui ne commence pas par `RGB-2520 - 1P MY9` refuse la feuille (`AutreMire`).

## À vérifier sur l'instrument

- Le texte exact rendu pour une RGB-2520 imprimée, et le sens de `011M` et `*1+` (essai E3).
- La taille de l'image A3 rendue, et son nombre de bits par pixel avant qu'il soit forcé à 8.

## Preuves (locales)

- `fd9-x86/exports/FD9_GetCaptureResult.asm.txt` (contrôles, `ret 0x10`) ; `fd9-x64/exports/FD9_GetCaptureResult.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008a610` ; contrôle du format `011` : `--address 0x1008d7c0` ; lecture du QR : `--address 0x10095990`.
- Textes constants `FAIL` (`0x1011854c`) et `FD-9` (`0x1011853c`), lus dans le fichier.
- `retroanalyse/logiciels/fd9-feuille.md` § 4 (QR) ; `fd-s2w.md` § 4.2 et 4.3.
