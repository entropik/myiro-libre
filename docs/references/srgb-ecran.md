# Montrer une couleur mesurée à l'écran (Lab D50 vers sRGB)

Date : 9 octobre 2026 (ticket #7).

## Résumé

- L'écran montre la couleur d'une mesure dans un carré. Le calcul est dans la crate `colorimetrie` (`lab_vers_srgb`, `xyz_d50_vers_srgb`), jamais dans la page.
- Chaîne : Lab D50 → XYZ (blanc D50 : X = 96,422, Y = 100, Z = 82,521) → adaptation chromatique de Bradford D50 → D65 → sRGB linéaire → courbe de transfert sRGB → 8 bits [établi : formules publiques ci-dessous].
- Une couleur hors du gamut sRGB est ramenée dedans canal par canal (chaque composante bornée à 0 et 1) et signalée : l'écran écrit « couleur approchée à l'écran ». Un écart d'au plus un demi-pas de 8 bits n'est pas signalé. Le bornage par canal peut décaler la teinte des couleurs très saturées [établi pour le principe ; ampleur non mesurée].
- Le carré suppose un écran proche de sRGB. Sur un écran non étalonné, il reste indicatif : les valeurs Lab font foi.

## Sources

| Quoi | Source |
|---|---|
| Matrices de Bradford D50 ↔ D65, blanc D50 | Bruce Lindbloom, [Chromatic Adaptation](http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html) |
| Matrices sRGB ↔ XYZ (D65, et D50 adaptée par Bradford) | Bruce Lindbloom, [RGB/XYZ Matrices](http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html) |
| Courbe de transfert sRGB, conversions Lab D50 → sRGB | W3C, [CSS Color Module Level 4, « Sample code for color conversions »](https://www.w3.org/TR/css-color-4/#color-conversion-code) ; la norme IEC 61966-2-1 elle-même est payante et n'est pas reprise |

## Valeurs de contrôle (tests de `crates/colorimetrie/tests/srgb.rs`)

- Lab (100 ; 0 ; 0) et XYZ D50 (96,422 ; 100 ; 82,521) donnent le blanc (255, 255, 255), sans être signalés.
- Lab (0 ; 0 ; 0) donne le noir.
- Gris L* = 50, calculé à la main : Y = ((50 + 16) / 116)³ = 0,18419, puis 1,055 × 0,18419^(1/2,4) − 0,055 = 0,4664, soit 119 sur 255.
- Les trois colonnes de la matrice « sRGB, D50 » de Lindbloom (primaires sRGB adaptées à D50) redonnent rouge (255, 0, 0), vert (0, 255, 0) et bleu (0, 0, 255).
- Lab (50 ; 0 ; −120) et Lab (60 ; −100 ; 80) sont hors gamut : ramenés et signalés.
