# FD9_GetChartImage

**En clair** : après la reconnaissance (événement 7), rend **l'image de la feuille** prise par l'instrument, découpée au bord du papier, en gris. C'est la photo que FD-S2w montre dans son onglet « Measured Point », avec un point rouge sur chaque plage. Le pont la réduit et la donne à l'application avec les points de [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md), pour que l'opérateur voie où l'instrument va mesurer. Lecture seule : rien n'est écrit dans l'instrument ni dans la DLL.

## Signature

```c
typedef struct { uint32_t largeur, hauteur, bits_par_pixel; uint8_t *donnees; } tFD9_Image;  /* 16 octets en x86, 24 en x64 */

int32_t __stdcall FD9_GetChartImage(tFD9_Image *image,
                                    tFD9_PatchMeasurePoint *points,   /* facultatif */
                                    const uint32_t *nombre);           /* x86 : ret 0xc */
```

`tFD9_Image` est la structure de [FD9_GetCaptureResult](FD9_GetCaptureResult.md) ; `tFD9_PatchMeasurePoint` celle de [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md).

## Ce qui est confirmé

### Contrôles de l'export (`fd9-x86` `0x100b17d0`)

- `image` nul : **1001** (`0x100b1800`).
- Hors connexion : code interne 8081, public **1102** (`0x100b180b`).
- **États 1 à 6 refusés** : code interne 8052, public **1104** (`0x100b181b..0x100b183c`). Les autres états, 0 compris, passent à la fonction de travail. Après la reconnaissance, l'état est 7.
- `nombre` est lu **sans test du pointeur nul** dès que le journal du SDK est actif (`0x100b17e7`) : le pont passe toujours un pointeur valide.
- Retour : le code de la fonction de travail, regroupé (`0x100b43b0`, table de `FD9SDK.md`).

### Fonction de travail (`CJungfrauCore`, `0x100a0780`)

- Un drapeau de l'objet (+0x1540) choisit entre deux chemins. À 0, le chemin décrit ici ; à 1, un autre (`0x1009b8c0`), non suivi, qui rend toujours 0.
- La DLL calcule d'abord la taille de l'image du papier (`0x1005e980`, échelle 1,0). Elle la compare à **`largeur × hauteur / (bits_par_pixel / 8)`**, lus dans la structure donnée : plus petit, refus interne 0x855, public **1005** (`0x100a0808..0x100a0821`), rien n'est écrit. `bits_par_pixel` sous 8 donne une **division par zéro** (`shr ecx, 3`, puis `div ecx`) : le pont passe toujours 8.
- Puis `CJungfrauCore::GetChartPaperImage` (`0x1009c830`) avec `donnees`, `&largeur`, `&hauteur` et la taille ; enfin, si `points` n'est pas nul et `*nombre` non plus, les points comme [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md) (`0x1009c920`).
- `PatchPosExtractor::GetChartPaperImage` (`0x1005ef60`) :
  - met `largeur`, `hauteur` et la taille à 0, puis les **réécrit** avec celles de l'image du papier ;
  - découpe ce rectangle dans l'image prise (`cv::Mat(image, Rect)`, opencv 2.4.7), et le copie (ou le redimensionne si la taille diffère) dans une `cv::Mat(hauteur, largeur, CV_8UC1, donnees)` : **gris sur 8 bits, une rangée après l'autre, sans bourrage** ;
  - tampon trop court pour la taille annoncée : code 0x1c, puis public **1999** (0x82d absent de la table).
- La DLL ne garde aucune adresse de l'appelant : elle écrit dans `donnees` pendant l'appel.

### FD-S2w (`s2w-x86`)

- `0x41ed00` enveloppe l'appel : il rend `FD9_GetChartImage(image, points, &nombre)` ; sans points (ou avec 0 points), **1005 est pris pour un succès**.
- Son appelant (`0x446280..0x4463f6`, fonction de `CModel` après `GetClippingChartResult`) prépare un tampon fixe : `largeur` = 3 638, `hauteur` = 19 000, `bits_par_pixel` = 8, `donnees` = 69 122 000 octets (3 638 × 19 000) mis à zéro, plus un tableau de points. 3 638 pixels est aussi la largeur maximale d'une image modèle de [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md).
- L'image de l'onglet « Measured Point » à l'événement 7 (`0x41e490`) est la **copie de l'image prise** gardée par FD-S2w depuis [FD9_GetCaptureResult](FD9_GetCaptureResult.md) (`0x41eca0`), pas un appel à cet export.

