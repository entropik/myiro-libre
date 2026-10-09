//! Couleur à l'écran : Lab D50 vers sRGB 8 bits (adaptation de Bradford
//! D50 → D65), sur des valeurs de référence publiques citées dans
//! `docs/references/srgb-ecran.md`.

use colorimetrie::{lab_vers_srgb, xyz_d50_vers_srgb, Inconnu, Lab, Srgb, Xyz};

fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

fn rvb(s: Srgb) -> [u8; 3] {
    [s.r, s.g, s.b]
}

/// Le blanc de référence D50 devient le blanc de l'écran (D65), sans être
/// « ramené ».
#[test]
fn le_blanc_d50_est_le_blanc_de_l_ecran() {
    let blanc = lab_vers_srgb(lab(100.0, 0.0, 0.0)).unwrap();
    assert_eq!(rvb(blanc), [255, 255, 255]);
    assert!(!blanc.ramenee);

    // Blanc D50 publié (Lindbloom, « Chromatic Adaptation ») : 96,422 ; 100 ; 82,521.
    let blanc = xyz_d50_vers_srgb(Xyz {
        x: 96.422,
        y: 100.0,
        z: 82.521,
    })
    .unwrap();
    assert_eq!(rvb(blanc), [255, 255, 255]);
}

#[test]
fn le_noir_reste_noir() {
    let noir = lab_vers_srgb(lab(0.0, 0.0, 0.0)).unwrap();
    assert_eq!(rvb(noir), [0, 0, 0]);
    assert!(!noir.ramenee);
}

/// Exemple calculé à la main avec les formules de l'IEC 61966-2-1 :
/// L* = 50 donne Y = ((50 + 16) / 116)³ = 0,18419, puis
/// 1,055 × 0,18419^(1/2,4) − 0,055 = 0,4664, soit 118,9 sur 255.
#[test]
fn un_gris_neutre_l50_donne_119() {
    let gris = lab_vers_srgb(lab(50.0, 0.0, 0.0)).unwrap();
    assert_eq!(rvb(gris), [119, 119, 119]);
    assert!(!gris.ramenee);
}

/// Primaires sRGB adaptées à D50 par Bradford : colonnes de la matrice
/// « sRGB, D50 » publiée par Lindbloom (« RGB/XYZ Matrices »), × 100.
#[test]
fn les_primaires_srgb_adaptees_a_d50_redonnent_rouge_vert_bleu() {
    let primaires = [
        (
            Xyz {
                x: 43.60747,
                y: 22.25045,
                z: 1.39322,
            },
            [255, 0, 0],
        ),
        (
            Xyz {
                x: 38.50649,
                y: 71.68786,
                z: 9.71045,
            },
            [0, 255, 0],
        ),
        (
            Xyz {
                x: 14.30804,
                y: 6.06169,
                z: 71.41733,
            },
            [0, 0, 255],
        ),
    ];
    for (xyz, attendu) in primaires {
        let couleur = xyz_d50_vers_srgb(xyz).unwrap();
        assert_eq!(rvb(couleur), attendu, "{xyz:?}");
        assert!(!couleur.ramenee, "{xyz:?}");
    }
}

/// Un bleu très saturé n'existe pas en sRGB : il est ramené dans le gamut,
/// et le résultat le dit.
#[test]
fn une_couleur_hors_gamut_est_ramenee_et_signalee() {
    let bleu = lab_vers_srgb(lab(50.0, 0.0, -120.0)).unwrap();
    assert!(bleu.ramenee);
    assert!(bleu.b > bleu.r && bleu.b > bleu.g, "{bleu:?}");

    let vert = lab_vers_srgb(lab(60.0, -100.0, 80.0)).unwrap();
    assert!(vert.ramenee);
}

#[test]
fn une_valeur_non_finie_donne_une_couleur_inconnue() {
    assert_eq!(
        lab_vers_srgb(lab(f64::NAN, 0.0, 0.0)),
        Err(Inconnu::ValeurNonFinie)
    );
    assert_eq!(
        xyz_d50_vers_srgb(Xyz {
            x: f64::INFINITY,
            y: 1.0,
            z: 1.0
        }),
        Err(Inconnu::ValeurNonFinie)
    );
}

#[test]
fn la_couleur_s_ecrit_en_hexadecimal_pour_l_ecran() {
    let gris = lab_vers_srgb(lab(50.0, 0.0, 0.0)).unwrap();
    assert_eq!(gris.hexadecimal(), "#777777");
}
