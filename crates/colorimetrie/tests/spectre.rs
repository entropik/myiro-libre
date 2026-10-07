//! Spectre de réflexion (380 à 730 nm par 10 nm) vers XYZ et Lab, D50 et 2°.

use colorimetrie::{
    spectre_vers_lab, spectre_vers_xyz, xyz_vers_lab, Inconnu, Xyz, LONGUEUR_SPECTRE,
};

fn proche(obtenu: f64, attendu: f64, tolerance: f64) {
    assert!(
        (obtenu - attendu).abs() <= tolerance,
        "obtenu {obtenu}, attendu {attendu} ± {tolerance}"
    );
}

#[test]
fn le_diffuseur_parfait_donne_le_blanc_de_reference() {
    let lab = spectre_vers_lab(&[1.0; LONGUEUR_SPECTRE]).unwrap();
    proche(lab.l, 100.0, 1e-9);
    proche(lab.a, 0.0, 1e-9);
    proche(lab.b, 0.0, 1e-9);
}

#[test]
fn le_blanc_d50_est_celui_publie_par_la_cie() {
    // Blanc D50, 2° : X = 96,42, Y = 100, Z = 82,51 (CIE 15:2018, tableau T.3).
    // Nos tables au pas de 10 nm s'en écartent de moins de 0,1.
    let blanc = spectre_vers_xyz(&[1.0; LONGUEUR_SPECTRE]).unwrap();
    proche(blanc.x, 96.42, 0.1);
    proche(blanc.y, 100.0, 1e-9);
    proche(blanc.z, 82.51, 0.1);
}

#[test]
fn un_gris_neutre_a_18_pour_cent_donne_l_49_5() {
    // L* = 116 × 0,18^(1/3) − 16 = 49,496.
    let lab = spectre_vers_lab(&[0.18; LONGUEUR_SPECTRE]).unwrap();
    proche(lab.l, 49.496, 1e-3);
    proche(lab.a, 0.0, 1e-9);
    proche(lab.b, 0.0, 1e-9);
}

#[test]
fn un_noir_tres_profond_passe_par_la_partie_lineaire() {
    // Y/Yn = 0,005 ≤ 216/24389 : L* = 24389/27 × 0,005 = 4,516.
    let lab = spectre_vers_lab(&[0.005; LONGUEUR_SPECTRE]).unwrap();
    proche(lab.l, 4.5164, 1e-3);
}

#[test]
fn un_spectre_incomplet_donne_un_lab_inconnu() {
    assert_eq!(
        spectre_vers_lab(&[0.5; 31]),
        Err(Inconnu::LongueurSpectre {
            attendue: 36,
            recue: 31
        })
    );
    assert!(spectre_vers_xyz(&[]).is_err());
}

#[test]
fn une_valeur_non_finie_donne_un_lab_inconnu() {
    let mut spectre = [0.5; LONGUEUR_SPECTRE];
    spectre[12] = f64::NAN;
    assert_eq!(spectre_vers_lab(&spectre), Err(Inconnu::ValeurNonFinie));
    spectre[12] = f64::INFINITY;
    assert_eq!(spectre_vers_xyz(&spectre), Err(Inconnu::ValeurNonFinie));
}

#[test]
fn un_blanc_de_reference_nul_donne_un_lab_inconnu() {
    let xyz = Xyz {
        x: 40.0,
        y: 50.0,
        z: 30.0,
    };
    let blanc = Xyz {
        x: 96.42,
        y: 0.0,
        z: 82.51,
    };
    assert_eq!(xyz_vers_lab(xyz, blanc), Err(Inconnu::BlancInvalide));
}

#[test]
fn un_spectre_qui_deborde_donne_un_lab_inconnu() {
    assert_eq!(
        spectre_vers_xyz(&[f64::MAX; LONGUEUR_SPECTRE]),
        Err(Inconnu::ResultatNonFini)
    );
    let blanc_infime = Xyz {
        x: 1e-300,
        y: 1e-300,
        z: 1e-300,
    };
    let xyz = Xyz {
        x: 1e300,
        y: 1e300,
        z: 1e300,
    };
    assert_eq!(
        xyz_vers_lab(xyz, blanc_infime),
        Err(Inconnu::ResultatNonFini)
    );
}
