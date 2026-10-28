//! Dessin d'une page MYIRO-1 (plages, texte lisible, QR code) et écriture
//! en TIFF RVB sans profil ICC.

use std::io::Cursor;

use mires::{
    contenu_qr, couper_ligne, ecrire_tiff, lire_qr, mettre_en_page, rendre_page, CodeTirage,
    Etiquette, Format, Gabarit, ImageRvb, MiseEnPage, RESOLUTION_PPP,
};
use tiff::decoder::ifd::Value;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

fn grille(pas: usize) -> Vec<[f64; 3]> {
    let niveau = |i: usize| i as f64 * 100.0 / (pas - 1) as f64;
    let mut v = Vec::new();
    for r in 0..pas {
        for g in 0..pas {
            for b in 0..pas {
                v.push([niveau(r), niveau(g), niveau(b)]);
            }
        }
    }
    v
}

fn code() -> CodeTirage {
    CodeTirage {
        tirage: 42,
        page: 2,
        pages: 4,
        empreinte: "1A2B3C4D".into(),
    }
}

fn etiquette(lignes: &[&str]) -> Etiquette {
    Etiquette {
        lignes: lignes.iter().map(|l| l.to_string()).collect(),
        code: contenu_qr(&code()),
    }
}

fn mise_a4() -> (MiseEnPage, Vec<[f64; 3]>) {
    let v = grille(6);
    (
        mettre_en_page(&v, &Gabarit::par_defaut(Format::A4), 5).unwrap(),
        v,
    )
}

fn page(lignes: &[&str]) -> ImageRvb {
    let (m, v) = mise_a4();
    rendre_page(&m, &v, 1, &etiquette(lignes)).unwrap()
}

fn pixel(image: &ImageRvb, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * image.largeur + x) * 3) as usize;
    [image.pixels[i], image.pixels[i + 1], image.pixels[i + 2]]
}

fn px(mm: f64) -> u32 {
    (mm * 300.0 / 25.4).round() as u32
}

#[test]
fn une_page_myiro1_a4_paysage_fait_3508_par_2480_pixels() {
    let image = page(&["Essai"]);
    assert_eq!((image.largeur, image.hauteur), (3508, 2480));
    assert_eq!(image.pixels.len(), 3508 * 2480 * 3);
    assert_eq!(RESOLUTION_PPP, 300);
}

#[test]
fn une_plage_a_la_bonne_couleur_au_bon_endroit() {
    let (m, v) = mise_a4();
    let image = rendre_page(&m, &v, 1, &etiquette(&["Essai"])).unwrap();
    for plage in &m.pages[0].plages[..40] {
        let (x, y) = m.gabarit.origine_plage(plage.rangee, plage.colonne);
        let centre = (px(x + 4.0), px(y + 4.0));
        let attendu = v[plage.index].map(|c| (c * 2.55).round() as u8);
        assert_eq!(pixel(&image, centre.0, centre.1), attendu, "{plage:?}");
    }
}

#[test]
fn deux_plages_voisines_se_touchent_sans_trou() {
    let (m, v) = mise_a4();
    let image = rendre_page(&m, &v, 1, &etiquette(&["Essai"])).unwrap();
    let (x, y) = m.gabarit.origine_plage(0, 1);
    let bord = px(x);
    let milieu = px(y + 4.0);
    let gauche = &m.pages[0].plages[0];
    let droite = &m.pages[0].plages[1];
    assert_eq!(
        pixel(&image, bord - 1, milieu),
        v[gauche.index].map(|c| (c * 2.55).round() as u8)
    );
    assert_eq!(
        pixel(&image, bord, milieu),
        v[droite.index].map(|c| (c * 2.55).round() as u8)
    );
}

#[test]
fn le_tiff_se_relit_et_n_a_pas_de_profil() {
    let image = page(&["Essai"]);
    let mut octets = Vec::new();
    ecrire_tiff(&image, &mut octets).unwrap();
    // PackBits réduit nettement une page faite d'aplats.
    assert!(octets.len() < image.pixels.len() / 4, "{}", octets.len());
    let mut d = Decoder::new(Cursor::new(&octets)).unwrap();
    assert_eq!(d.dimensions().unwrap(), (3508, 2480));
    assert_eq!(d.get_tag_u32(Tag::Compression).unwrap(), 32773);
    assert_eq!(d.get_tag_u32(Tag::ResolutionUnit).unwrap(), 2);
    for tag in [Tag::XResolution, Tag::YResolution] {
        assert_eq!(d.get_tag(tag).unwrap(), Value::Rational(300, 1));
    }
    assert_eq!(
        d.get_tag_ascii_string(Tag::Software).unwrap(),
        "myiro-libre"
    );
    assert!(d.find_tag(Tag::Unknown(34675)).unwrap().is_none());
    match d.read_image().unwrap() {
        DecodingResult::U8(pixels) => assert!(pixels == image.pixels),
        _ => panic!("pixels 8 bits attendus"),
    }
}

