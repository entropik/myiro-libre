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
    /// FD-9 en réseau : connexion directe à une adresse saisie, sans
    /// détection (paramètre de connexion propre au FD-9, ADR 0005). Un pont
    /// qui n'a pas de connexion par adresse répond `requete_invalide`.
    ConnecterAdresse {
        adresse: AdresseReseau,
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
    /// Version d'une DLL sans fonction publique de version (FD9SDK) : la
    /// version écrite dans la ressource du fichier et son empreinte SHA-256,
    /// lues sans appel à la DLL (fiche `docs/abi/FD9_GetLastError.md`).
    VersionDll {
        /// `FileVersion` de la ressource : majeur, mineur, révision, build.
        version_fichier: Info<[u32; 4]>,
        empreinte: Info<Empreinte>,
    },
    /// Détection du FD-9 : son identifiant est un texte, pas un nombre.
    InstrumentsFd9 {
        liste: Vec<InstrumentFd9>,
    },
    Connecte {
        identite: Identite,
    },
    /// Étalonnage réussi. `date` est l'heure du pont, avec fuseau, à la fin
    /// de l'étalonnage : celle que porteront les mesures qui suivent
    /// (`Provenance::etalonnage`). Une réponse sans date est refusée.
    Etalonne {
        date: Horodatage,
    },
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

/// Un FD-9 vu par `FD9_GetDeviceList` (fiche `docs/abi/FD9_GetDeviceList.md`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentFd9 {
    /// « reseau », « usb », ou « inconnue N ».
    pub liaison: String,
    /// Adresse IP `a.b.c.d` en réseau, `COMn` en USB.
    pub adresse: String,
    /// 8 caractères rendus par la DLL, tels quels : n° de série de
    /// l'instrument en USB ; en réseau, son sens est supposé.
    pub identifiant: String,
}

/// Adresse d'un FD-9 en réseau : adresse IP ou nom d'hôte, 23 caractères
/// ASCII visibles au plus. La DLL n'en recopie que 24 octets sans ajouter de
/// zéro final (fiche `docs/abi/FD9_Connect.md`) : plus longue, elle serait lue
/// au-delà de son texte. Le port (49152) est fixé dans la DLL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AdresseReseau(String);

/// Nombre de caractères permis dans une adresse réseau du FD-9.
pub const LONGUEUR_MAX_ADRESSE: usize = 23;

impl AdresseReseau {
    pub fn new(texte: impl Into<String>) -> Result<Self, String> {
        let texte = texte.into();
        if texte.is_empty() {
            Err("adresse vide".into())
        } else if !texte.bytes().all(|o| o.is_ascii_graphic()) {
            Err(format!(
                "adresse « {texte} » : caractères ASCII visibles seulement"
            ))
        } else if texte.len() > LONGUEUR_MAX_ADRESSE {
            Err(format!(
                "adresse « {texte} » : {LONGUEUR_MAX_ADRESSE} caractères au plus"
            ))
        } else {
            Ok(AdresseReseau(texte))
        }
    }

    pub fn texte(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AdresseReseau {
    type Error = String;
    fn try_from(texte: String) -> Result<Self, String> {
        AdresseReseau::new(texte)
    }
}

impl From<AdresseReseau> for String {
    fn from(adresse: AdresseReseau) -> Self {
        adresse.0
    }
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
    /// reste valable. Une reconnexion ne lève pas cette incertitude : seul un
    /// désarmement accepté suivi de l'événement 0 la lève, ou une fermeture
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
