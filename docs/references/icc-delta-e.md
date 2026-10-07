# Écart de couleur : CIEDE2000, ΔH et ΔCh, validation des calculs

Date de recherche : 7 octobre 2026.

## Résumé

- CIEDE2000 (ΔE00) est la formule d'écart de couleur imposée depuis 2016 par l'ISO 12647-7 (voir `docs/references/icc-12647-controle.md`). Elle corrige des défauts de l'ancien ΔE*ab (ΔE76).
- La formule est publique et décrite en détail ; le piège n'est pas la formule mais son implémentation : de nombreuses versions anciennes contenaient des erreurs de moyenne des angles de teinte.
- Gaurav Sharma et ses coauteurs ont publié en 2005 un article (Color Research and Application, volume 30, numéro 1) avec 34 paires de couleurs et leurs résultats : c'est le jeu de test que tout le monde utilise. La première paire donne ΔE00 = 2,0425.
- ΔH (écart de teinte) et ΔCh (distance dans le plan a*b* pour juger les gris) sont deux notions différentes de ΔE00 ; elles sont utilisées dans l'ISO 12647-7.
- Plusieurs implémentations libres existent (Little CMS, colour-science, caisses Rust `empfindung` et `deltae`, ArgyllCMS) : les comparer sur le jeu de Sharma, et sur des paires proches de 180 degrés de teinte, valide notre code.
- Le jeu de Sharma est offert pour la recherche, à citer ; sa réutilisation dans un dépôt GPL est à confirmer (voir questions ouvertes).

## 1. La formule et ses pièges

