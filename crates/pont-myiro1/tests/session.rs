//! Comportement de la session du pont MYIRO-1, contre un SDK simulé.

use fdx_sys::{Port, Version, TAILLE_TAMPON_INFOS};
use pont_myiro1::{Connexion, Evenement, SdkMyiro1, Session};
use pont_protocole::{ErreurPont, Palier};
use std::collections::VecDeque;
use std::time::Duration;

/// SDK simulé : répond comme FDXSDK d'après docs/abi/, et note chaque appel.
#[derive(Default)]
struct SdkSimule {
    ports: Vec<Port>,
    code_connexion: i32,
    code_etalonnage: i32,
    /// Événements que l'instrument simulé émettra, dans l'ordre.
    evenements: VecDeque<Evenement>,
    appels: Vec<String>,
}

fn evenement(code: i32) -> Evenement {
    Evenement {
        code,
        nb_donnees_brutes: 0,
        erreur: 0,
    }
}

impl SdkSimule {
    fn avec_un_myiro1() -> Self {
        SdkSimule {
            ports: vec![Port {
                code_liaison: 0,
                opaque: [0; 40],
            }],
            ..Default::default()
        }
    }
}

impl SdkMyiro1 for SdkSimule {
    fn version(&mut self) -> Result<Version, i32> {
        self.appels.push("version".into());
        Ok(Version {
            partie0: 1,
            partie1: 1,
            partie2: 0,
        })
    }
    fn ports(&mut self) -> Result<Vec<Port>, i32> {
        self.appels.push("ports".into());
        Ok(self.ports.clone())
    }
    fn connecter(&mut self, _port: &Port, delai: u32) -> Result<i32, i32> {
        self.appels.push(format!("connecter {delai}"));
        if self.code_connexion < 0 {
            Err(self.code_connexion)
        } else {
            Ok(self.code_connexion)
        }
    }
    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32> {
        self.appels.push("infos".into());
        let mut t = [0u8; TAILLE_TAMPON_INFOS];
        t[0..4].copy_from_slice(&12345678u32.to_le_bytes());
        Ok(t)
    }
    fn etalonner_blanc(&mut self) -> Result<i32, i32> {
        self.appels.push("etalonner blanc".into());
        if self.code_etalonnage < 0 {
            Err(self.code_etalonnage)
        } else {
            Ok(self.code_etalonnage)
        }
    }
    /// Sans événement en attente, simule l'expiration du délai.
    fn attendre_evenement(&mut self, _delai: Duration) -> Option<Evenement> {
        self.evenements.pop_front()
    }
}

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
    assert_eq!(session.palier_atteint(), Some(Palier::Connexion));
}

#[test]
fn un_etalonnage_reussi_ouvre_le_palier_suivant() {
    let mut session = session_connectee(sdk_qui_etalonne(&[7, 8]), Palier::Etalonnage);
    session.etalonner().unwrap();
    assert_eq!(session.palier_atteint(), Some(Palier::Etalonnage));
}
