//! Compensation du point noir (ticket #80), à la manière d'Adobe et de la
//! norme ICC : avant l'aller-retour colorimétrique relatif, la couleur
//! demandée est ramenée de façon que le noir de l'image tombe sur le noir le
//! plus profond du profil de sortie, le blanc restant le blanc.
//!
//! Les deux noirs sont pris neutres (a\* = b\* = 0) et l'échelle est
//! linéaire en XYZ relatifs D50, composante par composante :
//!
//! `XYZ' = s · XYZ + (1 − s) · D50`, avec `s = (1 − Y_sortie) / (1 − Y_image)`.
//!
//! C'est la formule d'Adobe pour deux noirs neutres : le noir de l'image
//! (Y_image) donne le noir de sortie (Y_sortie), le blanc D50 reste D50.

use serde::Serialize;

use crate::icc::{lab_vers_xyz, vers_lab_pcs, ProfilIcc, ILLUMINANT_D50};
use crate::source::ProfilSource;

/// Clarté L\* la plus haute admise pour le noir du profil de sortie : au-delà,
/// le profil est jugé sans vrai noir et le noir est ramené à cette valeur.
pub const L_NOIR_MAX: f64 = 50.0;

/// Rendu de l'aller-retour d'une image dans un profil de sortie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rendu {
    /// Colorimétrique relatif seul : la couleur visée est la couleur demandée.
    #[default]
    Relatif,
    /// Colorimétrique relatif avec compensation du point noir.
    PointNoir,
}

/// La compensation du point noir d'un profil d'image vers un profil de
/// sortie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointNoir {
    /// Noir de l'image, en L\*a\*b\* relatifs D50 (neutre).
    pub noir_image: [f64; 3],
    /// Noir du profil de sortie, en L\*a\*b\* relatifs D50 (neutre).
    pub noir_sortie: [f64; 3],
    /// Facteur `s` de la formule ; 1 quand la compensation ne fait rien.
    echelle: f64,
}

impl PointNoir {
    /// Noir de l'image : couleur demandée de RVB (0, 0, 0). Noir de sortie :
    /// table A2B1 en RVB (0, 0, 0), le maximum d'encre d'une imprimante RVB,
    /// clarté bornée à `L_NOIR_MAX`. Tous deux rendus neutres. Une valeur non
    /// finie, ou un noir d'image aussi clair que le blanc, laisse les
    /// couleurs telles quelles.
    pub fn nouveau(source: &ProfilSource, sortie: &ProfilIcc) -> PointNoir {
        let neutre = |l: f64| [l, 0.0, 0.0];
        let noir_image = neutre(source.demande([0.0; 3])[0].max(0.0));
        let noir_sortie = neutre(sortie.lab_relatif([0.0; 3])[0].clamp(0.0, L_NOIR_MAX));
        let y = |lab: [f64; 3]| lab_vers_xyz(lab, ILLUMINANT_D50)[1];
        let (y_image, y_sortie) = (y(noir_image), y(noir_sortie));
        let echelle = (1.0 - y_sortie) / (1.0 - y_image);
        let utilisable = y_image.is_finite() && y_sortie.is_finite() && y_image < 1.0;
        PointNoir {
            noir_image,
            noir_sortie,
            echelle: if utilisable && echelle.is_finite() {
                echelle
            } else {
                1.0
            },
        }
    }

    /// Couleur visée d'une couleur demandée (L\*a\*b\* relatifs D50).
    pub fn viser(&self, lab: [f64; 3]) -> [f64; 3] {
        if self.echelle == 1.0 {
            return lab;
        }
        let s = self.echelle;
        let xyz = lab_vers_xyz(lab, ILLUMINANT_D50);
        let compense = [0, 1, 2].map(|k| s * xyz[k] + (1.0 - s) * ILLUMINANT_D50[k]);
        vers_lab_pcs(compense, ILLUMINANT_D50)
    }
}

/// Compensation à appliquer pour `rendu` (aucune en colorimétrique relatif
/// seul).
pub(crate) fn compensation(
    rendu: Rendu,
    source: &ProfilSource,
    sortie: &ProfilIcc,
) -> Option<PointNoir> {
    match rendu {
        Rendu::Relatif => None,
        Rendu::PointNoir => Some(PointNoir::nouveau(source, sortie)),
    }
}
