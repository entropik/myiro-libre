//! Table A2B1 de type `mft2` (lut16) : lecture et évaluation, courbes
//! d'entrée, interpolation tétraédrique et courbes de sortie.

use crate::icc::{signature, u16_at, ErreurIcc};

/// Table `mft2` lue, valeurs ramenées de 0 à 1.
#[derive(Debug, Clone)]
pub(crate) struct Lut {
    pub(crate) grille: usize,
    entree: [Vec<f64>; 3],
    /// Nœuds de la grille, rouge le plus lent, bleu le plus rapide.
    table: Vec<[f64; 3]>,
    sortie: [Vec<f64>; 3],
}

impl Lut {
    /// Courbes d'entrée, interpolation tétraédrique, courbes de sortie.
    pub(crate) fn evaluer(&self, rvb: [f64; 3]) -> [f64; 3] {
        let x = [0, 1, 2].map(|k| interpoler(&self.entree[k], rvb[k]));
        let y = self.tetraedre(x);
        [0, 1, 2].map(|k| interpoler(&self.sortie[k], y[k]))
    }

    /// Interpolation tétraédrique dans la grille ; les six cas et l'ordre des
    /// égalités suivent la méthode de référence du prototype.
    fn tetraedre(&self, x: [f64; 3]) -> [f64; 3] {
        let g = self.grille;
        let mut i = [0usize; 3];
        let mut f = [0f64; 3];
        for k in 0..3 {
            let xf = borner(x[k]) * (g - 1) as f64;
            // Borné de 0 à g − 2 : la conversion est exacte.
            i[k] = (xf.floor() as usize).min(g - 2);
            f[k] = xf - i[k] as f64;
        }
        let noeud = |a: usize, b: usize, c: usize| {
            let index = ((i[0] + a) * g + i[1] + b) * g + i[2] + c;
            self.table.get(index).copied().unwrap_or([0.0; 3])
        };
        let [r, v, b] = f;
        let (p1, p2, w1, w2, w3) = if r >= v && v >= b {
            (noeud(1, 0, 0), noeud(1, 1, 0), r, v, b)
        } else if r >= b && b > v {
            (noeud(1, 0, 0), noeud(1, 0, 1), r, b, v)
        } else if b > r && r >= v {
            (noeud(0, 0, 1), noeud(1, 0, 1), b, r, v)
        } else if v > r && r >= b {
            (noeud(0, 1, 0), noeud(1, 1, 0), v, r, b)
        } else if v >= b && b > r {
            (noeud(0, 1, 0), noeud(0, 1, 1), v, b, r)
        } else {
            (noeud(0, 0, 1), noeud(0, 1, 1), b, v, r)
        };
        let (c000, c111) = (noeud(0, 0, 0), noeud(1, 1, 1));
        [0, 1, 2].map(|k| {
            c000[k] + w1 * (p1[k] - c000[k]) + w2 * (p2[k] - p1[k]) + w3 * (c111[k] - p2[k])
        })
    }
}

/// Interpolation linéaire dans une courbe d'au moins deux points.
fn interpoler(courbe: &[f64], x: f64) -> f64 {
    let n = courbe.len();
    let xf = borner(x) * (n - 1) as f64;
    let i = (xf.floor() as usize).min(n - 2);
    let f = xf - i as f64;
    courbe[i] * (1.0 - f) + courbe[i + 1] * f
}

/// Ramène dans [0, 1] ; une valeur non finie compte pour 0.
fn borner(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(crate) fn lire_mft2(d: &[u8]) -> Result<Lut, ErreurIcc> {
    let tronque = || ErreurIcc::Tronque {
        balise: "A2B1".into(),
    };
    let type_table = d.get(0..4).ok_or_else(tronque)?;
    if type_table != b"mft2" {
        return Err(ErreurIcc::StructureNonPriseEnCharge {
            structure: signature(type_table),
        });
    }
    let (n_entrees, n_sorties, grille) = match d.get(8..11) {
        Some(&[e, s, g]) => (e, s, usize::from(g)),
        _ => return Err(tronque()),
    };
    if n_entrees != 3 || n_sorties != 3 || grille < 2 {
        return Err(ErreurIcc::StructureNonPriseEnCharge {
            structure: format!("mft2 {n_entrees}→{n_sorties}, grille {grille}"),
        });
    }
    // La matrice (octets 12 à 47) ne s'applique qu'à une entrée XYZ : ignorée.
    let n_courbe_e = usize::from(u16_at(d, 48).ok_or_else(tronque)?);
    let n_courbe_s = usize::from(u16_at(d, 50).ok_or_else(tronque)?);
    if n_courbe_e < 2 || n_courbe_s < 2 {
        return Err(ErreurIcc::StructureNonPriseEnCharge {
            structure: format!("mft2, courbes de {n_courbe_e} et {n_courbe_s} points"),
        });
    }
    let mut lecteur = Mots { d, position: 52 };
    let entree = lecteur.courbes(n_courbe_e).ok_or_else(tronque)?;
    let noeuds = grille * grille * grille;
    let valeurs = lecteur.lire(noeuds * 3).ok_or_else(tronque)?;
    let table = valeurs
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    let sortie = lecteur.courbes(n_courbe_s).ok_or_else(tronque)?;
    Ok(Lut {
        grille,
        entree,
        table,
        sortie,
    })
}

/// Lecteur de mots de 16 bits, ramenés de 0 à 1.
struct Mots<'a> {
    d: &'a [u8],
    position: usize,
}

impl Mots<'_> {
    /// Trois courbes de `n` points, une par canal.
    fn courbes(&mut self, n: usize) -> Option<[Vec<f64>; 3]> {
        Some([self.lire(n)?, self.lire(n)?, self.lire(n)?])
    }

    fn lire(&mut self, n: usize) -> Option<Vec<f64>> {
        let fin = self.position.checked_add(n.checked_mul(2)?)?;
        let octets = self.d.get(self.position..fin)?;
        self.position = fin;
        Some(
            octets
                .as_chunks::<2>()
                .0
                .iter()
                .map(|m| f64::from(u16::from_be_bytes([m[0], m[1]])) / 65535.0)
                .collect(),
        )
    }
}
