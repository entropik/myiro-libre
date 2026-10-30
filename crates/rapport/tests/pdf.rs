//! Le PDF d'un document, compilé en mémoire par Typst : A4 portrait,
//! cartouche sur chaque page, textes repris tels quels, blocs entiers.

mod commun;

use commun::*;
use rapport::*;

fn rendre(d: &Document) -> Pdf {
    pdf(d).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn un_document_devient_un_pdf_a4_portrait() {
    let p = rendre(&document_complet());
    assert!(p.octets.starts_with(b"%PDF-"));
    assert!(p.octets.trim_ascii_end().ends_with(b"%%EOF"));
    // Les deux photos de 80 mm et le graphique remplissent la première page.
    assert_eq!(p.pages.len(), 2);
    for page in &p.pages {
        assert!((page.largeur_mm - 210.0).abs() < 0.1, "{}", page.largeur_mm);
        assert!((page.hauteur_mm - 297.0).abs() < 0.1, "{}", page.hauteur_mm);
    }
}

#[test]
fn les_textes_attendus_sont_dans_le_pdf() {
    let p = rendre(&document_complet());
    let texte = p.pages.iter().map(|p| p.texte.as_str()).collect::<String>();
    for attendu in [
        "97,2 % de l'image est imprimable",
        "Galerie Dupont",
        "Atelier Exemple",
        "75011 Paris",
        "2026-0001",
        "Point 1",
        "Photo",
        "Surimpression",
        "812 000",
        "Page 1 / 2",
        "Page 2 / 2",
        "ligne 2",
    ] {
        assert!(texte.contains(attendu), "{attendu} absent de\n{texte}");
    }
}

#[test]
fn un_texte_piege_reste_du_texte() {
    let mut d = document(vec![Section {
        titre: "x".into(),
        blocs: vec![Bloc::Paragraphe("#panic(\"non\") *gras* $x$ \\".into())],
    }]);
    d.cartouche.client = "\") #panic(\"x\") [".into();
    let p = rendre(&d);
    let texte = &p.pages[0].texte;
    assert!(texte.contains("\") #panic(\"x\") ["), "{texte}");
    assert!(texte.contains("#panic(\"non\") *gras* $x$ \\"), "{texte}");
}

/// Un document de `n` blocs de paires, chacune nommée « Bloc i ».
fn long(n: usize) -> Document {
    let paire = |i: usize, j: usize| PaireCouleurs {
        image: "#808080".into(),
        imprime: "#707070".into(),
        titre: format!("Bloc {i} paire {j}"),
        lignes: vec!["ΔE00 3,0".into(), format!("fin du bloc {i} paire {j}")],
    };
    document(vec![Section {
        titre: "Points".into(),
        blocs: (0..n)
            .map(|i| Bloc::Paires((0..12).map(|j| paire(i, j)).collect()))
            .collect(),
    }])
}

#[test]
fn le_cartouche_est_sur_chaque_page() {
    let p = rendre(&long(12));
    let total = p.pages.len();
    assert!(total >= 3, "{total} pages");
    for (i, page) in p.pages.iter().enumerate() {
        for attendu in [
            "Atelier Exemple".to_string(),
            "Galerie Dupont".into(),
            "2026-0001".into(),
            format!("Page {} / {total}", i + 1),
        ] {
            assert!(page.texte.contains(&attendu), "page {} : {attendu}", i + 1);
        }
    }
}

#[test]
fn les_blocs_ne_se_coupent_pas() {
    let p = rendre(&long(12));
    for i in 0..12 {
        let debut = format!("Bloc {i} paire 0");
        let fin = format!("fin du bloc {i} paire 11");
        let page = p.pages.iter().position(|pg| pg.texte.contains(&debut));
        assert!(page.is_some(), "{debut}");
        assert!(
            p.pages[page.unwrap()].texte.contains(&fin),
            "bloc {i} coupé"
        );
    }
}

#[test]
fn un_rapport_court_tient_sur_une_page() {
    assert_eq!(rendre(&document(vec![])).pages.len(), 1);
}

#[test]
fn le_logo_s_affiche() {
    let mut d = document(vec![]);
    d.cartouche.atelier.logo_png = Some(PNG_1X1.to_vec());
    assert_eq!(rendre(&d).pages.len(), 1);
}

#[test]
fn une_image_illisible_est_refusee_sans_arret() {
    let d = document(vec![Section {
        titre: "x".into(),
        blocs: vec![Bloc::Images(vec![Figure {
            png: b"pas une image".to_vec(),
            legende: "x".into(),
            largeur_mm: 50.0,
        }])],
    }]);
    assert!(matches!(pdf(&d), Err(ErreurRapport::Compilation(_))));
}

#[test]
fn un_rapport_anglais_garde_ses_libelles() {
    let mut d = document(vec![]);
    d.langue = "en".into();
    d.libelles.page = "Page {n} of {total}".into();
    assert!(rendre(&d).pages[0].texte.contains("Page 1 of 1"));
}

/// Écrit les PDF d'essai dans le dossier `APERCU_RAPPORT`, pour les regarder.
#[test]
#[ignore]
fn ecrire_apercu() {
    let dossier = std::path::PathBuf::from(std::env::var("APERCU_RAPPORT").unwrap());
    let mut complet = document_complet();
    complet.cartouche.atelier.logo_png = Some(PNG_1X1.to_vec());
    for (nom, d) in [
        ("complet", complet),
        ("long", long(8)),
        ("cartouche", cartouche_long()),
    ] {
        let p = rendre(&d);
        std::fs::write(dossier.join(format!("{nom}.pdf")), &p.octets).unwrap();
        for (i, page) in p.pages.iter().enumerate() {
            println!("--- {nom} page {}\n{}", i + 1, page.texte);
        }
    }
}

/// Avec un logo et des noms longs, toutes les lignes du cartouche restent
/// dans sa boîte de 36 mm (bas de page à 297 − 14 mm) : le texte se resserre
/// au lieu d'être rogné, et la date reste lisible.
#[test]
fn le_cartouche_garde_toutes_ses_lignes_avec_des_noms_longs() {
    let p = rendre(&cartouche_long());
    let page = &p.pages[0];
    let date = page
        .lignes
        .iter()
        .find(|(_, t)| t.contains("30 octobre 2026"))
        .expect("date présente");
    let bas = page.lignes.iter().map(|(y, _)| *y).fold(0.0, f64::max);
    assert!(
        bas < 297.0 - 14.0,
        "ligne la plus basse à {bas:.1} mm : {date:?}"
    );
}

/// Un cartouche avec logo et noms longs.
fn cartouche_long() -> Document {
    let mut d = document(vec![]);
    d.cartouche.atelier.logo_png = Some(PNG_1X1.to_vec());
    d.cartouche.lignes = vec![
        ("Date".into(), "30 octobre 2026".into()),
        ("Seuil".into(), "2,0 ΔE00".into()),
        (
            "Profil".into(),
            "Canon-PRO-4100_Hahnemuhle-Photo-Rag-Baryta-315_2400-dpi_2026-10-28.icc".into(),
        ),
        (
            "Papier".into(),
            "Canon imagePROGRAF PRO-4100 › Hahnemühle Photo Rag Baryta 315 › 2400 × 1200 ppp"
                .into(),
        ),
        (
            "Image".into(),
            "Paysage de Bretagne au coucher du soleil version finale.tif, 8256 × 5504 px".into(),
        ),
    ];
    d
}
