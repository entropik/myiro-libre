//! Palier Prénumérisation du pont FD-9 contre le FD-9 simulé : la feuille
//! est capturée, son QR lu et ses plages reconnues, puis rendue sans mesure.
//! Séquence et états admis : fiches `docs/abi/FD9SDK.md` et `FD9_*`.

mod commun;

use commun::Fd9Simule;
use pont_fd9::{Delais, Fermeture, Session};
use pont_protocole::{
    AdresseReseau, DefinitionFeuille, ErreurPont, Info, LectureFeuille, Palier, RemiseAuRepos,
    ZoneFeuille,
};
use std::time::Duration;

/// La RGB-2520 (nombres de l'étude statique, 1/100 mm).
fn rgb2520() -> DefinitionFeuille {
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
            rangees: 60,
            colonnes: 42,
        }],
    }
}

/// Délais courts, pour que les attentes vaines finissent vite.
const COURTS: Delais = Delais {
    feuille: Duration::from_millis(50),
    etape: Duration::from_millis(50),
    mesure: Duration::from_millis(50),
    deconnexion: Duration::from_millis(50),
};

/// Session connectée au plafond donné ; rend aussi le nombre d'appels faits
/// jusque-là.
fn connectee(sdk: Fd9Simule, plafond: Palier) -> (Session<Fd9Simule>, usize) {
    let mut session = Session::new(sdk, plafond).avec_delais(COURTS);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let appels = session.sdk().appels.len();
    (session, appels)
}

/// Appels de la feuille, après la lecture de l'état qui l'ouvre toujours.
fn appels_de_la_feuille(session: &Session<Fd9Simule>, depuis: usize) -> Vec<&str> {
    let appels = &session.sdk().appels_dll()[depuis..];
    assert_eq!(appels.first(), Some(&"FD9_GetStatus"), "{appels:?}");
    appels[1..].to_vec()
}

const ARRET: [&str; 2] = ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"];

#[test]
fn une_rgb2520_est_reconnue_sans_mesurer() {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);

    let lecture = session.prenumeriser(&rgb2520()).unwrap();

    assert_eq!(
        lecture,
        LectureFeuille {
            qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
            zones: 1,
            plages: 2_520,
            fiabilite: 97,
            ecart_max: 0,
        }
    );
    assert_eq!(
        appels_de_la_feuille(&session, depuis),
        [
            "FD9_StartCapture",
            "FD9_GetCaptureResult image nulle",
            "FD9_GetCaptureResult image 1000",
            "FD9_StartPatchRecognition mode 4, couleurs nulles, rangées 0, colonnes 0",
            "FD9_GetPatchRecognitionResult2",
            ARRET[0],
            ARRET[1],
        ]
    );
    // La reconnaissance a reçu la définition, champ par champ.
    let recue = session.sdk().coordonnees_recues.as_ref().unwrap();
    assert_eq!((recue.zones, recue.plages), (1, 2_520));
    assert_eq!(recue.geometries[0].gauche, 1_349);
    assert_eq!(session.palier_atteint(), Some(Palier::Prenumerisation));
}

#[test]
fn la_capture_est_lue_en_deux_appels() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.taille_image = 4_321;
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&rgb2520()).unwrap();

    // Image nulle d'abord, puis un tampon de la taille rendue.
    assert_eq!(
        appels_de_la_feuille(&session, depuis)[1..3],
        [
            "FD9_GetCaptureResult image nulle",
            "FD9_GetCaptureResult image 4321"
        ]
    );
}

#[test]
fn sans_feuille_le_delai_rend_feuille_absente_sans_arreter() {
    // Essai E2 : état 1, aucune feuille glissée. Comme MYIROtools, l'état est
    // relu et rien n'est arrêté : l'arrêt depuis l'état 1 n'a jamais été
    // suivi de l'événement 10, et la déconnexion est restée bloquée.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::FeuilleAbsente {})
    );
    assert_eq!(appels_de_la_feuille(&session, depuis), ["FD9_GetStatus"]);

    // La fermeture rend la main : sortie, déconnexion, état relu, puis
    // désinscription.
    let avant = session.sdk().appels.len();
    assert_eq!(session.fermer(), Fermeture::Confirmee);
    assert_eq!(
        session.sdk().appels_dll()[avant..],
        [
            "FD9_ExitMeasurement 0 nul",
            "FD9_Disconnect",
            "FD9_GetStatus",
            "FD9_RegisterDeviceEventHandler nul",
        ]
    );
    assert!(!session
        .sdk()
        .appels_dll()
        .iter()
        .any(|a| a.starts_with("FD9_StopMeasurement")));
}

#[test]
fn une_feuille_engagee_au_delai_est_arretee_puis_close() {
    // État 2 : la feuille s'engage mais ne finit pas de s'engager.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir = [2].into();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::FeuilleAbsente {})
    );
    assert_eq!(
        appels_de_la_feuille(&session, depuis),
        ["FD9_GetStatus", ARRET[0], ARRET[1]]
    );
}

