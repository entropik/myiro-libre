//! Exécutable `pont-fd9` : charge `FD9SDK.dll` et sert le protocole JSON sur
//! l'entrée et la sortie standard (ADR 0005), comme `pont-myiro1`.
//!
//! Usage : `pont-fd9 --dll <chemin de FD9SDK.dll> [--plafond <palier>]
//! [--attente-feuille <secondes>] [--trace]
//! [--vidage <fichier.dmp>]`
//! Paliers déclarés par ce pont : version (par défaut), detection, connexion,
//! prenumerisation, feuille. Tout autre palier est refusé au lancement.
//!
//! Avant chaque feuille, `zxing.dll` doit être à côté de cet exécutable : la
//! DLL le charge depuis ce dossier pour lire le QR (ticket #87). Sinon, la
//! feuille est refusée (`composant_qr_introuvable`), sans appel à la DLL.
//!
//! Diagnostic des essais : un plantage est toujours rapporté sur la sortie
//! d'erreur (code, module et décalage de l'adresse fautive, fil), puis le
//! pont s'arrête avec le code de l'exception (`pont_fd9::plantage`).
//! `--trace` écrit aussi, au fil de l'eau, chaque appel à la DLL avant et
//! après, chaque entrée et sortie du rappel, et les exceptions graves dès
//! leur premier passage. `--vidage` écrit au plantage un vidage mémoire,
//! local : il contient l'adresse et l'identifiant de l'instrument.
//!
//! Codes de sortie : 0 fin normale, 1 erreur de lecture ou d'écriture,
//! 2 arguments refusés, 3 DLL non chargée, 4 fermeture qui n'a pas rendu la
//! main dans `DELAI_FERMETURE` (`CODE_FERMETURE_BLOQUEE` ; la réponse à
//! `fermer` est donnée avant).

use pont_fd9::dll::Fd9Dll;
use pont_fd9::serveur::{servir, Garde, DELAI_FERMETURE};
use pont_fd9::trace::{self, SdkTrace};
use pont_fd9::{evenements, plantage, Delais, SdkFd9, Session};
use pont_protocole::Palier;
use std::io::Write;
use std::path::PathBuf;
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
    trace: bool,
    vidage: Option<PathBuf>,
}

fn arguments() -> Result<Arguments, String> {
    let mut dll = None;
    let mut plafond = Palier::Version;
    let mut delais = Delais::default();
    let mut trace = false;
    let mut vidage = None;
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
                return Err(
                    "--reconnaissance : retiré ; la reconnaissance est toujours COMBINE, avec les couleurs des plages données par la mire (essai E4 bis)"
                        .into(),
                )
            }
            "--trace" => trace = true,
            "--vidage" => {
                let chemin = args.next().unwrap_or_default();
                if chemin.is_empty() {
                    return Err("--vidage <fichier.dmp> : chemin manquant".into());
                }
                vidage = Some(PathBuf::from(chemin));
            }
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok(Arguments {
        dll: dll.ok_or("--dll <chemin de FD9SDK.dll> manquant")?,
        plafond,
        delais,
        trace,
        vidage,
    })
}

/// Écrit une ligne sur la sortie d'erreur sans jamais paniquer.
fn noter(ligne: &str) {
    let _ = writeln!(std::io::stderr(), "pont-fd9 : {ligne}");
}

fn main() -> ExitCode {
    let arguments = match arguments() {
        Ok(a) => a,
        Err(erreur) => {
            noter(&erreur);
            return ExitCode::from(2);
        }
    };
    // Avant le chargement : un plantage dans l'initialisation de la DLL est
    // rapporté lui aussi.
    trace::depuis_le_lancement();
    plantage::installer(arguments.vidage.as_deref(), arguments.trace);
    evenements::tracer(arguments.trace);
    let dll = match Fd9Dll::charger(&arguments.dll) {
        Ok(dll) => dll,
        Err(erreur) => {
            noter(&format!("{erreur:?}"));
            return ExitCode::from(3);
        }
    };
    if arguments.trace {
        noter("trace des appels à la DLL et du rappel active");
        servir_avec(SdkTrace::new(dll, noter), &arguments)
    } else {
        servir_avec(dll, &arguments)
    }
}

fn servir_avec<S: SdkFd9>(sdk: S, arguments: &Arguments) -> ExitCode {
    let trace = arguments.trace;
    // La DLL charge le composant de lecture du QR (`zxing.dll`) depuis le
    // dossier de l'exécutable (ticket #87) : il est vérifié avant chaque
    // feuille. Dossier illisible : le dossier courant, vérifié de même.
    let dossier_du_pont = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| {
            noter("dossier de l'exécutable illisible : composant de lecture du QR cherché dans le dossier courant");
            PathBuf::from(".")
        });
    let mut session = Session::new(sdk, arguments.plafond)
        .avec_delais(arguments.delais)
        .avec_dossier_du_pont(dossier_du_pont)
        // Au fil de l'eau : un pont arrêté de force a déjà tout écrit.
        .avec_echo_journal(move |ligne| {
            if trace {
                noter(&format!("{} {ligne}", trace::entete()));
            } else {
                noter(ligne);
            }
        });
    let resultat = servir(
        &mut session,
        std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        Garde::processus(DELAI_FERMETURE),
    );
    let orphelins = session.rappels_orphelins();
    if orphelins > 0 {
        noter(&format!(
            "{orphelins} rappel(s) de la DLL reçu(s) sans session, ignoré(s)"
        ));
    }
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            noter(&erreur.to_string());
            ExitCode::from(1)
        }
    }
}
