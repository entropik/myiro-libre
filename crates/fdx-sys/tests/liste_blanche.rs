//! La liste blanche ne doit jamais nommer un export dangereux (CLAUDE.md, ADR 0005).

use fdx_sys::EXPORTS_AUTORISES;

#[test]
fn aucun_export_de_maintenance_usine_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(!nom.contains("JIG"), "export usine autorisé : {nom}");
    }
}

/// Exceptions nommées : exports qui agissent sur l'instrument (`FDX_Set*`,
/// `FDX_Start*`) dont le contrat est établi et qui n'écrivent rien de permanent.
/// Chaque ajout exige sa fiche docs/abi/ et l'accord du responsable (ADR 0005).
const EXCEPTIONS_NOMMEES: &[&str] = &[
    // Arme la mesure (docs/abi/FDX_SetMeasureCondition.md).
    "FDX_SetMeasureCondition",
    // Déclenche la mesure armée sans le bouton (docs/abi/FDX_StartMeasurement.md),
    // accord du mainteneur du 9 octobre 2026 (ticket #51).
    "FDX_StartMeasurement",
];

fn agit_sur_l_instrument(nom: &str) -> bool {
    nom.starts_with("FDX_Set") || nom.starts_with("FDX_Start")
}

#[test]
fn aucun_export_d_ecriture_dans_l_instrument_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(
            !agit_sur_l_instrument(nom) || EXCEPTIONS_NOMMEES.contains(nom),
            "export d'écriture autorisé : {nom}"
        );
    }
}

#[test]
fn les_exceptions_autorisees_sont_exactement_les_deux_nommees() {
    let mut exceptions: Vec<&str> = EXPORTS_AUTORISES
        .iter()
        .copied()
        .filter(|nom| agit_sur_l_instrument(nom))
        .collect();
    exceptions.sort_unstable();
    assert_eq!(exceptions, EXCEPTIONS_NOMMEES);
}

#[test]
fn les_set_de_reglage_de_l_instrument_restent_interdits() {
    for nom in [
        "FDX_SetNetworkInfo",
        "FDX_SetCalibration",
        "FDX_SetUserRefData",
        "FDX_SetShutdownTime",
        "FDX_SetSoundSetting",
        "FDX_SetIndicator",
        "FDX_SetIrradianceAdapter",
        "FDX_SetOptionIndicatorSetting",
    ] {
        assert!(!EXPORTS_AUTORISES.contains(&nom), "autorisé à tort : {nom}");
    }
}

#[test]
fn les_exports_des_premiers_paliers_sont_autorises() {
    for nom in [
        "FDX_GetSDKVersion",
        "FDX_GetDevicePortList",
        "FDX_Connect",
        "FDX_Disconnect",
        "FDX_GetDeviceInfo",
        "FDX_GetError",
    ] {
        assert!(EXPORTS_AUTORISES.contains(&nom), "manque : {nom}");
    }
}

#[test]
fn les_exports_de_l_etalonnage_sont_autorises() {
    for nom in ["FDX_RegisterDeviceEventHandler", "FDX_Calibration"] {
        assert!(EXPORTS_AUTORISES.contains(&nom), "manque : {nom}");
    }
}

#[test]
fn chaque_export_autorise_existe_dans_la_dll_de_reference() {
    // Les 32 exports publics hors JIG relevés dans FDXSDK 1.0.1 x86
    // (Audit-MYIRO/retroanalyse/arguments-x86.csv).
    let exports_de_la_dll = [
        "FDX_CalculateMeasureData",
        "FDX_Calibration",
        "FDX_CancelScanMeasurement",
        "FDX_Connect",
        "FDX_Disconnect",
        "FDX_EnableColorDeltaCheck",
        "FDX_GetDeviceInfo",
        "FDX_GetDevicePortList",
        "FDX_GetError",
        "FDX_GetFactoryCalibData",
        "FDX_GetIrradianceAdapter",
        "FDX_GetMeasureCondition",
        "FDX_GetMeasureData",
        "FDX_GetMeasureDataFromRawData",
        "FDX_GetNetworkInfo",
        "FDX_GetRAWData",
        "FDX_GetSDKVersion",
        "FDX_GetShutdownTime",
        "FDX_GetSoundSetting",
        "FDX_GetUserRefData",
        "FDX_RegisterDeviceEventHandler",
        "FDX_SetCalibration",
        "FDX_SetIndicator",
        "FDX_SetIrradianceAdapter",
        "FDX_SetMeasureCondition",
        "FDX_SetNetworkInfo",
        "FDX_SetOptionIndicatorSetting",
        "FDX_SetShutdownTime",
        "FDX_SetSoundSetting",
        "FDX_SetUserRefData",
        "FDX_StartMeasurement",
        "FDX_StopMeasurement",
    ];
    for nom in EXPORTS_AUTORISES {
        assert!(
            exports_de_la_dll.contains(nom),
            "export inconnu de la DLL : {nom}"
        );
    }
}

#[test]
fn les_exports_de_la_mesure_ponctuelle_sont_autorises() {
    for nom in [
        "FDX_SetMeasureCondition",
        "FDX_StartMeasurement",
        "FDX_StopMeasurement",
        "FDX_GetMeasureData",
    ] {
        assert!(EXPORTS_AUTORISES.contains(&nom), "manque : {nom}");
    }
}
