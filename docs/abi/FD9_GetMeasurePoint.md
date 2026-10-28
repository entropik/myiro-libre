# FD9_GetMeasurePoint

**En clair** : après la reconnaissance (événement 7), rend les **points de mesure** que l'instrument va viser, un par plage. Le pont n'en a pas besoin pour mesurer ; il l'appelle parce que MYIROtools et FD-S2w le font juste avant de lancer la mesure, et ne fait que garder le nombre de points.

## Signature

```c
typedef struct { uint8_t octets[20]; } tFD9_PatchMeasurePoint;   /* contenu non décodé */

int32_t __stdcall FD9_GetMeasurePoint(tFD9_PatchMeasurePoint *points,
                                      const uint32_t *nombre);    /* x86 : ret 8 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b2710`) : `points` non nul et `*nombre` différent de 0, sinon **1001**. `nombre` est lu sans test du pointeur nul.
- Fonction de travail `0x1008d660` :
  - hors connexion : code interne 8082, public **1102** ;
  - **état autre que 7** : code interne 8062, public **1104** (`0x1008d675..0x1008d67e`) ;
  - sinon, recopie des points par une fonction de la DLL (`0x1009c920`), avec la valeur de `*nombre`.
- Taille d'un point : **20 octets** (MYIROtools alloue `20 × nombre`, FD-S2w aussi).

## Propriété de la mémoire

- Le tableau appartient à l'appelant, qui l'alloue pour `nombre` points ; la DLL le remplit pendant l'appel (confirmé pour l'appel ; supposé : rien n'est gardé, la fonction de copie n'a pas été suivie).

## Ce qui est supposé

- La DLL n'écrit pas plus de `nombre` points.
- L'appel n'est pas nécessaire à la mesure ; il est gardé pour suivre la séquence connue, sans lire le contenu.

## Pour le pont

- Allouer `plages reconnues × 20` octets, à zéro, et passer le nombre de plages reconnu par [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md).
- Ne jamais interpréter le contenu ; un retour autre que 0 arrête la feuille en le disant.

## À vérifier sur l'instrument

- Que l'appel rend 0 pour la RGB-2520 (essai E3).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetMeasurePoint.asm.txt` (`ret 8`) ; `fd9-x64/exports/FD9_GetMeasurePoint.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008d660`.
- Taille : `retroanalyse/logiciels/fd-s2w.md` § 4.3 ; appel de MYIROtools : `fd9-feuille.md` § 3.
