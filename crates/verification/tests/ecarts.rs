//! Écarts de fidélité et de prédiction plage par plage, couleurs hors gamut
//! et statistiques. Le blanc du support des essais est le blanc D50
//! parfait : la mesure relative y est la mesure elle-même.

use colorimetrie::Lab;
use verification::{
    comparer, reclasser, statistiques, vers_relatif, EcartPlage, PlageAVerifier, RefusVerification,
    ValeursProfil,
};

fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

const BLANC: Lab = Lab {
    l: 100.0,
    a: 0.0,
    b: 0.0,
};

/// Vingt couleurs régulières, dans le gamut de tout papier.
fn couleurs() -> Vec<Lab> {
    (0..20)
        .map(|i| {
            let t = f64::from(i);
            lab(30.0 + 2.5 * t, 20.0 - 2.0 * t, -10.0 + t)
        })
        .collect()
}

fn plages(mesures: &[Lab]) -> Vec<PlageAVerifier> {
    mesures
        .iter()
        .enumerate()
        .map(|(i, m)| PlageAVerifier {
            id: (i + 1).to_string(),
            rvb: [0.5, 0.5, 0.5],
            mesure: *m,
        })
        .collect()
}

/// Un profil qui prédit exactement les couleurs demandées.
fn parfait(couleurs: &[Lab]) -> ValeursProfil {
    ValeursProfil {
        demande: Some(couleurs.to_vec()),
        appareil: vec![[0.5, 0.5, 0.5]; couleurs.len()],
        prediction_rel: couleurs.to_vec(),
        prediction_abs: couleurs.to_vec(),
        blanc_support: BLANC,
    }
}

fn proche(obtenu: f64, attendu: f64, tolerance: f64) {
    assert!(
        (obtenu - attendu).abs() <= tolerance,
        "obtenu {obtenu}, attendu {attendu} ± {tolerance}"
    );
}

#[test]
fn le_blanc_du_support_devient_le_blanc_relatif() {
    let support = lab(94.2, 1.3, -4.8);
    let relatif = vers_relatif(support, support).unwrap();
    proche(relatif.l, 100.0, 1e-6);
    proche(relatif.a, 0.0, 1e-6);
    proche(relatif.b, 0.0, 1e-6);
}

#[test]
fn une_mesure_plus_sombre_que_le_support_reste_plus_sombre() {
    let relatif = vers_relatif(lab(50.0, 0.0, 0.0), lab(94.0, 0.0, 0.0)).unwrap();
    assert!(relatif.l > 50.0 && relatif.l < 100.0, "{relatif:?}");
}

#[test]
fn le_95e_centile_est_au_rang_le_plus_proche() {
    let e: Vec<_> = (1..=20)
        .map(|i| EcartPlage {
            id: i.to_string(),
            de00: f64::from(i),
        })
        .collect();
    let s = statistiques(&e).unwrap();
    assert_eq!((s.moyenne, s.p95, s.max, s.plages), (10.5, 19.0, 20.0, 20));
    assert_eq!(s.pires.len(), 10);
    assert_eq!(s.pires[0].id, "20");
    assert_eq!(s.pires[9].id, "11");
}

#[test]
fn une_liste_vide_n_a_pas_de_statistiques() {
    assert_eq!(statistiques(&[]), None);
}

#[test]
fn moins_de_dix_plages_sont_toutes_parmi_les_pires() {
    let e: Vec<_> = [0.4, 2.0, 1.0]
        .iter()
        .enumerate()
        .map(|(i, &d)| EcartPlage {
            id: i.to_string(),
            de00: d,
        })
        .collect();
    let s = statistiques(&e).unwrap();
    let ids: Vec<_> = s.pires.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["1", "2", "0"]);
    // rang ceil(0,95 × 3) = 3 : le plus grand.
    assert_eq!(s.p95, 2.0);
}

#[test]
fn un_profil_parfait_donne_des_ecarts_nuls() {
    let c = couleurs();
    let r = comparer(&plages(&c), &parfait(&c), 1.0).unwrap();
    let fidelite = r.fidelite.expect("fidélité");
    let prediction = r.prediction.expect("prédiction");
    assert_eq!(fidelite.plages, 20);
    proche(fidelite.max, 0.0, 1e-9);
    proche(prediction.max, 0.0, 1e-9);
    assert_eq!(r.hors_gamut, None);
    assert_eq!(r.seuil_gamut, 1.0);
    let p = &r.plages[3];
    assert_eq!(p.id, "4");
    assert_eq!(p.demande, Some(c[3]));
    assert_eq!(p.prediction, c[3]);
    assert_eq!(p.mesure, c[3]);
    proche(p.mesure_rel.l, c[3].l, 1e-9);
}

#[test]
fn une_plage_aberrante_est_la_pire_et_seule_au_dessus() {
    let c = couleurs();
    let mut mesures: Vec<Lab> = c.iter().map(|m| lab(m.l + 0.5, m.a, m.b)).collect();
    mesures[7] = lab(c[7].l, c[7].a + 12.0, c[7].b - 6.0);
    let r = comparer(&plages(&mesures), &parfait(&c), 1.0).unwrap();
    let fidelite = r.fidelite.unwrap();
    assert_eq!(fidelite.pires[0].id, "8");
    assert!(fidelite.pires[0].de00 > 5.0, "{:?}", fidelite.pires[0]);
    assert!(fidelite.pires[1].de00 < 1.0, "{:?}", fidelite.pires[1]);
    assert_eq!(fidelite.max, fidelite.pires[0].de00);
    // Une seule plage sur vingt : le 95e centile (rang 19) reste bas.
    assert!(fidelite.p95 < 1.0, "{}", fidelite.p95);
    let prediction = r.prediction.unwrap();
    assert_eq!(prediction.pires[0].id, "8");
}

