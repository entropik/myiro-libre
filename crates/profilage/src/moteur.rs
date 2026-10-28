//! Le moteur qui calcule un profil ICC à partir d'un `.ti3`, derrière un
//! trait : ArgyllCMS en vrai ([`crate::ArgyllCms`]), un moteur simulé pour
//! les tests (sur le modèle de l'ADR 0005).

use std::fmt;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::argyll::lignes_commandes;

/// Seule version d'ArgyllCMS acceptée : en changer est une décision
/// explicite (ADR 0002).
pub const VERSION_ARGYLL: &str = "3.5.0";

/// Qualité du calcul demandée à `colprof`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recette {
    /// `-qm`, pour un essai.
    Rapide,
    /// `-qh`, conseillée en production.
    Standard,
    /// En cours de réglage, pas encore disponible. Il lançait `-qu`, que la
    /// documentation de `colprof` donne pour un mode d'essai à ne pas
    /// utiliser (essai du 28 octobre 2026 : plus de 12 minutes sur un seul
    /// cœur, sans finir). Son contenu sera choisi par un essai sur des
    /// mesures réelles ; `-qu` n'est jamais lancé.
    Approfondi,
}

impl Recette {
    /// Option de qualité de `colprof`, ou rien pour une recette en cours de
    /// réglage.
    pub fn qualite(self) -> Option<&'static str> {
        match self {
            Recette::Rapide => Some("-qm"),
            Recette::Standard => Some("-qh"),
            Recette::Approfondi => None,
        }
    }

    /// La recette peut être calculée.
    pub fn disponible(self) -> bool {
        self.qualite().is_some()
    }
}

/// Ce qu'on demande au moteur : les plages au format `.ti3`, la description
/// du profil et la qualité du calcul.
#[derive(Clone, Copy, Debug)]
pub struct DemandeProfil<'a> {
    pub ti3: &'a str,
    pub description: &'a str,
    pub recette: Recette,
}

/// Écarts ΔE00 entre les mesures et ce que le profil prédit (`profcheck -k`).
/// C'est une précision de calcul, pas une vérification sur une autre mire.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Precision {
    pub moyenne_de00: f64,
    pub max_de00: f64,
}

/// Un profil calculé, avec de quoi refaire et comprendre le calcul.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfilCalcule {
    pub icc: Vec<u8>,
    /// Version d'ArgyllCMS qui a calculé le profil.
    pub version: String,
    /// Lignes de commande lancées, chemins réduits à leur nom.
    pub commandes: Vec<String>,
    /// Sorties des outils, dans l'ordre.
    pub journal: String,
    pub precision: Precision,
}

/// Étape du calcul où une erreur survient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Etape {
    /// Lecture de la version d'ArgyllCMS.
    Version,
    /// Calcul du profil (`colprof`).
    Profil,
    /// Contrôle du profil contre les mesures (`profcheck`).
    Controle,
    /// Limite du gamut d'un profil (`iccgamut`, comparaison des profils).
    Gamut,
}

/// Pourquoi le moteur n'a pas rendu de profil.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurMoteur {
    /// ArgyllCMS n'est ni à côté de l'application ni dans le dépôt.
    Introuvable,
    /// ArgyllCMS est là, mais pas en version [`VERSION_ARGYLL`].
    VersionNonValidee { trouvee: String },
    /// ArgyllCMS est là, mais il lui manque cet outil (par exemple
    /// `iccgamut`, pour la limite du gamut).
    OutilManquant { outil: String },
    /// Un outil a échoué ; `cause` est la dernière ligne non vide de sa
    /// sortie d'erreur.
    Echec { etape: Etape, cause: String },
    /// Lecture ou écriture d'un fichier de travail impossible.
    Fichier(String),
    /// Recette en cours de réglage ([`Recette::disponible`]) : rien n'est
    /// lancé.
    RecetteEnReglage,
    /// Un outil s'est arrêté en échec sans écrire d'erreur d'ArgyllCMS
    /// (« … Error - … ») : il a été arrêté de l'extérieur (programme fermé,
    /// poste en veille) ou s'est arrêté brutalement.
    Interrompu { etape: Etape },
}

