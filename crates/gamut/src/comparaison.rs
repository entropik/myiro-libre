//! Écarts ΔE00 entre deux profils : grille RVB, zones et couleurs de base.

use colorimetrie::delta_e00;
use serde::Serialize;

use crate::analyse::PRIMAIRES;
use crate::icc::{vers_lab, ProfilIcc};

/// Points par arête de la grille RVB des écarts (35 937 valeurs).
const GRILLE_ECARTS: usize = 33;
/// Nombre de plus grands écarts gardés.
const PIRES: usize = 10;
/// Sous cet écart maximal, les deux profils sont dits identiques.
const IDENTIQUES: f64 = 0.005;

/// Résumé d'une série d'écarts ΔE00 (parts en %).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Statistiques {
    pub nombre: usize,
    pub moyenne: f64,
    pub mediane: f64,
    pub p95: f64,
    pub max: f64,
    pub part_sup_1: f64,
    pub part_sup_2: f64,
}

/// Statistiques d'une série d'écarts ; `None` si elle est vide. Les centiles
/// sont interpolés linéairement au rang p/100·(n−1), comme
/// `numpy.percentile`.
pub fn statistiques(ecarts: &[f64]) -> Option<Statistiques> {
    if ecarts.is_empty() {
        return None;
    }
    let mut tries = ecarts.to_vec();
    tries.sort_by(f64::total_cmp);
    let n = tries.len();
    let part = |seuil: f64| 100.0 * tries.iter().filter(|&&e| e > seuil).count() as f64 / n as f64;
    Some(Statistiques {
        nombre: n,
        moyenne: tries.iter().sum::<f64>() / n as f64,
        mediane: centile(&tries, 50.0),
        p95: centile(&tries, 95.0),
        max: tries[n - 1],
        part_sup_1: part(1.0),
        part_sup_2: part(2.0),
    })
}

/// Centile `p` d'une série triée non vide, interpolé au rang p/100·(n−1).
fn centile(tries: &[f64], p: f64) -> f64 {
    let rang = p / 100.0 * (tries.len() - 1) as f64;
    let bas = rang.floor() as usize;
    let haut = rang.ceil() as usize;
    tries[bas] + (rang - bas as f64) * (tries[haut] - tries[bas])
}

/// Famille de teinte d'une couleur RVB (teinte TSV par sixièmes de cercle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Famille {
    Rouge,
    Jaune,
    Vert,
    Cyan,
    Bleu,
    Magenta,
}

const FAMILLES: [Famille; 6] = [
    Famille::Rouge,
    Famille::Jaune,
    Famille::Vert,
    Famille::Cyan,
    Famille::Bleu,
    Famille::Magenta,
];

impl Famille {
    fn nom(self) -> &'static str {
        match self {
            Famille::Rouge => "rouge",
            Famille::Jaune => "jaune",
            Famille::Vert => "vert",
            Famille::Cyan => "cyan",
            Famille::Bleu => "bleu",
            Famille::Magenta => "magenta",
        }
    }
}

/// Zone du cube RVB où l'on résume les écarts. Les zones se recouvrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Zone {
    Gris,
    Ombres,
    Clairs,
    Saturee(Famille),
    Moyenne(Famille),
}

impl Zone {
    /// Clé du texte de la zone (`comparer.zone.gris`,
    /// `comparer.zone.saturee.rouge`…).
    pub fn cle(&self) -> String {
        match self {
            Zone::Gris => "comparer.zone.gris".into(),
            Zone::Ombres => "comparer.zone.ombres".into(),
            Zone::Clairs => "comparer.zone.clairs".into(),
            Zone::Saturee(f) => format!("comparer.zone.saturee.{}", f.nom()),
            Zone::Moyenne(f) => format!("comparer.zone.moyenne.{}", f.nom()),
        }
    }

