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
        "FDX_SetCalibration",
        "FDX_StartMeasurement",
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

/// Palier 3 : étalonnage sur le blanc du MYIRO-1 réel. L'instrument doit être
/// posé sur son capuchon MY-A01 avant le lancement (docs/abi/FDX_Calibration.md).
/// Aucune mesure n'est faite.
#[test]
#[ignore = "étalonne le MYIRO-1 posé sur son capuchon"]
fn palier_etalonnage_avec_le_vrai_instrument() {
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::Etalonnage);
    session.version().expect("FDX_GetSDKVersion");
    session.detecter().expect("FDX_GetDevicePortList");
    session
        .connecter(0)
        .expect("FDX_Connect + FDX_GetDeviceInfo");
    let debut = std::time::Instant::now();
    let resultat = session.etalonner();
    println!("durée            : {:?}", debut.elapsed());
    match &resultat {
        Ok(etalonnage) => {
            for e in &etalonnage.evenements {
                println!("événement        : {e:?}");
            }
        }
        Err(erreur) => println!("échec            : {erreur:?}"),
    }
    resultat.expect("étalonnage sur le blanc");
    assert_eq!(session.palier_atteint(), Some(Palier::Etalonnage));
}

/// Palier 4 : étalonnage puis mesures ponctuelles du MYIRO-1 réel, une par plage
/// de la liste `MYIRO_PLAGES` (par défaut « papier,1A1,1D1,2B1 »). Pour chacune,
/// l'opérateur pose l'instrument et appuie sur le bouton (120 s au plus). Les
/// résultats vont dans le fichier `MYIRO_SORTIE` (CSV ; par défaut dans `target/`).
#[test]
#[ignore = "étalonne puis mesure avec le MYIRO-1 réel ; demande l'opérateur"]
fn palier_mesure_ponctuelle_avec_le_vrai_instrument() {
    use std::io::Write;
    let plages = std::env::var("MYIRO_PLAGES").unwrap_or("papier,1A1,1D1,2B1".into());
    let sortie = std::env::var("MYIRO_SORTIE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/mesures-myiro1.csv")
        });
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::MesurePonctuelle);
    session.version().expect("FDX_GetSDKVersion");
    session.detecter().expect("FDX_GetDevicePortList");
    session
        .connecter(0)
        .expect("FDX_Connect + FDX_GetDeviceInfo");
    session.etalonner().expect("étalonnage sur le blanc");
    println!("étalonnage réussi");

    let mut fichier = std::fs::File::create(&sortie).expect("fichier de sortie");
    let longueurs: Vec<String> = (0..36).map(|i| format!("nm{}", 380 + 10 * i)).collect();
    writeln!(fichier, "plage;donnees;L;a;b;{}", longueurs.join(";")).unwrap();
    let texte = |v: &[f32]| {
        v.iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(";")
    };
    for plage in plages.split(',') {
        println!("mesure de {plage} : posez l'instrument et appuyez sur le bouton");
        let deja = session.journal().len();
        let resultat = session.mesurer_ponctuelle();
        for ligne in &session.journal()[deja..] {
            println!("    journal : {ligne}");
        }
        let mesure = resultat.expect("mesure ponctuelle");
        for (nom, spectre, lab) in [
            ("M0", &mesure.m0, &mesure.lab_dll[0]),
            ("M1", &mesure.m1, &mesure.lab_dll[1]),
            ("M2", &mesure.m2, &mesure.lab_dll[2]),
        ] {
            writeln!(fichier, "{plage};{nom};{};{}", texte(lab), texte(spectre)).unwrap();
            println!("  {nom} Lab {lab:?}");
        }
        writeln!(fichier, "{plage};brutes;;;;{}", texte(&mesure.brutes)).unwrap();
        println!(
            "  événements {:?}",
            mesure.evenements.iter().map(|e| e.code).collect::<Vec<_>>()
        );
    }
    println!("résultats écrits dans {}", sortie.display());
}

/// Palier 5 : étalonnage puis lecture en bande des rangées de la mire de
/// comparaison, une par passage (`MYIRO_RANGEES`, par défaut « 1,2,3 »). Pour
/// chacune, l'opérateur fait glisser l'instrument le long de la rangée, du blanc
/// au blanc (120 s au plus). Résultats dans `MYIRO_SORTIE` (CSV), plages nommées
/// comme dans les exports FD-S2w (1A1, 1B1…).
#[test]
#[ignore = "étalonne puis lit des bandes avec le MYIRO-1 réel ; demande l'opérateur"]
fn palier_bande_avec_le_vrai_instrument() {
    use std::io::Write;
    let rangees = std::env::var("MYIRO_RANGEES").unwrap_or("1,2,3".into());
    // Nombre de plages par rangée transmis à la DLL (MYIRO_PLAGES_ATTENDUES,
    // 0 ou absent : aucun contrôle).
    let plages_attendues = std::env::var("MYIRO_PLAGES_ATTENDUES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&n| n > 0);
    let sortie = std::env::var("MYIRO_SORTIE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/bandes-myiro1.csv")
        });
    let dll = FdxDll::charger(dll_du_poste()).expect("chargement de FDXSDK.dll");
    let mut session = Session::new(dll, Palier::Bande);
    session.version().expect("FDX_GetSDKVersion");
    session.detecter().expect("FDX_GetDevicePortList");
    session
        .connecter(0)
        .expect("FDX_Connect + FDX_GetDeviceInfo");
    session.etalonner().expect("étalonnage sur le blanc");
    println!("étalonnage réussi");

    let mut fichier = std::fs::File::create(&sortie).expect("fichier de sortie");
    let longueurs: Vec<String> = (0..36).map(|i| format!("nm{}", 380 + 10 * i)).collect();
    writeln!(fichier, "plage;donnees;L;a;b;{}", longueurs.join(";")).unwrap();
    let texte = |v: &[f32]| {
        v.iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(";")
    };
    for rangee in rangees.split(',') {
        println!("bande {rangee} : faites glisser l'instrument le long de la rangée");
        let deja = session.journal().len();
        let resultat = session.mesurer_bande(plages_attendues);
        for ligne in &session.journal()[deja..] {
            println!("    journal : {ligne}");
        }
        let bande = resultat.expect("lecture de bande");
        println!("  {} plages, sens {}", bande.plages.len(), bande.sens);
        for (i, plage) in bande.plages.iter().enumerate() {
            let nom = format!("{rangee}{}1", (b'A' + i as u8) as char);
            for (cond, spectre, lab) in [
                ("M0", &plage.m0, &plage.lab_dll[0]),
                ("M1", &plage.m1, &plage.lab_dll[1]),
                ("M2", &plage.m2, &plage.lab_dll[2]),
            ] {
                writeln!(fichier, "{nom};{cond};{};{}", texte(lab), texte(spectre)).unwrap();
            }
            writeln!(fichier, "{nom};brutes;;;;{}", texte(&plage.brutes)).unwrap();
            println!("  {nom} M1 Lab {:?}", plage.lab_dll[1]);
        }
    }
    println!("résultats écrits dans {}", sortie.display());
}
