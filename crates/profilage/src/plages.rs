//! Plages d'une mesure importée, contrôlées pour le calcul d'un profil.

use std::fmt;

use cgats::MesureImportee;
use pont_protocole::{ConditionMesure, Echantillonnage, Info};

/// Une plage prête pour le calcul : ce qui a été envoyé à l'imprimante et ce
/// qui a été mesuré.
#[derive(Clone, Debug, PartialEq)]
pub struct PlageProfil {
    /// Valeurs RVB envoyées à l'imprimante, de 0 à 100 (échelle d'ArgyllCMS).
    pub rvb: [f64; 3],
    pub lab: [f64; 3],
    /// Spectre de réflexion en pourcentage, comme ArgyllCMS l'attend.
    pub spectre: Option<Vec<f64>>,
}

/// Les plages d'une mesure pour une condition de mesure, avec ce qui n'est
/// que supposé.
#[derive(Clone, Debug, PartialEq)]
pub struct PlagesProfil {
    pub plages: Vec<PlageProfil>,
    pub condition: ConditionMesure,
    /// La condition est déduite d'une convention du fabricant (source `D50`).
    pub condition_supposee: bool,
    /// L'échelle des RVB est déduite d'une convention de logiciel (FD-S2w).
    pub rvb_supposes: bool,
    /// Longueurs d'onde des spectres, connues seulement si toutes les plages
    /// ont un spectre de même longueur.
    pub longueurs_onde: Option<Echantillonnage>,
}

/// Pourquoi une mesure ne peut pas servir au calcul d'un profil.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusPlages {
    /// Aucune plage de la mesure n'a cette condition de mesure.
    ConditionAbsente(ConditionMesure),
    /// Aucune colonne RVB, ou une échelle RVB inconnue.
    RvbInconnus,
    /// Une plage sans Lab pour cette condition (identifiant de la plage).
    LabInconnu { plage: String },
    /// Une valeur RVB sous 0 ou au-delà de 100 %.
    RvbHorsEchelle { plage: String },
    /// Une mesure sans plage.
    Vide,
    /// RVB connus, mais aucune condition de mesure n'a un Lab pour toutes
    /// les plages ([`conditions_utilisables`] est vide).
    AucuneCondition,
}

impl RefusPlages {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            RefusPlages::ConditionAbsente(_) => "profil.refus.condition_absente",
            RefusPlages::RvbInconnus => "profil.refus.rvb_inconnus",
            RefusPlages::LabInconnu { .. } => "profil.refus.lab_inconnu",
            RefusPlages::RvbHorsEchelle { .. } => "profil.refus.rvb_hors_echelle",
            RefusPlages::Vide => "profil.refus.vide",
            RefusPlages::AucuneCondition => "profil.refus.aucune_condition",
        }
    }
}

impl fmt::Display for RefusPlages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefusPlages::ConditionAbsente(c) => {
                write!(f, "la mesure ne contient pas la condition {c:?}")
            }
            RefusPlages::RvbInconnus => f.write_str(
                "les valeurs RVB envoyées à l'imprimante sont absentes ou d'échelle inconnue",
            ),
            RefusPlages::LabInconnu { plage } => write!(f, "la plage {plage} n'a pas de Lab"),
            RefusPlages::RvbHorsEchelle { plage } => {
                write!(f, "la plage {plage} a une valeur RVB hors de 0 à 100 %")
            }
            RefusPlages::Vide => f.write_str("la mesure ne contient aucune plage"),
            RefusPlages::AucuneCondition => {
                f.write_str("aucune condition de mesure n'a un Lab pour toutes les plages")
            }
        }
    }
}

impl std::error::Error for RefusPlages {}

/// Emplacement de spectre (`m0`, `m1`, `m2`) qui porte cette condition, s'il
/// est connu du fichier : rien n'est deviné.
fn emplacement(mesure: &MesureImportee, condition: ConditionMesure) -> Option<usize> {
    mesure
        .conditions
        .iter()
        .position(|c| c.valeur() == Some(&condition))
}

/// Prépare les plages d'une mesure importée pour une condition de mesure.
///
/// Refuse une mesure sans plage, des RVB inconnus, une condition absente du
/// fichier, une plage sans Lab, une valeur RVB sous 0 ou au-delà de 100 % :
/// rien n'est deviné ni complété.
pub fn preparer_plages(
    mesure: &MesureImportee,
    condition: ConditionMesure,
) -> Result<PlagesProfil, RefusPlages> {
    if mesure.plages.is_empty() {
        return Err(RefusPlages::Vide);
    }
    if mesure.plages.iter().any(|p| p.rvb.valeur().is_none()) {
        return Err(RefusPlages::RvbInconnus);
    }
    let e = emplacement(mesure, condition).ok_or(RefusPlages::ConditionAbsente(condition))?;
    let mut plages = Vec::with_capacity(mesure.plages.len());
    for p in &mesure.plages {
        let rvb = p.rvb.valeur().ok_or(RefusPlages::RvbInconnus)?;
        if rvb.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err(RefusPlages::RvbHorsEchelle {
                plage: p.identifiant.clone(),
            });
        }
        let lab = p.lab[e].valeur().ok_or_else(|| RefusPlages::LabInconnu {
            plage: p.identifiant.clone(),
        })?;
        plages.push(PlageProfil {
            rvb: rvb.map(|c| f64::from(c) * 100.0),
            lab: lab.valeurs().map(f64::from),
            spectre: None,
        });
    }

    // Spectres pris seulement s'ils sont tous là, de même longueur, et que
    // le fichier nomme leurs longueurs d'onde.
    let spectres: Option<Vec<_>> = mesure
        .plages
        .iter()
        .map(|p| p.spectres[e].valeur())
        .collect();
    let longueurs_onde = match (spectres, mesure.longueurs_onde.valeur()) {
        (Some(spectres), Some(&echantillonnage))
            if spectres.iter().all(|s| s.len() == spectres[0].len()) =>
        {
            for (plage, spectre) in plages.iter_mut().zip(spectres) {
                // En pourcentage, comme ArgyllCMS.
                plage.spectre = Some(spectre.iter().map(|v| f64::from(*v) * 100.0).collect());
            }
            Some(echantillonnage)
        }
        _ => None,
    };

    Ok(PlagesProfil {
        plages,
        condition,
        condition_supposee: matches!(mesure.conditions[e], Info::Supposee(_)),
        rvb_supposes: mesure
            .plages
            .iter()
            .any(|p| matches!(p.rvb, Info::Supposee(_))),
        longueurs_onde,
    })
}

/// Conditions de mesure que le fichier nomme et dont toutes les plages ont
/// un Lab, quand les RVB sont connus. Une valeur RVB hors échelle n'en retire
/// aucune : c'est le calcul qui la refuse, en nommant la plage.
pub fn conditions_utilisables(mesure: &MesureImportee) -> Vec<ConditionMesure> {
    if mesure.plages.is_empty() || mesure.plages.iter().any(|p| p.rvb.valeur().is_none()) {
        return Vec::new();
    }
    [
        ConditionMesure::M0,
        ConditionMesure::M1,
        ConditionMesure::M2,
    ]
    .into_iter()
    .filter(|&c| {
        emplacement(mesure, c)
            .is_some_and(|e| mesure.plages.iter().all(|p| p.lab[e].valeur().is_some()))
    })
    .collect()
}
