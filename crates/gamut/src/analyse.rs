//! Analyse d'un profil : blanc et noir, densité du noir, volume, couleurs de
//! base, table de chroma par clarté et teinte, chemins du cube RVB.

use colorimetrie::lab_vers_lch;
use serde::Serialize;

use crate::icc::{lab_vers_xyz, vers_lab, ProfilIcc};
use crate::surface::{couleur_ecran, maillage_affiche, maillage_profil, volume, MaillageAffiche};

/// Points par arête de la surface qui sert au volume.
const ARETE_VOLUME: usize = 33;
/// Points par arête de la surface affichée (1 734 sommets, 3 072 triangles).
const ARETE_AFFICHAGE: usize = 17;
/// Points par arête de la grille RVB de la table de chroma.
const GRILLE_CHROMA: usize = 33;
/// Lignes de clarté (L\* de 0 à 99) et colonnes de teinte (2° chacune).
const CLARTES: usize = 100;
const TEINTES: usize = 180;
/// Sous cette chroma, la teinte n'est que du bruit d'arrondi : elle est
/// inconnue, comme celle d'un gris.
const CHROMA_GRIS: f64 = 0.01;

/// Couleurs de base, dans l'ordre de la page.
pub(crate) const PRIMAIRES: [(&str, [u8; 3]); 8] = [
    ("blanc", [255, 255, 255]),
    ("noir", [0, 0, 0]),
    ("rouge", [255, 0, 0]),
    ("vert", [0, 255, 0]),
    ("bleu", [0, 0, 255]),
    ("cyan", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("jaune", [255, 255, 0]),
];

/// Une couleur de base du profil, en colorimétrie absolue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Primaire {
    pub nom: &'static str,
    pub rvb: [u8; 3],
    pub lab: [f64; 3],
    pub c: f64,
    /// Teinte en degrés ; inconnue pour un gris.
    pub h: Option<f64>,
    /// Couleur écran `#rrggbb`.
    pub ecran: String,
    /// La couleur sort de l'écran : `ecran` n'en est qu'une approximation.
    pub ramenee: bool,
}

/// Chiffres et formes d'un profil, pour la page « Comparer ».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Analyse {
    pub version: String,
    pub description: Option<String>,
    pub grille: u8,
    /// L\*, a\*, b\* absolus du blanc (RVB 1, 1, 1) et du noir (0, 0, 0).
    pub blanc: [f64; 3],
    pub noir: [f64; 3],
    /// Densité **approchée** du noir, −log₁₀ Y (blanc Y = 1) : ce n'est pas
    /// une densité ISO status T.
    pub dmax_approche: f64,
    /// Volume de la surface brute du cube RVB, en unités Lab au cube.
    pub volume_surface: f64,
    pub primaires: Vec<Primaire>,
    /// C\* maximal par colonne de teinte de 2° (**approché**).
    pub chroma_par_teinte: Vec<f64>,
    /// `coupes[L]` : C\* maximal par teinte à la clarté L (**approché**).
    pub coupes: Vec<Vec<f64>>,
    pub affichage: MaillageAffiche,
}

/// Analyse complète d'un profil.
pub fn analyser(p: &ProfilIcc) -> Analyse {
    let blanc = p.lab_absolu([1.0, 1.0, 1.0]);
    let noir = p.lab_absolu([0.0, 0.0, 0.0]);
    let y_noir = lab_vers_xyz(noir, [1.0, 1.0, 1.0])[1];
    let coupes = coupes(p);
    let chroma_par_teinte = (0..TEINTES)
        .map(|h| coupes.iter().map(|ligne| ligne[h]).fold(0.0, f64::max))
        .collect();
    Analyse {
        version: p.version(),
        description: p.description(),
        grille: p.grille(),
        blanc,
        noir,
        dmax_approche: -y_noir.log10(),
        volume_surface: volume(&maillage_profil(p, ARETE_VOLUME)),
        primaires: PRIMAIRES
            .iter()
            .map(|&(nom, rvb)| primaire(p, nom, rvb))
            .collect(),
        chroma_par_teinte,
        coupes,
        affichage: maillage_affiche(&maillage_profil(p, ARETE_AFFICHAGE)),
    }
}

/// L\*, C\* le long du segment du cube RVB de `depart` à `arrivee` :
/// 256 points, t = i / 255.
pub fn chemin(p: &ProfilIcc, depart: [u8; 3], arrivee: [u8; 3]) -> Vec<[f64; 2]> {
    (0..256)
        .map(|i| {
            let t = f64::from(i) / 255.0;
            let rvb = [0, 1, 2].map(|k| {
                let (d, a) = (f64::from(depart[k]), f64::from(arrivee[k]));
                (d + t * (a - d)) / 255.0
            });
            let [l, a, b] = p.lab_absolu(rvb);
            [l, a.hypot(b)]
        })
        .collect()
}

