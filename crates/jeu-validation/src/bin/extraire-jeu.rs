//! Produit le jeu de validation en local à partir des sorties du pont archivées :
//!
//! ```text
//! extraire-jeu <jeu.json> <sortie du pont>...
//! ```
//!
//! Le jeu contient des mesures réelles : il reste hors de git (par exemple dans
//! `Archivage/donnees/`), même sans identifiant d'instrument.

use std::path::Path;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() < 2 {
        eprintln!("usage : extraire-jeu <jeu.json> <sortie du pont>...");
        std::process::exit(2);
    }
    let (destination, entrees) = (&arguments[0], &arguments[1..]);
    let mut textes = Vec::new();
    for chemin in entrees {
        let nom = Path::new(chemin)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| chemin.clone());
        match std::fs::read_to_string(chemin) {
            Ok(contenu) => textes.push((nom, contenu)),
            Err(erreur) => echouer(&format!("{chemin} : {erreur}")),
        }
    }
    let sorties: Vec<_> = textes
        .iter()
        .map(|(nom, contenu)| jeu_validation::SortieArchivee { nom, contenu })
        .collect();
    let jeu = jeu_validation::extraire(&sorties).unwrap_or_else(|e| echouer(&e));
    let texte = serde_json::to_string_pretty(&jeu).expect("le jeu se sérialise toujours");
    if let Err(erreur) = std::fs::write(destination, texte) {
        echouer(&format!("{destination} : {erreur}"));
    }
    // Correspondance affichée à l'écran seulement : le jeu ne garde que le numéro.
    for (k, (nom, _)) in textes.iter().enumerate() {
        println!("sortie-{} : {nom}", k + 1);
    }
    println!(
        "{} paires extraites de {} sorties vers {destination}",
        jeu.paires.len(),
        sorties.len()
    );
}

fn echouer(message: &str) -> ! {
    eprintln!("extraire-jeu : {message}");
    std::process::exit(1);
}
