//! Écarts de couleur entre deux Lab : ΔE00 (CIEDE2000), ΔC et ΔH (CIELAB).

use colorimetrie::{delta_c, delta_e00, delta_h, Inconnu, Lab};

fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

fn proche(obtenu: f64, attendu: f64, tolerance: f64) {
    assert!(
        (obtenu - attendu).abs() <= tolerance,
        "obtenu {obtenu}, attendu {attendu} ± {tolerance}"
    );
}

#[test]
fn delta_e00_de_la_premiere_paire_publiee_par_sharma() {
    // G. Sharma, W. Wu, E. N. Dalal, Color Research and Application 30(1),
    // 2005, tableau 1, paire 1 (citée dans docs/references/icc-delta-e.md).
    let e = delta_e00(lab(50.0, 2.6772, -79.7751), lab(50.0, 0.0, -82.7485)).unwrap();
    proche(e, 2.0425, 5e-5);
}

#[test]
fn deux_couleurs_identiques_ont_un_ecart_nul() {
    let c = lab(62.0, -12.5, 33.0);
    proche(delta_e00(c, c).unwrap(), 0.0, 1e-12);
}

#[test]
fn deux_gris_ne_different_que_par_la_clarte() {
    // Gris : S_L = 1 + 0,015 (L̄ − 50)² / √(20 + (L̄ − 50)²) ; L̄ = 50 → S_L = 1.
    proche(
        delta_e00(lab(49.0, 0.0, 0.0), lab(51.0, 0.0, 0.0)).unwrap(),
        2.0,
        1e-12,
    );
}

#[test]
fn delta_e00_est_symetrique_de_part_et_d_autre_de_180_degres() {
    // Teintes de 179° et 181° : le calcul de la teinte moyenne est le piège
    // classique ; l'écart ne dépend pas de l'ordre des couleurs.
    let a = lab(50.0, -20.0, 0.349);
    let b = lab(50.0, -20.0, -0.349);
    proche(delta_e00(a, b).unwrap(), delta_e00(b, a).unwrap(), 1e-12);
}

#[test]
fn un_lab_non_fini_donne_un_ecart_inconnu() {
    assert_eq!(
        delta_e00(lab(50.0, f64::NAN, 0.0), lab(50.0, 0.0, 0.0)),
        Err(Inconnu::ValeurNonFinie)
    );
    assert_eq!(
        delta_c(lab(50.0, 1.0, 0.0), lab(f64::INFINITY, 0.0, 0.0)),
        Err(Inconnu::ValeurNonFinie)
    );
    assert_eq!(
        delta_h(lab(50.0, 1.0, f64::NAN), lab(50.0, 0.0, 0.0)),
        Err(Inconnu::ValeurNonFinie)
    );
}

#[test]
fn delta_c_est_l_ecart_de_chroma_signe() {
    // C₁ = 5, C₂ = 10 : la seconde couleur est plus saturée de 5.
    proche(
        delta_c(lab(50.0, 3.0, 4.0), lab(50.0, 6.0, 8.0)).unwrap(),
        5.0,
        1e-12,
    );
    proche(
        delta_c(lab(50.0, 6.0, 8.0), lab(50.0, 3.0, 4.0)).unwrap(),
        -5.0,
        1e-12,
    );
}

#[test]
fn delta_h_est_nul_a_teinte_egale() {
    proche(
        delta_h(lab(50.0, 3.0, 4.0), lab(70.0, 6.0, 8.0)).unwrap(),
        0.0,
        1e-12,
    );
}

#[test]
fn delta_h_est_l_ecart_de_teinte_signe() {
    // Deux couleurs de chroma 10, à 0° et 90° : ΔE*ab = √200, ΔL = ΔC = 0,
    // donc |ΔH| = √200 ; positif quand la teinte tourne dans le sens direct.
    let rouge = lab(50.0, 10.0, 0.0);
    let jaune = lab(50.0, 0.0, 10.0);
    proche(delta_h(rouge, jaune).unwrap(), 200f64.sqrt(), 1e-12);
    proche(delta_h(jaune, rouge).unwrap(), -(200f64.sqrt()), 1e-12);
}

#[test]
fn delta_h_d_un_gris_vers_une_couleur_est_nul() {
    // Avec un gris, tout l'écart hors clarté est de la chroma :
    // ΔH² = ΔE*ab² − ΔL² − ΔC² = 0.
    proche(
        delta_h(lab(50.0, 0.0, 0.0), lab(50.0, 6.0, 8.0)).unwrap(),
        0.0,
        1e-12,
    );
}

#[test]
fn un_ecart_qui_deborde_est_inconnu_et_non_faux() {
    // Une chroma énorme fait déborder les calculs intermédiaires : le résultat
    // n'est pas un nombre, il doit être inconnu.
    let enorme = lab(50.0, 1e300, 1e300);
    let normal = lab(50.0, 10.0, 10.0);
    assert_eq!(delta_e00(enorme, normal), Err(Inconnu::ResultatNonFini));
    let extreme = lab(50.0, f64::MAX, f64::MAX);
    assert_eq!(delta_c(extreme, normal), Err(Inconnu::ResultatNonFini));
    assert_eq!(delta_h(extreme, extreme), Err(Inconnu::ResultatNonFini));
}
