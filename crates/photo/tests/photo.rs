//! Ouvrir une photo : formats, profil embarqué, CMJN, orientation, image
//! d'analyse. Toutes les images d'essai sont fabriquées ici, en mémoire.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use photo::{ouvrir, RefusPhoto};
use tiff::encoder::{colortype, TiffEncoder};
use tiff::tags::Tag;

/// Un faux profil ICC : seul l'en-tête compte (taille, espace des données).
fn profil(espace: &[u8; 4]) -> Vec<u8> {
    let mut icc = vec![0u8; 132];
    icc[0..4].copy_from_slice(&132u32.to_be_bytes());
    icc[12..16].copy_from_slice(b"prtr");
    icc[16..20].copy_from_slice(espace);
    icc[20..24].copy_from_slice(b"Lab ");
    icc[36..40].copy_from_slice(b"acsp");
    icc
}

/// Pixels RVB d'une image unie.
fn unie(largeur: u32, hauteur: u32, rvb: [u8; 3]) -> Vec<u8> {
    rvb.repeat((largeur * hauteur) as usize)
}

fn jpeg(
    pixels: &[u8],
    largeur: u32,
    hauteur: u32,
    icc: Option<Vec<u8>>,
    exif: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut sortie = Vec::new();
    let mut encodeur = JpegEncoder::new_with_quality(&mut sortie, 100);
    if let Some(icc) = icc {
        encodeur.set_icc_profile(icc).expect("profil accepté");
    }
    if let Some(exif) = exif {
        encodeur.set_exif_metadata(exif).expect("exif accepté");
    }
    encodeur
        .write_image(pixels, largeur, hauteur, ExtendedColorType::Rgb8)
        .expect("jpeg encodé");
    sortie
}

#[test]
fn un_jpeg_rvb_avec_profil_embarque_rend_son_profil() {
    let icc = profil(b"RGB ");
    let octets = jpeg(&unie(16, 8, [200, 40, 40]), 16, 8, Some(icc.clone()), None);

    let photo = ouvrir(&octets).expect("jpeg ouvert");

    assert_eq!(photo.icc, Some(icc));
    assert_eq!((photo.largeur, photo.hauteur), (16, 8));
    assert_eq!(photo.rvba.len(), 16 * 8);
    assert!(!photo.gris);
    assert!(!photo.seize_bits);
}

fn png(pixels: &[u8], largeur: u32, hauteur: u32, icc: Option<Vec<u8>>) -> Vec<u8> {
    let mut sortie = Vec::new();
    let mut encodeur = PngEncoder::new(&mut sortie);
    if let Some(icc) = icc {
        encodeur.set_icc_profile(icc).expect("profil accepté");
    }
    encodeur
        .write_image(pixels, largeur, hauteur, ExtendedColorType::Rgb8)
        .expect("png encodé");
    sortie
}

/// TIFF écrit par la crate `tiff` elle-même, profil dans la balise 34675.
fn tiff_rvb8(pixels: &[u8], largeur: u32, hauteur: u32, icc: Option<&[u8]>) -> Vec<u8> {
    let mut sortie = Cursor::new(Vec::new());
    let mut encodeur = TiffEncoder::new(&mut sortie).expect("tiff ouvert");
    let mut image = encodeur
        .new_image::<colortype::RGB8>(largeur, hauteur)
        .expect("image tiff");
    if let Some(icc) = icc {
        image
            .encoder()
            .write_tag(Tag::IccProfile, icc)
            .expect("balise 34675");
    }
    image.write_data(pixels).expect("tiff écrit");
    sortie.into_inner()
}

#[test]
fn un_png_avec_iccp_rend_son_profil() {
    let icc = profil(b"RGB ");
    let octets = png(&unie(4, 3, [10, 200, 30]), 4, 3, Some(icc.clone()));

    let photo = ouvrir(&octets).expect("png ouvert");

    assert_eq!(photo.icc, Some(icc));
    assert_eq!(photo.rvba[0], [10, 200, 30, 255]);
}

#[test]
fn un_tiff_avec_balise_34675_rend_son_profil() {
    let icc = profil(b"RGB ");
    let octets = tiff_rvb8(&unie(4, 3, [10, 20, 30]), 4, 3, Some(&icc));

    let photo = ouvrir(&octets).expect("tiff ouvert");

    assert_eq!(photo.icc, Some(icc));
    assert_eq!(photo.rvba[11], [10, 20, 30, 255]);
}

#[test]
fn une_image_sans_profil_rend_none() {
    let pixels = unie(4, 3, [128, 128, 128]);
    for octets in [
        jpeg(&unie(16, 8, [128, 128, 128]), 16, 8, None, None),
        png(&pixels, 4, 3, None),
        tiff_rvb8(&pixels, 4, 3, None),
    ] {
        assert_eq!(ouvrir(&octets).expect("image ouverte").icc, None);
    }
}

