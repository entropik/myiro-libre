//! Profils ICC d'une condition d'impression : un profil actif, l'historique
//! des précédents, leur état de vérification et leur provenance.

use serde::Serialize;

/// Numéro d'un profil dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdProfil(pub i64);
