//! Incidents d'une feuille au FD-9, contre le FD-9 simulé : chaque état
//! d'erreur du rappel devient une erreur nommée, la feuille est close sans
//! arrêt, aucune plage n'est rendue et la session reste fermable (fiche
//! `docs/abi/FD9SDK.md`, « États du rappel et action du pont »).

mod commun;

use commun::Fd9Simule;
use pont_fd9::{Delais, Fermeture, Session};
use pont_protocole::{AdresseReseau, DefinitionFeuille, ErreurPont, Palier, ZoneFeuille};
use std::time::Duration;

fn six_plages() -> DefinitionFeuille {
    DefinitionFeuille {
        nom: "RGB-2520 - 1P MY9".into(),
        largeur: 27_940,
        hauteur: 41_660,
        zones: vec![ZoneFeuille {
            gauche: 1_349,
            haut: 2_300,
            largeur_plage: 601,
            hauteur_plage: 601,
            pas_x: 601,
            pas_y: 601,
            rangees: 2,
            colonnes: 3,
        }],
        couleurs: Some(commun::couleurs_fictives(2_520)),
    }
}

const COURTS: Delais = Delais {
    feuille: Duration::from_millis(50),
    etape: Duration::from_millis(50),
    mesure: Duration::from_millis(50),
    deconnexion: Duration::from_millis(50),
};

fn connectee(sdk: Fd9Simule) -> (Session<Fd9Simule>, usize) {
    let mut session = Session::new(sdk, Palier::Feuille).avec_delais(COURTS);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let appels = session.sdk().appels.len();
    (session, appels)
}

/// L'instrument signale `etat` pendant la mesure des plages.
fn pendant_la_mesure(etat: u32) -> Fd9Simule {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8, etat]);
    sdk
}

fn appels(session: &Session<Fd9Simule>, depuis: usize) -> Vec<&str> {
    session.sdk().appels_dll()[depuis..].to_vec()
}

/// Close par ExitMeasurement seul : ni arrêt, ni plage lue.
fn close_sans_arret_ni_plage(session: &Session<Fd9Simule>, depuis: usize) {
    let appels = appels(session, depuis);
    assert_eq!(
        appels.last(),
        Some(&"FD9_ExitMeasurement 0 nul"),
        "{appels:?}"
    );
    assert!(!appels.iter().any(|a| a.starts_with("FD9_StopMeasurement")));
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetData")));
}

#[test]
fn une_feuille_valide_rend_toutes_ses_plages() {
    let (mut session, _) = connectee(Fd9Simule::avec_une_rgb2520());
    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
}

#[test]
fn un_bourrage_rend_une_erreur_et_aucune_plage() {
    let (mut session, depuis) = connectee(pendant_la_mesure(102));

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::IncidentFeuille {
            etat: 102,
            premesure: None
        })
    );
    close_sans_arret_ni_plage(&session, depuis);
}

#[test]
fn un_couvercle_ouvert_rend_une_erreur_nommee() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartCapture", vec![4, 101]);
    let (mut session, depuis) = connectee(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::IncidentFeuille {
            etat: 101,
            premesure: None
        })
    );
    close_sans_arret_ni_plage(&session, depuis);
}

#[test]
fn une_feuille_trop_longue_rend_une_erreur_nommee() {
    let (mut session, depuis) = connectee(pendant_la_mesure(205));

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::IncidentFeuille {
            etat: 205,
            premesure: None
        })
    );
    close_sans_arret_ni_plage(&session, depuis);
}

#[test]
fn une_erreur_de_premesure_donne_sa_cause() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions
        .insert("FD9_StartPatchRecognition", vec![6, 201]);
    // E_DEFDATA_OTHER_MISMATCH : la feuille ne ressemble pas à la définition.
    sdk.premesure = 3;
    let (mut session, depuis) = connectee(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::IncidentFeuille {
            etat: 201,
            premesure: Some(3)
        })
    );
    let appels = appels(&session, depuis);
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_GetPremeasurementError", "FD9_ExitMeasurement 0 nul"]
    );
    assert!(!appels
        .iter()
        .any(|a| a.starts_with("FD9_StartMeasurement2")));
}

#[test]
fn une_plage_non_reconnue_refuse_la_feuille_avant_la_mesure() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.plages_reconnues = Some(5);
    let (mut session, depuis) = connectee(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::MireNonReconnue {
            zones: 1,
            plages: 5,
            ecart_max: 0
        })
    );
    let appels = appels(&session, depuis);
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetMeasurePoint")));
    assert!(!appels
        .iter()
        .any(|a| a.starts_with("FD9_StartMeasurement2")));
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"]
    );
}

#[test]
fn un_etat_inconnu_arrete_sans_inventer() {
    let (mut session, depuis) = connectee(pendant_la_mesure(77));

    let Err(ErreurPont::ReponseInattendue { detail }) = session.mesurer_feuille(&six_plages())
    else {
        panic!("{:?}", session.journal())
    };
    assert!(detail.contains("77"), "{detail}");
    // Arrêt tenté, refusé hors des états 1 à 8 ; sortie tentée, refusée
    // hors des états de fin : rien n'est supposé de l'état 77.
    let appels = appels(&session, depuis);
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"]
    );
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetData")));
}

