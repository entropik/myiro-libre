//! Profil ICC v2 synthétique de sortie RVB, pour les tests de cette crate et
//! de l'application : aucun profil d'un autre logiciel n'entre dans le dépôt.

use crate::icc::{lab_vers_xyz, ILLUMINANT_D50};
use crate::table::{lab16_vers_lab, lab_vers_lab16, XYZ16};

/// Description d'un profil d'essai.
pub struct ProfilEssai {
    /// Points par arête de la table A2B1 (au moins 2).
    pub grille: u8,
    /// Blanc du support en L\*, a\*, b\* absolus (rapportés au D50 du PCS).
    pub blanc: [f64; 3],
    /// Valeurs de la table : RVB de 0 à 1 vers L\*, a\*, b\* relatifs.
    pub cible: fn([f64; 3]) -> [f64; 3],
    /// Table inverse B2A1, s'il y en a une : L\*, a\*, b\* relatifs vers RVB
    /// de 0 à 1 (borné), sur une grille de `GRILLE_INVERSE` points par arête.
    pub inverse: Option<fn([f64; 3]) -> [f64; 3]>,
    /// Texte de la balise `desc`, en ASCII (un autre caractère devient `?`).
    pub description: String,
    /// Texte de la balise `targ` (mesures embarquées), s'il y en a.
    pub mesures: Option<String>,
}

/// Points par arête de la table B2A1 d'un profil d'essai au PCS Lab.
const GRILLE_INVERSE: u8 = 33;

/// Points par arête de la table B2A1 d'un profil d'essai au PCS XYZ.
const GRILLE_INVERSE_XYZ: u8 = 45;

/// Diagonale de la matrice de B2A1 d'un profil d'essai au PCS XYZ :
/// valeurs exactes en s15Fixed16, différentes sur chaque canal.
const MATRICE_XYZ: [f64; 3] = [0.75, 0.875, 0.625];

/// Points des courbes d'un profil d'essai au PCS XYZ.
const POINTS_COURBE: usize = 4096;

/// f(0) de L\*a\*b\* : bas de l'entrée de B2A1 d'un profil d'essai XYZ.
const F_BAS: f64 = 4.0 / 29.0;

/// Haut de l'entrée de B2A1 d'un profil d'essai XYZ (un peu au-dessus du
/// blanc, f = 1).
const F_HAUT: f64 = 1.2;

/// Octets d'un profil ICC v2.4 de sortie RVB, PCS Lab : A2B1 `mft2` aux
/// courbes identité dont la grille vaut `cible`, A2B0 copie d'A2B1, `wtpt`
/// (XYZ de `blanc`), `desc`, `cprt`, B2A1 si `inverse` est donné, et `targ`
/// si `mesures` est donné.
pub fn profil_essai(p: &ProfilEssai) -> Vec<u8> {
    let cible = p.cible;
    let a2b1 = Table::nouvelle(p.grille, |rvb| lab_vers_lab16(cible(rvb)));
    let b2a1 = p
        .inverse
        .map(|inverse| Table::nouvelle(GRILLE_INVERSE, move |x| inverse(lab16_vers_lab(x))));
    assembler(p, b"Lab ", &a2b1, b2a1.as_ref())
}

/// Le même profil au PCS XYZ. A2B1 rend f(X/Xn), f(Y/Yn), f(Z/Zn) de
/// L\*a\*b\*, et ses courbes de sortie les passent en XYZ. B2A1 applique
/// d'abord la matrice diagonale `MATRICE_XYZ`, puis des courbes d'entrée
/// qui rendent ces mêmes f : `inverse` y est donc évalué sans détour.
pub fn profil_essai_xyz(p: &ProfilEssai) -> Vec<u8> {
    let cible = p.cible;
    let a2b1 = Table {
        sortie: (POINTS_COURBE, |k, v| {
            let f = F_BAS + v * (f_haut_xyz(k) - F_BAS);
            ILLUMINANT_D50[k] * f_inverse(f) / XYZ16
        }),
        ..Table::nouvelle(p.grille, |rvb| {
            let f = f_de_lab(cible(rvb));
            [0, 1, 2].map(|k| (f[k] - F_BAS) / (f_haut_xyz(k) - F_BAS))
        })
    };
    let b2a1 = p.inverse.map(|inverse| Table {
        diagonale: MATRICE_XYZ,
        // t = m × X / XYZ16 : X = t × XYZ16 / m.
        entree: (POINTS_COURBE, |k, t| {
            let x = t * XYZ16 / MATRICE_XYZ[k];
            (f_lab(x / ILLUMINANT_D50[k]) - F_BAS) / (F_HAUT - F_BAS)
        }),
        ..Table::nouvelle(GRILLE_INVERSE_XYZ, move |u: [f64; 3]| {
            let f = u.map(|v| F_BAS + v * (F_HAUT - F_BAS));
            inverse(lab_de_f(f))
        })
    });
    assembler(p, b"XYZ ", &a2b1, b2a1.as_ref())
}

