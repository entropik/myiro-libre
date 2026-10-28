//! Profil ICC v2 synthétique de sortie RVB, pour les tests de cette crate et
//! de l'application : aucun profil d'un autre logiciel n'entre dans le dépôt.

use crate::icc::{lab_vers_xyz, ILLUMINANT_D50};

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

/// Points par arête de la table B2A1 d'un profil d'essai.
const GRILLE_INVERSE: u8 = 33;

/// Octets d'un profil ICC v2.4 de sortie RVB, PCS Lab : A2B1 `mft2` aux
/// courbes identité dont la grille vaut `cible`, A2B0 copie d'A2B1, `wtpt`
/// (XYZ de `blanc`), `desc`, `cprt`, B2A1 si `inverse` est donné, et `targ`
/// si `mesures` est donné.
pub fn profil_essai(p: &ProfilEssai) -> Vec<u8> {
    let cible = p.cible;
    let table = mft2(p.grille, |rvb| {
        let lab = cible(rvb);
        [
            lab[0] * 652.80,
            (lab[1] + 128.0) * 256.0,
            (lab[2] + 128.0) * 256.0,
        ]
    });
    let mut balises: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"desc", desc(&p.description)),
        (*b"cprt", texte(b"Profil d'essai, domaine public")),
        (*b"wtpt", wtpt(p.blanc)),
        (*b"A2B0", table.clone()),
        (*b"A2B1", table),
    ];
    if let Some(inverse) = p.inverse {
        let table = mft2(GRILLE_INVERSE, |x| {
            // Nœud en Lab « hérité » 16 bits, l'encodage d'entrée de B2A1.
            let lab = [
                x[0] * 65535.0 / 652.80,
                x[1] * 65535.0 / 256.0 - 128.0,
                x[2] * 65535.0 / 256.0 - 128.0,
            ];
            inverse(lab).map(|v| v * 65535.0)
        });
        balises.push((*b"B2A1", table));
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
    let mut octets = en_tete(taille);
    octets.extend_from_slice(&table_balises);
    octets.extend_from_slice(&corps);
    octets
}

fn en_tete(taille: usize) -> Vec<u8> {
    let mut h = vec![0u8; 128];
    h[0..4].copy_from_slice(&u32_be(taille));
    h[8..12].copy_from_slice(&[0x02, 0x40, 0, 0]);
    h[12..16].copy_from_slice(b"prtr");
    h[16..20].copy_from_slice(b"RGB ");
    h[20..24].copy_from_slice(b"Lab ");
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

/// Table `mft2` aux courbes identité : `noeud` donne les trois mots (de 0 à
/// 65 535) d'un nœud dont l'entrée va de 0 à 1.
fn mft2(grille: u8, noeud: impl Fn([f64; 3]) -> [f64; 3]) -> Vec<u8> {
    let g = grille.max(2);
    let mut t = Vec::new();
    t.extend_from_slice(b"mft2");
    t.extend_from_slice(&[0; 4]);
    t.extend_from_slice(&[3, 3, g, 0]);
    for i in 0..9 {
        t.extend_from_slice(&s15(if i % 4 == 0 { 1.0 } else { 0.0 }));
    }
    t.extend_from_slice(&2u16.to_be_bytes());
    t.extend_from_slice(&2u16.to_be_bytes());
    let identite = |t: &mut Vec<u8>| {
        for _ in 0..3 {
            t.extend_from_slice(&0u16.to_be_bytes());
            t.extend_from_slice(&u16::MAX.to_be_bytes());
        }
    };
    identite(&mut t);
    let pas = f64::from(g - 1);
    for r in 0..g {
        for v in 0..g {
            for b in 0..g {
                let mots = noeud([f64::from(r) / pas, f64::from(v) / pas, f64::from(b) / pas]);
                for w in mots {
                    t.extend_from_slice(&mot16(w).to_be_bytes());
                }
            }
        }
    }
    identite(&mut t);
    t
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

fn u32_be(n: usize) -> [u8; 4] {
    u32::try_from(n).unwrap_or(u32::MAX).to_be_bytes()
}