/// Chroma et teinte d'un L\*, a\*, b\* ; teinte inconnue pour un gris.
fn chroma_teinte(lab: [f64; 3]) -> (f64, Option<f64>) {
    match lab_vers_lch(vers_lab(lab)) {
        Ok(lch) if lch.c >= CHROMA_GRIS => (lch.c, lch.h.ok()),
        Ok(lch) => (lch.c, None),
        Err(_) => (f64::NAN, None),
    }
}

fn primaire(p: &ProfilIcc, nom: &'static str, rvb: [u8; 3]) -> Primaire {
    let lab = p.lab_absolu(rvb.map(|v| f64::from(v) / 255.0));
    let (c, h) = chroma_teinte(lab);
    let (ecran, ramenee) = couleur_ecran(lab);
    Primaire {
        nom,
        rvb,
        lab,
        c,
        h,
        ecran,
        ramenee,
    }
}

/// Table de chroma : grille RVB 33³, case (⌊L\*⌋, ⌊teinte / 2°⌋), C\* maximal
/// par case, cases vides comblées, valeurs arrondies à 0,1. Une case restée
/// vide vaut 0 : aucune couleur du profil n'y tombe.
fn coupes(p: &ProfilIcc) -> Vec<Vec<f64>> {
    let mut table = vec![vec![None::<f64>; TEINTES]; CLARTES];
    let pas = (GRILLE_CHROMA - 1) as f64;
    for r in 0..GRILLE_CHROMA {
        for g in 0..GRILLE_CHROMA {
            for b in 0..GRILLE_CHROMA {
                let lab = p.lab_absolu([r as f64 / pas, g as f64 / pas, b as f64 / pas]);
                let (c, Some(h)) = chroma_teinte(lab) else {
                    continue;
                };
                if !lab[0].is_finite() {
                    continue;
                }
                let l = (lab[0].floor().max(0.0) as usize).min(CLARTES - 1);
                let colonne = ((h / 2.0).floor() as usize).min(TEINTES - 1);
                let case = &mut table[l][colonne];
                *case = Some(case.map_or(c, |m: f64| m.max(c)));
            }
        }
    }
    table
        .into_iter()
        .map(|mut ligne| {
            combler(&mut ligne);
            ligne
                .into_iter()
                .map(|c| c.map_or(0.0, |c| (c * 10.0).round() / 10.0))
                .collect()
        })
        .collect()
}

/// Comble les cases vides d'une ligne de teintes : une case vide entre deux
/// cases pleines prend la plus petite des deux, la teinte bouclant de 358° à
/// 0°. Une ligne qui n'a pas deux cases pleines reste telle quelle.
fn combler(ligne: &mut [Option<f64>]) {
    let pleines: Vec<usize> = (0..ligne.len()).filter(|&i| ligne[i].is_some()).collect();
    if pleines.len() < 2 {
        return;
    }
    let n = ligne.len();
    for (k, &depart) in pleines.iter().enumerate() {
        let arrivee = pleines[(k + 1) % pleines.len()];
        let (Some(a), Some(b)) = (ligne[depart], ligne[arrivee]) else {
            continue;
        };
        let valeur = a.min(b);
        let mut i = (depart + 1) % n;
        while i != arrivee {
            ligne[i] = Some(valeur);
            i = (i + 1) % n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::combler;

    #[test]
    fn une_case_vide_entre_deux_pleines_est_comblee() {
        let mut ligne = vec![None; 8];
        ligne[1] = Some(10.0);
        ligne[4] = Some(30.0);
        ligne[6] = Some(20.0);
        combler(&mut ligne);
        // 2 et 3 entre 10 et 30 ; 5 entre 30 et 20 ; 7 et 0 entre 20 et 10
        // en bouclant.
        let attendu = [10.0, 10.0, 10.0, 10.0, 30.0, 20.0, 20.0, 10.0].map(Some);
        assert_eq!(ligne, attendu);
    }

    #[test]
    fn une_ligne_d_une_seule_case_pleine_reste_telle_quelle() {
        let mut ligne = vec![None; 4];
        ligne[2] = Some(5.0);
        combler(&mut ligne);
        assert_eq!(ligne, [None, None, Some(5.0), None]);
        let mut vide = vec![None::<f64>; 4];
        combler(&mut vide);
        assert_eq!(vide, [None; 4]);
    }
}
