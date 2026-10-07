//! Exécutable `pont-fd9` : charge `FD9SDK.dll` et sert le protocole JSON sur
//! l'entrée et la sortie standard (ADR 0005), comme `pont-myiro1`.
//!
//! Usage : `pont-fd9 --dll <chemin de FD9SDK.dll> [--plafond <palier>]`
//! Paliers déclarés par ce pont : version (par défaut), detection. Tout autre
//! palier est refusé au lancement.

use pont_fd9::dll::Fd9Dll;
use pont_fd9::serveur::servir;
use pont_fd9::Session;
use pont_protocole::Palier;
use std::process::ExitCode;

fn palier(nom: &str) -> Option<Palier> {
    Some(match nom {
        "version" => Palier::Version,
        "detection" => Palier::Detection,
        _ => return None,
    })
}

fn arguments() -> Result<(String, Palier), String> {
    let mut dll = None;
    let mut plafond = Palier::Version;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dll" => dll = args.next(),
            "--plafond" => {
                let nom = args.next().unwrap_or_default();
                plafond = palier(&nom).ok_or(format!(
                    "palier non déclaré par le pont FD-9 : {nom} (version ou detection)"
                ))?;
            }
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok((dll.ok_or("--dll <chemin de FD9SDK.dll> manquant")?, plafond))
}

fn main() -> ExitCode {
    let (chemin, plafond) = match arguments() {
        Ok(a) => a,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur}");
            return ExitCode::from(2);
        }
    };
    let dll = match Fd9Dll::charger(&chemin) {
        Ok(dll) => dll,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur:?}");
            return ExitCode::from(3);
        }
    };
    let mut session = Session::new(dll, plafond);
    let resultat = servir(
        &mut session,
        std::io::stdin().lock(),
        &mut std::io::stdout(),
    );
    for ligne in session.journal() {
        eprintln!("pont-fd9 : {ligne}");
    }
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur}");
            ExitCode::from(1)
        }
    }
}
