//! La liste blanche ne doit jamais nommer un export dangereux (CLAUDE.md, ADR 0005).

use fdx_sys::EXPORTS_AUTORISES;

#[test]
fn aucun_export_de_maintenance_usine_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(!nom.contains("JIG"), "export usine autorisé : {nom}");
    }
}

#[test]
fn aucun_export_d_ecriture_dans_l_instrument_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(
            !nom.starts_with("FDX_Set"),
            "export d'écriture autorisé : {nom}"
        );
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
