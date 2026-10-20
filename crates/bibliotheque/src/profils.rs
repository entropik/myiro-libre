//! Profils ICC d'une condition d'impression : un profil actif, l'historique
//! des précédents, leur état de vérification et leur provenance.

use serde::Serialize;

/// Numéro d'un profil dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdProfil(pub i64);

/// Espace de données d'un profil ICC accepté par la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EspaceProfil {
    Rvb,
    Cmjn,
}

/// État de vérification d'un profil.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatVerification {
    NonVerifie,
    Accepte,
    HorsTolerance,
    Incomplet,
}

/// Ce que l'arbre et l'historique montrent d'un profil, sans son fichier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeProfil {
    pub id: IdProfil,
    pub nom: String,
    /// Création du profil, en RFC 3339 UTC.
    pub horodatage: String,
    pub espace: EspaceProfil,
    pub etat: EtatVerification,
    /// SHA-256 du fichier, en hexadécimal minuscule.
    pub empreinte: String,
    pub actif: bool,
}
