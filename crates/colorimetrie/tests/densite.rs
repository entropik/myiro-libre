//! Densités par réflexion (statuts E et T, visuelle ISO) depuis un spectre,
//! absolues ou relatives au papier.
//!
//! Les valeurs attendues des encres d'exemple viennent de la méthode
//! d'ArgyllCMS 3.2.0 (`xsp_density`, `xicc/xspect.c`) appliquée aux mêmes
//! spectres : c'est elle que `spotread -d` affiche.

use colorimetrie::{densites, densites_relatives, Densites, Inconnu, StatutDensite};

fn proche(obtenu: Densites, attendu: [f64; 4], tolerance: f64) {
    let obtenu_tableau = [obtenu.cyan, obtenu.magenta, obtenu.jaune, obtenu.visuel];
    for (o, a) in obtenu_tableau.iter().zip(attendu) {
        assert!(
            (o - a).abs() <= tolerance,
            "obtenu {obtenu_tableau:?}, attendu {attendu:?} ± {tolerance}"
        );
    }
}

/// Encre cyane d'exemple : réfléchit autour de 480 nm, absorbe le rouge.
const CYAN: [f64; 36] = [
    0.0842, 0.108, 0.143, 0.191, 0.2523, 0.3246, 0.4026, 0.4783, 0.5422, 0.5849, 0.6, 0.5849,
    0.5422, 0.4783, 0.4026, 0.3246, 0.2523, 0.191, 0.143, 0.108, 0.0842, 0.0691, 0.0601, 0.055,
    0.0524, 0.0511, 0.0504, 0.0502, 0.0501, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05,
];

/// Encre magenta d'exemple : absorbe autour de 540 nm.
const MAGENTA: [f64; 36] = [
    0.84, 0.84, 0.84, 0.84, 0.8399, 0.8396, 0.8385, 0.8349, 0.8253, 0.8026, 0.7557, 0.6723, 0.5457,
    0.3842, 0.217, 0.0885, 0.04, 0.0885, 0.217, 0.3842, 0.5457, 0.6723, 0.7557, 0.8026, 0.8253,
    0.8349, 0.8385, 0.8396, 0.8399, 0.84, 0.84, 0.84, 0.84, 0.84, 0.84, 0.84,
];

/// Encre jaune d'exemple : absorbe le bleu, sous 500 nm.
const JAUNE: [f64; 36] = [
    0.03, 0.03, 0.03, 0.0301, 0.0303, 0.0308, 0.0321, 0.0357, 0.0453, 0.0703, 0.1313, 0.2586,
    0.455, 0.6514, 0.7787, 0.8397, 0.8647, 0.8743, 0.8779, 0.8792, 0.8797, 0.8799, 0.88, 0.88,
    0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88, 0.88,
];

#[test]
fn le_diffuseur_parfait_a_une_densite_nulle() {
    for statut in [StatutDensite::E, StatutDensite::T] {
        proche(densites(&[1.0; 36], statut).unwrap(), [0.0; 4], 1e-12);
    }
}

#[test]
fn un_gris_qui_renvoie_un_dixieme_de_la_lumiere_a_une_densite_de_1() {
    // Par définition : densité = −log10(0,1) = 1, quel que soit le filtre.
    for statut in [StatutDensite::E, StatutDensite::T] {
        proche(densites(&[0.1; 36], statut).unwrap(), [1.0; 4], 1e-12);
        proche(densites(&[0.01; 36], statut).unwrap(), [2.0; 4], 1e-12);
    }
}

#[test]
fn les_encres_d_exemple_donnent_les_densites_d_argyllcms_en_statut_t() {
    let t = StatutDensite::T;
    proche(
        densites(&CYAN, t).unwrap(),
        [1.244746, 0.502194, 0.315106, 0.778666],
        2e-6,
    );
    proche(
        densites(&MAGENTA, t).unwrap(),
        [0.105708, 0.683781, 0.111393, 0.333270],
        2e-6,
    );
    proche(
        densites(&JAUNE, t).unwrap(),
        [0.055522, 0.101804, 0.931680, 0.076584],
        2e-6,
    );
}

