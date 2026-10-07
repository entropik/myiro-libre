# Contrôle d'impression : ISO 12647-7 (épreuve contractuelle) et ISO 12647-8 (épreuve de validation)

Date de recherche : 7 octobre 2026. Sources ouvertes et officielles seulement. Les normes ISO sont payantes : rien n'en est recopié ici, on ne cite que leur titre et leur numéro.

## Résumé

- L'ISO 12647-7 (3e édition, 2016) fixe comment vérifier une épreuve numérique contractuelle ; l'ISO 12647-8 fait la même chose pour l'épreuve de validation, avec des limites plus larges.
- Le contrôle se fait avec une petite bande de plages mesurées (la « Media Wedge » Ugra/Fogra V3, 72 plages) et, pour une vérification plus complète, avec la grande mire de 1 617 plages (ISO 12642-2).
- Depuis 2016, les écarts se disent en ΔE00 (CIEDE2000), plus ΔCh pour la balance des gris. Avant 2016 : ΔE*ab (ancienne formule) et ΔH.
- Les chiffres exacts de 2016 ne sont pas sur les sites officiels gratuits. Un document public d'imprimeur les recopie dans un tableau (voir section 3) : c'est la meilleure source gratuite trouvée, mais ce n'est pas la norme.
- Les notions « 95e centile » et « bord de gamut » (outer gamut) existent bien ; le chiffre « Outer 269 » n'a pas été retrouvé.
- Pour affirmer « conforme à l'ISO 12647-7 », il faudra acheter la norme. D'ici là, l'outil peut proposer des « préréglages de tolérances » modifiables, marqués « d'après sources publiques ».

## 1. Les normes et leurs titres