/// JPEG CMJN de 8 × 8 écrit à la main, comme ceux d'une agence : quatre
/// composantes, marqueur Adobe, un bloc par composante (tout à zéro).
fn jpeg_cmjn() -> Vec<u8> {
    let mut o = vec![0xFF, 0xD8];
    o.extend([0xFF, 0xEE, 0, 14]);
    o.extend(b"Adobe");
    o.extend([0, 100, 0, 0, 0, 0, 0]);
    o.extend([0xFF, 0xDB, 0, 67, 0]);
    o.extend([1u8; 64]);
    o.extend([0xFF, 0xC0, 0, 20, 8, 0, 8, 0, 8, 4]);
    for id in 1..=4 {
        o.extend([id, 0x11, 0]);
    }
    for classe in [0x00, 0x10] {
        o.extend([0xFF, 0xC4, 0, 20, classe, 1]);
        o.extend([0u8; 15]);
        o.push(0);
    }
    o.extend([0xFF, 0xDA, 0, 14, 4]);
    for id in 1..=4 {
        o.extend([id, 0]);
    }
    o.extend([0, 63, 0]);
    o.extend([0x00, 0xFF, 0xD9]);
    o
}

fn tiff_cmjn() -> Vec<u8> {
    let mut sortie = Cursor::new(Vec::new());
    let mut encodeur = TiffEncoder::new(&mut sortie).expect("tiff ouvert");
    encodeur
        .write_image::<colortype::CMYK8>(2, 2, &[0, 255, 255, 0].repeat(4))
        .expect("tiff cmjn écrit");
    sortie.into_inner()
}

#[test]
fn une_image_cmjn_est_refusee() {
    let rvb_a_profil_cmjn = jpeg(&unie(16, 8, [9, 9, 9]), 16, 8, Some(profil(b"CMYK")), None);
    for octets in [jpeg_cmjn(), tiff_cmjn(), rvb_a_profil_cmjn] {
        assert_eq!(ouvrir(&octets).err(), Some(RefusPhoto::Cmjn));
    }
}

/// Bloc Exif (TIFF petit-boutiste) qui ne porte que la balise Orientation.
fn exif_orientation(valeur: u16) -> Vec<u8> {
    let mut o = b"II*\0".to_vec();
    o.extend(8u32.to_le_bytes());
    o.extend(1u16.to_le_bytes());
    o.extend(0x0112u16.to_le_bytes());
    o.extend(3u16.to_le_bytes());
    o.extend(1u32.to_le_bytes());
    o.extend(valeur.to_le_bytes());
    o.extend([0, 0]);
    o.extend(0u32.to_le_bytes());
    o
}

const ROUGE: [u8; 3] = [230, 20, 20];
const BLEU: [u8; 3] = [20, 20, 230];

fn proche(pixel: [u8; 4], rvb: [u8; 3]) -> bool {
    (0..3).all(|c| pixel[c].abs_diff(rvb[c]) < 40)
}

#[test]
fn l_orientation_exif_est_appliquee() {
    // Prise de vue 32 × 16, appareil tourné : seul le quart bas gauche est
    // rouge. Orientation 6 = tourner de 90° dans le sens horaire pour voir
    // l'image droite, qui fait alors 16 × 32, le rouge en haut à gauche.
    let (l, h) = (32u32, 16u32);
    let mut pixels = Vec::new();
    for y in 0..h {
        for x in 0..l {
            pixels.extend(if x < 16 && y >= 8 { ROUGE } else { BLEU });
        }
    }
    let octets = jpeg(&pixels, l, h, None, Some(exif_orientation(6)));

    let photo = ouvrir(&octets).expect("jpeg ouvert");

    assert_eq!((photo.largeur, photo.hauteur), (16, 32));
    assert_eq!((photo.largeur_origine, photo.hauteur_origine), (32, 16));
    let pixel = |x: u32, y: u32| photo.rvba[(y * photo.largeur + x) as usize];
    assert!(proche(pixel(2, 2), ROUGE), "{:?}", pixel(2, 2));
    assert!(proche(pixel(13, 2), BLEU), "{:?}", pixel(13, 2));
    assert!(proche(pixel(2, 29), BLEU), "{:?}", pixel(2, 29));
    assert!(proche(pixel(13, 29), BLEU), "{:?}", pixel(13, 29));
}

