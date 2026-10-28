//! La liste blanche ne doit jamais nommer un export dangereux (CLAUDE.md, ADR 0005).

use fd9_sys::EXPORTS_AUTORISES;

#[test]
fn la_liste_blanche_est_celle_de_la_mesure_d_une_feuille() {
    // Plan 6, Global Constraints : exactement ces exports, dans cet ordre.
    assert_eq!(
        EXPORTS_AUTORISES,
        [
            "FD9_GetLastError",
            "FD9_GetDeviceList",
            "FD9_RegisterDeviceEventHandler",
            "FD9_Connect",
            "FD9_GetSystemInfo",
            "FD9_GetStatus",
            "FD9_Disconnect",
            "FD9_StartCapture",
            "FD9_GetCaptureResult",
            "FD9_StartPatchRecognition",
            "FD9_GetPatchRecognitionResult2",
            "FD9_GetMeasurePoint",
            "FD9_StartMeasurement2",
            "FD9_GetData",
            "FD9_SetOption",
            "FD9_CalculateData",
            "FD9_StopMeasurement",
            "FD9_ExitMeasurement",
            "FD9_GetPremeasurementError",
        ]
    );
}

#[test]
fn aucun_export_jig_n_est_autorise() {
    for nom in EXPORTS_AUTORISES {
        assert!(!nom.contains("JIG"), "export JIG autorisé : {nom}");
    }
}

#[test]
fn set_option_est_la_seule_fonction_set_admise() {
    // FD9_SetOption range quatre réglages de calcul dans la DLL, sans rien
    // écrire dans l'instrument (fiche FD9_SetOption). Toute autre écriture
    // reste dehors.
    let ecritures: Vec<_> = EXPORTS_AUTORISES
        .iter()
        .filter(|nom| nom.contains("_Set"))
        .collect();
    assert_eq!(ecritures, [&"FD9_SetOption"]);
}

#[test]
fn la_liste_blanche_n_a_ni_jig_ni_reglage_reseau() {
    for nom in [
        "JIG_GetSDKVersion",
        "JIG_TileMeasurement",
        "JIG_SetNetworkSetting",
        "JIG_SetFirstStartupDate",
        "FD9_SetNetworkSetting",
        // Effet sur l'appareil inconnu : contrat non établi.
        "FD9_RegisterUserIlluminant",
        // Prise de main prioritaire : contrat non établi.
        "FD9_TAConnect",
        // Travaux et images de mire : la RGB-2520 se mesure sans eux.
        "FD9_RegisterJob",
        "FD9_DeleteJob",
        "FD9_GetJobList",
        "FD9_MatchingChart",
        "FD9_ClippingChart",
        "FD9_GetChartImage",
        "FD9_RegisterDisplayImage",
        "FD9_CreateBarcodeImage",
        "FD9_CreateChartImage",
        "FD9_DeleteChartImage",
        "FD9_WriteBmpFile",
        // Anciennes formes, remplacées par StartMeasurement2 et
        // GetPatchRecognitionResult2.
        "FD9_StartMeasurement",
        "FD9_StartPreMeasurement",
        "FD9_GetPatchRecogResult",
        "FD9_GetNetworkSetting",
        "FD9_GetOption",
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
