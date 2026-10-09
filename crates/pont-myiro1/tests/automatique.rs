//! Mesure ponctuelle déclenchée par le pont, sans le bouton (ticket #51,
//! fiche `docs/abi/FDX_StartMeasurement.md`), contre un SDK simulé.
//!
//! Le simulé suit la fiche : `FDX_StartMeasurement` n'est accepté qu'en attente
//! de mesure (après l'événement 1), sinon -9986. Que le vrai MYIRO-1 accepte ce
//! déclenchement en réflexion est SUPPOSÉ : ces tests vérifient la logique du
//! pont, pas ce comportement de l'instrument.

mod commun;

use commun::{evenement, SdkSimule};
use pont_myiro1::Session;
use pont_protocole::{Declenchement, ErreurPont, Palier};

fn salve(codes: &[i32]) -> Vec<pont_myiro1::Evenement> {
    codes.iter().map(|&c| evenement(c)).collect()
}

/// Session étalonnée : `a_l_armement` est émis à l'armement, `au_declenchement`
/// au déclenchement accepté.
fn session(a_l_armement: &[i32], au_declenchement: &[i32]) -> Session<SdkSimule> {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend(salve(&[7, 8]));
    sdk.salves.push_back(salve(a_l_armement));
    sdk.salves_declenchement.push_back(salve(au_declenchement));
    let mut session = Session::new(sdk, Palier::MesurePonctuelle);
    session.version().unwrap();
    session.detecter().unwrap();
    session.connecter(0).unwrap();
    session.etalonner().unwrap();
    session
}

fn mesurer_auto(session: &mut Session<SdkSimule>) -> Result<Vec<i32>, ErreurPont> {
    session
        .mesurer_ponctuelle_avec(Declenchement::Automatique)
        .map(|mesure| mesure.evenements.iter().map(|e| e.code).collect())
}

fn appels(session: &Session<SdkSimule>) -> Vec<&str> {
    session.sdk().appels.iter().map(String::as_str).collect()
}

#[test]
fn en_automatique_le_pont_declenche_apres_l_armement_puis_lit() {
    let mut s = session(&[1], &[2, 3, 0]);
    assert_eq!(mesurer_auto(&mut s), Ok(vec![1, 2, 3]));
    let appels = appels(&s);
    let armer = appels
        .iter()
        .position(|a| *a == "armer ponctuelle")
        .unwrap();
    assert_eq!(appels[armer + 1], "declencher");
    assert_eq!(appels[armer + 2], "lire 0 10");
    assert_eq!(*appels.last().unwrap(), "arreter");
}

#[test]
fn en_manuel_le_pont_ne_declenche_jamais() {
    let mut s = session(&[1, 2, 3], &[]);
    s.mesurer_ponctuelle_avec(Declenchement::Manuel).unwrap();
    assert!(!appels(&s).contains(&"declencher"));
}

#[test]
fn sans_armement_signale_le_pont_ne_declenche_pas() {
    let mut s = session(&[], &[2, 3]);
    assert_eq!(mesurer_auto(&mut s), Err(ErreurPont::Delai {}));
    assert!(!appels(&s).contains(&"declencher"));
    assert_eq!(*appels(&s).last().unwrap(), "arreter");
}

#[test]
fn un_declenchement_refuse_au_repos_est_rapporte_et_l_instrument_desarme() {
    let mut s = session(&[1], &[]);
    s.sdk_mut().code_declenchement = -9986;
    assert_eq!(
        mesurer_auto(&mut s),
        Err(ErreurPont::DeclenchementRefuse { code: -9986 })
    );
    assert_eq!(*appels(&s).last().unwrap(), "arreter");
    assert!(!appels(&s).iter().any(|a| a.starts_with("lire")));
}

#[test]
fn un_refus_de_l_instrument_garde_son_code_et_l_etalonnage() {
    // Codes -9793 à -9789 : refus rapporté par l'instrument (supposé).
    // L'événement 0 suit le désarmement accepté : le repos est prouvé.
    let mut s = session(&[1, 0], &[]);
    s.sdk_mut().code_declenchement = -9791;
    s.sdk_mut().salves.push_back(salve(&[1, 2, 3]));
    assert_eq!(
        mesurer_auto(&mut s),
        Err(ErreurPont::DeclenchementRefuse { code: -9791 })
    );
    // L'étalonnage reste : la mesure suivante, au bouton, est possible.
    s.mesurer_ponctuelle_avec(Declenchement::Manuel).unwrap();
}

#[test]
fn une_perte_de_liaison_avant_l_armement_empeche_le_declenchement() {
    let mut s = session(&[6], &[2, 3]);
    assert_eq!(mesurer_auto(&mut s), Err(ErreurPont::InstrumentPerdu {}));
    assert!(!appels(&s).contains(&"declencher"));
}

#[test]
fn une_mesure_partie_au_bouton_pendant_l_attente_est_gardee() {
    // L'opérateur a appuyé avant que le pont ne déclenche : la mesure est faite.
    let mut s = session(&[2, 3], &[]);
    assert_eq!(mesurer_auto(&mut s), Ok(vec![2, 3]));
    assert!(!appels(&s).contains(&"declencher"));
}

#[test]
fn sans_fin_de_mesure_apres_le_declenchement_le_delai_expire_et_desarme() {
    let mut s = session(&[1], &[2]);
    assert_eq!(mesurer_auto(&mut s), Err(ErreurPont::Delai {}));
    assert_eq!(*appels(&s).last().unwrap(), "arreter");
}

/// SUPPOSÉ : l'opérateur appuie sur le bouton entre l'événement 1 et le
/// déclenchement ; la DLL, déjà sortie de l'attente de mesure, refuse (-9986).
/// Le pont attend brièvement : la mesure partie au bouton est gardée.
#[test]
fn une_mesure_partie_au_bouton_juste_avant_le_declenchement_est_gardee() {
    let mut s = session(&[1, 2, 3], &[]);
    s.sdk_mut().code_declenchement = -9986;
    assert_eq!(mesurer_auto(&mut s), Ok(vec![1, 2, 3]));
    let appels = appels(&s);
    let declencher = appels.iter().position(|a| *a == "declencher").unwrap();
    assert_eq!(
        appels[declencher + 1],
        "lire 0 10",
        "lecture avant tout désarmement"
    );
}

#[test]
fn un_refus_sans_mesure_qui_part_reste_un_refus() {
    // Rien ne suit le refus : le pont rapporte le refus, puis désarme.
    let mut s = session(&[1], &[]);
    s.sdk_mut().code_declenchement = -9986;
    assert_eq!(
        mesurer_auto(&mut s),
        Err(ErreurPont::DeclenchementRefuse { code: -9986 })
    );
}

#[test]
fn une_mesure_au_bouton_qui_echoue_apres_le_refus_est_signalee() {
    let mut s = session(&[1, 2], &[]);
    s.sdk_mut().code_declenchement = -9986;
    s.sdk_mut().salves[0].push(pont_myiro1::Evenement {
        code: 4,
        nb_donnees_brutes: 0,
        erreur: -9898,
    });
    assert_eq!(
        mesurer_auto(&mut s),
        Err(ErreurPont::MesureEchouee { erreur: -9898 })
    );
}

#[test]
fn le_journal_note_le_declenchement() {
    let mut s = session(&[1], &[2, 3]);
    mesurer_auto(&mut s).unwrap();
    let journal = s.journal().join("\n");
    assert!(journal.contains("déclenchement : code 0"), "{journal}");
}
