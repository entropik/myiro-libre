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