#[test]
fn une_grande_image_est_echantillonnee_sans_melange() {
    // Chaque pixel porte sa position : x dans le rouge et le vert, y dans le
    // bleu. Un mélange de voisins donnerait une valeur absente de l'original.
    let (l, h) = (5_000u32, 100u32);
    let mut pixels = Vec::with_capacity((l * h * 3) as usize);
    for y in 0..h {
        for x in 0..l {
            pixels.extend([(x % 256) as u8, (x / 256) as u8, y as u8]);
        }
    }
    let octets = png(&pixels, l, h, None);

    let photo = ouvrir(&octets).expect("png ouvert");

    assert_eq!(photo.pas, 3);
    assert_eq!((photo.largeur, photo.hauteur), (1_667, 34));
    assert_eq!(photo.rvba.len(), 1_667 * 34);
    for (i, pixel) in photo.rvba.iter().enumerate() {
        let (x, y) = (i as u32 % photo.largeur * 3, i as u32 / photo.largeur * 3);
        assert_eq!(*pixel, [(x % 256) as u8, (x / 256) as u8, y as u8, 255]);
    }
}

#[test]
fn un_tiff_16_bits_est_ramene_a_8_bits() {
    let mut sortie = Cursor::new(Vec::new());
    TiffEncoder::new(&mut sortie)
        .expect("tiff ouvert")
        .write_image::<colortype::RGB16>(2, 1, &[65_535, 0, 32_896, 0, 65_535, 0])
        .expect("tiff 16 bits écrit");

    let photo = ouvrir(&sortie.into_inner()).expect("tiff ouvert");

    assert!(photo.seize_bits);
    assert_eq!(photo.rvba, vec![[255, 0, 128, 255], [0, 255, 0, 255]]);
}

#[test]
fn une_image_en_gris_est_dite_grise() {
    let mut sortie = Vec::new();
    PngEncoder::new(&mut sortie)
        .write_image(&[0, 100, 255], 3, 1, ExtendedColorType::L8)
        .expect("png gris encodé");

    let photo = ouvrir(&sortie).expect("png ouvert");

    assert!(photo.gris);
    assert_eq!(photo.rvba[1], [100, 100, 100, 255]);
}

#[test]
fn un_fichier_qui_n_est_pas_une_image_est_illisible() {
    let tronque = jpeg(&unie(16, 8, [1, 2, 3]), 16, 8, None, None)[..40].to_vec();
    for octets in [b"CGATS.17\nORIGINATOR".to_vec(), Vec::new(), tronque] {
        match ouvrir(&octets) {
            Err(RefusPhoto::Illisible(cause)) => assert!(!cause.is_empty()),
            autre => panic!("attendu Illisible, obtenu {:?}", autre.err()),
        }
    }
}

#[test]
fn un_format_gif_est_refuse() {
    let mut gif = b"GIF89a".to_vec();
    gif.extend([1, 0, 1, 0, 0, 0, 0, 0x3B]);

    assert_eq!(ouvrir(&gif).err(), Some(RefusPhoto::FormatNonPrisEnCharge));
}

/// En-tête TIFF gris de 20 000 × 10 000 pixels, sans les pixels : le refus
/// doit venir avant tout décodage.
fn tiff_annonce(largeur: u32, hauteur: u32) -> Vec<u8> {
    let entrees: [(u16, u16, u32); 8] = [
        (256, 4, largeur),
        (257, 4, hauteur),
        (258, 3, 8),
        (259, 3, 1),
        (262, 3, 1),
        (273, 4, 0),
        (278, 4, hauteur),
        (279, 4, largeur * hauteur),
    ];
    let mut o = b"II*\0".to_vec();
    o.extend(8u32.to_le_bytes());
    o.extend((entrees.len() as u16).to_le_bytes());
    for (balise, genre, valeur) in entrees {
        o.extend(balise.to_le_bytes());
        o.extend(genre.to_le_bytes());
        o.extend(1u32.to_le_bytes());
        if genre == 3 {
            o.extend((valeur as u16).to_le_bytes());
            o.extend([0, 0]);
        } else {
            o.extend(valeur.to_le_bytes());
        }
    }
    o.extend(0u32.to_le_bytes());
    o
}

#[test]
fn une_image_trop_grande_est_refusee_avant_decodage() {
    let octets = tiff_annonce(20_000, 10_000);

    assert_eq!(
        ouvrir(&octets).err(),
        Some(RefusPhoto::TropGrande {
            pixels: 200_000_000
        })
    );
}

#[test]
fn chaque_refus_a_sa_cle_de_texte() {
    let cles = [
        RefusPhoto::Illisible("cause".into()).cle(),
        RefusPhoto::FormatNonPrisEnCharge.cle(),
        RefusPhoto::Cmjn.cle(),
        RefusPhoto::TropGrande { pixels: 1 }.cle(),
    ];
    assert_eq!(
        cles,
        [
            "image.refus.illisible",
            "image.refus.format",
            "image.refus.cmjn",
            "image.refus.trop_grande",
        ]
    );
}
