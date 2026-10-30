//! Boucle du pont FD-9 : lit une requête JSON par ligne, la confie à la
//! session et écrit une réponse JSON par ligne (ADR 0005), avec le même
//! protocole que le pont MYIRO-1.

use crate::{Annulation, Fermeture, SdkFd9, Session};
use fd9_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, EffetAnnulation, Empreinte, ErreurPont, Info, InstrumentFd9,
    LiaisonFd9, Palier, RemiseAuRepos, Reponse, Requete,
};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Délai laissé à la fermeture (sortie de la feuille, `FD9_Disconnect`,
/// attente de la fin de session, désinscription du rappel) pour rendre la
/// main. À l'essai E2, `FD9_Disconnect` est resté bloqué environ 5 min. Plus
/// long que la sortie, la déconnexion et l'attente de la fin de session
/// (`ATTENTE_APRES_DECONNEXION`, 10 s) réunies, pour que la garde ne se
/// déclenche pas à tort ; plus court que les 30 s pendant lesquelles
/// l'application attend la réponse à `fermer` : elle la reçoit avant de
/// couper le pont elle-même.
pub const DELAI_FERMETURE: Duration = Duration::from_secs(20);

/// Chien de garde de la fermeture : un fil à part, qui n'appelle jamais la
/// DLL. Si la fermeture ne rend pas la main dans `delai`, il appelle
/// `depassement` avec la réponse due à `fermer` (rien si la fermeture vient
/// de la fin de l'entrée). L'exécutable y écrit la réponse et arrête le
/// processus ; la boucle, elle, n'écrit plus rien ensuite.
pub struct Garde {
    delai: Duration,
    depassement: Box<dyn FnOnce(Option<Reponse>) + Send>,
}

/// Code de sortie du pont quand sa fermeture n'a pas rendu la main à temps.
pub const CODE_FERMETURE_BLOQUEE: u8 = 4;

impl Garde {
    pub fn new(
        delai: Duration,
        depassement: impl FnOnce(Option<Reponse>) + Send + 'static,
    ) -> Self {
        Garde {
            delai,
            depassement: Box::new(depassement),
        }
    }

    /// Garde de l'exécutable : écrit la réponse due à `fermer` sur la sortie
    /// standard, le note au journal (sortie d'erreur), puis arrête le
    /// processus avec `CODE_FERMETURE_BLOQUEE`. Rien n'est appelé dans la DLL.
    pub fn processus(delai: Duration) -> Self {
        Garde::new(delai, move |reponse| {
            if let Some(reponse) = reponse {
                let mut sortie = io::stdout().lock();
                let _ = writeln!(sortie, "{}", ecrire_reponse(&reponse));
                let _ = sortie.flush();
            }
            eprintln!(
                "pont-fd9 : la fermeture n'a pas rendu la main en {} ms (FD9_ExitMeasurement, FD9_Disconnect, FD9_GetStatus ou désinscription du rappel) : arrêt du pont, code {CODE_FERMETURE_BLOQUEE}",
                delai.as_millis()
            );
            terminer(CODE_FERMETURE_BLOQUEE)
        })
    }
}

/// Arrête le processus sur-le-champ. La sortie ordinaire de Windows
/// (`ExitProcess`) prévient chaque DLL chargée ; FD9SDK, bloquée dans un
/// appel, pourrait alors ne jamais rendre la main (supposé). L'arrêt forcé,
/// lui, a terminé le pont à l'essai E2.
#[cfg(windows)]
fn terminer(code: u8) -> ! {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn TerminateProcess(processus: *mut c_void, code: u32) -> i32;
    }
    // SAFETY : pseudo-poignée du processus courant, toujours valable.
    unsafe {
        TerminateProcess(GetCurrentProcess(), code.into());
    }
    std::process::exit(code.into())
}

#[cfg(not(windows))]
fn terminer(code: u8) -> ! {
    std::process::exit(code.into())
}