#[test]
fn les_encres_d_exemple_donnent_les_densites_d_argyllcms_en_statut_e() {
    let e = StatutDensite::E;
    proche(
        densites(&CYAN, e).unwrap(),
        [1.244744, 0.502194, 0.385408, 0.778666],
        2e-6,
    );
    proche(
        densites(&MAGENTA, e).unwrap(),
        [0.105709, 0.683781, 0.081962, 0.333270],
        2e-6,
    );
    // Le statut E pèse le bleu plus étroitement : le jaune y est plus dense.
    proche(
        densites(&JAUNE, e).unwrap(),
        [0.055522, 0.101804, 1.355367, 0.076584],
        2e-6,
    );
}

#[test]
fn la_densite_relative_retire_celle_du_papier() {
    // Papier à 90 %, aplat à 9 % : 1 de densité au-dessus du papier.
    let relative = densites_relatives(&[0.09; 36], &[0.9; 36], StatutDensite::T).unwrap();
    proche(relative, [1.0; 4], 1e-12);
    // Le papier lui-même est à zéro.
    let papier = densites_relatives(&JAUNE, &JAUNE, StatutDensite::E).unwrap();
    proche(papier, [0.0; 4], 1e-12);
}

#[test]
fn un_spectre_absent_ou_invalide_donne_une_densite_inconnue() {
    let t = StatutDensite::T;
    assert_eq!(
        densites(&[], t),
        Err(Inconnu::LongueurSpectre {
            attendue: 36,
            recue: 0
        })
    );
    let mut abime = CYAN;
    abime[3] = f64::NAN;
    assert_eq!(densites(&abime, t), Err(Inconnu::ValeurNonFinie));
    assert_eq!(
        densites_relatives(&CYAN, &abime, t),
        Err(Inconnu::ValeurNonFinie)
    );
    assert_eq!(
        densites_relatives(&CYAN, &[0.5; 10], t),
        Err(Inconnu::LongueurSpectre {
            attendue: 36,
            recue: 10
        })
    );
}

#[test]
fn un_noir_total_a_une_densite_inconnue() {
    // Rien de renvoyé : la densité serait infinie. Pas de valeur plafonnée
    // inventée (ArgyllCMS rendrait 8) : c'est le seul écart avec lui.
    assert_eq!(
        densites(&[0.0; 36], StatutDensite::T),
        Err(Inconnu::ResultatNonFini)
    );
    let mut negatif = [0.0; 36];
    negatif[30] = -0.01;
    assert_eq!(
        densites(&negatif, StatutDensite::E),
        Err(Inconnu::ResultatNonFini)
    );
    // Un papier noir ne peut pas servir de référence.
    assert_eq!(
        densites_relatives(&CYAN, &[0.0; 36], StatutDensite::T),
        Err(Inconnu::ResultatNonFini)
    );
}

#[test]
fn le_facteur_pondere_est_borne_comme_dans_argyllcms() {
    // ArgyllCMS borne le facteur de réflexion pondéré dans [1e-8, 2] : une
    // plage presque noire plafonne à 8, une plage qui renvoie plus du double
    // de la lumière ne descend pas sous −log10(2).
    let plancher = -(2f64.log10());
    for statut in [StatutDensite::E, StatutDensite::T] {
        proche(densites(&[1e-9; 36], statut).unwrap(), [8.0; 4], 1e-12);
        proche(densites(&[1e-8; 36], statut).unwrap(), [8.0; 4], 1e-9);
        proche(densites(&[3.0; 36], statut).unwrap(), [plancher; 4], 1e-12);
        proche(densites(&[2.0; 36], statut).unwrap(), [plancher; 4], 1e-9);
    }
}
