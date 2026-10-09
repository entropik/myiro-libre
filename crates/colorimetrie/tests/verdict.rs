//! Verdict d'un écart ΔE00 au regard d'un seuil choisi par l'utilisateur :
//! conforme, proche de la limite (à partir de 80 % du seuil), hors tolérance
//! (au-delà du seuil). Un écart égal au seuil est encore dans la tolérance.

use colorimetrie::{Inconnu, Seuil, Verdict};

#[test]
fn un_seuil_est_un_nombre_fini_strictement_positif() {
    assert_eq!(Seuil::new(2.5).map(|s| s.valeur()), Some(2.5));
    for impossible in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Seuil::new(impossible).is_none(), "{impossible}");
    }
}

#[test]
fn trois_niveaux_autour_du_seuil() {
    let seuil = Seuil::new(2.5).unwrap();
    assert_eq!(seuil.verdict(0.0), Ok(Verdict::Conforme));
    assert_eq!(seuil.verdict(1.99), Ok(Verdict::Conforme));
    // 80 % de 2,5 = 2,0 : la limite approche.
    assert_eq!(seuil.verdict(2.0), Ok(Verdict::ProcheDeLaLimite));
    assert_eq!(seuil.verdict(2.5), Ok(Verdict::ProcheDeLaLimite));
    assert_eq!(seuil.verdict(2.51), Ok(Verdict::HorsTolerance));
}

#[test]
fn un_ecart_non_fini_n_a_pas_de_verdict() {
    let seuil = Seuil::new(1.0).unwrap();
    assert_eq!(seuil.verdict(f64::NAN), Err(Inconnu::ValeurNonFinie));
    assert_eq!(seuil.verdict(f64::INFINITY), Err(Inconnu::ValeurNonFinie));
}
