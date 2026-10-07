//! Exécutable `pont-myiro1` : charge `FDXSDK.dll` et sert le protocole JSON
//! sur l'entrée et la sortie standard (ADR 0005).
//!
//! Usage : `pont-myiro1 --dll <chemin de FDXSDK.dll> [--plafond <palier>]`
//! Paliers : version (par défaut), detection, connexion, etalonnage,
//! mesure_ponctuelle, bande. Le pont refuse tout palier au-delà du plafond.

use pont_myiro1::dll::FdxDll;
use pont_myiro1::serveur::servir;
use pont_myiro1::Session;
use pont_protocole::Palier;
use std::process::ExitCode;

fn palier(nom: &str) -> Option<Palier> {
    Some(match nom {
        "version" => Palier::Version,
        "detection" => Palier::Detection,
        "connexion" => Palier::Connexion,
        "etalonnage" => Palier::Etalonnage,
        "mesure_ponctuelle" => Palier::MesurePonctuelle,
        "bande" => Palier::Bande,
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
                plafond = palier(&nom).ok_or(format!("palier inconnu : {nom}"))?;
            }
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok((dll.ok_or("--dll <chemin de FDXSDK.dll> manquant")?, plafond))
}

fn main() -> ExitCode {
    let (chemin, plafond) = match arguments() {
        Ok(a) => a,
        Err(erreur) => {
            eprintln!("pont-myiro1 : {erreur}");
            return ExitCode::from(2);
        }
    };
    let dll = match FdxDll::charger(&chemin) {
        Ok(dll) => dll,
        Err(erreur) => {
            eprintln!("pont-myiro1 : {erreur:?}");
            return ExitCode::from(3);
        }
    };
    let mut session = Session::new(dll, plafond);
    let resultat = servir(
        &mut session,
        std::io::stdin().lock(),
        &mut std::io::stdout(),
    );
    // `servir` a déjà fermé la session (désarmement puis déconnexion) ; la
    // DLL ne tente un ultime nettoyage que si la déconnexion a échoué.
    drop(session);
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("pont-myiro1 : {erreur}");
            ExitCode::from(1)
        }
    }
}