    /// Rang fixe de la zone, pour ranger ses écarts.
    fn rang(self) -> usize {
        match self {
            Zone::Gris => 0,
            Zone::Ombres => 1,
            Zone::Clairs => 2,
            Zone::Saturee(f) => 3 + f as usize,
            Zone::Moyenne(f) => 9 + f as usize,
        }
    }

    fn toutes() -> impl Iterator<Item = Zone> {
        [Zone::Gris, Zone::Ombres, Zone::Clairs]
            .into_iter()
            .chain(FAMILLES.map(Zone::Saturee))
            .chain(FAMILLES.map(Zone::Moyenne))
    }
}

/// Écarts d'une zone.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EcartZone {
    pub zone: Zone,
    pub nombre: usize,
    pub moyenne: f64,
    pub p95: f64,
    pub max: f64,
}

/// Un point de la grille et son écart entre les deux profils.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PointEcart {
    pub rvb: [u8; 3],
    pub reference: [f64; 3],
    pub compare: [f64; 3],
    pub ecart: f64,
}

/// Écarts entre un profil de référence et un profil comparé.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ecarts {
    pub grille: Statistiques,
    /// Zones non vides, par écart moyen décroissant.
    pub zones: Vec<EcartZone>,
    /// Les plus grands écarts de la grille, du plus fort au plus faible.
    pub pires: Vec<PointEcart>,
    /// Écart de chaque couleur de base, dans l'ordre de `Analyse::primaires`.
    pub primaires: Vec<f64>,
    pub identiques: bool,
}

/// ΔE00 entre deux L\*, a\*, b\* ; `None` s'il ne se calcule pas.
pub(crate) fn ecart(a: [f64; 3], b: [f64; 3]) -> Option<f64> {
    delta_e00(vers_lab(a), vers_lab(b)).ok()
}

/// Écarts ΔE00 de `compare` à `reference` sur la grille RVB 33³ et sur les
/// couleurs de base. Un point dont l'écart ne se calcule pas est écarté.
pub fn comparer(reference: &ProfilIcc, compare: &ProfilIcc) -> Ecarts {
    let pas = (GRILLE_ECARTS - 1) as f64;
    let mut points = Vec::with_capacity(GRILLE_ECARTS.pow(3));
    let mut par_zone = vec![Vec::new(); Zone::toutes().count()];
    for r in 0..GRILLE_ECARTS {
        for g in 0..GRILLE_ECARTS {
            for b in 0..GRILLE_ECARTS {
                let rvb = [r, g, b].map(|v| v as f64 / pas);
                let (la, lb) = (reference.lab_absolu(rvb), compare.lab_absolu(rvb));
                let Some(e) = ecart(la, lb) else {
                    continue;
                };
                for zone in zones(rvb) {
                    par_zone[zone.rang()].push(e);
                }
                points.push(PointEcart {
                    rvb: rvb.map(|v| (v * 255.0).round() as u8),
                    reference: la,
                    compare: lb,
                    ecart: e,
                });
            }
        }
    }
    let valeurs: Vec<f64> = points.iter().map(|p| p.ecart).collect();
    let grille = statistiques(&valeurs).unwrap_or(Statistiques {
        nombre: 0,
        moyenne: f64::NAN,
        mediane: f64::NAN,
        p95: f64::NAN,
        max: f64::NAN,
        part_sup_1: f64::NAN,
        part_sup_2: f64::NAN,
    });
    let mut zones: Vec<EcartZone> = Zone::toutes()
        .filter_map(|zone| {
            let s = statistiques(&par_zone[zone.rang()])?;
            Some(EcartZone {
                zone,
                nombre: s.nombre,
                moyenne: s.moyenne,
                p95: s.p95,
                max: s.max,
            })
        })
        .collect();
    zones.sort_by(|a, b| b.moyenne.total_cmp(&a.moyenne));
    points.sort_by(|a, b| b.ecart.total_cmp(&a.ecart));
    points.truncate(PIRES);
    let primaires: Vec<f64> = PRIMAIRES
        .iter()
        .map(|&(_, rvb)| {
            let rvb = rvb.map(|v| f64::from(v) / 255.0);
            ecart(reference.lab_absolu(rvb), compare.lab_absolu(rvb)).unwrap_or(f64::NAN)
        })
        .collect();
    let identiques = grille.max < IDENTIQUES && primaires.iter().all(|&e| e < IDENTIQUES);
    Ecarts {
        grille,
        zones,
        pires: points,
        primaires,
        identiques,
    }
}

