//! Session du pont FD-9 contre le FD-9 simulé : paliers Version et Détection,
//! plafond, et refus de tout le reste sans appel à la DLL (ADR 0005).

mod commun;

use commun::{fd9_usb, Fd9Simule, EMPREINTE_SIMULEE};
use fd9_sys::Liaison;
use pont_fd9::{Session, CAPACITE_DETECTION, PALIER_MAX};
use pont_protocole::{ErreurPont, Palier};

#[test]
fn le_palier_version_lit_le_fichier_et_n_appelle_que_get_last_error() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Version);

    let version = session.version().unwrap();

    assert_eq!(version.version_fichier, Some([1, 3, 2, 3]));
    assert_eq!(version.empreinte, Some(EMPREINTE_SIMULEE.repeat(32)));
    assert_eq!(version.code_interne, 0);
    assert_eq!(session.sdk().appels_dll(), ["FD9_GetLastError"]);
    assert_eq!(session.palier_atteint(), Some(Palier::Version));
}

#[test]
fn une_version_illisible_reste_inconnue_sans_bloquer_le_palier() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.version_fichier = None;
    sdk.empreinte = None;
    let mut session = Session::new(sdk, Palier::Version);

    let version = session.version().unwrap();

    assert_eq!(version.version_fichier, None);
    assert_eq!(version.empreinte, None);
}

#[test]
fn la_detection_rend_le_fd9_du_reseau() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Detection);
    session.version().unwrap();

    let appareils = session.detecter().unwrap();

    assert_eq!(appareils.len(), 1);
    assert_eq!(appareils[0].liaison(), Liaison::Reseau);
    assert_eq!(appareils[0].adresse(), "192.0.2.40");
    assert_eq!(appareils[0].identifiant(), "12345678");
    assert_eq!(
        session.sdk().appels_dll(),
        [
            "FD9_GetLastError",
            &format!("FD9_GetDeviceList {CAPACITE_DETECTION}")
        ]
    );
    assert_eq!(session.palier_atteint(), Some(Palier::Detection));
}

#[test]
fn aucun_fd9_visible_n_est_pas_une_erreur() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.appareils.clear();
    let mut session = Session::new(sdk, Palier::Detection);
    session.version().unwrap();

    assert_eq!(session.detecter(), Ok(vec![]));
}

#[test]
fn un_echec_de_la_detection_garde_le_code_public_et_journalise_le_code_interne() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.appareils.clear();
    sdk.code_liste = 1002;
    sdk.code_interne = 12051;
    let mut session = Session::new(sdk, Palier::Detection);
    session.version().unwrap();

    assert_eq!(session.detecter(), Err(ErreurPont::Sdk { code: 1002 }));
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("1002") && l.contains("12051")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn un_fd9_en_usb_reste_visible_si_le_reseau_echoue() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.appareils = vec![fd9_usb()];
    sdk.code_liste = 1002;
    let mut session = Session::new(sdk, Palier::Detection);
    session.version().unwrap();

    let appareils = session.detecter().unwrap();

    assert_eq!(appareils[0].liaison(), Liaison::Usb);
    assert_eq!(appareils[0].adresse(), "COM4");
}

#[test]
fn un_compteur_plus_grand_que_le_tableau_est_une_reponse_inattendue() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.trouves_force = Some(CAPACITE_DETECTION as u32 + 1);
    let mut session = Session::new(sdk, Palier::Detection);
    session.version().unwrap();

    assert!(matches!(
        session.detecter(),
        Err(ErreurPont::ReponseInattendue { .. })
    ));
}

#[test]
fn la_detection_attend_la_version_et_respecte_le_plafond() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Detection);
    assert_eq!(
        session.detecter(),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Version
        })
    );

    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Version);
    session.version().unwrap();
    assert_eq!(
        session.detecter(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Detection,
            plafond: Palier::Version,
        })
    );
    assert_eq!(session.sdk().appels_dll(), ["FD9_GetLastError"]);
}

#[test]
fn les_paliers_du_fd9_s_arretent_a_la_detection() {
    assert_eq!(PALIER_MAX, Palier::Detection);
    // Même lancé avec un plafond plus haut, le pont ne va pas au-delà de ses
    // propres paliers : il n'a ni connexion, ni étalonnage, ni mesure.
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Bande);
    session.version().unwrap();
    session.detecter().unwrap();
    for palier in [
        Palier::Connexion,
        Palier::Etalonnage,
        Palier::MesurePonctuelle,
        Palier::Bande,
    ] {
        assert_eq!(
            session.hors_paliers(palier),
            ErreurPont::PalierNonAutorise {
                demande: palier,
                plafond: Palier::Detection,
            }
        );
    }
    assert_eq!(session.sdk().appels.len(), 2);
}

#[test]
fn apres_la_fermeture_plus_rien_n_atteint_la_dll() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Detection);
    session.fermer();

    assert_eq!(session.version(), Err(ErreurPont::SessionFermee {}));
    assert_eq!(session.detecter(), Err(ErreurPont::SessionFermee {}));
    assert!(session.sdk().appels.is_empty());
}
