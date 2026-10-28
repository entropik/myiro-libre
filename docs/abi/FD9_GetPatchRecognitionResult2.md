# FD9_GetPatchRecognitionResult2

**En clair** : après l'événement 7 (reconnaissance finie), rend ce que la DLL a trouvé sur la feuille : un indice de fiabilité, et la géométrie reconnue (zones, nombre de plages, origine et pas de chaque zone), dans la même structure que celle donnée à [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md). Le pont la compare à la mire attendue **avant de mesurer**.

## Signature

```c
int32_t __stdcall FD9_GetPatchRecognitionResult2(uint32_t *fiabilite,
                                                 tFD9_ChartLayout *disposition,        /* NULL admis */
                                                 tFD9_ChartCoordinate *coordonnees);   /* NULL admis ; x86 : ret 0xc */
```

## Ce qui est confirmé

- **Contrôle de l'export** (`fd9-x86` `0x100b2640`) : `fiabilite` non nul, sinon **1001**.
- Fonction de travail `0x1008d300` :
  - hors connexion : code interne 8084, public **1102** ;
  - **état autre que 7** : code interne 8085, public **1104** (`0x1008d336..0x1008d33f`) ;
  - écrit la **fiabilité** (`0x1008d37b`, valeur gardée par la DLL sous le nom « Confidence ») ;
  - recopie le résultat (`0x10085d10`) : `disposition`, si non nul, reçoit 608 octets (`0x98` mots) ; `coordonnees`, si non nul, reçoit l'en-tête (largeur, hauteur, zones, plages), les 50 × {rangées, colonnes, plages} à +0x10 et les 50 géométries à +0x268 ;
  - puis, dans `coordonnees`, **remet à zéro** « modèle présent » (+0x718), les 0x644 octets suivants et l'image modèle (+0xd60, 16 octets) (`0x1008d3ac..0x1008d3cf`). La structure rendue est donc complète et sans pointeur.
- L'export appelle ensuite la lecture de la cause d'erreur de préparation pour son journal (`0x100b26b8`), sans effet sur le retour.

## Contenu

`coordonnees` : même disposition que dans [FD9_StartPatchRecognition](FD9_StartPatchRecognition.md) (0xd70 octets en x86, 0xd78 en x64). `disposition` (`tFD9_ChartLayout`, 608 octets en x86) : non décodée, inutile au pont.

## Propriété de la mémoire

- Les trois structures appartiennent à l'appelant ; la DLL y recopie ses résultats pendant l'appel et ne garde aucune adresse (confirmé).
- Le pont passe une structure de 0xd70 octets (0xd78 en 64 bits) mise à zéro ; la DLL n'écrit pas au-delà.

## Ce qui est supposé

- La géométrie rendue est dans la même unité que celle donnée (1/100 mm).
- La fiabilité est un nombre brut dont le sens et l'échelle ne sont pas établis ; le pont la garde sans l'interpréter.

## Pour le pont

- Appeler avec `disposition` nul et `coordonnees` vers une structure à zéro, comme MYIROtools et FD-S2w.
- **Comparer à la définition** : nombre de zones, nombre de plages de chaque zone (comme MYIROtools, qui refuse « Chart recognition does not match definition »), puis origine et pas de chaque zone à **3 mm près** (300 en 1/100 mm). Un écart refuse la feuille (`MireNonReconnue`), avant toute mesure.
- Garder la fiabilité et le plus grand écart dans la lecture de la feuille.

## À vérifier sur l'instrument

- L'unité et l'écart réel de la géométrie rendue pour une RGB-2520 (essai E3) ; la valeur de la fiabilité.
- Ce qui est rendu pour une page blanche (essai E3 bis).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetPatchRecognitionResult2.asm.txt` (`ret 0xc`) ; `fd9-x64/exports/FD9_GetPatchRecognitionResult2.asm.txt`.
- Fonctions de travail : `desassemble.py fd9-x86 --address 0x1008d300` ; recopie : `--address 0x10085d10`.
- Appels de MYIROtools et de FD-S2w : `retroanalyse/logiciels/fd9-feuille.md` § 3, `fd-s2w.md` § 5.
