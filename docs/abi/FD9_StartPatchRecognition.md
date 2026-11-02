# FD9_StartPatchRecognition

**En clair** : après la prise d'image (événement 5), lance la **reconnaissance des plages** sur l'image de la feuille. Le pont donne la description de la mire attendue (taille de la feuille, zones, origine, taille et pas des plages) ; la DLL cherche les plages et les compare à cette description. La fin est annoncée par l'événement 7 (`WAIT_PATCHRECOG_COMPLETION`) ; le résultat se lit avec [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md).

## Signature

```c
int32_t __stdcall FD9_StartPatchRecognition(const uint32_t *mode,                   /* FD9_eRecogMode */
                                            const tFD9_ChartCoordinate *coordonnees, /* 0xd70 octets en x86 */
                                            const tFD9_PatchColors *couleurs,        /* exigé en COMBINE sans modèle */
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
| 4 | COMBINE | aucune exigence propre : seuls les contrôles des coordonnées ; `couleurs` nul **accepté par l'export, mais fatal plus loin** (voir « Fil de reconnaissance ») |

- Un **modèle** (image de référence, `modele_present ≠ 0`) doit avoir des données, une largeur de 1 à 3 638, une hauteur de 1 à 19 000 et 8, 16 ou 24 bits par pixel (`0x100b2555..0x100b2586`).
- Tout refus de l'export : public **1001**.

### Fonction de travail (`0x1008a890`)

- Hors connexion : code interne 8023, public **1102**.
- **État 5 ou 7 seulement** (`WAIT_CAPTURE_COMPLETION`, ou une reconnaissance déjà finie), sinon code interne 8024, public **1104** (`0x1008a8d7..0x1008a8e7`).
- Les paramètres sont recopiés dans un objet de la DLL (`0x100ad370`), puis un fil de reconnaissance est lancé ; retour 0 : la reconnaissance est **lancée**.

### Fil de reconnaissance (essai E3 du 30 octobre 2026)

- Objet `CPatchRecognitionThread` (0xda4 octets, créé une fois à `0x1008a8f8`, table de méthodes `0x1011c470`) : copie des coordonnées à +0x18, mode à +0xd88, en-tête des couleurs `{type, nombre}` à +0xd8c et leur liste à +0xd94, « coordonnées données » à +0xd98, « couleurs données » à +0xd99 (`0x100ad370`). Sans couleurs, +0xd99 vaut 0.
- Lancement (`0x100ad2b0`) : un `std::thread` de la bibliothèque C++ (pile par défaut de l'exécutable : 1 Mo pour le pont comme pour FD-S2w, lu dans leurs en-têtes PE), corps `0x100ad620`. Le fil annonce d'abord son état (`0x100ad560`), ce qui donne l'événement 6, et attend que l'événement soit parti avant de continuer (supposé : drapeau +0x578 de l'objet principal), puis appelle `CJungfrauCore::PatchRecognition` (`0x100a0ec0`) avec le **pointeur des couleurs nul** si +0xd99 vaut 0.
- Dans `PatchRecognition`, pour les modes 0, 1 et 4 avec des coordonnées, et sans résultat précédent réutilisable : **si « modèle présent » (+0x718) vaut 0, la DLL fabrique une image modèle à partir des couleurs** (« CreateChartImage »), et lit leur type **sans tester le pointeur** : `mov edi, [esp+0x38]` (`0x100a14e0`, 4ᵉ argument), puis `cmp dword ptr [edi], 1` (`0x100a14f1`). Couleurs nulles : **lecture à l'adresse 0, le processus s'arrête net**, sans message (confirmé par lecture ; c'est l'arrêt de l'essai E3, juste après l'événement 6).
- Type des couleurs lu à cet endroit : 0 = quatre valeurs par plage, converties par `(1 − v/100) × 255` (supposé : CMJN en %) ; 1 = trois valeurs moyennées telles quelles (supposé : RVB de 0 à 255) ; autre : refus interne 0x856.
- **COORDINATE (2)** ne passe pas par là : `CoordinateMatching` (`0x10097260`) ne reçoit que la copie des coordonnées et l'image prise ; aucune lecture des couleurs (confirmé).
- MYIROtools ne tombe jamais dans ce cas : sa fonction qui retient la mire (`mit-x64` `0x1403ec2a0`) n'accepte une définition **qu'avec ses couleurs** (les deux pointeurs non nuls), et passe toujours l'adresse de ses couleurs à `FD9_StartPatchRecognition` (confirmé).

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

## Contenu de `tFD9_PatchColors` (étude du 30 octobre 2026)

`{type, nombre, entrées*}` : 12 octets en x86 (copie `0x100ad4b7..0x100ad537`), 16 en x64. Entrées de 28 octets `{zone, rangée, colonne, v0, v1, v2, v3}` (entiers puis 4 flottants).

| Point | Niveau | Preuve |
|---|---|---|
| `type` 1 = **RVB** (trois valeurs), 0 = **CMJN** (quatre valeurs), autre : refus interne 0x856 | confirmé | textes « RGB » et « CMYK » de `CreateChartImage` (`0x100afa00`), nombre de valeurs `0x100a1683..0x100a169b` |
| RVB de **0 à 255** en flottants (CMJN en %, converti par `(1 − v/100) × 255`) | confirmé | peinture de l'image modèle `0x100af2b0` : plafond 242 pour un blanc au-dessus de 242, arrondi à l'octet |
| Les valeurs sont peintes **dans l'ordre de la liste**, zone par zone, rangée par rangée, de gauche à droite ; `zone` sert à grouper, `rangée` et `colonne` ne placent rien | confirmé | regroupement par zone `0x100a16d7..0x100a17b1`, parcours rangée puis colonne `0x100af2b0` |
| L'image modèle est dessinée à **300 ppp** depuis la définition (`× 300 / 2540`) : les coordonnées sont bien au **1/100 mm** | confirmé | constante 0,11811… (`0x100af860`, `0x100af2b0`) |
| MYIROtools : `type` = 1 pour une plage RVB, entrées `{zone, rangée, colonne}` comptées depuis 0 dans l'ordre de la définition, valeurs = `ColorValue` de sa définition `.cdx` **divisée par 100** (8500 → 85,0) | confirmé | `mit-x64` `0x1402642e0` (liste), `0x140232f06` (lecture ÷ 100,0), `0x14023ad1c` (écriture × 100,0) |
| MYIROtools n'accepte une définition **qu'avec** ses couleurs | confirmé | `mit-x64` `0x1403ec2a0` |

## La RGB-2520

Nombres relevés dans la définition de mire de MYIROtools (unité : 1/100 mm) : feuille 27 940 × 41 660 ; une zone de 60 rangées × 42 colonnes, 2 520 plages ; origine (1 349 ; 2 300) ; plage 601 × 601 ; pas 601 × 601. Contrôles de la DLL : 1 349 + 601 × 42 = 26 591 ≤ 27 940 ; 2 300 + 601 × 60 = 38 360 ≤ 41 660. MYIROtools la reconnaît en mode COMBINE (4).

## Propriété de la mémoire

- **Tout est recopié avant le retour** (confirmé, `0x100ad370`) : les 0xd70 octets des coordonnées, les données de l'image modèle (dans un tampon alloué par la DLL), l'en-tête des couleurs et toutes leurs entrées, les valeurs de rangées et colonnes. L'appelant peut libérer ses structures dès le retour.
- La DLL libère elle-même ses copies à la reconnaissance suivante (`0x100ad3a3..0x100ad408`).

## Ce qui est supposé

- COORDINATE place les plages d'après la définition seule, sans aucun recalage : c'est la cause de la mesure fausse de l'essai E4 bis (confirmé par la mesure ; le mécanisme est supposé : `CoordinateMatching` n'a pas été suivi jusqu'au bout).
- FD-S2w : sa première reconnaissance est en mode 4 (réglage par défaut) ; après correction des points à la main (« Edit measurement points »), il la relance à l'événement 7 (`CFD9SDKController::ReStartPatchRecognition`, `s2w-x86` `0x41f4c0`) en mode 1 (DEFINITION) ou 2 (COORDINATE) selon un réglage, avec la définition corrigée. Aucun export n'envoie de points corrigés : on renvoie une définition corrigée.
- L'unité des coordonnées est le 1/100 mm : FD-S2w recopie les définitions de mire champ par champ, sans conversion (`fd9-feuille.md` § 5), et ses fichiers donnent des plages de 600 pour 6 mm.
- COMBINE = reconnaissance automatique des plages, puis comparaison aux coordonnées données ; il corrige un décalage de la feuille (tolérance de 21 mm d'après le manuel de FD-S2w).

## Pour le pont

- Construire la structure à zéro, puis y écrire la définition ; refuser avant l'appel une définition hors bornes (mêmes règles que la DLL).
- Mode **COMBINE (4) avec les couleurs RVB de la mire** (type 1, 0 à 255, ordre de la définition), comme MYIROtools et FD-S2w : la DLL retrouve la feuille sur l'image prise, et la mesure vise les plages trouvées. **Jamais COORDINATE** : à l'essai E4 bis, la RGB-2520 imprimée sur A3 était décalée d'environ 8,8 mm, et COORDINATE a mesuré à côté des plages (ΔE00 moyen de 38 avec la mesure de FD-S2w de la même feuille).
- Une mire sans couleurs est refusée avant la feuille (`parametre_refuse`) ; l'adapter de la DLL refuse COMBINE sans couleurs ni modèle (essai E3) ; l'option `--reconnaissance` est retirée.
- `rangees` et `colonnes` vers des zéros.
- L'appeler dans le fil de la session, après l'événement 5 et la lecture du QR ; attendre ensuite l'événement 7.

## À vérifier sur l'instrument

- ~~COMBINE sans couleurs reconnaît-il la RGB-2520 ?~~ Non : il plante (essai E3). ~~COORDINATE ?~~ Non : il mesure à côté (essai E4 bis). COMBINE avec les couleurs : fiabilité, décalage et écart des points pour la RGB-2520 sur A3 (essai E4 ter).
- L'unité et l'écart de la géométrie rendue par [FD9_GetPatchRecognitionResult2](FD9_GetPatchRecognitionResult2.md).

## Preuves (locales)

- `fd9-x86/exports/FD9_StartPatchRecognition.asm.txt` (contrôles, `ret 0x14`) ; `fd9-x64/exports/FD9_StartPatchRecognition.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x1008a890` ; recopie : `--address 0x100ad370`.
- Fil de reconnaissance : `--address 0x100ad2b0` (lancement), `--address 0x100ad620` (corps) ; lecture des couleurs nulles : `desassemble.py fd9-x86 --address 0x100a0ec0 --limit 6000`, lignes `0x100a14a2..0x100a14f4` ; COORDINATE : `--address 0x10097260`.
- Pile des fils : `SizeOfStackReserve` 0x100000 dans `pont-fd9.exe` (i686) et `FD-S2w.exe`.
- Taille x64 : `retroanalyse/logiciels/fd-s2w.md` § 4.3 (effacement de 0xd78 octets) ; noms des champs : journal du SDK, même section.
- Définition de la RGB-2520 et séquence de MYIROtools : `retroanalyse/logiciels/fd9-feuille.md` § 2 et 3 ; conversion de FD-S2w : § 5.
