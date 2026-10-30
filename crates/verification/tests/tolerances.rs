//! Tolérances d'un jeu, contrôles un par un et conclusion. Règle : un seuil
//! dépassé donne « hors tolérance » ; sinon un critère requis non calculable
//! donne « incomplet » ; sinon « accepté ».

use colorimetrie::Lab;
use verification::{
    comparer, conclure, valider_criteres, Conclusion, Controle, Critere, Criteres, Famille,
    PlageAVerifier, RefusCriteres, Resultats, Seuils, ValeursProfil,
};

fn lab(l: f64, a: f64, b: f64) -> Lab {
    Lab { l, a, b }
}

const BLANC: Lab = Lab {
    l: 100.0,
    a: 0.0,
    b: 0.0,
};

fn couleurs() -> Vec<Lab> {
    (0..20)
        .map(|i| {
            let t = f64::from(i);
            lab(30.0 + 2.5 * t, 20.0 - 2.0 * t, -10.0 + t)
        })
        .collect()
}

/// Résultats d'une mire mesurée `mesures`, profil qui prédit `couleurs()`
/// et demande `demande`.
fn resultats(mesures: &[Lab], demande: Option<Vec<Lab>>) -> Resultats {
    let c = couleurs();
    let plages: Vec<_> = mesures
        .iter()
        .enumerate()
        .map(|(i, m)| PlageAVerifier {
            id: (i + 1).to_string(),
            rvb: [0.5; 3],
            mesure: *m,
        })
        .collect();
    let valeurs = ValeursProfil {
        demande,
        appareil: vec![[0.5; 3]; c.len()],
        prediction_rel: c.clone(),
        prediction_abs: c,
        blanc_support: BLANC,
    };
    comparer(&plages, &valeurs, 1.0).unwrap()
}

/// Valeurs de départ du jeu « Atelier » (décision du mainteneur du
/// 28 octobre 2026).
fn atelier() -> Criteres {
    Criteres {
        fidelite: Seuils {
            moyenne: Some(2.0),
            p95: Some(4.0),
            max: Some(6.0),
        },
        prediction: Seuils {
            moyenne: Some(1.5),
            p95: Some(3.0),
            max: Some(5.0),
        },
        seuil_gamut: 1.0,
    }
}

#[test]
fn tous_les_seuils_respectes_donnent_accepte() {
    let c = couleurs();
    let mesures: Vec<_> = c.iter().map(|m| lab(m.l + 0.5, m.a, m.b)).collect();
    let (conclusion, controles) = conclure(&resultats(&mesures, Some(c)), &atelier());
    assert_eq!(conclusion, Conclusion::Accepte);
    assert_eq!(controles.len(), 6);
    assert!(controles.iter().all(|c| c.respecte == Some(true)));
    let ordre: Vec<_> = controles.iter().map(|c| (c.famille, c.critere)).collect();
    assert_eq!(
        ordre,
        [
            (Famille::Fidelite, Critere::Moyenne),
            (Famille::Fidelite, Critere::P95),
            (Famille::Fidelite, Critere::Max),
            (Famille::Prediction, Critere::Moyenne),
            (Famille::Prediction, Critere::P95),
            (Famille::Prediction, Critere::Max),
        ]
    );
    assert_eq!(controles[2].limite, 6.0);
}

#[test]
fn un_maximum_depasse_donne_hors_tolerance_et_nomme_le_critere() {
    let c = couleurs();
    let mut mesures = c.clone();
    mesures[3] = lab(c[3].l + 9.0, c[3].a + 6.0, c[3].b);
    let criteres = Criteres {
        fidelite: Seuils::default(),
        ..atelier()
    };
    let (conclusion, controles) = conclure(&resultats(&mesures, Some(c)), &criteres);
    assert_eq!(conclusion, Conclusion::HorsTolerance);
    let depasses: Vec<&Controle> = controles
        .iter()
        .filter(|c| c.respecte == Some(false))
        .collect();
    assert_eq!(depasses.len(), 1);
    assert_eq!(
        (depasses[0].famille, depasses[0].critere),
        (Famille::Prediction, Critere::Max)
    );
    assert!(depasses[0].valeur.unwrap() > 5.0);
}

