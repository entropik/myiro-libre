//! Boucle du pont : lit une requête JSON par ligne, la confie à la session et
//! écrit une réponse JSON par ligne (ADR 0005).

use crate::{Connexion, MesurePlage, SdkMyiro1, Session};
use fdx_sys::Liaison;
use pont_protocole::{
    ecrire_reponse, lire_requete, DonneesBrutes, ErreurMesure, ErreurPont, Identite,
    InstrumentDetecte, Lab, Mesure, Plage, Provenance, Reponse, Requete, Spectre,
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
            reponse_mesure(vec![plage], mesure.provenance)
        }),
        Requete::MesurerBande { plages_attendues } => session
            .mesurer_bande(plages_attendues)
            .and_then(|bande| reponse_mesure(bande.plages, bande.provenance)),
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

/// Vérifie la mesure avant qu'elle ne sorte du pont : une valeur non finie
/// ou une forme imprévue de la DLL devient une erreur, jamais une mesure.
fn reponse_mesure(plages: Vec<MesurePlage>, provenance: Provenance) -> Result<Reponse, ErreurPont> {
    let mesure = plages
        .into_iter()
        .map(plage)
        .collect::<Result<Vec<_>, _>>()
        .and_then(|plages| Mesure::new(plages, provenance))
        .map_err(|erreur| ErreurPont::ReponseInattendue {
            detail: erreur.to_string(),
        })?;
    Ok(Reponse::Mesure { mesure })
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
