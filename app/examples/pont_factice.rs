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
//! - `sortie_fermee` : lit une requête, ferme sa sortie et ne se termine pas ;
//! - `mesure_lente` : répond à une mesure ponctuelle après 800 ms ;
//! - `mesure_muette` : un MYIRO-1 fictif qui s'étalonne, puis ne répond jamais à la mesure ;
//! - `annulable` : comme `normal`, mais la mesure attend `annuler`, puis
//!   répond `mesure_annulee` et `annulation appliquee` (ticket #26) ;
//! - `resultat_tardif` : la mesure attend `annuler`, puis rend son résultat
//!   (un délai) et `annulation sans_effet`.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use pont_protocole::{
    ecrire_reponse, lire_requete, EffetAnnulation, ErreurPont, Horodatage, Identite,
    InstrumentDetecte, RemiseAuRepos, Reponse, Requete,
};

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
    let mut lignes = stdin.lock().lines();
    while let Some(ligne) = lignes.next() {
        let Ok(ligne) = ligne else { break };
        let requete = lire_requete(&ligne);
        let reponse = match scenario.as_str() {
            // La mesure attend la demande suivante : `annuler` l'interrompt.
            // Réponse de la mesure, puis celle de `annuler`, dans l'ordre.
            "annulable" | "resultat_tardif"
                if matches!(requete, Ok(Requete::MesurerPonctuelle { .. })) =>
            {
                let suivante = lignes.next().and_then(Result::ok).unwrap_or_default();
                assert_eq!(lire_requete(&suivante), Ok(Requete::Annuler {}));
                let (mesure, effet) = if scenario == "annulable" {
                    (
                        ErreurPont::MesureAnnulee {
                            remise_au_repos: RemiseAuRepos::AuRepos {},
                        },
                        EffetAnnulation::Appliquee,
                    )
                } else {
                    // La mesure s'est terminée (ici, par le délai) juste avant.
                    (ErreurPont::Delai {}, EffetAnnulation::SansEffet)
                };
                writeln!(
                    sortie,
                    "{}",
                    ecrire_reponse(&Reponse::Erreur { erreur: mesure })
                )
                .unwrap();
                Reponse::Annulation { effet }
            }
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
                Reponse::Etalonne {
                    date: Horodatage::new("2026-10-07T09:30:00+02:00").unwrap(),
                }
            }
            // Plus long que le double du délai : l'application doit couper.
            "etalonnage_trop_lent" if matches!(requete, Ok(Requete::Etalonner {})) => loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            },
            // Mesure plus longue que trois fois le délai d'une autre demande
            // (200 ms dans le test), mais moins que six fois ; l'instrument
            // fictif n'a pas vu d'appui sur son bouton.
            "mesure_lente" if matches!(requete, Ok(Requete::MesurerPonctuelle { .. })) => {
                std::thread::sleep(std::time::Duration::from_millis(800));
                Reponse::Erreur {
                    erreur: ErreurPont::Delai {},
                }
            }
            // MYIRO-1 fictif qui s'étalonne, puis ne répond jamais à la mesure.
            "mesure_muette" if matches!(requete, Ok(Requete::Etalonner {})) => Reponse::Etalonne {
                date: Horodatage::new("2026-10-07T09:30:00+02:00").unwrap(),
            },
            "mesure_muette" if matches!(requete, Ok(Requete::MesurerPonctuelle { .. })) => loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            },
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
