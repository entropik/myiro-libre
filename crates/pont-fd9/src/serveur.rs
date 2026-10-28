//! Boucle du pont FD-9 : lit une requête JSON par ligne, la confie à la
//! session et écrit une réponse JSON par ligne (ADR 0005), avec le même
//! protocole que le pont MYIRO-1.

use crate::{Annulation, Fermeture, SdkFd9, Session};
use fd9_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, EffetAnnulation, Empreinte, ErreurPont, Info, InstrumentFd9,
    LiaisonFd9, Palier, Reponse, Requete,
};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

/// Sert les requêtes jusqu'à `fermer` ou la fin de l'entrée. Une ligne
/// illisible reçoit `requete_invalide` et ne déclenche rien. Les lignes sont
/// lues par un fil à part : `annuler` atteint une feuille en cours sans
/// attendre sa fin (ticket #26). La fin de l'entrée abandonne une feuille pas
/// encore glissée. Puis la session est fermée.
pub fn servir<S: SdkFd9>(
    session: &mut Session<S>,
    entree: impl BufRead + Send + 'static,
    sortie: &mut impl Write,
) -> io::Result<()> {
    let (envoi, lignes) = mpsc::channel();
    let annulation = session.annulation();
    let feuille_active = Arc::new(AtomicBool::new(false));
    let active = feuille_active.clone();
    // Le fil s'arrête de lui-même à la fin de l'entrée ou du processus.
    std::thread::spawn(move || lire(entree, envoi, annulation, active));
    let resultat = repondre(session, lignes, sortie, &feuille_active);
    session.fermer();
    resultat
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
                (Reponse::Annulation { effet }, false)
            }
            Ok(requete) => {
                let feuille = est_une_feuille(&requete);
                let (reponse, fin) = traiter(session, requete);
                if feuille {
                    // Avant la réponse : qui la lit peut aussitôt en envoyer
                    // une autre.
                    feuille_active.store(false, Ordering::SeqCst);
                    annulation.effacer();
                    annulee = matches!(
                        reponse,
                        Reponse::Erreur {
                            erreur: ErreurPont::MesureAnnulee { .. }
                        }
                    );
                }
                (reponse, fin)
            }
            Err(detail) => (Reponse::RequeteInvalide { detail }, false),
        };
        writeln!(sortie, "{}", ecrire_reponse(&reponse))?;
        sortie.flush()?;
        if fin {
            break;
        }
    }
    Ok(())
}

/// Exécute une requête ; le booléen indique que le pont doit s'arrêter.
fn traiter<S: SdkFd9>(session: &mut Session<S>, requete: Requete) -> (Reponse, bool) {
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
        Requete::Prenumeriser { mire } => session
            .prenumeriser(&mire)
            .map(|lecture| Reponse::Prenumerisee { lecture }),
        Requete::MesurerFeuille { .. } => Err(session.hors_paliers(Palier::Feuille)),
        // Déconnexion puis désinscription du rappel ; une déconnexion
        // refusée arrête aussi le pont, sans nouvel essai.
        Requete::Fermer {} => {
            let reponse = match session.fermer() {
                Fermeture::Confirmee => Reponse::Ferme {},
                Fermeture::Incertaine { remise_au_repos } => {
                    Reponse::FermetureIncertaine { remise_au_repos }
                }
            };
            return (reponse, true);
        }
        // Traitée par la boucle, qui sait si la feuille précédente a été
        // interrompue ; seule, elle n'a rien à interrompre.
        Requete::Annuler {} => Ok(Reponse::Annulation {
            effet: EffetAnnulation::SansEffet,
        }),
    };
    (
        reponse.unwrap_or_else(|erreur| Reponse::Erreur { erreur }),
        false,
    )
}
