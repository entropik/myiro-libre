//! Échange des mesures au format CGATS.17 (ADR 0001 : les données ne sont
//! jamais enfermées ; ADR 0004 : format partagé avec le futur RIP).
//!
//! Indépendante de Tauri, de Windows et des ponts. Format décrit dans
//! `docs/formats/cgats.md`.
//!
//! - [`ecrire`] écrit une mesure de myiro-libre : un tableau par spectre de
//!   plage (M0, M1, M2), Lab et spectre de chaque plage, provenance complète
//!   dans l'en-tête. Une donnée inconnue y est écrite `inconnue`, jamais
//!   remplacée par une valeur par défaut.
//! - [`lire`] relit un fichier CGATS.17, de myiro-libre ou d'un autre logiciel,
//!   en [`MesureImportee`] : ce que le fichier ne donne pas reste inconnu.

mod ecriture;
mod lecture;
mod provenance;

use std::fmt;

use pont_protocole::{ConditionMesure, Echantillonnage, Info, Lab, Provenance, Spectre};

pub use ecriture::ecrire;
pub use lecture::lire;

/// Version du CGATS écrit par myiro-libre (mot-clé `MYIRO_LIBRE_FORMAT`).
pub const FORMAT_CGATS: &str = "myiro-libre/cgats/1";

/// Défaut d'un fichier CGATS ou d'une mesure à écrire, expliqué en clair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErreurCgats(pub String);

impl fmt::Display for ErreurCgats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErreurCgats {}

pub(crate) fn erreur<T>(detail: impl Into<String>) -> Result<T, ErreurCgats> {
    Err(ErreurCgats(detail.into()))
}

/// Une mesure lue dans un fichier CGATS : elle est **importée**, ce n'est pas
/// une mesure attestée par un pont de myiro-libre.
///
/// Toutes les données sont qualifiées : `Confirmee` veut dire « écrite telle
/// quelle dans le fichier », `Supposee` « déduite d'une convention de
/// logiciel » (voir `docs/formats/cgats.md`), `Inconnue` « absente du
/// fichier ». Un fichier sans spectre donne des spectres inconnus : tout ce
/// qui exige un spectre reste inconnu.
#[derive(Clone, Debug, PartialEq)]
pub struct MesureImportee {
    /// Provenance complète, quand le fichier a été écrit par myiro-libre.
    /// C'est ce que le fichier déclare, pas ce qu'un pont atteste.
    pub provenance: Info<Provenance>,
    /// Instrument déclaré par le fichier (`INSTRUMENTATION`).
    pub instrument: Info<String>,
    /// Numéro de série déclaré par le fichier (`SERIAL`).
    pub numero_serie: Info<String>,
    /// Date déclarée par le fichier (`CREATED`), telle qu'écrite.
    pub date: Info<String>,
    /// Condition de mesure de chacun des trois emplacements de spectre d'une
    /// plage (`m0`, `m1`, `m2`).
    pub conditions: [Info<ConditionMesure>; 3],
    /// Longueurs d'onde des spectres, si le fichier les nomme.
    pub longueurs_onde: Info<Echantillonnage>,
    pub plages: Vec<PlageImportee>,
}

/// Une plage d'une mesure importée.
#[derive(Clone, Debug, PartialEq)]
pub struct PlageImportee {
    /// `SAMPLE_ID` du fichier, ou rang de la plage (à partir de 1) s'il n'en a pas.
    pub identifiant: String,
    /// Spectres des emplacements `m0`, `m1`, `m2`.
    pub spectres: [Info<Spectre>; 3],
    /// Lab des emplacements `m0`, `m1`, `m2`.
    pub lab: [Info<Lab>; 3],
}

/// Noms des trois emplacements de spectre d'une plage.
pub(crate) const EMPLACEMENTS: [&str; 3] = ["m0", "m1", "m2"];
