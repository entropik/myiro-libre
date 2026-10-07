//! Comportement de la session du pont MYIRO-1, contre un SDK simulé.

use fdx_sys::{Port, Version, TAILLE_TAMPON_INFOS};
use pont_myiro1::{SdkMyiro1, Session};
use pont_protocole::{ErreurPont, Palier};

/// SDK simulé : répond comme FDXSDK d'après docs/abi/, et note chaque appel.
#[derive(Default)]
struct SdkSimule {
    ports: Vec<Port>,
    code_connexion: i32,
    appels: Vec<String>,
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
    fn connecter(&mut self, _port: &Port, delai: u32) -> Result<(), i32> {
        self.appels.push(format!("connecter {delai}"));
        if self.code_connexion < 0 {
            Err(self.code_connexion)
        } else {
            Ok(())
        }
    }
    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32> {
        self.appels.push("infos".into());
        let mut t = [0u8; TAILLE_TAMPON_INFOS];
        t[0..4].copy_from_slice(&10002006u32.to_le_bytes());
        Ok(t)
    }
}

#[test]
fn la_progression_complete_donne_l_identite_de_l_instrument() {
    let mut session = Session::new(SdkSimule::avec_un_myiro1(), Palier::Connexion);
    session.version().unwrap();
    let ports = session.detecter().unwrap();
    assert_eq!(ports.len(), 1);
    let infos = session.connecter(0).unwrap();
    assert_eq!(infos.numero, 10002006);
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

fn session_qui_echoue_a_la_connexion(code: i32) -> Result<fdx_sys::InfosInstrument, ErreurPont> {
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