#[test]
fn un_etat_illisible_au_delai_rend_son_erreur_sans_arret() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);
    // Le premier GetStatus ouvre la feuille ; le second, au délai, échoue.
    session.sdk_mut().codes_statut = [0, 1102].into();

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::Sdk { code: 1102 })
    );
    assert_eq!(appels_de_la_feuille(&session, depuis), ["FD9_GetStatus"]);
}

#[test]
fn une_feuille_glissee_avant_la_requete_est_prise() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);
    // L'opérateur a glissé la feuille avant la requête : événements 2 et 3
    // reçus entre deux requêtes, état 3 lu par FD9_GetStatus.
    session.sdk_mut().rappeler(2);
    session.sdk_mut().rappeler(3);

    assert_eq!(session.prenumeriser(&rgb2520()).unwrap().plages, 2_520);
    assert_eq!(
        appels_de_la_feuille(&session, depuis)[0],
        "FD9_StartCapture"
    );
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("hors requête") && l.contains("état 3")));
}

#[test]
fn une_feuille_en_cours_d_engagement_est_attendue() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir = [3].into();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);
    session.sdk_mut().rappeler(2);

    assert_eq!(session.prenumeriser(&rgb2520()).unwrap().plages, 2_520);
    assert_eq!(
        appels_de_la_feuille(&session, depuis)[0],
        "FD9_StartCapture"
    );
}

#[test]
fn un_instrument_qui_n_attend_pas_de_feuille_est_refuse_sans_appel_de_mesure() {
    // 0 : un autre logiciel a la main ; 7 : une reconnaissance déjà finie.
    for etat in [0, 7, 77] {
        let mut sdk = Fd9Simule::avec_une_rgb2520();
        sdk.reactions.insert("FD9_Connect", vec![etat]);
        let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

        assert_eq!(
            session.prenumeriser(&rgb2520()),
            Err(ErreurPont::InstrumentIndisponible { etat })
        );
        assert_eq!(appels_de_la_feuille(&session, depuis), [] as [&str; 0]);
    }
}

#[test]
fn un_etat_illisible_refuse_la_feuille_sans_appel_de_mesure() {
    let sdk = Fd9Simule::avec_une_rgb2520();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);
    session.sdk_mut().codes.insert("FD9_GetStatus", 1102);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::Sdk { code: 1102 })
    );
    assert_eq!(appels_de_la_feuille(&session, depuis), [] as [&str; 0]);
}

#[test]
fn un_autre_qr_refuse_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.qr_texte = "CMYK-1617 - 1P MY9*1+".into();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::AutreMire {
            texte: "CMYK-1617 - 1P MY9*1+".into()
        })
    );
    let appels = appels_de_la_feuille(&session, depuis);
    assert!(!appels
        .iter()
        .any(|a| a.starts_with("FD9_StartPatchRecognition")));
    assert_eq!(appels[appels.len() - 2..], ARRET);
}

#[test]
fn un_qr_absent_n_empeche_pas_la_reconnaissance() {
    // Rien de lu (1), contenu refusé par le SDK (2), ou « FAIL » rendu comme
    // trouvé : dans les trois cas, le QR n'est pas lu.
    for (resultat, appareil) in [(1, *b"\0\0\0\0"), (2, *b"FD-9"), (0, *b"FAIL")] {
        let mut sdk = Fd9Simule::avec_une_rgb2520();
        sdk.qr_resultat = resultat;
        sdk.qr_appareil = appareil;
        let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

        let lecture = session.prenumeriser(&rgb2520()).unwrap();

        assert_eq!(lecture.qr, Info::Inconnue, "{resultat}");
        assert_eq!(lecture.plages, 2_520);
    }
}

#[test]
fn une_geometrie_decalee_refuse_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.decalage = 400;
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::MireNonReconnue {
            zones: 1,
            plages: 2_520,
            ecart_max: 400,
        })
    );
    let appels = appels_de_la_feuille(&session, depuis);
    assert_eq!(appels[appels.len() - 2..], ARRET);
}

#[test]
fn un_ecart_de_3_mm_est_admis_et_rendu() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.decalage = 300;
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(session.prenumeriser(&rgb2520()).unwrap().ecart_max, 300);
}

#[test]
fn un_nombre_de_plages_different_refuse_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.plages_reconnues = Some(2_478);
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::MireNonReconnue {
            zones: 1,
            plages: 2_478,
            ecart_max: 0,
        })
    );
}

#[test]
fn annuler_pendant_l_attente_ne_touche_pas_a_l_instrument() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let mut session = Session::new(sdk, Palier::Prenumerisation).avec_delais(Delais {
        feuille: Duration::from_secs(30),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        deconnexion: Duration::from_millis(50),
    });
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let depuis = session.sdk().appels.len();
    // L'opérateur annule pendant que la session attend la feuille.
    let annulation = session.annulation();
    session.sdk_mut().annuler_pendant_l_attente = Some(annulation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {}
        })
    );
    // Comme MYIROtools à l'état 1 : ni arrêt ni sortie.
    assert_eq!(appels_de_la_feuille(&session, depuis), ["FD9_GetStatus"]);
}

