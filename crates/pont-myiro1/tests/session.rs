//! Comportement de la session du pont MYIRO-1, contre un SDK simulé.

mod commun;

use commun::{evenement, SdkSimule, EMPREINTE_SIMULEE};
use pont_myiro1::{Connexion, Evenement, Session};
use pont_protocole::{ErreurPont, Geometrie, Palier};

#[test]
fn la_progression_complete_donne_l_identite_de_l_instrument() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    let ports = session.detecter().unwrap();
    assert_eq!(ports.len(), 1);
    let connexion = session.connecter(0).unwrap();
    assert_eq!(connexion.infos.numero, 12345678);
    assert!(!connexion.anomalie_date_initiale);
}

#[test]
fn se_connecter_avant_la_detection_est_refuse_sans_toucher_la_dll() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    assert_eq!(
        session.connecter(0),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Detection
        })
    );
    assert_eq!(session.sdk().appels, ["version"]);
}

#[test]
fn detecter_avant_la_version_est_refuse() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    assert_eq!(
        session.detecter(),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Version
        })
    );
    assert!(session.sdk().appels.is_empty());
}

#[test]
fn le_plafond_interdit_la_connexion_sans_toucher_la_dll() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Detection);
    session.version().unwrap();
    session.detecter().unwrap();
    assert_eq!(
        session.connecter(0),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Connexion,
            plafond: Palier::Detection
        })
    );
    assert_eq!(session.sdk().appels, ["version", "ports"]);
}

#[test]
fn le_plafond_version_interdit_meme_la_detection() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Version);
    session.version().unwrap();
    assert_eq!(
        session.detecter(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Detection,
            plafond: Palier::Version
        })
    );
}

#[test]
fn aucun_instrument_branche_n_est_pas_une_erreur() {
    let mut session = Session::new(SdkSimule::default(), Palier::Connexion);
    session.version().unwrap();
    assert_eq!(session.detecter(), Ok(vec![]));
}

#[test]
fn designer_un_instrument_hors_de_la_liste_est_refuse_sans_toucher_la_dll() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    session.detecter().unwrap();
    assert_eq!(session.connecter(1), Err(ErreurPont::InstrumentInconnu));
    assert_eq!(session.sdk().appels, ["version", "ports"]);
}

#[test]
fn la_connexion_attend_10_comme_eizo() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    session.detecter().unwrap();
    session.connecter(0).unwrap();
    assert_eq!(
        session.sdk().appels,
        ["version", "ports", "connecter 10", "infos"]
    );
}

fn session_qui_echoue_a_la_connexion(code: i32) -> Result<Connexion, ErreurPont> {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.code_connexion = code;
    let mut session = Session::new(sdk, Palier::Connexion);
    session.version().unwrap();
    session.detecter().unwrap();
    session.connecter(0)
}

#[test]
fn les_codes_d_erreur_de_la_dll_sont_traduits() {
    assert_eq!(
        session_qui_echoue_a_la_connexion(-9992),
        Err(ErreurPont::ParametreRefuse)
    );
    assert_eq!(
        session_qui_echoue_a_la_connexion(-9986),
        Err(ErreurPont::EtatIncompatible)
    );
    assert_eq!(
        session_qui_echoue_a_la_connexion(-1),
        Err(ErreurPont::Sdk { code: -1 })
    );
}

#[test]
fn apres_un_echec_de_connexion_on_peut_reessayer_sans_redetecter() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.code_connexion = -9992;
    let mut session = Session::new(sdk, Palier::Connexion);
    session.version().unwrap();
    session.detecter().unwrap();
    let _ = session.connecter(0);
    session.sdk_mut().code_connexion = 0;
    assert!(
        session.connecter(0).is_ok(),
        "la liste détectée reste valable"
    );
}

#[test]
fn le_bit_4_de_la_connexion_signale_une_anomalie_de_date_initiale() {
    let connexion = session_qui_echoue_a_la_connexion(4).unwrap();
    assert!(connexion.anomalie_date_initiale);
}

#[test]
fn un_code_negatif_reste_un_echec_meme_avec_le_bit_4() {
    assert_eq!(
        session_qui_echoue_a_la_connexion(-9992 | 4),
        Err(ErreurPont::Sdk { code: -9992 | 4 })
    );
}

#[test]
fn la_connexion_garde_les_octets_bruts_de_l_identite() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    session.detecter().unwrap();
    let connexion = session.connecter(0).unwrap();
    assert_eq!(connexion.identite_brute.len(), 40);
    assert_eq!(connexion.identite_brute[0..4], 12345678u32.to_le_bytes());
}

