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
    /// Désarme, se déconnecte, puis répond `ferme` ou `fermeture_incertaine`
    /// et termine le pont. Si la déconnexion échoue, le pont répond par une
    /// erreur et reste à l'écoute : seul un nouveau `fermer` touche la DLL.
    Fermer {},
}

/// Lit une requête ; rejette les commandes inconnues, les champs en trop et
/// les valeurs hors type, plutôt que de deviner.
pub fn lire_requete(ligne: &str) -> Result<Requete, String> {
    serde_json::from_str(ligne).map_err(|erreur| erreur.to_string())
}

/// Réponses du pont, une par requête. Comme pour les requêtes, un champ en
/// trop est refusé à la lecture plutôt qu'ignoré.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rep", rename_all = "snake_case", deny_unknown_fields)]
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
        /// Désarmement qui a suivi la lecture : la mesure reste valable même
        /// si le repos n'est pas prouvé. Absent des réponses écrites avant le
        /// ticket #24 : relu comme inconnu, jamais comme réussi. Ce n'est pas
        /// une donnée de la mesure conservée (format `myiro-libre/mesure/1`).
        #[serde(default = "Info::inconnue")]
        remise_au_repos: Info<RemiseAuRepos>,
    },
    /// Fermeture confirmée : désarmement prouvé puis déconnexion faite. Le pont
    /// s'arrête.
    Ferme {},
    /// Déconnexion faite, mais remise au repos non prouvée : l'instrument peut
    /// demander une intervention. Le pont s'arrête aussi.
    FermetureIncertaine {
        remise_au_repos: RemiseAuRepos,
    },
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
            .map(|mesure| Reponse::Mesure {
                mesure,
                remise_au_repos: Info::Inconnue,
            })
            .map_err(|erreur| erreur.to_string());
    }
    serde_json::from_value(valeur).map_err(|erreur| erreur.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentDetecte {
    /// « usb », « reseau », ou « inconnue N ».
    pub liaison: String,
    /// `COMn`, `/dev/cu.usbmodem…` ou adresse IP.
    pub port: String,
    pub numero_serie: u32,
}

/// Identité de l'instrument connecté (fiche `docs/abi/FDX_GetDeviceInfo.md`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Remise au repos : résultat d'un désarmement (`FDX_StopMeasurement`) et de
/// l'attente du retour au repos (événement 0). `au_repos` et `repos_suppose`
/// permettent d'armer ; seul `au_repos` confirme une fermeture ; les autres
/// cas disent pourquoi le repos n'est pas prouvé. Une absence de preuve n'est
/// jamais une réussite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "etat", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemiseAuRepos {
    // Accolades vides : sans elles, serde accepterait des champs en trop.
    /// Événement 0 reçu après un désarmement accepté ; ou désarmement refusé
    /// (-9986) sans autre événement alors que le repos avait déjà été prouvé
    /// par l'événement 0, sans armement depuis.
    AuRepos {},
    /// Désarmement refusé (-9986) sans événement, sans armement depuis le
    /// lancement du pont ou la dernière déconnexion : le refus est constaté au repos (fiche `FDX_StopMeasurement`),
    /// mais en déduire le repos est une supposition. Permet d'armer ; ne
    /// confirme pas une fermeture.
    ReposSuppose {},
    /// Le désarmement accepté n'a pas été suivi de l'événement 0 dans le délai.
    ReposNonSignale {},
    /// La DLL a refusé le désarmement (code brut), essais épuisés.
    ArretRefuse { code: i32 },
    /// Liaison perdue (événement 6) avant la preuve du repos.
    LiaisonPerdue {},
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
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ErreurPont {
    // Accolades vides sur les erreurs sans détail : sans elles, serde
    // accepterait des champs en trop. Le JSON reste `{"type":"delai"}`.
    /// Appel hors de l'ordre des paliers ; la DLL n'a pas été appelée.
    EtatInvalide { attendu: Palier },
    /// Palier au-delà du plafond autorisé ; la DLL n'a pas été appelée.
    PalierNonAutorise { demande: Palier, plafond: Palier },
    /// Instrument désigné absent de la dernière détection.
    InstrumentInconnu {},
    /// Code -9992 de la DLL : valeur envoyée refusée.
    ParametreRefuse {},
    /// Code -9986 de la DLL : appel refusé dans l'état actuel.
    EtatIncompatible {},
    /// L'instrument a signalé l'échec de l'étalonnage (événement 9), avec
    /// le code d'erreur de la DLL à cet instant.
    EtalonnageEchoue { erreur: i32 },
    /// Code -9983 : l'instrument n'est pas (ou plus) étalonné.
    NonEtalonne {},
    /// Aucun étalonnage utilisable dans la connexion en cours (jamais fait,
    /// échoué, expiré, ou refusé par l'instrument) : étalonner avant de
    /// mesurer. La DLL n'a pas été appelée.
    EtalonnageRequis {},
    /// La connexion est ouverte mais l'identité de l'instrument n'a pas pu
    /// être lue : rien ne peut se faire avant une nouvelle connexion. La DLL
    /// n'a pas été appelée.
    SessionInexploitable {},
    /// L'instrument a signalé l'échec de la mesure (événement 4).
    MesureEchouee { erreur: i32 },
    /// La DLL a rendu une réponse de forme imprévue (nombre ou taille de résultats).
    ReponseInattendue { detail: String },
    /// L'instrument n'a pas répondu dans le délai prévu.
    Delai {},
    /// L'instrument s'est déconnecté ou ne répond plus (événement 6). La
    /// session et son étalonnage sont invalidés : toute demande suivante reçoit
    /// cette même erreur, sans appel à la DLL, jusqu'à une nouvelle connexion.
    InstrumentPerdu {},
    /// La remise au repos de l'instrument n'est pas prouvée (`remise_au_repos`
    /// dit pourquoi) : l'instrument n'a pas été armé. Une mesure déjà rendue
    /// reste valable. Une reconnexion ne lève pas ce doute : seul un
    /// désarmement accepté suivi de l'événement 0 le lève, ou une fermeture
    /// suivie d'un nouveau pont.
    ReposIncertain { remise_au_repos: RemiseAuRepos },
    /// `FDX_Disconnect` a échoué (code brut) : la fermeture n'est pas faite.
    /// `remise_au_repos` est le résultat du désarmement qui l'a précédée ; une
    /// nouvelle demande `fermer` ne refait que la déconnexion.
    DeconnexionEchouee {
        code: i32,
        remise_au_repos: RemiseAuRepos,
    },
    /// Une fermeture a été demandée : plus rien n'est transmis à la DLL, sauf
    /// une nouvelle demande `fermer` si la déconnexion a échoué.
    SessionFermee {},
    /// Autre code d'erreur de la DLL, conservé brut.
    Sdk { code: i32 },
}
