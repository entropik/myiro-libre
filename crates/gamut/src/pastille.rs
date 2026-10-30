//! Pastilles des couleurs hors gamut d'une image (ticket #80) : la couleur
//! de l'image (demandée) à côté de la couleur imprimée (reproduite), pour les
//! cellules du nuage les plus présentes, et pour un pixel montré à la
//! pipette.

use colorimetrie::delta_e00;
use serde::Serialize;

use crate::icc::{vers_lab, ErreurIcc, ProfilIcc};
use crate::image::code_ecart;
use crate::source::ProfilSource;
use crate::surface::couleur_ecran;

/// Pastilles d'une analyse, au plus.
pub const PASTILLES: usize = 48;
/// Plus petit seuil d'alerte que la page autorise, en ΔE00 : une cellule n'a
/// sa pastille que si l'écart de sa couleur représentante l'atteint (même
/// règle que les pixels, `code ≥ round(10 × seuil)`).
pub const SEUIL_PASTILLES: f64 = 0.5;

/// Une couleur de l'image et sa couleur imprimée par un profil de sortie.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paire {
    /// Couleur demandée, en L\*a\*b\* absolus sur le blanc du papier du
    /// profil de sortie (la base du nuage).
    pub demandee: [f64; 3],
    /// Couleur reproduite, sur la même base.
    pub reproduite: [f64; 3],
    /// Couleurs écran `#rrggbb` des deux, calculées comme celles du nuage.
    pub couleur_demandee: String,
    pub couleur_reproduite: String,
    /// ΔE00 entre les deux, en Lab relatif, comme l'écart des pixels.
    pub ecart: f64,
    /// Code d'écart (dixièmes de ΔE00 arrondis vers le bas, bornés à 254) :
    /// la page le compare au seuil comme celui d'un pixel.
    pub code: u8,
}

/// Une cellule du nuage hors gamut, représentée par sa couleur RVB la plus
/// fréquente.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pastille {
    /// Couleur RVB représentante (la plus fréquente de la cellule).
    pub rvb: [u8; 3],
    /// Pixels de toute la cellule.
    pub pixels: u32,
    #[serde(flatten)]
    pub paire: Paire,
}

/// Paire d'une couleur demandée et de sa couleur reproduite, toutes deux en
/// Lab relatif, d'écart `ecart`.
pub(crate) fn paire(
    demandee: [f64; 3],
    reproduite: [f64; 3],
    ecart: f64,
    sortie: &ProfilIcc,
) -> Paire {
    let demandee = sortie.relatif_vers_absolu(demandee);
    let reproduite = sortie.relatif_vers_absolu(reproduite);
    Paire {
        couleur_demandee: couleur_ecran(demandee).0,
        couleur_reproduite: couleur_ecran(reproduite).0,
        demandee,
        reproduite,
        ecart,
        code: code_ecart(ecart),
    }
}

/// Paire d'une couleur RVB de l'image (profil `source`) face au profil
/// `sortie`, par le même calcul que les pixels d'une analyse. `Ok(None)` si
/// l'écart n'est pas calculable (valeur non finie) ; un profil sans table
/// inverse lisible rend sa raison.
pub fn paire_couleur(
    rvb: [u8; 3],
    source: &ProfilSource,
    sortie: &ProfilIcc,
) -> Result<Option<Paire>, ErreurIcc> {
    sortie.inverse()?;
    let demandee = source.demande(rvb.map(|v| f64::from(v) / 255.0));
    Ok(sortie.reproduire(demandee).ok().and_then(|reproduite| {
        delta_e00(vers_lab(demandee), vers_lab(reproduite))
            .ok()
            .map(|ecart| paire(demandee, reproduite, ecart, sortie))
    }))
}

/// Couleur représentante d'une cellule : la plus fréquente, et à égalité la
/// première dans l'ordre des valeurs RVB.
#[derive(Clone, Copy)]
pub(crate) struct Representant {
    pub rvb: u32,
    pub pixels: u32,
    pub demandee: [f64; 3],
    pub reproduite: [f64; 3],
    pub ecart: f64,
}

/// Les `PASTILLES` cellules les plus peuplées dont le représentant atteint
/// `SEUIL_PASTILLES`. `cellules` : (clé, pixels de la cellule, représentant).
pub(crate) fn pastilles(
    mut cellules: Vec<([i32; 3], u64, Representant)>,
    sortie: &ProfilIcc,
) -> Vec<Pastille> {
    let minimum = code_ecart(SEUIL_PASTILLES);
    cellules.retain(|(_, _, r)| code_ecart(r.ecart) >= minimum);
    cellules.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    cellules.truncate(PASTILLES);
    cellules
        .into_iter()
        .map(|(_, pixels, r)| Pastille {
            rvb: [r.rvb >> 16, r.rvb >> 8, r.rvb].map(|v| (v & 0xff) as u8),
            pixels: u32::try_from(pixels).unwrap_or(u32::MAX),
            paire: paire(r.demandee, r.reproduite, r.ecart, sortie),
        })
        .collect()
}
