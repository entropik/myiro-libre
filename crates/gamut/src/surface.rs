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