#[test]
fn sans_demande_une_fidelite_requise_rend_incomplet() {
    let c = couleurs();
    let (conclusion, controles) = conclure(&resultats(&c, None), &atelier());
    assert_eq!(conclusion, Conclusion::Incomplet);
    let fidelite: Vec<_> = controles
        .iter()
        .filter(|c| c.famille == Famille::Fidelite)
        .collect();
    assert_eq!(fidelite.len(), 3);
    assert!(fidelite
        .iter()
        .all(|c| c.valeur.is_none() && c.respecte.is_none()));
}

/// Mire très saturée sur papier mat : toutes les couleurs demandées sont
/// hors gamut. La fidélité dans le gamut n'a aucune plage : inconnue, la
/// vérification est incomplète, jamais acceptée.
#[test]
fn sans_plage_dans_le_gamut_une_fidelite_requise_rend_incomplet() {
    let c = couleurs();
    let demande: Vec<_> = c
        .iter()
        .map(|m| lab(m.l, m.a * 3.0, m.b * 3.0 + 15.0))
        .collect();
    let r = resultats(&c, Some(demande));
    assert_eq!(r.fidelite, None);
    assert_eq!(r.hors_gamut.as_ref().map(|s| s.plages), Some(20));
    let (conclusion, _) = conclure(&r, &atelier());
    assert_eq!(conclusion, Conclusion::Incomplet);
}

#[test]
fn un_depassement_l_emporte_sur_un_critere_incalculable() {
    let c = couleurs();
    let mesures: Vec<_> = c.iter().map(|m| lab(m.l - 6.0, m.a, m.b)).collect();
    let (conclusion, controles) = conclure(&resultats(&mesures, None), &atelier());
    assert_eq!(conclusion, Conclusion::HorsTolerance);
    assert!(controles.iter().any(|c| c.respecte.is_none()));
}

#[test]
fn une_famille_sans_seuil_n_est_pas_requise() {
    let c = couleurs();
    let criteres = Criteres {
        fidelite: Seuils::default(),
        ..atelier()
    };
    let (conclusion, controles) = conclure(&resultats(&c, None), &criteres);
    assert_eq!(conclusion, Conclusion::Accepte);
    assert!(controles.iter().all(|c| c.famille == Famille::Prediction));
}

#[test]
fn un_jeu_sans_aucun_seuil_est_refuse() {
    let vide = Criteres {
        fidelite: Seuils::default(),
        prediction: Seuils::default(),
        seuil_gamut: 1.0,
    };
    assert_eq!(valider_criteres(&vide), Err(RefusCriteres::AucunSeuil));
    assert_eq!(
        RefusCriteres::AucunSeuil.cle(),
        "verif.tolerances.aucun_seuil"
    );
    assert_eq!(valider_criteres(&atelier()), Ok(()));
}

#[test]
fn un_seuil_nul_ou_negatif_est_refuse() {
    for mauvais in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut c = atelier();
        c.prediction.p95 = Some(mauvais);
        assert_eq!(valider_criteres(&c), Err(RefusCriteres::SeuilInvalide));
        let mut c = atelier();
        c.seuil_gamut = mauvais;
        assert_eq!(valider_criteres(&c), Err(RefusCriteres::SeuilInvalide));
    }
    assert_eq!(
        RefusCriteres::SeuilInvalide.cle(),
        "verif.tolerances.seuil_invalide"
    );
}

#[test]
fn les_criteres_se_relisent_a_l_identique_en_json() {
    let c = Criteres {
        fidelite: Seuils {
            moyenne: None,
            ..atelier().fidelite
        },
        ..atelier()
    };
    let texte = serde_json::to_string(&c).unwrap();
    assert_eq!(serde_json::from_str::<Criteres>(&texte).unwrap(), c);
    let (_, controles) = conclure(&resultats(&couleurs(), None), &c);
    let texte = serde_json::to_string(&controles).unwrap();
    assert!(texte.contains("\"famille\":\"prediction\""), "{texte}");
    assert_eq!(
        serde_json::from_str::<Vec<Controle>>(&texte).unwrap(),
        controles
    );
}
