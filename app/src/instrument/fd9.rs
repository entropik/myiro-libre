//! Le FD-9 dans le module `instrument` (ticket #13) : son pont `pont-fd9` et
//! ses paliers (version, détection). Isolé du reste du module : le MYIRO-1
//! garde son chemin, `Instrument::ouvrir` ; le choix est dans `choix`.

use std::path::{Path, PathBuf};

use pont_protocole::{Info, Palier, Reponse, Requete};

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
                etalonnage: None,
            },
            Err(probleme) => Self::en_echec(None, probleme),
        }
    }
}

/// Détail technique quand la détection ne voit aucun FD-9 : sans règle
/// entrante du pare-feu, la réponse à la diffusion est bloquée (confirmé sur
/// le poste le 9 octobre 2026, fiche `FD9_GetDeviceList`).
const DETAIL_AUCUN_FD9: &str = "détection du FD-9 : liste vide, sans erreur. \
Le FD-9 répond sur le port UDP 49152 de l'ordinateur ; le pare-feu de Windows \
bloque cette réponse sans règle entrante pour le programme pont-fd9 \
(UDP, port local 49152, adresse du FD-9). FD-S2w doit aussi être fermé.";

/// Paliers du FD-9 : version (lue sur le fichier de la DLL), puis détection.
/// Rend l'identifiant du premier FD-9 vu.
fn detecter(pont: &mut impl Pont) -> Result<Info<String>, Probleme> {
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
            .ok_or_else(|| Probleme::AucunFd9 {
                detail: DETAIL_AUCUN_FD9.into(),
            }),
        Reponse::Erreur { erreur } => Err(Probleme::DetectionImpossible {
            detail: format!("détection du FD-9 : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}
