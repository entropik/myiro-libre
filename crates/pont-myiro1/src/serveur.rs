//! Boucle du pont : lit une requête JSON par ligne, la confie à la session et
//! écrit une réponse JSON par ligne (ADR 0005).
//!
//! Les lignes sont lues par un fil à part, qui ne bloque jamais sur
//! l'instrument : une demande `annuler` est vue pendant qu'une mesure attend
//! (ticket #26). Les appels à la DLL restent tous faits par le fil principal,
//! une demande après l'autre, et chaque demande reçoit une seule réponse, dans
//! l'ordre d'arrivée.

use crate::{Annulation, Connexion, Fermeture, MesurePlage, SdkMyiro1, Session};
use fdx_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, DonneesBrutes, EffetAnnulation, ErreurMesure, ErreurPont,
    Identite, Info, InstrumentDetecte, Lab, Mesure, Plage, Provenance, RemiseAuRepos, Reponse,
    Requete, Spectre,
};
use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, Receiver, Sender};

/// Une ligne lue, avec son numéro d'ordre parmi les demandes.
type LigneNumerotee = (u64, io::Result<String>);

/// Sert les requêtes jusqu'à une fermeture faite ou la fin de l'entrée. Une
/// ligne illisible reçoit `requete_invalide` et ne déclenche rien. `annuler`
/// interrompt la demande qui la précède si elle attend encore l'instrument.
/// À la fin de l'entrée, la mesure en attente est annulée de la même façon ;
/// puis, comme sur une erreur d'écriture, la session est fermée par la même
/// politique que `fermer` (sans réponse, faute de demande) ; une fermeture déjà
/// faite n'est pas refaite.
pub fn servir<S: SdkMyiro1>(
    session: &mut Session<S>,
    entree: impl BufRead + Send + 'static,
    sortie: &mut impl Write,
) -> io::Result<()> {
    let (envoi, lignes) = mpsc::channel();
    let annulation = session.annulation();
    // Les demandes sont numérotées à partir de 1 pour cette entrée-ci.
    annulation.effacer();
    // Le fil de lecture s'arrête de lui-même à la fin de l'entrée, ou à la fin
    // du processus.
    std::thread::spawn(move || lire(entree, envoi, annulation));
    let resultat = repondre(session, lignes, sortie);
    // Sans demande, pas de réponse à écrire : `fermer` note son résultat au
    // journal de la session.
    let _ = session.fermer();
    resultat
}

/// Fil de lecture : numérote les demandes et transmet chaque ligne. `annuler`
/// vise aussitôt la dernière demande qui n'est pas elle-même une annulation ;
/// la fin de l'entrée aussi, mais seulement si cette demande attend
/// l'opérateur (une entrée lue d'un bloc puis fermée n'annule rien qui part).
fn lire(entree: impl BufRead, envoi: Sender<LigneNumerotee>, annulation: Annulation) {
    let mut numero = 0;
    let mut derniere = 0;
    for ligne in entree.lines() {
        let erreur = ligne.is_err();
        if let Ok(texte) = &ligne {
            if texte.trim().is_empty() {
                continue;
            }
            numero += 1;
            if lire_requete(texte) == Ok(Requete::Annuler {}) {
                annulation.viser(derniere);
            } else {
                derniere = numero;
            }
        }
        if envoi.send((numero, ligne)).is_err() || erreur {
            return;
        }
    }
    annulation.viser_si_elle_attend(derniere);
}

