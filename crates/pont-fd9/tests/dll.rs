//! Appels réels à `FD9SDK.dll`, palier par palier, par l'exécutable comme le
//! fera l'application. Ignorés par défaut : à lancer seulement avec l'accord
//! du responsable du projet, FD-S2w fermé, un palier à la fois.
//!
//! La DLL de FD-S2w est 32 bits : `--target i686-pc-windows-msvc`. Une autre
//! DLL se désigne par la variable `FD9SDK_DLL`. Les sorties (adresse,
//! identifiant réels) restent à l'écran : ne jamais les recopier dans un
//! fichier versionné.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn dll() -> PathBuf {
    if let Ok(chemin) = std::env::var("FD9SDK_DLL") {
        return PathBuf::from(chemin);
    }
    if cfg!(target_pointer_width = "32") {
        PathBuf::from("C:/Program Files (x86)/KONICA MINOLTA/FD-S2w/Module/FD9SDK.dll")
    } else {
        PathBuf::from("C:/Program Files/Ergosoft 16/FD9SDK.dll")
    }
}

/// Lance le pont avec ce plafond, lui envoie ces lignes et rend ses réponses.
fn dialoguer(plafond: &str, requetes: &[&str]) -> Vec<String> {
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_pont-fd9"))
        .arg("--dll")
        .arg(dll())
        .args(["--plafond", plafond])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut entree = enfant.stdin.take().unwrap();
    for requete in requetes {
        writeln!(entree, "{requete}").unwrap();
    }
    drop(entree);
    let sortie = enfant.wait_with_output().unwrap();
    let texte = String::from_utf8(sortie.stdout).unwrap();
    println!("{texte}");
    assert!(sortie.status.success(), "code {:?}", sortie.status.code());
    texte.lines().map(String::from).collect()
}

/// Palier Version : charge la DLL, résout la liste blanche, appelle
/// `FD9_GetLastError`. Ne parle pas à l'instrument.
#[test]
#[ignore = "charge la DLL Konica Minolta du poste"]
fn palier_version() {
    let lignes = dialoguer(
        "version",
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    assert_eq!(lignes.len(), 3);
    assert!(lignes[0].contains(r#""rep":"version_dll""#));
    assert!(lignes[1].contains("palier_non_autorise"));
    assert!(lignes[2].contains(r#""rep":"ferme""#));
}

/// Palier Détection : `FD9_GetDeviceList` (USB puis diffusion UDP sur le
/// port 49152). FD-S2w doit être fermé.
#[test]
#[ignore = "interroge le réseau local avec la DLL Konica Minolta du poste"]
fn palier_detection() {
    let lignes = dialoguer(
        "detection",
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    assert_eq!(lignes.len(), 4);
    assert!(lignes[0].contains(r#""rep":"version_dll""#));
    assert!(lignes[1].contains(r#""rep":"instruments_fd9""#));
    assert!(lignes[2].contains("palier_non_autorise"));
    assert!(lignes[3].contains(r#""rep":"ferme""#));
}

/// Adresse du FD-9 pour un essai réel, donnée par l'opérateur au moment de
/// l'essai (variable `FD9_ADRESSE`) : jamais écrite dans un fichier.
fn adresse() -> String {
    std::env::var("FD9_ADRESSE").expect("FD9_ADRESSE : adresse du FD-9 pour cet essai")
}

/// Palier Connexion (essai E1) : rappel inscrit, `FD9_Connect` (0 ou 1901),
/// identité, état, puis déconnexion. Aucun geste de l'opérateur. FD-S2w et
/// MYIROtools fermés.
#[test]
#[ignore = "se connecte au FD-9 avec la DLL Konica Minolta du poste"]
fn palier_connexion() {
    let connecter = format!(r#"{{"cmd":"connecter_adresse","adresse":"{}"}}"#, adresse());
    let lignes = dialoguer(
        "connexion",
        &[r#"{"cmd":"version"}"#, &connecter, r#"{"cmd":"fermer"}"#],
    );
    assert_eq!(lignes.len(), 3);
    assert!(lignes[0].contains(r#""rep":"version_dll""#));
    assert!(lignes[1].contains(r#""rep":"connecte""#), "{}", lignes[1]);
    assert!(lignes[2].contains(r#""rep":"ferme""#), "{}", lignes[2]);
}