/// Haut de la sortie d'A2B1 d'un profil d'essai XYZ : f du plus grand XYZ
/// qu'une lut16 écrit.
fn f_haut_xyz(k: usize) -> f64 {
    f_lab(XYZ16 / ILLUMINANT_D50[k])
}

/// f de L\*a\*b\* : racine cubique, droite près du noir.
fn f_lab(t: f64) -> f64 {
    if t > (6.0f64 / 29.0).powi(3) {
        t.cbrt()
    } else {
        t * 841.0 / 108.0 + F_BAS
    }
}

fn f_inverse(f: f64) -> f64 {
    if f > 6.0 / 29.0 {
        f * f * f
    } else {
        (f - F_BAS) * 108.0 / 841.0
    }
}

/// f(X/Xn), f(Y/Yn), f(Z/Zn) d'un L\*, a\*, b\*.
fn f_de_lab(lab: [f64; 3]) -> [f64; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    [fy + lab[1] / 500.0, fy, fy - lab[2] / 200.0]
}

fn lab_de_f(f: [f64; 3]) -> [f64; 3] {
    [
        116.0 * f[1] - 16.0,
        500.0 * (f[0] - f[1]),
        200.0 * (f[1] - f[2]),
    ]
}

/// Courbe d'une table d'essai : nombre de points, et valeur (de 0 à 1) de
/// chaque canal pour une entrée de 0 à 1.
type Courbe = (usize, fn(usize, f64) -> f64);

/// Une table `mft2` d'essai : `noeud` donne les trois valeurs (de 0 à 1)
/// d'un nœud dont l'entrée va de 0 à 1.
struct Table<N> {
    grille: u8,
    diagonale: [f64; 3],
    entree: Courbe,
    noeud: N,
    sortie: Courbe,
}

impl<N: Fn([f64; 3]) -> [f64; 3]> Table<N> {
    /// Table de ces nœuds, matrice et courbes identité.
    fn nouvelle(grille: u8, noeud: N) -> Self {
        Table {
            grille,
            diagonale: [1.0; 3],
            entree: (2, |_, x| x),
            noeud,
            sortie: (2, |_, x| x),
        }
    }
}

fn assembler<N, M>(
    p: &ProfilEssai,
    pcs: &[u8; 4],
    a2b1: &Table<N>,
    b2a1: Option<&Table<M>>,
) -> Vec<u8>
where
    N: Fn([f64; 3]) -> [f64; 3],
    M: Fn([f64; 3]) -> [f64; 3],
{
    let table = mft2(a2b1);
    let mut balises: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"desc", desc(&p.description)),
        (*b"cprt", texte(b"Profil d'essai, domaine public")),
        (*b"wtpt", wtpt(p.blanc)),
        (*b"A2B0", table.clone()),
        (*b"A2B1", table),
    ];
    if let Some(b2a1) = b2a1 {
        balises.push((*b"B2A1", mft2(b2a1)));
    }
    if let Some(mesures) = &p.mesures {
        balises.push((*b"targ", texte(mesures.as_bytes())));
    }

    let debut = 128 + 4 + 12 * balises.len();
    let mut corps = Vec::new();
    let mut table_balises = Vec::new();
    table_balises.extend_from_slice(&u32_be(balises.len()));
    for (signature, donnees) in &balises {
        while !(debut + corps.len()).is_multiple_of(4) {
            corps.push(0);
        }
        table_balises.extend_from_slice(signature);
        table_balises.extend_from_slice(&u32_be(debut + corps.len()));
        table_balises.extend_from_slice(&u32_be(donnees.len()));
        corps.extend_from_slice(donnees);
    }

    let taille = debut + corps.len();
    let mut octets = en_tete(taille, pcs);
    octets.extend_from_slice(&table_balises);
    octets.extend_from_slice(&corps);
    octets
}