fn repondre<S: SdkMyiro1>(
    session: &mut Session<S>,
    lignes: Receiver<LigneNumerotee>,
    sortie: &mut impl Write,
) -> io::Result<()> {
    let annulation = session.annulation();
    // La dernière demande (hors annulation) s'est terminée par une annulation.
    let mut annulee = false;
    for (numero, ligne) in lignes {
        let ligne = ligne?;
        annulation.commencer(numero);
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
                let (reponse, fin) = traiter(session, requete);
                annulee = matches!(
                    reponse,
                    Reponse::Erreur {
                        erreur: ErreurPont::MesureAnnulee { .. }
                    }
                );
                (reponse, fin)
            }
            Err(detail) => {
                annulee = false;
                (Reponse::RequeteInvalide { detail }, false)
            }
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
fn traiter<S: SdkMyiro1>(session: &mut Session<S>, requete: Requete) -> (Reponse, bool) {
    let reponse = match requete {
        Requete::Version {} => session.version().map(|v| Reponse::Version {
            parties: [v.partie0, v.partie1, v.partie2],
        }),
        Requete::Detecter {} => session.detecter().map(|ports| Reponse::Instruments {
            liste: ports
                .iter()
                .map(|port| InstrumentDetecte {
                    liaison: match port.liaison() {
                        Liaison::Usb => "usb".into(),
                        Liaison::Reseau => "reseau".into(),
                        Liaison::Inconnue(code) => format!("inconnue {code}"),
                    },
                    port: port.nom(),
                    numero_serie: port.numero_serie(),
                })
                .collect(),
        }),
        Requete::Connecter { instrument } => {
            session
                .connecter(instrument as usize)
                .map(|connexion| Reponse::Connecte {
                    identite: identite(&connexion),
                })
        }
        // Paramètre de connexion du FD-9 : le MYIRO-1 n'a pas de connexion
        // par adresse. Rien n'est transmis à la DLL.
        Requete::ConnecterAdresse { .. } => {
            return (
                Reponse::RequeteInvalide {
                    detail: "connecter_adresse : commande du FD-9, inconnue du pont MYIRO-1".into(),
                },
                false,
            )
        }
        Requete::Etalonner {} => session.etalonner().map(|etalonnage| Reponse::Etalonne {
            date: etalonnage.date,
        }),
        Requete::MesurerPonctuelle { declenchement } => session
            .mesurer_ponctuelle_avec(declenchement)
            .and_then(|mesure| {
                let plage = MesurePlage {
                    m0: mesure.m0,
                    m1: mesure.m1,
                    m2: mesure.m2,
                    brutes: mesure.brutes,
                    lab_dll: mesure.lab_dll,
                };
                reponse_mesure(vec![plage], mesure.provenance, mesure.remise_au_repos)
            }),
        Requete::MesurerBande { plages_attendues } => {
            session.mesurer_bande(plages_attendues).and_then(|bande| {
                reponse_mesure(bande.plages, bande.provenance, bande.remise_au_repos)
            })
        }
        // La réponse suit la déconnexion effective ; un échec de celle-ci
        // laisse le pont à l'écoute d'un nouveau `fermer`.
        Requete::Fermer {} => match session.fermer() {
            Ok(Fermeture::Confirmee) => return (Reponse::Ferme {}, true),
            Ok(Fermeture::Incertaine { remise_au_repos }) => {
                return (Reponse::FermetureIncertaine { remise_au_repos }, true)
            }
            Err(erreur) => Err(erreur),
        },
        // Traitée par la boucle, qui sait si la demande précédente a été
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

fn identite(connexion: &Connexion) -> Identite {
    let infos = &connexion.infos;
    Identite {
        numero_serie: infos.numero,
        micrologiciel: infos.micrologiciel.clone(),
        code_produit: infos.code_produit.clone(),
        adresse_mac: infos.adresse_mac.clone(),
        date_initiale: infos.date_initiale,
        anomalie_date_initiale: connexion.anomalie_date_initiale,
        brute_hex: connexion
            .identite_brute
            .iter()
            .map(|octet| format!("{octet:02x}"))
            .collect(),
    }
}

/// Vérifie la mesure avant qu'elle ne sorte du pont : une valeur non finie
/// ou une forme imprévue de la DLL devient une erreur, jamais une mesure. Le
/// résultat de la remise au repos, observé par le pont, est rendu à côté.
fn reponse_mesure(
    plages: Vec<MesurePlage>,
    provenance: Provenance,
    remise_au_repos: RemiseAuRepos,
) -> Result<Reponse, ErreurPont> {
    let mesure = plages
        .into_iter()
        .map(plage)
        .collect::<Result<Vec<_>, _>>()
        .and_then(|plages| Mesure::new(plages, provenance))
        .map_err(|erreur| ErreurPont::ReponseInattendue {
            detail: erreur.to_string(),
        })?;
    Ok(Reponse::Mesure {
        mesure,
        remise_au_repos: Info::Confirmee(remise_au_repos),
    })
}

fn plage(mesure: MesurePlage) -> Result<Plage, ErreurMesure> {
    let [lab_m0, lab_m1, lab_m2] = mesure.lab_dll.map(Lab::try_from);
    Plage::new(
        [
            Spectre::new(mesure.m0)?,
            Spectre::new(mesure.m1)?,
            Spectre::new(mesure.m2)?,
        ],
        DonneesBrutes::new(mesure.brutes)?,
        [lab_m0?, lab_m1?, lab_m2?],
    )
}
