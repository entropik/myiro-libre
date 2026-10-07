# myiro-libre et le futur RIP sont deux projets qui partagent des crates

Un RIP libre est en préparation. Il sera un projet distinct de myiro-libre (pilotes d'imprimante, trame et file d'impression sont un autre produit), mais réutilisera les crates de myiro-libre : `colorimetrie`, `mires` et les formats d'échange. myiro-libre reste l'outil de mesure ; il produit profils, linéarisations et conditions d'impression que le RIP consomme.

## Conséquences

- Ces crates restent indépendantes de l'application, de Tauri, des ponts et de Windows, et leur API publique est traitée comme un contrat.
- Les courbes de linéarisation ont un format maison ouvert et documenté (courbes, limite d'encre par canal, condition d'impression, mesure d'origine), exportable aussi en `.cal` ArgyllCMS.
- En attendant le RIP, la linéarisation ne s'applique pas aux mires de profilage générées : un profil fait sur courbes non appliquées en aval serait faux pour qui imprime sans RIP.

## Contrat de la crate `colorimetrie` (7 octobre 2026)

- Interface publique : spectre (380 à 730 nm par 10 nm, 36 valeurs) vers XYZ puis Lab, D50 et 2° ; Lab vers LCH ; écarts ΔE00 (CIEDE2000, k_L = k_C = k_H = 1), ΔC et ΔH (CIELAB, signés).
- Une valeur impossible à calculer rend `Err(Inconnu)` avec sa raison (spectre incomplet, valeur non finie, blanc invalide, teinte d'un gris), jamais zéro. Un ΔH nul entre un gris et une couleur est un vrai zéro, pas une valeur inconnue.
- Tables : illuminant D50 et observateur 2° de la CIE (CC BY-SA 4.0), une ligne sur dix de 380 à 780 nm, dernière valeur du spectre prolongée au-delà de 730 nm ; Lab rapporté au blanc calculé avec ces mêmes tables (voir `docs/references/cie-tables-colorimetrie.md`). Sur les mesures MYIRO-1 archivées, l'écart à la DLL reste sous 0,02 en ΔE00.
- Changer de tables ou de méthode change tous les Lab : c'est une rupture du contrat, à décider dans un ADR.
