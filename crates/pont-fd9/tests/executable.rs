//! L'exécutable pont-fd9, lancé comme le fera l'application.

use std::process::Command;

fn pont() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pont-fd9"))
}

#[test]
fn sans_dll_le_pont_s_arrete_avec_un_message() {
    let sortie = pont().output().unwrap();
    assert_eq!(sortie.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("--dll"));
}

#[test]
fn un_palier_que_le_pont_fd9_ne_declare_pas_est_refuse_au_lancement() {
    for palier in ["connexion", "etalonnage", "bande", "tout"] {
        let sortie = pont()
            .args(["--dll", "x.dll", "--plafond", palier])
            .output()
            .unwrap();
        assert_eq!(sortie.status.code(), Some(2), "{palier}");
        assert!(String::from_utf8_lossy(&sortie.stderr).contains(palier));
    }
}

#[test]
fn une_dll_introuvable_est_signalee() {
    let sortie = pont()
        .args([
            "--dll",
            "C:/nulle-part/FD9SDK.dll",
            "--plafond",
            "detection",
        ])
        .output()
        .unwrap();
    assert_eq!(sortie.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("DllIntrouvable"));
}
