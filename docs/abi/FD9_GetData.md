# FD9_GetData

**En clair** : lit les données d'une plage mesurée, désignée par son numéro. Selon le réglage posé par [FD9_SetOption](FD9_SetOption.md) (clé 3), la DLL rend les **données brutes** (152 nombres) ou une valeur calculée depuis elles (spectre, L\*a\*b\*…). Elle rend aussi la position de la plage (zone, rangée, colonne), qui permet de vérifier qu'on lit bien la plage voulue.

## Signature

```c
typedef struct { uint32_t zone_ou_index; int32_t rangee; int32_t colonne; } tFD9_PatchPosition;  /* 12 octets */

int32_t __stdcall FD9_GetData(tFD9_PatchPosition *position_rendue,
                              float *donnees,
                              const uint32_t *longueur,              /* nombre de float */
                              const tFD9_PatchPosition *demandee);   /* x86 : ret 0x10 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b1b90`) : `position_rendue` et `donnees` non nuls, `*longueur` supérieur à 0, premier champ de `demandee` au plus **14 999** (`0x3a97`, `0x100b1bb6`), rangée et colonne au moins −1. Sinon : **1001**. `longueur` et `demandee` ne sont pas testés contre le pointeur nul : ils sont lus directement.
- **Forme `{index, −1, −1}`** (rangée et colonne à −1) : la plage n° `index` est lue directement (`0x1008c74b..0x1008c76f`, puis `0x1008c846`). Une autre forme est d'abord convertie en numéro (`0x1008c824`).
- **Données brutes** (clé 3 = `0x0c`) : `*longueur` doit valoir au moins **152**, sinon code interne 8012, public **1005** (`0x1008c87c..0x1008c884`). La DLL écrit alors **152 flottants, en 4 blocs de 38** : une boucle de 38 tours remplit les positions k, 38 + k, 76 + k et 114 + k depuis quatre tampons distincts (`0x1008c8c0..0x1008c8ef`).
- **Autre type** : la DLL calcule la valeur depuis les 152 données brutes, avec le **même calcul** que [FD9_CalculateData](FD9_CalculateData.md) (`0x1008c910` → `0x1008c980`) et les réglages courants ; la longueur minimale dépend du type (table de cette fiche-là).
- **Position rendue** : pour la forme `{index, −1, −1}`, la DLL réécrit `position_rendue` avec la zone, la rangée et la colonne qu'elle déduit du numéro (`0x1008c940..0x1008c95e`). Chaque champ est un nombre de 16 bits étendu à 32.
- **Succès : retour direct de 0, sans passer par la fonction de regroupement** (`0x100b1bd5..0x100b1bdd`) : après un `FD9_GetData` réussi, `FD9_GetLastError` rend encore le code de l'appel précédent.
- Ni l'export ni le début de la fonction de travail (`0x1008c6e0`) ne testent la connexion ou l'état du SDK. La lecture de la plage elle-même (`0x100933b0`) n'a pas été suivie.

## Contenu des structures

| Champ | Position | Sens |
|---|---|---|
| `zone_ou_index` | +0 | numéro de plage (forme `{index, −1, −1}`) ou numéro de zone |
| `rangee` | +4 | −1 pour la forme par numéro ; rangée sinon |
| `colonne` | +8 | −1 pour la forme par numéro ; colonne sinon |

Même disposition en 32 et 64 bits (trois entiers de 32 bits, aucun pointeur).

## Propriété de la mémoire

- Les deux positions et le tableau de flottants appartiennent à l'appelant ; la DLL écrit pendant l'appel et ne garde aucune adresse (confirmé pour l'export et la fonction de travail).
- Pour les données brutes, elle écrit exactement 152 flottants ; un tableau plus court déborderait si `*longueur` mentait. Le pont passe un tableau de 152 et la longueur 152.

## Ce qui est supposé

- Zone, rangée et colonne rendues comptent depuis 0, et les plages suivent l'ordre de la définition (rangées de haut en bas, colonnes de gauche à droite) : la plage n° i d'une zone unique de 42 colonnes est en rangée `i / 42`, colonne `i % 42`.
- Les quatre blocs de 38 sont quatre lectures de la même plage (le nom interne évoque un « facteur de luminance avant calcul »).
- Les données d'une feuille restent lisibles jusqu'à [FD9_ExitMeasurement](FD9_ExitMeasurement.md), qui efface l'état de la mesure.

## Pour le pont

- Poser `FD9_SetOption(3, 0x0c)` juste avant, puis lire chaque plage avec `{i, −1, −1}`, un tableau de 152 flottants et la longueur 152.
- Comparer la position rendue à la position attendue ; un écart refuse toute la mesure.
- Calculer ensuite spectres et L\*a\*b\* hors ligne avec [FD9_CalculateData](FD9_CalculateData.md), depuis les 152 valeurs gardées.
- Ne pas lire `FD9_GetLastError` après un `FD9_GetData` réussi : il ne dit rien de cet appel.

## À vérifier sur l'instrument

- Les données sont-elles encore lisibles à l'événement 9 (fin de mesure), avant `FD9_ExitMeasurement` ? (essai E4)
- Les positions rendues pour la RGB-2520 : zone 0, rangée `i / 42`, colonne `i % 42` ?

## Preuves (locales)

- `fd9-x86/exports/FD9_GetData.asm.txt` (contrôles, retour direct, `ret 0x10`) ; `fd9-x64/exports/FD9_GetData.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008c6e0`.
- Signature : symbole Mac `CJungfrauApp::GetData(tFD9_PatchPosition*, float*, const unsigned&, const tFD9_PatchPosition&)`, `retroanalyse/logiciels/fd-s2w.md` § 3 et 4.2 ; appels de FD-S2w et de MYIROtools : `fd-s2w.md` § 5, `fd9-feuille.md` § 3.
