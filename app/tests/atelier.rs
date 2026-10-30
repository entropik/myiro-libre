//! Réglage « Atelier » (ticket #89) : coordonnées, logo et numéro de
//! rapport, gardés à côté des données de l'application, sans Tauri.

use std::io::Cursor;

use app::atelier::{AtelierSaisi, ErreurAtelier, ReglageAtelier};
use tempfile::TempDir;

fn reglage() -> (TempDir, ReglageAtelier) {
    let dossier = tempfile::tempdir().unwrap();
    let reglage = ReglageAtelier::new(dossier.path().join("atelier"));
    (dossier, reglage)
}

fn saisi() -> AtelierSaisi {
    AtelierSaisi {
        nom: "Atelier Exemple".into(),
        adresse: "12 rue des Presses\n75011 Paris".into(),
        courriel: "contact@exemple.fr".into(),
        telephone: "01 23 45 67 89".into(),
        site: "exemple.fr".into(),
    }
}

fn png(largeur: u32, hauteur: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(largeur, hauteur, image::Rgba([200, 30, 20, 255]));
    let mut octets = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut octets), image::ImageFormat::Png)
        .unwrap();
    octets
}

#[test]
fn sans_fichier_l_atelier_est_vide() {
    let (_d, r) = reglage();
    assert_eq!(r.lire().unwrap(), rapport::Atelier::default());
    assert_eq!(r.saisi().unwrap(), AtelierSaisi::default());
    assert!(!r.a_un_logo());
}

#[test]
fn l_atelier_enregistre_se_relit_a_l_identique() {
    let (_d, r) = reglage();
    r.enregistrer(&saisi()).unwrap();
    assert_eq!(r.saisi().unwrap(), saisi());
    let a = r.lire().unwrap();
    assert_eq!(a.nom, "Atelier Exemple");
    assert_eq!(a.adresse, "12 rue des Presses\n75011 Paris");
    assert_eq!(a.logo_png, None);
    // Un second réglage sur le même dossier relit la même chose.
    let autre = ReglageAtelier::new(r.dossier().to_path_buf());
    assert_eq!(autre.saisi().unwrap(), saisi());
}

#[test]
fn un_champ_trop_long_est_refuse() {
    let (_d, r) = reglage();
    for (champ, a) in [
        (
            "nom",
            AtelierSaisi {
                nom: "x".repeat(121),
                ..saisi()
            },
        ),
        (
            "adresse",
            AtelierSaisi {
                adresse: "é".repeat(401),
                ..saisi()
            },
        ),
        (
            "courriel",
            AtelierSaisi {
                courriel: "x".repeat(201),
                ..saisi()
            },
        ),
        (
            "telephone",
            AtelierSaisi {
                telephone: "1".repeat(201),
                ..saisi()
            },
        ),
        (
            "site",
            AtelierSaisi {
                site: "x".repeat(201),
                ..saisi()
            },
        ),
    ] {
        assert_eq!(r.enregistrer(&a), Err(ErreurAtelier::TropLong(champ)));
    }
    // 400 caractères accentués passent : la borne compte des caractères.
    r.enregistrer(&AtelierSaisi {
        adresse: "é".repeat(400),
        ..saisi()
    })
    .unwrap();
    assert_eq!(
        ErreurAtelier::TropLong("nom").cle(),
        "atelier.erreur.trop_long"
    );
}

#[test]
fn un_logo_svg_est_refuse_et_un_grand_logo_reduit() {
    let (d, r) = reglage();
    let svg = d.path().join("logo.svg");
    std::fs::write(
        &svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>x</script></svg>",
    )
    .unwrap();
    assert_eq!(r.poser_logo(&svg), Err(ErreurAtelier::LogoFormat));
    let abime = d.path().join("abime.png");
    std::fs::write(&abime, &png(10, 10)[..40]).unwrap();
    assert_eq!(r.poser_logo(&abime), Err(ErreurAtelier::LogoIllisible));
    assert!(!r.a_un_logo());

    let grand = d.path().join("grand.png");
    std::fs::write(&grand, png(4000, 1000)).unwrap();
    r.poser_logo(&grand).unwrap();
    let logo = r.lire().unwrap().logo_png.unwrap();
    let relu = image::load_from_memory_with_format(&logo, image::ImageFormat::Png).unwrap();
    assert_eq!((relu.width(), relu.height()), (600, 150));

    // Un petit JPEG est gardé à sa taille, réencodé en PNG.
    let jpeg = d.path().join("petit.jpg");
    let mut octets = Vec::new();
    image::RgbImage::from_pixel(120, 80, image::Rgb([10, 20, 30]))
        .write_to(&mut Cursor::new(&mut octets), image::ImageFormat::Jpeg)
        .unwrap();
    std::fs::write(&jpeg, octets).unwrap();
    r.poser_logo(&jpeg).unwrap();
    let logo = r.lire().unwrap().logo_png.unwrap();
    assert!(logo.starts_with(b"\x89PNG"));
    let relu = image::load_from_memory(&logo).unwrap();
    assert_eq!((relu.width(), relu.height()), (120, 80));

    r.retirer_logo().unwrap();
    assert!(!r.a_un_logo());
    assert_eq!(r.lire().unwrap().logo_png, None);
    // Retirer un logo absent ne fait rien.
    r.retirer_logo().unwrap();
}

#[test]
fn le_numero_se_suit_et_repart_a_1_chaque_annee() {
    let (_d, r) = reglage();
    assert_eq!(r.prochain_numero(2026).unwrap(), "2026-0001");
    // Regarder le prochain numéro ne le prend pas.
    assert_eq!(r.prochain_numero(2026).unwrap(), "2026-0001");
    assert_eq!(r.prendre_numero(2026).unwrap(), "2026-0001");
    assert_eq!(r.prendre_numero(2026).unwrap(), "2026-0002");
    assert_eq!(r.prochain_numero(2026).unwrap(), "2026-0003");
    assert_eq!(r.prendre_numero(2027).unwrap(), "2027-0001");
    assert_eq!(r.prochain_numero(2026).unwrap(), "2026-0003");
}

#[test]
fn une_ecriture_interrompue_garde_l_ancien_atelier() {
    let (_d, r) = reglage();
    r.enregistrer(&saisi()).unwrap();
    // Le fichier temporaire ne peut pas être écrit : un dossier occupe sa place.
    std::fs::create_dir_all(r.dossier().join("atelier.json.tmp")).unwrap();
    let autre = AtelierSaisi {
        nom: "Autre".into(),
        ..saisi()
    };
    assert!(matches!(
        r.enregistrer(&autre),
        Err(ErreurAtelier::Ecriture(_))
    ));
    assert_eq!(r.saisi().unwrap(), saisi());
}

#[test]
fn un_fichier_abime_est_dit_illisible() {
    let (_d, r) = reglage();
    std::fs::create_dir_all(r.dossier()).unwrap();
    std::fs::write(r.dossier().join("atelier.json"), "{ pas du json").unwrap();
    assert!(matches!(r.lire(), Err(ErreurAtelier::Lecture(_))));
}

#[test]
fn chaque_erreur_a_son_texte() {
    for e in [
        ErreurAtelier::LogoFormat,
        ErreurAtelier::LogoIllisible,
        ErreurAtelier::TropLong("nom"),
        ErreurAtelier::Ecriture(String::new()),
        ErreurAtelier::Lecture(String::new()),
    ] {
        let cle = e.cle();
        for langue in [app::textes::Langue::Francais, app::textes::Langue::Anglais] {
            assert_ne!(app::textes::texte(langue, cle), cle, "{cle}");
        }
    }
}
