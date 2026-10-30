//! Familles de teinte des couleurs d'une photo (ticket #80) : six familles
//! ancrées sur les primaires et secondaires sRVB, plus les gris. Un imprimeur
//! lit « bleus » pour le bleu d'une photo, ce que des secteurs de 30° ne
//! disent pas (le bleu sRVB tombe vers 306°, entre deux secteurs).

use colorimetrie::lab_vers_lch;
use serde::Serialize;

use crate::icc::vers_lab;
use crate::source::ProfilSource;

/// Lignes de `AnalyseImage::par_teinte` : six familles, puis les gris.
pub const SECTEURS_TEINTE: usize = 7;
/// Noms des lignes de `AnalyseImage::par_teinte`, dans leur ordre.
pub const FAMILLES_TEINTE: [&str; SECTEURS_TEINTE] =
    ["rouge", "jaune", "vert", "cyan", "bleu", "magenta", "gris"];

/// Couleurs sRVB pures qui ancrent les six familles, dans l'ordre des
/// teintes croissantes.
const ANCRES: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];
/// Sous cette chroma, la couleur demandée est un gris.
const CHROMA_GRIS: f64 = 5.0;

/// Une famille de teinte : de `debut` (inclus) à `fin` (exclu), en degrés
/// de h CIELab relatif D50, en tournant dans le sens des teintes
/// croissantes. Le magenta passe par 0° : son `debut` dépasse sa `fin`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FamilleTeinte {
    pub nom: &'static str,
    /// Teinte de la couleur sRVB pure de la famille.
    pub ancrage: f64,
    /// Mi-chemin avec la famille précédente.
    pub debut: f64,
    /// Mi-chemin avec la famille suivante.
    pub fin: f64,
}

/// Les six familles, dans l'ordre de `FAMILLES_TEINTE` : ancrage calculé par
/// `ProfilSource::srvb`, bornes à mi-chemin des ancrages voisins.
pub fn familles_teinte() -> [FamilleTeinte; 6] {
    let srvb = ProfilSource::srvb();
    let ancrages = ANCRES.map(|rvb| {
        let [_, a, b] = srvb.demande(rvb);
        b.atan2(a).to_degrees().rem_euclid(360.0)
    });
    let milieu = |k: usize| {
        let suivant = ancrages[(k + 1) % 6];
        (ancrages[k] + (suivant - ancrages[k]).rem_euclid(360.0) / 2.0).rem_euclid(360.0)
    };
    std::array::from_fn(|k| FamilleTeinte {
        nom: FAMILLES_TEINTE[k],
        ancrage: ancrages[k],
        debut: milieu((k + 5) % 6),
        fin: milieu(k),
    })
}

/// Ligne de `par_teinte` d'une couleur demandée : sa famille, ou les gris
/// (C\* < 5, ou teinte inconnue).
pub(crate) fn famille(familles: &[FamilleTeinte; 6], lab: [f64; 3]) -> usize {
    let gris = SECTEURS_TEINTE - 1;
    let h = match lab_vers_lch(vers_lab(lab)) {
        Ok(lch) if lch.c >= CHROMA_GRIS => match lch.h {
            Ok(h) => h,
            Err(_) => return gris,
        },
        _ => return gris,
    };
    familles
        .iter()
        .position(|f| (h - f.debut).rem_euclid(360.0) < (f.fin - f.debut).rem_euclid(360.0))
        .unwrap_or(gris)
}
