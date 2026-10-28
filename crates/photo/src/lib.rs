//! Ouvrir une photo de client pour la comparer à un profil (ticket #80).
//!
//! La crate décode un JPEG, un PNG ou un TIFF en RVB ou en gris, reconnu par
//! son contenu et non par son extension. Elle rend le profil ICC embarqué tel
//! quel, applique l'orientation Exif et ramène l'image à une **image
//! d'analyse** d'au plus `COTE_MAX_ANALYSE` pixels de côté, un pixel sur
//! `pas` dans chaque sens, sans mélange de voisins.
//!
//! Une image CMJN est refusée avant tout calcul (`RefusPhoto::Cmjn`) : la
//! comparaison des images CMJN est une seconde étape. L'image ne vit qu'en
//! mémoire : rien n'est écrit sur le disque.

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::tiff::TiffDecoder;
use image::{
    DynamicImage, ExtendedColorType, GenericImageView, ImageDecoder, ImageFormat, ImageReader,
    Limits, RgbaImage,
};

/// Plus grand côté de l'image d'analyse, en pixels.
pub const COTE_MAX_ANALYSE: u32 = 2_048;
/// Au-delà de ce nombre de pixels, la photo est refusée sans être décodée.
pub const PIXELS_MAX_ORIGINE: u64 = 120_000_000;
/// Mémoire que le décodeur peut prendre (`Limits::max_alloc`).
pub const OCTETS_MAX_DECODAGE: u64 = 1 << 30;

/// Pourquoi une photo ne s'ouvre pas.
#[derive(Debug, PartialEq)]
pub enum RefusPhoto {
    /// Fichier qui n'est pas une image, ou image abîmée ; la cause du
    /// décodeur suit.
    Illisible(String),
    /// Image d'un autre format que JPEG, PNG ou TIFF (GIF, WebP…).
    FormatNonPrisEnCharge,
    /// Image CMJN : prévue dans une seconde étape.
    Cmjn,
    /// Plus de `PIXELS_MAX_ORIGINE` pixels.
    TropGrande { pixels: u64 },
}

impl RefusPhoto {
    /// Clé du texte affiché (catalogue `textes.rs` de l'application).
    pub fn cle(&self) -> &'static str {
        match self {
            RefusPhoto::Illisible(_) => "image.refus.illisible",
            RefusPhoto::FormatNonPrisEnCharge => "image.refus.format",
            RefusPhoto::Cmjn => "image.refus.cmjn",
            RefusPhoto::TropGrande { .. } => "image.refus.trop_grande",
        }
    }
}

/// Une photo ouverte, réduite à son image d'analyse.
#[derive(Debug)]
pub struct Photo {
    /// Taille du fichier, avant orientation.
    pub largeur_origine: u32,
    pub hauteur_origine: u32,
    /// Taille de l'image d'analyse, orientation appliquée.
    pub largeur: u32,
    pub hauteur: u32,
    /// Un pixel sur `pas` dans chaque sens (approché si `pas` > 1).
    pub pas: u32,
    /// Valeurs de 16 bits ramenées à 8 bits (approché).
    pub seize_bits: bool,
    pub gris: bool,
    /// Profil embarqué, tel quel.
    pub icc: Option<Vec<u8>>,
    /// `largeur` × `hauteur` pixels, ligne par ligne.
    pub rvba: Vec<[u8; 4]>,
}

/// Ouvre une photo depuis ses octets, sans rien écrire sur le disque.
pub fn ouvrir(octets: &[u8]) -> Result<Photo, RefusPhoto> {
    let format = ImageReader::new(Cursor::new(octets))
        .with_guessed_format()
        .map_err(illisible)?
        .format();
    let source = Cursor::new(octets);
    match format {
        Some(ImageFormat::Jpeg) => {
            // Le décodeur JPEG passe le CMJN en RVB sans le dire : on compte
            // les composantes de l'en-tête avant lui.
            if composantes_jpeg(octets) == Some(4) {
                return Err(RefusPhoto::Cmjn);
            }
            lire(JpegDecoder::new(source).map_err(illisible)?)
        }
        Some(ImageFormat::Png) => lire(PngDecoder::new(source).map_err(illisible)?),
        Some(ImageFormat::Tiff) => lire(TiffDecoder::new(source).map_err(illisible)?),
        Some(_) => Err(RefusPhoto::FormatNonPrisEnCharge),
        None => Err(RefusPhoto::Illisible("format inconnu".into())),
    }
}

