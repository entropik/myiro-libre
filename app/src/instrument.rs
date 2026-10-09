//! Module `instrument` (ADR 0005) : trouve le logiciel du fabricant installé,
//! ouvre l'instrument par le pont de la bonne architecture et dit son état.
//! La recherche, l'ordre des paliers, le plafond et la traduction des échecs
//! sont ici, jamais dans les écrans. Rien ici ne dépend de Tauri.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pont_protocole::{
    Declenchement, ErreurPont, Geometrie, Horodatage, Info, Mesure, Palier, RemiseAuRepos, Reponse,
    Requete,
};
use serde::Serialize;

use crate::pont::{architecture, Architecture, Panne, Pont};

/// Dernier palier que l'application autorise au pont : la mesure ponctuelle,
/// jamais la bande (ticket #7). L'ouverture s'arrête à la connexion ;
/// l'étalonnage et la mesure ne sont demandés que par
/// [`Instrument::etalonner`] et [`Instrument::mesurer_ponctuelle`].
pub const PLAFOND: Palier = Palier::MesurePonctuelle;

/// Geste que l'instrument demande à l'opérateur (ADR 0005 : « les gestes
/// humains passent par un trait dédié »).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Geste {
    /// Poser le MYIRO-1 sur son capuchon, où se trouve son blanc de référence.
    PoserSurBlanc,
    /// Poser le MYIRO-1 à plat sur la couleur à mesurer ; en manuel,
    /// l'opérateur appuiera ensuite sur le bouton de l'instrument, une fois
    /// celui-ci armé ; en automatique, le pont déclenche la mesure.
    PoserSurCouleur,
}

/// Une mesure lue par l'instrument, telle que le pont l'a rendue : sa
/// provenance est celle du pont, jamais refaite par l'application.
#[derive(Clone, Debug, PartialEq)]
pub struct MesureAcquise {
    pub mesure: Mesure,
    /// Retour au repos de l'instrument après la lecture (ticket #24). La
    /// mesure reste valable même s'il n'est pas prouvé.
    pub remise_au_repos: Info<RemiseAuRepos>,
}

impl MesureAcquise {
    /// Le pont permet d'armer de nouveau : repos prouvé, ou supposé faute
    /// d'armement. Un repos inconnu n'est jamais pris pour une réussite.
    pub fn repos_sur(&self) -> bool {
        matches!(
            self.remise_au_repos,
            Info::Confirmee(RemiseAuRepos::AuRepos {} | RemiseAuRepos::ReposSuppose {})
        )
    }
}

/// Réponse de l'opérateur à un geste demandé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accord {
    /// Le geste est fait : l'instrument peut continuer.
    Fait,
    /// L'opérateur renonce : rien n'est envoyé à l'instrument.
    Annule,
}

/// Les gestes humains. L'adapter réel est l'écran (il montre le geste et
/// attend la réponse de l'opérateur) ; en test, une fonction simule l'opérateur.
pub trait Gestes {
    fn demander(&mut self, geste: Geste) -> Accord;
}

/// Opérateur simulé : toute fonction `Geste -> Accord`.
impl<F: FnMut(Geste) -> Accord> Gestes for F {
    fn demander(&mut self, geste: Geste) -> Accord {
        self(geste)
    }
}

/// Choix entre le pont MYIRO-1 et le pont FD-9 (ticket #13).
pub mod choix;
/// Le FD-9 : son pont et ses paliers (ticket #13).
pub mod fd9;
/// La règle du pare-feu de Windows pour la détection du FD-9 (ticket #47).
pub mod parefeu;

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

/// Dossier des DLL embarquées, à côté de l'application installée
/// (`sdk/x64`, `sdk/x86`) : seulement dans un installateur construit sur le
/// poste avec le dossier `SDK/` du dépôt, jamais publié (ADR 0005).
pub const DOSSIER_EMBARQUE: &str = "sdk";

