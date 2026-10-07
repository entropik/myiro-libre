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
    /// L'instrument a signalé l'échec de l'étalonnage (événement 9), avec
    /// le code d'erreur de la DLL à cet instant.
    EtalonnageEchoue { erreur: i32 },
    /// Code -9983 : l'instrument n'est pas (ou plus) étalonné.
    NonEtalonne,
    /// L'instrument a signalé l'échec de la mesure (événement 4).
    MesureEchouee { erreur: i32 },
    /// La DLL a rendu une réponse de forme imprévue (nombre ou taille de résultats).
    ReponseInattendue(String),
    /// L'instrument n'a pas répondu dans le délai prévu.
    Delai,
    /// L'instrument s'est déconnecté ou ne répond plus (événement 6).
    InstrumentPerdu,
    /// Autre code d'erreur de la DLL, conservé brut.
    Sdk { code: i32 },
}
