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
    /// L'instrument a pris la feuille (états 2 ou 3 du SDK).
    FeuilleEngagee {},
    /// Prise d'image acceptée : image, QR et reconnaissance des plages.
    Lecture {},
    /// Plages trouvées sur l'image et contrôlées : points visés et image
    /// réduite, avant la mesure (palier Feuille) ou l'arrêt
    /// (Prénumérisation).
    CalagePret { calage: CalageFeuille },
    /// Mesure en cours : `mesurees` plages déjà mesurées sur `plages`, tel
    /// que le rappel de la DLL le compte (état 11, 4ᵉ nombre) ; 0 au départ.
    Mesure { mesurees: u32, plages: u32 },
}
