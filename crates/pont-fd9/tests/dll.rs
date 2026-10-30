//! Appels réels à `FD9SDK.dll`, palier par palier, par l'exécutable comme le
//! fera l'application. Ignorés par défaut : à lancer seulement avec l'accord
//! du responsable du projet, FD-S2w fermé, un palier à la fois.
//!
//! La DLL de FD-S2w est 32 bits : `--target i686-pc-windows-msvc`. Une autre
//! DLL se désigne par la variable `FD9SDK_DLL`. Les sorties (adresse,
//! identifiant réels) restent à l'écran : ne jamais les recopier dans un
//! fichier versionné.

use std::io::{BufRead, BufReader, Write};
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

/// Arguments de diagnostic donnés par l'opérateur : `FD9_TRACE` (non vide)
/// ajoute `--trace`, `FD9_VIDAGE` ajoute `--vidage <fichier>` (vidage local,
/// jamais versionné).
fn diagnostic() -> Vec<String> {
    let mut arguments = Vec::new();
    if std::env::var("FD9_TRACE").is_ok_and(|v| !v.is_empty()) {
        arguments.push("--trace".to_string());
    }
    if let Ok(vidage) = std::env::var("FD9_VIDAGE") {
        arguments.extend(["--vidage".to_string(), vidage]);
    }
    arguments
}

/// Composant de lecture du QR (ticket #87) : la DLL charge `zxing.dll` (et
/// `zxing.presentation.dll`) depuis le dossier du pont. S'ils manquent à côté
/// de l'exécutable de test (`target/…/debug/`, ignoré par git), ils y sont
/// copiés depuis le dossier de la DLL. `FD9_ZXING=absent` n'en copie aucun,
/// pour l'essai sans composant : le pont doit alors refuser la feuille
/// (`composant_qr_introuvable`). Rien n'est jamais effacé : un composant
/// déjà là arrête l'essai, à retirer à la main.
fn poser_zxing() {
    let pont = PathBuf::from(env!("CARGO_BIN_EXE_pont-fd9"));
    let dossier = pont.parent().unwrap();
    if std::env::var("FD9_ZXING").is_ok_and(|v| v == "absent") {
        let present = dossier.join("zxing.dll");
        assert!(
            !present.exists(),
            "FD9_ZXING=absent, mais {} existe : retirez-le à la main pour cet essai",
            present.display()
        );
        return;
    }
    let source = dll();
    let Some(module) = source.parent() else {
        return;
    };
    for nom in ["zxing.dll", "zxing.presentation.dll"] {
        let (de, vers) = (module.join(nom), dossier.join(nom));
        if de.is_file() && !vers.exists() {
            std::fs::copy(&de, &vers).unwrap();
            println!("{nom} copié à côté du pont : {}", vers.display());
        }
    }
}

/// Lance le pont avec ce plafond, lui envoie ces lignes et rend ses réponses.
fn dialoguer(plafond: &str, requetes: &[&str]) -> Vec<String> {
    dialoguer_avec(plafond, &[], requetes)
}

