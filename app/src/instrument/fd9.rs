//! Le FD-9 dans le module `instrument` (ticket #13) : son pont `pont-fd9`, ses
//! paliers (version, détection) et le choix entre les deux ponts. Isolé du
//! reste du module : le MYIRO-1 garde son chemin, `Instrument::ouvrir`.

use std::path::{Path, PathBuf};

use pont_protocole::{Palier, Reponse, Requete};

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

/// Où chercher la DLL d'un instrument, et ses ponts par architecture.
#[derive(Clone, Copy)]
pub struct Recherche<'a> {
    pub emplacements: &'a [PathBuf],
    pub ponts: &'a [(Architecture, PathBuf)],
}

/// Ouvre le MYIRO-1 ; s'il n'est pas détecté, cherche un FD-9. Si aucun des
/// deux ne l'est, le problème du MYIRO-1 reste celui qu'on montre.
pub fn ouvrir_l_un_ou_l_autre<P: Pont>(
    myiro1: Recherche,
    fd9: Recherche,
    mut lancer: impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
) -> Instrument<P> {
    let premier = Instrument::ouvrir(myiro1.emplacements, myiro1.ponts, &mut lancer);
    if premier.etat != Etat::NonDetecte {
        return premier;
    }
    let second = Instrument::ouvrir_fd9(fd9.emplacements, fd9.ponts, &mut lancer);
    if second.etat != Etat::NonDetecte {
        return second;
    }
    premier
}

impl<P: Pont> Instrument<P> {
    /// Cherche `FD9SDK.dll`, lance `pont-fd9` de la même architecture avec le
    /// plafond Détection, puis demande la version et la détection. Un FD-9
    /// vu devient `Etat::Detecte` : ce pont ne connecte pas encore.
    pub fn ouvrir_fd9(
        emplacements: &[PathBuf],
        ponts: &[(Architecture, PathBuf)],
        lancer: impl FnOnce(&Path, &Path, Palier) -> Result<P, Panne>,
    ) -> Self {
        let (programme, dll) = match choisir(NOM_DLL_FD9, NOM_PONT_FD9, emplacements, ponts) {
            Ok(choix) => choix,
            Err(probleme) => return Self::en_echec(None, probleme),
        };
        let mut pont = match lancer(&programme, &dll, PLAFOND_FD9) {
            Ok(pont) => pont,
            Err(panne) => return Self::en_echec(None, panne.into()),
        };
        match detecter(&mut pont) {
            Ok(identifiant) => Instrument {
                pont: Some(pont),
                sdk: None,
                etat: Etat::Detecte {
                    modele: MODELE_FD9.into(),
                    identifiant,
                },
                probleme: None,
            },
            Err(probleme) => Self::en_echec(None, probleme),
        }
    }
}

/// Paliers du FD-9 : version (lue sur le fichier de la DLL), puis détection.
/// Rend l'identifiant du premier FD-9 vu.
fn detecter(pont: &mut impl Pont) -> Result<String, Probleme> {
    match pont.demander(&Requete::Version {})? {
        Reponse::VersionDll { .. } => {}
        Reponse::Erreur { erreur } => {
            return Err(Probleme::LogicielInutilisable {
                detail: format!("version du SDK du FD-9 : {erreur:?}"),
            })
        }
        autre => return Err(inattendue(autre)),
    }
    match pont.demander(&Requete::Detecter {})? {
        Reponse::InstrumentsFd9 { liste } => liste
            .into_iter()
            .next()
            .map(|fd9| fd9.identifiant)
            .ok_or(Probleme::AucunInstrument),
        Reponse::Erreur { erreur } => Err(Probleme::DetectionImpossible {
            detail: format!("détection du FD-9 : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}
