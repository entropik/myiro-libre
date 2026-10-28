//! Écarts des pixels d'une image à un profil de sortie (ticket #80) : pour
//! chaque couleur distincte, couleur demandée (profil de l'image), couleur
//! reproduite (aller-retour B2A1 puis A2B1 du profil de sortie, en
//! colorimétrique relatif) et ΔE00 entre les deux.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use colorimetrie::{delta_e00, lab_vers_lch};
use serde::Serialize;

use crate::icc::{vers_lab, ErreurIcc, ProfilIcc};
use crate::source::ProfilSource;
use crate::surface::couleur_ecran;

/// Écart d'un pixel, en dixièmes de ΔE00 (arrondi vers le bas), borné à
/// 254 ; 255 = transparent ou inconnu.
pub const ECART_INCONNU: u8 = 255;
/// Largeur d'une case de l'histogramme : 255 cases de 0,1 ΔE00, la case k
/// reçoit les écarts de k × 0,1 (inclus) à (k + 1) × 0,1 ; la dernière
/// reçoit tout écart d'au moins 25,4.
pub const PAS_HISTOGRAMME: f64 = 0.1;
/// Douze secteurs de teinte de 30° (le premier part de 0°), puis « gris »
/// (C\* < 5).
pub const SECTEURS_TEINTE: usize = 13;
/// Points du nuage de couleurs, au plus.
pub const POINTS_NUAGE: usize = 4_000;

/// Cases de l'histogramme (codes d'écart 0 à 254).
const CASES: usize = 255;
/// Sous cette chroma, la couleur demandée est un gris.
const CHROMA_GRIS: f64 = 5.0;
/// Côté d'une cellule du nuage, en unités Lab.
const CELLULE: f64 = 4.0;
/// L'annulation est regardée toutes les `PAS_ANNULATION` couleurs distinctes.
const PAS_ANNULATION: usize = 65_536;

/// Un point du nuage : une cellule de 4 unités Lab de la couleur demandée.
#[derive(Debug, Clone, Serialize)]
pub struct PointNuage {
    /// Moyenne de la cellule, en L\*a\*b\* absolus sur le blanc du papier du
    /// profil de sortie.
    pub lab: [f64; 3],
    /// Pixels de l'image dans la cellule.
    pub pixels: u32,
    /// Plus grand écart de la cellule, en ΔE00.
    pub ecart_max: f64,
    /// Couleur écran `#rrggbb` de `lab`.
    pub couleur: String,
}

/// Écarts d'une image d'analyse à un profil de sortie.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyseImage {
    /// Pixels comptés : tous sauf les transparents (alpha = 0).
    pub pixels_comptes: u64,
    /// Couleurs RVB distinctes parmi les pixels comptés.
    pub couleurs_distinctes: u32,
    /// Pixels par case de 0,1 ΔE00 (`PAS_HISTOGRAMME`), 255 cases. Un pixel
    /// compté dont l'écart est inconnu n'y figure pas : la somme peut être
    /// inférieure à `pixels_comptes`.
    pub histogramme: Vec<u64>,
    /// Le même histogramme par secteur de teinte de la couleur demandée
    /// (`SECTEURS_TEINTE` lignes, la dernière pour les gris).
    pub par_teinte: Vec<Vec<u64>>,
    /// Au plus `POINTS_NUAGE` cellules, les plus peuplées d'abord.
    pub nuage: Vec<PointNuage>,
    /// Code d'écart de chaque pixel, dans l'ordre des lignes : dixièmes de
    /// ΔE00 (0 à 254) ou `ECART_INCONNU`.
    #[serde(skip)]
    pub ecarts: Vec<u8>,
}

/// L'analyse a été arrêtée à la demande de l'opérateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Annulee;

