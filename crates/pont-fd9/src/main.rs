//! Exécutable `pont-fd9` : charge `FD9SDK.dll` et sert le protocole JSON sur
//! l'entrée et la sortie standard (ADR 0005), comme `pont-myiro1`.
//!
//! Usage : `pont-fd9 --dll <chemin de FD9SDK.dll> [--plafond <palier>]
//! [--attente-feuille <secondes>] [--reconnaissance combine|coordinate]`
//! Paliers déclarés par ce pont : version (par défaut), detection, connexion,
//! prenumerisation, feuille. Tout autre palier est refusé au lancement.

use fd9_sys::ModeReconnaissance;
use pont_fd9::dll::Fd9Dll;
use pont_fd9::serveur::servir;
use pont_fd9::{Delais, Session};
use pont_protocole::Palier;
use std::process::ExitCode;
use std::time::Duration;

fn palier(nom: &str) -> Option<Palier> {
    Some(match nom {
        "version" => Palier::Version,
        "detection" => Palier::Detection,
        "connexion" => Palier::Connexion,
        "prenumerisation" => Palier::Prenumerisation,
        "feuille" => Palier::Feuille,
        _ => return None,
    })
}

struct Arguments {
    dll: String,
    plafond: Palier,
    delais: Delais,
    reconnaissance: ModeReconnaissance,
}

fn arguments() -> Result<Arguments, String> {
    let mut dll = None;
    let mut plafond = Palier::Version;
    let mut delais = Delais::default();
    let mut reconnaissance = ModeReconnaissance::Combine;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dll" => dll = args.next(),
            "--plafond" => {
                let nom = args.next().unwrap_or_default();
                plafond = palier(&nom).ok_or(format!(
                    "palier non déclaré par le pont FD-9 : {nom} (version, detection, connexion, prenumerisation ou feuille)"
                ))?;
            }
            "--attente-feuille" => {
                let texte = args.next().unwrap_or_default();
                let secondes: u64 = texte.parse().map_err(|_| {
                    format!("--attente-feuille : « {texte} » n'est pas un nombre de secondes")
                })?;
                delais.feuille = Duration::from_secs(secondes);
            }
            "--reconnaissance" => {
                reconnaissance = match args.next().unwrap_or_default().as_str() {
                    "combine" => ModeReconnaissance::Combine,
                    "coordinate" => ModeReconnaissance::Coordonnees,
                    autre => {
                        return Err(format!(
                            "--reconnaissance : « {autre} » (combine ou coordinate)"
                        ))
                    }
                };
            }
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok(Arguments {
        dll: dll.ok_or("--dll <chemin de FD9SDK.dll> manquant")?,
        plafond,
        delais,
        reconnaissance,
    })
}

fn main() -> ExitCode {
    let arguments = match arguments() {
        Ok(a) => a,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur}");
            return ExitCode::from(2);
        }
    };
    let dll = match Fd9Dll::charger(&arguments.dll) {
        Ok(dll) => dll,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur:?}");
            return ExitCode::from(3);
        }
    };
    let mut session = Session::new(dll, arguments.plafond)
        .avec_delais(arguments.delais)
        .avec_reconnaissance(arguments.reconnaissance);
    let resultat = servir(
        &mut session,
        std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
    );
    for ligne in session.journal() {
        eprintln!("pont-fd9 : {ligne}");
    }
    let orphelins = session.rappels_orphelins();
    if orphelins > 0 {
        eprintln!("pont-fd9 : {orphelins} rappel(s) de la DLL reçu(s) sans session, ignoré(s)");
    }
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("pont-fd9 : {erreur}");
            ExitCode::from(1)
        }
    }
}
