//! Gamut des profils ICC RVB, pour comparer des profils (ticket #71).
//!
//! La crate lit la table **A2B1** (colorimétrique relative) d'un profil de
//! sortie RVB, de type `mft2` (lut16), et la passe en **colorimétrie
//! absolue** avec le blanc du support (`wtpt`) : le blanc du papier est
//! compris dans les valeurs. Un profil d'une autre structure, sans table ou
//! sans blanc du support est refusé en le disant (`ErreurIcc`).
//!
//! Elle ne dépend que de `colorimetrie` : ni Windows, ni l'interface, ni les
//! ponts. Son interface publique est un contrat (ADR 0004).

mod essai;
mod icc;
mod table;

pub use essai::{profil_essai, ProfilEssai};
pub use icc::{ErreurIcc, ProfilIcc};
