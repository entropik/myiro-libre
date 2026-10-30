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
- **FD-S2w** (`s2w-x86` `0x41e490`, `CFD9SDKController::EventWaitPatchrecogCompletion`) : à l'événement 7, `FD9_GetPatchRecognitionResult2`, puis une copie de l’image prise, gardée depuis `FD9_GetCaptureResult` (`0x41eca0`, taille puis image ; correction du 30 octobre 2026 : ce n’est pas [FD9_GetChartImage](FD9_GetChartImage.md)), puis `FD9_GetMeasurePoint` pour toutes les plages ; il garde image et points pour l'onglet « Measured Point » (`0x41f750`, `0x41ddf0`).

## Propriété de la mémoire

- Le tableau appartient à l'appelant, qui l'alloue pour `nombre` points ; la DLL le remplit pendant l'appel et ne garde aucune adresse (confirmé, `0x1009c920`).

## Ce qui est supposé

- Le repère des points (1/100 mm, origine au coin de la feuille vue par l'instrument) est celui des coordonnées de la définition : les points d'une feuille bien placée tombent sur les centres théoriques. À vérifier à l'essai E4 ter (journal « points de mesure : … décalage de la feuille (dx ; dy) »).
- En COORDINATE, les plages « trouvées » sont celles de la définition : les points ne prouvent rien (essai E4 bis).

## Pour le pont

- Après la reconnaissance COMBINE, lire un point par plage reconnue ; **chaque place de la mire une fois** (sinon `positions_plages` : doublon, hors mire, manquante) ; décalage moyen de la feuille ≤ 21 mm, et chaque point à moins de 3 mm de la grille décalée (sinon `mire_non_reconnue`). La mesure vise ces points : jamais les seules coordonnées théoriques.
- Fait aux deux paliers, Prénumérisation et Feuille.

### Pour l'application : le calage (fait le 30 octobre 2026, ticket #34)

Pour montrer, comme FD-S2w, l'image de la feuille et les points visés, le pont rend un champ `calage` dans ses réponses `prenumerisee` et `mesure` (feuille), ou, quand l'avancement est suivi, une seule fois dans l'étape `calage_pret` (section suivante) :

```json
{"rep":"prenumerisee","lecture":{...},
 "calage":{"decalage":[880,-110],"ecart_max":40,
           "points":[{"zone":0,"rangee":0,"colonne":0,"x":1649,"y":2600}, ...],
           "image":{"largeur":660,"hauteur":984,"ppp":60,"gris":"<base64, 8 bits>"}}}
```

- `decalage`, `ecart_max` : ceux du contrôle ci-dessus, en 1/100 mm.
- `points` : les points de `FD9_GetMeasurePoint`, dans l'ordre de la définition ; `x`, `y` en 1/100 mm.
- `image` : l'image de [FD9_GetChartImage](FD9_GetChartImage.md), lue juste après les points, réduite par le pont à 1 000 pixels de haut au plus (carrés de k × k pixels moyennés, k diviseur de 300 : 60 ppp pour une A3), gris 8 bits, une rangée après l'autre, en base64. Un point tombe sur le pixel `(x × ppp / 2540, y × ppp / 2540)` (supposé, fiche `FD9_GetChartImage`). Elle ne contient ni adresse ni identifiant, mais peut montrer ce que l'opérateur a écrit sur la feuille : jamais dans un fichier versionné. Si la DLL ne la rend pas, `image` est absent et la feuille continue.
- Champs facultatifs : une réponse sans `calage`, ou un calage sans `image`, reste lisible. Le calage n'entre pas dans la mesure conservée (`myiro-libre/mesure/1`).

### Avancement de la feuille (fait le 30 octobre 2026, ticket #34)

Pendant `prenumeriser` et `mesurer_feuille`, le pont écrit, avant sa réponse finale, une ligne par étape :

```json
{"rep":"avancement","avancement":{"etape":"feuille_engagee"}}
{"rep":"avancement","avancement":{"etape":"lecture"}}
{"rep":"avancement","avancement":{"etape":"calage_pret","calage":{...}}}
{"rep":"avancement","avancement":{"etape":"mesure_lancee"}}
{"rep":"avancement","avancement":{"etape":"mesure","mesurees":42,"plages":2520}}
```

- `feuille_engagee` : premier état 2 ou 3 de la feuille (lu par `FD9_GetStatus` ou reçu du rappel), une fois. Les noms de ces états (`RUN_FEED`, `WAIT_FEED_COMPLETION`) sont **confirmés** (table de la DLL) ; qu'ils suivent l'entrée de la feuille dans l'instrument est **supposé**, d'après ces noms : le journal des essais E3 et E4 ne l'a pas encore établi.
- `lecture` : `FD9_StartCapture` accepté (image, QR, reconnaissance).
- `calage_pret` : le calage ci-dessus, dès les contrôles passés, **avant** `FD9_StartMeasurement2` : l'application montre les points pendant la mesure. Annoncé, il est rendu : la réponse finale (`prenumerisee`, `mesure`) porte alors `calage` absent, et l'image ne part qu'une fois. Sans suivi de l'avancement (session appelée directement), la réponse le porte comme avant.
- `mesure_lancee` : `FD9_StartMeasurement2` accepté, sans compte.
- `mesure` : seulement à un événement 11 du rappel, avec son 4ᵉ nombre (`mesurees`, nom tiré du symbole de la bibliothèque Mac) ; un compte répété n'est pas réécrit, et aucun compte n'est écrit sans événement 11. Qu'il compte les plages déjà mesurées est **supposé** (table des états) ; à l'essai E4, seul le journal l'a vu.
- Une feuille refusée s'arrête à la dernière étape atteinte, puis sa réponse d'erreur. Un pont d'avant ce contrat n'écrit que sa réponse finale : l'application la lit comme avant. Une ligne d'avancement que l'application ne sait pas lire (étape ou champ d'un pont plus récent) est notée et écartée : la feuille continue.

## À vérifier sur l'instrument

- Le décalage et l'écart rendus pour la RGB-2520 imprimée sur A3 (essai E4 ter).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetMeasurePoint.asm.txt` (`ret 8`) ; `fd9-x64/exports/FD9_GetMeasurePoint.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008d660` ; contenu : `--address 0x1009c920`, `--address 0x1009fd20`, `--address 0x1009fb30`.
- FD-S2w : `s2w-x86` `0x41e490` (événement 7), `0x41ed00` (`FD9_GetChartImage`), `0x41f750` (suite) ; notes locales `retroanalyse/logiciels/fd9-feuille.md` § 10.
