//! La pipette de la section « Image » (ticket #80) : un clic sur l'image
//! montre la couleur du pixel et sa couleur imprimée par chaque profil.
//! Sans Tauri ; images d'essai en mémoire, jamais écrites.

use std::io::Cursor;
use std::path::Path;

use app::colonne::{emplacement, BibliothequeOuverte};
use app::comparer::image::{pixel_affiche, EtatPixel, ImageOuverte, PointAffiche};
use app::comparer::{FichiersOuverts, SourceComparaison};
use gamut::{ProfilEssai, ProfilIcc, ProfilSource};
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use tempfile::TempDir;

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Inverse exact de `lineaire`, borné au cube RVB.
fn lineaire_inverse(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

fn essai(inverse: Option<fn([f64; 3]) -> [f64; 3]>) -> Vec<u8> {
    gamut::profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible: lineaire,
        inverse,
        description: "Essai".into(),
        mesures: None,
    })
}

/// PNG RVBA de 5 × 3 pixels : le pixel i vaut (37·i mod 256, 255 − ·, · / 2),
/// et le dernier est transparent.
fn png() -> Vec<u8> {
    let pixels: Vec<u8> = (0..15u32)
        .flat_map(|i| {
            let v = (i * 37 % 256) as u8;
            [v, 255 - v, v / 2, if i == 14 { 0 } else { 255 }]
        })
        .collect();
    let mut octets = Vec::new();
    PngEncoder::new(Cursor::new(&mut octets))
        .write_image(&pixels, 5, 3, ExtendedColorType::Rgba8)
        .unwrap();
    octets
}

struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    fichiers: FichiersOuverts,
    image: ImageOuverte,
}

fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    Banc {
        dossier,
        ouverte,
        fichiers: FichiersOuverts::default(),
        image: ImageOuverte::default(),
    }
}

impl Banc {
    fn ouvrir_profil(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, icc).unwrap();
        let resume = self.fichiers.ouvrir(Path::new(&chemin)).unwrap();
        std::fs::remove_file(&chemin).unwrap();
        SourceComparaison::Fichier {
            jeton: resume.jeton,
        }
    }

    /// Clic au point (x, y) d'une image affichée en 500 × 300.
    fn cliquer(
        &self,
        sources: &[SourceComparaison],
        x: f64,
        y: f64,
    ) -> Result<app::comparer::image::PixelImage, String> {
        let point = PointAffiche {
            x,
            y,
            largeur: 500.0,
            hauteur: 300.0,
        };
        self.ouverte
            .couleur_pixel(&self.image, sources, &self.fichiers, point)
    }
}

fn point(x: f64, y: f64, largeur: f64, hauteur: f64) -> PointAffiche {
    PointAffiche {
        x,
        y,
        largeur,
        hauteur,
    }
}

#[test]
fn un_clic_est_ramene_au_pixel_de_l_image_d_analyse() {
    // Image d'analyse de 5 × 3 affichée en 500 × 300 : 100 points par pixel.
    assert_eq!(
        pixel_affiche(point(0.0, 0.0, 500.0, 300.0), 5, 3),
        Some((0, 0))
    );
    assert_eq!(
        pixel_affiche(point(250.0, 150.0, 500.0, 300.0), 5, 3),
        Some((2, 1))
    );
    assert_eq!(
        pixel_affiche(point(499.9, 299.9, 500.0, 300.0), 5, 3),
        Some((4, 2))
    );
    assert_eq!(
        pixel_affiche(point(199.9, 100.0, 500.0, 300.0), 5, 3),
        Some((1, 1))
    );
    // Affichée plus petite que l'image d'analyse.
    assert_eq!(pixel_affiche(point(1.5, 0.2, 2.5, 1.5), 5, 3), Some((3, 0)));
    // Hors de l'image, taille d'affichage nulle ou valeur non finie : rien.
    for p in [
        point(500.0, 10.0, 500.0, 300.0),
        point(10.0, 300.0, 500.0, 300.0),
        point(-0.1, 10.0, 500.0, 300.0),
        point(10.0, 10.0, 0.0, 300.0),
        point(f64::NAN, 10.0, 500.0, 300.0),
        point(10.0, 10.0, 500.0, f64::INFINITY),
    ] {
        assert_eq!(pixel_affiche(p, 5, 3), None, "{p:?}");
    }
    assert_eq!(pixel_affiche(point(0.0, 0.0, 500.0, 300.0), 0, 3), None);
}

#[test]
fn un_pixel_clique_a_la_paire_de_chaque_profil() {
    let b = banc();
    let avec = b.ouvrir_profil("avec.icc", &essai(Some(lineaire_inverse)));
    let sans = b.ouvrir_profil("sans.icc", &essai(None));
    b.image.ouvrir_octets("photo.png", &png()).unwrap();
    // Pixel (1, 0) : v = 37.
    let pixel = b.cliquer(&[avec, sans], 150.0, 50.0).unwrap();
    assert_eq!((pixel.x, pixel.y), (1, 0));
    assert_eq!(pixel.rvb, [37, 218, 18]);
    assert_eq!(pixel.profils.len(), 2);
    let sortie = ProfilIcc::lire(&essai(Some(lineaire_inverse))).unwrap();
    let attendue = gamut::paire_couleur([37, 218, 18], &ProfilSource::srvb(), &sortie)
        .unwrap()
        .unwrap();
    match &pixel.profils[0] {
        EtatPixel::Calcule { paire } => assert_eq!(*paire, attendue),
        autre => panic!("profil avec B2A1 non calculé : {autre:?}"),
    }
    match &pixel.profils[1] {
        EtatPixel::Inconnu { cle, .. } => assert_eq!(*cle, "comparer.refus.sans_table_inverse"),
        autre => panic!("profil sans B2A1 calculé : {autre:?}"),
    }
    // Même code d'écart que le pixel de la surimpression.
    let analyses = b
        .ouverte
        .analyser_image(&b.image, &[avec], &b.fichiers)
        .unwrap();
    assert_eq!(
        b.image.ecarts(analyses.numero, 0).unwrap()[1],
        attendue.code
    );
}

#[test]
fn un_clic_hors_de_l_image_ou_sur_un_pixel_transparent_est_refuse() {
    let b = banc();
    let avec = b.ouvrir_profil("avec.icc", &essai(Some(lineaire_inverse)));
    assert_eq!(
        b.cliquer(&[avec], 10.0, 10.0).unwrap_err(),
        "image.refus.aucune"
    );
    b.image.ouvrir_octets("photo.png", &png()).unwrap();
    assert_eq!(
        b.cliquer(&[avec], 600.0, 10.0).unwrap_err(),
        "image.pixel.hors_image"
    );
    // Dernier pixel (4, 2), transparent.
    assert_eq!(
        b.cliquer(&[avec], 450.0, 250.0).unwrap_err(),
        "image.pixel.transparent"
    );
}

#[test]
fn les_refus_de_la_pipette_sont_au_catalogue() {
    use app::textes::{texte, Langue};
    for cle in [
        "image.pixel.hors_image",
        "image.pixel.transparent",
        "image.pixel.inconnu",
    ] {
        assert!(app::comparer::image::CLES_IMAGE.contains(&cle), "{cle}");
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} manque");
        }
    }
}
