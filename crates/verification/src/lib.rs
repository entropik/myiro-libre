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
mod mire_fd9;
mod serde_lab;
mod tolerances;

pub use mire_fd9::{
    couleurs_verification_fd9, ecrire_xml_fds2w, lire_xml_fds2w, mire_verification_fd9, MireFd9,
    PlageMire, MIRE_FD9_VERSION, RGB2520_NIVEAUX,
};

pub use tolerances::{
    conclure, valider_criteres, Conclusion, Controle, Critere, Criteres, Famille, RefusCriteres,
    Seuils,
};

pub use ecarts::{
    comparer, reclasser, statistiques, vers_relatif, EcartPlage, PlageAVerifier, RefusVerification,
    ResultatPlage, Resultats, Statistiques, ValeursProfil, PIRES,
};
