# FDX_GetMeasureData

**En clair** : lit les résultats d'une mesure terminée. On choisit, par une « condition de calcul », ce qu'on veut recevoir : un spectre (36 valeurs) sous l'éclairage M0, M1 ou M2, des valeurs Lab ou XYZ, une densité, ou les données brutes de l'instrument. On peut relire la même mesure plusieurs fois avec des conditions différentes.

## Signature

```c
typedef struct { float *MeasureData; uint32_t MeasureDataLength; } FDX_MeasureData; /* 8 octets x86, 16 x64 */
int32_t __stdcall FDX_GetMeasureData(FDX_MeasureData *resultats,      /* tableau, NULL au 1er appel */
                                     uint32_t *nombre,                /* obligatoire */
                                     FDX_eMeasureDirection *sens,     /* obligatoire, en sortie */
                                     uint32_t capacite,               /* en nombre de résultats */
                                     const FDX_CalcCondition *calcul); /* 0x31C octets, obligatoire */
```

## Ce qui est confirmé

- **Appel en deux temps** (MYIRO tools, EIZO) : `(NULL, &n, &sens, 0, &calcul)` donne le nombre de résultats (une plage en ponctuelle, toutes les plages d'une bande) ; puis `n` descripteurs, chacun pointant vers un tableau de `float` préparé par l'appelant, et second appel.
- Deux capacités distinctes : `capacite` en **nombre de résultats**, `MeasureDataLength` en **nombre de valeurs** par résultat. La DLL contrôle les pointeurs et les capacités.
- `sens` : sens de passage d'une bande (0, 1, 2) ; avec 2, l'ordre des plages est inversé. Les données d'une bande arrivent toujours de gauche à droite selon le manuel.
- **Condition de calcul** `FDX_CalcCondition` (0x31C octets, à mettre à zéro puis remplir) :

  | Position | Nom du fabricant | Valeurs utiles |
  |---|---|---|
  | `+0x00` | `Illuminant` | **condition de mesure** : **0 = M0** (A), **1 = M1** (D50), **2 = M2** (A sans UV) ; 3 = illuminant utilisateur |
  | `+0x04` | `ObsIlluminant` | illuminant colorimétrique pour Lab/XYZ : **2 = D50**, 4 = D65 |
  | `+0x08` | `Observer` | 0 ou 1 ; **0 = 2°** (supposé fort, utilisé par MYIRO tools pour son export 2°) |
  | `+0x0C` | `DensityStatus` | 0 à 3, libellés non établis |
  | `+0x10` | `DataType` | ci-dessous |
  | `+0x14` | illuminant utilisateur | 97 valeurs, 300 à 780 nm par 5 nm ; inutile hors `Illuminant = 3` |

- `DataType` pour la réflexion (types de mesure 0 et 1) :

  | Valeur | Résultat | Valeurs par résultat |
  |---|---|---|
  | 0 | L\*a\*b\* | 3 |
  | 1 | L\*C\*h | 3 |
  | 2 | Hunter Lab | 3 |
  | 3 | Yxy | 3 |
  | 4 | XYZ | 3 |
  | 9 | densité | 4 au moins |
  | **10** | **spectre de réflexion, 380 à 730 nm par 10 nm** | **36** |
  | **11** | **données brutes « PreRad »** de l'instrument | **152** |

- Le spectre est calculé **par la DLL** à partir des données brutes et de l'éclairage demandé : une seule mesure donne M0, M1 et M2.
- Grille : valeur *i* = 380 + 10·*i* nm (table de pondération du SDK, et manuel MYIRO-1).

## Ce qui est supposé

- L'échelle du spectre (0 à 1 ou 0 à 100) : à établir sur la première mesure d'un blanc.
- Le sens exact des valeurs de `sens` et de `DensityStatus`.

## Pour le pont

- Ne lire qu'après l'événement 3 (« mesure terminée »).
- Pour chaque mesure : `DataType` 10 avec `Illuminant` 0, 1 et 2 (**trois spectres M0, M1, M2**), puis `DataType` 11 (**données brutes**, gardées pour le futur pilote libre, ADR 0006). La colorimétrie se calcule ensuite dans notre code, pas dans la DLL.
- Allouer chaque tableau à la longueur exacte attendue (36 ou 152), vérifier `nombre` après le second appel, et archiver `sens`.

## À vérifier sur l'instrument

- Les valeurs d'un blanc papier (échelle) et d'un noir ; l'écart M0/M2 sur un papier azuré.
- Le nombre de résultats d'une mesure ponctuelle (1 attendu).

## Preuves (locales)

- `fdx-x86/exports/FDX_GetMeasureData.asm.txt` (`ret 0x14`, contrôle des arguments `0x1003673e..0x10036779`) ; `fdx-x86/internes/GetMeasureData-linear.asm.txt` (table des `DataType`) ; `fdx-x86/internes/spectral-helpers-window.asm.txt` (36 valeurs, `0x10020600`).
- `retroanalyse/logiciels/myiro-tools.md` § 7, § 8 et § 13 : noms des champs, tables d'éclairage, grille, séquence de lecture de MYIRO tools.
- `retroanalyse/logiciels/manuels.md` : 380 à 730 nm par 10 nm, toutes les conditions en une passe.
