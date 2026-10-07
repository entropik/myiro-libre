//! Module `instrument` (ADR 0005) : trouve le logiciel du fabricant installé,
//! ouvre l'instrument par le pont de la bonne architecture et dit son état.
//! La recherche, l'ordre des paliers, le plafond et la traduction des échecs
//! sont ici, jamais dans les écrans. Rien ici ne dépend de Tauri.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pont_protocole::{Palier, Reponse, Requete};
use serde::Serialize;

use crate::pont::{architecture, Architecture, Panne, Pont};

/// Dernier palier que l'application demande : la connexion. L'étalonnage et
/// les mesures viendront avec leurs écrans.
pub const PLAFOND: Palier = Palier::Connexion;

/// Nom de la DLL du MYIRO-1 cherchée dans le logiciel du fabricant.
pub const NOM_DLL: &str = "FDXSDK.dll";

/// Dossiers d'installation connus des logiciels qui livrent `FDXSDK.dll`
/// (audit du poste, `Audit-MYIRO/analyse/manifest.json`), dans l'ordre de
/// préférence : la DLL 64 bits 1.0.1 essayée sur l'instrument réel, la 64 bits
/// 1.0.3, puis la 32 bits 1.0.1 de l'outil de configuration du fabricant.
pub const EMPLACEMENTS_CONNUS: &[&str] = &[
    "C:/Program Files/Ergosoft 16",
    "C:/ProgramData/EIZO/ColorNavigator 7",
    "C:/Program Files (x86)/Configuration Tool MY-CT1",
];

/// Profondeur de recherche sous un emplacement (ColorNavigator range sa DLL
/// trois niveaux plus bas).
const PROFONDEUR: usize = 3;

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
    /// Logiciel du fabricant introuvable : choisir son dossier.
    ChoixDossier,
}

/// Pourquoi l'instrument n'est pas prêt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probleme {
    /// Aucun `FDXSDK.dll` utilisable aux emplacements examinés.
    LogicielAbsent { examines: String },
    /// Le pont n'a pas pu charger la DLL trouvée, ou elle ne répond pas comme
    /// celle du MYIRO-1 (fichier abîmé, version inconnue).
    LogicielInutilisable { detail: String },
    /// Aucun pont `pont-myiro1` de l'architecture voulue, ou il n'a pas pu
    /// être lancé.
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
            Probleme::LogicielAbsent { .. } | Probleme::LogicielInutilisable { .. } => {
                Ecran::ChoixDossier
            }
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
            Probleme::LogicielAbsent { .. } => "logiciel_absent",
            Probleme::LogicielInutilisable { .. } => "logiciel_inutilisable",
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
            Probleme::AucunInstrument => None,
            Probleme::LogicielAbsent { examines: d }
            | Probleme::LogicielInutilisable { detail: d }
            | Probleme::PontIntrouvable { detail: d }
            | Probleme::PontEnPanne { detail: d }
            | Probleme::PontBloque { detail: d }
            | Probleme::DetectionImpossible { detail: d }
            | Probleme::ConnexionImpossible { detail: d } => Some(d),
        }
    }
}

/// Ce que la page reçoit : l'état en un mot, le modèle et le problème
/// éventuel. Le numéro de série et les chemins restent dans le module, sauf
/// dans le détail technique replié.
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
            Panne::DllRefusee { detail } => Probleme::LogicielInutilisable { detail },
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
    /// DLL retenue, pour la retrouver directement la fois suivante.
    sdk: Option<PathBuf>,
    etat: Etat,
    probleme: Option<Probleme>,
}

impl<P: Pont> Instrument<P> {
    /// Cherche `FDXSDK.dll` dans les emplacements donnés (dans l'ordre ; une
    /// DLL 64 bits est préférée), lance le pont de la même architecture avec
    /// le plafond de l'application, et monte les paliers jusqu'à la connexion.
    ///
    /// `ponts` : les programmes `pont-myiro1` disponibles, par architecture.
    /// `lancer` reçoit le programme, la DLL et le plafond.
    pub fn ouvrir(
        emplacements: &[PathBuf],
        ponts: &[(Architecture, PathBuf)],
        lancer: impl FnOnce(&Path, &Path, Palier) -> Result<P, Panne>,
    ) -> Self {
        let (programme, dll) = match choisir(emplacements, ponts) {
            Ok(choix) => choix,
            Err(probleme) => return Self::en_echec(None, probleme),
        };
        let mut pont = match lancer(&programme, &dll, PLAFOND) {
            Ok(pont) => pont,
            Err(panne) => return Self::en_echec(Some(dll), panne.into()),
        };
        match monter(&mut pont) {
            Ok(fiche) => Instrument {
                pont: Some(pont),
                sdk: Some(dll),
                etat: Etat::EtalonnageRequis(fiche),
                probleme: None,
            },
            Err(probleme) => Self::en_echec(Some(dll), probleme),
        }
    }

