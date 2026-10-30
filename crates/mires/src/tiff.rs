//! Écriture d'une page en TIFF RVB 8 bits, sans profil ICC ni autre
//! étiquette de couleur : la page s'imprime sans gestion des couleurs.
//!
//! TIFF 6.0 de base : petit-boutiste, un seul IFD, une seule bande d'image
//! compressée en PackBits (section 9), ligne par ligne.

use std::io::Write;

use crate::rendu::{ImageRvb, RESOLUTION_PPP};
use crate::ErreurMire;

/// Nombre d'entrées de l'IFD.
const ENTREES: u16 = 14;
/// Nom écrit dans l'étiquette `Software`, terminé par un octet nul.
const LOGICIEL: &[u8] = b"myiro-libre\0";

/// Écrit l'image en TIFF PackBits, avec sa résolution (300 ppp).
pub fn ecrire_tiff(image: &ImageRvb, sortie: &mut impl Write) -> Result<(), ErreurMire> {
    let (largeur, hauteur) = (image.largeur as usize, image.hauteur as usize);
    if largeur == 0 || hauteur == 0 || image.pixels.len() != largeur * hauteur * 3 {
        return Err(ErreurMire::Ecriture(
            "image incomplète, rien n'est écrit".to_string(),
        ));
    }
    let mut donnees = Vec::new();
    for ligne in image.pixels.chunks(largeur * 3) {
        packbits(ligne, &mut donnees);
    }

    // Disposition : en-tête (8), IFD, valeurs longues, puis l'image.
    let ifd = 8u32;
    let bits = ifd + 2 + 12 * ENTREES as u32 + 4;
    let resolution_x = bits + 6;
    let resolution_y = resolution_x + 8;
    let logiciel = resolution_y + 8;
    let image_debut = logiciel + LOGICIEL.len() as u32;
    let taille = u32::try_from(donnees.len())
        .ok()
        .filter(|t| t.checked_add(image_debut).is_some())
        .ok_or_else(|| ErreurMire::Ecriture("image trop grande pour un TIFF".to_string()))?;

    let mut o = Vec::with_capacity(image_debut as usize + donnees.len());
    o.extend(b"II*\0");
    o.extend(ifd.to_le_bytes());
    o.extend(ENTREES.to_le_bytes());
    const SHORT: u16 = 3;
    const LONG: u16 = 4;
    const ASCII: u16 = 2;
    const RATIONNEL: u16 = 5;
    let mut entree = |tag: u16, genre: u16, nombre: u32, valeur: u32| {
        o.extend(tag.to_le_bytes());
        o.extend(genre.to_le_bytes());
        o.extend(nombre.to_le_bytes());
        o.extend(valeur.to_le_bytes());
    };
    entree(256, LONG, 1, image.largeur); // ImageWidth
    entree(257, LONG, 1, image.hauteur); // ImageLength
    entree(258, SHORT, 3, bits); // BitsPerSample : 8, 8, 8
    entree(259, SHORT, 1, 32773); // Compression : PackBits
    entree(262, SHORT, 1, 2); // PhotometricInterpretation : RVB
    entree(273, LONG, 1, image_debut); // StripOffsets
    entree(277, SHORT, 1, 3); // SamplesPerPixel
    entree(278, LONG, 1, image.hauteur); // RowsPerStrip : une seule bande
    entree(279, LONG, 1, taille); // StripByteCounts
    entree(282, RATIONNEL, 1, resolution_x); // XResolution
    entree(283, RATIONNEL, 1, resolution_y); // YResolution
    entree(284, SHORT, 1, 1); // PlanarConfiguration : entrelacé
    entree(296, SHORT, 1, 2); // ResolutionUnit : pouce
    entree(305, ASCII, LOGICIEL.len() as u32, logiciel); // Software
    o.extend(0u32.to_le_bytes()); // pas d'autre IFD
    for _ in 0..3 {
        o.extend(8u16.to_le_bytes());
    }
    for _ in 0..2 {
        o.extend(RESOLUTION_PPP.to_le_bytes());
        o.extend(1u32.to_le_bytes());
    }
    o.extend(LOGICIEL);
    o.extend(donnees);
    sortie
        .write_all(&o)
        .and_then(|()| sortie.flush())
        .map_err(|e| ErreurMire::Ecriture(e.to_string()))
}

/// PackBits d'une ligne : suites d'au moins 3 octets égaux codées en
/// répétition, le reste en littéraux, par paquets de 128 au plus.
fn packbits(ligne: &[u8], sortie: &mut Vec<u8>) {
    let n = ligne.len();
    let mut i = 0;
    while i < n {
        let mut suite = 1;
        while i + suite < n && suite < 128 && ligne[i + suite] == ligne[i] {
            suite += 1;
        }
        if suite >= 3 {
            sortie.push((1 - suite as i16) as i8 as u8);
            sortie.push(ligne[i]);
            i += suite;
            continue;
        }
        let debut = i;
        while i < n && i - debut < 128 {
            if i + 2 < n && ligne[i] == ligne[i + 1] && ligne[i] == ligne[i + 2] {
                break;
            }
            i += 1;
        }
        sortie.push((i - debut - 1) as u8);
        sortie.extend(&ligne[debut..i]);
    }
}

#[cfg(test)]
mod tests {
    use super::packbits;

    /// Exemple de la spécification TIFF 6.0, section 9.
    #[test]
    fn packbits_suit_l_exemple_de_la_specification() {
        let ligne = [
            0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0xAA, 0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x22,
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
        ];
        let mut sortie = Vec::new();
        packbits(&ligne, &mut sortie);
        assert_eq!(
            sortie,
            [
                0xFE, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0xFD, 0xAA, 0x03, 0x80, 0x00, 0x2A, 0x22, 0xF7,
                0xAA
            ]
        );
    }

    #[test]
    fn une_longue_suite_se_coupe_par_128() {
        let mut sortie = Vec::new();
        packbits(&[7; 300], &mut sortie);
        assert_eq!(sortie, [0x81, 7, 0x81, 7, 0xD5, 7]);
    }
}