fn session_connectee(sdk: SdkSimule, plafond: Palier) -> Session<SdkSimule> {
    let mut session = Session::new(sdk, plafond);
    session.version().unwrap();
    session.detecter().unwrap();
    session.connecter(0).unwrap();
    session
}

fn sdk_qui_etalonne(codes: &[i32]) -> SdkSimule {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements = codes.iter().map(|&c| evenement(c)).collect();
    sdk
}

#[test]
fn l_etalonnage_reussit_a_l_evenement_8() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 8]), Palier::Etalonnage);
    let etalonnage = session.etalonner().unwrap();
    let codes: Vec<i32> = etalonnage.evenements.iter().map(|e| e.code).collect();
    assert_eq!(codes, [7, 8]);
    assert!(session
        .sdk()
        .appels
        .contains(&"etalonner blanc".to_string()));
}

#[test]
fn l_etalonnage_echoue_a_l_evenement_9() {
    let mut sdk = sdk_qui_etalonne(&[7]);
    sdk.evenements.push_back(Evenement {
        code: 9,
        nb_donnees_brutes: 0,
        erreur: -9984,
    });
    let mut session = session_connectee(sdk, Palier::Etalonnage);
    assert_eq!(
        session.etalonner(),
        Err(ErreurPont::EtalonnageEchoue { erreur: -9984 })
    );
}

#[test]
fn sans_reponse_de_l_instrument_l_etalonnage_expire() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7]), Palier::Etalonnage);
    assert_eq!(session.etalonner(), Err(ErreurPont::Delai));
}

#[test]
fn une_deconnexion_pendant_l_etalonnage_est_signalee() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 6]), Palier::Etalonnage);
    assert_eq!(session.etalonner(), Err(ErreurPont::InstrumentPerdu));
}

#[test]
fn le_plafond_connexion_interdit_l_etalonnage_sans_toucher_la_dll() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 8]), Palier::Connexion);
    assert_eq!(
        session.etalonner(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Etalonnage,
            plafond: Palier::Connexion
        })
    );
    assert!(!session
        .sdk()
        .appels
        .contains(&"etalonner blanc".to_string()));
}

#[test]
fn etalonner_sans_connexion_est_refuse() {
    let mut session = Session::new(sdk_qui_etalonne(&[7, 8]), Palier::Etalonnage);
    session.version().unwrap();
    session.detecter().unwrap();
    assert_eq!(
        session.etalonner(),
        Err(ErreurPont::EtatInvalide {
            attendu: Palier::Connexion
        })
    );
}

#[test]
fn un_refus_immediat_de_la_dll_est_traduit() {
    let mut sdk = sdk_qui_etalonne(&[]);
    sdk.code_etalonnage = -9986;
    let mut session = session_connectee(sdk, Palier::Etalonnage);
    assert_eq!(session.etalonner(), Err(ErreurPont::EtatIncompatible));
}

#[test]
fn un_echec_d_etalonnage_ne_permet_pas_de_mesurer() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 9]), Palier::MesurePonctuelle);
    let _ = session.etalonner();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
    assert!(!a_arme(&session));
}

#[test]
fn un_etalonnage_reussi_ouvre_le_palier_suivant() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 8]), Palier::Etalonnage);
    session.etalonner().unwrap();
    assert_eq!(session.palier_atteint(), Some(Palier::Etalonnage));
}

/// Session étalonnée dont l'instrument émettra `apres` (codes d'événement) au
/// premier armement.
fn session_etalonnee(apres: &[i32]) -> Session<SdkSimule> {
    session_etalonnee_salves(&[apres])
}

/// Idem, avec une salve d'événements par armement successif.
fn session_etalonnee_salves(salves: &[&[i32]]) -> Session<SdkSimule> {
    let mut sdk = sdk_qui_etalonne(&[7, 8]);
    sdk.salves = salves
        .iter()
        .map(|salve| salve.iter().map(|&c| evenement(c)).collect())
        .collect();
    let mut session = session_connectee(sdk, Palier::MesurePonctuelle);
    session.etalonner().unwrap();
    session
}