impl ErreurMoteur {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurMoteur::Introuvable => "profil.erreur.argyll_introuvable",
            ErreurMoteur::VersionNonValidee { .. } => "profil.erreur.argyll_version",
            ErreurMoteur::OutilManquant { .. } => "profil.erreur.outil_manquant",
            ErreurMoteur::Echec {
                etape: Etape::Version,
                ..
            } => "profil.erreur.argyll_illisible",
            ErreurMoteur::Echec {
                etape: Etape::Profil,
                ..
            } => "profil.erreur.calcul",
            ErreurMoteur::Echec {
                etape: Etape::Controle,
                ..
            } => "profil.erreur.controle",
            ErreurMoteur::Echec {
                etape: Etape::Gamut,
                ..
            } => "comparer.erreur.limite",
            ErreurMoteur::Fichier(_) => "profil.erreur.fichier",
            ErreurMoteur::RecetteEnReglage => "profil.erreur.recette_en_reglage",
            ErreurMoteur::Interrompu { .. } => "profil.erreur.interrompu",
        }
    }
}

impl fmt::Display for ErreurMoteur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurMoteur::Introuvable => f.write_str("ArgyllCMS est introuvable"),
            ErreurMoteur::VersionNonValidee { trouvee } => write!(
                f,
                "ArgyllCMS est en version {trouvee}, seule la version {VERSION_ARGYLL} est validée"
            ),
            ErreurMoteur::OutilManquant { outil } => {
                write!(f, "l'outil {outil} d'ArgyllCMS manque")
            }
            ErreurMoteur::Echec { etape, cause } => {
                write!(f, "échec à l'étape {} : {cause}", nom_etape(*etape))
            }
            ErreurMoteur::Fichier(cause) => write!(f, "fichier de travail : {cause}"),
            ErreurMoteur::RecetteEnReglage => {
                f.write_str("recette en cours de réglage, rien n'est lancé")
            }
            ErreurMoteur::Interrompu { etape } => write!(
                f,
                "arrêt sans erreur d'ArgyllCMS à l'étape {} : calcul interrompu",
                nom_etape(*etape)
            ),
        }
    }
}

impl std::error::Error for ErreurMoteur {}

fn nom_etape(etape: Etape) -> &'static str {
    match etape {
        Etape::Version => "lecture de la version",
        Etape::Profil => "calcul du profil",
        Etape::Controle => "contrôle du profil",
        Etape::Gamut => "calcul de la limite du gamut",
    }
}

/// Étape d'un calcul, annoncée à l'écran pendant qu'il tourne. Les parties
/// de `colprof` sont repérées dans ses lignes ([`etape_colprof`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtapeCalcul {
    /// Lecture et contrôle des mesures, écriture du `.ti3` (application).
    Preparation,
    /// Début de `colprof` : courbes et matrice.
    Profil,
    /// `colprof` : table principale (« Create final clut from scattered data »).
    Table,
    /// `colprof` : correspondance des gamuts (« Creating Gamut Mapping »).
    Gamut,
    /// `colprof` : tables inverses (« Creating B to A tables »).
    Inverses,
    /// `colprof` : limite du gamut (« Creating gamut boundary table »).
    Limite,
    /// Contrôle de précision (`profcheck`).
    Controle,
}

impl EtapeCalcul {
    /// Toutes les étapes, dans l'ordre où elles se suivent.
    pub const TOUTES: [EtapeCalcul; 7] = [
        EtapeCalcul::Preparation,
        EtapeCalcul::Profil,
        EtapeCalcul::Table,
        EtapeCalcul::Gamut,
        EtapeCalcul::Inverses,
        EtapeCalcul::Limite,
        EtapeCalcul::Controle,
    ];
}

/// Partie de `colprof` qui commence avec cette ligne (`colprof -v`,
/// ArgyllCMS 3.5.0) ; rien pour une autre ligne.
pub fn etape_colprof(ligne: &str) -> Option<EtapeCalcul> {
    let ligne = ligne.trim();
    [
        ("Create final clut from scattered data", EtapeCalcul::Table),
        ("Creating Gamut Mapping", EtapeCalcul::Gamut),
        ("Creating B to A tables", EtapeCalcul::Inverses),
        ("Creating gamut boundary table", EtapeCalcul::Limite),
    ]
    .into_iter()
    .find(|(debut, _)| ligne.starts_with(debut))
    .map(|(_, etape)| etape)
}