/// Sert les requêtes jusqu'à `fermer` ou la fin de l'entrée. Une ligne
/// illisible reçoit `requete_invalide` et ne déclenche rien. Les lignes sont
/// lues par un fil à part : `annuler` atteint une feuille en cours sans
/// attendre sa fin (ticket #26). La fin de l'entrée abandonne une feuille pas
/// encore glissée. Puis la session est fermée. Chaque fermeture se fait
/// sous `garde`.
pub fn servir<S: SdkFd9>(
    session: &mut Session<S>,
    entree: impl BufRead + Send + 'static,
    sortie: &mut impl Write,
    garde: Garde,
) -> io::Result<()> {
    let (envoi, lignes) = mpsc::channel();
    let annulation = session.annulation();
    let feuille_active = Arc::new(AtomicBool::new(false));
    let active = feuille_active.clone();
    // Le fil s'arrête de lui-même à la fin de l'entrée ou du processus.
    std::thread::spawn(move || lire(entree, envoi, annulation, active));
    let mut garde = Some(garde);
    let resultat = repondre(session, lignes, sortie, &feuille_active, &mut garde);
    // Fin de l'entrée ou erreur : la même garde, sans réponse à écrire. Après
    // `fermer`, la fermeture est déjà faite et n'est pas refaite.
    if let Some(garde) = garde {
        fermer_sous_garde(session, garde, false);
    }
    resultat
}

/// Ferme la session pendant que le fil de garde compte le délai. Rend `None`
/// si la garde a agi la première : la fermeture a rendu la main trop tard,
/// et la réponse à `fermer` est déjà donnée.
fn fermer_sous_garde<S: SdkFd9>(
    session: &mut Session<S>,
    garde: Garde,
    requete: bool,
) -> Option<Fermeture> {
    // Qui a parlé le premier : la fermeture ou la garde. Le premier l'emporte.
    let tranche = Arc::new(Mutex::new(false));
    let (fin, attente) = mpsc::channel::<()>();
    let tranche_garde = tranche.clone();
    let fil = std::thread::spawn(move || {
        if attente.recv_timeout(garde.delai) != Err(RecvTimeoutError::Timeout) {
            return;
        }
        let mut tranche = tranche_garde.lock().unwrap_or_else(|e| e.into_inner());
        if !*tranche {
            *tranche = true;
            let reponse = requete.then_some(Reponse::FermetureIncertaine {
                remise_au_repos: RemiseAuRepos::DeconnexionBloquee {},
            });
            (garde.depassement)(reponse);
        }
    });
    let fermeture = session.fermer();
    let trop_tard = std::mem::replace(
        &mut *tranche.lock().unwrap_or_else(|e| e.into_inner()),
        true,
    );
    let _ = fin.send(());
    let _ = fil.join();
    if trop_tard {
        session.journal.push(format!(
            "fermeture rendue après le délai : {fermeture:?}, réponse déjà donnée par la garde"
        ));
        return None;
    }
    Some(fermeture)
}

fn est_une_feuille(requete: &Requete) -> bool {
    matches!(
        requete,
        Requete::Prenumeriser { .. } | Requete::MesurerFeuille { .. }
    )
}

