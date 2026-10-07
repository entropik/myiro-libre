//! Module `instrument` (ADR 0005) : ouvre l'instrument par un pont et dit son
//! état. L'ordre des paliers, le plafond et la traduction des échecs sont ici,
//! jamais dans les écrans. Rien ici ne dépend de Tauri.

use std::path::{Path, PathBuf};

use pont_protocole::{Palier, Reponse, Requete};
use serde::Serialize;

use crate::pont::{Panne, Pont};

/// Dernier palier que l'application demande : la connexion. L'étalonnage et
/// les mesures viendront avec leurs écrans.
pub const PLAFOND: Palier = Palier::Connexion;

/// Nom de la DLL du MYIRO-1 cherchée dans l'emplacement du SDK.
pub const NOM_DLL: &str = "FDXSDK.dll";

/// Seul modèle servi par `pont-myiro1`.
const MODELE: &str = "MYIRO-1";

/// Ce que l'application montre d'un instrument connecté.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fiche {
    pub modele: String,
    pub numero_serie: u32,
    pub micrologiciel: String,
}

/// État de l'instrument, tel que la barre l'affiche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Etat {
    NonDetecte,
    /// Connecté, sans étalonnage à faire (instrument qui n'en a pas besoin).
    Connecte(Fiche),
    /// Connecté ; un étalonnage sur le blanc est nécessaire avant de mesurer.
    EtalonnageRequis(Fiche),
    Etalonne(Fiche),
}

/// Écran qui aide à résoudre un problème.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ecran {
    /// Instrument non détecté : câble, port USB direct, réessayer.
    NonDetecte,
    /// Emplacement du SDK installé sur le poste.
    EmplacementSdk,
}

/// Pourquoi l'instrument n'est pas prêt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probleme {
    /// L'emplacement du SDK installé n'a pas encore été indiqué.
    SdkNonIndique,
    /// Aucun `FDXSDK.dll` à l'emplacement indiqué.
    AucuneDll { emplacement: String },
    /// Le pont n'a pas pu charger la DLL trouvée, ou elle ne répond pas comme
    /// celle du MYIRO-1 (autre architecture, fichier incomplet).
    SdkInutilisable { detail: String },
    /// Le programme `pont-myiro1` n'a pas pu être lancé.
    PontIntrouvable { detail: String },
    /// Le pont s'est arrêté ou a répondu hors du protocole.
    PontEnPanne { detail: String },
    /// Le pont n'a pas répondu dans le délai et a été arrêté de force :
    /// l'instrument est peut-être resté dans un état incertain (#21).
    PontBloque { detail: String },
    /// Le pont répond, mais ne voit aucun instrument.
    AucunInstrument,
    /// La recherche des instruments branchés a échoué.
    DetectionImpossible { detail: String },
    /// L'instrument détecté a refusé la connexion ou ne répond pas.
    ConnexionImpossible { detail: String },
}

impl Probleme {
    /// Écran qui aide l'opérateur à le résoudre.
    pub fn ecran(&self) -> Ecran {
        match self {
            Probleme::SdkNonIndique
            | Probleme::AucuneDll { .. }
            | Probleme::SdkInutilisable { .. } => Ecran::EmplacementSdk,
            Probleme::PontIntrouvable { .. }
            | Probleme::PontEnPanne { .. }
            | Probleme::PontBloque { .. }
            | Probleme::AucunInstrument
            | Probleme::DetectionImpossible { .. }
            | Probleme::ConnexionImpossible { .. } => Ecran::NonDetecte,
        }
    }

    /// Les étapes « câble, port USB direct » aident-elles ? Seulement quand
    /// c'est l'instrument qui manque, pas le programme pont.
    pub fn guide_cablage(&self) -> bool {
        matches!(
            self,
            Probleme::AucunInstrument
                | Probleme::DetectionImpossible { .. }
                | Probleme::ConnexionImpossible { .. }
        )
    }