#[test]
fn une_mesure_ponctuelle_donne_m0_m1_m2_et_les_donnees_brutes() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    let mesure = session.mesurer_ponctuelle().unwrap();
    assert_eq!(mesure.m0, vec![10.0; 36]);
    assert_eq!(mesure.m1, vec![20.0; 36]);
    assert_eq!(mesure.m2, vec![30.0; 36]);
    assert_eq!(mesure.brutes, vec![21.0; 152]);
    assert_eq!(mesure.lab_dll, [vec![0.0; 3], vec![10.0; 3], vec![20.0; 3]]);
    let codes: Vec<i32> = mesure.evenements.iter().map(|e| e.code).collect();
    assert_eq!(codes, [1, 2, 3]);
}

#[test]
fn la_mesure_arme_lit_puis_desarme() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.mesurer_ponctuelle().unwrap();
    let appels = &session.sdk().appels;
    let fin: Vec<&str> = appels[appels.len() - 9..]
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fin,
        [
            "armer ponctuelle",
            "lire 0 10",
            "lire 1 10",
            "lire 2 10",
            "lire 1 11",
            "lire 0 0",
            "lire 1 0",
            "lire 2 0",
            "arreter"
        ]
    );
}

#[test]
fn mesurer_sans_etalonnage_est_refuse_sans_toucher_la_dll() {
    let mut session = session_connectee(sdk_qui_etalonne(&[]), Palier::MesurePonctuelle);
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
    assert!(!a_arme(&session));
}

#[test]
fn le_plafond_etalonnage_interdit_la_mesure() {
    let mut sdk = sdk_qui_etalonne(&[7, 8]);
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let mut session = session_connectee(sdk, Palier::Etalonnage);
    session.etalonner().unwrap();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::MesurePonctuelle,
            plafond: Palier::Etalonnage
        })
    );
}

#[test]
fn une_mesure_echouee_est_signalee_et_l_etalonnage_reste_valable() {
    let mut session = session_etalonnee(&[1, 2]);
    session.sdk_mut().salves[0].push(Evenement {
        code: 4,
        nb_donnees_brutes: 0,
        erreur: -9898,
    });
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::MesureEchouee { erreur: -9898 })
    );
    assert_eq!(session.palier_atteint(), Some(Palier::Etalonnage));
    assert_eq!(session.sdk().appels.last().unwrap(), "arreter");
}

#[test]
fn sans_appui_sur_le_bouton_la_mesure_expire_et_desarme() {
    let mut session = session_etalonnee(&[1]);
    assert_eq!(session.mesurer_ponctuelle(), Err(ErreurPont::Delai));
    assert_eq!(session.sdk().appels.last().unwrap(), "arreter");
}

#[test]
fn un_instrument_qui_se_dit_non_etalonne_exige_un_nouvel_etalonnage() {
    let mut session = session_etalonnee(&[]);
    session.sdk_mut().code_armement = -9983;
    assert_eq!(session.mesurer_ponctuelle(), Err(ErreurPont::NonEtalonne));
    let armements = session.sdk().appels.len();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
    assert_eq!(session.sdk().appels.len(), armements, "DLL non appelée");
}

#[test]
fn une_deconnexion_pendant_la_mesure_est_signalee() {
    let mut session = session_etalonnee(&[1, 6]);
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
}

#[test]
fn plusieurs_resultats_pour_une_mesure_ponctuelle_sont_refuses() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().resultats_par_lecture = 2;
    assert!(matches!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::ReponseInattendue { .. })
    ));
}

#[test]
fn apres_une_mesure_le_pont_attend_le_retour_au_repos() {
    // L'instrument signale son retour au repos (événement 0) après le désarmement :
    // le pont doit l'avoir consommé avant de rendre la main.
    let mut session = session_etalonnee(&[1, 2, 3, 0]);
    session.mesurer_ponctuelle().unwrap();
    assert!(
        session.sdk().evenements.is_empty(),
        "événement 0 non attendu"
    );
}

#[test]
fn deux_mesures_s_enchainent() {
    let mut session = session_etalonnee_salves(&[&[1, 2, 3, 0], &[1, 2, 3, 0]]);
    session.mesurer_ponctuelle().unwrap();
    let seconde = session.mesurer_ponctuelle().unwrap();
    let codes: Vec<i32> = seconde.evenements.iter().map(|e| e.code).collect();
    assert_eq!(codes, [1, 2, 3]);
}

#[test]
fn le_pont_desarme_avant_d_armer_comme_myiro_tools() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.mesurer_ponctuelle().unwrap();
    let appels = &session.sdk().appels;
    let armer = appels.iter().position(|a| a == "armer ponctuelle").unwrap();
    assert_eq!(appels[armer - 1], "arreter");
}

