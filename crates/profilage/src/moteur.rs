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
    /// `-qu`, la plus longue.
    Approfondi,
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
}

/// Pourquoi le moteur n'a pas rendu de profil.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurMoteur {
    /// ArgyllCMS n'est ni à côté de l'application ni dans le dépôt.
    Introuvable,
    /// ArgyllCMS est là, mais pas en version [`VERSION_ARGYLL`].
    VersionNonValidee { trouvee: String },
    /// Un outil a échoué ; `cause` est la dernière ligne non vide de sa
    /// sortie d'erreur.
    Echec { etape: Etape, cause: String },
    /// Lecture ou écriture d'un fichier de travail impossible.
    Fichier(String),
}

impl ErreurMoteur {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurMoteur::Introuvable => "profil.erreur.argyll_introuvable",
            ErreurMoteur::VersionNonValidee { .. } => "profil.erreur.argyll_version",
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
            ErreurMoteur::Fichier(_) => "profil.erreur.fichier",
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
            ErreurMoteur::Echec { etape, cause } => {
                let etape = match etape {
                    Etape::Version => "lecture de la version",
                    Etape::Profil => "calcul du profil",
                    Etape::Controle => "contrôle du profil",
                };
                write!(f, "échec à l'étape {etape} : {cause}")
            }
            ErreurMoteur::Fichier(cause) => write!(f, "fichier de travail : {cause}"),
        }
    }
}

impl std::error::Error for ErreurMoteur {}

/// Un moteur de calcul de profil ICC.
pub trait MoteurProfil: Send + Sync {
    /// Commandes équivalentes, sans rien lancer (section « Avancé »).
    fn commandes(&self, demande: &DemandeProfil) -> Vec<String>;
    /// Calcule le profil. En cas d'erreur, rien n'est rendu.
    fn calculer(&self, demande: &DemandeProfil) -> Result<ProfilCalcule, ErreurMoteur>;
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

    fn calculer(&self, demande: &DemandeProfil) -> Result<ProfilCalcule, ErreurMoteur> {
        self.demandes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((demande.ti3.to_string(), demande.recette));
        let icc = self.resultat.clone()?;
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
