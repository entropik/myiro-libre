//! Plan figé au départ d'une séance.

use serde::{Deserialize, Serialize};

/// La mire mesurée, telle que la bibliothèque l'a rangée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteMire {
    /// Numéro de la mire dans la bibliothèque.
    pub id: i64,
    pub nom: String,
    /// Empreinte de la mire (SHA-256 du TIFF pour le FD-9).
    pub empreinte: String,
}

/// Une plage que la séance attend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlageAttendue {
    /// « 1 », « 2 »… dans l'ordre de la mire : le `SAMPLE_ID` des mesures
    /// de myiro-libre et des exports de FD-S2w.
    pub identifiant: String,
    /// RVB envoyés à l'imprimante, de 0 à 100.
    pub rvb: [f64; 3],
}

/// Le plan d'une séance : la mire, ses plages et la fiche d'impression
/// confirmée, copiés au départ. Il ne change plus ensuite, même si la mire
/// ou la fiche sont modifiées ailleurs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub mire: IdentiteMire,
    pub plages: Vec<PlageAttendue>,
    /// Fiche d'impression confirmée, en JSON, telle que reçue.
    pub fiche: String,
}

/// Pourquoi un plan n'est pas figé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusPlan {
    /// La fiche d'impression n'a pas été confirmée par l'imprimeur.
    FicheNonConfirmee,
    /// La mire n'a aucune plage.
    MireVide,
    /// Une valeur RVB de la mire n'est pas entre 0 et 100.
    ValeurHorsEchelle { identifiant: String },
}

impl Plan {
    /// Fige le plan d'une séance : `valeurs` sont les RVB de la mire (de 0
    /// à 100, dans l'ordre des plages), `fiche` la fiche d'impression en
    /// JSON, `fiche_confirmee` la case cochée par l'imprimeur. Les plages
    /// sont numérotées à partir de 1, dans l'ordre de la mire.
    pub fn figer(
        mire: IdentiteMire,
        valeurs: &[[f64; 3]],
        fiche: &str,
        fiche_confirmee: bool,
    ) -> Result<Plan, RefusPlan> {
        if !fiche_confirmee {
            return Err(RefusPlan::FicheNonConfirmee);
        }
        if valeurs.is_empty() {
            return Err(RefusPlan::MireVide);
        }
        let mut plages = Vec::with_capacity(valeurs.len());
        for (rang, rvb) in valeurs.iter().enumerate() {
            let identifiant = (rang + 1).to_string();
            if rvb.iter().any(|c| !(0.0..=100.0).contains(c)) {
                return Err(RefusPlan::ValeurHorsEchelle { identifiant });
            }
            plages.push(PlageAttendue {
                identifiant,
                rvb: *rvb,
            });
        }
        Ok(Plan {
            mire,
            plages,
            fiche: fiche.to_string(),
        })
    }
}