/// Zones d'une couleur RVB de 0 à 1 (seuils du prototype).
fn zones(rvb: [f64; 3]) -> Vec<Zone> {
    let [r, g, b] = rvb;
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let ch = mx - mn;
    let mut zones = Vec::new();
    if ch <= 0.1 {
        zones.push(Zone::Gris);
    }
    if mx <= 0.35 {
        zones.push(Zone::Ombres);
    }
    if mn >= 0.75 {
        zones.push(Zone::Clairs);
    }
    if let Some(famille) = famille(rvb, mx, ch) {
        if ch > 0.6 {
            zones.push(Zone::Saturee(famille));
        } else if ch > 0.1 && mx > 0.35 {
            zones.push(Zone::Moyenne(famille));
        }
    }
    zones
}

/// Famille de la teinte TSV ; aucune pour un gris parfait (ch = 0).
fn famille(rvb: [f64; 3], mx: f64, ch: f64) -> Option<Famille> {
    if ch <= 0.0 {
        return None;
    }
    let [r, g, b] = rvb;
    let hh = if mx == r {
        60.0 * ((g - b) / ch).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / ch + 2.0)
    } else {
        60.0 * ((r - g) / ch + 4.0)
    };
    let sixieme = ((hh + 30.0).rem_euclid(360.0) / 60.0).floor() as usize;
    FAMILLES.get(sixieme).copied()
}

#[cfg(test)]
mod tests {
    use super::{zones, Famille, Zone};

    #[test]
    fn les_zones_suivent_les_seuils_du_prototype() {
        assert_eq!(zones([1.0, 0.0, 0.0]), [Zone::Saturee(Famille::Rouge)]);
        assert_eq!(zones([0.5, 0.5, 0.5]), [Zone::Gris]);
        assert_eq!(zones([0.2, 0.1, 0.1]), [Zone::Gris, Zone::Ombres]);
        assert_eq!(zones([0.9, 0.8, 0.8]), [Zone::Gris, Zone::Clairs]);
        assert_eq!(zones([0.8, 0.5, 0.3]), [Zone::Moyenne(Famille::Rouge)]);
        // Les familles suivent le cercle des teintes TSV.
        assert_eq!(zones([1.0, 1.0, 0.0]), [Zone::Saturee(Famille::Jaune)]);
        assert_eq!(zones([0.0, 1.0, 0.0]), [Zone::Saturee(Famille::Vert)]);
        assert_eq!(zones([0.0, 1.0, 1.0]), [Zone::Saturee(Famille::Cyan)]);
        assert_eq!(zones([0.0, 0.0, 1.0]), [Zone::Saturee(Famille::Bleu)]);
        assert_eq!(zones([1.0, 0.0, 1.0]), [Zone::Saturee(Famille::Magenta)]);
        // Un rouge tirant sur le magenta (teinte 345°) reste rouge.
        assert_eq!(zones([1.0, 0.0, 0.25]), [Zone::Saturee(Famille::Rouge)]);
        assert_eq!(zones([0.0, 0.0, 0.0]), [Zone::Gris, Zone::Ombres]);
    }

    #[test]
    fn la_cle_d_une_zone_nomme_sa_famille() {
        assert_eq!(Zone::Gris.cle(), "comparer.zone.gris");
        assert_eq!(
            Zone::Saturee(Famille::Rouge).cle(),
            "comparer.zone.saturee.rouge"
        );
        assert_eq!(
            Zone::Moyenne(Famille::Cyan).cle(),
            "comparer.zone.moyenne.cyan"
        );
    }
}
