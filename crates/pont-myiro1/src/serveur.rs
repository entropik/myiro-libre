//! Boucle du pont : lit une requête JSON par ligne, la confie à la session et
//! écrit une réponse JSON par ligne (ADR 0005).

use crate::{Connexion, MesurePlage, SdkMyiro1, Session};
use fdx_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, Identite, InstrumentDetecte, Plage, Reponse, Requete,
};
use std::io::{self, BufRead, Write};

/// Sert les requêtes jusqu'à `fermer` ou la fin de l'entrée. Une ligne illisible
/// reçoit `requete_invalide` et ne déclenche rien.
pub fn servir<S: SdkMyiro1>(
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
    let reponse =
        match requete {
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
            Requete::MesurerPonctuelle {} => {
                session.mesurer_ponctuelle().map(|mesure| Reponse::Mesure {
                    plages: vec![Plage {
                        m0: mesure.m0,
                        m1: mesure.m1,
                        m2: mesure.m2,
                        brutes: mesure.brutes,
                        lab_m0: mesure.lab_dll[0].clone(),
                        lab_m1: mesure.lab_dll[1].clone(),
                        lab_m2: mesure.lab_dll[2].clone(),
                    }],
                    sens: 0,
                    provenance: mesure.provenance,
                })
            }
            Requete::MesurerBande { plages_attendues } => session
                .mesurer_bande(plages_attendues)
                .map(|bande| Reponse::Mesure {
                    plages: bande.plages.into_iter().map(plage).collect(),
                    sens: bande.sens,
                    provenance: bande.provenance,
                }),
            // La déconnexion elle-même a lieu à la fermeture de l'adapter.
            Requete::Fermer {} => return (Reponse::Ferme {}, true),
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

fn plage(mesure: MesurePlage) -> Plage {
    let [lab_m0, lab_m1, lab_m2] = mesure.lab_dll;
    Plage {
        m0: mesure.m0,
        m1: mesure.m1,
        m2: mesure.m2,
        brutes: mesure.brutes,
        lab_m0,
        lab_m1,
        lab_m2,
    }
}
