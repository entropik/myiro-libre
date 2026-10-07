//! Protocole entre l'application et les ponts d'instruments (ADR 0005).
//!
//! Une ligne JSON par message, dans chaque sens. Ce crate ne dépend ni de
//! Windows ni d'une DLL de fabricant.

mod mesure;

pub use mesure::*;
use mesure::{est_mesure_initiale, mesure_initiale};

use serde::{Deserialize, Serialize};

/// Demandes de l'application au pont.
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
    /// Étalonnage sur le blanc : l'instrument est posé sur son capuchon.
    Etalonner {},
    /// L'instrument attend l'appui sur son bouton, posé sur une plage.
    MesurerPonctuelle {},
    /// L'instrument attend un passage le long d'une rangée ; avec
    /// `plages_attendues`, une lecture qui n'en compte pas autant est refusée.
    MesurerBande {
        #[serde(default)]
        plages_attendues: Option<u32>,
    },
    /// Désarme, se déconnecte et termine le pont.
    Fermer {},
}

/// Lit une requête ; rejette les commandes inconnues, les champs en trop et
/// les valeurs hors type, plutôt que de deviner.
pub fn lire_requete(ligne: &str) -> Result<Requete, String> {
    serde_json::from_str(ligne).map_err(|erreur| erreur.to_string())
}

/// Réponses du pont, une par requête.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rep", rename_all = "snake_case")]
pub enum Reponse {
    /// Les trois nombres rendus par la DLL, tels quels.
    Version {
        parties: [u32; 3],
    },
    Instruments {
        liste: Vec<InstrumentDetecte>,
    },
    Connecte {
        identite: Identite,
    },
    Etalonne {},
    /// Une plage en mesure ponctuelle, une par plage reconnue en bande, avec
    /// sa provenance (ADR 0005), au format conservable versionné.
    Mesure {
        mesure: Mesure,
    },
    Ferme {},
    Erreur {
        erreur: ErreurPont,
    },
    /// La ligne reçue n'est pas une requête valable ; rien n'a été fait.
    RequeteInvalide {
        detail: String,
    },
}

pub fn ecrire_reponse(reponse: &Reponse) -> String {
    serde_json::to_string(reponse).expect("une réponse se sérialise toujours")
}

/// Lit une réponse du pont. Une réponse `mesure` du format initial (pont
/// 0.1.0, sans numéro de format) est reprise par [`lire_mesure`].
pub fn lire_reponse(ligne: &str) -> Result<Reponse, String> {
    let valeur: serde_json::Value =
        serde_json::from_str(ligne).map_err(|erreur| erreur.to_string())?;
    if est_mesure_initiale(&valeur) {
        return mesure_initiale(valeur)
            .map(|mesure| Reponse::Mesure { mesure })
            .map_err(|erreur| erreur.to_string());
    }
    serde_json::from_value(valeur).map_err(|erreur| erreur.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentDetecte {
    /// « usb », « reseau », ou « inconnue N ».
    pub liaison: String,
    /// `COMn`, `/dev/cu.usbmodem…` ou adresse IP.
    pub port: String,
    pub numero_serie: u32,
}

/// Identité de l'instrument connecté (fiche `docs/abi/FDX_GetDeviceInfo.md`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identite {
    pub numero_serie: u32,
    pub micrologiciel: String,
    pub code_produit: String,
    pub adresse_mac: String,
    /// AAAAMMJJ ; absente si l'instrument porte encore la date d'usine.
    pub date_initiale: Option<u32>,
    pub anomalie_date_initiale: bool,
    /// Les 40 octets bruts, en hexadécimal, pour réinterprétation ultérieure.
    pub brute_hex: String,
}

/// Étapes de la progression imposée sur instrument réel, dans l'ordre.
/// Le pont refuse tout palier au-delà du plafond fixé à son lancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Palier {
    Version,
    Detection,
    Connexion,
    Etalonnage,
    MesurePonctuelle,
    Bande,
}

/// Erreurs qu'un pont peut renvoyer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ErreurPont {
    /// Appel hors de l'ordre des paliers ; la DLL n'a pas été appelée.
    EtatInvalide { attendu: Palier },
    /// Palier au-delà du plafond autorisé ; la DLL n'a pas été appelée.
    PalierNonAutorise { demande: Palier, plafond: Palier },
    /// Instrument désigné absent de la dernière détection.
    InstrumentInconnu,
    /// Code -9992 de la DLL : valeur envoyée refusée.
    ParametreRefuse,
    /// Code -9986 de la DLL : appel refusé dans l'état actuel.
    EtatIncompatible,
    /// L'instrument a signalé l'échec de l'étalonnage (événement 9), avec
    /// le code d'erreur de la DLL à cet instant.
    EtalonnageEchoue { erreur: i32 },
    /// Code -9983 : l'instrument n'est pas (ou plus) étalonné.
    NonEtalonne,
    /// Aucun étalonnage utilisable dans la connexion en cours (jamais fait,
    /// échoué, expiré, ou refusé par l'instrument) : étalonner avant de
    /// mesurer. La DLL n'a pas été appelée.
    EtalonnageRequis,
    /// La connexion est ouverte mais l'identité de l'instrument n'a pas pu
    /// être lue : rien ne peut se faire avant une nouvelle connexion. La DLL
    /// n'a pas été appelée.
    SessionInexploitable,
    /// L'instrument a signalé l'échec de la mesure (événement 4).
    MesureEchouee { erreur: i32 },
    /// La DLL a rendu une réponse de forme imprévue (nombre ou taille de résultats).
    ReponseInattendue { detail: String },
    /// L'instrument n'a pas répondu dans le délai prévu.
    Delai,
    /// L'instrument s'est déconnecté ou ne répond plus (événement 6). La
    /// session et son étalonnage sont invalidés : toute demande suivante reçoit
    /// cette même erreur, sans appel à la DLL, jusqu'à une nouvelle connexion.
    InstrumentPerdu,
    /// Autre code d'erreur de la DLL, conservé brut.
    Sdk { code: i32 },
}
