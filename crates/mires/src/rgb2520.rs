//! Mire FD-9 RGB-2520, reprise telle quelle (décision du mainteneur, 26
//! octobre 2026). Le fichier n'est ni redessiné, ni recadré, ni recompressé :
//! la crate en vérifie l'empreinte et la forme, et lit ses valeurs au centre
//! des plages. Les valeurs de référence sont celles du TIFF, parce que c'est
//! lui qui est imprimé.

use std::io::Cursor;

use ::tiff::decoder::{Decoder, DecodingResult};
use ::tiff::ColorType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ErreurMire;

/// Nom du fichier posé dans `mire-fd9/`.
pub const RGB2520_FICHIER: &str = "RGB-2520 A3 - 1p.tif";
/// SHA-256 du fichier retenu.
pub const RGB2520_EMPREINTE: &str =
    "7550f4a38725ad9260982152bb6bcbe077922289adb48dd954f0bd7d2c7fc713";
/// Nom porté par son QR code (`FD-9:011M:RGB-2520 - 1P MY9*1+` ; le sens de
/// `011M` et `*1+` est à vérifier).
pub const RGB2520_NOM: &str = "RGB-2520 - 1P MY9";

/// Géométrie de la zone de plages, en pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometrieFd9 {
    pub largeur_px: u32,
    pub hauteur_px: u32,
    /// Coin haut gauche de la première plage.
    pub origine_px: (u32, u32),
    /// Côté d'une plage ; les plages sont jointives.
    pub pas_px: u32,
    pub colonnes: usize,
    pub rangees: usize,
}

/// Géométrie mesurée sur le TIFF le 26 octobre 2026, à 300 ppp : une zone de
/// 42 × 60 plages de 71 pixels (6,01 mm), à 13,46 mm du bord gauche et
/// 24,13 mm du haut.
pub const RGB2520_GEOMETRIE: GeometrieFd9 = GeometrieFd9 {
    largeur_px: 3300,
    hauteur_px: 4961,
    origine_px: (159, 285),
    pas_px: 71,
    colonnes: 42,
    rangees: 60,
};

/// Distance au bord de la zone où le blanc est contrôlé.
const BORD_BLANC_PX: u32 = 3;

/// La mire lue : octets d'origine (copiés tels quels), empreinte, géométrie
/// et valeurs RVB de 0 à 100, dans l'ordre des plages (rangée par rangée, de
/// gauche à droite, de haut en bas).
#[derive(Clone, Debug, PartialEq)]
pub struct MireFd9 {
    pub octets: Vec<u8>,
    pub empreinte: String,
    pub geometrie: GeometrieFd9,
    pub valeurs: Vec<[f64; 3]>,
}

/// Lit et vérifie la mire : empreinte d'abord, puis TIFF RVB 8 bits,
/// dimensions et 2 520 plages.
pub fn lire_mire_fd9(octets: Vec<u8>) -> Result<MireFd9, ErreurMire> {
    let empreinte = empreinte(&octets);
    if empreinte != RGB2520_EMPREINTE {
        return Err(ErreurMire::MireFd9Autre { empreinte });
    }
    lire_mire_fd9_sans_empreinte(octets, &RGB2520_GEOMETRIE)
}