- ISO 12647-7:2016, « Graphic technology — Process control for the production of halftone colour separations, proof and production prints — Part 7: Proofing processes working directly from digital data ». Troisième édition, 15 novembre 2016. [établi] (page de présentation de la norme : https://standards.iteh.ai/catalog/standards/iso/1ddc1d52-3c97-4808-b6bf-532d65a7b2a2/iso-12647-7-2016 ; l'extrait gratuit de 12 pages donne le sommaire : exigences sur la mesure des écarts de couleur, papier d'épreuve, gamut, etc.)
- ISO 12647-8:2021, épreuve de validation (« validation print »). [établi] (https://iteh.es/catalog/standards/iso/166cb0e7-d4ea-4ccb-a2fd-862912810ee0/iso-12647-8-2021)
- ISO 12642-2 : mire de caractérisation de 1 617 plages (appelée aussi IT8.7/4 ou ECI2002). Le fichier de référence Fogra « FOGRA39L » contient ces 1 617 plages, « FOGRA39 » en contient 1 485. [établi] (https://registry.color.org/cmyk-registry/fogra39)
- ISO 13655 : conditions de mesure (voir `docs/references/icc-conditions-de-mesure.md`).

## 2. Les éléments de contrôle

- **Media Wedge Ugra/Fogra CMYK V3.0** : 72 plages en 3 rangées de 24 colonnes. [établi] (https://fogra.org/en/downloads/work-tools/characterisation-data : l'archive « MediaWedge subsets » pour FOGRA39 à FOGRA60 contient les valeurs de référence de ces 72 plages ; document d'un imprimeur : https://www.typos.cz/files/dateien/typos/GTC/Production%20Parameters%20and%20their%20Tolerances.pdf)
- La V3 est plus sensible dans les hautes lumières et les ombres que la V2. [établi] (https://shop.proof.de/uk/a3-proof-colour-accurate-colour-proof-digital-proof-online-proof.html)
- Une épreuve est dite contractuelle si sa bande est mesurée et reste dans les tolérances, et si une étiquette de travail (logiciel, encres, papier, condition simulée, profils, date) l'accompagne. [établi] (https://help.fiery.com/cws/FieryXF/7.0/en-us/GUID-C6B321AA-30EE-4D99-8542-CA6FC96AFB7C.html)
- La certification Fogra d'un système d'épreuve (« Contract Proof Creation ») examine : blanc du papier (brillant, teinte, fluorescence), exactitude sur la Media Wedge V3 et sur la mire ISO 12642-2, uniformité, reproduction des tons, repérage. Les conditions de mesure admises sont M0, M1 ou M2. [établi] (https://fogra.org/en/certification/prepress-technology/contract-proof-creation)
- Droits : les données de caractérisation Fogra (y compris les sous-ensembles de la Media Wedge) peuvent être redistribuées, même dans un logiciel, à condition de ne pas les modifier et de citer Fogra ; la désignation « FOGRAxx » sert seulement à identifier ces données ; l'usage n'est pas une certification. [établi] (https://fogra.org/en/downloads/work-tools/characterisation-data)

## 3. Les critères et leurs limites

### 3.1 Édition 2016 (ΔE00)

Tableau tiré d'un document qualité public d'un imprimeur, qui cite l'ISO 12647-7:2016 pour sa propre évaluation. [établi dans ce document ; non vérifié sur la norme elle-même]

| Plages | Limite 2016 |
|---|---|
| Plages « primaires » (aplats C, M, J, N) | ΔE00 maximum 3,0 ; ΔH maximum 2,5 |
| Toutes les autres plages du paragraphe 5.2 de la norme | ΔE00 maximum 5,0 ; ΔE00 moyen 2,5 |
| « Gris » : au moins 5 plages CMJ qui reproduisent les neutres, régulièrement espacées en clarté | ΔCh maximum 3,5 ; ΔCh moyen 2,0 |
| Blanc du papier d'épreuve | ΔE00 3,0 |

Source : https://www.typos.cz/files/dateien/typos/GTC/Production%20Parameters%20and%20their%20Tolerances.pdf

Points recoupés par d'autres pages publiques :

- Tous les écarts doivent être rapportés en CIEDE2000 ; les anciennes valeurs en ΔE*ab ne se convertissent pas directement, de nouvelles limites ont donc été fixées. [établi] (https://proofing.de/proof-de-offers-proofs-according-to-the-latest-tolerance-criteria-of-iso-12647-72016/)
- ΔCh est la distance réelle entre deux couleurs dans le plan a*b*, pour juger la dérive des gris. [établi] (même page)
- Pour les tons directs mesurés (PANTONE, HKS), la limite est de l'ordre de ΔE 2,5. [établi sur la page proof.de ci-dessus ; en ΔE00 d'après une synthèse de recherche : à vérifier]
- Sur la grande mire de 1 617 plages : moyenne ΔE00 ≤ 2,5 et 95e centile ΔE00 ≤ 5,0. [probable] (synthèse de recherche, non lue directement dans la norme ; recoupe la limite « autres plages » du tableau ci-dessus)

### 3.2 Édition précédente (ΔE*ab, avec ΔH)

Deux sources concordantes (le même imprimeur pour la colonne « 2007 », et l'aide en ligne d'un logiciel de RIP datant de 2012) donnent pour la Media Wedge : [établi]

- Moyenne sur toutes les plages : ΔE*ab 3,0 ; maximum 6,0 (5,0 pour les aplats primaires, avec ΔH max 2,5) ; blanc du papier 3,0 ; moyenne de ΔH sur les gris CMJ : 1,5.
- Épreuve de validation (ISO 12647-8, même ancienne édition) : même chose, mais maximum sur toutes les plages 8,0 au lieu de 6,0.
- Sur la mire de 1 617 plages, l'aide cite trois critères (moyenne générale, moyenne des plages « bord de gamut », maximum pour 95 % des plages) avec des valeurs de 4,0 à 6,0 : l'attribution exacte de chaque valeur n'est pas fiable dans la source (tableau mal extrait). [à vérifier]

Sources : https://help.fiery.com/cws/FieryXF/7.1_cws_6.5/en-us/GUID-9053A1DA-6042-47D5-B76F-19309911BBB9.html (liste des préréglages, sans chiffres) ; https://docs.esko.com/docs/en-us/flexproofe/12/otherdocs/FlexProofE_EskoVerification_12.pdf (chiffres).

Un consultant indépendant résume aussi l'évolution : les tolérances de la norme 12647-7 (en ΔE76 plus ΔH) étaient plus serrées que celles de l'ancienne ISO 12642 (par exemple 3 au lieu de 4 en moyenne), et il critique l'usage de ΔE76. [établi, avis d'auteur] (https://www.color-source.net/en/Docs_Formation/2021_POINT_ABOUT_ISO_12647_STANDARDS.pdf)

### 3.3 Épreuve de validation (ISO 12647-8)

- Qualitativement : tolérances plus larges que l'épreuve contractuelle, plus serrées qu'une simple évaluation de l'épreuve de conception. [établi] (https://fogra.org/en/certification/prepress-technology/validation-print-creation)
- Chiffres 2021 : non trouvés. Seules les valeurs de l'ancienne édition sont publiques (voir 3.2). [à vérifier, achat de la norme nécessaire]

### 3.4 « Outer 269 »

- La notion de plages « bord de gamut » (outer gamut) existe : l'annexe C de l'ISO 12647-7 y est consacrée selon un aperçu en vente ; l'aide du RIP la cite comme critère sur la mire de 1 617 plages. [probable]
- Le nombre 269 n'apparaît dans aucune source gratuite lue. [à vérifier]

## 4. Ce qu'on en tire pour myiro-libre

- Mettre dans l'outil un **préréglage de tolérances** : critères (primaires, autres plages, gris, blanc papier, 95e centile), limites en ΔE00/ΔCh, éditable, avec une mention de provenance (« d'après sources publiques, à confirmer sur la norme »).
- Ne jamais afficher « conforme ISO 12647-7 » sans avoir acheté et lu la norme ; afficher plutôt « résultat selon le préréglage X ».
- Utiliser les 72 plages de la Media Wedge V3 et le fichier FOGRA de référence tels que Fogra les publie (non modifiés, source citée), plutôt que de les recopier à la main.
- Le MYIRO-1 lit en bandes : la Media Wedge (3 rangées de 24 plages) s'y prête ; la mire de 1 617 plages demande plusieurs bandes. FD-9 : lecture de feuille entière (phase 7 du plan d'action).
- Un projet libre de validation existe déjà (voir `docs/references/icc-projets-libres.md`, ValiProof, GPL-3.0, 2009) : il sert de comparaison, pas de base (ancien ΔE76).

## 5. Questions ouvertes

- Valeurs exactes de la norme 2016 (tableau 2) : à confirmer par achat de la norme (environ 150 à 260 francs suisses d'après les revendeurs).
- Les ΔH du tableau 2016 sont-ils en ΔH*ab ou ΔH00 ? Le document source écrit seulement « H ». [à vérifier]
- Que couvre exactement le « 95e centile » : toutes les plages de la mire de 1 617, ou seulement un sous-ensemble ? [à vérifier]
- Combien de plages dans le sous-ensemble « outer gamut » ? [à vérifier]
- Condition de mesure imposée pour chaque jeu FOGRA (M0 pour FOGRA39, M1 pour FOGRA51 et FOGRA52 selon les pages ICC) : la page FOGRA39 de l'ICC ne la précise pas. [à vérifier]

## Sources consultées

- https://standards.iteh.ai/catalog/standards/iso/1ddc1d52-3c97-4808-b6bf-532d65a7b2a2/iso-12647-7-2016 (sommaire public de l'ISO 12647-7:2016)
- https://iteh.es/catalog/standards/iso/166cb0e7-d4ea-4ccb-a2fd-862912810ee0/iso-12647-8-2021
- https://fogra.org/en/certification/prepress-technology/contract-proof-creation
- https://fogra.org/en/certification/prepress-technology/validation-print-creation
- https://fogra.org/en/downloads/work-tools/characterisation-data
- https://registry.color.org/cmyk-registry/fogra39
- https://registry.color.org/cmyk-registry/fogra51
- https://www.typos.cz/files/dateien/typos/GTC/Production%20Parameters%20and%20their%20Tolerances.pdf
- https://proofing.de/proof-de-offers-proofs-according-to-the-latest-tolerance-criteria-of-iso-12647-72016/
- https://shop.proof.de/uk/a3-proof-colour-accurate-colour-proof-digital-proof-online-proof.html
- https://help.fiery.com/cws/FieryXF/7.1_cws_6.5/en-us/GUID-9053A1DA-6042-47D5-B76F-19309911BBB9.html
- https://help.fiery.com/cws/FieryXF/7.0/en-us/GUID-C6B321AA-30EE-4D99-8542-CA6FC96AFB7C.html
- https://docs.esko.com/docs/en-us/flexproofe/12/otherdocs/FlexProofE_EskoVerification_12.pdf
- https://www.color-source.net/en/Docs_Formation/2021_POINT_ABOUT_ISO_12647_STANDARDS.pdf
- https://valiproof.sourceforge.net/documentation.html
