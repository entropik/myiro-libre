//! Le pont FD-9 en vrai processus, contre le FD-9 simulé des tests
//! (`tests/commun`) au lieu de la DLL : il sert aux tests de l'exécutable.
//!
//! Usage : `pont_fd9_simule <palier>` (connexion, prenumerisation, feuille).
//! Après le dialogue, chaque appel reçu par le simulé est écrit sur la sortie
//! d'erreur, une ligne `appel : …` par appel.

#[path = "../tests/commun/mod.rs"]
mod commun;

use pont_fd9::serveur::servir;
use pont_fd9::Session;
use pont_protocole::Palier;
use std::process::ExitCode;

fn main() -> ExitCode {
    let palier = match std::env::args().nth(1).as_deref() {
        Some("connexion") => Palier::Connexion,
        Some("prenumerisation") => Palier::Prenumerisation,
        Some("feuille") => Palier::Feuille,
        autre => {
            eprintln!("pont_fd9_simule : palier {autre:?} (connexion, prenumerisation, feuille)");
            return ExitCode::from(2);
        }
    };
    let mut session = Session::new(commun::Fd9Simule::avec_une_rgb2520(), palier);
    let resultat = servir(
        &mut session,
        std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
    );
    for appel in &session.sdk().appels {
        eprintln!("appel : {appel}");
    }
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("pont_fd9_simule : {erreur}");
            ExitCode::from(1)
        }
    }
}
