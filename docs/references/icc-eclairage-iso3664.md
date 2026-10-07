# Contrôle d'éclairage : ISO 3664 (cabine de visualisation, D50, rendu des couleurs, métamérie, uniformité)

Date de recherche : 7 octobre 2026. Norme citée : ISO 3664, « Graphic technology and photography — Viewing conditions » (payante). Rien n'en est recopié.

## Résumé

- L'ISO 3664 dit sous quelle lumière on compare un tirage à son épreuve : lumière du jour normalisée D50, intensité, uniformité, rendu des couleurs, entourage neutre.
- **Attention : une nouvelle édition existe.** L'ISO 3664:2025 (4e édition, juillet 2025) remplace l'édition 2009. Elle ajoute des conditions P3 et P4 (D50 sans ultraviolets), retire les conditions pour écrans, et prévoit des critères pour les éclairages à DEL. La plupart des chiffres publics trouvés datent de l'édition 2009.
- Conditions de l'édition 2009 : P1 (comparaison critique, environ 2 000 lux) et P2 (appréciation pratique, environ 500 lux). Chiffres usuels : P1 2 000 lux à plus ou moins 250 ; indice de rendu des couleurs (IRC ou CRI) de 90 ou plus ; indice de métamérie visible inférieur à 1,0 et UV inférieur à 1,5. [probable : sources secondaires, pas la norme]
- Le contrôle d'une cabine demande un spectroradiomètre ; myiro-libre n'en a pas besoin pour fonctionner, mais peut **enregistrer** quelle condition de vision est employée et la rattacher à la condition de mesure (M1 pour P1/P2, M2 pour P3/P4).
- La méthode de l'indice de métamérie est publiée par la CIE et l'ISO (ISO/CIE 23603:2024) : payante.

## 1. Ce qui est établi par les sources ouvertes