- Structure : ΔE00 = racine de la somme de trois termes (clarté, chroma, teinte) corrigés chacun par une fonction de pondération (S_L, S_C, S_H) et facteurs de réglage k_L, k_C, k_H (égaux à 1 par défaut), plus un terme croisé de rotation R_T qui sert surtout dans le bleu (autour de 275 degrés de teinte). [établi] (https://en.wikipedia.org/wiki/Color_difference ; référence : https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/)
- Article de référence : G. Sharma, W. Wu, E. N. Dalal, « The CIEDE2000 Color-Difference Formula: Implementation Notes, Supplementary Test Data, and Mathematical Observations », Color Research and Application, février 2005. La page de l'université propose aussi une feuille Excel et un programme Matlab, et précise que ce n'est pas une implémentation officielle de la CIE. [établi] (page ci-dessus ; article : https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/ciede2000noteCRNA.pdf)
- Pièges d'implémentation relevés dans l'article (points 1, 2, 5 et 6 lus dans le texte : [établi] ; points 3 et 4 : [probable], formulations courantes à relire dans l'article) :
  1. Moyenne des teintes (h-barre) : quand les deux teintes sont à plus de 180 degrés l'une de l'autre, la moyenne simple est fausse ; il faut ajouter ou retrancher 360 degrés selon les cas.
  2. Différence de teinte Δh' : même traitement, et valeur nulle si l'une des chromas est nulle.
  3. Teinte d'une couleur sans chroma : indéfinie, on la prend nulle.
  4. Utiliser l'arc-tangente à deux arguments (atan2), puis ramener l'angle dans 0 à 360 degrés.
  5. La différence de chroma est signée ; ΔH' est calculé avec le signe.
  6. La formule est discontinue : une très petite variation de couleur peut faire sauter la moyenne de teinte de 180 degrés, ce qui crée un petit saut du résultat (inférieur à 4 % d'après Wikipédia). Ce n'est pas un bogue de l'implémentation.
- Jeu de test : 34 paires, résultats à 4 décimales ; les paires 7 à 16 vérifient l'arc-tangente, les paires 25 à 34 sont des cas « naturels » de la CIE. La paire 1 (L=50, a=2,6772, b=-79,7751 contre L=50, a=0, b=-82,7485) donne 2,0425 ; la paire 2 donne 2,8615 ; la paire 3 donne 3,4412. [établi, lu dans le PDF de l'article ; à contrôler avec le tableau complet avant d'écrire les tests]
- Norme : la formule fait l'objet d'une norme ISO/CIE payante (ISO/CIE 11664-6) ; l'annexe B (informative) de l'ISO 13655:2009 la décrit aussi. [probable pour le numéro ; établi pour l'annexe B (sommaire public de l'extrait gratuit)]

## 2. ΔH et ΔCh

- ΔH*ab (CIELAB) : la part de l'écart qui n'est ni de la clarté ni du chroma : racine de (ΔE*ab au carré moins ΔL au carré moins ΔC au carré). [établi] (https://en.wikipedia.org/wiki/Color_difference)
- ΔCh (ISO 12647-7:2016) : décrit comme la distance réelle entre deux couleurs dans le plan a*b* ; sert à juger les gris CMJ (maximum 3,5 et moyenne 2,0 dans le tableau d'un document public). [établi pour la description (https://proofing.de/proof-de-offers-proofs-according-to-the-latest-tolerance-criteria-of-iso-12647-72016/) ; la formule exacte de la norme : à vérifier]
- ΔH de l'ancienne édition de l'ISO 12647-7 : écart de teinte en ΔH*ab ; les « H » du tableau 2016 sont de nature non confirmée. [à vérifier]

## 3. Implémentations libres à comparer

| Projet | Licence | État | Remarque |
|---|---|---|---|
| Little CMS (https://github.com/mm2/Little-CMS) | MIT | actif (dernier envoi le 5 octobre 2026) | fonction `cmsCIE2000DeltaE(Lab1, Lab2, Kl, Kc, Kh)`, plus CIE94, CMC, BFD [établi, en-tête lu] |
| colour-science (https://github.com/colour-science/colour) | BSD-3-Clause | actif (6 octobre 2026) | `delta_E_CIE2000`, option textile, références Sharma et Melgosa 2013, renvoie aussi dL, dC, dH [établi] (https://colour.readthedocs.io/en/develop/generated/colour.difference.delta_E_CIE2000.html) |
| caisse Rust `empfindung` (https://github.com/mina86/empfindung) | MIT | version 0.2.6 d'août 2026 | CIEDE2000, CIE94, CIE76, CMC ; les tests sur le jeu de Sharma ne sont pas mentionnés dans la documentation [établi] |
| caisse Rust `deltae` (https://gitlab.com/ryanobeirne/deltae) | MIT | version 0.3.2 d'août 2026 | DE2000, DE1976, DE1994, CMC [établi] |
| python-colormath (https://github.com/gtaylor/python-colormath) | BSD-3-Clause | archivé en décembre 2023 | à éviter comme référence ; calcul délégué à un autre module [établi] |
| ArgyllCMS `profcheck` | AGPL-3.0 | actif | `-k` : écarts en CIEDE2000 ; `-c` : CIE94 ; par défaut CIE76 [établi] (https://www.argyllcms.com/doc/profcheck.html) |

## 4. Ce qu'on en tire pour myiro-libre

- Écrire notre propre ΔE00 en Rust (court, sans dépendance) avec des tests : les 34 paires de Sharma, des paires à cheval sur la limite de 180 degrés, des gris (chroma nulle) et des noirs.
- Comparer à Little CMS et à colour-science sur des milliers de paires aléatoires (tolérance de l'ordre de 1e-6) ; si une caisse Rust passe tous les tests, on peut l'employer à la place.
- Montrer à l'utilisateur ΔE00 par défaut, avec ΔE*ab en option pour les anciens contrôles ; toujours dire quelle formule est employée (la même mesure donne des chiffres très différents).
- Pour le contrôle ISO : ΔE00 (plages), ΔH (primaires) et ΔCh (gris) sont des critères séparés ; ne pas les confondre dans le code ni dans l'interface.

## 5. Questions ouvertes

- Droit de reprendre le jeu de test de Sharma dans les tests du dépôt : **tranché le 7 octobre 2026, on ne le reprend pas.** La page de publication autorise l'usage personnel et de recherche avec citation, mais ne donne aucun droit de redistribution. Le fichier `ciede2000testdata.txt` se télécharge à la main depuis la page de l'auteur et se pose dans `crates/colorimetrie/tests/donnees-locales/` (ignoré par git) ; le test `sharma.rs`, ignoré par défaut, le lit avec `cargo test -p colorimetrie -- --ignored`. Le dépôt ne garde que la paire 1, citée ci-dessus. Résultat : les 34 paires sont reproduites à 0,00005 près.
- Définition exacte de ΔCh de la norme 2016 (valeur signée ? en CIELAB ou en LCh00 ?). [à vérifier]
- Quels paramètres k_L, k_C, k_H sont employés pour le contrôle d'épreuve (supposé 1, 1, 1) ? [à vérifier]
- Les caisses Rust sont-elles testées contre Sharma ? À vérifier dans leur code de tests.

## Sources consultées

- https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/
- https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/ciede2000noteCRNA.pdf
- https://en.wikipedia.org/wiki/Color_difference
- https://colour.readthedocs.io/en/develop/generated/colour.difference.delta_E_CIE2000.html
- https://github.com/mm2/Little-CMS et https://raw.githubusercontent.com/mm2/Little-CMS/master/include/lcms2.h
- https://docs.rs/empfindung/latest/empfindung/
- https://docs.rs/deltae/latest/deltae/
- https://github.com/gtaylor/python-colormath et https://raw.githubusercontent.com/gtaylor/python-colormath/master/colormath/color_diff.py
- https://www.argyllcms.com/doc/profcheck.html
- https://proofing.de/proof-de-offers-proofs-according-to-the-latest-tolerance-criteria-of-iso-12647-72016/
- https://cdn.standards.iteh.ai/samples/39877/fc7f9f9e2f954e279c475d6c603fd8c1/ISO-13655-2009.pdf