fn en_tete(taille: usize, pcs: &[u8; 4]) -> Vec<u8> {
    let mut h = vec![0u8; 128];
    h[0..4].copy_from_slice(&u32_be(taille));
    h[8..12].copy_from_slice(&[0x02, 0x40, 0, 0]);
    h[12..16].copy_from_slice(b"prtr");
    h[16..20].copy_from_slice(b"RGB ");
    h[20..24].copy_from_slice(pcs);
    // Date : 26 octobre 2026, midi.
    for (i, v) in [2026u16, 10, 26, 12, 0, 0].iter().enumerate() {
        h[24 + 2 * i..26 + 2 * i].copy_from_slice(&v.to_be_bytes());
    }
    h[36..40].copy_from_slice(b"acsp");
    for (k, v) in ILLUMINANT_D50.iter().enumerate() {
        h[68 + 4 * k..72 + 4 * k].copy_from_slice(&s15(*v));
    }
    h
}

fn mft2<N: Fn([f64; 3]) -> [f64; 3]>(table: &Table<N>) -> Vec<u8> {
    let g = table.grille.max(2);
    let mut t = Vec::new();
    t.extend_from_slice(b"mft2");
    t.extend_from_slice(&[0; 4]);
    t.extend_from_slice(&[3, 3, g, 0]);
    for i in 0..9 {
        t.extend_from_slice(&s15(if i % 4 == 0 {
            table.diagonale[i / 4]
        } else {
            0.0
        }));
    }
    t.extend_from_slice(&u16_be(table.entree.0));
    t.extend_from_slice(&u16_be(table.sortie.0));
    courbes(&mut t, table.entree);
    let pas = f64::from(g - 1);
    for r in 0..g {
        for v in 0..g {
            for b in 0..g {
                let valeurs =
                    (table.noeud)([f64::from(r) / pas, f64::from(v) / pas, f64::from(b) / pas]);
                for w in valeurs {
                    t.extend_from_slice(&mot16(w * 65535.0).to_be_bytes());
                }
            }
        }
    }
    courbes(&mut t, table.sortie);
    t
}

/// Les trois courbes d'un canal après l'autre.
fn courbes(t: &mut Vec<u8>, (points, courbe): Courbe) {
    let pas = (points.max(2) - 1) as f64;
    for k in 0..3 {
        for i in 0..points.max(2) {
            let v = courbe(k, i as f64 / pas);
            t.extend_from_slice(&mot16(v * 65535.0).to_be_bytes());
        }
    }
}

fn wtpt(blanc: [f64; 3]) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(b"XYZ ");
    t.extend_from_slice(&[0; 4]);
    for v in lab_vers_xyz(blanc, ILLUMINANT_D50) {
        t.extend_from_slice(&s15(v));
    }
    t
}

/// Type `desc` de l'ICC v2 : ASCII, puis Unicode et ScriptCode vides.
fn desc(texte: &str) -> Vec<u8> {
    let ascii: Vec<u8> = texte
        .chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .collect();
    let mut t = Vec::new();
    t.extend_from_slice(b"desc");
    t.extend_from_slice(&[0; 4]);
    t.extend_from_slice(&u32_be(ascii.len() + 1));
    t.extend_from_slice(&ascii);
    t.push(0);
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&[0; 3]);
    t.extend_from_slice(&[0; 67]);
    t
}

fn texte(octets: &[u8]) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(b"text");
    t.extend_from_slice(&[0; 4]);
    t.extend_from_slice(octets);
    t.push(0);
    t
}

fn mot16(w: f64) -> u16 {
    // Borné dans l'échelle du PCS avant conversion : la conversion est exacte.
    w.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

fn s15(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

fn u16_be(n: usize) -> [u8; 2] {
    u16::try_from(n).unwrap_or(u16::MAX).to_be_bytes()
}

fn u32_be(n: usize) -> [u8; 4] {
    u32::try_from(n).unwrap_or(u32::MAX).to_be_bytes()
}
