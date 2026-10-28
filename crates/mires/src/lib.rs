//! Mires de profilage RVB, à imprimer sans gestion des couleurs.
//!
//! Indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS (ADR 0004,
//! ADR 0007) :
//!
//! - **FD-9** : la mire est RGB-2520, reprise telle quelle (décision du
//!   mainteneur, 26 octobre 2026). La crate en vérifie l'empreinte et la
//!   géométrie, et en lit les valeurs au centre des plages ;
//! - **MYIRO-1** : pages en bandes ([`Gabarit`]), plages choisies ailleurs
//!   (`targen`, par la crate `profilage`) puis placées, dessinées avec un
//!   texte lisible et un QR code, et écrites en TIFF.

use std::fmt;

use serde::{Deserialize, Serialize};

mod gabarit;
mod mise_en_page;
mod rgb2520;
mod ti1;

pub use gabarit::{
    Gabarit, Rectangle, CONTRASTE_MIN_DE76, DISTANCE_TEXTE_MM, MYIRO1_BANDEAU_MM,
    MYIRO1_BANDE_MAX_MM, MYIRO1_BLANC_BOUT_MM, MYIRO1_COTE_MIN_MM, MYIRO1_COTE_PAR_DEFAUT_MM,
    MYIRO1_ECART_BANDES_MM, MYIRO1_MARGE_PAGE_MM,
};
pub use mise_en_page::{
    estimer_lab, mettre_en_page, verifier, Defaut, MiseEnPage, Page, PlagePlacee,
};
pub use rgb2520::{
    lire_mire_fd9, lire_mire_fd9_sans_empreinte, GeometrieFd9, MireFd9, RGB2520_EMPREINTE,
    RGB2520_FICHIER, RGB2520_GEOMETRIE, RGB2520_NOM,
};
pub use ti1::lire_ti1;

/// Instrument qui lira la mire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    Fd9,
    Myiro1,
}

/// Format de la feuille.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    A4,
    A3,
}

impl Format {
    /// Largeur et hauteur en millimètres, en portrait.
    pub fn dimensions(self) -> (f64, f64) {
        match self {
            Format::A4 => (210.0, 297.0),
            Format::A3 => (297.0, 420.0),
        }
    }
}

/// Taille d'une mire MYIRO-1, en nombre de plages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TailleMire {
    Rapide,
    Standard,
    Approfondi,
}

/// Nombre de plages d'une taille de mire MYIRO-1. Tailles validées par le
/// mainteneur le 23 octobre 2026, comme point de départ à corriger après
/// essais.
pub fn nombre_plages_myiro1(taille: TailleMire) -> usize {
    match taille {
        TailleMire::Rapide => 232,
        TailleMire::Standard => 464,
        TailleMire::Approfondi => 928,
    }
}

/// Pourquoi une mire n'a pas pu être préparée.
#[derive(Debug, PartialEq)]
pub enum ErreurMire {
    /// Côté de plage hors des limites du manuel, ou sans place sur la page.
    CoteHorsLimites { min: f64, max: Option<f64> },
    /// Fichier `.ti1` illisible.
    Ti1(String),
    /// Pas le nombre de plages demandé.
    NombreInattendu { attendu: usize, trouve: usize },
    /// Des plages restent trop proches de leur voisine ou du blanc papier.
    Contraste { defauts: usize },
    /// Écriture d'une page impossible.
    Ecriture(String),
    /// La mire RGB-2520 n'est pas posée à côté de l'application.
    MireFd9Introuvable,
    /// Fichier lu, mais ce n'est pas la mire RGB-2520 retenue.
    MireFd9Autre { empreinte: String },
    /// TIFF illisible, ou géométrie différente de celle attendue.
    MireFd9Illisible(String),
}

impl ErreurMire {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurMire::CoteHorsLimites { .. } => "mire.erreur.cote",
            ErreurMire::Ti1(_) => "mire.erreur.ti1",
            ErreurMire::NombreInattendu { .. } => "mire.erreur.nombre",
            ErreurMire::Contraste { .. } => "mire.erreur.contraste",
            ErreurMire::Ecriture(_) => "mire.erreur.ecriture",
            ErreurMire::MireFd9Introuvable => "mire.erreur.fd9_introuvable",
            ErreurMire::MireFd9Autre { .. } => "mire.erreur.fd9_autre",
            ErreurMire::MireFd9Illisible(_) => "mire.erreur.fd9_illisible",
        }
    }
}

impl fmt::Display for ErreurMire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurMire::CoteHorsLimites { min, max: None } => {
                write!(
                    f,
                    "côté de plage refusé : {min} mm au moins, et la bande doit tenir sur la page"
                )
            }
            ErreurMire::CoteHorsLimites {
                min,
                max: Some(max),
            } => {
                write!(f, "côté de plage refusé : de {min} à {max} mm")
            }
            ErreurMire::Ti1(cause) => write!(f, "fichier .ti1 illisible : {cause}"),
            ErreurMire::NombreInattendu { attendu, trouve } => {
                write!(f, "{trouve} plages au lieu de {attendu}")
            }
            ErreurMire::Contraste { defauts } => {
                write!(f, "plages trop proches pour être lues : {defauts} défauts")
            }
            ErreurMire::Ecriture(cause) => write!(f, "écriture impossible : {cause}"),
            ErreurMire::MireFd9Introuvable => f.write_str("mire RGB-2520 introuvable"),
            ErreurMire::MireFd9Autre { empreinte } => write!(
                f,
                "ce fichier n'est pas la mire RGB-2520 (empreinte {empreinte})"
            ),
            ErreurMire::MireFd9Illisible(cause) => {
                write!(f, "mire RGB-2520 illisible : {cause}")
            }
        }
    }
}

impl std::error::Error for ErreurMire {}
