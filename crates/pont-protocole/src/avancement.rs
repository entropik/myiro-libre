//! Avancement d'une feuille au FD-9 (ticket #34) : ce que le pont annonce
//! pendant la requête, avant sa réponse finale (`prenumerisee`, `mesure` ou
//! une erreur). Une ligne `{"rep":"avancement",…}` par étape, dans l'ordre.
//!
//! Facultatif : un pont qui n'en écrit pas (écrit avant le ticket #34) ne
//! rend que sa réponse finale, et l'application la lit comme avant.

use serde::{Deserialize, Serialize};

use crate::CalageFeuille;

/// Une étape de la feuille en cours, annoncée par le pont.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "etape", rename_all = "snake_case", deny_unknown_fields)]
pub enum AvancementFeuille {
    // Accolades vides : sans elles, serde accepterait des champs en trop.
    /// L'instrument a pris la feuille : premier état 2 ou 3 du SDK (que ces
    /// états suivent l'entrée de la feuille est supposé, d'après leur nom).
    FeuilleEngagee {},
    /// Prise d'image acceptée : image, QR et reconnaissance des plages.
    Lecture {},
    /// Plages trouvées sur l'image et contrôlées : points visés et image
    /// réduite, avant la mesure (palier Feuille) ou l'arrêt
    /// (Prénumérisation). Le calage ainsi annoncé est rendu : la réponse
    /// finale ne le renvoie pas.
    CalagePret { calage: CalageFeuille },
    /// `FD9_StartMeasurement2` accepté : mesure en cours, sans compte tant
    /// que le FD-9 n'en a pas donné.
    MesureLancee {},
    /// Mesure en cours : `mesurees` sur `plages`, le 4ᵉ nombre d'un
    /// événement 11 du rappel. Qu'il compte les plages déjà mesurées est
    /// supposé (nom tiré du symbole de la bibliothèque Mac, table des
    /// états) ; aucun compte n'est annoncé sans événement 11.
    Mesure { mesurees: u32, plages: u32 },
}
