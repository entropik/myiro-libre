//! Vérification d'un profil ICC sur une seconde mire (ticket #41), partagée
//! avec le futur RIP (ADR 0004).
//!
//! Indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS : elle prend
//! des RVB, des Lab mesurés et ce que le profil en dit, d'où qu'ils viennent.
//!
//! - [`comparer`] calcule, plage par plage, la **fidélité aux couleurs
//!   demandées** (demande contre mesure ramenée au blanc du support) et
//!   l'**accord avec la prédiction du profil** (prédiction absolue contre
//!   mesure), et partage les plages entre gamut et hors gamut.
//! - [`statistiques`] : moyenne, 95e centile au rang le plus proche,
//!   maximum et pires plages.
//!
//! Une valeur non calculable est refusée ou reste inconnue (`None`), jamais
//! zéro.

mod ecarts;
mod serde_lab;

pub use ecarts::{
    comparer, reclasser, statistiques, vers_relatif, EcartPlage, PlageAVerifier, RefusVerification,
    ResultatPlage, Resultats, Statistiques, ValeursProfil, PIRES,
};
