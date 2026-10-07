//! Protocole entre l'application et les ponts d'instruments (ADR 0005).
//!
//! Ce crate ne dépend ni de Windows ni d'une DLL de fabricant.

use serde::{Deserialize, Serialize};

/// Demandes de l'application au pont, une ligne JSON par message.
/// La liste est fermée : toute autre commande est rejetée à la lecture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum Requete {
    // Accolades vides : sans elles, serde accepterait des champs en trop.
    Version {},
    Detecter {},
    /// Instrument n° `instrument` de la dernière détection.
    Connecter {
        instrument: u32,
    },
}

/// Lit une requête ; rejette les commandes inconnues, les champs en trop et
/// les valeurs hors type, plutôt que de deviner.
pub fn lire_requete(ligne: &str) -> Result<Requete, String> {
    serde_json::from_str(ligne).map_err(|erreur| erreur.to_string())
}

/// Étapes de la progression imposée sur instrument réel, dans l'ordre.
/// Le pont refuse tout palier au-delà du plafond fixé à son lancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Palier {
    Version,
    Detection,
    Connexion,
    Etalonnage,
    MesurePonctuelle,
    Bande,
}

/// Erreurs qu'un pont peut renvoyer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurPont {
    /// Appel hors de l'ordre des paliers ; la DLL n'a pas été appelée.
    EtatInvalide { attendu: Palier },
    /// Palier au-delà du plafond autorisé ; la DLL n'a pas été appelée.
    PalierNonAutorise { demande: Palier, plafond: Palier },
    /// Instrument désigné absent de la dernière détection.
    InstrumentInconnu,
    /// Code -9992 de la DLL : valeur envoyée refusée (sens supposé).
    ParametreRefuse,
    /// Code -9986 de la DLL : appel refusé dans l'état actuel (sens supposé).
    EtatIncompatible,
    /// Autre code d'erreur de la DLL, conservé brut.
    Sdk { code: i32 },
}