/// Le profil est lu avant de poser les limites : une fois posées, le
/// décodeur TIFF borne la lecture des balises à la taille de l'image
/// décodée, et une petite image perdrait un grand profil.
fn lire(mut decodeur: impl ImageDecoder) -> Result<Photo, RefusPhoto> {
    let icc = decodeur.icc_profile().map_err(illisible)?;
    let cmjn_decode = matches!(
        decodeur.original_color_type(),
        ExtendedColorType::Cmyk8 | ExtendedColorType::Cmyk16
    );
    let cmjn_profil = icc.as_ref().and_then(|p| p.get(16..20)) == Some(b"CMYK".as_slice());
    if cmjn_decode || cmjn_profil {
        return Err(RefusPhoto::Cmjn);
    }
    let orientation = decodeur.orientation().map_err(illisible)?;
    let (largeur_origine, hauteur_origine) = decodeur.dimensions();
    let type_pixels = decodeur.color_type();
    let seize_bits = type_pixels.bytes_per_pixel() > type_pixels.channel_count();
    let gris = !type_pixels.has_color();
    let pixels = u64::from(largeur_origine) * u64::from(hauteur_origine);
    if pixels > PIXELS_MAX_ORIGINE {
        return Err(RefusPhoto::TropGrande { pixels });
    }
    let mut limites = Limits::default();
    limites.max_alloc = Some(OCTETS_MAX_DECODAGE);
    decodeur.set_limits(limites).map_err(illisible)?;
    let image = DynamicImage::from_decoder(decodeur).map_err(illisible)?;
    let pas = pas_analyse(largeur_origine, hauteur_origine);
    let mut analyse = DynamicImage::ImageRgba8(echantillonner(&image, pas));
    drop(image);
    analyse.apply_orientation(orientation);
    let (largeur, hauteur) = (analyse.width(), analyse.height());
    let rvba = analyse.to_rgba8().pixels().map(|p| p.0).collect();
    Ok(Photo {
        largeur_origine,
        hauteur_origine,
        largeur,
        hauteur,
        pas,
        seize_bits,
        gris,
        icc,
        rvba,
    })
}

/// Plus petit pas qui ramène les deux côtés à `COTE_MAX_ANALYSE` au plus.
fn pas_analyse(largeur: u32, hauteur: u32) -> u32 {
    largeur.max(hauteur).div_ceil(COTE_MAX_ANALYSE).max(1)
}

/// Un pixel tous les `pas` dans chaque sens, en partant du coin, sans
/// moyenne : une moyenne de voisins inventerait des couleurs moins saturées
/// que la photo. Les valeurs de 16 bits sont ramenées à 8 bits au passage.
fn echantillonner(image: &DynamicImage, pas: u32) -> RgbaImage {
    let largeur = image.width().div_ceil(pas);
    let hauteur = image.height().div_ceil(pas);
    RgbaImage::from_fn(largeur, hauteur, |x, y| image.get_pixel(x * pas, y * pas))
}

/// Nombre de composantes du premier en-tête de trame (SOFn) d'un JPEG :
/// 4 pour du CMJN ou du YCCK. `None` si l'en-tête n'est pas trouvé.
fn composantes_jpeg(octets: &[u8]) -> Option<u8> {
    let mut i = 2;
    while i + 4 <= octets.len() {
        if octets[i] != 0xFF {
            return None;
        }
        let marqueur = octets[i + 1];
        if marqueur == 0xFF {
            i += 1;
            continue;
        }
        let longueur = usize::from(u16::from_be_bytes([octets[i + 2], octets[i + 3]]));
        let trame = (0xC0..=0xCF).contains(&marqueur) && ![0xC4, 0xC8, 0xCC].contains(&marqueur);
        if trame {
            return octets.get(i + 9).copied();
        }
        if marqueur == 0xDA || marqueur == 0xD9 {
            return None;
        }
        i += 2 + longueur;
    }
    None
}

fn illisible(erreur: impl std::fmt::Display) -> RefusPhoto {
    RefusPhoto::Illisible(erreur.to_string())
}
