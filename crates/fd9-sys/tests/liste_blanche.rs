//! La liste blanche ne doit jamais nommer un export dangereux (CLAUDE.md, ADR 0005).

use fd9_sys::EXPORTS_AUTORISES;

#[test]
fn les_exports_des_paliers_version_detection_connexion_sont_autorises() {
    for nom in [
        "FD9_GetLastError",
        "FD9_GetDeviceList",
        "FD9_RegisterDeviceEventHandler",
        "FD9_Connect",
        "FD9_GetSystemInfo",
        "FD9_Disconnect",
    ] {
        assert!(EXPORTS_AUTORISES.contains(&nom), "manque : {nom}");
    }
}

#[test]
fn aucun_export_jig_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(!nom.contains("JIG"), "export JIG autorisé : {nom}");
    }
}

#[test]
fn aucun_export_set_n_est_autorise() {
    // Aucune exception pour le FD-9 : SetOption et SetNetworkSetting restent
    // dehors tant que leur contrat n'est pas établi.
    for nom in EXPORTS_AUTORISES {
        assert!(!nom.contains("_Set"), "export d'écriture autorisé : {nom}");
    }
}

#[test]
fn les_exports_a_risque_restent_interdits() {
    for nom in [
        "JIG_GetSDKVersion",
        "JIG_TileMeasurement",
        "JIG_SetNetworkSetting",
        "JIG_SetFirstStartupDate",
        "FD9_SetNetworkSetting",
        "FD9_SetOption",
        // Effet sur l'appareil inconnu : contrat non établi.
        "FD9_RegisterUserIlluminant",
        // Prise de main prioritaire : contrat non établi.
        "FD9_TAConnect",
    ] {
        assert!(!EXPORTS_AUTORISES.contains(&nom), "autorisé à tort : {nom}");
    }
}

#[test]
fn chaque_export_autorise_existe_dans_la_dll_de_reference() {
    // Les 38 exports FD9_* de FD9SDK 1.3.2.3 x86 (FD-S2w), identiques en
    // 1.3.1.5 x64 (Audit-MYIRO/retroanalyse/fd9-x86/metadata.json).
    let exports_de_la_dll = [
        "FD9_CalculateData",
        "FD9_ClippingChart",
        "FD9_Connect",
        "FD9_CreateBarcodeImage",
        "FD9_CreateChartImage",
        "FD9_DeleteChartImage",
        "FD9_DeleteJob",
        "FD9_Disconnect",
        "FD9_ExitMeasurement",
        "FD9_GetCaptureResult",
        "FD9_GetChartImage",
        "FD9_GetData",
        "FD9_GetDeviceList",
        "FD9_GetJobList",
        "FD9_GetLastError",
        "FD9_GetMeasurePoint",
        "FD9_GetNetworkSetting",
        "FD9_GetOption",
        "FD9_GetPatchRecogResult",
        "FD9_GetPatchRecognitionResult2",
        "FD9_GetPremeasurementError",
        "FD9_GetStatus",
        "FD9_GetSystemInfo",
        "FD9_MatchingChart",
        "FD9_RegisterDeviceEventHandler",
        "FD9_RegisterDisplayImage",
        "FD9_RegisterJob",
        "FD9_RegisterUserIlluminant",
        "FD9_SetNetworkSetting",
        "FD9_SetOption",
        "FD9_StartCapture",
        "FD9_StartMeasurement",
        "FD9_StartMeasurement2",
        "FD9_StartPatchRecognition",
        "FD9_StartPreMeasurement",
        "FD9_StopMeasurement",
        "FD9_TAConnect",
        "FD9_WriteBmpFile",
    ];
    for nom in EXPORTS_AUTORISES {
        assert!(
            exports_de_la_dll.contains(nom),
            "export inconnu de la DLL : {nom}"
        );
    }
}
