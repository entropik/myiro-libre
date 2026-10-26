//! Suivi de l'avancement d'un calcul, tel que l'écran le montre : étape,
//! position de la barre, pourcentage de la suite en cours. Rien ici
//! ne lance ArgyllCMS.

use profilage::{Avancement, EtapeCalcul, SuiviAvancement};

fn etape(e: EtapeCalcul) -> Avancement {
    Avancement::Etape(e)
}

fn pct(p: u8) -> Avancement {
    Avancement::Pourcentage(p)
}

fn ligne(t: &str) -> Avancement {
    Avancement::Ligne(t.into())
}

const TOTAL: f64 = EtapeCalcul::TOUTES.len() as f64;

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn au_depart_la_preparation_commence_et_rien_n_avance() {
    let s = SuiviAvancement::new();
    let e = s.etat();
    assert_eq!(e.etape, EtapeCalcul::Preparation);
    assert_eq!((e.faites, e.total), (0, 7));
    assert!(proche(e.position, 0.0));
    assert_eq!((e.pourcentage, e.ligne.as_deref()), (None, None));
}

/// Chaque étape occupe une part égale : une étape qui commence place la
/// barre au début de sa part ; une étape déjà passée ne la fait pas reculer.
#[test]
fn une_etape_qui_commence_avance_la_barre_d_une_part() {
    let mut s = SuiviAvancement::new();
    s.recevoir(&etape(EtapeCalcul::Table));
    assert_eq!(s.etat().etape, EtapeCalcul::Table);
    assert_eq!(s.etat().faites, 2);
    assert!(proche(s.etat().position, 2.0 / TOTAL));
    s.recevoir(&etape(EtapeCalcul::Profil));
    assert_eq!(
        s.etat().etape,
        EtapeCalcul::Table,
        "une étape ne recule pas"
    );
    assert!(proche(s.etat().position, 2.0 / TOTAL));
}

/// Un pourcentage écrit par ArgyllCMS fait avancer la barre dans la part de
/// l'étape en cours.
#[test]
fn un_pourcentage_reel_avance_la_barre_dans_sa_part() {
    let mut s = SuiviAvancement::new();
    s.recevoir(&etape(EtapeCalcul::Inverses));
    s.recevoir(&ligne("Creating B to A tables"));
    s.recevoir(&pct(0));
    s.recevoir(&pct(50));
    let e = s.etat();
    assert_eq!(e.pourcentage, Some(50));
    assert_eq!(e.ligne.as_deref(), Some("Creating B to A tables"));
    assert!(proche(e.position, (4.0 + 0.5) / TOTAL));
}

/// Plusieurs suites de 0 à 100 % dans la même étape : la barre suit la plus
/// haute valeur de la suite en cours et ne recule jamais.
#[test]
fn plusieurs_suites_de_pourcentages_dans_une_etape() {
    let mut s = SuiviAvancement::new();
    s.recevoir(&etape(EtapeCalcul::Profil));
    s.recevoir(&ligne("About to optimise temporary matrix"));
    for p in [0, 21, 60, 100] {
        s.recevoir(&pct(p));
    }
    let fin_de_part = 2.0 / TOTAL;
    assert!(proche(s.etat().position, fin_de_part));
    assert_eq!(s.etat().pourcentage, Some(100));

    // Nouvelle suite, après une ligne : la barre ne recule pas.
    s.recevoir(&ligne("About to optimise input curves and matrix"));
    assert_eq!(s.etat().pourcentage, None);
    s.recevoir(&pct(0));
    s.recevoir(&pct(30));
    assert_eq!(s.etat().pourcentage, Some(30));
    assert!(proche(s.etat().position, fin_de_part));

    // Nouvelle suite sans ligne entre les deux : le pourcentage qui baisse
    // ouvre une nouvelle suite, sans faire reculer la barre.
    s.recevoir(&pct(10));
    assert_eq!(s.etat().pourcentage, Some(10));
    assert!(proche(s.etat().position, fin_de_part));

    // L'étape suivante repart du début de sa part.
    s.recevoir(&etape(EtapeCalcul::Table));
    assert!(proche(s.etat().position, fin_de_part));
    assert_eq!(s.etat().pourcentage, None);
}

/// Dans une suite, la plus haute valeur compte : un pourcentage répété ou
/// arrivé en retard ne fait rien reculer.
#[test]
fn dans_une_suite_la_plus_haute_valeur_compte() {
    let mut s = SuiviAvancement::new();
    s.recevoir(&etape(EtapeCalcul::Inverses));
    s.recevoir(&pct(40));
    s.recevoir(&pct(40));
    assert_eq!(s.etat().pourcentage, Some(40));
    s.recevoir(&pct(41));
    assert!(proche(s.etat().position, (4.0 + 0.41) / TOTAL));
}

/// Ce que la page reçoit, en JSON.
#[test]
fn l_etat_s_envoie_en_json() {
    let mut s = SuiviAvancement::new();
    s.recevoir(&etape(EtapeCalcul::Inverses));
    s.recevoir(&ligne("Creating B to A tables"));
    s.recevoir(&pct(50));
    let json = serde_json::to_value(s.etat()).unwrap();
    assert_eq!(json["etape"], "inverses");
    assert_eq!(json["faites"], 4);
    assert_eq!(json["total"], 7);
    assert_eq!(json["pourcentage"], 50);
    assert_eq!(json["ligne"], "Creating B to A tables");
    // La lumière de la barre reste allumée pendant tout le calcul : l'état
    // ne dit plus s'il faut un reflet.
    assert!(json.get("reflet").is_none(), "{json}");
    assert!(json["position"].as_f64().is_some());
}
