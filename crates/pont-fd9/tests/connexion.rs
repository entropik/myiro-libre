//! Palier Connexion du pont FD-9 contre le FD-9 simulé : rappel d'événements
//! inscrit avant la connexion, identité lue, fermeture dans l'ordre des
//! fiches `docs/abi/FD9_*`. Valeurs fictives (n° 12345678, 192.0.2.40).

mod commun;

use commun::{infos_fictives, Fd9Simule};
use pont_fd9::{Fermeture, Session, PALIER_MAX};
use pont_protocole::{AdresseReseau, ErreurPont, Palier, RemiseAuRepos};

/// Session au plafond donné, version et détection faites.
fn detectee(sdk: Fd9Simule, plafond: Palier) -> Session<Fd9Simule> {
    let mut session = Session::new(sdk, plafond);
    session.version().unwrap();
    session.detecter().unwrap();
    session
}

fn appels_depuis_la_detection(session: &Session<Fd9Simule>) -> Vec<&str> {
    session.sdk().appels_dll()[2..].to_vec()
}

#[test]
fn le_pont_fd9_va_jusqu_a_la_connexion() {
    assert!(PALIER_MAX >= Palier::Connexion);
}

#[test]
fn la_connexion_inscrit_le_rappel_avant_connect() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    // La main est donnée à la connexion : état 1 rendu à +0x10.
    sdk.reactions.insert("FD9_Connect", vec![1]);
    let mut session = detectee(sdk, Palier::Connexion);

    let identite = session.connecter(0).unwrap();

    assert_eq!(
        appels_depuis_la_detection(&session),
        [
            "FD9_RegisterDeviceEventHandler rappel",
            "FD9_Connect 192.0.2.40 myiro-libre",
            "FD9_GetSystemInfo",
            "FD9_GetStatus",
        ]
    );
    assert_eq!(identite.numero_serie, 12345678);
    assert_eq!(identite.micrologiciel, "1.02.0005");
    assert_eq!(identite.code_produit, "9C1A");
    assert_eq!(identite.adresse_mac, "02:00:00:00:00:01");
    assert!(!identite.recertification_due);
    // Les 100 octets de l'identité, bruts.
    assert_eq!(identite.brute_hex.len(), 200);
    assert_eq!(session.palier_atteint(), Some(Palier::Connexion));
    // Les 8 nombres de FD9_GetStatus sont gardés au journal.
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("FD9_GetStatus") && l.contains("[0, 0, 0, 0, 1, 0, 0, 0]")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn le_code_1901_connecte_avec_recertification_due() {
    let sdk = Fd9Simule::avec_un_fd9().rendre("FD9_Connect", 1901);
    let mut session = detectee(sdk, Palier::Connexion);

    let identite = session.connecter(0).unwrap();

    assert!(identite.recertification_due);
    assert!(session.sdk().connecte);
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("recertification")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn un_echec_de_connexion_desinscrit_le_rappel() {
    let mut sdk = Fd9Simule::avec_un_fd9().rendre("FD9_Connect", 1003);
    sdk.code_interne = 4001;
    let mut session = detectee(sdk, Palier::Connexion);

    assert_eq!(session.connecter(0), Err(ErreurPont::Sdk { code: 1003 }));

    assert_eq!(
        appels_depuis_la_detection(&session),
        [
            "FD9_RegisterDeviceEventHandler rappel",
            "FD9_Connect 192.0.2.40 myiro-libre",
            "FD9_GetLastError",
            "FD9_RegisterDeviceEventHandler nul",
        ]
    );
    assert!(!session.sdk().inscrit);
    assert!(session.journal().iter().any(|l| l.contains("4001")));
    assert_eq!(session.palier_atteint(), Some(Palier::Detection));
}

#[test]
fn des_infos_non_remplies_refusent_la_connexion() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.infos = None;
    let mut session = detectee(sdk, Palier::Connexion);

    assert!(matches!(
        session.connecter(0),
        Err(ErreurPont::ReponseInattendue { .. })
    ));
    // Connecté puis refusé : déconnecter, puis désinscrire.
    assert_eq!(
        appels_depuis_la_detection(&session)[2..],
        [
            "FD9_GetSystemInfo",
            "FD9_Disconnect",
            "FD9_RegisterDeviceEventHandler nul"
        ]
    );
    assert!(!session.sdk().connecte && !session.sdk().inscrit);
}