    /// Sur un échec, le pont est fermé tout de suite : un nouvel essai en relance un.
    fn en_echec(sdk: Option<PathBuf>, probleme: Probleme) -> Self {
        Instrument {
            pont: None,
            sdk,
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

    /// DLL du fabricant trouvée et confiée au pont, à retenir.
    pub fn sdk(&self) -> Option<&Path> {
        self.sdk.as_deref()
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
                    Ecran::ChoixDossier => "choix_dossier",
                },
                guide_cablage: p.guide_cablage(),
                detail: p.detail().map(String::from),
            }),
        }
    }
}

/// Choisit la DLL et le pont : la première DLL 64 bits trouvée qui a son
/// pont, sinon la première 32 bits qui a le sien.
fn choisir(
    emplacements: &[PathBuf],
    ponts: &[(Architecture, PathBuf)],
) -> Result<(PathBuf, PathBuf), Probleme> {
    let mut examines = String::new();
    let mut trouvees = Vec::new();
    for emplacement in emplacements {
        let dlls = chercher_dlls(emplacement);
        if dlls.is_empty() {
            let _ = writeln!(examines, "{} : aucun {NOM_DLL}", emplacement.display());
        }
        for dll in dlls {
            match architecture(&dll) {
                Some(arch) => {
                    let _ = writeln!(examines, "{} : {}", dll.display(), nom_architecture(arch));
                    trouvees.push((arch, dll));
                }
                None => {
                    let _ = writeln!(examines, "{} : pas une DLL Windows", dll.display());
                }
            }
        }
    }
    for voulue in [Architecture::X64, Architecture::X86] {
        for (arch, dll) in &trouvees {
            if *arch != voulue {
                continue;
            }
            if let Some((_, programme)) = ponts.iter().find(|(a, _)| a == arch) {
                return Ok((programme.clone(), dll.clone()));
            }
        }
    }
    let examines = examines.trim_end().to_string();
    if trouvees.is_empty() {
        return Err(Probleme::LogicielAbsent { examines });
    }
    let disponibles: Vec<_> = ponts.iter().map(|(a, _)| nom_architecture(*a)).collect();
    Err(Probleme::PontIntrouvable {
        detail: format!(
            "{examines}\naucun pont-myiro1 de cette architecture ; ponts disponibles : {}",
            if disponibles.is_empty() {
                "aucun".to_string()
            } else {
                disponibles.join(", ")
            }
        ),
    })
}

fn nom_architecture(arch: Architecture) -> &'static str {
    match arch {
        Architecture::X64 => "64 bits",
        Architecture::X86 => "32 bits",
    }
}

/// Monte les paliers dans l'ordre : version du SDK, détection, connexion au
/// premier instrument détecté. Jamais plus loin que `PLAFOND`.
fn monter(pont: &mut impl Pont) -> Result<Fiche, Probleme> {
    match pont.demander(&Requete::Version {})? {
        Reponse::Version { .. } => {}
        Reponse::Erreur { erreur } => {
            return Err(Probleme::LogicielInutilisable {
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

/// Tous les `FDXSDK.dll` d'un emplacement : la DLL elle-même, ou celles d'un
/// dossier et de ses sous-dossiers (trois niveaux au plus), dans l'ordre des
/// noms. Le pont exécute le code de la DLL qu'on lui donne : un fichier d'un
/// autre nom n'est jamais retenu.
fn chercher_dlls(emplacement: &Path) -> Vec<PathBuf> {
    if emplacement.is_file() {
        let bon_nom = emplacement
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(NOM_DLL));
        return if bon_nom {
            vec![emplacement.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut trouvees = Vec::new();
    chercher_dans(emplacement, PROFONDEUR, &mut trouvees);
    trouvees
}

fn chercher_dans(dossier: &Path, profondeur: usize, trouvees: &mut Vec<PathBuf>) {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return;
    };
    let mut chemins: Vec<PathBuf> = entrees.flatten().map(|e| e.path()).collect();
    chemins.sort();
    let mut sous_dossiers = Vec::new();
    for chemin in chemins {
        if chemin.is_dir() {
            sous_dossiers.push(chemin);
        } else if chemin
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(NOM_DLL))
        {
            trouvees.push(chemin);
        }
    }
    if profondeur > 0 {
        for sous_dossier in sous_dossiers {
            chercher_dans(&sous_dossier, profondeur - 1, trouvees);
        }
    }
}
