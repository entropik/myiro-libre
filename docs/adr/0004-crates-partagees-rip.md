# myiro-libre et le futur RIP sont deux projets qui partagent des crates

> Remplacé par l'[ADR 0007](0007-poste-unique.md) pour le RIP séparé ; le contrat de `colorimetrie` reste valable.

Un RIP libre est en préparation. Il sera un projet distinct de myiro-libre (pilotes d'imprimante, trame et file d'impression sont un autre produit), mais réutilisera les crates de myiro-libre : `colorimetrie`, `mires` et les formats d'échange. myiro-libre reste l'outil de mesure ; il produit profils, linéarisations et conditions d'impression que le RIP consomme.

## Conséquences

- Ces crates restent indépendantes de l'application, de Tauri, des ponts et de Windows, et leur API publique est traitée comme un contrat.
- Les courbes de linéarisation ont un format maison ouvert et documenté (courbes, limite d'encre par canal, condition d'impression, mesure d'origine), exportable aussi en `.cal` ArgyllCMS.
- En attendant le RIP, la linéarisation ne s'applique pas aux mires de profilage générées : un profil fait sur courbes non appliquées en aval serait faux pour qui imprime sans RIP.

## Contrat de la crate `colorimetrie` (7 octobre 2026)

- Interface publique : spectre (380 à 730 nm par 10 nm, 36 valeurs) vers XYZ puis Lab, D50 et 2° ; Lab vers LCH ; écarts ΔE00 (CIEDE2000, k_L = k_C = k_H = 1), ΔC et ΔH (CIELAB, signés).
- Une valeur impossible à calculer rend `Err(Inconnu)` avec sa raison (spectre incomplet, valeur non finie, blanc invalide, teinte d'un gris, calcul qui déborde), jamais zéro. Un ΔH nul entre un gris et une couleur est un vrai zéro, pas une valeur inconnue.
- Tables : illuminant D50 et observateur 2° de la CIE (CC BY-SA 4.0), une ligne sur dix de 380 à 780 nm, dernière valeur du spectre prolongée au-delà de 730 nm ; Lab rapporté au blanc calculé avec ces mêmes tables (voir `docs/references/cie-tables-colorimetrie.md`). Sur les mesures MYIRO-1 archivées, l'écart à la DLL reste sous 0,02 en ΔE00.
- Changer de tables ou de méthode change tous les Lab : c'est une rupture du contrat, à décider dans un ADR.

### Complément du 9 octobre 2026 : couleur à l'écran (ticket #7)

Ajout seulement, rien ne change dans l'interface existante : `lab_vers_srgb` et `xyz_d50_vers_srgb` rendent une couleur sRGB 8 bits (`Srgb`, avec `hexadecimal()`) pour montrer une mesure à l'écran. Lab D50 rapporté au blanc D50 de Bradford (96,422 ; 100 ; 82,521), adaptation de Bradford D50 → D65, matrice et courbe de transfert sRGB. Une couleur hors gamut est bornée canal par canal et marquée `ramenee` ; une valeur non finie rend `Err(Inconnu)`. Sources et valeurs de contrôle : `docs/references/srgb-ecran.md`. Cette couleur sert à l'affichage seulement, jamais au calcul d'un écart.

### Complément du 9 octobre 2026 : verdict d'un écart (ticket #8)

Ajout seulement, rien ne change dans l'interface existante : `Seuil` (écart ΔE00 accepté, un nombre fini strictement positif ; `Seuil::new` rend `None` sinon) et `Seuil::verdict(ecart)`, qui rend `Verdict::Conforme`, `ProcheDeLaLimite` ou `HorsTolerance`, ou `Err(Inconnu::ValeurNonFinie)` pour un écart non fini.

- Hors tolérance : écart strictement au-dessus du seuil ; un écart égal au seuil reste dans la tolérance (une limite « maximum 3,0 » admet 3,0).
- Proche de la limite : à partir de 80 % du seuil (`PART_PROCHE_DE_LA_LIMITE`). C'est un choix du projet, repris du trait de limite de la jauge du système graphique, et non une valeur de norme : la question des seuils de « proche de la limite » reste ouverte (journal du 7 octobre 2026).
- Aucun seuil par défaut. La seule limite publique trouvée pour un ton direct (« de l'ordre de ΔE 2,5 », `docs/references/icc-12647-controle.md`) ne précise pas sa formule ; tant qu'elle n'est pas établie en ΔE00, l'utilisateur fixe lui-même l'écart accepté.
- Côté application (relecture du ticket #8) : le verdict juge exactement l'écart affiché, arrondi par la même fonction que l'écriture à deux décimales, et le seuil saisi passe par le même arrondi (« 1,115 » devient 1,11, valeur binaire 1,1149…). Deux spectres de conditions de mesure différentes (M0 contre M1) ne se comparent pas : l'écran montre l'avertissement, sans écart ni verdict, plutôt qu'un chiffre qui mélangerait la condition de mesure et l'impression.

## Contrat de la crate `cgats` (9 octobre 2026)

- Interface publique : `ecrire` (mesure `myiro-libre/mesure/1` vers CGATS.17, version `myiro-libre/cgats/1`) et `lire` (CGATS.17 de myiro-libre ou d'un autre logiciel vers `MesureImportee`). Description : [`docs/formats/cgats.md`](../formats/cgats.md).
- Une donnée absente du fichier reste `Inconnue` ; une condition de mesure déduite de la source lumineuse du fabricant est `Supposee` ; un fichier sans condition de mesure reconnaissable est refusé. Une mesure écrite puis relue rend exactement ses spectres, ses Lab et sa provenance.
- Dépend seulement de `pont-protocole` ; contrôlée aussi sur le poste Linux de la CI. Changer le nom ou le sens d'un mot-clé `MYIRO_LIBRE_*` demande un nouveau numéro de format.

### Complément du 28 octobre 2026 : mesures embarquées dans un profil (ticket #71)

Ajout seulement : `lire_sans_condition` lit un CGATS comme `lire`, mais un tableau sans condition de mesure écrite est accepté. Ses plages vont dans l'emplacement `m0`, sous une condition **inconnue**, jamais supposée. Il sert aux mesures qu'un profil embarque (balise `targ`) : ArgyllCMS y recopie le `.ti3` de myiro-libre, qui ne porte pas de condition, et les RVB et Lab suffisent pour juger le profil. `lire` reste strict : à l'import dans la bibliothèque, un fichier sans condition de mesure reconnaissable est toujours refusé.

## Contrat de la crate `gamut` (28 octobre 2026, ticket #71)

La crate `gamut` calcule tout ce qui sert à comparer des profils ICC RVB. Elle ne dépend que de `colorimetrie` et de `serde` (ni Tauri, ni Windows, ni ponts, ni `cgats`) et fait partie de la liste `CRATES` du poste Linux de la CI.

- **Interface publique** : `ProfilIcc` (`lire`, `version`, `description`, `grille`, `lab_absolu`, `blanc_support`, `mesures_embarquees`) et `ErreurIcc` ; `analyser` (`Analyse`, `Primaire`) et `chemin` ; `surface_cube`, `volume`, `maillage_affiche` (`Maillage`, `MaillageAffiche`) ; `comparer` (`Ecarts`, `EcartZone`, `PointEcart`, `Zone`, `Famille`) et `statistiques` (`Statistiques`) ; `fidelite`, `repetabilite`, `memes_mesures`, `points_du_chemin` (`Mesure`, `Repetabilite`) ; `lire_gam`. `profil_essai` (`ProfilEssai`) fabrique un profil synthétique pour les tests.
- **Entrée** : un profil de sortie RVB, PCS Lab ou XYZ, v2 ou v4, dont la table **A2B1** (colorimétrique relative) est de type `mft2` (lut16). Évaluation : courbes d'entrée, interpolation tétraédrique dans la table, courbes de sortie.
- **Colorimétrie absolue** : les valeurs relatives sont ramenées au blanc du support (`wtpt`), composante par composante en XYZ, puis passées en Lab rapporté à l'illuminant du PCS. Le blanc du papier est donc compris dans tous les chiffres.
- **Refus nommés** (`ErreurIcc::cle`, jamais de valeur devinée) : en-tête illisible, profil qui n'est pas RVB (un profil CMJN est dit « pas encore prévu »), table A2B1 absente, structure non lue (`mft1`, `mAB`… nommée), balise qui sort du fichier, blanc du support absent. Sans `wtpt`, on ne suppose jamais un blanc D50.
- **Grilles** : volume de la surface du cube RVB à 33 points par arête ; affichage à 17 points par arête (1 734 sommets, 3 072 triangles) ; écarts entre profils sur la grille RVB 33 × 33 × 33 (35 937 valeurs), puis sur les huit couleurs de base. Chaque carré de la surface est coupé par la même diagonale que le prototype du 26 octobre : sur une surface courbe, le volume dépend de la diagonale choisie, et ce choix retrouve les volumes de référence à 2 près.
- **Volume** : valeur absolue de Σ p0 · (p1 × p2) / 6 sur les triangles de la surface, en unités L\*a\*b\* au cube. Le volume dit l'étendue des couleurs d'un profil, jamais sa qualité.
- **Statistiques** : moyenne, médiane, p95, maximum et parts au-delà de 1 et de 2 ΔE00 ; centiles interpolés linéairement au rang p/100 · (n − 1), comme `numpy.percentile`. Une liste vide rend « rien », jamais 0. Deux profils sont dits identiques quand l'écart maximal reste sous 0,005.
- **Zones du cube RVB** (elles peuvent se recouvrir ; une zone vide n'est pas listée), avec mx, mn et ch = mx − mn des RVB de 0 à 1 : gris ch ≤ 0,1 ; ombres mx ≤ 0,35 ; clairs mn ≥ 0,75 ; saturée ch > 0,6 ; moyenne 0,1 < ch ≤ 0,6 et mx > 0,35. La famille (rouge, jaune, vert, cyan, bleu, magenta) vient de la teinte TSV par tranches de 60° centrées sur chaque couleur ; un gris parfait n'en a pas. Les zones sont triées par écart moyen décroissant ; les 10 points les plus forts sont rendus.
- **Fidélité, répétabilité, mêmes mesures** : la fidélité est le ΔE00 entre ce que le profil prédit et les Lab mesurés, plage par plage. La répétabilité compare toutes les paires de plages aux RVB strictement égaux, et vaut « rien » sans plage répétée. Deux jeux sont « les mêmes mesures » s'ils ont le même nombre de plages, les mêmes RVB à 0,01 % près une fois triés, et au plus 0,1 ΔE00 d'écart plage par plage.
- **Approché, et dit comme tel** : coupes à clarté constante, sections de la surface du cube RVB (33 points par arête) par le plan L\* = L + 0,5 pour L de 0 à 99, lues tous les 2° au milieu de chaque tranche de teinte ; chroma par teinte, maximum par tranche de 2° des coupes et de la grille RVB 33³ ; densité du noir tirée de Y (−log₁₀ Y), qui n'est pas une densité ISO status T ; couleurs d'écran des pastilles et de la vue 3D (`colorimetrie::lab_vers_srgb`), bornées et marquées quand elles sortent de l'écran.
- **Limite du gamut** : `lire_gam` lit le `.gam` écrit par `iccgamut` (table des sommets, puis des triangles) ; un indice de sommet faux est refusé. Le calcul lui-même est fait par ArgyllCMS, derrière le trait `MoteurGamut` de `profilage` (ADR 0002).
- Changer une de ces règles (grilles, diagonale, zones, centiles, seuils, colorimétrie absolue) change les chiffres affichés : c'est une rupture du contrat, à décider dans un ADR.

### Complément du 28 octobre 2026 : comparer une image et un profil (ticket #80)

Ajout seulement : rien ne change dans l'interface existante ni dans les chiffres de la comparaison de profils. La crate dépend toujours de `colorimetrie` et de `serde` seulement.

- **Ajouts à l'interface publique** : `ProfilIcc::lab_relatif`, `relatif_vers_absolu`, `reproduire` et `inverse` ; `ErreurIcc::SansTableInverse` ; `ProfilSource` (`lire`, `srvb`, `demande`, `nom`, `ecartees`, `cle_refus`) ; `analyser_pixels` (`AnalyseImage`, `PointNuage`, `Annulee`, `ECART_INCONNU`, `PAS_HISTOGRAMME`, `POINTS_NUAGE`) ; `familles_teinte` (`FamilleTeinte`, `FAMILLES_TEINTE`, `SECTEURS_TEINTE`) ; `profil_essai_xyz` pour les tests.
- **Table inverse** : la B2A1 du profil de sortie, de type `mft2` comme l'A2B1, PCS Lab ou XYZ. `lab_relatif` rend la couleur de la table A2B1 sans le blanc du support (blanc du papier à L\* = 100) ; `relatif_vers_absolu` la ramène au blanc du support, comme `lab_absolu`. `reproduire(lab)` rend A2B1(B2A1(lab)) en colorimétrique relatif : la **couleur reproduite**. L'A2B1 reste interpolée en tétraédrique ; la B2A1 l'est en **trilinéaire**, comme ArgyllCMS. Un profil sans B2A1 lisible se lit et se compare toujours : seul l'aller-retour est refusé (`SansTableInverse`), et l'analyse d'une image pour ce profil reste inconnue sans bloquer les autres.
- **Profil de l'image** (`ProfilSource`), lu dans l'ordre de la norme ICC : table A2B1, puis A2B0 (`mft2`), puis matrice et courbes (`rXYZ`…, `rTRC`… en `curv` ou `para` de types 0 à 4) ; gris par `kTRC`. Une table illisible est écartée avec sa raison (`ecartees`) et la suivante est essayée : rien n'est remplacé sans le dire. CMJN (`PasRvb`) et tout autre espace de données (`EnTete`) sont refusés ; `cle_refus` donne les textes propres au profil d'une image. Sans profil, `srvb()` : primaires sRVB adaptées D50 à 4 décimales, ramenées sur le D50 du PCS, courbe `para` de type 3. La **couleur demandée** (`demande`) est en Lab relatif D50.
- **Écarts d'une image** (`analyser_pixels`) : pour chaque couleur RVB distincte des pixels opaques, ΔE00 entre la couleur demandée et la couleur reproduite, reporté sur chaque pixel en code d'écart : dixièmes de ΔE00 arrondis vers le bas, bornés à 254, 255 (`ECART_INCONNU`) pour un pixel transparent ou à l'écart incalculable. Un pixel est au-delà du seuil si son code vaut au moins round(10 × seuil). Histogramme en 255 cases de 0,1 ΔE00, le même par famille de teinte, et nuage d'au plus 4 000 cellules de 4 unités Lab, placées sur le blanc du papier du profil de sortie. Les pixels transparents ne comptent pas ; les pixels à l'écart inconnu sont comptés à part (`pixels_inconnus`), jamais comme bons. L'analyse regarde la demande d'arrêt toutes les 65 536 couleurs distinctes.
- **Familles de teinte** : six familles (rouge, jaune, vert, cyan, bleu, magenta) ancrées sur la teinte h de la couleur demandée des couleurs sRVB pures, bornées à mi-chemin des ancrages voisins, plus les gris (C\* < 5). Ce choix du général (28 octobre 2026) remplace les douze secteurs de 30° du plan : le bleu sRVB, vers 306°, tombait entre deux secteurs. Elles ne se confondent pas avec les familles TSV des zones du cube RVB ci-dessus.
- **Confirmé** : l'aller-retour concorde avec `xicclu` d'ArgyllCMS 3.5.0 à 0,0001 ΔE00 au plus sur deux profils de l'atelier (B2A1 en trilinéaire ; 0,58 en tétraédrique) ; `srvb()` s'écarte du `sRGB.icm` d'ArgyllCMS de 0,0116 ΔE00 au plus ; le `DisplayP3.icm` d'ArgyllCMS est lu comme tel (rouge à C\* ≈ 136, contre 107 en sRVB). Ces essais sont des tests `#[ignore]` (`tests/inverse.rs`, `tests/source.rs`).
- **Supposé** : au PCS Lab, la courbe `kTRC` d'un profil gris donne L\* (de 0 à 1 pour 0 à 100). C'est notre lecture de l'annexe F.2 de la norme ICC, de mémoire, non vérifiée sur un profil gris réel.
- Changer le seuil de lecture, les cases, les familles, la cellule du nuage ou l'interpolation de la table inverse change les chiffres affichés : rupture du contrat, à décider dans un ADR.

### Complément du 30 octobre 2026 : pastilles hors gamut et pipette (ticket #80)

Ajout seulement : les chiffres déjà affichés ne changent pas.

- **Ajouts à l'interface publique** : `AnalyseImage::pastilles` (`Pastille`, `Paire`, `PASTILLES` = 48, `SEUIL_PASTILLES` = 0,5) et `paire_couleur`.
- **Pastilles** : les cellules du nuage (4 unités Lab de la couleur demandée) sont représentées chacune par sa couleur RVB la plus fréquente (à égalité, la plus petite valeur 0xRRVVBB). Une cellule a sa pastille si le code d'écart de ce représentant atteint `SEUIL_PASTILLES`, le plus petit seuil du curseur, par la règle des pixels. Au plus 48, les plus peuplées d'abord (pixels de toute la cellule). Une pastille porte la valeur RVB du représentant, les pixels de la cellule et sa paire.
- **Paire** : couleur demandée et couleur reproduite en L\*a\*b\* absolus sur le blanc du papier du profil de sortie (la base du nuage), leurs couleurs écran `#rrggbb` par la même conversion que le nuage, et l'écart ΔE00 avec son code. L'écart est calculé en Lab relatif, comme celui des pixels : il ne se recalcule pas sur les valeurs absolues.
- **Pipette** : `paire_couleur(rvb, source, sortie)` fait le même calcul pour une seule couleur ; `Ok(None)` si l'écart n'est pas calculable, la raison du profil sans table inverse sinon.
- **Supposé** : à l'écran, les deux pastilles sont approchées (conversion sRVB, écran non calibré) ; la page le dit. Changer la règle du représentant, le seuil minimal ou le plafond change ce qui est montré : rupture du contrat, à décider dans un ADR.

La crate `photo` (ouvrir une photo, ticket #80) rejoint aussi la liste `CRATES` du poste Linux de la CI : elle ne dépend que de `image` 0.25 (JPEG, PNG, TIFF), sans Windows ni Tauri. **Confirmé** par ses tests : le profil d'un TIFF se lit dans la balise 34675 ; il est lu **avant** de poser les limites de mémoire du décodeur, qui bornent ensuite la lecture des balises à la taille de l'image décodée, et une petite image perdrait un grand profil.
