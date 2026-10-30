//! Le pont FD-9 en vrai processus, contre le FD-9 simulé des tests
//! (`tests/commun`) au lieu de la DLL : il sert aux tests de l'exécutable.
//!
//! Usage : `pont_fd9_simule <palier> [--deconnexion-bloquee <ms>]` (palier :
//! connexion, prenumerisation, feuille). Le journal part sur la sortie
//! d'erreur au fil de l'eau, comme dans `pont-fd9`. Après le dialogue, chaque
//! appel reçu par le simulé y est écrit aussi, une ligne `appel : …` par
//! appel. Avec `--deconnexion-bloquee`, `FD9_Disconnect` ne rend pas la main
//! (10 s au plus) et la garde de l'exécutable agit après `<ms>`.

#[path = "../tests/commun/mod.rs"]
mod commun;

use pont_fd9::serveur::{servir, Garde, DELAI_FERMETURE};
use pont_fd9::Session;
use pont_protocole::Palier;
use std::process::ExitCode;
use std::time::Duration;

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
    let mut sdk = commun::Fd9Simule::avec_une_rgb2520();
    let mut delai = DELAI_FERMETURE;
    // Gardé jusqu'à la fin : sans lui, la déconnexion retenue rendrait la main.
    let (_retenir, retenue) = std::sync::mpsc::channel();
    if std::env::args().nth(2).as_deref() == Some("--deconnexion-bloquee") {
        let Some(ms) = std::env::args().nth(3).and_then(|t| t.parse().ok()) else {
            eprintln!("pont_fd9_simule : --deconnexion-bloquee <ms>");
            return ExitCode::from(2);
        };
        delai = Duration::from_millis(ms);
        sdk.deconnexion_retenue = Some(retenue);
    }
    let mut session =
        Session::new(sdk, palier).avec_echo_journal(|ligne| eprintln!("pont-fd9 : {ligne}"));
    let resultat = servir(
        &mut session,
        std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        Garde::processus(delai),
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
