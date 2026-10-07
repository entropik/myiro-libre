# Tables CIE pour passer d'un spectre à Lab (D50, 2°)

Date de recherche : 7 octobre 2026.

## Résumé

- La CIE publie gratuitement ses tables de référence sous forme de fichiers CSV, avec un DOI et une licence **CC BY-SA 4.0** [établi, lu dans les métadonnées de chaque fichier]. Cette licence est compatible dans un sens avec la GPL-3.0 (une œuvre CC BY-SA 4.0 peut entrer dans un projet GPL-3.0) : la crate `colorimetrie` peut donc contenir les valeurs, avec l'attribution.
- Deux tables suffisent : l'illuminant D50 et les fonctions colorimétriques de l'observateur 2° (1931). Les deux sont au pas de 1 nm ; on en garde une ligne sur dix, de 380 à 780 nm, sans autre modification.
- Notre calcul : somme sur 380 à 780 nm par pas de 10 nm de S_D50 × facteur de réflexion × fonction colorimétrique, normalisée pour que le blanc parfait ait Y = 100. Le spectre mesuré s'arrête à 730 nm : on prolonge sa dernière valeur jusqu'à 780 nm, comme le recommande la norme ASTM E308 pour les bords de spectre [établi pour le principe ; la norme elle-même est payante et n'est pas reprise].
- Résultat sur les mesures MYIRO-1 archivées (174 spectres M0, M1 et M2, 7 octobre 2026) : notre Lab rejoint celui de la DLL à ΔE00 0,007 en moyenne et 0,017 au plus. L'écart restant vient probablement de tables de pondération légèrement différentes dans la DLL (tables ASTM E308 au pas de 10 nm, par exemple) [supposé].

## Sources

| Table | DOI | Fichier | Origine |
|---|---|---|---|
| Illuminant normalisé D50 | [10.25039/CIE.DS.etgmuqt5](https://doi.org/10.25039/CIE.DS.etgmuqt5) | [CIE_std_illum_D50.csv](https://files.cie.co.at/Publications-datasets/CIE_std_illum_D50.csv) | ISO/CIE 11664-2:2022, tableau B.1 ; 300 à 830 nm par 1 nm |
| Fonctions colorimétriques 2° (1931) | [10.25039/CIE.DS.xvudnb9b](https://doi.org/10.25039/CIE.DS.xvudnb9b) | [CIE_xyz_1931_2deg.csv](https://files.cie.co.at/Publications-datasets/CIE_xyz_1931_2deg.csv) | 360 à 830 nm par 1 nm |

Pages de présentation : [D50](https://cie.co.at/datatable/cie-standard-illuminant-d50) et [observateur 2°](https://cie.co.at/datatable/cie-1931-colour-matching-functions-2-degree-observer). Licence : [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## Contrôles faits

- Blanc D50 obtenu avec nos tables : X = 96,39, Y = 100, Z = 82,45, contre 96,42 et 82,51 publiés par la CIE au pas de 1 nm (CIE 15:2018) ; l'écart vient du pas de 10 nm.
- Lab est calculé par rapport à ce blanc, obtenu avec les mêmes tables, pour qu'un blanc parfait donne exactement L = 100, a = b = 0.

## Questions ouvertes

- Tables exactes employées par la DLL du MYIRO-1 (pondération ASTM E308 ou CIE au pas de 10 nm) : non établies. L'écart mesuré (moins de 0,02 en ΔE00) ne justifie pas d'en chercher davantage pour l'instant.
- D'autres illuminants (D65, A) et l'observateur 10° pourront s'ajouter avec les tables CIE correspondantes, de même source et même licence.
