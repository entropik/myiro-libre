//! Mire FD-9 RGB-2520, reprise telle quelle : empreinte, forme du TIFF et
//! valeurs au centre des plages. Les tests courants travaillent sur un petit
//! TIFF fictif de même forme ; la vraie mire, locale et non versionnée, est
//! lue par un test ignoré :
//! `cargo test -p mires --test rgb2520 -- --ignored --nocapture`
//! (variable `MIRE_FD9`, sinon `mire-fd9/` du dépôt ; variable facultative
//! `MESURE_REELLE` : export CGATS du 26 octobre 2026).

use std::io::Cursor;
use std::path::{Path, PathBuf};

use mires::{
    lire_mire_fd9, lire_mire_fd9_sans_empreinte, ErreurMire, GeometrieFd9, RGB2520_EMPREINTE,
    RGB2520_FICHIER, RGB2520_GEOMETRIE,
};
use sha2::{Digest, Sha256};
use tiff::encoder::{colortype, TiffEncoder};

/// Couleur fictive de la plage de rang `k` (à partir de 0).
fn couleur(k: usize) -> [u8; 3] {
    [
        (k * 10 % 256) as u8,
        (255 - k * 5) as u8,
        (k * 37 % 256) as u8,
    ]
}

/// TIFF RVB 8 bits de 6 × 4 plages de 5 pixels, origine (3, 3), fond blanc.
fn tiff_fictif() -> (Vec<u8>, GeometrieFd9) {
    let g = GeometrieFd9 {
        largeur_px: 36,
        hauteur_px: 26,
        origine_px: (3, 3),
        pas_px: 5,
        colonnes: 6,
        rangees: 4,
    };
    (ecrire_tiff(&g, |x, y| pixel_fictif(&g, x, y)), g)
}

fn pixel_fictif(g: &GeometrieFd9, x: u32, y: u32) -> [u8; 3] {
    let (x0, y0) = g.origine_px;
    let dans = |v: u32, debut: u32, n: usize| v >= debut && v < debut + g.pas_px * n as u32;
    if dans(x, x0, g.colonnes) && dans(y, y0, g.rangees) {
        let colonne = ((x - x0) / g.pas_px) as usize;
        let rangee = ((y - y0) / g.pas_px) as usize;
        couleur(rangee * g.colonnes + colonne)
    } else {
        [255; 3]
    }
}

fn ecrire_tiff(g: &GeometrieFd9, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..g.hauteur_px {
        for x in 0..g.largeur_px {
            pixels.extend(pixel(x, y));
        }
    }
    let mut octets = Cursor::new(Vec::new());
    TiffEncoder::new(&mut octets)
        .unwrap()
        .write_image::<colortype::RGB8>(g.largeur_px, g.hauteur_px, &pixels)
        .unwrap();
    octets.into_inner()
}

#[test]
fn les_valeurs_d_une_mire_fictive_se_lisent_dans_l_ordre_des_plages() {
    let (octets, g) = tiff_fictif();
    let mire = lire_mire_fd9_sans_empreinte(octets.clone(), &g).unwrap();
    assert_eq!(mire.octets, octets);
    assert_eq!(mire.geometrie, g);
    assert_eq!(mire.valeurs.len(), 24);
    for (k, v) in mire.valeurs.iter().enumerate() {
        let attendu = couleur(k).map(|o| o as f64 / 2.55);
        for c in 0..3 {
            assert!((v[c] - attendu[c]).abs() < 1e-9, "plage {k} : {v:?}");
        }
    }
    assert_eq!(mire.valeurs[0], [0.0, 100.0, 0.0]);
}

#[test]
fn une_mire_fd9_d_une_autre_empreinte_est_refusee() {
    let (octets, _) = tiff_fictif();
    let empreinte = format!("{:x}", Sha256::digest(&octets));
    assert_eq!(
        lire_mire_fd9(octets),
        Err(ErreurMire::MireFd9Autre { empreinte })
    );
}

#[test]
fn des_dimensions_differentes_sont_refusees() {
    let (octets, g) = tiff_fictif();
    let autre = GeometrieFd9 {
        largeur_px: 40,
        ..g
    };
    assert!(matches!(
        lire_mire_fd9_sans_empreinte(octets, &autre),
        Err(ErreurMire::MireFd9Illisible(_))
    ));
}

#[test]
fn une_zone_sans_blanc_autour_est_refusee() {
    // Même image, mais la zone de plages est décalée : le bord n'est plus blanc.
    let (_, g) = tiff_fictif();
    let octets = ecrire_tiff(&g, |x, y| {
        if x < 2 || y < 2 {
            [0, 0, 0]
        } else {
            pixel_fictif(&g, x, y)
        }
    });
    assert!(matches!(
        lire_mire_fd9_sans_empreinte(octets, &g),
        Err(ErreurMire::MireFd9Illisible(_))
    ));
}

#[test]
fn un_fichier_qui_n_est_pas_un_tiff_est_refuse() {
    let (_, g) = tiff_fictif();
    assert!(matches!(
        lire_mire_fd9_sans_empreinte(b"pas un tiff".to_vec(), &g),
        Err(ErreurMire::MireFd9Illisible(_))
    ));
}

#[test]
fn la_geometrie_de_rgb2520_donne_2520_plages() {
    let g = RGB2520_GEOMETRIE;
    assert_eq!(g.colonnes * g.rangees, 2520);
    assert_eq!((g.largeur_px, g.hauteur_px), (3300, 4961));
}

fn chemin_mire() -> PathBuf {
    if let Some(c) = std::env::var_os("MIRE_FD9") {
        return PathBuf::from(c);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|d| d.join("mire-fd9").join(RGB2520_FICHIER))
        .find(|c| c.is_file())
        .expect("mire RGB-2520 introuvable : variable MIRE_FD9 ou dossier mire-fd9/ du dépôt")
}

#[test]
#[ignore = "demande la mire RGB-2520, locale et non versionnée"]
fn la_vraie_mire_rgb2520_se_lit() {
    let octets = std::fs::read(chemin_mire()).unwrap();
    let mire = lire_mire_fd9(octets).unwrap();
    assert_eq!(mire.empreinte, RGB2520_EMPREINTE);
    assert_eq!(mire.valeurs.len(), 2520);
    let premiere = [85.0, 255.0, 127.0].map(|o: f64| o / 2.55);
    assert_eq!(mire.valeurs[0], premiere);
    println!("plage 1 : {:?}", mire.valeurs[0]);

    let Some(export) = std::env::var_os("MESURE_REELLE") else {
        println!("MESURE_REELLE absente : comparaison avec l'export sautée");
        return;
    };
    let texte = cgats::decoder(&std::fs::read(export).unwrap());
    let mesure = cgats::lire(&texte).unwrap();
    assert_eq!(mesure.plages.len(), 2520);
    let mut differentes = 0;
    for (k, (plage, valeur)) in mesure.plages.iter().zip(&mire.valeurs).enumerate() {
        let rvb = match plage.rvb {
            pont_protocole::Info::Confirmee(v) | pont_protocole::Info::Supposee(v) => v,
            pont_protocole::Info::Inconnue => panic!("plage {} sans RVB", k + 1),
        };
        let ecart = (0..3)
            .map(|c| (rvb[c] as f64 * 255.0 - valeur[c] * 2.55).abs())
            .fold(0.0, f64::max);
        assert!(ecart <= 1.0 + 1e-3, "plage {} : écart {ecart}", k + 1);
        if ecart > 1e-3 {
            differentes += 1;
        }
    }
    println!("{differentes} plages à 1/255 près");
    assert!(differentes <= 8);
}
