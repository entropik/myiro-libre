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

## La fiabilité (étude du 30 octobre 2026)

- Ce n'est pas une note : ce sont des **drapeaux d'alerte** de la reconnaissance (`m_confidence`, `+0x158c` du cœur ; textes « ConfidenceFlags: 0x%08X », « m_warn_confidence »), **0 sans alerte** (confirmé, `0x1008d300`).
- Trois drapeaux sont changés par la DLL elle-même en cause de préparation ([FD9_GetPremeasurementError](FD9_GetPremeasurementError.md)) : **0x4000 → 1** (densité différente de la définition), **0x2000 → 2** (taille différente), **0x8000 → 3** (autre différence) (confirmé, `PremeasureErrorCodeFromConfidence`, `0x10097bf0`). Les autres bits ne sont pas décodés.
- **En COORDINATE**, elle est remise à zéro au début de `CJungfrauCore::PatchRecognition` (`0x100a0ec0`) et aucun drapeau n'est posé : 0 ne prouve rien (essai E4 bis : fiabilité 0, mesure fausse).
- **FD-S2w** : une fiabilité non nulle arrête l'enchaînement automatique, sans mesurer (`FD9_StopMeasurement`), et montre les points à vérifier, avec « Measurement points might not be recognized correctly. Check the measurement points. » (`s2w-x86` `0x41f750`) (confirmé). **MYIROtools** ne la lit pas (`mit-x64` `0x1403eaca0`, état 7) (confirmé).

## Ce qui est supposé

- La géométrie rendue est dans la même unité que celle donnée (1/100 mm) ; en COMBINE, l'origine rendue peut suivre le décalage de la feuille.

## Pour le pont

- Appeler avec `disposition` nul et `coordonnees` vers une structure à zéro, comme MYIROtools et FD-S2w.
- **Fiabilité non nulle : la feuille est refusée** (`reconnaissance_douteuse`), comme FD-S2w qui ne mesure pas sans vérification.
- **Comparer à la définition** : nombre de zones, nombre de plages de chaque zone (comme MYIROtools et FD-S2w), puis pas de chaque zone à **3 mm près**. L'origine n'est plus comparée : COMBINE recale la feuille, et son décalage se lit sur les [points de mesure](FD9_GetMeasurePoint.md). Un écart refuse la feuille (`MireNonReconnue`), avant toute mesure.
- Garder la fiabilité et le plus grand écart dans la lecture de la feuille.

## À vérifier sur l'instrument

- L'unité et l'écart réel de la géométrie rendue pour une RGB-2520 (essai E3) ; la valeur de la fiabilité.
- Ce qui est rendu pour une page blanche (essai E3 bis).

## Preuves (locales)

- `fd9-x86/exports/FD9_GetPatchRecognitionResult2.asm.txt` (`ret 0xc`) ; `fd9-x64/exports/FD9_GetPatchRecognitionResult2.asm.txt`.
- Fonctions de travail : `desassemble.py fd9-x86 --address 0x1008d300` ; recopie : `--address 0x10085d10`.
- Appels de MYIROtools et de FD-S2w : `retroanalyse/logiciels/fd9-feuille.md` § 3, `fd-s2w.md` § 5.