## Propriété de la mémoire

- L'appelant alloue `donnees` pour au moins `largeur × hauteur` octets (8 bits) ; la DLL y écrit l'image et rend sa taille réelle dans `largeur` et `hauteur`. Le tableau de points, s'il est donné, appartient aussi à l'appelant.

## Ce qui est supposé

- **Résolution : 300 ppp**, comme l'image prise (les points sont convertis de pixels à 300 ppp en 1/100 mm, fiche `FD9_GetMeasurePoint`) ; l'échelle 1,0 le laisse penser.
- **Origine** : celle des points de mesure. Un point `(x, y)` en 1/100 mm tombe sur le pixel `(x × 300 / 2540, y × 300 / 2540)` de cette image. À vérifier à l'essai : chaque point doit tomber au centre d'une plage. Doute : FD-S2w dessine ses points sur l'image **prise** entière (sa copie de `FD9_GetCaptureResult`), pas sur cette image découpée au bord du papier. Si les points sont comptés depuis le coin de l'image prise, ils seront décalés ici de la marge découpée ; l'essai le tranchera (au besoin, le pont rendra plutôt l'image prise, qu'il lit déjà).
- L'image du papier est une partie de l'image prise : elle n'a pas plus de pixels qu'elle. Si ce n'est pas le cas, la DLL refuse (1005) sans rien écrire.
- Une feuille vue tournée (90°, 180°, 270°) : l'image n'est pas retournée (non suivi).

## Pour le pont

- À l'événement 7, après [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md) et ses contrôles, aux paliers Prénumérisation et Feuille (avant `FD9_StartMeasurement2`).
- `points` nul, `nombre` vers 0 : l'image seule.
- Tampon : `largeur` = 3 638, `hauteur` = la taille de l'image prise divisée par 3 638 (arrondie au-dessus, au plus 19 000), `bits_par_pixel` = 8 ; `donnees` de `largeur × hauteur` octets. Moins que les 69 Mo de FD-S2w, assez si l'image du papier est une partie de l'image prise ; sinon la DLL refuse (1005) sans rien écrire.
- Taille rendue plus grande que le tampon : réponse inattendue, image ignorée.
- **Un échec ne fait pas échouer la feuille** : le pont le note au journal et rend le calage sans image.
- Réduction par le pont à environ 1 000 pixels de haut (moyenne de carrés de k × k pixels, k diviseur de 300 : la résolution reste un nombre entier de ppp), gris 8 bits, donnée à l'application dans le champ `calage` (contrat de [FD9_GetMeasurePoint](FD9_GetMeasurePoint.md)). L'image peut montrer ce que l'opérateur a écrit à la main sur la feuille : jamais dans un fichier versionné, jamais dans un test autre que synthétique.

## À vérifier sur l'instrument

- Taille rendue pour la RGB-2520 sur A3 (attendue autour de 3 300 × 4 920 à 300 ppp).
- Les points de mesure tombent-ils au centre des plages sur l'image réduite ?

## Preuves (locales)

- `fd9-x86/exports/FD9_GetChartImage.asm.txt` (contrôles, `ret 0xc`) ; `fd9-x64/exports/FD9_GetChartImage.asm.txt`.
- Fonction de travail : `desassemble.py fd9-x86 --address 0x100a0780` ; découpe : `--address 0x1009c830`, `--address 0x1005ef60`.
- FD-S2w : `desassemble.py fds2w --address 0x41ed00` ; appelant : `--address 0x446280 --end 0x446420` ; image de l'onglet : `--address 0x41e490`, `--address 0x41eca0`.
- Codes publics : table `fd9-x86` `0x10108ca0` (0x855 dans le groupe 1005, 8081 dans 1102, 8052 dans 1104).
