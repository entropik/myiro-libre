//! Pont factice pour les tests de `PontProcessus` : il se lance comme
//! `pont-myiro1 --dll <chemin> --plafond <palier>`, mais ne charge aucune DLL.
//! Le « chemin de la DLL » choisit le scénario :
//!
//! - `normal` : un MYIRO-1 fictif (n° 12345678), version, détection, connexion ;
//! - `echo` : répond à tout par `requete_invalide` en recopiant ses arguments ;
//! - `dll_refusee` : s'arrête comme `pont-myiro1` devant une DLL introuvable ;
//! - `muet` : lit une requête puis s'arrête avec le code 1 ;
//! - `illisible` : répond par une ligne hors protocole ;
//! - `erreur_inconnue` : répond par une erreur d'un type inconnu du protocole ;
//! - `bloque` : lit une requête et ne répond jamais (DLL bloquée) ;
//! - `sortie_fermee` : lit une requête, ferme sa sortie et ne se termine pas.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use pont_protocole::{ecrire_reponse, lire_requete, Identite, InstrumentDetecte, Reponse, Requete};

/// Ferme la sortie standard sans terminer le processus.
#[cfg(windows)]
fn fermer_sortie() {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    let poignee = std::io::stdout().as_raw_handle();
    // SAFETY : la poignée de la sortie standard est valide ; la refermer est
    // le but du scénario, et plus rien n'écrit dessus ensuite.
    drop(unsafe { std::fs::File::from_raw_handle(poignee) });
}

#[cfg(not(windows))]
fn fermer_sortie() {
    use std::os::fd::FromRawFd;
    // SAFETY : descripteur 1 valide, refermé exprès ; plus rien n'écrit dessus.
    drop(unsafe { std::fs::File::from_raw_fd(1) });
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scenario = args
        .windows(2)
        .find(|a| a[0] == "--dll")
        .map(|a| a[1].clone())
        .unwrap_or_default();
    if scenario == "dll_refusee" {
        eprintln!("pont-myiro1 : DllIntrouvable(\"{scenario}\")");
        return ExitCode::from(3);
    }
    let stdin = std::io::stdin();
    let mut sortie = std::io::stdout();
    for ligne in stdin.lock().lines() {
        let Ok(ligne) = ligne else { break };
        let requete = lire_requete(&ligne);
        let reponse = match scenario.as_str() {
            "muet" => {
                eprintln!("arrêt simulé");
                return ExitCode::from(1);
            }
            "bloque" => loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            },
            "sortie_fermee" => {
                fermer_sortie();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
            }
            "erreur_inconnue" => {
                // Erreur d'un protocole plus récent, inconnue de l'application.
                writeln!(
                    sortie,
                    r#"{{"rep":"erreur","erreur":{{"type":"panne_future"}}}}"#
                )
                .unwrap();
                sortie.flush().unwrap();
                continue;
            }
            "illisible" => {
                writeln!(sortie, "pas du JSON").unwrap();
                continue;
            }
            // Étalonnage plus long que le délai d'une autre demande (300 ms
            // dans le test), mais moins que le double.
            "etalonnage_lent" if matches!(requete, Ok(Requete::Etalonner {})) => {
                std::thread::sleep(std::time::Duration::from_millis(450));
                Reponse::Etalonne {}
            }
            "echo" => Reponse::RequeteInvalide {
                detail: args.join(" "),
            },
            _ => match requete {
                Ok(Requete::Version {}) => Reponse::Version { parties: [1, 0, 1] },
                Ok(Requete::Detecter {}) => Reponse::Instruments {
                    liste: vec![InstrumentDetecte {
                        liaison: "usb".into(),
                        port: "COM3".into(),
                        numero_serie: 12345678,
                    }],
                },
                Ok(Requete::Connecter { .. }) => Reponse::Connecte {
                    identite: Identite {
                        numero_serie: 12345678,
                        micrologiciel: "1.00".into(),
                        code_produit: "factice".into(),
                        adresse_mac: "00:00:00:00:00:00".into(),
                        date_initiale: None,
                        anomalie_date_initiale: false,
                        brute_hex: String::new(),
                    },
                },
                Ok(Requete::Fermer {}) => {
                    writeln!(sortie, "{}", ecrire_reponse(&Reponse::Ferme {})).unwrap();
                    return ExitCode::SUCCESS;
                }
                Ok(_) => Reponse::RequeteInvalide {
                    detail: "hors du scénario".into(),
                },
                Err(detail) => Reponse::RequeteInvalide { detail },
            },
        };
        writeln!(sortie, "{}", ecrire_reponse(&reponse)).unwrap();
        sortie.flush().unwrap();
    }
    ExitCode::SUCCESS
}