/// Comme `dialoguer`, avec des arguments de plus pour le pont. Chaque requête
/// attend sa réponse avant la suivante : envoyer `fermer`, ou fermer l'entrée,
/// pendant une attente de feuille l'annulerait (essai E3). L'entrée n'est
/// fermée qu'après la dernière réponse. Le journal du pont (sortie d'erreur)
/// s'affiche au fil de l'eau.
fn dialoguer_avec(plafond: &str, arguments: &[&str], requetes: &[&str]) -> Vec<String> {
    poser_zxing();
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_pont-fd9"))
        .arg("--dll")
        .arg(dll())
        .args(["--plafond", plafond])
        .args(arguments)
        .args(diagnostic())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut entree = enfant.stdin.take().unwrap();
    let mut sortie = BufReader::new(enfant.stdout.take().unwrap());
    let mut lignes = Vec::new();
    for requete in requetes {
        writeln!(entree, "{requete}").unwrap();
        entree.flush().unwrap();
        let mut ligne = String::new();
        if sortie.read_line(&mut ligne).unwrap() == 0 {
            break;
        }
        let ligne = ligne.trim_end().to_string();
        // Une mesure fait plusieurs mégaoctets : seul le début s'affiche.
        println!("{}", ligne.chars().take(400).collect::<String>());
        lignes.push(ligne);
    }
    drop(entree);
    let statut = enfant.wait().unwrap();
    assert!(statut.success(), "code {:?}", statut.code());
    lignes
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

/// Palier Prénumérisation (essais E2, E3, E3 bis) : connexion, puis attente
/// de la feuille (`FD9_ATTENTE` secondes, 120 par défaut), capture, QR et
/// reconnaissance de la RGB-2520, puis arrêt sans mesure. Geste : glisser la
/// feuille (E3), rien (E2), ou une autre feuille (E3 bis). Affiche le texte
/// du QR, les zones, les plages, la fiabilité et l'écart.
#[test]
#[ignore = "fait passer une feuille dans le FD-9 avec la DLL Konica Minolta du poste"]
fn palier_prenumerisation() {
    let attente = std::env::var("FD9_ATTENTE").unwrap_or_else(|_| "120".into());
    let connecter = format!(r#"{{"cmd":"connecter_adresse","adresse":"{}"}}"#, adresse());
    let prenumeriser = format!(r#"{{"cmd":"prenumeriser","mire":{}}}"#, rgb2520());
    let lignes = dialoguer_avec(
        "prenumerisation",
        &["--attente-feuille", &attente],
        &[
            r#"{"cmd":"version"}"#,
            &connecter,
            &prenumeriser,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    assert_eq!(lignes.len(), 4);
    assert!(lignes[1].contains(r#""rep":"connecte""#), "{}", lignes[1]);
    // E3 : `prenumerisee` ; E2 : `feuille_absente` ; E3 bis : refus nommé.
    println!("prénumérisation : {}", lignes[2]);
    assert!(lignes[3].contains(r#""rep":"ferme""#), "{}", lignes[3]);
}

/// Palier Feuille (essai E4) : la RGB-2520 est glissée, recalée sur l'image
/// prise (COMBINE avec ses couleurs, `FD9_MIRE_RVB`), mesurée, ses 2 520
/// plages lues. La mesure (JSON) va dans le
/// dossier `FD9_SORTIE` (sous `Archivage/donnees/`, local, jamais versionné) ;
/// le Lab M1 des plages 1, 42, 2479 et 2520 s'affiche pour la comparaison.
#[test]
#[ignore = "mesure une feuille au FD-9 avec la DLL Konica Minolta du poste"]
fn palier_feuille() {
    let sortie = std::path::PathBuf::from(
        std::env::var("FD9_SORTIE").expect("FD9_SORTIE : dossier local de la mesure"),
    );
    let connecter = format!(r#"{{"cmd":"connecter_adresse","adresse":"{}"}}"#, adresse());
    let mesurer = format!(r#"{{"cmd":"mesurer_feuille","mire":{}}}"#, rgb2520());
    let lignes = dialoguer(
        "feuille",
        &[
            r#"{"cmd":"version"}"#,
            &connecter,
            &mesurer,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    assert_eq!(lignes.len(), 4);
    let Ok(pont_protocole::Reponse::Mesure { mesure, .. }) =
        pont_protocole::lire_reponse(&lignes[2])
    else {
        panic!("{}", lignes[2])
    };
    assert_eq!(mesure.plages().len(), 2_520);
    std::fs::create_dir_all(&sortie).unwrap();
    let nom = format!(
        "fd9-rgb2520-{}.json",
        mesure.provenance().horodatage.texte().replace(':', "-")
    );
    std::fs::write(sortie.join(&nom), pont_protocole::ecrire_mesure(&mesure)).unwrap();
    println!("mesure écrite : {}", sortie.join(&nom).display());
    for n in [1, 42, 2_479, 2_520] {
        let [_, m1, _] = mesure.plages()[n - 1].lab();
        println!("plage {n} : Lab M1 {:?}", m1.valeurs());
    }
    assert!(lignes[3].contains(r#""rep":"ferme""#), "{}", lignes[3]);
}

/// Définition de la RGB-2520 envoyée au pont (nombres de l'étude statique),
/// avec la couleur RVB imprimée sur chaque plage. Les couleurs sont lues
/// dans un CGATS local (`FD9_MIRE_RVB`, colonnes `RGB_R`, `RGB_G`, `RGB_B`
/// en consignes de 0 à 255, par exemple l'export de MYIROtools), dans
/// l'ordre des SAMPLE_ID : jamais versionnées ici.
fn rgb2520() -> String {
    let chemin = std::env::var("FD9_MIRE_RVB")
        .expect("FD9_MIRE_RVB : CGATS local des couleurs RVB de la RGB-2520");
    let octets = std::fs::read(&chemin).unwrap();
    // CGATS en Latin-1 : chaque octet est un caractère.
    let texte: String = octets.iter().map(|&o| char::from(o)).collect();
    let lignes: Vec<&str> = texte.lines().map(str::trim).collect();
    let format = lignes
        .iter()
        .position(|l| *l == "BEGIN_DATA_FORMAT")
        .unwrap();
    let colonnes: Vec<&str> = lignes[format + 1].split_whitespace().collect();
    let place = |nom: &str| colonnes.iter().position(|c| *c == nom).unwrap();
    let rvb = [place("RGB_R"), place("RGB_G"), place("RGB_B")];
    let debut = lignes.iter().position(|l| *l == "BEGIN_DATA").unwrap() + 1;
    let fin = lignes.iter().position(|l| *l == "END_DATA").unwrap();
    let couleurs: Vec<String> = lignes[debut..fin]
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let champs: Vec<&str> = l.split_whitespace().collect();
            let v = rvb.map(|i| {
                let valeur: f64 = champs[i].parse().unwrap();
                assert!(
                    (0.0..=255.0).contains(&valeur) && valeur.fract() == 0.0,
                    "{valeur}"
                );
                valeur as u8
            });
            format!("[{},{},{}]", v[0], v[1], v[2])
        })
        .collect();
    assert_eq!(couleurs.len(), 2_520, "{chemin}");
    format!(
        r#"{{"nom":"RGB-2520 - 1P MY9","largeur":27940,"hauteur":41660,"zones":[{{"gauche":1349,"haut":2300,"largeur_plage":601,"hauteur_plage":601,"pas_x":601,"pas_y":601,"rangees":60,"colonnes":42}}],"couleurs":[{}]}}"#,
        couleurs.join(",")
    )
}
