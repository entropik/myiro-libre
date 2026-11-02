//! Vignettes du rapport d'une image (ticket #89) : la photo, et sa
//! surimpression où chaque pixel au-delà du seuil est en vert fluo, comme à
//! l'écran (`--couleur-hors-gamut-calque`). Au plus `cote_max` pixels de
//! côté, proportions gardées, en JPEG de qualité 90 : une photo en PNG
//! pesait plusieurs Mo dans le PDF, pour une différence invisible à
//! l'impression (ADR 0008).

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};

/// Vert fluo de la surimpression, le même que le jeton
/// `--couleur-hors-gamut-calque` du système graphique.
pub const VERT_SURIMPRESSION: [u8; 4] = [0x39, 0xff, 0x14, 0xff];
/// Code d'un pixel dont l'écart est inconnu (`gamut::ECART_INCONNU`).
const INCONNU: u8 = 255;
/// Qualité JPEG des vignettes : sans perte visible au tirage du rapport.
pub const QUALITE_JPEG: u8 = 90;

fn jpeg(pixels: &[[u8; 4]], largeur: u32, cote_max: u32, filtre: FilterType) -> Vec<u8> {
    let largeur = largeur.max(1);
    let hauteur = (pixels.len() as u32 / largeur).max(1);
    let octets: Vec<u8> = pixels.iter().flatten().copied().collect();
    let mut image = RgbaImage::from_raw(largeur, hauteur, octets)
        .map(DynamicImage::ImageRgba8)
        .unwrap_or_else(|| DynamicImage::new_rgba8(1, 1));
    if largeur.max(hauteur) > cote_max {
        image = image.resize(cote_max, cote_max, filtre);
    }
    let mut sortie = Vec::new();
    // L'écriture d'un JPEG en mémoire n'échoue pas ; l'alpha est laissé.
    let _ = image
        .to_rgb8()
        .write_with_encoder(JpegEncoder::new_with_quality(
            Cursor::new(&mut sortie),
            QUALITE_JPEG,
        ));
    sortie
}

/// La photo (pixels RVBA de l'image d'analyse, ligne par ligne) en JPEG.
pub fn vignette(pixels: &[[u8; 4]], largeur: u32, cote_max: u32) -> Vec<u8> {
    jpeg(pixels, largeur, cote_max, FilterType::Triangle)
}

/// Les pixels de la photo, chaque pixel au-delà du seuil en vert fluo :
/// code d'écart au moins `round(10 × seuil)` et connu, la règle de la page.
pub fn marques_surimpression(pixels: &[[u8; 4]], ecarts: &[u8], seuil: f64) -> Vec<[u8; 4]> {
    let code = (seuil * 10.0).round().clamp(0.0, 254.0) as u8;
    pixels
        .iter()
        .zip(ecarts.iter().chain(std::iter::repeat(&INCONNU)))
        .map(|(p, &e)| {
            if e != INCONNU && e >= code {
                VERT_SURIMPRESSION
            } else {
                *p
            }
        })
        .collect()
}

/// La surimpression ([`marques_surimpression`]) en JPEG.
pub fn vignette_surimpression(
    pixels: &[[u8; 4]],
    ecarts: &[u8],
    largeur: u32,
    seuil: f64,
    cote_max: u32,
) -> Vec<u8> {
    let marques = marques_surimpression(pixels, ecarts, seuil);
    // Plus proche voisin : le vert reste franc une fois réduit.
    jpeg(&marques, largeur, cote_max, FilterType::Nearest)
}
