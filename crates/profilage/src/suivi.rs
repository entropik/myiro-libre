//! Suivi de l'avancement d'un calcul pour l'écran : ce que la barre et la
//! ligne « en cours » montrent, tiré des annonces du moteur ([`Avancement`]).
//!
//! Chaque étape occupe une part égale de la barre. Elle avance d'une part à
//! chaque étape et, dans une étape, seulement d'après un pourcentage écrit
//! par ArgyllCMS. Elle ne recule jamais. Quand rien de réel ne la fait
//! avancer, un reflet dit que le calcul travaille.

use serde::Serialize;

use crate::moteur::{Avancement, EtapeCalcul};

/// Ce que l'écran montre d'un calcul en cours.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EtatAvancement {
    /// Étape en cours.
    pub etape: EtapeCalcul,
    /// Étapes faites, de 0 à `total` : la valeur annoncée de la barre.
    pub faites: usize,
    pub total: usize,
    /// Position de la barre, de 0 à 1. Ne recule jamais.
    pub position: f64,
    /// Plus haute valeur de la suite de pourcentages en cours, écrite par
    /// ArgyllCMS ; elle vaut pour une partie du calcul, pas pour le tout.
    pub pourcentage: Option<u8>,
    /// Dernière ligne écrite par ArgyllCMS.
    pub ligne: Option<String>,
    /// Rien de réel ne fait avancer la barre en ce moment : un reflet dit que
    /// le calcul travaille.
    pub reflet: bool,
}

/// Suit les annonces d'un calcul, de la préparation au contrôle.
#[derive(Clone, Debug)]
pub struct SuiviAvancement {
    etat: EtatAvancement,
}

impl Default for SuiviAvancement {
    fn default() -> Self {
        Self::new()
    }
}

/// Rang d'une étape dans [`EtapeCalcul::TOUTES`].
fn rang(etape: EtapeCalcul) -> usize {
    EtapeCalcul::TOUTES
        .iter()
        .position(|e| *e == etape)
        .unwrap_or(0)
}

impl SuiviAvancement {
    pub fn new() -> Self {
        SuiviAvancement {
            etat: EtatAvancement {
                etape: EtapeCalcul::Preparation,
                faites: 0,
                total: EtapeCalcul::TOUTES.len(),
                position: 0.0,
                pourcentage: None,
                ligne: None,
                reflet: true,
            },
        }
    }

    pub fn etat(&self) -> &EtatAvancement {
        &self.etat
    }

    /// Tient compte d'une annonce et rend le nouvel état.
    pub fn recevoir(&mut self, annonce: &Avancement) -> &EtatAvancement {
        let e = &mut self.etat;
        let total = e.total as f64;
        match annonce {
            // Une étape déjà passée ne fait rien reculer.
            Avancement::Etape(etape) if rang(*etape) > e.faites => {
                e.etape = *etape;
                e.faites = rang(*etape);
                e.position = e.position.max(e.faites as f64 / total);
                e.pourcentage = None;
                e.ligne = None;
                e.reflet = true;
            }
            Avancement::Etape(_) => {}
            // Une ligne de texte clôt la suite de pourcentages en cours.
            Avancement::Ligne(texte) => {
                e.ligne = Some(texte.clone());
                e.pourcentage = None;
                e.reflet = true;
            }
            Avancement::Pourcentage(p) => {
                // colprof écrit chaque suite en montant : la valeur reçue est
                // la plus haute de la suite en cours, et une valeur plus basse
                // ouvre une nouvelle suite.
                let suite = (*p).min(100);
                e.pourcentage = Some(suite);
                let ici = (e.faites as f64 + f64::from(suite) / 100.0) / total;
                // La suite en cours ne fait avancer la barre que si elle va
                // plus loin ; sinon le reflet revient, la barre ne paraît pas
                // figée.
                e.reflet = ici + 1e-9 < e.position;
                e.position = e.position.max(ici);
            }
        }
        &self.etat
    }
}
