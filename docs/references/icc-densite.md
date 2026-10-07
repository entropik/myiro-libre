# Densité : statuts E, T, A, M, I (ISO 5-3, ISO 5-4), absolu ou relatif au papier, sec ou humide

Date de recherche : 7 octobre 2026. Normes citées par titre seulement : ISO 5-3:2009 (conditions spectrales) et ISO 5-4:2009 (conditions géométriques pour la densité par réflexion), payantes.

## Résumé

- La densité d'un aplat mesure combien il absorbe la lumière : plus il est dense, plus il est sombre. Le « statut » (T, E, A, M, I) dit quelle partie du spectre on pèse pour chaque encre.
- Statut T : le plus courant aux États-Unis. Statut E : le plus courant en Europe. Statut I : variante aussi proposée par les instruments. Statut A : photo sur papier ; statut M : négatifs.
- Une densité dépend aussi de la polarisation, du filtre UV, du fond et du choix « avec ou sans soustraction du papier ». Deux densitomètres peuvent donc afficher des valeurs différentes pour la même encre sans être en panne.
- Les tables de pondération officielles sont dans la norme payante. Des tables libres existent dans des bibliothèques sous licence BSD, et ArgyllCMS (outil spotread, option `-d`) calcule déjà plusieurs densités : on peut s'y comparer.
- Pour un MYIRO-1 ou un FD-9 qui fournissent des spectres, la densité se calcule à partir du spectre ; il faut donc garder les spectres, et noter condition de mesure et fond.
- Les critères de l'ISO 12647-7 (épreuve) lus dans les sources ouvertes sont colorimétriques (ΔE00, ΔCh), pas des densités ; la densité sert surtout au suivi de presse (ISO 12647-2) et du gain de point. [probable]

## 1. Ce qui est établi par les sources ouvertes

