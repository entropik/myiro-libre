//! Chargement de la vraie FDXSDK.dll.

use pont_myiro1::dll::{autoriser_export, ErreurChargement, FdxDll};
use pont_myiro1::Session;
use pont_protocole::{ErreurPont, Palier};
use std::path::PathBuf;

#[test]
fn un_export_hors_liste_blanche_est_refuse_avant_toute_resolution() {
    for nom in [
        "FDX_JIG_UpdateProgram",
        "FDX_JIG_SetFactoryCalib_WriteSector",
        "FDX_SetNetworkInfo",
        "FDX_Calibration",
    ] {
        assert_eq!(
            autoriser_export(nom),
            Err(ErreurChargement::ExportRefuse(nom.to_string()))
        );
    }
}

#[test]
fn un_export_de_la_liste_blanche_est_accepte() {
    assert_eq!(autoriser_export("FDX_GetSDKVersion"), Ok(()));
}

#[test]
fn une_dll_absente_est_signalee() {
    assert!(matches!(
        FdxDll::charger("C:/nulle-part/FDXSDK.dll"),
        Err(ErreurChargement::DllIntrouvable(_))
    ));
}

/// DLL de travail du dépôt (`SDK/`, non versionné) pour l'architecture compilée.
fn dll_du_poste() -> PathBuf {
    let racine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if cfg!(target_pointer_width = "32") {
        racine.join("SDK/MY-CT1-x86/FDXSDK.dll")
    } else {
        racine.join("SDK/Ergosoft-x64/FDXSDK.dll")
    }
}

/// Palier 0 : charge la vraie DLL et lit sa version, sans contact avec
/// l'instrument. Lancé seulement à la demande :
/// `cargo test -p pont-myiro1 --test dll -- --ignored`
#[test]
#[ignore = "charge la DLL Konica Minolta du poste"]
fn palier_version_avec_la_vraie_dll() {
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::Version);
    let version = session.version().expect("FDX_GetSDKVersion");
    println!("FDXSDK : {version:?}");
    // 1.0.1.0 en x86 (MY-CT1) comme en x64 (Ergosoft) : {1, 1, 0}.
    assert_eq!(
        (version.partie0, version.partie1, version.partie2),
        (1, 1, 0)
    );
    assert_eq!(
        session.detecter(),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Detection,
            plafond: Palier::Version
        }),
        "le plafond doit empêcher la détection"
    );
}

/// Palier 1 : liste les instruments vus par la DLL (USB et réseau), sans
/// se connecter. Lancé seulement à la demande, comme le palier 0.
#[test]
#[ignore = "charge la DLL Konica Minolta du poste et cherche les instruments"]
fn palier_detection_avec_la_vraie_dll() {
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::Detection);
    session.version().expect("FDX_GetSDKVersion");
    let ports = session.detecter().expect("FDX_GetDevicePortList");
    println!("{} instrument(s) détecté(s)", ports.len());
    for port in &ports {
        println!(
            "  liaison {:?}, port {:?}, n° de série {}",
            port.liaison(),
            port.nom(),
            port.numero_serie()
        );
    }
    assert_eq!(
        session.connecter(0),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Connexion,
            plafond: Palier::Detection
        }),
        "le plafond doit empêcher la connexion"
    );
}

/// Palier 2 : se connecte au premier instrument détecté, lit son identité,
/// puis se déconnecte (à la fermeture de l'adapter). Aucun étalonnage ni
/// mesure. Vérifier l'horloge de l'ordinateur avant le premier lancement
/// (docs/abi/FDX_Connect.md).
#[test]
#[ignore = "se connecte au MYIRO-1 branché sur le poste"]
fn palier_connexion_avec_le_vrai_instrument() {
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::Connexion);
    session.version().expect("FDX_GetSDKVersion");
    let ports = session.detecter().expect("FDX_GetDevicePortList");
    assert!(!ports.is_empty(), "aucun instrument détecté");
    let connexion = session
        .connecter(0)
        .expect("FDX_Connect + FDX_GetDeviceInfo");
    let infos = &connexion.infos;
    println!("n° de série      : {}", infos.numero);
    println!("micrologiciel    : {}", infos.micrologiciel);
    println!("adresse MAC      : {}", infos.adresse_mac);
    println!("code produit     : {}", infos.code_produit);
    println!("date initiale    : {:?}", infos.date_initiale);
    println!("anomalie de date : {}", connexion.anomalie_date_initiale);
    let hexa: Vec<String> = connexion
        .identite_brute
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect();
    println!("identité brute   : {}", hexa.join(" "));
    assert_eq!(
        infos.numero,
        ports[0].numero_serie(),
        "n° de série incohérent"
    );
}
