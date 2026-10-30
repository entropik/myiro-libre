//! Calage d'une feuille sur ce que l'instrument a trouvé, et vraisemblance de
//! la mesure (essai E4 bis : mesure faite à côté des plages, rendue « ok »).
//!
//! - Les **points de mesure** (`FD9_GetMeasurePoint`) sont les centres des
//!   plages trouvées sur l'image prise, au 1/100 mm sur le papier : les
//!   points rouges de l'onglet « Measured Point » de FD-S2w. Chaque plage de
//!   la mire doit y être une fois ; leur écart à la grille de la définition
//!   donne le décalage de la feuille, et ce qui reste une fois le décalage
//!   retiré dit si la grille trouvée est bien celle de la mire.
//! - La **vraisemblance** compare les L* mesurés à la clarté des couleurs
//!   imprimées : une mesure à côté des plages ne les suit pas.

use fd9_sys::PointMesure;
use pont_protocole::{DefautPositions, DefinitionFeuille};
use std::collections::HashMap;

/// Décalage de la feuille admis, en 1/100 mm : la tolérance de 21 mm du
/// manuel de FD-S2w pour la reconnaissance (supposé).
pub const DECALAGE_ADMIS: u32 = 2_100;

/// Corrélation minimale entre L* mesurés et clarté des couleurs imprimées.
/// Établie sur deux mesures de la RGB-2520 (supposé) : 0,85 pour une mesure
/// juste de MYIROtools, −0,19 pour celle de l'essai E4 bis, −0,40 pour la
/// mesure juste décalée d'une colonne.
pub const CORRELATION_MIN: f64 = 0.5;

/// Ce que les points de mesure disent de la feuille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calage {
    /// Décalage moyen des points sur la grille de la définition (x, y), en
    /// 1/100 mm.
    pub decalage: (i32, i32),
    /// Plus grand écart d'un point à la grille, décalage retiré.
    pub ecart_max: u32,
}

/// Contrôle les points rendus par la DLL contre la définition : chaque
/// plage une fois (sinon le défaut nommé, `lue` = rang du point depuis 1),
/// puis décalage et écart restant.
pub fn caler(mire: &DefinitionFeuille, points: &[PointMesure]) -> Result<Calage, DefautPositions> {
    // Centre théorique de chaque plage, par (zone, rangée, colonne).
    let mut centres: HashMap<(u32, u32, u32), (i64, i64)> = HashMap::new();
    for (zone, z) in mire.zones.iter().enumerate() {
        for rangee in 0..z.rangees {
            for colonne in 0..z.colonnes {
                let x = i64::from(z.gauche)
                    + i64::from(z.pas_x) * i64::from(colonne)
                    + i64::from(z.largeur_plage) / 2;
                let y = i64::from(z.haut)
                    + i64::from(z.pas_y) * i64::from(rangee)
                    + i64::from(z.hauteur_plage) / 2;
                centres.insert((zone as u32, rangee, colonne), (x, y));
            }
        }
    }
    let mut vus: HashMap<(u32, u32, u32), ()> = HashMap::new();
    let mut ecarts = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        let cle = (p.zone, p.rangee, p.colonne);
        // Bornés par PLAGES_MAX et par les bornes du SDK : conversions exactes.
        let (lue, zone, rangee, colonne) =
            (i as u32 + 1, p.zone, p.rangee as i32, p.colonne as i32);
        let Some(&(cx, cy)) = centres.get(&cle) else {
            return Err(DefautPositions::HorsMire {
                lue,
                zone,
                rangee,
                colonne,
            });
        };
        if vus.insert(cle, ()).is_some() {
            return Err(DefautPositions::Doublon {
                lue,
                zone,
                rangee,
                colonne,
            });
        }
        ecarts.push((i64::from(p.x) - cx, i64::from(p.y) - cy));
    }
    if let Some((zone, rangee, colonne)) = mire_dans_l_ordre(mire).find(|c| !vus.contains_key(c)) {
        return Err(DefautPositions::Manquante {
            zone,
            rangee: rangee as i32,
            colonne: colonne as i32,
        });
    }
    let n = ecarts.len().max(1) as i64;
    let dx = ecarts.iter().map(|e| e.0).sum::<i64>() / n;
    let dy = ecarts.iter().map(|e| e.1).sum::<i64>() / n;
    let ecart_max = ecarts
        .iter()
        .map(|e| (e.0 - dx).abs().max((e.1 - dy).abs()))
        .max()
        .unwrap_or(0);
    let borne = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    Ok(Calage {
        decalage: (borne(dx), borne(dy)),
        ecart_max: ecart_max.min(i64::from(u32::MAX)) as u32,
    })
}

/// Places de la mire dans l'ordre de la définition.
fn mire_dans_l_ordre(mire: &DefinitionFeuille) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
    mire.zones.iter().enumerate().flat_map(|(zone, z)| {
        (0..z.rangees).flat_map(move |rangee| {
            (0..z.colonnes).map(move |colonne| (zone as u32, rangee, colonne))
        })
    })
}

/// Clarté L* (D65, sRGB) d'une couleur RVB imprimée : une approximation,
/// suffisante pour suivre l'ordre des clartés.
pub fn clarte_attendue(rvb: [u8; 3]) -> f64 {
    let lineaire = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let y = 0.2126 * lineaire(rvb[0]) + 0.7152 * lineaire(rvb[1]) + 0.0722 * lineaire(rvb[2]);
    let f = if y > 216.0 / 24_389.0 {
        y.cbrt()
    } else {
        (24_389.0 / 27.0 * y + 16.0) / 116.0
    };
    116.0 * f - 16.0
}

/// Corrélation (Pearson) entre les L* mesurés et la clarté des couleurs
/// imprimées, dans le même ordre. `None` si l'une des deux séries ne varie
/// pas : rien à comparer.
pub fn correlation(l_mesures: &[f32], couleurs: &[[u8; 3]]) -> Option<f64> {
    let n = l_mesures.len().min(couleurs.len());
    if n < 2 {
        return None;
    }
    let x: Vec<f64> = couleurs[..n].iter().map(|&c| clarte_attendue(c)).collect();
    let y: Vec<f64> = l_mesures[..n].iter().map(|&l| f64::from(l)).collect();
    let mx = x.iter().sum::<f64>() / n as f64;
    let my = y.iter().sum::<f64>() / n as f64;
    let sxy: f64 = x.iter().zip(&y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sx = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let sy = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
    if sx < 1e-9 || sy < 1e-9 {
        return None;
    }
    Some(sxy / (sx * sy))
}
