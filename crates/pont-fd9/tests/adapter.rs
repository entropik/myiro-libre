//! Adapter réel de `FD9SDK.dll`, vérifié sans jamais la charger : liste
//! blanche des exports résolus et lecture de la version du fichier.

use fd9_sys::EXPORTS_AUTORISES;
use pont_fd9::dll::{autoriser_export, version_du_fichier, ErreurChargement, EXPORTS_RESOLUS};

#[test]
fn l_adapter_ne_resout_que_des_exports_de_la_liste_blanche() {
    for nom in EXPORTS_RESOLUS {
        assert!(
            EXPORTS_AUTORISES.contains(nom),
            "hors liste blanche : {nom}"
        );
        assert_eq!(autoriser_export(nom), Ok(()));
    }
}

#[test]
fn les_exports_usine_et_d_ecriture_sont_refuses_avant_toute_resolution() {
    for nom in [
        "JIG_GetSDKVersion",
        "JIG_SetNetworkSetting",
        "FD9_SetNetworkSetting",
        "FD9_SetOption",
        "FD9_TAConnect",
        "FD9_StartMeasurement",
    ] {
        assert_eq!(
            autoriser_export(nom),
            Err(ErreurChargement::ExportRefuse(nom.into()))
        );
    }
}

/// `LOAD_WITH_ALTERED_SEARCH_PATH` ne cherche les dépendances dans le dossier
/// de la DLL que pour un chemin absolu écrit avec des « \ » : avec des « / »,
/// le chargement de la DLL de FD-S2w échoue (constaté le 9 octobre 2026).
#[cfg(windows)]
#[test]
fn le_chemin_de_la_dll_est_rendu_absolu_avec_des_barres_inverses() {
    use pont_fd9::dll::chemin_pour_chargement;
    use std::path::{Path, PathBuf};
    assert_eq!(
        chemin_pour_chargement(Path::new(
            "C:/Program Files (x86)/KONICA MINOLTA/FD-S2w/Module/FD9SDK.dll"
        )),
        PathBuf::from(r"C:\Program Files (x86)\KONICA MINOLTA\FD-S2w\Module\FD9SDK.dll")
    );
    assert_eq!(
        chemin_pour_chargement(Path::new(r"C:\Logiciel/Module\FD9SDK.dll")),
        PathBuf::from(r"C:\Logiciel\Module\FD9SDK.dll")
    );
    let relatif = chemin_pour_chargement(Path::new("SDK/FD9SDK.dll"));
    assert!(relatif.is_absolute(), "{}", relatif.display());
    assert!(!relatif.to_string_lossy().contains('/'));
    assert!(relatif.ends_with(r"SDK\FD9SDK.dll"));
}

#[cfg(windows)]
#[test]
fn la_version_est_lue_dans_la_ressource_sans_charger_le_fichier() {
    // Une DLL système quelconque : seule sa ressource de version est lue.
    let systeme = std::env::var("SystemRoot").unwrap_or_else(|_| "C:/Windows".into());
    let version = version_du_fichier(std::path::Path::new(&format!(
        "{systeme}/System32/kernel32.dll"
    )));
    let [majeur, ..] = version.expect("kernel32.dll a une ressource de version");
    assert!(majeur >= 6, "{version:?}");
}

#[test]
fn un_fichier_sans_ressource_de_version_rend_inconnu() {
    let fichier = std::env::temp_dir().join("myiro-libre-pas-une-dll.txt");
    std::fs::write(&fichier, "pas une DLL").unwrap();
    assert_eq!(version_du_fichier(&fichier), None);
    assert_eq!(
        version_du_fichier(std::path::Path::new("C:/nulle-part/FD9SDK.dll")),
        None
    );
}
