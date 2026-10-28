# FD9_StartPatchRecognition

**En clair** : après la prise d'image (événement 5), lance la **reconnaissance des plages** sur l'image de la feuille. Le pont donne la description de la mire attendue (taille de la feuille, zones, origine, taille et pas des plages) ; la DLL cherche les plages et les compare à cette description. La fin est annoncée par l'événement 7 (`WAIT_PATCHRECOG_COMPLETION`) ; le résultat se lit avec [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md).

## Signature

```c
int32_t __stdcall FD9_StartPatchRecognition(const uint32_t *mode,                   /* FD9_eRecogMode */
                                            const tFD9_ChartCoordinate *coordonnees, /* 0xd70 octets en x86 */
                                            const tFD9_PatchColors *couleurs,        /* NULL admis en COMBINE */
                                            const uint32_t *rangees,                 /* mode MATRIX */
                                            const uint32_t *colonnes);               /* x86 : ret 0x14 */
```

## Ce qui est confirmé

### Contrôles de l'export (`fd9-x86` `0x100b1f80`)

- `mode`, `rangees` et `colonnes` sont lus **sans test du pointeur nul** (`0x100b2373..0x100b237e` ; `rangees` et `colonnes` aussi dès que le journal du SDK est actif, `0x100b1fb2..0x100b1fba`). Le pont passe toujours trois pointeurs valides, à 0 hors mode MATRIX.
- Si `coordonnees` est donné :
  - plus de **50 zones** : refus, cause 4 (`TOO_MANY_AREAS`) pour [FD9_GetPremeasurementError](FD9_GetPremeasurementError.md) (`0x100b2077`, `0x100b20e6`) ;
  - plus de **15 000 plages** : refus, cause 5 (`0x100b21b6`, `0x100b222b`) ;
  - pour chaque zone : **`gauche + pas_x × colonnes ≤ largeur de feuille`** et **`haut + pas_y × rangées ≤ hauteur de feuille`**, sinon refus (`0x100b22f3..0x100b2334`). Pas, origine et feuille sont donc dans **la même unité**.
- Exigences de chaque mode (`0x100b2373..0x100b2593`) :

| Mode | Nom | Exigence de l'export |
|---|---|---|
| 0 | AUTO | refusé si des coordonnées sont données sans modèle ni couleurs |
| 1 | DEFINITION | coordonnées obligatoires, avec un modèle ou des couleurs |
| 2 | COORDINATE | coordonnées obligatoires |
| 3 | MATRIX | rangées de 1 à 250, colonnes de 1 à 60 ; sinon cause 5 |
| 4 | **COMBINE** | **aucune exigence propre** : seuls les contrôles des coordonnées ; **`couleurs` nul admis** |

- Un **modèle** (image de référence, `modele_present ≠ 0`) doit avoir des données, une largeur de 1 à 3 638, une hauteur de 1 à 19 000 et 8, 16 ou 24 bits par pixel (`0x100b2555..0x100b2586`).
- Tout refus de l'export : public **1001**.

### Fonction de travail (`0x1008a890`)

- Hors connexion : code interne 8023, public **1102**.
- **État 5 ou 7 seulement** (`WAIT_CAPTURE_COMPLETION`, ou une reconnaissance déjà finie), sinon code interne 8024, public **1104** (`0x1008a8d7..0x1008a8e7`).
- Les paramètres sont recopiés dans un objet de la DLL (`0x100ad370`), puis un fil de reconnaissance est lancé ; retour 0 : la reconnaissance est **lancée**.

## Contenu de `tFD9_ChartCoordinate`

Taille **0xd70 octets (3 440) en x86**, **0xd78 (3 448) en x64**. Tous les nombres sont des entiers de 32 bits sans signe.