    /// Code du problème : les textes de l'écran sont les clés
    /// `probleme.<code>.cause` et `probleme.<code>.action` du catalogue.
    pub fn code(&self) -> &'static str {
        match self {
            Probleme::SdkNonIndique => "sdk_non_indique",
            Probleme::AucuneDll { .. } => "aucune_dll",
            Probleme::SdkInutilisable { .. } => "sdk_inutilisable",
            Probleme::PontIntrouvable { .. } => "pont_introuvable",
            Probleme::PontEnPanne { .. } => "pont_en_panne",
            Probleme::PontBloque { .. } => "pont_bloque",
            Probleme::AucunInstrument => "aucun_instrument",
            Probleme::DetectionImpossible { .. } => "detection_impossible",
            Probleme::ConnexionImpossible { .. } => "connexion_impossible",
        }
    }

    /// Détail technique, montré replié sous l'explication.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Probleme::SdkNonIndique | Probleme::AucunInstrument => None,
            Probleme::AucuneDll { emplacement: d }
            | Probleme::SdkInutilisable { detail: d }
            | Probleme::PontIntrouvable { detail: d }
            | Probleme::PontEnPanne { detail: d }
            | Probleme::PontBloque { detail: d }
            | Probleme::DetectionImpossible { detail: d }
            | Probleme::ConnexionImpossible { detail: d } => Some(d),
        }
    }
}

/// Ce que la page reçoit : l'état en un mot, le modèle et le problème
/// éventuel. Le numéro de série reste dans le module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Vue {
    pub etat: &'static str,
    pub modele: Option<String>,
    pub probleme: Option<VueProbleme>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueProbleme {
    pub code: &'static str,
    pub ecran: &'static str,
    /// Montrer les étapes câble et port USB.
    pub guide_cablage: bool,
    pub detail: Option<String>,
}

impl From<Panne> for Probleme {
    fn from(panne: Panne) -> Self {
        match panne {
            Panne::Lancement { detail } => Probleme::PontIntrouvable { detail },
            Panne::DllRefusee { detail } => Probleme::SdkInutilisable { detail },
            Panne::Arret { code, detail } => Probleme::PontEnPanne {
                detail: match code {
                    Some(code) => format!("arrêt du pont, code {code} : {detail}"),
                    None => format!("arrêt du pont : {detail}"),
                },
            },
            Panne::ReponseIllisible { detail } => Probleme::PontEnPanne { detail },
            Panne::SansReponse { detail } => Probleme::PontBloque { detail },
        }
    }
}

/// Un instrument ouvert par un pont, ou la raison pour laquelle il ne l'est pas.
/// Petit à dessein : la fermeture, l'annulation et les mesures s'y ajouteront
/// (spécification #21).
pub struct Instrument<P: Pont> {
    /// Pont gardé ouvert tant que l'instrument est connecté ; il se ferme quand
    /// l'instrument est abandonné. Les paliers suivants passeront par lui.
    #[allow(dead_code)]
    pont: Option<P>,
    etat: Etat,
    probleme: Option<Probleme>,
}

impl<P: Pont> Instrument<P> {
    /// Cherche la DLL dans l'emplacement du SDK, lance le pont avec le plafond
    /// de l'application et monte les paliers jusqu'à la connexion.
    pub fn ouvrir(
        sdk: Option<&Path>,
        lancer: impl FnOnce(&Path, Palier) -> Result<P, Panne>,
    ) -> Self {
        let dll = match sdk {
            None => return Self::en_echec(Probleme::SdkNonIndique),
            Some(emplacement) => match chercher_dll(emplacement) {
                Some(dll) => dll,
                None => {
                    let emplacement = emplacement.display().to_string();
                    return Self::en_echec(Probleme::AucuneDll { emplacement });
                }
            },
        };
        let mut pont = match lancer(&dll, PLAFOND) {
            Ok(pont) => pont,
            Err(panne) => return Self::en_echec(panne.into()),
        };
        match monter(&mut pont) {
            Ok(fiche) => Instrument {
                pont: Some(pont),
                etat: Etat::EtalonnageRequis(fiche),
                probleme: None,
            },
            Err(probleme) => Self::en_echec(probleme),
        }
    }

