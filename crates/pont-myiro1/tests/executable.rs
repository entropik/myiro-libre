//! L'exécutable pont-myiro1, lancé comme le fera l'application.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn pont() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pont-myiro1"))
}

#[test]
fn sans_dll_le_pont_s_arrete_avec_un_message() {
    let sortie = pont().output().unwrap();
    assert_eq!(sortie.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("--dll"));
}

#[test]
fn un_palier_inconnu_est_refuse() {
    let sortie = pont()
        .args(["--dll", "x.dll", "--plafond", "tout"])
        .output()
        .unwrap();
    assert_eq!(sortie.status.code(), Some(2));
}

#[test]
fn une_dll_introuvable_est_signalee() {
    let sortie = pont()
        .args(["--dll", "C:/nulle-part/FDXSDK.dll"])
        .output()
        .unwrap();
    assert_eq!(sortie.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&sortie.stderr).contains("DllIntrouvable"));
}

/// Dialogue réel par l'entrée et la sortie standard, avec la DLL du poste et le
/// plafond Détection : aucun contact avec l'instrument au-delà de sa détection.
#[test]
#[ignore = "charge la DLL Konica Minolta du poste"]
fn dialogue_reel_par_l_entree_et_la_sortie_standard() {
    let racine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dll = if cfg!(target_pointer_width = "32") {
        racine.join("SDK/MY-CT1-x86/FDXSDK.dll")
    } else {
        racine.join("SDK/Ergosoft-x64/FDXSDK.dll")
    };
    let mut enfant = pont()
        .arg("--dll")
        .arg(&dll)
        .args(["--plafond", "detection"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    enfant
        .stdin
        .take()
        .unwrap()
        .write_all(
            concat!(
                r#"{"cmd":"version"}"#,
                "\n",
                r#"{"cmd":"detecter"}"#,
                "\n",
                r#"{"cmd":"connecter","instrument":0}"#,
                "\n",
                r#"{"cmd":"fermer"}"#,
                "\n"
            )
            .as_bytes(),
        )
        .unwrap();
    let sortie = enfant.wait_with_output().unwrap();
    let texte = String::from_utf8(sortie.stdout).unwrap();
    println!("{texte}");
    assert!(sortie.status.success());
    let lignes: Vec<&str> = texte.lines().collect();
    assert_eq!(lignes.len(), 4);
    assert!(lignes[0].contains(r#""rep":"version""#));
    assert!(lignes[1].contains(r#""rep":"instruments""#));
    assert!(lignes[2].contains("palier_non_autorise"));
    assert!(lignes[3].contains(r#""rep":"ferme""#));
}
