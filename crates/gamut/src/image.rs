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

/// Code d'écart d'un pixel : dixièmes de ΔE00 arrondis vers le bas
/// (`⌊10 × écart⌋`), bornés à 254 ; 255 = transparent ou inconnu.
///
/// Règle de lecture du seuil, pour les chiffres comme pour la surimpression :
/// un pixel est **au-delà du seuil** si `code ≥ round(10 × seuil)` et que
/// son code n'est pas `ECART_INCONNU`.
pub const ECART_INCONNU: u8 = 255;
/// Largeur d'une case de l'histogramme : 255 cases de 0,1 ΔE00, la case k
/// reçoit les écarts de k × 0,1 (inclus) à (k + 1) × 0,1 ; la dernière
/// reçoit tout écart d'au moins 25,4. La case d'un pixel est son code
/// d'écart (hors `ECART_INCONNU`) : au-delà du seuil, ce sont les cases à partir
/// de `round(10 × seuil)`.
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
    /// Pixels comptés dont l'écart n'a pas pu être calculé (valeur non
    /// finie). La part hors gamut se calcule sur `pixels_comptes −
    /// pixels_inconnus` ; les inconnus s'affichent à part, jamais comme bons.
    pub pixels_inconnus: u64,
    /// Couleurs RVB distinctes parmi les pixels comptés.
    pub couleurs_distinctes: u32,
    /// Pixels par case de 0,1 ΔE00 (`PAS_HISTOGRAMME`), 255 cases. Un pixel
    /// dont l'écart est inconnu n'y figure pas : la somme vaut
    /// `pixels_comptes − pixels_inconnus`.
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
    analyser(pixels, &|rvb| source.demande(rvb), sortie, annuler)
}

/// Analyse, la couleur demandée donnée par `demande` (les tests y glissent
/// une valeur non finie).
fn analyser(
    pixels: &[[u8; 4]],
    demande: &dyn Fn([f64; 3]) -> [f64; 3],
    sortie: &ProfilIcc,
    annuler: &dyn Fn() -> bool,
) -> Result<Result<AnalyseImage, ErreurIcc>, Annulee> {
    if let Err(raison) = sortie.inverse() {
        return Ok(Err(raison));
    }

    // 1. Pixels opaques regroupés par valeur RVB.
    let mut comptes: Carte<u32> = Carte::default();
    let mut pixels_comptes = 0u64;
    for p in pixels.iter().filter(|p| p[3] != 0) {
        *comptes.entry(cle(p)).or_insert(0) += 1;
        pixels_comptes += 1;
    }
    let mut couleurs: Vec<(u32, u32)> = comptes.into_iter().collect();
    // Ordre fixe : le nuage (sommes en virgule flottante) ne dépend pas du
    // hasard de la table.
    couleurs.sort_unstable_by_key(|&(c, _)| c);

    // 2. Un aller-retour par couleur distincte, reporté sur ses pixels.
    let mut histogramme = vec![0u64; CASES];
    let mut par_teinte = vec![vec![0u64; CASES]; SECTEURS_TEINTE];
    let mut cellules: HashMap<[i32; 3], Cellule> = HashMap::new();
    let mut codes: Carte<u8> = Carte::default();
    codes.reserve(couleurs.len());
    let mut pixels_inconnus = 0u64;
    for (i, &(c, n)) in couleurs.iter().enumerate() {
        if i % PAS_ANNULATION == 0 && annuler() {
            return Err(Annulee);
        }
        let rvb = [c >> 16, c >> 8, c].map(|v| f64::from(v & 0xff) / 255.0);
        let demandee = demande(rvb);
        let ecart = sortie
            .reproduire(demandee)
            .ok()
            .and_then(|reproduite| delta_e00(vers_lab(demandee), vers_lab(reproduite)).ok());
        let code = ecart.map_or(ECART_INCONNU, code_ecart);
        codes.insert(c, code);
        let Some(ecart) = ecart else {
            pixels_inconnus += u64::from(n);
            continue;
        };
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
            _ => codes.get(&cle(p)).copied().unwrap_or(ECART_INCONNU),
        })
        .collect();

    Ok(Ok(AnalyseImage {
        pixels_comptes,
        pixels_inconnus,
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

/// Écart en dixièmes de ΔE00, arrondi vers le bas, borné à 254. Produit
/// par 10, et non quotient par 0,1 : 0,3 / 0,1 vaut 2,999… en virgule
/// flottante.
fn code_ecart(ecart: f64) -> u8 {
    (ecart * 10.0).floor().clamp(0.0, (CASES - 1) as f64) as u8
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essai::{profil_essai, ProfilEssai};

    #[test]
    fn un_ecart_tombe_dans_sa_case_au_dixieme_pres() {
        // 0,3 / 0,1 vaut 2,999… en virgule flottante : la case serait 2.
        assert_eq!(code_ecart(0.3), 3);
        assert_eq!(code_ecart(0.6), 6);
        assert_eq!(code_ecart(1.99), 19);
        assert_eq!(code_ecart(2.0), 20);
        assert_eq!(code_ecart(30.0), 254);
    }

    fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
        let [r, g, b] = rvb;
        [
            25.0 + 75.0 * (r + g + b) / 3.0,
            60.0 * (r - g),
            60.0 * (g - b),
        ]
    }

    fn lineaire_inverse(lab: [f64; 3]) -> [f64; 3] {
        let [l, a, b] = lab;
        let base = (l - 25.0) / 75.0;
        [
            base + a / 90.0 + b / 180.0,
            base - a / 180.0 + b / 180.0,
            base - a / 180.0 - b / 90.0,
        ]
        .map(|v| v.clamp(0.0, 1.0))
    }

    #[test]
    fn un_ecart_non_calculable_est_compte_a_part() {
        let p = ProfilEssai {
            grille: 9,
            blanc: [95.0, -0.5, 2.5],
            cible: lineaire,
            inverse: Some(lineaire_inverse),
            description: "Profil d'essai".into(),
            mesures: None,
        };
        let sortie = ProfilIcc::lire(&profil_essai(&p)).expect("profil d'essai lisible");
        let source = ProfilSource::srvb();
        // Le noir pur donne une couleur demandée non finie.
        let demande = |rvb: [f64; 3]| {
            if rvb == [0.0; 3] {
                [f64::NAN; 3]
            } else {
                source.demande(rvb)
            }
        };
        let pixels = [[0, 0, 0, 255], [128, 128, 128, 255], [0, 0, 0, 255]];
        let analyse = analyser(&pixels, &demande, &sortie, &|| false)
            .expect("non annulée")
            .expect("table inverse lue");
        assert_eq!(analyse.pixels_comptes, 3);
        assert_eq!(analyse.pixels_inconnus, 2);
        assert_eq!(analyse.histogramme.iter().sum::<u64>(), 1);
        assert_eq!(analyse.ecarts, vec![ECART_INCONNU, 0, ECART_INCONNU]);
        assert_eq!(analyse.nuage.iter().map(|p| p.pixels).sum::<u32>(), 1);
    }
}
