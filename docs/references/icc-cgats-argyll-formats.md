# Formats CGATS.17 et fichiers .ti1 / .ti2 / .ti3 / .cal d'ArgyllCMS

Date de recherche : 7 octobre 2026.

## Résumé

- CGATS.17 est un format texte d'échange de mesures de couleur (spectres, Lab, densités) ; c'est l'adoption américaine de l'ISO 28178:2009. La norme est payante, mais le format est simple : quelques mots-clés d'en-tête, la liste des colonnes, puis les lignes de données.
- ArgyllCMS réutilise ce principe pour ses propres fichiers : `.ti1` (valeurs à imprimer), `.ti2` (mise en page), `.ti3` (valeurs et mesures : c'est ce que lit `colprof`), `.cal` (courbes de calibration). Ces fichiers portent un identifiant propre (`CTI3`, `CAL`) pour qu'on ne les confonde pas avec du CGATS standard.
- Notre chaîne (mesure avec nos ponts, puis profil avec ArgyllCMS) revient à **écrire des fichiers `.ti3` corrects** : mêmes `SAMPLE_ID` que le `.ti1` / `.ti2`, colonnes CMYK en pourcentages, Lab et spectres `SPEC_380` à `SPEC_730`.
- Le MYIRO-1 donne 36 bandes de 380 à 730 nm : cela correspond exactement à l'exemple `SPECTRAL_BANDS 36` du format `.ti3`.
- Des lecteurs existent (Little CMS : famille `cmsIT8*`, sous licence MIT ; une caisse Rust `cgats` en GPL-3.0, mais archivée).
- Les fichiers de référence de Fogra (FOGRA39 et suivants) sont au même format texte et sont librement redistribuables sous conditions.

## 1. Le format CGATS.17

- CGATS.17-2009 est l'adoption américaine identique de l'ISO 28178:2009 ; il définit un format d'échange pour les données de couleur et de contrôle de procédé, en texte ou en XML, lisible par l'humain et par la machine, avec des mots-clés prédéfinis et la possibilité d'en définir d'autres. [établi] (https://webstore.ansi.org/standards/npes/ansicgats172024 et pages voisines de la boutique ANSI : résumés seulement)
- Structure lue dans les exemples publics : première ligne d'identification, mots-clés d'en-tête (`DESCRIPTOR`, `ORIGINATOR`, `CREATED`...), `NUMBER_OF_FIELDS`, bloc `BEGIN_DATA_FORMAT` ... `END_DATA_FORMAT` (noms des colonnes), `NUMBER_OF_SETS`, bloc `BEGIN_DATA` ... `END_DATA`. [établi] (https://argyllcms.com/doc/ti3_format.html)
- Le contenu complet de la norme (mots-clés prédéfinis, règles d'écriture) est payant : ne pas chercher à l'implémenter en entier ; viser le sous-ensemble qu'ArgyllCMS et Fogra emploient. [établi pour le prix ; à vérifier pour la liste des mots-clés]

## 2. Les fichiers d'ArgyllCMS

Description officielle : https://www.argyllcms.com/doc/File_Formats.html [établi]

| Fichier | Rôle | Produit par |
|---|---|---|
| `.ti1` | valeurs d'appareil prêtes pour une mire, avec valeurs CIE estimées | `targen` |
| `.ti2` | valeurs placées dans une mise en page de mire, avec l'emplacement de chaque plage | `printtarg` |
| `.ti3` | paires valeurs d'appareil / valeurs CIE ou spectrales : les données brutes de `colprof` | `chartread`, `scanin`, `txt2ti3`, `cb2ti3`... |
| `.cal` | courbes de calibration de l'appareil (linéarisation) | `printcal`, `dispcal` |
| `.cht` | description de la mire pour reconnaître une image scannée | `printtarg -s` ou `-S` |
| `.ccmx` | matrice de correction de colorimètre | outils de correction |
| `.sp` | un ou plusieurs spectres (illuminant, couleur) | outils spectraux |

La page de présentation dit s'appuyer sur le format d'échange CGATS.5 (annexe J). [établi]

### 2.1 Le `.ti3` (format décrit en détail)

Source : https://argyllcms.com/doc/ti3_format.html [établi]

- Identifiant du fichier : `CTI3`. Mots-clés obligatoires : `DEVICE_CLASS` (`OUTPUT` pour une imprimante, `DISPLAY`, `INPUT`, `EMISINPUT`), `COLOR_REP` (couple espace d'appareil / espace de couleur, par exemple `CMYK_LAB`, `RGB_XYZ`), `NUMBER_OF_FIELDS`, `NUMBER_OF_SETS`, `BEGIN_DATA_FORMAT` / `END_DATA_FORMAT`, `BEGIN_DATA` / `END_DATA`.
- Mots-clés facultatifs utiles : `DESCRIPTOR`, `ORIGINATOR`, `CREATED`, `TOTAL_INK_LIMIT` (limite d'encre totale en %), `FINAL_TOTAL_INK_LIMIT` (après calibration), `ILLUMINANT_WHITE_POINT_XYZ` (D50 par défaut), `TARGET_INSTRUMENT`, `INSTRUMENT_TYPE_SPECTRAL` (YES/NO), `INSTRUMENT_FILTER` (`POLARIZED`, `D65` ou `UVCUT`), `DEVCALSTD` (étalon du fabricant : `XRDI`, `GMDI` ou `XRGA`), `SPECTRAL_BANDS`, `SPECTRAL_START_NM`, `SPECTRAL_END_NM`.
- Colonnes : `SAMPLE_ID`, `SAMPLE_LOC` (position de la plage, entre guillemets), `CMYK_C`, `CMYK_M`, `CMYK_Y`, `CMYK_K` (0 à 100 %), `RGB_R`, `RGB_G`, `RGB_B`, `XYZ_X`, `XYZ_Y`, `XYZ_Z` (normalisés à Y = 100), `LAB_L`, `LAB_A`, `LAB_B`, `SPEC_380` ... (réflectance spectrale).
- Exemple public d'en-tête (RVB, 441 plages) : `CTI3`, `DESCRIPTOR "Argyll Calibration Target chart information 3"`, `ORIGINATOR "Argyll printread"`, `DEVICE_CLASS "OUTPUT"`, `COLOR_REP "iRGB_LAB"`, colonnes `SAMPLE_ID SAMPLE_LOC RGB_R RGB_G RGB_B LAB_L LAB_A LAB_B`. [établi, d'après une recherche de la documentation]
- `NORMALIZED_TO_Y_100` ne concerne que les écrans. Comment `colprof` interprète précisément chaque mot-clé n'est pas décrit sur cette page. [établi]

### 2.2 Le `.cal`

Source : https://argyllcms.com/doc/cal_format.html [établi]

- Identifiant : `CAL`. Mots-clés : `DEVICE_CLASS`, `COLOR_REP` (lettres C, M, Y, K, R, G, B, W ; préfixes pour les encres claires), `NUMBER_OF_FIELDS`, `NUMBER_OF_SETS`. Colonnes par exemple `CMYK_I`, `CMYK_C`, `CMYK_M`, `CMYK_Y`, `CMYK_K`, avec des valeurs de 0,0 à 1,0.

### 2.3 Importer des mesures d'autres sources

- `txt2ti3` : convertit en `.ti3` des résultats de mires RVB ou CMJN d'autres logiciels (formats connus de certains fabricants) ; accepte un fichier unique ou deux ou trois fichiers (valeurs d'appareil, CIE, spectres) ; détecte l'échelle (1, 100 ou 255) ; options `-l` (limite d'encre 0 à 400 %), `-2` (crée aussi un `.ti2`). [établi] (https://argyllcms.com/doc/txt2ti3.html)
- `cb2ti3` : conversion d'un format précis pour RVB ou CMY seulement : pas utile pour nous. [établi] (https://argyllcms.com/doc/cb2ti3.html)
- `average` : moyenne ou fusion de fichiers de mesure ; `spec2cie` : calcule XYZ et Lab D50 à partir de spectres et applique la compensation des azurants. [établi] (https://www.argyllcms.com/doc/ArgyllDoc.html)

## 3. Lecteurs et écrivains libres

- Little CMS : famille de fonctions `cmsIT8LoadFromFile`, `cmsIT8SaveToFile`, `cmsIT8GetPropertyDbl`, `cmsIT8GetDataRowColDbl`, etc. (lecture/écriture du format IT8/CGATS). Licence MIT. [établi pour les noms des fonctions (en-tête `lcms2.h`) ; leur aptitude à lire les `.ti3` d'Argyll : à vérifier] (https://raw.githubusercontent.com/mm2/Little-CMS/master/include/lcms2.h)
- Caisse Rust `cgats` : lit, écrit, moyenne ; licence GPL-3.0 ; dernière version 0.2.0 de novembre 2022 ; dépôt **archivé** (dernier envoi le 4 janvier 2026). [établi] (https://github.com/ryanobeirne/cgats, https://lib.rs/crates/cgats) Ce que dit `docs/references/icc.md` (« lecture, écriture et moyenne ») reste vrai, mais il faut ajouter qu'elle n'est plus entretenue.
- Écrire notre propre lecteur/écrivain dans une caisse du dépôt est probablement plus simple que d'adopter l'une de ces dépendances : le sous-ensemble utile est petit. [probable]

## 4. Exemples publics

- Fichiers de référence Fogra (FOGRA39.txt de 1 485 plages, FOGRA39L.txt de 1 617 plages ; FOGRA51 et suivants) : https://registry.color.org/cmyk-registry/fogra39 et https://fogra.org/en/downloads/work-tools/characterisation-data. Redistribution libre à condition de ne pas modifier le fichier et de citer Fogra. [établi]
- Il existe un export CGATS de référence du MYIRO-1 dans les archives locales du dépôt (voir `CLAUDE.md`) ; sa structure est à comparer au `.ti3` d'Argyll.

## 5. Ce qu'on en tire pour myiro-libre

- Écrire dans `.ti3` ce que le pont a mesuré : spectres, Lab calculé, `INSTRUMENT_TYPE_SPECTRAL YES`, `SPECTRAL_BANDS 36`, `SPECTRAL_START_NM 380`, `SPECTRAL_END_NM 730`, `TARGET_INSTRUMENT` et `INSTRUMENT_FILTER` s'ils sont connus et exacts, plus un champ de provenance dans un commentaire ou un mot-clé à nous (CGATS permet d'en ajouter).
- Tester l'aller-retour : un `.ti3` écrit par nous doit passer `colprof` et `profcheck` sans erreur.
- Conserver la correspondance `SAMPLE_ID` / `SAMPLE_LOC` avec le `.ti2` produit par `printtarg` : sans elle, `colprof` ne retrouve pas les valeurs d'appareil.

## 6. Questions ouvertes

- Liste complète des mots-clés CGATS.17 prédéfinis (norme payante) : pas nécessaire tant que l'on s'en tient au format d'Argyll. [à vérifier]
- Comment un `.ti3` signale-t-il la condition M0/M1/M2 autrement que par `INSTRUMENT_FILTER` ? Pas de mot-clé dédié trouvé. [à vérifier]
- Comment Argyll relit-il les spectres d'un instrument absent de sa liste (aucun mot-clé de décalibration connu pour Konica Minolta) : `DEVCALSTD` ne cite que des étalons X-Rite. [à vérifier]
- Que fait `colprof` avec un `.ti3` sans `TOTAL_INK_LIMIT` ? À essayer sans instrument.

## Sources consultées

- https://www.argyllcms.com/doc/File_Formats.html
- https://argyllcms.com/doc/ti3_format.html
- https://argyllcms.com/doc/cal_format.html
- https://argyllcms.com/doc/txt2ti3.html
- https://argyllcms.com/doc/cb2ti3.html
- https://www.argyllcms.com/doc/ArgyllDoc.html
- https://webstore.ansi.org/standards/npes/ansicgats172024
- https://raw.githubusercontent.com/mm2/Little-CMS/master/include/lcms2.h
- https://github.com/ryanobeirne/cgats
- https://lib.rs/crates/cgats
- https://registry.color.org/cmyk-registry/fogra39
- https://fogra.org/en/downloads/work-tools/characterisation-data
