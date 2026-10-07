//! Lab vers LCH : clarté, chroma et teinte (degrés, de 0 à 360).

use colorimetrie::{lab_vers_lch, Inconnu, Lab};

fn proche(obtenu: f64, attendu: f64) {
    assert!(
        (obtenu - attendu).abs() <= 1e-9,
        "obtenu {obtenu}, attendu {attendu}"
    );
}

fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

#[test]
fn chroma_et_teinte_d_un_jaune() {
    let lch = lab_vers_lch(lab(80.0, 0.0, 10.0)).unwrap();
    proche(lch.l, 80.0);
    proche(lch.c, 10.0);
    proche(lch.h.unwrap(), 90.0);
}

#[test]
fn la_teinte_reste_entre_0_et_360_degres() {
    proche(lab_vers_lch(lab(50.0, 3.0, 4.0)).unwrap().c, 5.0);
    proche(
        lab_vers_lch(lab(50.0, -10.0, 0.0)).unwrap().h.unwrap(),
        180.0,
    );
    proche(
        lab_vers_lch(lab(50.0, 0.0, -10.0)).unwrap().h.unwrap(),
        270.0,
    );
    proche(
        lab_vers_lch(lab(50.0, 10.0, -10.0)).unwrap().h.unwrap(),
        315.0,
    );
}

#[test]
fn un_gris_a_une_teinte_inconnue_et_non_nulle() {
    let lch = lab_vers_lch(lab(50.0, 0.0, 0.0)).unwrap();
    proche(lch.c, 0.0);
    assert_eq!(lch.h, Err(Inconnu::TeinteSansChroma));
}

#[test]
fn un_lab_non_fini_donne_un_lch_inconnu() {
    assert_eq!(
        lab_vers_lch(lab(f64::NAN, 1.0, 1.0)),
        Err(Inconnu::ValeurNonFinie)
    );
}

#[test]
fn une_teinte_a_peine_sous_0_degre_reste_sous_360() {
    // atan2 rend un angle infime négatif ; ramené dans [0, 360), il ne doit
    // pas valoir 360.
    let h = lab_vers_lch(lab(50.0, 10.0, -1e-17)).unwrap().h.unwrap();
    assert!((0.0..360.0).contains(&h), "teinte {h}");
}

#[test]
fn une_chroma_qui_deborde_donne_un_lch_inconnu() {
    assert_eq!(
        lab_vers_lch(lab(50.0, f64::MAX, f64::MAX)),
        Err(Inconnu::ResultatNonFini)
    );
}