#[test]
fn le_qr_d_une_page_se_relit() {
    let image = page(&["Essai"]);
    // Le QR code est en haut à droite : on le lit dans ce coin.
    let (x0, hauteur) = (image.largeur - 700, 400);
    let mut prepare = rqrr::PreparedImage::prepare_from_greyscale(700, hauteur, |x, y| {
        pixel(&image, x0 + x as u32, y as u32)[0]
    });
    let grilles = prepare.detect_grids();
    assert_eq!(grilles.len(), 1);
    let (_, texte) = grilles[0].decode().unwrap();
    assert_eq!(texte, "MYIRO-LIBRE:T42:P2/4:1A2B3C4D");
    assert_eq!(lire_qr(&texte), Some(code()));
}

#[test]
fn contenu_qr_et_lire_qr_font_l_aller_retour() {
    let code = CodeTirage {
        tirage: 7,
        page: 1,
        pages: 1,
        empreinte: "0123abcdef99".into(),
    };
    let texte = contenu_qr(&code);
    assert_eq!(texte, "MYIRO-LIBRE:T7:P1/1:0123ABCD");
    assert_eq!(
        lire_qr(&texte),
        Some(CodeTirage {
            empreinte: "0123ABCD".into(),
            ..code
        })
    );
}

#[test]
fn un_texte_qui_n_est_pas_a_nous_n_est_pas_lu() {
    assert_eq!(lire_qr("https://exemple.org"), None);
    assert_eq!(lire_qr("MYIRO-LIBRE:T7:P3/2:0123ABCD"), None);
    assert_eq!(lire_qr("MYIRO-LIBRE:Tx:P1/2:0123ABCD"), None);
    assert_eq!(lire_qr("MYIRO-LIBRE:T7:P1/2:0123"), None);
}

#[test]
fn une_ligne_trop_longue_est_coupee_par_des_points() {
    assert_eq!(
        couper_ligne("Hahnemühle Photo Rag 308", 30),
        "Hahnemühle Photo Rag 308"
    );
    assert_eq!(couper_ligne("Hahnemühle Photo Rag 308", 12), "Hahnemühle …");
    assert_eq!(couper_ligne("Hahnemühle", 10), "Hahnemühle");
    assert_eq!(couper_ligne("Hahnemühle", 0), "");
}

#[test]
fn le_texte_reste_dans_sa_zone() {
    let longue = "Canson « Baryta » / 2 · ".repeat(20);
    let (m, v) = mise_a4();
    let image = rendre_page(
        &m,
        &v,
        1,
        &etiquette(&[&longue, &longue, "Imprimer à 100 %"]),
    )
    .unwrap();
    let g = &m.gabarit;
    let zone = g.zone_texte();
    let dans = |x: u32, y: u32, gx: f64, gy: f64, l: f64, h: f64| {
        x >= px(gx) && x < px(gx + l) && y >= px(gy) && y < px(gy + h)
    };
    let plages: Vec<(f64, f64)> = m.pages[0]
        .plages
        .iter()
        .map(|p| g.origine_plage(p.rangee, p.colonne))
        .collect();
    let mut texte = 0;
    for y in 0..image.hauteur {
        for x in 0..image.largeur {
            if pixel(&image, x, y) == [255; 3] {
                continue;
            }
            if dans(x, y, zone.x, zone.y, zone.largeur, zone.hauteur) {
                texte += 1;
                continue;
            }
            assert!(
                plages
                    .iter()
                    .any(|&(px0, py0)| dans(x, y, px0, py0, g.cote_mm, g.cote_mm)),
                "pixel ({x}, {y}) hors des plages et de la zone de texte"
            );
        }
    }
    assert!(texte > 0);
}

#[test]
fn les_accents_sont_dessines() {
    assert_ne!(page(&["é"]).pixels, page(&["e"]).pixels);
    assert_ne!(page(&["à"]).pixels, page(&["a"]).pixels);
}

#[test]
fn une_page_inconnue_est_refusee() {
    let (m, v) = mise_a4();
    assert!(rendre_page(&m, &v, 2, &etiquette(&["Essai"])).is_err());
}
