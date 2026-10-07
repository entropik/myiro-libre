# FD9_GetSystemInfo

**En clair** : demande l'identité de l'instrument connecté (n° de série, micrologiciel, adresse MAC, dates) et la version du SDK. C'est la seconde moitié du palier Connexion. **Piège** : sans instrument connecté, la fonction dit « succès » sans rien écrire.

## Signature

```c
int32_t __stdcall FD9_GetSystemInfo(tFD9_SystemInfo *infos);   /* x86 : ret 4 */
```

## Ce qui est confirmé

- Un argument : la structure à remplir. Pointeur nul : code **1001**.
- **Hors connexion, la fonction renvoie 0 sans toucher à la structure** (`fd9-x86` `0x100b102e..0x100b1036`, même chose en x64 `0x1800d051e..0x1800d0526`). Seul un tampon mis à zéro avant l'appel permet de voir qu'il n'a pas été rempli.
- Connecté, la DLL **efface 100 octets** puis les remplit (`memset` de `0x64`, `fd9-x86` `0x1008f3c1`, `fd9-x64` `0x1800a9309`).
- La version du SDK est **écrite en dur** : `{1, 0x20, 3}` en 1.3.2.3 x86, `{1, 0x1f, 5}` en 1.3.1.5 x64. Le nom du produit aussi : le texte constant `KONICA MINOLTA FD-9`.
- Le reste vient de trois questions à l'instrument. **Seul le résultat de la dernière (les dates) est renvoyé** : un échec des deux premières (micrologiciel ; code produit, n° de série et MAC) passe inaperçu et laisse leurs champs à zéro (`0x1008f410`, `0x1008f4a3`).

## Contenu (100 octets, mêmes positions en 32 et 64 bits)

| Position | Taille | Champ | Niveau |
|---|---|---|---|
| `+0x00` | 3 × u32 | version du SDK, brute | confirmé |
| `+0x0c` | 3 × u32 | version du micrologiciel, décodée par le SDK depuis des chiffres codés en BCD | confirmé ; affichage `%u.%02u.%04u` du journal du SDK |
| `+0x18` | 4 octets | code produit, sans zéro final | position confirmée, nom tiré du journal Mac |
| `+0x1c` | 8 octets | n° de série : 4 octets de l'instrument écrits en hexadécimal (`%02x` × 4), sans zéro final | confirmé |
| `+0x24` | 6 octets | adresse MAC | confirmé |
| `+0x2a` | 2 octets | remplissage (à zéro) | confirmé |
| `+0x2c` | 3 × u32 | date année, mois, jour, décodée du BCD | position confirmée ; « date d'étalonnage en usine » supposé, d'après l'ordre du journal Mac |
| `+0x38` | 3 × u32 | date année, mois, jour | position confirmée ; « date de première mesure » supposé, même raison |
| `+0x44` | 32 octets | nom du produit, terminé par zéro (31 caractères copiés au plus) | confirmé |

`InfosSysteme` dans `crates/fd9-sys` reprend cette forme ; ses tests vérifient taille et positions en 32 et 64 bits. L'étude Mac donnait 104 octets ; les deux DLL Windows n'en écrivent que 100. Le pont passe un tampon de **256 octets** (`TamponInfosSysteme`) pour garder une marge.

## Ce qui est supposé

- Le n° de série est codé en BCD dans l'instrument : l'écriture hexadécimale donne alors 8 chiffres décimaux.
- `{1, 0x20, 3}` se lit 1.3.2.3 (majeur, dix fois le mineur plus la révision, build), règle tirée des deux DLL dont la ressource de version est connue.

## Pour le pont

- Mettre le tampon à zéro avant chaque appel ; vérifier ensuite qu'il est rempli (`est_rempli` : version du SDK non nulle) et que le n° de série n'est pas vide.
- Archiver la version du SDK brute et l'empreinte de la DLL dans la provenance, avec le n° de série, le micrologiciel et le code produit ; les dates avec leur nom « supposé ».
- Les vraies valeurs (n° de série, MAC) ne vont jamais dans un fichier versionné.

## À vérifier sur l'instrument

- Les valeurs réelles des deux dates et du code produit, comparées à celles qu'affiche FD-S2w.

## Preuves (locales)

- `fd9-x86/exports/FD9_GetSystemInfo.asm.txt`, `fd9-x64/exports/FD9_GetSystemInfo.asm.txt`.
- Remplissage : `desassemble.py fd9-x86 --address 0x1008f380` (version `0x1008f3d2..0x1008f3df`, micrologiciel `0x1008f415..0x1008f477`, code produit `0x1008f4b0`, n° de série `0x1008f4cb..0x1008f504`, MAC `0x1008f507..0x1008f534`, dates `0x1008f57a..0x1008f634`, nom `0x1008f58e`, `0x1008f637`) ; `fd9-x64 --address 0x1800a92b0`.
- Noms des champs : journal du SDK Mac, `retroanalyse/logiciels/fd-s2w.md` § 4.3.