#[test]
fn le_journal_retrace_les_etapes_de_la_mesure() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.mesurer_ponctuelle().unwrap();
    let journal = session.journal().join("\n");
    assert!(journal.contains("armement : code 0"), "{journal}");
    assert!(journal.contains("événement 3"), "{journal}");
}

#[test]
fn le_desarmement_refuse_apres_une_mesure_attend_le_rearmement_automatique() {
    // Observé sur le MYIRO-1 : après l'événement 3, désarmer est refusé (-9986)
    // jusqu'au réarmement automatique (événement 1) ; ensuite il est accepté.
    let mut session = session_etalonnee_salves(&[&[1, 2, 3, 1, 0], &[1, 2, 3, 1, 0]]);
    session.sdk_mut().arret_refuse_apres_mesure = true;
    session.mesurer_ponctuelle().unwrap();
    let seconde = session.mesurer_ponctuelle().unwrap();
    let codes: Vec<i32> = seconde.evenements.iter().map(|e| e.code).collect();
    assert_eq!(codes, [1, 2, 3]);
}

/// Session étalonnée au plafond Bande, dont l'instrument rendra `plages`
/// résultats par lecture après la salve d'événements `apres`.
fn session_pour_bande(apres: &[i32], plages: usize) -> Session<SdkSimule> {
    let mut sdk = sdk_qui_etalonne(&[7, 8]);
    sdk.salves
        .push_back(apres.iter().map(|&c| evenement(c)).collect());
    sdk.resultats_par_lecture = plages;
    sdk.sens = 2;
    let mut session = session_connectee(sdk, Palier::Bande);
    session.etalonner().unwrap();
    session
}

#[test]
fn une_bande_rend_une_mesure_par_plage() {
    let mut session = session_pour_bande(&[1, 2, 3], 12);
    let bande = session.mesurer_bande(None).unwrap();
    assert_eq!(bande.plages.len(), 12);
    for plage in &bande.plages {
        assert_eq!(plage.m0, vec![10.0; 36]);
        assert_eq!(plage.m1, vec![20.0; 36]);
        assert_eq!(plage.m2, vec![30.0; 36]);
        assert_eq!(plage.brutes, vec![21.0; 152]);
        assert_eq!(plage.lab_dll[1], vec![10.0; 3]);
    }
    assert_eq!(bande.sens, 2);
}

#[test]
fn la_bande_arme_l_instrument_en_mode_bande_puis_desarme() {
    let mut session = session_pour_bande(&[1, 2, 3], 12);
    session.mesurer_bande(None).unwrap();
    let appels = &session.sdk().appels;
    assert!(appels.contains(&"armer bande 0".to_string()));
    assert!(!appels.contains(&"armer ponctuelle".to_string()));
    assert_eq!(appels.last().unwrap(), "arreter");
}

#[test]
fn la_bande_exige_le_palier_bande() {
    let mut sdk = sdk_qui_etalonne(&[7, 8]);
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let mut session = session_connectee(sdk, Palier::MesurePonctuelle);
    session.etalonner().unwrap();
    assert_eq!(
        session.mesurer_bande(None),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Bande,
            plafond: Palier::MesurePonctuelle
        })
    );
    assert!(!session
        .sdk()
        .appels
        .iter()
        .any(|a| a.starts_with("armer bande")));
}

#[test]
fn une_bande_sans_plage_reconnue_est_refusee() {
    let mut session = session_pour_bande(&[1, 2, 3], 0);
    assert!(matches!(
        session.mesurer_bande(None),
        Err(ErreurPont::ReponseInattendue { .. })
    ));
}

#[test]
fn une_bande_echouee_est_signalee() {
    let mut session = session_pour_bande(&[1, 2], 12);
    session.sdk_mut().salves[0].push(Evenement {
        code: 4,
        nb_donnees_brutes: 0,
        erreur: -9898,
    });
    assert_eq!(
        session.mesurer_bande(None),
        Err(ErreurPont::MesureEchouee { erreur: -9898 })
    );
}

#[test]
fn le_nombre_de_plages_attendu_est_transmis_a_la_dll() {
    let mut session = session_pour_bande(&[1, 2, 3], 12);
    session.mesurer_bande(Some(12)).unwrap();
    assert!(session.sdk().appels.contains(&"armer bande 12".to_string()));
}