#[test]
fn sans_evenement_10_l_arret_rend_delai_sans_sortir() {
    // Feuille en cours d'engagement (état 2) : l'arrêt est admis.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir = [2].into();
    sdk.reactions.remove("FD9_StopMeasurement");
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(session.prenumeriser(&rgb2520()), Err(ErreurPont::Delai {}));
    assert_eq!(
        appels_de_la_feuille(&session, depuis),
        ["FD9_GetStatus", ARRET[0]]
    );
}

#[test]
fn une_erreur_sort_sans_arreter() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartCapture", vec![4, 102]);
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::IncidentFeuille {
            etat: 102,
            premesure: None
        })
    );
    assert_eq!(
        appels_de_la_feuille(&session, depuis),
        ["FD9_StartCapture", "FD9_ExitMeasurement 0 nul"]
    );
}

#[test]
fn un_echec_de_preparation_donne_sa_cause() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions
        .insert("FD9_StartPatchRecognition", vec![6, 201]);
    sdk.premesure = 2;
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::IncidentFeuille {
            etat: 201,
            premesure: Some(2)
        })
    );
    let appels = appels_de_la_feuille(&session, depuis);
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_GetPremeasurementError", "FD9_ExitMeasurement 0 nul"]
    );
}

#[test]
fn un_etat_inconnu_arrete_sans_rien_inventer() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.reactions.insert("FD9_StartCapture", vec![4, 77]);
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    let Err(ErreurPont::ReponseInattendue { detail }) = session.prenumeriser(&rgb2520()) else {
        panic!("{:?}", session.journal())
    };
    assert!(detail.contains("77"), "{detail}");
    // L'état 77 n'est pas dans la table : l'arrêt est tenté, refusé par le
    // SDK hors des états 1 à 8, et la sortie aussi (pas un état de fin).
    let appels = appels_de_la_feuille(&session, depuis);
    assert_eq!(appels[1..], ARRET);
}

#[test]
fn la_feuille_est_refusee_sous_le_plafond_sans_appel() {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Connexion);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Prenumerisation,
            plafond: Palier::Connexion,
        })
    );
    assert_eq!(session.sdk().appels.len(), depuis);
}

#[test]
fn la_prenumerisation_exige_la_connexion() {
    let mut session = Session::new(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);
    session.version().unwrap();

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Connexion
        })
    );
    assert_eq!(session.sdk().appels.len(), 1);
}

#[test]
fn une_definition_hors_bornes_est_refusee_sans_appel() {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);
    let mut mire = rgb2520();
    mire.zones[0].colonnes = 61;
    mire.largeur = 100_000;

    assert_eq!(
        session.prenumeriser(&mire),
        Err(ErreurPont::ParametreRefuse {})
    );
    assert_eq!(session.sdk().appels.len(), depuis);
}

#[test]
fn deux_feuilles_se_suivent_dans_la_meme_connexion() {
    let (mut session, _) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);
    session.prenumeriser(&rgb2520()).unwrap();
    session.sdk_mut().a_venir = [2, 3].into();

    assert_eq!(session.prenumeriser(&rgb2520()).unwrap().plages, 2_520);
}

#[test]
fn sans_la_main_une_annulation_n_arrete_rien() {
    // Un autre logiciel prend la main (état 0) pendant l'attente : rien à
    // arrêter, rien à clore.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir = [0].into();
    let mut session = Session::new(sdk, Palier::Prenumerisation).avec_delais(Delais {
        feuille: Duration::from_secs(30),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        deconnexion: Duration::from_millis(50),
    });
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let depuis = session.sdk().appels.len();
    let annulation = session.annulation();
    session.sdk_mut().annuler_pendant_l_attente = Some(annulation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {}
        })
    );
    // État 0 relu : comme MYIROtools, ni arrêt ni sortie.
    assert_eq!(appels_de_la_feuille(&session, depuis), ["FD9_GetStatus"]);
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("un autre logiciel a la main")));
}

#[test]
fn sans_la_main_le_delai_rend_feuille_absente() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir = [0].into();
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::FeuilleAbsente {})
    );
    assert_eq!(appels_de_la_feuille(&session, depuis), ["FD9_GetStatus"]);
}

#[test]
fn un_refus_de_la_capture_d_une_feuille_glissee_avant_est_rendu_tel_quel() {
    // L'état 3 lu au début ne vaut que ce que dit StartCapture, qui contrôle
    // lui-même l'état du SDK (1104 hors de l'état 3).
    let sdk = Fd9Simule::avec_une_rgb2520().rendre("FD9_StartCapture", 1104);
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);
    session.sdk_mut().a_venir.clear();
    session.sdk_mut().rappeler(2);
    session.sdk_mut().rappeler(3);

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::EtatIncompatible {})
    );
    assert_eq!(
        appels_de_la_feuille(&session, depuis),
        ["FD9_StartCapture", "FD9_GetLastError", ARRET[0], ARRET[1]]
    );
    assert!(session
        .journal()
        .iter()
        .any(|l| l.starts_with("FD9_StartCapture : code 1104")));
}