| Position | Contenu | Niveau |
|---|---|---|
| +0x000 | largeur de la feuille | confirmé |
| +0x004 | hauteur de la feuille | confirmé |
| +0x008 | nombre de zones (≤ 50) | confirmé |
| +0x00c | nombre total de plages (≤ 15 000) | confirmé |
| +0x010 + 12 × i | zone i : rangées, colonnes, plages (50 zones) | confirmé |
| +0x268 + 24 × i | zone i : gauche, haut, largeur de plage, hauteur de plage, pas X, pas Y (50 zones) | confirmé ; noms tirés du journal du SDK |
| +0x718 | modèle présent (0 ou 1) | confirmé |
| +0x71c | 0x644 octets non décodés (un compte, puis 50 × 32 octets) | à mettre à zéro |
| +0xd60 | image modèle `tFD9_Image` : largeur, hauteur, bits par pixel, données (16 octets en x86, 24 en x64) | confirmé x86 (`0x100ad458..0x100ad498`) |

Les positions jusqu'à +0xd60 sont les mêmes en 32 et 64 bits ; seule l'image finale grandit, à cause du pointeur.

`tFD9_PatchColors` : `{type, nombre, liste*}`, entrées de 28 octets `{zone, rangée, colonne, 4 flottants}` ; 12 octets en x86 (copie `0x100ad4b7..0x100ad537`), 16 en x64. Le pont n'en passe pas.

## La RGB-2520

Nombres relevés dans la définition de mire de MYIROtools (unité : 1/100 mm) : feuille 27 940 × 41 660 ; une zone de 60 rangées × 42 colonnes, 2 520 plages ; origine (1 349 ; 2 300) ; plage 601 × 601 ; pas 601 × 601. Contrôles de la DLL : 1 349 + 601 × 42 = 26 591 ≤ 27 940 ; 2 300 + 601 × 60 = 38 360 ≤ 41 660. MYIROtools la reconnaît en mode COMBINE (4).

## Propriété de la mémoire

- **Tout est recopié avant le retour** (confirmé, `0x100ad370`) : les 0xd70 octets des coordonnées, les données de l'image modèle (dans un tampon alloué par la DLL), l'en-tête des couleurs et toutes leurs entrées, les valeurs de rangées et colonnes. L'appelant peut libérer ses structures dès le retour.
- La DLL libère elle-même ses copies à la reconnaissance suivante (`0x100ad3a3..0x100ad408`).

## Ce qui est supposé

- L'unité des coordonnées est le 1/100 mm : FD-S2w recopie les définitions de mire champ par champ, sans conversion (`fd9-feuille.md` § 5), et ses fichiers donnent des plages de 600 pour 6 mm.
- COMBINE = reconnaissance automatique des plages, puis comparaison aux coordonnées données ; il corrige un décalage de la feuille (tolérance de 21 mm d'après le manuel de FD-S2w).
- COMBINE sans couleurs reconnaît la RGB-2520.

## Pour le pont

- Construire la structure à zéro, puis y écrire la définition ; refuser avant l'appel une définition hors bornes (mêmes règles que la DLL).
- Mode **COMBINE (4)**, `couleurs` nul, `rangees` et `colonnes` vers des zéros. Repli prévu : COORDINATE (2), sur réglage explicite.
- L'appeler dans le fil de la session, après l'événement 5 et la lecture du QR ; attendre ensuite l'événement 7.

## À vérifier sur l'instrument

- COMBINE sans couleurs reconnaît-il la RGB-2520 ? Sinon, COORDINATE (essai E3).
- L'unité et l'écart de la géométrie rendue par [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md).

## Preuves (locales)

- `fd9-x86/exports/FD9_StartPatchRecognition.asm.txt` (contrôles, `ret 0x14`) ; `fd9-x64/exports/FD9_StartPatchRecognition.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008a890` ; recopie : `--address 0x100ad370`.
- Taille x64 : `retroanalyse/logiciels/fd-s2w.md` § 4.3 (effacement de 0xd78 octets) ; noms des champs : journal du SDK, même section.
- Définition de la RGB-2520 et séquence de MYIROtools : `retroanalyse/logiciels/fd9-feuille.md` § 2 et 3 ; conversion de FD-S2w : § 5.
