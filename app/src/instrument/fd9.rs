//! Le FD-9 dans le module `instrument` (ticket #13) : son pont `pont-fd9` et
//! ses paliers (version, détection), et la règle du pare-feu (ticket #47).
//! Isolé du reste du module : le MYIRO-1 garde son chemin,
//! `Instrument::ouvrir` ; le choix est dans `choix`.

use std::path::{Path, PathBuf};

use pont_protocole::{Info, Palier, Reponse, Requete};

use super::parefeu::{
    AutorisationPareFeu, EchecPareFeu, EtatRegle, PareFeu, RegleFd9, PORT_DETECTION,
};
use super::{choisir, inattendue, Etat, Instrument, Probleme};
use crate::pont::{Architecture, Panne, Pont};

/// DLL du FD-9 cherchée dans le logiciel du fabricant.
pub const NOM_DLL_FD9: &str = "FD9SDK.dll";

/// Programme pont du FD-9 (`pont-fd9-x64.exe`, `pont-fd9-x86.exe` installés).
pub const NOM_PONT_FD9: &str = "pont-fd9";

/// Derniers paliers déclarés par `pont-fd9` : la version et la détection.
pub const PLAFOND_FD9: Palier = Palier::Detection;

/// Modèle affiché dans la barre.
pub const MODELE_FD9: &str = "FD-9";

/// Dossiers d'installation connus qui livrent `FD9SDK.dll` (audit du poste),
/// dans l'ordre : FD-S2w (1.3.2.3, 32 bits, logiciel qui pilote le FD-9),
/// puis Ergosoft 16 (1.3.1.5, 64 bits).
pub const EMPLACEMENTS_FD9: &[&str] = &[
    "C:/Program Files (x86)/KONICA MINOLTA/FD-S2w",
    "C:/Program Files/Ergosoft 16",
];

/// Emplacements à essayer pour le FD-9 : le dossier choisi par l'opérateur,
/// puis les emplacements connus. La DLL du FD-9 n'est ni embarquée ni retenue
/// d'une fois sur l'autre : elle a besoin des DLL voisines de son logiciel.
pub fn emplacements_fd9(choisi: Option<PathBuf>) -> Vec<PathBuf> {
    choisi
        .into_iter()
        .chain(EMPLACEMENTS_FD9.iter().map(PathBuf::from))
        .collect()
}

impl<P: Pont> Instrument<P> {
    /// Cherche `FD9SDK.dll`, lance `pont-fd9` de la même architecture avec le
    /// plafond Détection, puis demande la version et la détection. Un FD-9
    /// vu devient `Etat::Detecte` : ce pont ne connecte pas encore.
    ///
    /// À la première détection vide, si la règle du pare-feu manque, elle est
    /// demandée à Windows pour le pont lancé, puis la détection est relancée
    /// une fois (ticket #47). `autorisation` garde la trace de la demande
    /// d'une ouverture à l'autre : elle n'est jamais faite deux fois d'elle-même.
    pub fn ouvrir_fd9<F: PareFeu>(
        emplacements: &[PathBuf],
        ponts: &[(Architecture, PathBuf)],
        lancer: impl FnOnce(&Path, &Path, Palier) -> Result<P, Panne>,
        autorisation: &mut AutorisationPareFeu<F>,
    ) -> Self {
        let (programme, dll) = match choisir(NOM_DLL_FD9, NOM_PONT_FD9, emplacements, ponts) {
            Ok(choix) => choix,
            Err(probleme) => return Self::en_echec(None, probleme),
        };
        let mut pont = match lancer(&programme, &dll, PLAFOND_FD9) {
            Ok(pont) => pont,
            Err(panne) => return Self::en_echec(None, panne.into()),
        };
        let vu = lire_version(&mut pont)
            .and_then(|()| detecter(&mut pont))
            .and_then(|vu| match vu {
                Some(identifiant) => Ok(identifiant),
                None => ouvrir_pare_feu(&mut pont, &programme, autorisation),
            });
        match vu {
            Ok(identifiant) => Instrument {
                pont: Some(pont),
                sdk: None,
                etat: Etat::Detecte {
                    modele: MODELE_FD9.into(),
                    identifiant,
                },
                probleme: None,
                etalonnage: None,
                repos_incertain: false,
                annulation: crate::pont::Annulation::default(),
            },
            Err(probleme) => Self::en_echec(None, probleme),
        }
    }
}

