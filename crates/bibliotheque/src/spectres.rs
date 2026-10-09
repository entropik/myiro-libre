//! Spectres d'une mesure rangés par condition de mesure (M0, M1, M2), pour
//! la courbe de spectre (ticket #10). Rien n'est deviné : un spectre dont la
//! condition n'est pas dite ne prend aucune place, et la condition reste
//! « non disponible » ; rien n'est jamais remplacé par zéro.

use cgats::MesureImportee;
use pont_protocole::{ConditionMesure, ConditionsCalcul, Echantillonnage, Info, Mesure};
use serde::Serialize;

/// Les spectres d'une mesure, plage par plage, rangés par condition de mesure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpectresMesure {
    /// Longueur d'onde de chaque valeur d'un spectre, en nm, avec son niveau
    /// de connaissance : inconnue, la courbe ne peut pas être placée.
    pub longueurs_onde: Info<Vec<u32>>,
    pub plages: Vec<SpectresPlage>,
}

/// Une plage : son identifiant et ses spectres dans l'ordre M0, M1, M2.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpectresPlage {
    /// Rang de la plage à partir de 1, ou `SAMPLE_ID` d'un fichier importé.
    pub identifiant: String,
    /// `None` : la mesure n'a pas de spectre dans cette condition (non disponible).
    pub conditions: [Option<SpectreCondition>; 3],
}

/// Le spectre d'une condition de mesure : facteurs de réflexion tels que
/// mesurés (1 = 100 %, plus de 1 pour un papier azuré).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpectreCondition {
    pub valeurs: Vec<f32>,
    /// Condition supposée, pas confirmée : à l'écran, « à confirmer ».
    pub supposee: bool,
}

fn rang(condition: ConditionMesure) -> usize {
    match condition {
        ConditionMesure::M0 => 0,
        ConditionMesure::M1 => 1,
        ConditionMesure::M2 => 2,
    }
}

/// Longueurs d'onde de `n` valeurs, au même niveau de connaissance que
/// l'échantillonnage.
fn longueurs(echantillonnage: Info<Echantillonnage>, n: usize) -> Info<Vec<u32>> {
    let liste = |e: Echantillonnage| -> Vec<u32> {
        (0..n as u32)
            .map(|i| e.debut_nm.saturating_add(e.pas_nm.saturating_mul(i)))
            .collect()
    };
    match echantillonnage {
        Info::Confirmee(e) => Info::Confirmee(liste(e)),
        Info::Supposee(e) => Info::Supposee(liste(e)),
        Info::Inconnue => Info::Inconnue,
    }
}

/// Place chaque emplacement de spectre (`m0`, `m1`, `m2`) sous sa condition
/// dite. Deux emplacements qui disent la même condition : le premier compte.
fn ranger(
    conditions: &[Info<ConditionMesure>; 3],
    spectres: [Option<&[f32]>; 3],
    toutes_supposees: bool,
) -> [Option<SpectreCondition>; 3] {
    let mut rangees: [Option<SpectreCondition>; 3] = [None, None, None];
    for (condition, spectre) in conditions.iter().zip(spectres) {
        let (Some(c), Some(valeurs)) = (condition.valeur(), spectre) else {
            continue;
        };
        let place = &mut rangees[rang(*c)];
        if place.is_none() {
            *place = Some(SpectreCondition {
                valeurs: valeurs.to_vec(),
                supposee: toutes_supposees || matches!(condition, Info::Supposee(_)),
            });
        }
    }
    rangees
}

/// Spectres d'une mesure du pont, d'après ce que le pont a demandé.
pub fn spectres_mesure(mesure: &Mesure) -> SpectresMesure {
    let demande = &mesure.provenance().calcul.demande;
    let (conditions, echantillonnage, supposee) = match demande {
        Info::Confirmee(c) => (
            c.conditions_spectres.clone(),
            c.longueurs_onde.clone(),
            false,
        ),
        Info::Supposee(ConditionsCalcul {
            conditions_spectres,
            longueurs_onde,
            ..
        }) => (
            conditions_spectres.clone(),
            match longueurs_onde {
                Info::Confirmee(e) | Info::Supposee(e) => Info::Supposee(*e),
                Info::Inconnue => Info::Inconnue,
            },
            true,
        ),
        Info::Inconnue => (
            [Info::Inconnue, Info::Inconnue, Info::Inconnue],
            Info::Inconnue,
            false,
        ),
    };
    let n = mesure.plages().first().map_or(0, |p| p.m0().len());
    SpectresMesure {
        longueurs_onde: longueurs(echantillonnage, n),
        plages: mesure
            .plages()
            .iter()
            .enumerate()
            .map(|(i, p)| SpectresPlage {
                identifiant: (i + 1).to_string(),
                conditions: ranger(
                    &conditions,
                    [Some(&p.m0()[..]), Some(&p.m1()[..]), Some(&p.m2()[..])],
                    supposee,
                ),
            })
            .collect(),
    }
}

/// Spectres d'une mesure importée, d'après ce que le fichier déclare.
pub fn spectres_importee(mesure: &MesureImportee) -> SpectresMesure {
    let n = mesure
        .plages
        .iter()
        .flat_map(|p| p.spectres.iter())
        .find_map(|s| s.valeur().map(|s| s.len()))
        .unwrap_or(0);
    SpectresMesure {
        longueurs_onde: longueurs(mesure.longueurs_onde.clone(), n),
        plages: mesure
            .plages
            .iter()
            .map(|p| SpectresPlage {
                identifiant: p.identifiant.clone(),
                conditions: ranger(
                    &mesure.conditions,
                    [0, 1, 2].map(|e| p.spectres[e].valeur().map(|s| &s[..])),
                    false,
                ),
            })
            .collect(),
    }
}
