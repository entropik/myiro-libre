# FD9_GetMeasurePoint

**En clair** : après la reconnaissance (événement 7), rend les **points de mesure** que l'instrument va viser, un par plage : le centre de chaque plage **trouvée sur l'image de la feuille**, et sa place dans la mire. Ce sont les points rouges que FD-S2w dessine sur l'image de la feuille dans son onglet « Measured Point ». Le pont s'en sert pour vérifier que la feuille a bien été retrouvée, avant de mesurer.

## Signature

```c
typedef struct {
    int32_t  x, y;                    /* centre de la plage, 1/100 mm sur le papier */
    uint32_t zone, rangee, colonne;   /* place dans la mire, comptées depuis 0 */
} tFD9_PatchMeasurePoint;             /* 20 octets */

int32_t __stdcall FD9_GetMeasurePoint(tFD9_PatchMeasurePoint *points,
                                      const uint32_t *nombre);    /* x86 : ret 8 */
```

## Ce qui est confirmé

- **Contrôles de l'export** (`fd9-x86` `0x100b2710`) : `points` non nul et `*nombre` différent de 0, sinon **1001**. `nombre` est lu sans test du pointeur nul.
- Fonction de travail `0x1008d660` :
  - hors connexion : code interne 8082, public **1102** ;
  - **état autre que 7** : code interne 8062, public **1104** (`0x1008d675..0x1008d67e`) ;
  - sinon `CJungfrauCore::GetChartPaperMeasPoint` (`0x1009c920`), avec `*nombre`.
- **Contenu** (`0x1009c920`, écriture `0x1009fd20`) : pour chaque plage de chaque zone, dans l'ordre zone, rangée, colonne, la DLL cherche la plage dans la liste des plages trouvées sur l'image (`FindPatchFromPatchList`, `0x1009fb30`), puis écrit :
  - `x`, `y` : le centre de la plage trouvée, en pixels de l'image à 300 ppp, converti en 1/100 mm (`× 2540 / 300`, arrondi) ;
  - `zone`, `rangee`, `colonne` : sa place dans la définition, corrigée si la feuille a été vue tournée de 90°, 180° ou 270° (`+0x159c`).
- Plus de points demandés que de plages trouvées : refus interne 0x807 (journal Windows « result = %d, pMeasurePoint = %p, length = %d, layout.allPatchCount = %d »).
- **FD-S2w** (`s2w-x86` `0x41e490`, `CFD9SDKController::EventWaitPatchrecogCompletion`) : à l'événement 7, `FD9_GetPatchRecognitionResult2`, puis l'image de la feuille par [FD9_GetChartImage](FD9SDK.md#hors-liste-blanche) (taille, puis image), puis `FD9_GetMeasurePoint` pour toutes les plages ; il garde image et points pour l'onglet « Measured Point » (`0x41f750`, `0x41ddf0`).

## Propriété de la mémoire

- Le tableau appartient à l'appelant, qui l'alloue pour `nombre` points ; la DLL le remplit pendant l'appel et ne garde aucune adresse (confirmé, `0x1009c920`).

## Ce qui est supposé

- Le repère des points (1/100 mm, origine au coin de la feuille vue par l'instrument) est celui des coordonnées de la définition : les points d'une feuille bien placée tombent sur les centres théoriques. À vérifier à l'essai E4 ter (journal « points de mesure : … décalage de la feuille (dx ; dy) »).
- En COORDINATE, les plages « trouvées » sont celles de la définition : les points ne prouvent rien (essai E4 bis).

## Pour le pont

- Après la reconnaissance COMBINE, lire un point par plage reconnue ; **chaque place de la mire une fois** (sinon `positions_plages` : doublon, hors mire, manquante) ; décalage moyen de la feuille ≤ 21 mm, et chaque point à moins de 3 mm de la grille décalée (sinon `mire_non_reconnue`). La mesure vise ces points : jamais les seules coordonnées théoriques.
- Fait aux deux paliers, Prénumérisation et Feuille.

### Pour l'application, plus tard (contrat proposé, pas encore fait)

Montrer, comme FD-S2w, l'image de la feuille et les points visés, avant de mesurer :

```json
{"rep":"prenumerisee","lecture":{...},
 "calage":{"decalage":[880,-110],"ecart_max":40,
           "points":[{"zone":0,"rangee":0,"colonne":0,"x":1649,"y":2600}, ...],
           "image":{"largeur":330,"hauteur":496,"ppp":30,"gris":"<base64, 8 bits>"}}}
```

- `points` : les points de `FD9_GetMeasurePoint`, dans l'ordre de la définition ; `x`, `y` en 1/100 mm.
- `image` : l'image de `FD9_GetChartImage`, réduite par le pont (par exemple au dixième, 30 ppp), en gris 8 bits ; elle ne contient ni adresse ni identifiant. `FD9_GetChartImage` est à ajouter à la liste blanche (lecture seule, admise hors des états 1 à 6) après une fiche.
- Champ facultatif : une réponse sans `calage` reste lisible.

## À vérifier sur l'instrument

- Le décalage et l'écart rendus pour la RGB-2520 imprimée sur A3 (essai E4 ter).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetMeasurePoint.asm.txt` (`ret 8`) ; `fd9-x64/exports/FD9_GetMeasurePoint.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008d660` ; contenu : `--address 0x1009c920`, `--address 0x1009fd20`, `--address 0x1009fb30`.
- FD-S2w : `s2w-x86` `0x41e490` (événement 7), `0x41ed00` (`FD9_GetChartImage`), `0x41f750` (suite) ; notes locales `retroanalyse/logiciels/fd9-feuille.md` § 10.
