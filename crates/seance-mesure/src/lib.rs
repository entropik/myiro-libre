//! Séance de mesure d'un tirage (ticket #35) : ce qui se calcule sans
//! bibliothèque, sans instrument et sans écran.
//!
//! Indépendante de Tauri, de Windows et des ponts.

mod association;
mod moyenne;
mod plan;

pub use association::{associer, vue_plages, Association, Doute, EtatPlage, VuePlage};
pub use moyenne::{
    moyenner, Apport, Complement, CompositionPlage, Ecart, Incompatible, Moyenne, Role, Source,
    SEUIL_ECART_DE00,
};
pub use plan::{IdentiteMire, PlageAttendue, Plan, RefusPlan};