/// Emplacements à essayer, dans l'ordre : le dossier choisi par l'opérateur,
/// les DLL embarquées à côté de l'application, en développement le dossier
/// `SDK/` du dépôt, la DLL retenue la fois précédente, puis les emplacements
/// connus des logiciels du fabricant.
pub fn emplacements_a_essayer(
    dossier_application: &Path,
    retenu: Option<PathBuf>,
    choisi: Option<PathBuf>,
) -> Vec<PathBuf> {
    let depot = cfg!(debug_assertions)
        .then(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("SDK"));
    choisi
        .into_iter()
        .chain([dossier_application.join(DOSSIER_EMBARQUE)])
        .chain(depot)
        .chain(retenu)
        .chain(EMPLACEMENTS_CONNUS.iter().map(PathBuf::from))
        .collect()
}

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
    /// Vu par la détection, pas connecté : le pont de cet instrument ne va pas
    /// plus loin pour l'instant (FD-9). `identifiant` : texte rendu par la DLL.
    Detecte {
        modele: String,
        identifiant: pont_protocole::Info<String>,
    },
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
    /// Étalonnage à refaire : schéma du blanc et bouton « Étalonner ».
    Etalonnage,
    /// Feuille de la tâche Mesurer : l'avis s'affiche au-dessus du bouton
    /// « Mesurer », l'instrument reste connecté.
    Mesure,
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
    /// Le pont FD-9 ne voit aucun FD-9 : sur le réseau, le pare-feu de
    /// Windows bloque peut-être la réponse (fiche `FD9_GetDeviceList`).
    AucunFd9 { detail: String },
    /// La règle du pare-feu pour le FD-9 manque : l'opérateur a refusé la
    /// fenêtre de contrôle de compte, ou l'ajout a échoué. Rien n'est
    /// redemandé tout seul ; le bouton « Autoriser le FD-9 dans le pare-feu »
    /// le fait. `detail` donne la commande équivalente et celle qui la retire.
    PareFeuFerme { detail: String },
    /// La recherche des instruments branchés a échoué.
    DetectionImpossible { detail: String },
    /// L'instrument détecté a refusé la connexion ou ne répond pas.
    ConnexionImpossible { detail: String },
    /// L'instrument a refusé ou raté l'étalonnage (événement d'échec, refus) :
    /// il reste connecté, l'étalonnage est à refaire. Le message suppose que
    /// l'instrument était mal posé ; l'événement 9 sans capuchon n'a pas été
    /// observé (fiche `docs/abi/FDX_Calibration.md`, « Reste à vérifier »).
    EtalonnageEchoue { detail: String },
    /// L'instrument n'a pas terminé l'étalonnage dans le délai du pont.
    EtalonnageDelai { detail: String },
    /// Liaison perdue avec l'instrument (événement 6) : rien n'est possible
    /// avant une nouvelle connexion.
    InstrumentPerdu { detail: String },
    /// Après une mesure (gardée), le retour au repos de l'instrument n'est
    /// pas prouvé : le pont refuserait d'armer de nouveau. Seul un nouveau
    /// pont (« Réessayer ») lève ce doute (ticket #24).
    ReposIncertain { detail: String },
    /// L'instrument a signalé l'échec de la mesure (événement 4), l'a refusée,
    /// ou sa lecture est inexploitable : l'instrument reste étalonné.
    MesureEchouee { detail: String },
    /// Personne n'a appuyé sur le bouton de l'instrument dans le délai du pont.
    MesureDelai { detail: String },
    /// L'instrument ne se dit plus étalonné au moment de mesurer : son
    /// étalonnage est à refaire avant toute mesure.
    EtalonnageARefaire { detail: String },
    /// Mesure automatique : l'instrument a refusé de mesurer sans son bouton
    /// (fiche `docs/abi/FDX_StartMeasurement.md`). Il reste étalonné ; la
    /// mesure manuelle, au bouton, reste possible.
    DeclenchementRefuse { detail: String },
    /// Mesure automatique : l'instrument n'a pas signalé la fin de la mesure
    /// dans le délai du pont. Personne n'avait de bouton à presser.
    MesureDelaiAutomatique { detail: String },
}

impl Probleme {
    /// Écran qui aide l'opérateur à le résoudre.
    pub fn ecran(&self) -> Ecran {
        match self {
            Probleme::LogicielAbsent { .. } | Probleme::LogicielInutilisable { .. } => {
                Ecran::ChoixDossier
            }
            Probleme::EtalonnageEchoue { .. } | Probleme::EtalonnageDelai { .. } => {
                Ecran::Etalonnage
            }
            Probleme::PontIntrouvable { .. }
            | Probleme::PontEnPanne { .. }
            | Probleme::PontBloque { .. }
            | Probleme::AucunInstrument
            | Probleme::AucunFd9 { .. }
            | Probleme::PareFeuFerme { .. }
            | Probleme::DetectionImpossible { .. }
            | Probleme::ConnexionImpossible { .. }
            | Probleme::InstrumentPerdu { .. } => Ecran::NonDetecte,
            Probleme::ReposIncertain { .. }
            | Probleme::MesureEchouee { .. }
            | Probleme::MesureDelai { .. }
            | Probleme::MesureDelaiAutomatique { .. }
            | Probleme::DeclenchementRefuse { .. }
            | Probleme::EtalonnageARefaire { .. } => Ecran::Mesure,
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
                | Probleme::InstrumentPerdu { .. }
        )
    }