/// Détail technique quand la détection ne voit aucun FD-9 alors que la règle
/// du pare-feu est en place : le FD-9 est éteint, sur un autre réseau, ou
/// FD-S2w occupe le port.
const DETAIL_AUCUN_FD9: &str = "détection du FD-9 : liste vide, sans erreur. \
La règle du pare-feu de Windows pour le programme pont-fd9 (UDP, port local \
49152, réseau local) est en place : le FD-9 n'a pas répondu. Il est peut-être \
éteint ou sur un autre réseau ; FD-S2w, s'il est ouvert, occupe le port 49152.";

/// La liste est vide. Sans règle du pare-feu, la réponse du FD-9 a été
/// bloquée (confirmé sur le poste le 9 octobre 2026, fiche
/// `FD9_GetDeviceList`) : la règle est demandée une fois, puis la détection
/// relancée une fois.
fn ouvrir_pare_feu<F: PareFeu>(
    pont: &mut impl Pont,
    programme: &Path,
    autorisation: &mut AutorisationPareFeu<F>,
) -> Result<Info<String>, Probleme> {
    let aucun = || Probleme::AucunFd9 {
        detail: DETAIL_AUCUN_FD9.into(),
    };
    let regle = RegleFd9 {
        programme: programme.to_path_buf(),
    };
    // Une règle de ce nom pour un autre programme ne sert à rien : elle est
    // remplacée comme si elle manquait.
    let etat = autorisation.pare_feu_mut().lire(&regle);
    if etat == EtatRegle::Presente {
        return Err(aucun());
    }
    match autorisation.demander(&regle) {
        Some(Ok(())) => detecter(pont)?.ok_or_else(aucun),
        Some(Err(EchecPareFeu::Refuse)) => Err(pare_feu_ferme(
            &regle,
            "la fenêtre de contrôle de compte de Windows a été refusée",
        )),
        Some(Err(EchecPareFeu::Echec { detail })) => Err(pare_feu_ferme(&regle, &detail)),
        None if etat == EtatRegle::AutreProgramme => Err(pare_feu_ferme(
            &regle,
            "la règle de ce nom vise un autre programme ; elle a déjà été demandée",
        )),
        None => Err(pare_feu_ferme(
            &regle,
            "la règle manque toujours ; elle a déjà été demandée",
        )),
    }
}

fn pare_feu_ferme(regle: &RegleFd9, raison: &str) -> Probleme {
    Probleme::PareFeuFerme {
        detail: format!(
            "pare-feu de Windows : {raison}.\n\
             Le FD-9 répond sur le port UDP {PORT_DETECTION} de l'ordinateur ; sans \
             cette règle, Windows bloque sa réponse.\n\
             Commande équivalente, dans une invite de commandes ouverte en administrateur :\n\
             {}\n\
             Pour retirer la règle :\n\
             {}",
            regle.commande_ajout(),
            RegleFd9::commande_retrait()
        ),
    }
}

/// Premier palier : la version, lue sur le fichier de la DLL.
fn lire_version(pont: &mut impl Pont) -> Result<(), Probleme> {
    match pont.demander(&Requete::Version {})? {
        Reponse::VersionDll { .. } => Ok(()),
        Reponse::Erreur { erreur } => Err(Probleme::LogicielInutilisable {
            detail: format!("version du SDK du FD-9 : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}

/// Détection : l'identifiant du premier FD-9 vu, `None` si la liste est vide.
fn detecter(pont: &mut impl Pont) -> Result<Option<Info<String>>, Probleme> {
    match pont.demander(&Requete::Detecter {})? {
        Reponse::InstrumentsFd9 { liste } => {
            Ok(liste.into_iter().next().map(|fd9| fd9.identifiant))
        }
        Reponse::Erreur { erreur } => Err(Probleme::DetectionImpossible {
            detail: format!("détection du FD-9 : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}
