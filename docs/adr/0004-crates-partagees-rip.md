# myiro-libre et le futur RIP sont deux projets qui partagent des crates

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

## Contrat de la crate `cgats` (9 octobre 2026)

- Interface publique : `ecrire` (mesure `myiro-libre/mesure/1` vers CGATS.17, version `myiro-libre/cgats/1`) et `lire` (CGATS.17 de myiro-libre ou d'un autre logiciel vers `MesureImportee`). Description : [`docs/formats/cgats.md`](../formats/cgats.md).
- Une donnée absente du fichier reste `Inconnue` ; une condition de mesure déduite de la source lumineuse du fabricant est `Supposee` ; un fichier sans condition de mesure reconnaissable est refusé. Une mesure écrite puis relue rend exactement ses spectres, ses Lab et sa provenance.
- Dépend seulement de `pont-protocole` ; contrôlée aussi sur le poste Linux de la CI. Changer le nom ou le sens d'un mot-clé `MYIRO_LIBRE_*` demande un nouveau numéro de format.