    /// Le bouton « Autoriser le FD-9 dans le pare-feu » est-il montré ?
    pub fn autoriser_pare_feu(&self) -> bool {
        matches!(self, Probleme::PareFeuFerme { .. })
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
            Probleme::AucunFd9 { .. } => "aucun_fd9",
            Probleme::PareFeuFerme { .. } => "pare_feu_ferme",
            Probleme::DetectionImpossible { .. } => "detection_impossible",
            Probleme::ConnexionImpossible { .. } => "connexion_impossible",
            Probleme::EtalonnageEchoue { .. } => "etalonnage_echoue",
            Probleme::EtalonnageDelai { .. } => "etalonnage_delai",
            Probleme::InstrumentPerdu { .. } => "instrument_perdu",
            Probleme::ReposIncertain { .. } => "repos_incertain",
            Probleme::MesureEchouee { .. } => "mesure_echouee",
            Probleme::MesureDelai { .. } => "mesure_delai",
            Probleme::MesureDelaiAutomatique { .. } => "mesure_delai_automatique",
            Probleme::EtalonnageARefaire { .. } => "etalonnage_a_refaire",
            Probleme::DeclenchementRefuse { .. } => "declenchement_refuse",
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
            | Probleme::AucunFd9 { detail: d }
            | Probleme::PareFeuFerme { detail: d }
            | Probleme::DetectionImpossible { detail: d }
            | Probleme::ConnexionImpossible { detail: d }
            | Probleme::EtalonnageEchoue { detail: d }
            | Probleme::EtalonnageDelai { detail: d }
            | Probleme::ReposIncertain { detail: d }
            | Probleme::MesureEchouee { detail: d }
            | Probleme::MesureDelai { detail: d }
            | Probleme::MesureDelaiAutomatique { detail: d }
            | Probleme::EtalonnageARefaire { detail: d }
            | Probleme::DeclenchementRefuse { detail: d }
            | Probleme::InstrumentPerdu { detail: d } => Some(d),
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
    /// L'instrument peut mesurer (connecté sans étalonnage à faire, ou étalonné).
    pub pret: bool,
    /// Une mesure peut partir : instrument prêt, et repos de l'instrument non
    /// mis en doute par la mesure précédente.
    pub mesurable: bool,
    pub probleme: Option<VueProbleme>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueProbleme {
    pub code: &'static str,
    pub ecran: &'static str,
    /// Montrer les étapes câble et port USB.
    pub guide_cablage: bool,
    /// Montrer le bouton « Autoriser le FD-9 dans le pare-feu ».
    pub autoriser_pare_feu: bool,
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
    /// l'instrument est abandonné. Les paliers suivants passent par lui.
    pont: Option<P>,
    /// DLL retenue, pour la retrouver directement la fois suivante.
    sdk: Option<PathBuf>,
    etat: Etat,
    probleme: Option<Probleme>,
    /// Date du dernier étalonnage réussi, avec fuseau, telle que le pont l'a
    /// rendue : celle de la provenance des mesures qui suivent. Effacée dès
    /// qu'un nouvel étalonnage commence.
    etalonnage: Option<Horodatage>,
    /// Le repos de l'instrument n'a pas été prouvé après une mesure : le pont
    /// refuserait d'armer de nouveau (ticket #24). Seul un nouveau pont lève
    /// ce doute.
    repos_incertain: bool,
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
        let (programme, dll) = match choisir(NOM_DLL, "pont-myiro1", emplacements, ponts) {
            Ok(choix) => choix,
            Err(probleme) => return Self::en_echec(None, probleme),
        };
        let mut pont = match lancer(&programme, &dll, PLAFOND) {
            Ok(pont) => pont,
            Err(panne) => return Self::en_echec(None, panne.into()),
        };
        // La DLL n'est retenue qu'une fois qu'elle a rendu sa version : une
        // DLL refusée serait sinon réessayée en premier à chaque lancement.
        if let Err(probleme) = lire_version(&mut pont) {
            return Self::en_echec(None, probleme);
        }
        match monter(&mut pont) {
            Ok(fiche) => Instrument {
                pont: Some(pont),
                sdk: Some(dll),
                etat: Etat::EtalonnageRequis(fiche),
                probleme: None,
                etalonnage: None,
                repos_incertain: false,
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
            etalonnage: None,
            repos_incertain: false,
        }
    }

    /// Étalonnage sur le blanc : demande d'abord à l'opérateur de poser
    /// l'instrument sur son capuchon, puis demande l'étalonnage au pont.
    /// Sans instrument connecté, ou si l'opérateur renonce, rien n'est envoyé.
    pub fn etalonner(&mut self, gestes: &mut impl Gestes) {
        let fiche = match &self.etat {
            Etat::EtalonnageRequis(f) | Etat::Etalonne(f) => f.clone(),
            // Un FD-9 détecté n'est pas connecté : son pont n'étalonne pas.
            Etat::NonDetecte | Etat::Detecte { .. } | Etat::Connecte(_) => return,
        };
        let Some(pont) = self.pont.as_mut() else {
            return;
        };
        if gestes.demander(Geste::PoserSurBlanc) == Accord::Annule {
            return;
        }
        // Un nouvel étalonnage rend l'ancien inutilisable dès son début (#23).
        self.etalonnage = None;
        self.etat = Etat::EtalonnageRequis(fiche.clone());
        self.probleme = None;
        let probleme = match pont.demander(&Requete::Etalonner {}) {
            Ok(Reponse::Etalonne { date }) => {
                // La date du pont, telle quelle : c'est elle que porteront les
                // mesures (ADR 0005, la provenance est posée par le pont).
                self.etat = Etat::Etalonne(fiche);
                self.etalonnage = Some(date);
                return;
            }
            Ok(Reponse::Erreur { erreur }) => {
                let detail = format!("étalonnage : {erreur:?}");
                match erreur {
                    ErreurPont::Delai {} => Probleme::EtalonnageDelai { detail },
                    ErreurPont::EtalonnageEchoue { .. }
                    | ErreurPont::EtalonnageRequis {}
                    | ErreurPont::NonEtalonne {}
                    | ErreurPont::EtatIncompatible {}
                    | ErreurPont::ParametreRefuse {}
                    | ErreurPont::Sdk { .. } => Probleme::EtalonnageEchoue { detail },
                    ErreurPont::InstrumentPerdu {} => Probleme::InstrumentPerdu { detail },
                    ErreurPont::SessionInexploitable {} => Probleme::ConnexionImpossible { detail },
                    _ => Probleme::PontEnPanne { detail },
                }
            }
            Ok(autre) => inattendue(autre),
            Err(panne) => panne.into(),
        };
        // L'instrument reste connecté si seul l'étalonnage est à refaire ;
        // sinon le pont est fermé, et « Réessayer » en relance un.
        if probleme.ecran() != Ecran::Etalonnage {
            self.pont = None;
            self.etat = Etat::NonDetecte;
        }
        self.probleme = Some(probleme);
    }

    /// Une mesure peut-elle partir vers le pont ?
    fn mesurable(&self) -> bool {
        self.pont.is_some()
            && !self.repos_incertain
            && matches!(self.etat, Etat::Connecte(_) | Etat::Etalonne(_))
    }

    /// Mesure ponctuelle : demande d'abord à l'opérateur de poser l'instrument
    /// sur la couleur, puis la mesure au pont, qui arme l'instrument et attend
    /// l'appui sur son bouton. Rend la mesure telle que le pont l'a donnée,
    /// avec sa provenance et sa remise au repos. Sans instrument étalonné, si
    /// le repos de la mesure précédente est incertain, ou si l'opérateur
    /// renonce, rien n'est envoyé.
    pub fn mesurer_ponctuelle(&mut self, gestes: &mut impl Gestes) -> Option<MesureAcquise> {
        self.mesurer_ponctuelle_avec(gestes, Declenchement::Manuel)
    }

    /// Mesure ponctuelle, au bouton de l'instrument (`Manuel`) ou déclenchée
    /// par le pont (`Automatique`, ticket #51) : l'opérateur pose seulement
    /// l'instrument. Si l'instrument refuse de mesurer sans son bouton, il
    /// reste étalonné et le problème propose de passer en manuel.
    pub fn mesurer_ponctuelle_avec(
        &mut self,
        gestes: &mut impl Gestes,
        declenchement: Declenchement,
    ) -> Option<MesureAcquise> {
        if !self.mesurable() {
            return None;
        }
        let pont = self.pont.as_mut()?;
        if gestes.demander(Geste::PoserSurCouleur) == Accord::Annule {
            return None;
        }
        self.probleme = None;
        let probleme = match pont.demander(&Requete::MesurerPonctuelle { declenchement }) {
            Ok(Reponse::Mesure {
                mesure,
                remise_au_repos,
            }) if mesure.provenance().geometrie == Geometrie::Ponctuelle {} => {
                let acquise = MesureAcquise {
                    mesure,
                    remise_au_repos,
                };
                // La mesure est gardée ; seule la suivante est bloquée.
                if !acquise.repos_sur() {
                    self.repos_incertain = true;
                    self.probleme = Some(Probleme::ReposIncertain {
                        detail: format!("remise au repos : {:?}", acquise.remise_au_repos),
                    });
                }
                return Some(acquise);
            }
            Ok(Reponse::Erreur { erreur }) => {
                let detail = format!("mesure ponctuelle : {erreur:?}");
                match erreur {
                    ErreurPont::MesureEchouee { .. }
                    | ErreurPont::EtatIncompatible {}
                    | ErreurPont::ParametreRefuse {}
                    | ErreurPont::ReponseInattendue { .. }
                    | ErreurPont::Sdk { .. } => Probleme::MesureEchouee { detail },
                    ErreurPont::Delai {} => match declenchement {
                        Declenchement::Manuel => Probleme::MesureDelai { detail },
                        Declenchement::Automatique => Probleme::MesureDelaiAutomatique { detail },
                    },
                    ErreurPont::DeclenchementRefuse { .. } => {
                        Probleme::DeclenchementRefuse { detail }
                    }
                    ErreurPont::NonEtalonne {} | ErreurPont::EtalonnageRequis {} => {
                        Probleme::EtalonnageARefaire { detail }
                    }
                    ErreurPont::ReposIncertain { .. } => Probleme::ReposIncertain { detail },
                    ErreurPont::InstrumentPerdu {} => Probleme::InstrumentPerdu { detail },
                    ErreurPont::SessionInexploitable {} => Probleme::ConnexionImpossible { detail },
                    _ => Probleme::PontEnPanne { detail },
                }
            }
            Ok(autre) => inattendue(autre),
            Err(panne) => panne.into(),
        };
        match &probleme {
            // L'instrument reste connecté et étalonné : une nouvelle mesure suffit.
            Probleme::MesureEchouee { .. }
            | Probleme::MesureDelai { .. }
            | Probleme::MesureDelaiAutomatique { .. }
            | Probleme::DeclenchementRefuse { .. } => {}
            Probleme::ReposIncertain { .. } => self.repos_incertain = true,
            Probleme::EtalonnageARefaire { .. } => {
                if let Etat::Etalonne(fiche) | Etat::Connecte(fiche) = &self.etat {
                    self.etat = Etat::EtalonnageRequis(fiche.clone());
                }
                self.etalonnage = None;
            }
            // Sinon le pont est fermé, et « Réessayer » en relance un.
            _ => {
                self.pont = None;
                self.etat = Etat::NonDetecte;
                self.etalonnage = None;
            }
        }
        self.probleme = Some(probleme);
        None
    }

    /// Heure du dernier étalonnage réussi, avec fuseau, tant qu'il est valable.
    pub fn etalonnage(&self) -> Option<&Horodatage> {
        self.etalonnage.as_ref()
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
        let (etat, modele) = match &self.etat {
            Etat::NonDetecte => ("non_detecte", None),
            Etat::Detecte { modele, .. } => ("detecte", Some(modele)),
            Etat::Connecte(f) => ("connecte", Some(&f.modele)),
            Etat::EtalonnageRequis(f) => ("etalonnage_requis", Some(&f.modele)),
            Etat::Etalonne(f) => ("etalonne", Some(&f.modele)),
        };
        Vue {
            etat,
            modele: modele.cloned(),
            pret: matches!(self.etat, Etat::Connecte(_) | Etat::Etalonne(_)),
            mesurable: self.mesurable(),
            probleme: self.probleme.as_ref().map(|p| VueProbleme {
                code: p.code(),
                ecran: match p.ecran() {
                    Ecran::NonDetecte => "non_detecte",
                    Ecran::ChoixDossier => "choix_dossier",
                    Ecran::Etalonnage => "etalonnage",
                    Ecran::Mesure => "mesure",
                },
                guide_cablage: p.guide_cablage(),
                autoriser_pare_feu: p.autoriser_pare_feu(),
                detail: p.detail().map(String::from),
            }),
        }
    }
}

/// Choisit la DLL et le pont. Les emplacements sont essayés dans l'ordre ;
/// dans chacun, une DLL 64 bits qui a son pont est préférée, sinon une
/// 32 bits qui a le sien. Le premier emplacement qui contient une DLL
/// décide : sans pont pour elle, la recherche s'arrête là. `nom_dll` : la DLL
/// cherchée (`FDXSDK.dll`, `FD9SDK.dll`) ; `nom_pont` : son pont, pour le détail.
fn choisir(
    nom_dll: &str,
    nom_pont: &str,
    emplacements: &[PathBuf],
    ponts: &[(Architecture, PathBuf)],
) -> Result<(PathBuf, PathBuf), Probleme> {
    let mut examines = String::new();
    let mut trouvees = 0;
    for emplacement in emplacements {
        let dlls = chercher_dlls(emplacement, nom_dll);
        if dlls.is_empty() {
            let _ = writeln!(examines, "{} : aucun {nom_dll}", emplacement.display());
        }
        let mut ici = Vec::new();
        for dll in dlls {
            match architecture(&dll) {
                Some(arch) => {
                    let _ = writeln!(examines, "{} : {}", dll.display(), nom_architecture(arch));
                    ici.push((arch, dll));
                }
                None => {
                    let _ = writeln!(examines, "{} : pas une DLL Windows", dll.display());
                }
            }
        }
        trouvees += ici.len();
        for voulue in [Architecture::X64, Architecture::X86] {
            for (arch, dll) in &ici {
                if *arch != voulue {
                    continue;
                }
                if let Some((_, programme)) = ponts.iter().find(|(a, _)| a == arch) {
                    return Ok((programme.clone(), dll.clone()));
                }
            }
        }
        // Un emplacement qui a une DLL mais pas son pont arrête la recherche :
        // le dossier choisi par l'opérateur ne cède pas en silence la place à
        // un autre logiciel. C'est l'application qui est incomplète.
        if !ici.is_empty() {
            break;
        }
    }
    let examines = examines.trim_end().to_string();
    if trouvees == 0 {
        return Err(Probleme::LogicielAbsent { examines });
    }
    let disponibles: Vec<_> = ponts.iter().map(|(a, _)| nom_architecture(*a)).collect();
    Err(Probleme::PontIntrouvable {
        detail: format!(
            "{examines}\naucun {nom_pont} de cette architecture ; ponts disponibles : {}",
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

/// Premier palier : la version du SDK, preuve que la DLL est chargée.
fn lire_version(pont: &mut impl Pont) -> Result<(), Probleme> {
    match pont.demander(&Requete::Version {})? {
        Reponse::Version { .. } => Ok(()),
        Reponse::Erreur { erreur } => Err(Probleme::LogicielInutilisable {
            detail: format!("version du SDK : {erreur:?}"),
        }),
        autre => Err(inattendue(autre)),
    }
}

/// Paliers suivants, dans l'ordre : détection, connexion au premier
/// instrument détecté. Jamais plus loin que `PLAFOND`.
fn monter(pont: &mut impl Pont) -> Result<Fiche, Probleme> {
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

/// Tous les fichiers `nom_dll` d'un emplacement : la DLL elle-même, ou celles
/// d'un dossier et de ses sous-dossiers (trois niveaux au plus), dans l'ordre
/// des noms. Le pont exécute le code de la DLL qu'on lui donne : un fichier
/// d'un autre nom n'est jamais retenu.
fn chercher_dlls(emplacement: &Path, nom_dll: &str) -> Vec<PathBuf> {
    if emplacement.is_file() {
        let bon_nom = emplacement
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(nom_dll));
        return if bon_nom {
            vec![emplacement.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut trouvees = Vec::new();
    chercher_dans(emplacement, nom_dll, PROFONDEUR, &mut trouvees);
    trouvees
}

fn chercher_dans(dossier: &Path, nom_dll: &str, profondeur: usize, trouvees: &mut Vec<PathBuf>) {
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
            .is_some_and(|n| n.eq_ignore_ascii_case(nom_dll))
        {
            trouvees.push(chemin);
        }
    }
    if profondeur > 0 {
        for sous_dossier in sous_dossiers {
            chercher_dans(&sous_dossier, nom_dll, profondeur - 1, trouvees);
        }
    }
}
