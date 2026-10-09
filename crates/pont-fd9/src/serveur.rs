//! Boucle du pont FD-9 : lit une requête JSON par ligne, la confie à la
//! session et écrit une réponse JSON par ligne (ADR 0005), avec le même
//! protocole que le pont MYIRO-1.

use crate::{SdkFd9, Session};
use fd9_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, Empreinte, Info, InstrumentFd9, LiaisonFd9, Palier, Reponse,
    Requete,
};
use std::io::{self, BufRead, Write};

/// Sert les requêtes jusqu'à `fermer` ou la fin de l'entrée. Une ligne
/// illisible reçoit `requete_invalide` et ne déclenche rien.
pub fn servir<S: SdkFd9>(
    session: &mut Session<S>,
    entree: impl BufRead,
    sortie: &mut impl Write,
) -> io::Result<()> {
    for ligne in entree.lines() {
        let ligne = ligne?;
        if ligne.trim().is_empty() {
            continue;
        }
        let (reponse, fin) = match lire_requete(&ligne) {
            Ok(requete) => traiter(session, requete),
            Err(detail) => (Reponse::RequeteInvalide { detail }, false),
        };
        writeln!(sortie, "{}", ecrire_reponse(&reponse))?;
        sortie.flush()?;
        if fin {
            break;
        }
    }
    session.fermer();
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
        Requete::Connecter { .. } | Requete::ConnecterAdresse { .. } => {
            Err(session.hors_paliers(Palier::Connexion))
        }
        Requete::Etalonner {} => Err(session.hors_paliers(Palier::Etalonnage)),
        Requete::MesurerPonctuelle {} => Err(session.hors_paliers(Palier::MesurePonctuelle)),
        Requete::MesurerBande { .. } => Err(session.hors_paliers(Palier::Bande)),
        // Aucune session n'est ouverte avec l'instrument : rien à désarmer ni
        // à déconnecter, la fermeture est confirmée.
        Requete::Fermer {} => {
            session.fermer();
            return (Reponse::Ferme {}, true);
        }
    };
    (
        reponse.unwrap_or_else(|erreur| Reponse::Erreur { erreur }),
        false,
    )
}
