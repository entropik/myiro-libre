//! Tolérances d'un jeu, contrôles un par un et conclusion d'une
//! vérification.
//!
//! Règle d'acceptation (ADR 0004) : un critère est un seuil renseigné
//! (moyenne, 95e centile ou maximum d'une famille). Un critère dépassé donne
//! « hors tolérance » ; sinon, un critère non calculable donne
//! « incomplet » ; sinon « accepté ». Une famille sans aucun seuil n'est pas
//! requise. Aucun seuil n'est inventé : il n'existe que si l'imprimeur l'a
//! enregistré.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ecarts::{Resultats, Statistiques};

/// Seuils ΔE00 d'une famille ; `None` : pas de critère.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Seuils {
    pub moyenne: Option<f64>,
    pub p95: Option<f64>,
    pub max: Option<f64>,
}

/// Les critères d'un jeu de tolérances.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Criteres {
    /// Fidélité aux couleurs demandées, plages dans le gamut.
    pub fidelite: Seuils,
    /// Accord avec la prédiction du profil.
    pub prediction: Seuils,
    /// Au-delà de cet écart ΔE00 entre la demande et la prédiction, une
    /// couleur demandée est hors gamut.
    pub seuil_gamut: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Famille {
    Fidelite,
    Prediction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Critere {
    Moyenne,
    P95,
    Max,
}

/// Un critère contrôlé : sa limite, la valeur trouvée et s'il est respecté ;
/// `valeur` et `respecte` restent `None` quand la famille n'a pas pu être
/// calculée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Controle {
    pub famille: Famille,
    pub critere: Critere,
    pub limite: f64,
    pub valeur: Option<f64>,
    pub respecte: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conclusion {
    Accepte,
    HorsTolerance,
    Incomplet,
}

/// Pourquoi un jeu de tolérances est refusé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusCriteres {
    /// Aucun seuil renseigné : le jeu n'accepterait rien de mesurable.
    AucunSeuil,
    /// Un seuil nul, négatif ou non fini (seuil de gamut compris).
    SeuilInvalide,
}

impl RefusCriteres {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            RefusCriteres::AucunSeuil => "verif.tolerances.aucun_seuil",
            RefusCriteres::SeuilInvalide => "verif.tolerances.seuil_invalide",
        }
    }
}

impl fmt::Display for RefusCriteres {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RefusCriteres::AucunSeuil => "aucun seuil renseigné",
            RefusCriteres::SeuilInvalide => "un seuil n'est pas un nombre strictement positif",
        })
    }
}

impl std::error::Error for RefusCriteres {}

fn valide(seuil: f64) -> bool {
    seuil.is_finite() && seuil > 0.0
}

impl Seuils {
    fn renseignes(&self) -> [(Critere, Option<f64>); 3] {
        [
            (Critere::Moyenne, self.moyenne),
            (Critere::P95, self.p95),
            (Critere::Max, self.max),
        ]
    }
}

/// Un jeu doit avoir au moins un seuil, et chaque seuil (gamut compris)
/// doit être un nombre fini strictement positif.
pub fn valider_criteres(c: &Criteres) -> Result<(), RefusCriteres> {
    let seuils: Vec<f64> = [c.fidelite, c.prediction]
        .iter()
        .flat_map(|s| s.renseignes())
        .filter_map(|(_, s)| s)
        .collect();
    if !valide(c.seuil_gamut) || seuils.iter().any(|&s| !valide(s)) {
        return Err(RefusCriteres::SeuilInvalide);
    }
    if seuils.is_empty() {
        return Err(RefusCriteres::AucunSeuil);
    }
    Ok(())
}

fn valeur(stats: &Statistiques, critere: Critere) -> f64 {
    match critere {
        Critere::Moyenne => stats.moyenne,
        Critere::P95 => stats.p95,
        Critere::Max => stats.max,
    }
}

/// Un contrôle par seuil renseigné, fidélité puis prédiction, moyenne, 95e
/// centile puis maximum, et la conclusion. Le seuil de gamut des résultats
/// doit être celui des critères : l'appelant passe par
/// [`crate::reclasser`] avant.
pub fn conclure(resultats: &Resultats, criteres: &Criteres) -> (Conclusion, Vec<Controle>) {
    let mut controles = Vec::new();
    for (famille, seuils, stats) in [
        (Famille::Fidelite, criteres.fidelite, &resultats.fidelite),
        (
            Famille::Prediction,
            criteres.prediction,
            &resultats.prediction,
        ),
    ] {
        for (critere, limite) in seuils.renseignes() {
            let Some(limite) = limite else { continue };
            let valeur = stats.as_ref().map(|s| valeur(s, critere));
            controles.push(Controle {
                famille,
                critere,
                limite,
                valeur,
                respecte: valeur.map(|v| v <= limite),
            });
        }
    }
    let conclusion = if controles.iter().any(|c| c.respecte == Some(false)) {
        Conclusion::HorsTolerance
    } else if controles.iter().any(|c| c.respecte.is_none()) {
        Conclusion::Incomplet
    } else {
        Conclusion::Accepte
    };
    (conclusion, controles)
}
