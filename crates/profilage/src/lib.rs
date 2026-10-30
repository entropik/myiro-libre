//! Calcul d'un profil ICC à partir d'une mesure (ADR 0002 : ArgyllCMS inclus).
//!
//! Indépendante de Tauri, de Windows et des ponts.
//!
//! - [`preparer_plages`] contrôle les plages d'une mesure importée pour une
//!   condition de mesure : RVB et Lab connus, RVB de 0 à 100 %. Rien n'est
//!   deviné ni complété.
//! - [`ecrire_ti3`] écrit ces plages dans le fichier `.ti3` qu'ArgyllCMS lit.
//! - [`MoteurProfil`] calcule le profil à partir du `.ti3` : [`ArgyllCms`]
//!   en vrai, version figée [`VERSION_ARGYLL`], [`MoteurSimule`] pour les
//!   tests.
//! - [`MoteurGamut`] calcule la limite du gamut d'un profil (`iccgamut`) :
//!   [`ArgyllCms`] en vrai, [`MoteurGamutSimule`] pour les tests.
//! - [`controler_coherence`] vérifie, avant tout calcul, que les mesures
//!   correspondent aux plages (ticket #1) : il refuse ou laisse passer, sans
//!   jamais corriger.
//! - [`MoteurVerification`] interroge un profil pour le vérifier sur une
//!   seconde mire (`xicclu`) : [`ArgyllCms`] en vrai,
//!   [`MoteurVerificationSimule`] pour les tests.

mod argyll;
mod coherence;
mod evaluation;
mod limite;
mod moteur;
mod plages;
mod suivi;
mod ti3;

pub use argyll::{
    arguments_colprof, arguments_targen, chercher_argyll, controler_version, description_argyll,
    lignes_commandes, lire_precision, lire_version, ArgyllCms,
};
pub use coherence::{
    controler_coherence, ControleCoherence, Critere, PlageSuspecte, DECALAGE_MAX,
    ECART_BLANC_MAX_L, ECART_DOUBLONS_MAX_DE00, ECART_NOIR_MAX_L, ECART_VOISINES_MAX_DE00,
    RETOUR_GRIS_MAX_L, VOISINES,
};
pub use evaluation::{
    arguments_xicclu, lire_sortie_xicclu, DemandeEvaluation, Evaluation, Impression,
    MoteurVerification, MoteurVerificationSimule,
};
pub use limite::{
    arguments_iccgamut, lire_volume_iccgamut, LimiteCalculee, MoteurGamut, MoteurGamutSimule,
};
pub use moteur::PlagesGenerees;
pub use moteur::{
    etape_colprof, lire_avancement, Avancement, DemandeProfil, ErreurMoteur, Etape, EtapeCalcul,
    MoteurProfil, MoteurSimule, Precision, ProfilCalcule, Recette, VERSION_ARGYLL,
};

pub use plages::{conditions_utilisables, preparer_plages, PlageProfil, PlagesProfil, RefusPlages};
pub use suivi::{EtatAvancement, SuiviAvancement};
pub use ti3::ecrire_ti3;
