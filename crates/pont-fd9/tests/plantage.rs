//! Rapport de plantage du pont FD-9 : un arrêt dans la DLL (essai E3) doit
//! laisser sur la sortie d'erreur le code de l'exception, le module et le
//! décalage de l'adresse fautive, et, sur demande, un vidage mémoire.

use pont_fd9::plantage::{ecrire_rapport, Rapport};

fn texte(rapport: &Rapport) -> String {
    let mut s = String::new();
    ecrire_rapport(&mut s, rapport).unwrap();
    s
}

#[test]
fn le_rapport_donne_le_module_le_decalage_et_l_acces() {
    let nom: Vec<u16> = r"C:\Program Files (x86)\FD-S2w\Module\FD9SDK.dll"
        .encode_utf16()
        .collect();
    let rapport = Rapport {
        premier_passage: false,
        code: 0xc000_0005,
        adresse: 0x6a1a_14f1,
        module: Some((&nom, 0x6a10_0000)),
        acces: Some((0, 0)),
        fil: 4_242,
    };

    assert_eq!(
        texte(&rapport),
        "pont-fd9 : plantage : exception 0xc0000005 (accès mémoire refusé) à FD9SDK.dll+0xa14f1 (adresse 0x6a1a14f1), lecture à l'adresse 0x0, fil 4242\n"
    );
}

#[test]
fn le_rapport_nomme_les_arrets_connus_sans_module() {
    let rapport = Rapport {
        premier_passage: true,
        code: 0xe043_4352,
        adresse: 0x7700_1234,
        module: None,
        acces: None,
        fil: 7,
    };

    assert_eq!(
        texte(&rapport),
        "pont-fd9 : exception (premier passage) : exception 0xe0434352 (exception .NET non rattrapée) à 0x77001234 (module inconnu), fil 7\n"
    );
}

#[test]
fn une_ecriture_fautive_est_dite() {
    let rapport = Rapport {
        premier_passage: false,
        code: 0xc000_0005,
        adresse: 0x10,
        module: None,
        acces: Some((1, 0xdead)),
        fil: 1,
    };
    assert!(texte(&rapport).contains("écriture à l'adresse 0xdead"));
}

/// Variable qui fait de ce test l'enfant qui plante : sa valeur est le
/// chemin du vidage, ou vide sans vidage.
#[cfg(windows)]
const ENFANT: &str = "PONT_FD9_ESSAI_PLANTAGE";

/// L'enfant : installe le rapport, puis lève une violation d'accès comme la
/// DLL à l'essai E3 (lecture à l'adresse 0x10). Sans la variable, ne fait
/// rien : le test passe.
#[cfg(windows)]
#[test]
fn enfant_qui_plante() {
    let Ok(vidage) = std::env::var(ENFANT) else {
        return;
    };
    let vidage = (!vidage.is_empty()).then(|| std::path::PathBuf::from(vidage));
    pont_fd9::plantage::installer(vidage.as_deref(), false);
    #[link(name = "kernel32")]
    extern "system" {
        fn RaiseException(code: u32, drapeaux: u32, nombre: u32, arguments: *const usize);
    }
    let arguments: [usize; 2] = [0, 0x10];
    // SAFETY : lève une exception structurée ; le filtre du pont la rapporte
    // puis arrête le processus.
    unsafe { RaiseException(0xc000_0005, 0, 2, arguments.as_ptr()) };
    unreachable!("le filtre aurait dû arrêter le processus");
}

#[cfg(windows)]
fn faire_planter(vidage: Option<&std::path::Path>) -> std::process::Output {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "enfant_qui_plante",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            ENFANT,
            vidage.map(|v| v.display().to_string()).unwrap_or_default(),
        )
        .output()
        .unwrap()
}

#[cfg(windows)]
#[test]
fn un_plantage_est_rapporte_sur_la_sortie_d_erreur() {
    let sortie = faire_planter(None);

    // Le processus finit avec le code de l'exception, sans fenêtre de
    // Windows.
    assert_eq!(sortie.status.code(), Some(0xc000_0005_u32 as i32));
    let erreur = String::from_utf8_lossy(&sortie.stderr).to_lowercase();
    assert!(
        erreur.contains(
            "pont-fd9 : plantage : exception 0xc0000005 (accès mémoire refusé) à kernelbase.dll+0x"
        ),
        "{erreur}"
    );
    assert!(erreur.contains("lecture à l'adresse 0x10"), "{erreur}");
}

#[cfg(windows)]
#[test]
fn un_plantage_ecrit_le_vidage_demande() {
    let dossier = std::env::temp_dir().join(format!("pont-fd9-vidage-{}", std::process::id()));
    std::fs::create_dir_all(&dossier).unwrap();
    let vidage = dossier.join("pont-fd9.dmp");

    let sortie = faire_planter(Some(&vidage));

    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreur.contains("vidage mémoire écrit"), "{erreur}");
    let contenu = std::fs::read(&vidage).unwrap();
    assert_eq!(&contenu[..4], b"MDMP");
    std::fs::remove_dir_all(&dossier).unwrap();
}