/// Écarts des pixels RVBA d'une image d'analyse au profil `sortie`. Les
/// couleurs demandées viennent de `source`. `annuler` est regardé toutes
/// les 65 536 couleurs distinctes, dès la première. Un profil sans table
/// inverse lisible rend `Ok(Err(..))` avec sa raison.
pub fn analyser_pixels(
    pixels: &[[u8; 4]],
    source: &ProfilSource,
    sortie: &ProfilIcc,
    annuler: &dyn Fn() -> bool,
) -> Result<Result<AnalyseImage, ErreurIcc>, Annulee> {
    if let Err(raison) = sortie.inverse() {
        return Ok(Err(raison));
    }

    // 1. Pixels opaques regroupés par valeur RVB.
    let mut codes: Carte<u32> = Carte::default();
    let mut pixels_comptes = 0u64;
    for p in pixels.iter().filter(|p| p[3] != 0) {
        *codes.entry(cle(p)).or_insert(0) += 1;
        pixels_comptes += 1;
    }
    let mut couleurs: Vec<(u32, u32)> = codes.iter().map(|(&c, &n)| (c, n)).collect();
    // Ordre fixe : le nuage (sommes en virgule flottante) ne dépend pas du
    // hasard de la table.
    couleurs.sort_unstable_by_key(|&(c, _)| c);

    // 2. Un aller-retour par couleur distincte, reporté sur ses pixels.
    let mut histogramme = vec![0u64; CASES];
    let mut par_teinte = vec![vec![0u64; CASES]; SECTEURS_TEINTE];
    let mut cellules: HashMap<[i32; 3], Cellule> = HashMap::new();
    for (i, &(c, n)) in couleurs.iter().enumerate() {
        if i % PAS_ANNULATION == 0 && annuler() {
            return Err(Annulee);
        }
        let rvb = [c >> 16, c >> 8, c].map(|v| f64::from(v & 0xff) / 255.0);
        let demandee = source.demande(rvb);
        let ecart = sortie
            .reproduire(demandee)
            .ok()
            .and_then(|reproduite| delta_e00(vers_lab(demandee), vers_lab(reproduite)).ok());
        let code = ecart.map_or(ECART_INCONNU, code_ecart);
        codes.insert(c, u32::from(code));
        let Some(ecart) = ecart else { continue };
        let case = usize::from(code);
        histogramme[case] += u64::from(n);
        par_teinte[secteur(demandee)][case] += u64::from(n);
        cellules
            .entry(demandee.map(|v| (v / CELLULE).floor() as i32))
            .or_default()
            .ajouter(demandee, n, ecart);
    }

    // 3. Code d'écart de chaque pixel.
    let ecarts = pixels
        .iter()
        .map(|p| match p[3] {
            0 => ECART_INCONNU,
            _ => codes
                .get(&cle(p))
                .and_then(|&code| u8::try_from(code).ok())
                .unwrap_or(ECART_INCONNU),
        })
        .collect();

    Ok(Ok(AnalyseImage {
        pixels_comptes,
        couleurs_distinctes: u32::try_from(couleurs.len()).unwrap_or(u32::MAX),
        histogramme,
        par_teinte,
        nuage: nuage(cellules, sortie),
        ecarts,
    }))
}

/// Valeur RVB d'un pixel en un entier 0xRRVVBB.
fn cle(p: &[u8; 4]) -> u32 {
    u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
}

/// Écart en dixièmes de ΔE00, arrondi vers le bas, borné à 254.
fn code_ecart(ecart: f64) -> u8 {
    (ecart / PAS_HISTOGRAMME)
        .floor()
        .clamp(0.0, (CASES - 1) as f64) as u8
}

/// Secteur de teinte d'une couleur demandée : 0 à 11 par 30°, 12 pour un gris.
fn secteur(lab: [f64; 3]) -> usize {
    match lab_vers_lch(vers_lab(lab)) {
        Ok(lch) if lch.c >= CHROMA_GRIS => match lch.h {
            Ok(h) => ((h / 30.0).floor() as usize).min(SECTEURS_TEINTE - 2),
            Err(_) => SECTEURS_TEINTE - 1,
        },
        _ => SECTEURS_TEINTE - 1,
    }
}

/// Pixels d'une cellule du nuage.
#[derive(Default)]
struct Cellule {
    somme: [f64; 3],
    pixels: u64,
    ecart_max: f64,
}

impl Cellule {
    fn ajouter(&mut self, lab: [f64; 3], pixels: u32, ecart: f64) {
        let n = f64::from(pixels);
        for (somme, v) in self.somme.iter_mut().zip(lab) {
            *somme += v * n;
        }
        self.pixels += u64::from(pixels);
        self.ecart_max = self.ecart_max.max(ecart);
    }
}

/// Les `POINTS_NUAGE` cellules les plus peuplées, placées en colorimétrie
/// absolue sur le blanc du papier de `sortie`.
fn nuage(cellules: HashMap<[i32; 3], Cellule>, sortie: &ProfilIcc) -> Vec<PointNuage> {
    let mut cellules: Vec<([i32; 3], Cellule)> = cellules.into_iter().collect();
    cellules.sort_unstable_by(|a, b| b.1.pixels.cmp(&a.1.pixels).then(a.0.cmp(&b.0)));
    cellules.truncate(POINTS_NUAGE);
    cellules
        .into_iter()
        .map(|(_, c)| {
            let n = c.pixels as f64;
            let lab = sortie.relatif_vers_absolu(c.somme.map(|s| s / n));
            PointNuage {
                lab,
                pixels: u32::try_from(c.pixels).unwrap_or(u32::MAX),
                ecart_max: c.ecart_max,
                couleur: couleur_ecran(lab).0,
            }
        })
        .collect()
}

/// Table de hachage des valeurs RVB : la clé est déjà un entier de 24 bits,
/// un mélange multiplicatif suffit (le hachage par défaut, prévu contre les
/// clés hostiles, coûte ici plus que l'aller-retour).
type Carte<V> = HashMap<u32, V, BuildHasherDefault<Melange>>;

#[derive(Default)]
struct Melange(u64);

impl Hasher for Melange {
    /// Les bits hauts du produit, qui dépendent de toute la clé, replient
    /// sur les bits bas qui choisissent la case.
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 32)
    }

    fn write(&mut self, octets: &[u8]) {
        for &o in octets {
            self.0 = (self.0 << 8 | u64::from(o)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }

    fn write_u32(&mut self, v: u32) {
        self.0 = u64::from(v).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}
