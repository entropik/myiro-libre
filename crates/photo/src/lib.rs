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
    ColorType, DynamicImage, ExtendedColorType, GenericImageView, ImageDecoder, ImageError,
    ImageFormat, ImageReader, Limits, RgbaImage,
};

/// Plus grand côté de l'image d'analyse, en pixels.
pub const COTE_MAX_ANALYSE: u32 = 2_048;
/// Au-delà de ce nombre de pixels, la photo est refusée sans être décodée.
pub const PIXELS_MAX_ORIGINE: u64 = 120_000_000;
/// Mémoire que le décodeur peut prendre (`Limits::max_alloc`).
pub const OCTETS_MAX_DECODAGE: u64 = 1 << 30;
/// Mémoire que le décodeur PNG peut prendre pour lire l'en-tête : profil
/// décompressé, autres blocs de métadonnées et une ligne de pixels. Un
/// profil plus grand est dit illisible.
pub const OCTETS_MAX_METADONNEES_PNG: u64 = 16 << 20;

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
    /// Un profil est présent dans le fichier mais le décodeur n'a pas pu le
    /// lire (abîmé, incomplet, trop grand) : la raison, jamais un « sans
    /// profil » silencieux. `icc` vaut alors `None`, mais l'image ne doit
    /// **jamais** être traitée comme sRVB sans le choix explicite de
    /// l'opérateur (« Traiter comme sRVB »), affiché ensuite « supposé ».
    pub profil_illisible: Option<String>,
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
            // les composantes de l'en-tête avant lui. Un en-tête qu'on ne
            // sait pas lire est refusé, jamais pris pour du RVB.
            let en_tete = en_tete_jpeg(octets)
                .ok_or_else(|| RefusPhoto::Illisible("en-tête JPEG sans trame".into()))?;
            if en_tete.composantes == 4 {
                return Err(RefusPhoto::Cmjn);
            }
            let decodeur = JpegDecoder::new(source).map_err(refus)?;
            lire(decodeur, en_tete.profil)
        }
        Some(ImageFormat::Png) => {
            // Sans limite, le décodeur PNG décompresserait le profil
            // quelle que soit sa taille.
            let mut limites = Limits::default();
            limites.max_alloc = Some(OCTETS_MAX_METADONNEES_PNG);
            let decodeur = PngDecoder::with_limits(source, limites).map_err(refus)?;
            lire(decodeur, bloc_png_present(octets, b"iCCP"))
        }
        Some(ImageFormat::Tiff) => {
            let decodeur = TiffDecoder::new(source).map_err(refus)?;
            lire(decodeur, balise_tiff_presente(octets, BALISE_TIFF_ICC))
        }
        Some(_) => Err(RefusPhoto::FormatNonPrisEnCharge),
        None => Err(RefusPhoto::Illisible("format inconnu".into())),
    }
}

