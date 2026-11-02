# FD9_CalculateData

**En clair** : refait un calcul de couleur à partir de données déjà lues : 152 données brutes d'une plage, ou un spectre de 36 valeurs. Le résultat dépend des réglages posés par [FD9_SetOption](FD9_SetOption.md) (condition M0, M1 ou M2, illuminant, observateur, type rendu). C'est un calcul dans la DLL : il ne demande rien à l'instrument. Le pont s'en sert pour tirer M0, M1 et M2 d'une seule lecture brute.

## Signature

```c
int32_t __stdcall FD9_CalculateData(float *sortie,
                                    const uint32_t *longueur_sortie,
                                    const float *entree,
                                    const uint32_t *longueur_entree);  /* 36 ou 152 ; x86 : ret 0x10 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b1c60`) : `sortie` et `entree` non nuls, `*longueur_sortie` non nul, `*longueur_entree` égal à **36** ou **152** ; sinon **1001**. Les deux pointeurs de longueur sont lus sans test du pointeur nul.
- La fonction de travail `0x1008c980` est **la même que celle de [FD9_GetData](FD9_GetData.md)** quand il calcule : le résultat est identique à celui que rendrait la lecture directe.
- Elle lit les quatre réglages de `FD9_SetOption` (`0x1022234c`, `0x10222350`, `0x10222354`, `0x10222358`).
- **Entrée de 152** : conversion en 36 valeurs par `_CalcVirRadianceFactor` de **`DIColor.dll`** (appel indirect `0x1008ce1e`), avec la condition de la clé 0. Un échec donne le code interne 2056 (`0x808`), public 1999. Les valeurs sous un petit seuil sont relevées à 0,0001 (`0x1008ce33..0x1008ce56`).
- **Entrée de 36** : recopiée telle quelle ; les types 8, 9 et 10 sont alors refusés (code interne 8021, public 1001, `0x1008cd90..0x1008cdbf`).
- **Longueur de sortie minimale par type** (table construite en `0x1008cab4..0x1008cd87`, contrôle en `0x1008ceec`) ; trop court : code interne 8022, public **1005** :

| Type (clé 3) | Sortie minimale |
|---|---|
| 0 (L\*a\*b\*), 1, 2, 13, 14 | 3 |
| 3 à 7 (états de densité) | 4 |
| 8 | 2 |
| 9 | 1 |
| 10 | 3 |
| **11 (`0x0b`, spectre)** | **36** |
| 12 (`0x0c`, données brutes) | 152 |

- Aucun test de la connexion ni de l'état du SDK dans l'export et la partie lue de la fonction de travail ; la fin du calcul passe par une table de saut (`0x1008d1a0`) non suivie.

## Propriété de la mémoire

- Les deux tableaux et les deux longueurs appartiennent à l'appelant ; l'entrée est recopiée dans des tampons de la pile de la DLL avant le calcul (`0x1008ce1b`, `0x1008cdb2`), rien n'est gardé après le retour (confirmé pour la partie lue).
- Supposé : la DLL n'écrit pas plus que la longueur minimale du type ; le pont passe de toute façon des tableaux de la longueur exacte.

## Ce qui est supposé

- Le spectre de 36 valeurs couvre 380 à 730 nm au pas de 10 nm (36 confirmé, bornes supposées).
- Le calcul se fait sans aucun échange avec l'instrument : rien de tel dans la partie lue, et le calcul suit la même route que hors connexion dans FD-S2w (`fd-s2w.md` § 5, point 7).
- Les sens des types 1, 2, 8, 9, 10, 13 et 14.
- `DIColor.dll` doit se trouver à côté de `FD9SDK.dll` : elle est importée par la DLL et posée dans le même dossier par FD-S2w.

## Pour le pont

- Pour chaque condition M0 (0), M1 (2), M2 (15) : `FD9_SetOption(0, c)`, puis `FD9_SetOption(3, 0x0b)` et `FD9_CalculateData(spectre, 36, brutes, 152)` ; puis `FD9_SetOption(3, 0)`, D50 (`FD9_SetOption(1, 2)`), 2° (`FD9_SetOption(2, 0)`) et `FD9_CalculateData(lab, 3, brutes, 152)`.
- Garder les 152 données brutes dans la mesure : le calcul peut être refait plus tard.
- Un retour autre que 0 refuse la plage en le disant ; aucune valeur n'est complétée.

## À vérifier sur l'instrument

- Les spectres et L\*a\*b\* calculés pour M0, M1, M2, comparés à l'export de MYIROtools pour la même feuille (essai E4).

## Preuves (locales)

- `fd9-x86/exports/FD9_CalculateData.asm.txt` (contrôles, `ret 0x10`) ; `fd9-x64/exports/FD9_CalculateData.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008c980 --limit 400` ; import `DIColor.dll!_CalcVirRadianceFactor` dans `fd9-x86/metadata.json`.
- `retroanalyse/logiciels/fd-s2w.md` § 4.5 (entrées de 36 ou 152, types de la clé 3).
