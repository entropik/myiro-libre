# FD9_GetData

**En clair** : lit les données d'une plage mesurée, désignée par son numéro **dans l'ordre où l'instrument l'a mesurée**. Selon le réglage posé par [FD9_SetOption](FD9_SetOption.md) (clé 3), la DLL rend les **données brutes** (152 nombres) ou une valeur calculée depuis elles (spectre, L\*a\*b\*…). Elle rend aussi la position de la plage sur la mire (zone, rangée, colonne), gardée avec ses données : c'est elle qui dit à quelle plage de la mire les données appartiennent. Le FD-9 mesure en **serpentin** (essai E4) : le numéro de lecture n'est pas le numéro de la plage dans la mire.

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
- **D'où vient la position rendue** (étude du 30 octobre 2026) : `0x100933b0` lit l'**enregistrement n° index** de la mémoire des mesures (`CBuffer::ReadDataBufferIndex`, `0x100853e0`) : 0x270 octets par plage, qui portent ensemble la position (zone, rangée, colonne, trois nombres de 16 bits en tête) et les 152 données (à +0x10). Les données et la position viennent donc **du même enregistrement** (confirmé). La position est d'abord écrite dans `position_rendue` (`0x1008c855..0x1008c869`), corrigée si la reconnaissance COMBINE a trouvé la feuille tournée de 90°, 180° ou 270° (`0x100933b0`), puis traduite par une table de correspondance si elle y figure (`0x10085e20`, appelée en `0x1008c940` ; réécriture `0x1008c94a..0x1008c95e`). Chaque champ est un nombre de 16 bits étendu à 32.
- **Ordre des enregistrements** : celui de la mesure. **Confirmé à l'essai E4** (30 octobre 2026, RGB-2520, 60 × 42) : les plages lues 1 à 42 rendent la rangée 0, colonnes 0 à 41 ; la plage lue 43 rend (zone 0, rangée 1, colonne 41) : la rangée 1 est mesurée de droite à gauche (serpentin).
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

- Zone, rangée et colonne rendues comptent depuis 0, dans le repère de la définition (confirmé pour la rangée 0 et le début de la rangée 1 à l'essai E4).
- ~~Les plages suivent l'ordre de la définition~~ : **faux** (essai E4, serpentin). Le serpentin n'est pas supposé par le pont : il range par la position rendue.
- Les quatre blocs de 38 sont quatre lectures de la même plage (le nom interne évoque un « facteur de luminance avant calcul »).
- Les données d'une feuille restent lisibles jusqu'à [FD9_ExitMeasurement](FD9_ExitMeasurement.md), qui efface l'état de la mesure.

## Pour le pont

- Poser `FD9_SetOption(3, 0x0c)` juste avant, puis lire chaque plage avec `{i, −1, −1}`, un tableau de 152 flottants et la longueur 152.
- Lire toutes les plages par leur numéro, puis **les ranger par la position rendue** dans l'ordre de la définition (SAMPLE_ID 1 = zone 0, rangée 0, colonne 0, puis de gauche à droite et de haut en bas, zone par zone). Chaque position de la mire doit être rendue **une fois et une seule** : un doublon, une position hors de la mire ou une position jamais rendue refuse toute la mesure (`positions_plages`, avec `doublon`, `hors_mire` ou `manquante`).
- MYIROtools garde lui aussi la position rendue avec les données de chaque plage et la remet à son appelant (`mit-x64` `0x1403ebaf0`, copie vers `0x14068ee94..0x14068ee9c`, appelant `0x1403f2a30`) ; l'usage qu'en fait l'appelant n'a pas été suivi.
- Calculer ensuite spectres et L\*a\*b\* hors ligne avec [FD9_CalculateData](FD9_CalculateData.md), depuis les 152 valeurs gardées.
- Ne pas lire `FD9_GetLastError` après un `FD9_GetData` réussi : il ne dit rien de cet appel.

## À vérifier sur l'instrument

- Les données sont-elles encore lisibles à l'événement 9 (fin de mesure), avant `FD9_ExitMeasurement` ? (essai E4)
- ~~Les positions rendues pour la RGB-2520 : zone 0, rangée `i / 42`, colonne `i % 42` ?~~ Non : serpentin (essai E4). À vérifier à l'essai E4 bis : chaque position de la RGB-2520 rendue une fois (1 260 plages lues hors de l'ordre de la mire attendues).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetData.asm.txt` (contrôles, retour direct, `ret 0x10`) ; `fd9-x64/exports/FD9_GetData.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008c6e0` ; enregistrement : `--address 0x100933b0`, `--address 0x100853e0` ; table de correspondance : `--address 0x10085e20`.
- Signature : symbole Mac `CJungfrauApp::GetData(tFD9_PatchPosition*, float*, const unsigned&, const tFD9_PatchPosition&)`, `retroanalyse/logiciels/fd-s2w.md` § 3 et 4.2 ; appels de FD-S2w et de MYIROtools : `fd-s2w.md` § 5, `fd9-feuille.md` § 3.