#[test]
fn un_numero_de_serie_non_numerique_refuse_la_connexion() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.infos = Some(infos_fictives(b"1234ABCD"));
    let mut session = detectee(sdk, Palier::Connexion);

    let Err(ErreurPont::ReponseInattendue { detail }) = session.connecter(0) else {
        panic!("{:?}", session.journal())
    };
    assert!(detail.contains("1234ABCD"), "{detail}");
    assert!(!session.sdk().connecte && !session.sdk().inscrit);
}

#[test]
fn la_connexion_par_adresse_ne_detecte_pas() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.version().unwrap();

    let identite = session
        .connecter_adresse(&AdresseReseau::new("192.0.2.41").unwrap())
        .unwrap();

    assert_eq!(identite.numero_serie, 12345678);
    assert!(!session
        .sdk()
        .appels_dll()
        .iter()
        .any(|a| a.starts_with("FD9_GetDeviceList")));
    assert!(session
        .sdk()
        .appels_dll()
        .contains(&"FD9_Connect 192.0.2.41 myiro-libre"));
}

#[test]
fn la_connexion_par_numero_exige_la_detection() {
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.version().unwrap();
    assert_eq!(
        session.connecter(0),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Detection
        })
    );
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    assert_eq!(session.connecter(1), Err(ErreurPont::InstrumentInconnu {}));
    assert_eq!(session.sdk().appels.len(), 2);
}

#[test]
fn une_seconde_connexion_n_appelle_pas_connect() {
    // Déjà connectée, la DLL rendrait 0 sans rien faire (fiche FD9_Connect).
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.connecter(0).unwrap();
    let appels = session.sdk().appels.len();

    assert_eq!(session.connecter(0), Err(ErreurPont::EtatIncompatible {}));
    assert_eq!(session.sdk().appels.len(), appels);
}

#[test]
fn la_fermeture_deconnecte_puis_desinscrit() {
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.connecter(0).unwrap();

    assert_eq!(session.fermer(), Fermeture::Confirmee);

    assert_eq!(
        session.sdk().appels_dll()[6..],
        ["FD9_Disconnect", "FD9_RegisterDeviceEventHandler nul"]
    );
    assert_eq!(session.detecter(), Err(ErreurPont::SessionFermee {}));
    // Refermer ne refait rien.
    assert_eq!(session.fermer(), Fermeture::Confirmee);
    assert_eq!(session.sdk().appels.len(), 8);
}

#[test]
fn une_deconnexion_refusee_rend_fermeture_incertaine() {
    let sdk = Fd9Simule::avec_un_fd9().rendre("FD9_Disconnect", 1104);
    let mut session = detectee(sdk, Palier::Connexion);
    session.connecter(0).unwrap();

    assert_eq!(
        session.fermer(),
        Fermeture::Incertaine {
            remise_au_repos: RemiseAuRepos::ArretRefuse { code: 1104 }
        }
    );
    // Ni désinscription (refusée pendant une connexion), ni nouvel essai.
    assert_eq!(session.sdk().appels_dll().last(), Some(&"FD9_Disconnect"));
    let appels = session.sdk().appels.len();
    session.fermer();
    assert_eq!(session.sdk().appels.len(), appels);
}

#[test]
fn un_rappel_apres_fermeture_est_ignore_et_compte() {
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.connecter(0).unwrap();
    session.fermer();
    let appels = session.sdk().appels.len();

    // La DLL appelle encore le rappel : il n'a plus de session.
    session.sdk_mut().rappeler(3);

    assert_eq!(session.rappels_orphelins(), 1);
    assert_eq!(session.connecter(0), Err(ErreurPont::SessionFermee {}));
    assert_eq!(session.sdk().appels.len(), appels);
}

#[test]
fn un_evenement_recu_entre_deux_requetes_est_journalise_sans_effet() {
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    session.connecter(0).unwrap();
    session.sdk_mut().rappeler(1);
    let appels = session.sdk().appels.len();

    assert_eq!(session.fermer(), Fermeture::Confirmee);

    assert_eq!(session.sdk().appels.len(), appels + 2);
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("hors requête") && l.contains("état 1")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn le_plafond_detection_refuse_la_connexion_sans_appel() {
    let mut session = detectee(Fd9Simule::avec_un_fd9(), Palier::Detection);
    let refus = Err(ErreurPont::PalierNonAutorise {
        demande: Palier::Connexion,
        plafond: Palier::Detection,
    });
    assert_eq!(session.connecter(0), refus);
    assert_eq!(
        session.connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap()),
        refus
    );
    assert_eq!(session.sdk().appels.len(), 2);
    // Rien à fermer : aucun appel non plus.
    assert_eq!(session.fermer(), Fermeture::Confirmee);
    assert_eq!(session.sdk().appels.len(), 2);
}