- L'ISO 5-3:2009 fixe les conditions spectrales et la méthode de calcul de plusieurs types de densités ISO 5 en photographie et en arts graphiques ; l'ISO 5-4:2009 fixe les conditions géométriques de la densité par réflexion. [établi] (https://scc-ccn.ca/standardsdb/standards/8147114 ; https://iteh.es/catalog/standards/iso/6cdfb60e-5093-43ae-b7a4-7102bd9170bd/iso-5-3-2009)
- Aux États-Unis, la densité des arts graphiques se mesure le plus souvent en statut T ; en Europe, on préfère le statut E ; le statut I est aussi proposé. [établi] (https://xrite.com/service-support/my_density_readings_are_not_what_i_expected)
- Statut A : référence pour les tirages photographiques en couleur ; statut M : référence pour les supports intermédiaires (négatifs couleur). [établi, selon une synthèse de recherche d'un article public : à relire dans la source]
- Les causes d'écart entre deux densitomètres : norme de statut, filtres UV ou polarisation présents ou non, soustraction du papier ou non, valeur de référence (« zéro ») saisie par erreur. Le réglage conseillé pour des valeurs comparables : mode automatique, valeurs absolues (sans soustraction du papier) et norme de sa région. [établi] (même page X-Rite)
- Sec contre humide : une encre fraîche paraît plus dense qu'une encre sèche, parce que sa surface est lisse et réfléchit moins la lumière diffuse. Un filtre polarisant réduit cet écart sur papier couché ; il est courant en Europe, rare aux États-Unis. [établi] (https://www.xrite.com/service-support/Using_a_Polarization_Filter ; l'effet est décrit aussi dans des brevets publics, par exemple https://patents.google.com/patent/US4575249)
- Pour l'ISO 13655, M0 est la condition « compatible avec l'ISO 5-3 » (éclairage de type illuminant A) et M3 (polarisation) sert à prévoir la densité sèche à partir de la mesure humide. [établi] (extrait gratuit de l'ISO 13655:2009 et article du fabricant, voir `docs/references/icc-conditions-de-mesure.md`)

## 2. Les formules (niveau de détail public)

- Densité d'une plage = moins le logarithme décimal d'un facteur de réflexion « pesé » : on multiplie le spectre mesuré par la réponse spectrale du statut choisi (une pour le rouge, le vert, le bleu et le visuel), on somme sur les longueurs d'onde, on divise par la somme obtenue pour un blanc parfait, puis on prend moins le logarithme. [probable] (formulation générale de la densitométrie ; la forme exacte, le choix de l'éclairage dans la pondération et la normalisation sont dans l'ISO 5-3, payante)
- Densité relative au papier : on mesure le papier avec le même filtre, puis on calcule la densité de l'aplat par rapport à ce papier (le blanc devient 0). Densité absolue : la référence est le blanc parfait. [établi pour l'existence des deux modes (page X-Rite) ; formule exacte : à vérifier]
- Gain de point (Murray-Davies) à partir de densités : usage courant, hors de cette note. [à vérifier si besoin]
- La bibliothèque Python python-colormath (licence BSD-3-Clause, dépôt archivé le 12 décembre 2023) contient des tables de pondération de 50 valeurs pour les statuts A, E, M, T, plus « Type 1 », « Type 2 » et un visuel ISO. [établi] (https://raw.githubusercontent.com/gtaylor/python-colormath/master/colormath/density_standards.py ; https://github.com/gtaylor/python-colormath) L'origine exacte de ces tables et leur conformité à l'ISO 5-3 ne sont pas documentées dans le code. [à vérifier]
- ArgyllCMS : `spotread -d` affiche les densités suivantes : ISO Visuel, Type 1, Type 2, statuts A, M, T et E en cyan, magenta, jaune et visuel. [établi] (https://www.argyllcms.com/doc/spotread.html) Le code source (AGPL) montre sa méthode ; non lu ici.

## 3. Ce qu'on en tire pour myiro-libre

- Calculer la densité à partir des spectres (phase 6 du plan d'action) avec des tables de pondération provenant d'une source libre dont la licence est claire, et les valider contre `spotread -d` sur les mêmes spectres.
- Afficher toujours : statut (T ou E), absolue ou relative au papier, condition de mesure (M0, M1, M2), et « mesure sèche » ou « humide ».
- Ne pas promettre une densité « sèche » avec un MYIRO-1 : pas de polarisation (M3), voir `docs/references/icc-conditions-de-mesure.md`.
- Pour l'imprimeur : une densité du MYIRO-1 n'est pas comparable telle quelle à celle d'un vieux densitomètre à filtres ; l'écrire dans l'interface et dans l'aide.

## 4. Questions ouvertes

- Tables officielles de l'ISO 5-3 (statuts T et E) : existe-t-il une source libre fiable (publication de Fogra, ECI, ICC) ? Non trouvée. [à vérifier]
- Les tables de python-colormath sont-elles celles de l'ISO 5-3 et sous quelles hypothèses (éclairage de la pondération) ? [à vérifier]
- Comment ArgyllCMS fait-il le calcul (références, normalisation) ? Lire son code source. [à vérifier]
- Quelle densité faut-il afficher pour une mesure en M1 (éclairage D50) : la norme de densité suppose M0. [à vérifier]
- Exigences éventuelles de l'ISO 12647-2 (offset) et de G7 sur le statut de densité : hors de la présente recherche.

## Sources consultées

- https://scc-ccn.ca/standardsdb/standards/8147114
- https://iteh.es/catalog/standards/iso/6cdfb60e-5093-43ae-b7a4-7102bd9170bd/iso-5-3-2009
- https://xrite.com/service-support/my_density_readings_are_not_what_i_expected
- https://www.xrite.com/service-support/Using_a_Polarization_Filter
- https://patents.google.com/patent/US4575249
- https://github.com/gtaylor/python-colormath
- https://raw.githubusercontent.com/gtaylor/python-colormath/master/colormath/density_standards.py
- https://www.argyllcms.com/doc/spotread.html
- https://cdn.standards.iteh.ai/samples/39877/fc7f9f9e2f954e279c475d6c603fd8c1/ISO-13655-2009.pdf
- https://sensing.konicaminolta.us/us/?p=10979