#[test]
fn une_bande_qui_ne_compte_pas_les_plages_attendues_est_refusee() {
    // Même si la DLL ne contrôlait pas le nombre, le pont le vérifie.
    let mut session = session_pour_bande(&[1, 2, 3], 13);
    assert!(matches!(
        session.mesurer_bande(Some(12)),
        Err(ErreurPont::ReponseInattendue { .. })
    ));
}

/// Format RFC 3339 avec fuseau, à la seconde : 2026-10-07T15:04:05+02:00.
fn est_un_horodatage_avec_fuseau(texte: &str) -> bool {
    let octets = texte.as_bytes();
    texte.len() == 25
        && octets[10] == b'T'
        && (octets[19] == b'+' || octets[19] == b'-')
        && octets[22] == b':'
}

#[test]
fn une_mesure_ponctuelle_porte_sa_provenance() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    let mesure = session.mesurer_ponctuelle().unwrap();
    let p = &mesure.provenance;
    assert_eq!(p.instrument.modele, "MYIRO-1");
    assert_eq!(p.instrument.numero_serie, 12345678);
    assert_eq!(p.version_sdk, [1, 1, 0]);
    assert_eq!(
        p.empreinte_dll.valeur().map(|e| e.texte()),
        Some(EMPREINTE_SIMULEE.repeat(32).as_str())
    );
    assert_eq!(p.geometrie, Geometrie::Ponctuelle {});
    let horodatage = p.horodatage.texte();
    assert!(est_un_horodatage_avec_fuseau(horodatage), "{horodatage}");
    let etalonnage = p.etalonnage.valeur().expect("date de l'étalonnage").texte();
    assert!(est_un_horodatage_avec_fuseau(etalonnage), "{etalonnage}");
    assert!(etalonnage <= horodatage);
    assert!(!p.version_pont.is_empty());
}

#[test]
fn une_bande_porte_sa_provenance() {
    let mut session = session_pour_bande(&[1, 2, 3], 12);
    let bande = session.mesurer_bande(None).unwrap();
    assert_eq!(bande.provenance.geometrie, Geometrie::Bande { sens: 2 });
    assert_eq!(bande.provenance.instrument.numero_serie, 12345678);
}

// État courant : un palier atteint dans le passé n'autorise aucune mesure.

/// Vrai si l'instrument a été armé, pour une mesure ponctuelle ou une bande.
fn a_arme(session: &Session<SdkSimule>) -> bool {
    session.sdk().appels.iter().any(|a| a.starts_with("armer"))
}

#[test]
fn un_nouvel_etalonnage_echoue_interdit_la_mesure_avant_armement() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().evenements.extend([7, 9].map(evenement));
    assert!(matches!(
        session.etalonner(),
        Err(ErreurPont::EtalonnageEchoue { .. })
    ));
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
    assert!(!a_arme(&session));
}

#[test]
fn un_nouvel_etalonnage_expire_interdit_la_mesure_meme_si_le_palier_reste_atteint() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().evenements.push_back(evenement(7));
    assert_eq!(session.etalonner(), Err(ErreurPont::Delai));
    assert_eq!(
        session.palier_atteint(),
        Some(Palier::Etalonnage),
        "la progression reste un historique"
    );
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
    assert!(!a_arme(&session));
}

#[test]
fn un_refus_immediat_d_un_nouvel_etalonnage_invalide_l_ancien() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().code_etalonnage = -9986;
    assert_eq!(session.etalonner(), Err(ErreurPont::EtatIncompatible));
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis)
    );
}

#[test]
fn apres_une_deconnexion_pendant_l_etalonnage_plus_rien_ne_touche_la_dll() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().evenements.extend([7, 6].map(evenement));
    assert_eq!(session.etalonner(), Err(ErreurPont::InstrumentPerdu));
    let appels = session.sdk().appels.len();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
    assert_eq!(session.etalonner(), Err(ErreurPont::InstrumentPerdu));
    assert_eq!(session.sdk().appels.len(), appels);
}

#[test]
fn apres_une_deconnexion_pendant_la_mesure_la_suivante_n_arme_pas() {
    let mut session = session_etalonnee_salves(&[&[1, 6], &[1, 2, 3]]);
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
    let appels = session.sdk().appels.len();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
    assert_eq!(session.sdk().appels.len(), appels);
}

