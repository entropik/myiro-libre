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
    for palier in ["etalonnage", "mesure_ponctuelle", "bande", "tout"] {
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

#[test]
fn les_paliers_du_fd9_sont_acceptes_au_lancement() {
    // Plafond accepté : le pont va jusqu'au chargement de la DLL (absente ici).
    for palier in ["connexion", "prenumerisation", "feuille"] {
        let sortie = pont()
            .args(["--dll", "C:/nulle-part/FD9SDK.dll", "--plafond", palier])
            .output()
            .unwrap();
        assert_eq!(sortie.status.code(), Some(3), "{palier}");
    }
}

#[test]
fn une_attente_ou_une_reconnaissance_illisible_est_refusee_au_lancement() {
    for arguments in [
        ["--attente-feuille", "deux minutes"],
        ["--reconnaissance", "auto"],
    ] {
        let sortie = pont()
            .args(["--dll", "x.dll"])
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(sortie.status.code(), Some(2), "{arguments:?}");
        assert!(String::from_utf8_lossy(&sortie.stderr).contains(arguments[0]));
    }
}

/// L'exemple est compilé par `cargo test -p pont-fd9` (toutes cibles) ou par
/// `cargo build -p pont-fd9 --example pont_fd9_simule`.
fn pont_simule() -> std::path::PathBuf {
    let deps = std::env::current_exe().unwrap();
    let profil = deps.parent().unwrap().parent().unwrap();
    let chemin = profil
        .join("examples")
        .join(format!("pont_fd9_simule{}", std::env::consts::EXE_SUFFIX));
    assert!(
        chemin.exists(),
        "{} absent : lancer `cargo test -p pont-fd9` sans filtre de cible",
        chemin.display()
    );
    chemin
}

const MIRE: &str = r#"{"nom":"RGB-2520 - 1P MY9","largeur":27940,"hauteur":41660,"zones":[{"gauche":1349,"haut":2300,"largeur_plage":601,"hauteur_plage":601,"pas_x":601,"pas_y":601,"rangees":60,"colonnes":42}]}"#;

#[test]
fn l_executable_refuse_feuille_au_plafond_connexion() {
    use std::io::Write;
    use std::process::Stdio;
    let mut enfant = Command::new(pont_simule())
        .arg("connexion")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut entree = enfant.stdin.take().unwrap();
    for requete in [
        r#"{"cmd":"version"}"#.to_string(),
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#.to_string(),
        format!(r#"{{"cmd":"mesurer_feuille","mire":{MIRE}}}"#),
        format!(r#"{{"cmd":"prenumeriser","mire":{MIRE}}}"#),
        r#"{"cmd":"fermer"}"#.to_string(),
    ] {
        writeln!(entree, "{requete}").unwrap();
    }
    drop(entree);
    let sortie = enfant.wait_with_output().unwrap();
    assert!(sortie.status.success(), "{:?}", sortie.status);
    let lignes: Vec<String> = String::from_utf8(sortie.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(lignes.len(), 5, "{lignes:?}");
    assert!(lignes[1].contains(r#""rep":"connecte""#), "{}", lignes[1]);
    for (ligne, palier) in [(&lignes[2], "feuille"), (&lignes[3], "prenumerisation")] {
        assert!(
            ligne.contains(&format!(
                r#""type":"palier_non_autorise","demande":"{palier}","plafond":"connexion""#
            )),
            "{ligne}"
        );
    }
    assert_eq!(lignes[4], r#"{"rep":"ferme"}"#);
    // Aucun appel de la feuille : la connexion, puis la fermeture.
    let appels: Vec<String> = String::from_utf8_lossy(&sortie.stderr)
        .lines()
        .filter_map(|l| l.strip_prefix("appel : ").map(String::from))
        .collect();
    assert_eq!(
        appels,
        [
            "FD9_GetLastError",
            "FD9_RegisterDeviceEventHandler rappel",
            "FD9_Connect 192.0.2.40 myiro-libre",
            "FD9_GetSystemInfo",
            "FD9_GetStatus",
            "FD9_ExitMeasurement 0 nul",
            "FD9_Disconnect",
            "FD9_GetStatus",
            "FD9_RegisterDeviceEventHandler nul",
        ]
    );
}

#[test]
fn une_deconnexion_bloquee_repond_a_fermer_puis_arrete_le_processus() {
    use std::io::Write;
    use std::process::Stdio;
    let mut enfant = Command::new(pont_simule())
        .args(["connexion", "--deconnexion-bloquee", "200"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut entree = enfant.stdin.take().unwrap();
    for requete in [
        r#"{"cmd":"version"}"#,
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        r#"{"cmd":"fermer"}"#,
    ] {
        writeln!(entree, "{requete}").unwrap();
    }
    // L'entrée reste ouverte, comme dans l'application qui attend la réponse.
    let debut = std::time::Instant::now();
    let sortie = enfant.wait_with_output().unwrap();
    drop(entree);
    // Arrêté par la garde, bien avant les 10 s de la déconnexion retenue.
    assert!(debut.elapsed() < std::time::Duration::from_secs(8));
    assert_eq!(sortie.status.code(), Some(4), "{:?}", sortie.status);
    let lignes: Vec<String> = String::from_utf8(sortie.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(lignes.len(), 3, "{lignes:?}");
    assert_eq!(
        lignes[2],
        r#"{"rep":"fermeture_incertaine","remise_au_repos":{"etat":"deconnexion_bloquee"}}"#
    );
    // Le journal est déjà parti : il ne dépend pas d'une fin normale.
    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreurs.contains("pont-fd9 : FD9_Connect : 0"), "{erreurs}");
    assert!(erreurs.contains("n'a pas rendu la main"), "{erreurs}");
}
