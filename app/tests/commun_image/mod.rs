//! Banc d'essai d'une image comparée (repris de `comparer_image.rs`) :
//! bibliothèque temporaire, profils d'essai de la crate `gamut`, image PNG
//! construite en mémoire.

#![allow(dead_code)]

use std::io::Cursor;
use std::path::Path;

use app::colonne::{emplacement, BibliothequeOuverte};
use app::comparer::image::{Analyses, ImageOuverte};
use app::comparer::{FichiersOuverts, SourceComparaison};
use bibliotheque::IdCondition;
use gamut::ProfilEssai;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use tempfile::TempDir;

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
pub fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Un gamut réduit : la chroma est divisée par deux, les couleurs vives
/// sortent du gamut.
pub fn etroit(rvb: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lineaire(rvb);
    [l, a / 2.0, b / 2.0]
}

/// Inverse de `etroit`, borné au cube RVB.
pub fn etroit_inverse(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let (a, b) = (a * 2.0, b * 2.0);
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

pub fn essai(description: &str) -> Vec<u8> {
    gamut::profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible: etroit,
        inverse: Some(etroit_inverse),
        description: description.into(),
        mesures: None,
    })
}

/// PNG RVB de `largeur` × `hauteur` pixels aux couleurs variées.
pub fn png(largeur: u32, hauteur: u32) -> Vec<u8> {
    let pixels: Vec<u8> = (0..largeur * hauteur)
        .flat_map(|i| {
            let v = (i * 37 % 256) as u8;
            [v, 255 - v, v / 2]
        })
        .collect();
    let mut octets = Vec::new();
    PngEncoder::new(Cursor::new(&mut octets))
        .write_image(&pixels, largeur, hauteur, ExtendedColorType::Rgb8)
        .unwrap();
    octets
}

pub struct Banc {
    pub dossier: TempDir,
    pub ouverte: BibliothequeOuverte,
    pub fichiers: FichiersOuverts,
    pub canon: IdCondition,
    pub image: ImageOuverte,
}

pub fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let canon = ouverte
        .ajouter_papier(canon.id, "Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        dossier,
        ouverte,
        fichiers: FichiersOuverts::default(),
        canon,
        image: ImageOuverte::default(),
    }
}

impl Banc {
    pub fn ranger(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, icc).unwrap();
        let id = self
            .ouverte
            .ranger_profil(self.canon, Path::new(&chemin), "2026-10-28")
            .unwrap();
        std::fs::remove_file(&chemin).unwrap();
        SourceComparaison::Bibliotheque { profil: id.0 }
    }

    pub fn ouvrir_fichier(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, icc).unwrap();
        let resume = self.fichiers.ouvrir(&chemin).unwrap();
        std::fs::remove_file(&chemin).unwrap();
        SourceComparaison::Fichier {
            jeton: resume.jeton,
        }
    }

    pub fn analyser(&self, sources: &[SourceComparaison]) -> Analyses {
        self.ouverte
            .analyser_image(&self.image, sources, &self.fichiers, gamut::Rendu::Relatif)
            .unwrap()
    }
}
