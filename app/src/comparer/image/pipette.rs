//! La pipette : un clic sur l'image affichée montre la couleur du pixel
//! touché et sa couleur imprimée par chaque profil affiché, calculées à la
//! demande sur l'image d'analyse gardée en mémoire, par le même calcul que
//! les pastilles (`gamut::paire_couleur`).

use std::sync::Arc;

use gamut::{Paire, ProfilIcc, ProfilSource, Rendu};
use serde::{Deserialize, Serialize};

use super::{detail_refus, ImageOuverte, AUCUNE, PROFIL_REFUSE};
use crate::colonne::BibliothequeOuverte;
use crate::comparer::{FichiersOuverts, SourceComparaison};

/// Le clic est en dehors de l'image.
pub(super) const HORS_IMAGE: &str = "image.pixel.hors_image";
/// Le pixel touché est transparent : il n'a pas de couleur à imprimer.
pub(super) const TRANSPARENT: &str = "image.pixel.transparent";
/// L'écart du pixel n'a pas pu être calculé (valeur non finie).
pub(super) const PIXEL_INCONNU: &str = "image.pixel.inconnu";

/// Un clic sur l'image affichée : position dans la boîte de l'image (`x` de 0
/// à `largeur`, `y` de 0 à `hauteur`, sans son filet), en points d'écran.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct PointAffiche {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// La couleur d'un pixel face à un profil, ou pourquoi elle est inconnue.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatPixel {
    Calcule {
        paire: Paire,
    },
    Inconnu {
        cle: &'static str,
        detail: Option<String>,
    },
}

/// Le pixel touché : sa place dans l'image d'analyse, sa valeur RVB, et sa
/// couleur imprimée par chaque profil, dans l'ordre des profils demandés.
#[derive(Clone, Debug, Serialize)]
pub struct PixelImage {
    pub x: u32,
    pub y: u32,
    pub rvb: [u8; 3],
    pub profils: Vec<EtatPixel>,
}

/// Le pixel de l'image d'analyse (`largeur` × `hauteur`) sous un clic dans
/// la boîte où elle est affichée. L'image y est montrée entière, à ses
/// proportions, et centrée (`object-fit: contain`) : une boîte d'autres
/// proportions laisse deux bandes vides. `None` dans une bande, en dehors de
/// la boîte, ou si une taille est nulle ou une valeur non finie.
pub fn pixel_affiche(point: PointAffiche, largeur: u32, hauteur: u32) -> Option<(u32, u32)> {
    let finis = [point.x, point.y, point.largeur, point.hauteur];
    if !finis.iter().all(|v| v.is_finite()) || point.largeur <= 0.0 || point.hauteur <= 0.0 {
        return None;
    }
    if largeur == 0 || hauteur == 0 {
        return None;
    }
    let (l, h) = (f64::from(largeur), f64::from(hauteur));
    // Points d'écran par pixel de l'image d'analyse.
    let echelle = (point.largeur / l).min(point.hauteur / h);
    let axe = |v: f64, boite: f64, pixels: u32, cote: f64| {
        let marge = (boite - cote * echelle) / 2.0;
        let dans_l_image = (v - marge) / echelle;
        if dans_l_image < 0.0 || dans_l_image >= cote {
            return None;
        }
        // Une valeur juste sous le bord ne déborde jamais du dernier pixel.
        Some((dans_l_image.floor() as u32).min(pixels - 1))
    };
    Some((
        axe(point.x, point.largeur, largeur, l)?,
        axe(point.y, point.hauteur, hauteur, h)?,
    ))
}

impl ImageOuverte {
    /// Le pixel sous `point` et le profil de l'image, copiés sous le verrou.
    fn pixel(&self, point: PointAffiche) -> Result<(u32, u32, [u8; 3], Arc<ProfilSource>), String> {
        let etat = self.verrou();
        let image = etat.image.as_ref().ok_or_else(|| AUCUNE.to_string())?;
        let (largeur, hauteur) = (image.resume.largeur, image.resume.hauteur);
        let (x, y) =
            pixel_affiche(point, largeur, hauteur).ok_or_else(|| HORS_IMAGE.to_string())?;
        let indice = y as usize * largeur as usize + x as usize;
        let p = image
            .pixels
            .get(indice)
            .ok_or_else(|| HORS_IMAGE.to_string())?;
        if p[3] == 0 {
            return Err(TRANSPARENT.to_string());
        }
        let profil = image
            .profil
            .as_ref()
            .map(Arc::clone)
            .ok_or_else(|| PROFIL_REFUSE.to_string())?;
        Ok((x, y, [p[0], p[1], p[2]], profil))
    }
}

/// La couleur du pixel `rvb` face à un profil de sortie (octets ICC).
fn etat_pixel(icc: &[u8], rvb: [u8; 3], source: &ProfilSource, rendu: Rendu) -> EtatPixel {
    let inconnu = |e: &gamut::ErreurIcc| EtatPixel::Inconnu {
        cle: e.cle(),
        detail: detail_refus(e),
    };
    let sortie = match ProfilIcc::lire(icc) {
        Ok(p) => p,
        Err(e) => return inconnu(&e),
    };
    match gamut::paire_couleur(rvb, source, &sortie, rendu) {
        Ok(Some(paire)) => EtatPixel::Calcule { paire },
        Ok(None) => EtatPixel::Inconnu {
            cle: PIXEL_INCONNU,
            detail: None,
        },
        Err(e) => inconnu(&e),
    }
}

impl BibliothequeOuverte {
    /// Couleur du pixel sous un clic sur l'image ouverte, et sa couleur
    /// imprimée par chaque profil de `sources`, dans leur ordre, en `rendu`
    /// (celui de l'analyse affichée). Un clic en
    /// dehors de l'image ou sur un pixel transparent est refusé. Un profil
    /// illisible ou sans table inverse est « inconnu » sans empêcher les
    /// autres. Calcul hors de tout verrou, une couleur par profil.
    pub fn couleur_pixel(
        &self,
        image: &ImageOuverte,
        sources: &[SourceComparaison],
        fichiers: &FichiersOuverts,
        point: PointAffiche,
        rendu: Rendu,
    ) -> Result<PixelImage, String> {
        let (x, y, rvb, profil) = image.pixel(point)?;
        let lus = self.lire_sources(sources, fichiers)?;
        Ok(PixelImage {
            x,
            y,
            rvb,
            profils: lus
                .iter()
                .map(|l| etat_pixel(&l.icc, rvb, &profil, rendu))
                .collect(),
        })
    }
}