#[test]
fn une_deconnexion_au_retour_au_repos_garde_la_mesure_et_bloque_la_suivante() {
    let mut session = session_etalonnee_salves(&[&[1, 2, 3, 6], &[1, 2, 3]]);
    let mesure = session.mesurer_ponctuelle().unwrap();
    assert_eq!(mesure.m1, vec![20.0; 36]);
    assert_eq!(mesure.provenance.instrument.numero_serie, 12345678);
    assert!(mesure.provenance.etalonnage.valeur().is_some());
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
    let armements = session
        .sdk()
        .appels
        .iter()
        .filter(|a| a.starts_with("armer"))
        .count();
    assert_eq!(armements, 1);
}

#[test]
fn une_deconnexion_vue_avant_armement_empeche_d_armer() {
    // Le désarmement préalable est refusé (-9986) et l'événement attendu
    // avant de réessayer est une déconnexion.
    let mut session = session_etalonnee(&[1, 2, 3]);
    let sdk = session.sdk_mut();
    sdk.arret_refuse_apres_mesure = true;
    sdk.dernier_evenement = Some(3);
    sdk.evenements.push_back(evenement(6));
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::InstrumentPerdu)
    );
    assert!(!a_arme(&session));
}

#[test]
fn une_identite_illisible_rend_la_connexion_inexploitable() {
    let mut sdk = sdk_qui_etalonne(&[7, 8]);
    sdk.code_infos = -1;
    let mut session = Session::new(sdk, Palier::MesurePonctuelle);
    session.version().unwrap();
    session.detecter().unwrap();
    assert_eq!(session.connecter(0), Err(ErreurPont::Sdk { code: -1 }));
    assert_eq!(session.etalonner(), Err(ErreurPont::SessionInexploitable));
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::SessionInexploitable)
    );
    assert!(!session
        .sdk()
        .appels
        .contains(&"etalonner blanc".to_string()));
    assert!(!a_arme(&session));
}

#[test]
fn une_reconnexion_sans_identite_ne_reutilise_pas_l_ancienne() {
    let mut session = session_etalonnee(&[1, 2, 3]);
    session.sdk_mut().code_infos = -1;
    assert!(session.connecter(0).is_err());
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::SessionInexploitable)
    );
    assert!(!a_arme(&session));
}

#[test]
fn une_reconnexion_exige_un_nouvel_etalonnage_et_porte_la_nouvelle_identite() {
    let mut session = session_etalonnee_salves(&[&[1, 2, 3, 6], &[1, 2, 3]]);
    let avant = session.mesurer_ponctuelle().unwrap();
    session.sdk_mut().numero_serie = 87654321;
    let connexion = session.connecter(0).unwrap();
    assert_eq!(connexion.infos.numero, 87654321);
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::EtalonnageRequis),
        "l'étalonnage de l'ancienne connexion ne vaut plus"
    );
    session.sdk_mut().evenements.extend([7, 8].map(evenement));
    session.etalonner().unwrap();
    let apres = session.mesurer_ponctuelle().unwrap();
    assert_eq!(apres.provenance.instrument.numero_serie, 87654321);
    assert_eq!(
        avant.provenance.instrument.numero_serie, 12345678,
        "la mesure déjà obtenue reste intacte"
    );
}

#[test]
fn apres_reconnexion_la_date_d_etalonnage_vient_de_la_nouvelle_session() {
    let mut session = session_etalonnee_salves(&[&[1, 2, 3, 6], &[1, 2, 3]]);
    let avant = session.mesurer_ponctuelle().unwrap();
    let ancienne = avant.provenance.etalonnage.valeur().unwrap().texte();
    // La date est à la seconde : attendre d'en changer pour les distinguer.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    session.connecter(0).unwrap();
    session.sdk_mut().evenements.extend([7, 8].map(evenement));
    session.etalonner().unwrap();
    let apres = session.mesurer_ponctuelle().unwrap();
    let nouvelle = apres.provenance.etalonnage.valeur().unwrap().texte();
    assert!(nouvelle > ancienne, "{nouvelle} après {ancienne}");
}

#[test]
fn aucune_reconnexion_ne_releve_le_plafond() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 6]), Palier::Etalonnage);
    assert_eq!(session.etalonner(), Err(ErreurPont::InstrumentPerdu));
    session.connecter(0).unwrap();
    session.sdk_mut().evenements.extend([7, 8].map(evenement));
    session.etalonner().unwrap();
    assert_eq!(
        session.mesurer_ponctuelle(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::MesurePonctuelle,
            plafond: Palier::Etalonnage
        })
    );
    assert!(!a_arme(&session));
}