/// Même lecture, sans contrôle d'empreinte : pour les tests sur un TIFF
/// fictif de même forme.
pub fn lire_mire_fd9_sans_empreinte(
    octets: Vec<u8>,
    geometrie: &GeometrieFd9,
) -> Result<MireFd9, ErreurMire> {
    let pixels = lire_pixels(&octets, geometrie)?;
    let g = geometrie;
    let pixel = |x: u32, y: u32| {
        let i = (y as usize * g.largeur_px as usize + x as usize) * 3;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    controler_blanc(g, &pixel)?;
    let (x0, y0) = g.origine_px;
    let mut valeurs = Vec::with_capacity(g.colonnes * g.rangees);
    for rangee in 0..g.rangees as u32 {
        for colonne in 0..g.colonnes as u32 {
            let x = x0 + g.pas_px * colonne + g.pas_px / 2;
            let y = y0 + g.pas_px * rangee + g.pas_px / 2;
            valeurs.push(pixel(x, y).map(|o| o as f64 / 2.55));
        }
    }
    Ok(MireFd9 {
        empreinte: empreinte(&octets),
        octets,
        geometrie: geometrie.clone(),
        valeurs,
    })
}

/// SHA-256 hexadécimal, en minuscules.
fn empreinte(octets: &[u8]) -> String {
    format!("{:x}", Sha256::digest(octets))
}

/// Pixels RVB entrelacés, après contrôle du type et des dimensions, et de la
/// place de la zone de plages dans l'image.
fn lire_pixels(octets: &[u8], g: &GeometrieFd9) -> Result<Vec<u8>, ErreurMire> {
    let illisible = |cause: String| ErreurMire::MireFd9Illisible(cause);
    let mut decodeur = Decoder::new(Cursor::new(octets))
        .map_err(|e| illisible(format!("TIFF illisible : {e}")))?;
    let type_couleur = decodeur
        .colortype()
        .map_err(|e| illisible(format!("TIFF illisible : {e}")))?;
    if type_couleur != ColorType::RGB(8) {
        return Err(illisible(format!(
            "image en {type_couleur:?}, RVB 8 bits attendu"
        )));
    }
    let dimensions = decodeur
        .dimensions()
        .map_err(|e| illisible(format!("TIFF illisible : {e}")))?;
    if dimensions != (g.largeur_px, g.hauteur_px) {
        return Err(illisible(format!(
            "image de {} × {} pixels, {} × {} attendus",
            dimensions.0, dimensions.1, g.largeur_px, g.hauteur_px
        )));
    }
    let fin_x = g.origine_px.0 as u64 + g.pas_px as u64 * g.colonnes as u64 + BORD_BLANC_PX as u64;
    let fin_y = g.origine_px.1 as u64 + g.pas_px as u64 * g.rangees as u64 + BORD_BLANC_PX as u64;
    if g.origine_px.0 < BORD_BLANC_PX
        || g.origine_px.1 < BORD_BLANC_PX
        || fin_x > g.largeur_px as u64
        || fin_y > g.hauteur_px as u64
        || g.pas_px == 0
    {
        return Err(illisible("zone de plages hors de l'image".to_string()));
    }
    match decodeur
        .read_image()
        .map_err(|e| illisible(format!("TIFF illisible : {e}")))?
    {
        DecodingResult::U8(pixels)
            if pixels.len() == g.largeur_px as usize * g.hauteur_px as usize * 3 =>
        {
            Ok(pixels)
        }
        _ => Err(illisible("pixels illisibles".to_string())),
    }
}

/// Le blanc entoure la zone de plages, à [`BORD_BLANC_PX`] pixels de son
/// bord : aux quatre coins et au milieu de chaque côté. Sinon, la zone n'est
/// pas là où la géométrie la place.
fn controler_blanc(
    g: &GeometrieFd9,
    pixel: &impl Fn(u32, u32) -> [u8; 3],
) -> Result<(), ErreurMire> {
    let (x0, y0) = g.origine_px;
    let gauche = x0 - BORD_BLANC_PX;
    let haut = y0 - BORD_BLANC_PX;
    let droite = x0 + g.pas_px * g.colonnes as u32 + BORD_BLANC_PX - 1;
    let bas = y0 + g.pas_px * g.rangees as u32 + BORD_BLANC_PX - 1;
    let milieu_x = (gauche + droite) / 2;
    let milieu_y = (haut + bas) / 2;
    let points = [
        (gauche, haut),
        (milieu_x, haut),
        (droite, haut),
        (gauche, milieu_y),
        (droite, milieu_y),
        (gauche, bas),
        (milieu_x, bas),
        (droite, bas),
    ];
    match points.into_iter().find(|&(x, y)| pixel(x, y) != [255; 3]) {
        None => Ok(()),
        Some((x, y)) => Err(ErreurMire::MireFd9Illisible(format!(
            "pas de blanc autour de la zone de plages (pixel {x}, {y})"
        ))),
    }
}
