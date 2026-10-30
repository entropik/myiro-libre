//! Surface du cube RVB, maillage en Lab et volume qu'il enferme.

use colorimetrie::lab_vers_srgb;
use serde::Serialize;

use crate::icc::{vers_lab, ProfilIcc};

/// Maillage fermé, sommets en L\*, a\*, b\*.
#[derive(Debug, Clone, PartialEq)]
pub struct Maillage {
    pub sommets: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// Maillage prêt pour l'écran : sommets en L\*, a\*, b\* et couleur écran
/// (`#rrggbb`) de chacun.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaillageAffiche {
    pub sommets: Vec<[f64; 3]>,
    pub couleurs: Vec<String>,
    pub triangles: Vec<[u32; 3]>,
}

/// Les six faces du cube RVB (de 0 à 1), `n` points par arête (au moins 2).
/// Les sommets sont répétés d'une face à l'autre : 6·n² sommets et
/// 12·(n−1)² triangles, tournés vers l'extérieur du cube.
pub fn surface_cube(n: usize) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let n = n.max(2);
    let pas = (n - 1) as f64;
    let mut sommets = Vec::with_capacity(6 * n * n);
    let mut triangles = Vec::with_capacity(12 * (n - 1) * (n - 1));
    for k in 0..3 {
        // Axes de la face, dans l'ordre circulaire : e_u × e_v = e_k.
        let (u, v) = ((k + 1) % 3, (k + 2) % 3);
        for cote in [0.0, 1.0] {
            let debut = sommets.len();
            for i in 0..n {
                for j in 0..n {
                    let mut p = [0.0; 3];
                    p[k] = cote;
                    p[u] = i as f64 / pas;
                    p[v] = j as f64 / pas;
                    sommets.push(p);
                }
            }
            let indice = |i: usize, j: usize| (debut + i * n + j) as u32;
            for i in 0..n - 1 {
                for j in 0..n - 1 {
                    let (a, b, c, d) = (
                        indice(i, j),
                        indice(i + 1, j),
                        indice(i + 1, j + 1),
                        indice(i, j + 1),
                    );
                    // Chaque carré est coupé par la diagonale b–d, comme
                    // `faces()` du prototype : sur une surface courbe, le
                    // volume dépend de la diagonale choisie.
                    // Face du haut : normale +e_k ; face du bas : sens inverse.
                    if cote == 1.0 {
                        triangles.push([a, b, d]);
                        triangles.push([b, c, d]);
                    } else {
                        triangles.push([a, d, b]);
                        triangles.push([b, d, c]);
                    }
                }
            }
        }
    }
    (sommets, triangles)
}

/// Surface du cube RVB à `n` points par arête, passée par le profil en
/// colorimétrie absolue.
pub(crate) fn maillage_profil(p: &ProfilIcc, n: usize) -> Maillage {
    let (sommets, triangles) = surface_cube(n);
    Maillage {
        sommets: sommets.into_iter().map(|rvb| p.lab_absolu(rvb)).collect(),
        triangles,
    }
}

/// Volume enfermé par un maillage fermé, en unités Lab au cube : valeur
/// absolue de Σ p0 · (p1 × p2) / 6, coordonnées dans l'ordre (a\*, b\*, L\*).
pub fn volume(m: &Maillage) -> f64 {
    let point = |i: u32| {
        let [l, a, b] = m.sommets.get(i as usize).copied().unwrap_or([0.0; 3]);
        [a, b, l]
    };
    let somme: f64 = m
        .triangles
        .iter()
        .map(|t| {
            let [p0, p1, p2] = t.map(point);
            let croix = [
                p1[1] * p2[2] - p1[2] * p2[1],
                p1[2] * p2[0] - p1[0] * p2[2],
                p1[0] * p2[1] - p1[1] * p2[0],
            ];
            p0[0] * croix[0] + p0[1] * croix[1] + p0[2] * croix[2]
        })
        .sum();
    (somme / 6.0).abs()
}

/// Coupe d'un maillage par le plan de clarté `l` : pour chacune des
/// `teintes` colonnes, le C\* le plus loin où le rayon issu de l'axe neutre,
/// au milieu de la colonne, croise la section. `None` si aucun rayon de la
/// colonne ne la croise.
pub(crate) fn coupe_plan(m: &Maillage, l: f64, teintes: usize) -> Vec<Option<f64>> {
    let rayons: Vec<[f64; 2]> = (0..teintes)
        .map(|h| {
            let t = ((h as f64 + 0.5) * 360.0 / teintes as f64).to_radians();
            [t.cos(), t.sin()]
        })
        .collect();
    let mut ligne = vec![None::<f64>; teintes];
    for t in &m.triangles {
        let Some(v) = t
            .iter()
            .map(|&i| m.sommets.get(i as usize).copied())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        // Points où les arêtes du triangle traversent le plan, en (a*, b*).
        let traversees: Vec<[f64; 2]> = [(0, 1), (1, 2), (2, 0)]
            .iter()
            .filter(|&&(i, j)| (v[i][0] >= l) != (v[j][0] >= l))
            .map(|&(i, j)| {
                let k = (l - v[i][0]) / (v[j][0] - v[i][0]);
                [
                    v[i][1] + k * (v[j][1] - v[i][1]),
                    v[i][2] + k * (v[j][2] - v[i][2]),
                ]
            })
            .collect();
        let [q, r] = traversees[..] else {
            continue;
        };
        let e = [r[0] - q[0], r[1] - q[1]];
        for (case, d) in ligne.iter_mut().zip(&rayons) {
            // Rayon t·d et segment q + s·e : t = (q × e) / (d × e).
            let det = d[0] * e[1] - d[1] * e[0];
            if det.abs() < 1e-12 {
                continue;
            }
            let t = (q[0] * e[1] - q[1] * e[0]) / det;
            let s = (q[0] * d[1] - q[1] * d[0]) / det;
            if t.is_finite() && t >= 0.0 && (0.0..=1.0).contains(&s) {
                *case = Some(case.map_or(t, |c: f64| c.max(t)));
            }
        }
    }
    ligne
}

/// Couleur écran `#rrggbb` d'un L\*, a\*, b\*, et si elle a été ramenée dans
/// l'écran. Une valeur non finie donne le noir, dit ramené.
pub(crate) fn couleur_ecran(lab: [f64; 3]) -> (String, bool) {
    match lab_vers_srgb(vers_lab(lab)) {
        Ok(srgb) => (srgb.hexadecimal(), srgb.ramenee),
        Err(_) => ("#000000".into(), true),
    }
}

/// Couleurs écran des sommets d'un maillage, pour l'affichage.
pub fn maillage_affiche(m: &Maillage) -> MaillageAffiche {
    MaillageAffiche {
        sommets: m.sommets.clone(),
        couleurs: m.sommets.iter().map(|&s| couleur_ecran(s).0).collect(),
        triangles: m.triangles.clone(),
    }
}
