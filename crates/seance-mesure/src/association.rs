//! Association des plages d'une mesure à celles du plan.

use std::collections::HashMap;

use cgats::MesureImportee;
use serde::{Deserialize, Serialize};

use crate::Plan;

/// Écart toléré entre un RVB de la mesure et celui de la mire, sur 255 :
/// un demi-pas de 8 bits (comme la vérification d'un profil).
const ECART_RVB: f64 = 0.5;

/// Ce qui rend une association incertaine. Chaque doute nomme ses plages
/// par leur identifiant dans la mesure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "doute", rename_all = "snake_case")]
pub enum Doute {
    /// La mesure ne donne pas les RVB de ces plages : rien ne prouve que ce
    /// sont les couleurs de la mire.
    RvbInconnus { plages: Vec<String> },
    /// Les RVB de ces plages ne sont pas ceux de la mire.
    RvbDifferents { plages: Vec<String> },
    /// Ces identifiants ne sont pas dans le plan : la mesure a des plages en
    /// plus, jamais ignorées en silence.
    IdentifiantsInconnus { plages: Vec<String> },
    /// Ces identifiants apparaissent plusieurs fois : aucune des mesures
    /// n'est choisie.
    Doublons { plages: Vec<String> },
}

/// Ce qu'une mesure apporte au plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Association {
    /// Pour chaque plage du plan, dans son ordre : le rang de la plage de la
    /// mesure qui lui correspond, ou rien.
    pub rangs: Vec<Option<usize>>,
    /// Plages du plan que la mesure ne donne pas.
    pub manquantes: Vec<String>,
    /// Vide : l'association est certaine.
    pub doutes: Vec<Doute>,
}

impl Association {
    /// Aucun doute : chaque plage donnée par la mesure est bien celle du
    /// plan. Une association incertaine bloque le calcul d'un profil.
    pub fn certaine(&self) -> bool {
        self.doutes.is_empty()
    }

    /// Aucune plage du plan ne manque.
    pub fn complete(&self) -> bool {
        self.manquantes.is_empty()
    }

    /// Identifiants nommés par un doute.
    pub(crate) fn douteuses(&self) -> impl Iterator<Item = &str> {
        self.doutes.iter().flat_map(|d| match d {
            Doute::RvbInconnus { plages }
            | Doute::RvbDifferents { plages }
            | Doute::IdentifiantsInconnus { plages }
            | Doute::Doublons { plages } => plages.iter().map(String::as_str),
        })
    }
}

/// Rapproche les plages d'une mesure de celles du plan, par leur
/// identifiant, puis contrôle que leurs RVB sont ceux de la mire (à un
/// demi-pas sur 255 près). Rien n'est deviné : aucune plage n'est remise
/// en ordre d'après sa couleur, aucune n'est écartée en silence. Une plage
/// en plus du plan, en double, sans RVB ou d'un autre RVB rend
/// l'association incertaine ; une plage absente la rend incomplète.
pub fn associer(plan: &Plan, mesure: &MesureImportee) -> Association {
    let index: HashMap<&str, usize> = plan
        .plages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.identifiant.as_str(), i))
        .collect();
    let mut rangs: Vec<Option<usize>> = vec![None; plan.plages.len()];
    let mut doublees = vec![false; plan.plages.len()];
    let mut inconnus = Vec::new();
    for (rang, p) in mesure.plages.iter().enumerate() {
        match index.get(p.identifiant.as_str()) {
            None => inconnus.push(p.identifiant.clone()),
            Some(&i) if rangs[i].is_some() || doublees[i] => doublees[i] = true,
            Some(&i) => rangs[i] = Some(rang),
        }
    }
    let mut doublons = Vec::new();
    let mut manquantes = Vec::new();
    let mut sans_rvb = Vec::new();
    let mut differents = Vec::new();
    for (i, attendue) in plan.plages.iter().enumerate() {
        if doublees[i] {
            rangs[i] = None;
            doublons.push(attendue.identifiant.clone());
            continue;
        }
        let Some(rang) = rangs[i] else {
            manquantes.push(attendue.identifiant.clone());
            continue;
        };
        match mesure.plages[rang].rvb.valeur() {
            None => sans_rvb.push(attendue.identifiant.clone()),
            Some(v) => {
                let ecart = (0..3)
                    .map(|k| (f64::from(v[k]) * 255.0 - attendue.rvb[k] * 2.55).abs())
                    .fold(0.0, f64::max);
                if ecart > ECART_RVB {
                    differents.push(attendue.identifiant.clone());
                }
            }
        }
    }
    let mut doutes = Vec::new();
    if !sans_rvb.is_empty() {
        doutes.push(Doute::RvbInconnus { plages: sans_rvb });
    }
    if !differents.is_empty() {
        doutes.push(Doute::RvbDifferents { plages: differents });
    }
    if !inconnus.is_empty() {
        doutes.push(Doute::IdentifiantsInconnus { plages: inconnus });
    }
    if !doublons.is_empty() {
        doutes.push(Doute::Doublons { plages: doublons });
    }
    Association {
        rangs,
        manquantes,
        doutes,
    }
}

/// Où en est une plage du plan dans une mesure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatPlage {
    Associee,
    /// Nommée par un doute de l'association.
    Douteuse,
    Manquante,
}

/// Une plage du plan, l'attendu à côté du mesuré. Les valeurs sont celles
/// des fichiers : aucune couleur d'écran n'est calculée ici, et un aperçu à
/// l'écran n'est jamais une preuve colorimétrique.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VuePlage {
    pub identifiant: String,
    /// RVB de la mire, de 0 à 100.
    pub rvb_attendu: [f64; 3],
    /// RVB écrits dans la mesure, ramenés de 0 à 100 ; rien s'ils sont
    /// inconnus ou si la plage manque.
    pub rvb_fichier: Option<[f64; 3]>,
    /// Lab mesurés aux emplacements `m0`, `m1`, `m2`.
    pub lab: [Option<[f32; 3]>; 3],
    pub etat: EtatPlage,
}

/// Chaque plage du plan, dans son ordre, avec ce que la mesure en donne :
/// de quoi inspecter une mesure incomplète ou incertaine.
pub fn vue_plages(
    plan: &Plan,
    mesure: &MesureImportee,
    association: &Association,
) -> Vec<VuePlage> {
    let douteuses: Vec<&str> = association.douteuses().collect();
    plan.plages
        .iter()
        .zip(&association.rangs)
        .map(|(attendue, rang)| {
            let mesuree = rang.and_then(|r| mesure.plages.get(r));
            let etat = if douteuses.contains(&attendue.identifiant.as_str()) {
                EtatPlage::Douteuse
            } else if mesuree.is_some() {
                EtatPlage::Associee
            } else {
                EtatPlage::Manquante
            };
            VuePlage {
                identifiant: attendue.identifiant.clone(),
                rvb_attendu: attendue.rvb,
                rvb_fichier: mesuree
                    .and_then(|p| p.rvb.valeur())
                    .map(|v| v.map(|c| f64::from(c) * 100.0)),
                lab: match mesuree {
                    Some(p) => p.lab.each_ref().map(|l| l.valeur().map(|l| l.valeurs())),
                    None => [None, None, None],
                },
                etat,
            }
        })
        .collect()
}
