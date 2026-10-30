//! Pastilles des couleurs hors gamut d'une image (ticket #80) : la couleur
//! de l'image (demandée) à côté de la couleur imprimée (reproduite), pour les
//! cellules du nuage les plus présentes, et pour un pixel montré à la
//! pipette.

use colorimetrie::delta_e00;
use serde::Serialize;

use crate::icc::{vers_lab, ErreurIcc, ProfilIcc};
use crate::image::code_ecart;
use crate::point_noir::{compensation, PointNoir, Rendu};
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
    /// Couleur visée, sur la même base : la couleur demandée après
    /// compensation du point noir ; `None` sans compensation (la couleur
    /// visée est alors la couleur demandée).
    pub visee: Option<[f64; 3]>,
    /// Couleur reproduite (aller-retour de la couleur visée), sur la même
    /// base.
    pub reproduite: [f64; 3],
    /// Couleurs écran `#rrggbb`, calculées comme celles du nuage.
    pub couleur_demandee: String,
    pub couleur_visee: Option<String>,
    pub couleur_reproduite: String,
    /// ΔE00 entre la couleur demandée et la couleur visée, en Lab relatif ;
    /// `None` sans compensation.
    pub ecart_visee: Option<f64>,
    /// ΔE00 entre la couleur visée et la couleur reproduite, en Lab
    /// relatif, comme l'écart des pixels.
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

/// Aller-retour d'une couleur demandée (Lab relatif) : sa couleur visée
/// (après compensation du point noir s'il y en a une), la couleur reproduite
/// de la visée et leur écart. `None` si l'écart n'est pas calculable.
pub(crate) fn aller_retour(
    demandee: [f64; 3],
    point_noir: Option<&PointNoir>,
    sortie: &ProfilIcc,
) -> Option<([f64; 3], [f64; 3], f64)> {
    let visee = point_noir.map_or(demandee, |p| p.viser(demandee));
    let reproduite = sortie.reproduire(visee).ok()?;
    let ecart = delta_e00(vers_lab(visee), vers_lab(reproduite)).ok()?;
    Some((visee, reproduite, ecart))
}

/// Paire d'une couleur demandée, de sa couleur visée (avec compensation) et
/// de sa couleur reproduite, toutes en Lab relatif, d'écart `ecart`.
pub(crate) fn paire(
    demandee: [f64; 3],
    visee: Option<[f64; 3]>,
    reproduite: [f64; 3],
    ecart: f64,
    sortie: &ProfilIcc,
) -> Paire {
    let ecart_visee = visee.and_then(|v| delta_e00(vers_lab(demandee), vers_lab(v)).ok());
    let absolu = |lab| sortie.relatif_vers_absolu(lab);
    let (demandee, reproduite, visee) = (absolu(demandee), absolu(reproduite), visee.map(absolu));
    Paire {
        couleur_demandee: couleur_ecran(demandee).0,
        couleur_visee: visee.map(|v| couleur_ecran(v).0),
        couleur_reproduite: couleur_ecran(reproduite).0,
        demandee,
        visee,
        reproduite,
        ecart_visee,
        ecart,
        code: code_ecart(ecart),
    }
}

/// Paire d'une couleur RVB de l'image (profil `source`) face au profil
/// `sortie`, par le même calcul que les pixels d'une analyse de même
/// `rendu`. `Ok(None)` si l'écart n'est pas calculable (valeur non finie) ;
/// un profil sans table inverse lisible rend sa raison.
pub fn paire_couleur(
    rvb: [u8; 3],
    source: &ProfilSource,
    sortie: &ProfilIcc,
    rendu: Rendu,
) -> Result<Option<Paire>, ErreurIcc> {
    sortie.inverse()?;
    let point_noir = compensation(rendu, source, sortie);
    let demandee = source.demande(rvb.map(|v| f64::from(v) / 255.0));
    Ok(
        aller_retour(demandee, point_noir.as_ref(), sortie).map(|(visee, reproduite, ecart)| {
            let visee = point_noir.map(|_| visee);
            paire(demandee, visee, reproduite, ecart, sortie)
        }),
    )
}

/// Couleur représentante d'une cellule : la plus fréquente, et à égalité la
/// première dans l'ordre des valeurs RVB.
#[derive(Clone, Copy)]
pub(crate) struct Representant {
    pub rvb: u32,
    pub pixels: u32,
    pub demandee: [f64; 3],
    /// Couleur visée, avec compensation du point noir seulement.
    pub visee: Option<[f64; 3]>,
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
            paire: paire(r.demandee, r.visee, r.reproduite, r.ecart, sortie),
        })
        .collect()
}
