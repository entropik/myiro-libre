//! Plan figé au départ d'une séance de mesure.

use seance_mesure::{IdentiteMire, Plan, RefusPlan};

fn mire() -> IdentiteMire {
    IdentiteMire {
        id: 7,
        nom: "RGB-2520".into(),
        empreinte: "empreinte-fictive".into(),
    }
}

const FICHE: &str = r#"{"imprimante":"Traceur fictif","confirmee":true}"#;

#[test]
fn le_plan_numerote_les_plages_comme_la_mire() {
    let plan = Plan::figer(
        mire(),
        &[[100.0, 100.0, 100.0], [0.0, 50.0, 25.0]],
        FICHE,
        true,
    )
    .unwrap();
    let ids: Vec<&str> = plan.plages.iter().map(|p| p.identifiant.as_str()).collect();
    assert_eq!(ids, ["1", "2"]);
    assert_eq!(plan.plages[1].rvb, [0.0, 50.0, 25.0]);
    assert_eq!(plan.mire, mire());
    assert_eq!(plan.fiche, FICHE);
}

#[test]
fn une_fiche_non_confirmee_est_refusee() {
    let refus = Plan::figer(mire(), &[[100.0, 100.0, 100.0]], FICHE, false).unwrap_err();
    assert_eq!(refus, RefusPlan::FicheNonConfirmee);
}

#[test]
fn une_mire_sans_plage_est_refusee() {
    assert_eq!(
        Plan::figer(mire(), &[], FICHE, true).unwrap_err(),
        RefusPlan::MireVide
    );
}

#[test]
fn une_valeur_hors_de_0_a_100_est_refusee() {
    let refus =
        Plan::figer(mire(), &[[0.0, 0.0, 0.0], [0.0, 100.5, 0.0]], FICHE, true).unwrap_err();
    assert_eq!(
        refus,
        RefusPlan::ValeurHorsEchelle {
            identifiant: "2".into()
        }
    );
}

#[test]
fn le_plan_se_relit_tel_quel() {
    let plan = Plan::figer(mire(), &[[100.0, 0.0, 0.0]], FICHE, true).unwrap();
    let texte = serde_json::to_string(&plan).unwrap();
    assert_eq!(serde_json::from_str::<Plan>(&texte).unwrap(), plan);
}
