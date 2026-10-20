//! Parc du poste : imprimantes, papiers et résolutions d'impression. Une
//! condition d'impression est la feuille de l'arbre imprimante → papier →
//! résolution (GLOSSARY).

use serde::Serialize;

/// Numéro d'une imprimante dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdImprimante(pub i64);

/// Numéro d'un papier dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdPapier(pub i64);