#[test]
fn une_derive_globale_monte_moyenne_et_centile() {
    let c = couleurs();
    let mesures: Vec<Lab> = c.iter().map(|m| lab(m.l - 3.0, m.a, m.b)).collect();
    let r = comparer(&plages(&mesures), &parfait(&c), 1.0).unwrap();
    for s in [r.fidelite.unwrap(), r.prediction.unwrap()] {
        assert!(s.moyenne > 2.0, "{s:?}");
        assert!(s.p95 > 2.0, "{s:?}");
        assert!(s.max - s.moyenne < 1.0, "écarts réguliers : {s:?}");
    }
}

#[test]
fn fidelite_et_prediction_peuvent_diverger() {
    let c = couleurs();
    // Le profil prédit bien ce qu'il imprime, mais ne peut pas atteindre les
    // couleurs demandées : toutes sont à 4 unités de L de ce qu'il prédit.
    let demande: Vec<Lab> = c.iter().map(|m| lab(m.l + 4.0, m.a, m.b)).collect();
    let valeurs = ValeursProfil {
        demande: Some(demande),
        ..parfait(&c)
    };
    let r = comparer(&plages(&c), &valeurs, 1.0).unwrap();
    assert!(r.prediction.unwrap().moyenne < 0.01);
    let hors = r.hors_gamut.expect("plages hors gamut");
    assert_eq!(hors.plages, 20);
    assert!(hors.moyenne > 3.0, "{hors:?}");
    assert_eq!(r.fidelite, None, "aucune plage dans le gamut");
    assert!(r.plages.iter().all(|p| p.ecart_gamut.unwrap() > 1.0));
}

#[test]
fn sans_demande_seule_la_prediction_est_calculee() {
    let c = couleurs();
    let valeurs = ValeursProfil {
        demande: None,
        ..parfait(&c)
    };
    let r = comparer(&plages(&c), &valeurs, 1.0).unwrap();
    assert_eq!(r.fidelite, None);
    assert_eq!(r.hors_gamut, None);
    assert!(r.prediction.is_some());
    assert!(r
        .plages
        .iter()
        .all(|p| p.demande.is_none() && p.ecart_fidelite.is_none() && p.ecart_gamut.is_none()));
}

#[test]
fn reclasser_avec_un_autre_seuil_garde_les_ecarts() {
    let c = couleurs();
    // Écart au gamut croissant : 0, 0,2, 0,4… unités de L.
    let demande: Vec<Lab> = c
        .iter()
        .enumerate()
        .map(|(i, m)| lab(m.l + 0.2 * i as f64, m.a, m.b))
        .collect();
    let valeurs = ValeursProfil {
        demande: Some(demande),
        ..parfait(&c)
    };
    let r = comparer(&plages(&c), &valeurs, 1.0).unwrap();
    let r2 = reclasser(&r, 3.0);
    assert_eq!(r2.seuil_gamut, 3.0);
    assert_eq!(r2.plages, r.plages);
    let dans = |x: &verification::Resultats| x.fidelite.as_ref().map_or(0, |s| s.plages);
    let hors = |x: &verification::Resultats| x.hors_gamut.as_ref().map_or(0, |s| s.plages);
    assert_eq!(dans(&r) + hors(&r), 20);
    assert_eq!(dans(&r2) + hors(&r2), 20);
    assert!(dans(&r2) > dans(&r));
    assert_eq!(r2.prediction, r.prediction);
}

#[test]
fn des_tailles_differentes_sont_refusees() {
    let c = couleurs();
    let mut valeurs = parfait(&c);
    valeurs.prediction_abs.pop();
    assert_eq!(
        comparer(&plages(&c), &valeurs, 1.0),
        Err(RefusVerification::TaillesDifferentes)
    );
    assert_eq!(
        RefusVerification::TaillesDifferentes.cle(),
        "verif.refus.tailles"
    );
    assert_eq!(
        comparer(&[], &parfait(&[]), 1.0),
        Err(RefusVerification::Vide)
    );
    assert_eq!(RefusVerification::Vide.cle(), "verif.refus.vide");
}

#[test]
fn une_valeur_non_finie_nomme_la_plage() {
    let c = couleurs();
    let mut mesures = c.clone();
    mesures[4].a = f64::NAN;
    let refus = comparer(&plages(&mesures), &parfait(&c), 1.0).unwrap_err();
    assert_eq!(
        refus,
        RefusVerification::ValeurNonFinie {
            plage: "5".to_string()
        }
    );
    assert_eq!(refus.cle(), "verif.refus.valeur");
}

#[test]
fn les_resultats_se_relisent_a_l_identique_en_json() {
    let c = couleurs();
    let r = comparer(&plages(&c), &parfait(&c), 1.0).unwrap();
    let texte = serde_json::to_string(&r).unwrap();
    let relu: verification::Resultats = serde_json::from_str(&texte).unwrap();
    assert_eq!(relu, r);
}
