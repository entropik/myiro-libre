//! Calcul d'un profil ICC à partir d'une mesure (ADR 0002 : ArgyllCMS inclus).
//!
//! Indépendante de Tauri, de Windows et des ponts.
//!
//! - [`preparer_plages`] contrôle les plages d'une mesure importée pour une
//!   condition de mesure : RVB et Lab connus, RVB de 0 à 100 %. Rien n'est
//!   deviné ni complété.
//! - [`ecrire_ti3`] écrit ces plages dans le fichier `.ti3` qu'ArgyllCMS lit.

mod plages;
mod ti3;

pub use plages::{conditions_utilisables, preparer_plages, PlageProfil, PlagesProfil, RefusPlages};
pub use ti3::ecrire_ti3;