- L'ISO 3664:2025 est la quatrième édition, publiée en juillet 2025, et remplace l'ISO 3664:2009. [établi] (https://www.iso.org/standard/83759.html via catalogues : https://scc-ccn.ca/standardsdb/standards/8189341 ; https://www.austrian-standards.at/de/shop/iso-3664-2025-2025-07-11~p4323801)
- Sommaire du projet final public de l'édition 2025 [établi] (extrait gratuit : https://cdn.standards.iteh.ai/samples/83759/36bac9f08601457596dd3b40694864ba/ISO-FDIS-3664.pdf) :
  - conditions pour la **comparaison critique** : P1, P3 et T1 (transparents) ;
  - conditions pour l'**appréciation pratique** (contrôle courant) : P2 et P4 ;
  - T2 : petits transparents en projection ;
  - exigences générales : conditions ambiantes, appareillage de visualisation, spectre de l'illuminant de référence, entretien ;
  - essais : mesures spectrales, éclairement et luminance, résolution ;
  - annexe D (normative) : essai de conformité de la part d'ultraviolets pour P3 et P4 ; annexe B : validité des exigences pour une référence ; annexe E : comparaison simultanée entre plusieurs cabines.
- Changements de l'édition 2025 d'après plusieurs résumés publics : conditions P3 et P4 (D50 sans UV, correspondant à la condition de mesure M2 de l'ISO 13655), tolérances adaptées à l'éclairage moderne, indice de fidélité des couleurs pour les DEL (en recommandation, les exigences des fluorescents étant maintenues), suppression des conditions d'appréciation des images sur écran. [probable : résumés de revendeurs de normes et de fabricants d'éclairage, non vérifiés dans la norme] (https://standards.iteh.ai/catalog/standards/sist/4d875dea-0563-4bc1-a1e2-042c02ec4869/sist-iso-3664-2025 ; https://www.just-normlicht.com/en/norms-and-standards.html)
- Édition 2009 : couvre cinq grands types de conditions de vision dont P1 (comparaison critique d'un tirage) ; l'éclairement pour P1 est donné autour de 2 000 lux (une source dit 1 750 à 2 250 lux, avec 2 000 optimal ; une autre dit plus ou moins 500 : **contradiction, la bonne est probablement plus ou moins 250**) ; P2 vers 500 lux (plus ou moins 125, probable). [probable] (https://piworld.com/article/key-accurate-color-viewing ; https://www.piworld.com/the-key-to-accurate-color-viewing/all ; https://babelcolor.com/cta_iso3664-1.htm)
- Édition 2009, autres chiffres cités par deux synthèses : IRC (Ra) supérieur à 90 ; indice de métamérie visible inférieur à 1,0 ; indice de métamérie UV inférieur à 1,5 (contre 4 en 2000) ; entourage et fond neutres et mats (un fabricant emploie une peinture grise Munsell N8) ; au moins 1 200 lux (60 % de 2 000) en tout point de la surface d'observation, valeur à rapprocher de l'exigence d'uniformité (le taux exact d'uniformité n'est pas confirmé). [probable]
- Méthode des indices de métamérie : ISO/CIE 23603:2024, qui remplace CIE S 012:2004 et ISO 23603:2005 ; deux indices : domaine visible et domaine ultraviolet (fluorescence) ; publication payante de 19 pages. [établi] (https://cie.co.at/publications/standard-method-assessing-spectral-quality-daylight-simulators-visual-appraisal-and-1)
- Un éditeur de logiciel de mesure d'éclairage décrit ce qu'il évalue : uniformité sur 9 positions (grille 3 par 3), éclairement, température de couleur proche, chromaticité, CRI, indice de métamérie noté de A à E selon ISO 23603 / CIE S 012. [établi pour la description du produit] (https://babelcolor.com/cta_iso3664-1.htm)
- Spectre de D50 : défini par la CIE de 300 à 780 nm par pas de 5 nm. [établi pour la définition] (résumé de recherche d'un article public : https://piworld.com/article/key-accurate-color-viewing ; données CIE gratuites : https://cie.co.at)

## 2. Liens avec la mesure et le contrôle d'impression

- Mesurer en M1 (D50) pour juger un tirage regardé en P1 ou P2 ; mesurer en M2 (sans UV) pour un tirage regardé en P3 ou P4. [probable pour la correspondance P3/P4 et M2 ; voir `docs/references/icc-conditions-de-mesure.md`]
- Un consultant indépendant note que, même sous une lumière D50 conforme, les épreuves à jet d'encre et les tirages sur presse restent exposés à la métamérie et aux azurants : la cabine n'efface pas ces écarts. [établi, avis d'auteur] (https://www.color-source.net/en/Docs_Formation/2021_POINT_ABOUT_ISO_12647_STANDARDS.pdf)
- ArgyllCMS peut mesurer l'éclairage ambiant (`spotread -a`) et afficher température de couleur, CRI, TLCI et TM-30 (`-T`) si l'instrument sait mesurer une lumière ambiante ; ce n'est pas un contrôle de conformité ISO 3664. [établi pour les options] (https://www.argyllcms.com/doc/spotread.html)
- Les spectromètres de mire (MYIRO-1, FD-9) mesurent des surfaces éclairées par leur propre lampe : ils ne contrôlent pas la cabine. Le contrôle d'une cabine se fait avec un spectroradiomètre. [probable]

## 3. Ce qu'on en tire pour myiro-libre

- Ajouter dans la fiche d'un tirage ou d'un contrôle un champ « condition de vision » (P1, P2, P3, P4, autre, inconnue) et l'afficher sur le rapport, sans prétendre la vérifier.
- Un aide-mémoire pour l'imprimeur (page d'aide) : quoi vérifier dans sa cabine (lampe et son âge, entourage gris neutre mat, éclairement, pas de lumière parasite), en citant l'édition en vigueur.
- Ne pas coder de contrôle « conforme ISO 3664 » : exige la norme et un spectroradiomètre. Au mieux, un pense-bête.
- Surveiller la sortie des conditions P3 et P4 : elles relient la vision sans UV à la mesure M2, ce qui concerne directement le choix de condition du MYIRO-1.

## 4. Questions ouvertes

- Valeurs exactes de l'édition 2025 (éclairement, uniformité, IRC, indice de fidélité, limites de métamérie) : norme à acheter. [à vérifier]
- Confirmer pour l'édition 2009 : P1 plus ou moins 250 lux, P2 plus ou moins 125 lux, uniformité (60 % ou 75 % selon la condition ?), IRC 90 ou plus. [à vérifier]
- Les cabines vendues « 3664:2009 » restent-elles acceptées par Fogra ou les clients après 2025 ? [à vérifier]
- Le MYIRO-1 ou le FD-9 peuvent-ils mesurer un éclairage ambiant ? Aucune source. [à vérifier]
- Formule d'indice de métamérie de la CIE : publication payante ; existe-t-il une implémentation libre (par exemple dans colour-science) ? [à vérifier]

## Sources consultées

- https://www.iso.org/standard/83759.html (non joignable ; contenu connu par catalogues)
- https://scc-ccn.ca/standardsdb/standards/8189341
- https://www.austrian-standards.at/de/shop/iso-3664-2025-2025-07-11~p4323801
- https://cdn.standards.iteh.ai/samples/83759/36bac9f08601457596dd3b40694864ba/ISO-FDIS-3664.pdf
- https://standards.iteh.ai/catalog/standards/sist/4d875dea-0563-4bc1-a1e2-042c02ec4869/sist-iso-3664-2025
- https://www.just-normlicht.com/en/norms-and-standards.html
- https://piworld.com/article/key-accurate-color-viewing
- https://babelcolor.com/cta_iso3664-1.htm
- https://cie.co.at/publications/standard-method-assessing-spectral-quality-daylight-simulators-visual-appraisal-and-1
- https://www.color-source.net/en/Docs_Formation/2021_POINT_ABOUT_ISO_12647_STANDARDS.pdf
- https://www.argyllcms.com/doc/spotread.html
