//! Boucle du pont : lit une requête JSON par ligne, la confie à la session et
//! écrit une réponse JSON par ligne (ADR 0005).

use crate::{Connexion, Fermeture, MesurePlage, SdkMyiro1, Session};
use fdx_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, DonneesBrutes, ErreurMesure, ErreurPont, Identite, Info,
    InstrumentDetecte, Lab, Mesure, Plage, Provenance, RemiseAuRepos, Reponse, Requete, Spectre,
};
use std::io::{self, BufRead, Write};

/// Sert les requêtes jusqu'à une fermeture faite ou la fin de l'entrée. Une
/// ligne illisible reçoit `requete_invalide` et ne déclenche rien. À la fin de
/// l'entrée, ou sur une erreur d'écriture, la session est fermée par la même
/// politique que `fermer` (sans réponse, faute de demande) ; une fermeture déjà
/// faite n'est pas refaite.
pub fn servir<S: SdkMyiro1>(
    session: &mut Session<S>,
    entree: impl BufRead,
    sortie: &mut impl Write,
) -> io::Result<()> {
    let resultat = repondre(session, entree, sortie);
    let _ = session.fermer();
    resultat
}

fn repondre<S: SdkMyiro1>(
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
        Requete::Etalonner {} => session.etalonner().map(|_| Reponse::Etalonne {}),
        Requete::MesurerPonctuelle {} => session.mesurer_ponctuelle().and_then(|mesure| {
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
/// retour au repos, observé par le pont, est rendu à côté de la mesure.
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