#[test]
fn une_perte_de_liaison_rend_instrument_perdu() {
    // La DLL n'a plus de session après la perte : Disconnect rend 1004.
    let sdk = pendant_la_mesure(103).rendre("FD9_Disconnect", 1004);
    let (mut session, depuis) = connectee(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::InstrumentPerdu {})
    );
    close_sans_arret_ni_plage(&session, depuis);
    // Plus rien ne part vers l'instrument jusqu'à la fermeture.
    let avant = session.sdk().appels.len();
    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::InstrumentPerdu {})
    );
    assert_eq!(
        session.prenumeriser(&six_plages()),
        Err(ErreurPont::InstrumentPerdu {})
    );
    assert_eq!(session.sdk().appels.len(), avant);
    // « Aucun instrument connecté » à la fermeture n'est pas une anomalie
    // (fiche FD9_Disconnect) : la fermeture est faite, le rappel désinscrit.
    assert_eq!(session.fermer(), Fermeture::Confirmee);
    assert!(!session.sdk().inscrit);
}

#[test]
fn une_perte_de_liaison_puis_une_deconnexion_refusee_rend_fermeture_incertaine() {
    let sdk = pendant_la_mesure(103).rendre("FD9_Disconnect", 1104);
    let (mut session, _) = connectee(sdk);
    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::InstrumentPerdu {})
    );

    assert!(matches!(session.fermer(), Fermeture::Incertaine { .. }));
}

#[test]
fn apres_un_incident_la_session_reste_fermable() {
    let (mut session, depuis) = connectee(pendant_la_mesure(102));
    assert!(session.mesurer_feuille(&six_plages()).is_err());

    assert_eq!(session.fermer(), Fermeture::Confirmee);
    let appels = appels(&session, depuis);
    assert_eq!(
        appels[appels.len() - 4..],
        [
            "FD9_ExitMeasurement 0 nul",
            "FD9_Disconnect",
            "FD9_GetStatus",
            "FD9_RegisterDeviceEventHandler nul"
        ]
    );
}

#[test]
fn apres_un_incident_une_autre_feuille_peut_etre_mesuree() {
    let (mut session, _) = connectee(pendant_la_mesure(102));
    assert!(session.mesurer_feuille(&six_plages()).is_err());
    // L'opérateur retire la feuille coincée et en glisse une autre.
    session.sdk_mut().reactions.remove("FD9_StartMeasurement2");
    session.sdk_mut().a_venir = [2, 3].into();

    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
}

/// Session aux délais assez longs pour qu'une annulation arrive pendant
/// l'attente.
fn connectee_annulable(sdk: Fd9Simule) -> (Session<Fd9Simule>, usize) {
    let mut session = Session::new(sdk, Palier::Feuille).avec_delais(Delais {
        feuille: Duration::from_secs(5),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        deconnexion: Duration::from_millis(50),
    });
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let annulation = session.annulation();
    session.sdk_mut().annuler_pendant_l_attente = Some(annulation);
    let appels = session.sdk().appels.len();
    (session, appels)
}

#[test]
fn une_annulation_croisee_avec_la_fin_de_mesure_clot_la_feuille() {
    // L'opérateur annule pendant la mesure ; la mesure finit (état 9) juste
    // avant l'arrêt, qui est refusé (1104) : la feuille est close par
    // ExitMeasurement, admis à l'état 9.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8]);
    sdk.avant_arret = Some(9);
    let (mut session, depuis) = connectee_annulable(sdk);

    assert!(matches!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::MesureAnnulee { .. })
    ));
    let appels = appels(&session, depuis);
    assert_eq!(
        appels[appels.len() - 3..],
        [
            "FD9_StopMeasurement faux",
            "FD9_GetLastError",
            "FD9_ExitMeasurement 0 nul"
        ]
    );
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetData")));

    // La requête suivante repart : une nouvelle feuille est mesurée.
    session.sdk_mut().reactions.remove("FD9_StartMeasurement2");
    session.sdk_mut().a_venir = [2, 3].into();
    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
}

#[test]
fn un_arret_refuse_apres_un_incident_clot_la_feuille_et_le_dit() {
    // L'instrument signale un bourrage juste avant l'arrêt demandé.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8]);
    sdk.avant_arret = Some(102);
    let (mut session, depuis) = connectee_annulable(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::IncidentFeuille {
            etat: 102,
            premesure: None
        })
    );
    let appels = appels(&session, depuis);
    assert_eq!(appels.last(), Some(&"FD9_ExitMeasurement 0 nul"));
}

#[test]
fn un_arret_croise_avec_la_fin_de_mesure_clot_la_feuille() {
    // Arrêt admis pendant la mesure (état 8), mais c'est la fin de mesure
    // (9) qui arrive, pas la fin d'arrêt (10) : la feuille est close.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8]);
    sdk.reactions.insert("FD9_StopMeasurement", vec![9]);
    let (mut session, depuis) = connectee_annulable(sdk);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: pont_protocole::RemiseAuRepos::AuRepos {}
        })
    );
    let appels = appels(&session, depuis);
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"]
    );
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetData")));
}

#[test]
fn une_fin_de_mesure_annoncee_juste_apres_le_refus_de_l_arret_clot_la_feuille() {
    // La DLL est déjà à l'état 9 quand l'arrêt est demandé (refus 1104),
    // mais le rappel ne l'annonce qu'un peu plus tard.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8]);
    sdk.avant_arret_annonce_apres = Some(9);
    let (mut session, depuis) = connectee_annulable(sdk);

    assert!(matches!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::MesureAnnulee { .. })
    ));
    let appels = appels(&session, depuis);
    assert_eq!(appels.last(), Some(&"FD9_ExitMeasurement 0 nul"));
}

#[test]
fn une_annulation_ne_passe_pas_d_une_feuille_a_la_suivante() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, _) = connectee_annulable(sdk);
    // Première feuille : annulée pendant l'attente de la feuille.
    assert!(matches!(
        session.prenumeriser(&six_plages()),
        Err(ErreurPont::MesureAnnulee { .. })
    ));
    assert!(!session.annulation().demandee());

    // Deuxième feuille, sans nouvelle annulation : elle va à son terme.
    session.sdk_mut().a_venir = [2, 3].into();
    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
}
