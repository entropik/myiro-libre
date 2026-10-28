//! Mesures d'un profil : fidélité du profil à ses mesures, répétabilité des
//! plages répétées, mesures communes à deux profils, plages d'un chemin.

use serde::Serialize;

use crate::comparaison::{ecart, statistiques, Statistiques};
use crate::icc::ProfilIcc;

/// Une plage mesurée : RVB de 0 à 1 et L\*, a\*, b\* mesuré.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mesure {
    pub rvb: [f64; 3],
    pub lab: [f64; 3],
}

/// Écarts entre les mesures d'un même RVB.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Repetabilite {
    /// Couleurs mesurées plusieurs fois.
    pub couleurs: usize,
    /// Paires de mesures comparées.
    pub paires: usize,
    pub moyenne: f64,
    pub max: f64,
}

/// Fidélité du profil à des mesures : ΔE00 entre `lab_absolu(rvb)` et le
/// Lab mesuré, plage par plage ; `None` sans mesure.
pub fn fidelite(p: &ProfilIcc, mesures: &[Mesure]) -> Option<Statistiques> {
    let ecarts: Vec<f64> = mesures
        .iter()
        .filter_map(|m| ecart(p.lab_absolu(m.rvb), m.lab))
        .collect();
    statistiques(&ecarts)
}

/// Répétabilité : ΔE00 de toutes les paires de plages aux RVB strictement
/// égaux ; `None` sans plage répétée.
pub fn repetabilite(mesures: &[Mesure]) -> Option<Repetabilite> {
    let mut groupes: Vec<Vec<&Mesure>> = Vec::new();
    for m in mesures {
        match groupes.iter_mut().find(|g| g[0].rvb == m.rvb) {
            Some(groupe) => groupe.push(m),
            None => groupes.push(vec![m]),
        }
    }
    groupes.retain(|g| g.len() > 1);
    let mut ecarts = Vec::new();
    for groupe in &groupes {
        for (i, a) in groupe.iter().enumerate() {
            for b in &groupe[i + 1..] {
                ecarts.extend(ecart(a.lab, b.lab));
            }
        }
    }
    if ecarts.is_empty() {
        return None;
    }
    Some(Repetabilite {
        couleurs: groupes.len(),
        paires: ecarts.len(),
        moyenne: ecarts.iter().sum::<f64>() / ecarts.len() as f64,
        max: ecarts.iter().copied().fold(0.0, f64::max),
    })
}

/// Écart absolu, sur l'échelle RVB de 0 à 1, au-delà duquel deux RVB
/// diffèrent (0,01 % de l'échelle).
const RVB_MEME: f64 = 1e-4;
/// Écart ΔE00 au-delà duquel deux plages ne sont plus la même mesure.
const ECART_MEME: f64 = 0.1;

/// Les deux séries sont-elles les mêmes mesures ? Même nombre de plages,
/// mêmes RVB à 0,01 % près une fois triées, et au plus 0,1 ΔE00 plage par
/// plage.
pub fn memes_mesures(a: &[Mesure], b: &[Mesure]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    triees(a).iter().zip(triees(b)).all(|(x, y)| {
        (0..3).all(|k| (x.rvb[k] - y.rvb[k]).abs() <= RVB_MEME)
            && ecart(x.lab, y.lab).is_some_and(|e| e <= ECART_MEME)
    })
}

/// Plages triées par RVB, puis par Lab : les plages répétées d'un même RVB
/// se rangent de la même façon, quel que soit l'ordre du fichier.
fn triees(m: &[Mesure]) -> Vec<&Mesure> {
    let mut v: Vec<&Mesure> = m.iter().collect();
    v.sort_by(|x, y| {
        x.rvb
            .iter()
            .chain(&x.lab)
            .zip(y.rvb.iter().chain(&y.lab))
            .map(|(p, q)| p.total_cmp(q))
            .find(|o| o.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

/// Distance au segment, en RVB de 0 à 1, sous laquelle une plage est sur le
/// chemin.
const PROCHE_DU_CHEMIN: f64 = 0.5 / 255.0;

/// Plages mesurées le long du segment du cube RVB de `depart` à `arrivee` :
/// (position de 0 à 255, L\*, C\*) des plages à 0,5/255 au plus du segment,
/// rangées le long du chemin.
pub fn points_du_chemin(mesures: &[Mesure], depart: [u8; 3], arrivee: [u8; 3]) -> Vec<[f64; 3]> {
    let d = depart.map(|v| f64::from(v) / 255.0);
    let u: Vec<f64> = (0..3)
        .map(|k| f64::from(arrivee[k]) / 255.0 - d[k])
        .collect();
    let longueur2: f64 = u.iter().map(|x| x * x).sum();
    let mut points: Vec<[f64; 3]> = mesures
        .iter()
        .filter_map(|m| {
            let w: Vec<f64> = (0..3).map(|k| m.rvb[k] - d[k]).collect();
            let t = if longueur2 > 0.0 {
                ((0..3).map(|k| w[k] * u[k]).sum::<f64>() / longueur2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance2: f64 = (0..3).map(|k| (w[k] - t * u[k]).powi(2)).sum();
            (distance2.sqrt() <= PROCHE_DU_CHEMIN)
                .then(|| [t * 255.0, m.lab[0], m.lab[1].hypot(m.lab[2])])
        })
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]));
    points
}