/// Ce que le calcul annonce pendant qu'il tourne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Avancement {
    /// Une étape commence.
    Etape(EtapeCalcul),
    /// Une ligne de texte écrite par l'outil (`colprof -v`), sans ses espaces
    /// de bord.
    Ligne(String),
    /// Un pourcentage écrit par l'outil lui-même, seul sur sa ligne
    /// (« 45% ») : il vaut pour la partie en cours, jamais pour tout le
    /// calcul. Rien n'est estimé.
    Pourcentage(u8),
}

/// Ce qu'une ligne d'un outil annonce : un pourcentage écrit seul sur sa
/// ligne (de 0 à 100), sinon la ligne elle-même ; rien pour une ligne vide
/// ou faite seulement d'astérisques (« ****** » de la table principale).
pub fn lire_avancement(ligne: &str) -> Option<Avancement> {
    let ligne = ligne.trim();
    if ligne.chars().all(|c| c == '*' || c.is_whitespace()) {
        return None;
    }
    let pourcentage = ligne
        .strip_suffix('%')
        .filter(|n| !n.is_empty() && n.len() <= 3 && n.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| *n <= 100);
    Some(match pourcentage {
        Some(p) => Avancement::Pourcentage(p),
        None => Avancement::Ligne(ligne.to_string()),
    })
}

/// Un moteur de calcul de profil ICC.
pub trait MoteurProfil: Send + Sync {
    /// Commandes équivalentes, sans rien lancer (section « Avancé »).
    fn commandes(&self, demande: &DemandeProfil) -> Vec<String>;
    /// Calcule le profil en annonçant son avancement à `avancement` (sur le
    /// fil de l'appelant). En cas d'erreur, rien n'est rendu.
    fn calculer(
        &self,
        demande: &DemandeProfil,
        avancement: &mut dyn FnMut(Avancement),
    ) -> Result<ProfilCalcule, ErreurMoteur>;
}

/// Moteur des tests : rend toujours le même résultat et garde les demandes
/// reçues par `calculer`.
pub struct MoteurSimule {
    resultat: Result<Vec<u8>, ErreurMoteur>,
    demandes: Mutex<Vec<(String, Recette)>>,
}

impl MoteurSimule {
    /// Un moteur qui rend ce profil ICC, avec une précision fictive.
    pub fn reussit(icc: Vec<u8>) -> Self {
        MoteurSimule {
            resultat: Ok(icc),
            demandes: Mutex::new(Vec::new()),
        }
    }

    /// Un moteur qui échoue toujours ainsi.
    pub fn echoue(erreur: ErreurMoteur) -> Self {
        MoteurSimule {
            resultat: Err(erreur),
            demandes: Mutex::new(Vec::new()),
        }
    }

    /// Les demandes reçues par `calculer` : le `.ti3` et la recette.
    pub fn demandes(&self) -> Vec<(String, Recette)> {
        self.demandes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl MoteurProfil for MoteurSimule {
    fn commandes(&self, demande: &DemandeProfil) -> Vec<String> {
        lignes_commandes(demande.recette, demande.description)
    }

    /// Annonce le calcul puis le contrôle, comme ArgyllCMS, sans ligne d'outil.
    fn calculer(
        &self,
        demande: &DemandeProfil,
        avancement: &mut dyn FnMut(Avancement),
    ) -> Result<ProfilCalcule, ErreurMoteur> {
        if !demande.recette.disponible() {
            return Err(ErreurMoteur::RecetteEnReglage);
        }
        self.demandes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((demande.ti3.to_string(), demande.recette));
        avancement(Avancement::Etape(EtapeCalcul::Profil));
        let icc = self.resultat.clone()?;
        avancement(Avancement::Etape(EtapeCalcul::Controle));
        Ok(ProfilCalcule {
            icc,
            version: VERSION_ARGYLL.to_string(),
            commandes: self.commandes(demande),
            journal: "moteur simulé\n".to_string(),
            precision: Precision {
                moyenne_de00: 0.5,
                max_de00: 2.0,
            },
        })
    }
}