/// Le profil est lu avant de poser les limites : une fois posées, le
/// décodeur TIFF borne la lecture des balises à la taille de l'image
/// décodée, et une petite image perdrait un grand profil.
///
/// `profil_present` vient de notre propre lecture du fichier : les
/// décodeurs rendent « pas de profil » quand le leur est abîmé.
fn lire(mut decodeur: impl ImageDecoder, profil_present: bool) -> Result<Photo, RefusPhoto> {
    let icc = decodeur.icc_profile().map_err(refus)?;
    let profil_illisible = (profil_present && icc.is_none())
        .then(|| "profil intégré présent mais illisible par le décodeur".to_string());
    let cmjn_decode = matches!(
        decodeur.original_color_type(),
        ExtendedColorType::Cmyk8 | ExtendedColorType::Cmyk16
    );
    let cmjn_profil = icc.as_ref().and_then(|p| p.get(16..20)) == Some(b"CMYK".as_slice());
    if cmjn_decode || cmjn_profil {
        return Err(RefusPhoto::Cmjn);
    }
    let orientation = decodeur.orientation().map_err(refus)?;
    let (largeur_origine, hauteur_origine) = decodeur.dimensions();
    let type_pixels = decodeur.color_type();
    let seize_bits = matches!(
        type_pixels,
        ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16
    );
    let gris = !type_pixels.has_color();
    let pixels = u64::from(largeur_origine) * u64::from(hauteur_origine);
    if pixels > PIXELS_MAX_ORIGINE {
        return Err(RefusPhoto::TropGrande { pixels });
    }
    let mut limites = Limits::default();
    limites.max_alloc = Some(OCTETS_MAX_DECODAGE);
    decodeur.set_limits(limites).map_err(refus)?;
    let image = DynamicImage::from_decoder(decodeur).map_err(refus)?;
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
        profil_illisible,
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

/// Balise TIFF du profil ICC.
const BALISE_TIFF_ICC: u16 = 34_675;

/// Un bloc de ce type figure-t-il avant les données d'un PNG ?
fn bloc_png_present(octets: &[u8], genre: &[u8; 4]) -> bool {
    let mut i = 8;
    while let Some(tete) = octets.get(i..).and_then(|reste| reste.get(..8)) {
        let type_bloc = &tete[4..8];
        if type_bloc == genre {
            return true;
        }
        if type_bloc == b"IDAT" || type_bloc == b"IEND" {
            return false;
        }
        let longueur = u32::from_be_bytes([tete[0], tete[1], tete[2], tete[3]]);
        i = i.saturating_add(12).saturating_add(longueur as usize);
    }
    false
}

/// Une balise figure-t-elle dans le premier répertoire (IFD) d'un TIFF,
/// classique ou BigTIFF ?
fn balise_tiff_presente(octets: &[u8], balise: u16) -> bool {
    let petit = match octets.get(0..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return false,
    };
    let entier = |debut: usize, taille: usize| -> Option<u64> {
        let tranche = octets.get(debut..debut.checked_add(taille)?)?;
        let mut valeur = 0u64;
        for k in 0..taille {
            let octet = if petit {
                tranche[taille - 1 - k]
            } else {
                tranche[k]
            };
            valeur = (valeur << 8) | u64::from(octet);
        }
        Some(valeur)
    };
    let trouver = || -> Option<bool> {
        // Classique : nombre d'entrées sur 2 octets, entrées de 12 octets ;
        // BigTIFF (43) : sur 8 octets, entrées de 20 octets.
        let (debut, taille_nombre, taille_entree) = match entier(2, 2)? {
            42 => (entier(4, 4)?, 2, 12),
            43 => (entier(8, 8)?, 8, 20),
            _ => return None,
        };
        let debut = usize::try_from(debut).ok()?;
        let nombre = usize::try_from(entier(debut, taille_nombre)?).ok()?;
        let entrees = debut + taille_nombre;
        // Un nombre d'entrées farfelu s'arrête à la fin du fichier.
        let nombre = nombre.min(octets.len() / taille_entree);
        Some((0..nombre).any(|n| entier(entrees + n * taille_entree, 2) == Some(u64::from(balise))))
    };
    trouver().unwrap_or(false)
}

/// Ce que l'en-tête d'un JPEG dit avant décodage.
struct EnTeteJpeg {
    /// Composantes de la trame (SOFn) : 4 pour du CMJN ou du YCCK.
    composantes: u8,
    /// Un segment APP2 « ICC_PROFILE » est présent.
    profil: bool,
}

/// Parcourt les segments d'un JPEG jusqu'au début des données (SOS). Comme
/// le décodeur, saute les octets parasites jusqu'au marqueur suivant.
/// `None` si aucune trame n'est trouvée.
fn en_tete_jpeg(octets: &[u8]) -> Option<EnTeteJpeg> {
    let mut composantes = None;
    let mut profil = false;
    let mut i = 2;
    loop {
        while *octets.get(i)? != 0xFF {
            i += 1;
        }
        while *octets.get(i + 1)? == 0xFF {
            i += 1;
        }
        let marqueur = octets[i + 1];
        if marqueur == 0x00 || marqueur == 0x01 || (0xD0..=0xD7).contains(&marqueur) {
            i += 2;
            continue;
        }
        if marqueur == 0xDA || marqueur == 0xD9 {
            break;
        }
        let longueur = usize::from(u16::from_be_bytes([
            *octets.get(i + 2)?,
            *octets.get(i + 3)?,
        ]));
        let donnees = octets.get(i + 4..i + 2 + longueur.max(2))?;
        let trame = (0xC0..=0xCF).contains(&marqueur) && ![0xC4, 0xC8, 0xCC].contains(&marqueur);
        if trame && composantes.is_none() {
            composantes = Some(*donnees.get(5)?);
        }
        if marqueur == 0xE2 && donnees.starts_with(b"ICC_PROFILE\0") {
            profil = true;
        }
        i += 2 + longueur;
    }
    Some(EnTeteJpeg {
        composantes: composantes?,
        profil,
    })
}

/// Une particularité que le décodeur ne prend pas en charge (TIFF à palette
/// ou YCbCr…) n'est pas une image abîmée.
fn refus(erreur: ImageError) -> RefusPhoto {
    match erreur {
        ImageError::Unsupported(_) => RefusPhoto::FormatNonPrisEnCharge,
        autre => illisible(autre),
    }
}

fn illisible(erreur: impl std::fmt::Display) -> RefusPhoto {
    RefusPhoto::Illisible(erreur.to_string())
}