    /// Sur un échec, le pont est fermé tout de suite : un nouvel essai en relance un.
    fn en_echec(probleme: Probleme) -> Self {
        Instrument {
            pont: None,
            etat: Etat::NonDetecte,
            probleme: Some(probleme),
        }
    }

    pub fn etat(&self) -> &Etat {
        &self.etat
    }

    pub fn probleme(&self) -> Option<&Probleme> {
        self.probleme.as_ref()
    }

    pub fn vue(&self) -> Vue {
        let (etat, fiche) = match &self.etat {
            Etat::NonDetecte => ("non_detecte", None),
            Etat::Connecte(f) => ("connecte", Some(f)),
            Etat::EtalonnageRequis(f) => ("etalonnage_requis", Some(f)),
            Etat::Etalonne(f) => ("etalonne", Some(f)),
        };
        Vue {
            etat,
            modele: fiche.map(|f| f.modele.clone()),
            probleme: self.probleme.as_ref().map(|p| VueProbleme {
                code: p.code(),
                ecran: match p.ecran() {
                    Ecran::NonDetecte => "non_detecte",
                    Ecran::EmplacementSdk => "emplacement_sdk",
                },
                guide_cablage: p.guide_cablage(),
                detail: p.detail().map(String::from),
            }),
        }
    }
}

/// Monte les paliers dans l'ordre : version du SDK, détection, connexion au
/// premier instrument détecté. Jamais plus loin que `PLAFOND`.
fn monter(pont: &mut impl Pont) -> Result<Fiche, Probleme> {
    match pont.demander(&Requete::Version {})? {
        Reponse::Version { .. } => {}
        Reponse::Erreur { erreur } => {
            return Err(Probleme::SdkInutilisable {
                detail: format!("version du SDK : {erreur:?}"),
            })
        }
        autre => return Err(inattendue(autre)),
    }
    match pont.demander(&Requete::Detecter {})? {
        Reponse::Instruments { liste } if liste.is_empty() => {
            return Err(Probleme::AucunInstrument)
        }
        Reponse::Instruments { .. } => {}
        Reponse::Erreur { erreur } => {
            return Err(Probleme::DetectionImpossible {
                detail: format!("détection : {erreur:?}"),
            })
        }
        autre => return Err(inattendue(autre)),
    }
    match pont.demander(&Requete::Connecter { instrument: 0 })? {
        Reponse::Connecte { identite } => Ok(Fiche {
            modele: MODELE.into(),
            numero_serie: identite.numero_serie,
            micrologiciel: identite.micrologiciel,
        }),
        Reponse::Erreur { erreur } => Err(Probleme::ConnexionImpossible {
            detail: format!("connexion : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}

fn inattendue(reponse: Reponse) -> Probleme {
    Probleme::PontEnPanne {
        detail: format!("réponse inattendue : {reponse:?}"),
    }
}

/// Trouve `FDXSDK.dll` : l'emplacement est la DLL elle-même ou un dossier qui
/// la contient, au plus quelques niveaux plus bas.
pub fn chercher_dll(emplacement: &Path) -> Option<PathBuf> {
    if emplacement.is_file() {
        // Le pont exécute le code de la DLL qu'on lui donne : jamais un autre fichier.
        let nom = emplacement.file_name()?;
        return nom
            .eq_ignore_ascii_case(NOM_DLL)
            .then(|| emplacement.to_path_buf());
    }
    chercher_dans(emplacement, 3)
}

fn chercher_dans(dossier: &Path, profondeur: usize) -> Option<PathBuf> {
    let mut sous_dossiers = Vec::new();
    for entree in std::fs::read_dir(dossier).ok()?.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            sous_dossiers.push(chemin);
        } else if entree.file_name().eq_ignore_ascii_case(NOM_DLL) {
            return Some(chemin);
        }
    }
    sous_dossiers.sort();
    if profondeur == 0 {
        return None;
    }
    sous_dossiers
        .iter()
        .find_map(|d| chercher_dans(d, profondeur - 1))
}