/// Fil de lecture : transmet chaque ligne dans l'ordre. Une feuille lue est
/// active jusqu'à sa réponse ; `annuler` lu pendant ce temps la vise.
fn lire(
    entree: impl BufRead,
    envoi: Sender<io::Result<String>>,
    annulation: Annulation,
    feuille_active: Arc<AtomicBool>,
) {
    for ligne in entree.lines() {
        let texte = match ligne {
            Ok(texte) => texte,
            Err(erreur) => {
                annulation.fermer_entree();
                let _ = envoi.send(Err(erreur));
                return;
            }
        };
        if texte.trim().is_empty() {
            continue;
        }
        match lire_requete(&texte) {
            Ok(Requete::Annuler {}) if feuille_active.load(Ordering::SeqCst) => {
                annulation.annuler();
            }
            Ok(requete) if est_une_feuille(&requete) => {
                feuille_active.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
        if envoi.send(Ok(texte)).is_err() {
            return;
        }
    }
    annulation.fermer_entree();
}

fn repondre<S: SdkFd9>(
    session: &mut Session<S>,
    lignes: Receiver<io::Result<String>>,
    sortie: &mut impl Write,
    feuille_active: &AtomicBool,
    garde: &mut Option<Garde>,
) -> io::Result<()> {
    let annulation = session.annulation();
    // La dernière feuille s'est-elle terminée par une annulation ?
    let mut annulee = false;
    for ligne in lignes {
        let ligne = ligne?;
        let (reponse, fin) = match lire_requete(&ligne) {
            Ok(Requete::Annuler {}) => {
                let effet = if std::mem::take(&mut annulee) {
                    EffetAnnulation::Appliquee
                } else {
                    EffetAnnulation::SansEffet
                };
                (Some(Reponse::Annulation { effet }), false)
            }
            Ok(requete) => {
                let feuille = est_une_feuille(&requete);
                let (reponse, fin) = traiter(session, requete, garde);
                if feuille {
                    // Avant la réponse : qui la lit peut aussitôt en envoyer
                    // une autre.
                    feuille_active.store(false, Ordering::SeqCst);
                    annulation.effacer();
                    annulee = matches!(
                        reponse,
                        Some(Reponse::Erreur {
                            erreur: ErreurPont::MesureAnnulee { .. }
                        })
                    );
                }
                (reponse, fin)
            }
            Err(detail) => (Some(Reponse::RequeteInvalide { detail }), false),
        };
        // Sans réponse : la garde a déjà répondu à `fermer`.
        if let Some(reponse) = reponse {
            writeln!(sortie, "{}", ecrire_reponse(&reponse))?;
            sortie.flush()?;
        }
        if fin {
            break;
        }
    }
    Ok(())
}

/// Exécute une requête ; le booléen indique que le pont doit s'arrêter. Pas
/// de réponse si la garde a déjà répondu à `fermer`.
fn traiter<S: SdkFd9>(
    session: &mut Session<S>,
    requete: Requete,
    garde: &mut Option<Garde>,
) -> (Option<Reponse>, bool) {
    let reponse = match requete {
        Requete::Version {} => session.version().map(|v| Reponse::VersionDll {
            version_fichier: v.version_fichier.map_or(Info::Inconnue, Info::Confirmee),
            // Une empreinte mal formée n'est pas une empreinte : inconnue.
            empreinte: v
                .empreinte
                .and_then(|texte| Empreinte::new(texte).ok())
                .map_or(Info::Inconnue, Info::Confirmee),
        }),
        Requete::Detecter {} => session.detecter().map(|appareils| Reponse::InstrumentsFd9 {
            liste: appareils
                .iter()
                .map(|appareil| InstrumentFd9 {
                    // Codes 0 et 1 confirmés ; un autre code reste au journal
                    // de la session et sort inconnu.
                    liaison: match appareil.liaison() {
                        Liaison::Reseau => Info::Confirmee(LiaisonFd9::Reseau),
                        Liaison::Usb => Info::Confirmee(LiaisonFd9::Usb),
                        Liaison::Inconnue(_) => Info::Inconnue,
                    },
                    adresse: appareil.adresse(),
                    // Le texte est celui de la DLL ; qu'il désigne
                    // l'instrument est supposé (fiche FD9_GetDeviceList).
                    identifiant: Some(appareil.identifiant())
                        .filter(|texte| !texte.is_empty())
                        .map_or(Info::Inconnue, Info::Supposee),
                })
                .collect(),
        }),
        Requete::Connecter { instrument } => session
            .connecter(instrument as usize)
            .map(|identite| Reponse::Connecte { identite }),
        Requete::ConnecterAdresse { adresse } => session
            .connecter_adresse(&adresse)
            .map(|identite| Reponse::Connecte { identite }),
        Requete::Etalonner {} => Err(session.hors_paliers(Palier::Etalonnage)),
        Requete::MesurerPonctuelle { .. } => Err(session.hors_paliers(Palier::MesurePonctuelle)),
        Requete::MesurerBande { .. } => Err(session.hors_paliers(Palier::Bande)),
        Requete::Prenumeriser { mire } => {
            session
                .prenumeriser(&mire)
                .map(|lecture| Reponse::Prenumerisee {
                    lecture,
                    calage: session.prendre_calage(),
                })
        }
        // La feuille est close (ExitMeasurement) avant la réponse : rien ne
        // reste armé dans l'instrument.
        Requete::MesurerFeuille { mire } => {
            session
                .mesurer_feuille(&mire)
                .map(|mesure| Reponse::Mesure {
                    mesure,
                    remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
                    calage: session.prendre_calage(),
                })
        }
        // Déconnexion puis désinscription du rappel, sous la garde ; une
        // déconnexion refusée arrête aussi le pont, sans nouvel essai.
        Requete::Fermer {} => {
            let fermeture = match garde.take() {
                Some(garde) => fermer_sous_garde(session, garde, true),
                None => Some(session.fermer()),
            };
            let reponse = fermeture.map(|fermeture| match fermeture {
                Fermeture::Confirmee => Reponse::Ferme {},
                Fermeture::Incertaine { remise_au_repos } => {
                    Reponse::FermetureIncertaine { remise_au_repos }
                }
            });
            return (reponse, true);
        }
        // Traitée par la boucle, qui sait si la feuille précédente a été
        // interrompue ; seule, elle n'a rien à interrompre.
        Requete::Annuler {} => Ok(Reponse::Annulation {
            effet: EffetAnnulation::SansEffet,
        }),
    };
    (
        Some(reponse.unwrap_or_else(|erreur| Reponse::Erreur { erreur })),
        false,
    )
}
