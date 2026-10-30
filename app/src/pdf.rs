//! Le rapport en PDF (ticket #89, ADR 0008) : la crate `rapport` le compile
//! en mémoire (Typst en bibliothèque, sans Windows) ; l'application l'écrit
//! à sa place par un temporaire renommé. Un PDF ouvert dans un lecteur ou
//! un dossier protégé est refusé sans rien laisser derrière, et un PDF
//! existant n'est remplacé qu'après succès.

use std::path::Path;

use crate::ecriture::ecrire_atomique;

/// Pourquoi le rapport n'a pas pu être écrit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurPdf {
    /// Le fichier de destination est ouvert, protégé ou dans un dossier
    /// absent.
    Destination(String),
    /// Le moteur de rapport a refusé le document.
    Moteur(String),
}

impl ErreurPdf {
    /// Clé du catalogue de textes.
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurPdf::Destination(_) => "rapport.erreur.destination",
            ErreurPdf::Moteur(_) => "rapport.erreur.moteur",
        }
    }
}

/// Les octets du PDF d'un document.
pub fn fabriquer_pdf(document: &rapport::Document) -> Result<Vec<u8>, ErreurPdf> {
    rapport::pdf(document)
        .map(|p| p.octets)
        .map_err(|e| ErreurPdf::Moteur(format!("{e:?}")))
}

/// Écrit le PDF à `sortie`, par un temporaire renommé.
pub fn enregistrer_pdf(octets: &[u8], sortie: &Path) -> Result<(), ErreurPdf> {
    ecrire_atomique(sortie, octets).map_err(|e| ErreurPdf::Destination(e.to_string()))
}
