//! Rapport client d'une image (ticket #89) : le document est construit côté
//! Rust depuis l'image et les analyses gardées en mémoire, sans Tauri ; le
//! PDF est compilé pour de vrai (en mémoire) et écrit dans un dossier
//! temporaire.

mod commun_image;

use app::atelier::{AtelierSaisi, ReglageAtelier};
use app::comparer::image::rapport::{
    phrase_imprimable, vignette, vignette_surimpression, DemandeRapportImage, GraphiquePartie,
    Jour, TableauPage,
};
use app::textes::{texte, Langue};
use commun_image::*;
use gamut::AnalyseImage;
use rapport::{Bloc, Document};

const JOUR: Jour = Jour {
    annee: 2026,
    mois: 10,
    jour: 30,
};

fn analyse(histogramme: Vec<u64>, inconnus: u64) -> AnalyseImage {
    let connus: u64 = histogramme.iter().sum();
    AnalyseImage {
        pixels_comptes: connus + inconnus,
        pixels_inconnus: inconnus,
        couleurs_distinctes: 1,
        histogramme,
        par_teinte: vec![],
        nuage: vec![],
        pastilles: vec![],
        ecarts: vec![],
    }
}

/// Histogramme : `bons` pixels au code 0, `au_dela` au code 50 (5,0 ΔE00).
fn histo(bons: u64, au_dela: u64) -> Vec<u64> {
    let mut h = vec![0; 255];
    h[0] = bons;
    h[50] = au_dela;
    h
}

#[test]
fn la_part_imprimable_suit_la_regle_de_la_page() {
    let fr = Langue::Francais;
    let phrase = |b, a, i| phrase_imprimable(Some(&analyse(histo(b, a), i)), 2.0, fr);
    assert_eq!(
        phrase(972, 28, 0),
        "97,2\u{202f}% de l’image est imprimable sur ce papier avec ce profil"
    );
    assert!(phrase(10_000, 1, 0).starts_with("plus de 99,9\u{202f}%"));
    assert!(phrase(1, 10_000, 0).starts_with("moins de 0,1\u{202f}%"));
    assert!(phrase(995, 0, 5).ends_with("\u{202f}; 0,5\u{202f}% inconnus"));
    assert_eq!(
        phrase_imprimable(None, 2.0, fr),
        "Part imprimable inconnue avec ce profil"
    );
    // Le seuil compte : à 6,0 ΔE00, les pixels à 5,0 sont bons.
    assert!(
        phrase_imprimable(Some(&analyse(histo(972, 28), 0)), 6.0, fr).starts_with("100,0\u{202f}%")
    );
    assert!(
        phrase_imprimable(Some(&analyse(histo(972, 28), 0)), 2.0, Langue::Anglais)
            .starts_with("97.2%")
    );
}

/// Un banc avec une image de 40 × 30 analysée face à deux profils.
fn prepare() -> (Banc, Vec<app::comparer::SourceComparaison>, u64) {
    let b = banc();
    let rangee = b.ranger("PRO-4100 Photo Rag.icc", &essai("Essai rangé"));
    let fichier = b.ouvrir_fichier("autre.icc", &essai("Essai fichier"));
    b.image.ouvrir_octets("photo.png", &png(40, 30)).unwrap();
    let sources = vec![rangee, fichier];
    let numero = b.analyser(&sources).numero;
    (b, sources, numero)
}

fn demande(numero: u64) -> DemandeRapportImage {
    DemandeRapportImage {
        langue: "fr".into(),
        client: "Galerie Dupont".into(),
        objet: "Tirage 60 × 40".into(),
        analyse: numero,
        profil_surimpression: 0,
        seuil: 2.0,
        points: vec![],
        parties: vec![],
        graphiques: vec![],
    }
}

fn document(b: &Banc, d: &DemandeRapportImage) -> Document {
    b.ouverte
        .document_rapport_image(
            &b.image,
            &b.fichiers,
            d,
            rapport::Atelier::default(),
            "2026-0001",
            JOUR,
        )
        .unwrap()
}

fn blocs(d: &Document) -> Vec<&Bloc> {
    d.sections.iter().flat_map(|s| &s.blocs).collect()
}

/// Tous les textes d'un document, cartouche compris.
fn textes(d: &Document) -> String {
    let mut t = vec![
        d.cartouche.titre.clone(),
        d.cartouche.client.clone(),
        d.cartouche.objet.clone(),
        d.libelles.page.clone(),
        d.libelles.numero.clone(),
        d.libelles.atelier.clone(),
        d.libelles.client.clone(),
        d.libelles.objet.clone(),
    ];
    for (k, v) in &d.cartouche.lignes {
        t.extend([k.clone(), v.clone()]);
    }
    for s in &d.sections {
        t.push(s.titre.clone());
        for b in &s.blocs {
            t.push(format!("{b:?}"));
        }
    }
    t.join("\n")
}

#[test]
fn le_rapport_porte_verdict_vignettes_et_pastilles() {
    let (b, _, numero) = prepare();
    let d = document(&b, &demande(numero));
    let tout = blocs(&d);
    let verdict = tout
        .iter()
        .find_map(|b| match b {
            Bloc::Verdict { texte, detail } => Some((texte.clone(), detail.clone())),
            _ => None,
        })
        .unwrap();
    assert!(
        verdict.0.contains("de l’image est imprimable"),
        "{verdict:?}"
    );
    assert!(verdict.1.contains("Photo-Rag-308_720-dpi"), "{verdict:?}");
    let figures = tout
        .iter()
        .find_map(|b| match b {
            Bloc::Images(f) => Some(f.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(figures.len(), 2);
    for f in &figures {
        let png = image::load_from_memory(&f.png).unwrap();
        assert_eq!((png.width(), png.height()), (40, 30));
    }
    let paires = tout
        .iter()
        .find_map(|b| match b {
            Bloc::Paires(p) => Some(p.clone()),
            _ => None,
        })
        .unwrap();
    assert!(!paires.is_empty() && paires.len() <= 12, "{}", paires.len());
    assert!(paires
        .iter()
        .all(|p| p.image.starts_with('#') && p.imprime.starts_with('#')));
    // Le cartouche dit le profil, son papier, l'image et la date.
    let lignes = format!("{:?}", d.cartouche.lignes);
    for attendu in [
        "Photo-Rag-308_720-dpi",
        "Canon PRO-4100 › Photo Rag 308 › 720 dpi",
        "photo.png",
        "30 octobre 2026",
    ] {
        assert!(lignes.contains(attendu), "{attendu} absent de {lignes}");
    }
    assert_eq!(d.cartouche.numero, "2026-0001");
    assert_eq!(d.cartouche.client, "Galerie Dupont");
}

#[test]
fn la_surimpression_suit_le_seuil_choisi() {
    let pixels: Vec<[u8; 4]> = (0..100).map(|i| [i as u8, 0, 0, 255]).collect();
    // Codes de 0 à 99, et quelques pixels à l'écart inconnu (255).
    let mut ecarts: Vec<u8> = (0..100).map(|i| i as u8).collect();
    ecarts[99] = 255;
    let verts = |seuil: f64| {
        let png = vignette_surimpression(&pixels, &ecarts, 10, seuil, 1_600);
        let image = image::load_from_memory(&png).unwrap().to_rgba8();
        image
            .pixels()
            .filter(|p| p.0 == [0x39, 0xff, 0x14, 255])
            .count()
    };
    assert_eq!(verts(2.0), 79); // codes 20 à 98
    assert_eq!(verts(5.0), 49); // codes 50 à 98
                                // Réduite, la vignette garde ses proportions.
    let petite = image::load_from_memory(&vignette(&pixels, 10, 5)).unwrap();
    assert_eq!((petite.width(), petite.height()), (5, 5));
}

#[test]
fn les_points_releves_partent_avec_leur_paire() {
    let (b, _, numero) = prepare();
    let d = document(
        &b,
        &DemandeRapportImage {
            points: vec![(0, 0), (39, 29)],
            ..demande(numero)
        },
    );
    let section = d
        .sections
        .iter()
        .find(|s| s.titre == "Points relevés")
        .expect("section des points");
    let Bloc::Paires(p) = &section.blocs[0] else {
        panic!("{:?}", section.blocs)
    };
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].titre, "Point 1 (x 0, y 0)");
    let lignes = p[0].lignes.join(" | ");
    assert!(lignes.contains("Image L*"), "{lignes}");
    assert!(lignes.contains("Imprimé L*"), "{lignes}");
    assert!(lignes.contains("ΔE00"), "{lignes}");
    assert!(lignes.contains("du seuil d’alerte"), "{lignes}");
    // Plus de 8 points, ou un point hors de l'image : refus.
    for points in [vec![(0, 0); 9], vec![(40, 0)]] {
        let refus = b.ouverte.document_rapport_image(
            &b.image,
            &b.fichiers,
            &DemandeRapportImage {
                points,
                ..demande(numero)
            },
            rapport::Atelier::default(),
            "2026-0001",
            JOUR,
        );
        assert!(refus.is_err());
    }
}

fn graphique(partie: &str, svg: &str) -> GraphiquePartie {
    GraphiquePartie {
        partie: partie.into(),
        svg: Some(svg.into()),
        png_base64: None,
        tableau: None,
        texte: None,
        legende: None,
    }
}

const SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 40\"><rect width=\"100\" height=\"40\"/></svg>";

#[test]
fn une_partie_decochee_n_apparait_pas() {
    let (b, _, numero) = prepare();
    let mut d = demande(numero);
    d.graphiques = vec![
        graphique("comparer.coupe", SVG),
        graphique("comparer.teintes", SVG),
    ];
    d.parties = vec!["comparer.teintes".into()];
    let doc = document(&b, &d);
    let titres: Vec<&str> = doc.sections.iter().map(|s| s.titre.as_str()).collect();
    assert!(titres.contains(&"Chroma maximale par teinte"), "{titres:?}");
    assert!(!titres.contains(&"Coupe à clarté constante"), "{titres:?}");
    // Une partie inconnue est ignorée.
    d.parties.push("autre.chose".into());
    d.graphiques.push(graphique("autre.chose", SVG));
    assert_eq!(document(&b, &d).sections.len(), doc.sections.len());
}

#[test]
fn un_svg_refuse_n_arrete_pas_le_rapport() {
    let (b, _, numero) = prepare();
    let mut d = demande(numero);
    d.parties = vec!["comparer.coupe".into()];
    d.graphiques = vec![graphique(
        "comparer.coupe",
        "<svg><script>x()</script></svg>",
    )];
    let doc = document(&b, &d);
    let coupe = doc
        .sections
        .iter()
        .find(|s| s.titre == "Coupe à clarté constante")
        .unwrap();
    assert_eq!(
        coupe.blocs,
        vec![Bloc::Paragraphe(
            "Graphique non repris dans le rapport.".into()
        )]
    );
}

#[test]
fn un_tableau_de_la_page_devient_un_tableau() {
    let (b, _, numero) = prepare();
    let mut d = demande(numero);
    d.parties = vec!["comparer.primaires".into()];
    d.graphiques = vec![GraphiquePartie {
        partie: "comparer.primaires".into(),
        svg: None,
        png_base64: None,
        tableau: Some(TableauPage {
            entetes: vec!["Couleur".into(), "Profil".into()],
            lignes: vec![vec!["Rouge".into(), "L* 50".into()]],
        }),
        texte: None,
        legende: None,
    }];
    let doc = document(&b, &d);
    let s = doc
        .sections
        .iter()
        .find(|s| s.titre == "Couleurs de base")
        .unwrap();
    assert!(matches!(&s.blocs[0], Bloc::Tableau { lignes, .. } if lignes[0][0] == "Rouge"));
}

#[test]
fn le_rapport_ne_porte_aucun_identifiant_d_instrument() {
    let b = banc();
    let source = b.ranger(
        "piege.icc",
        &essai("MYIRO-1 Serial 12345678 MAC 00:11:22:33:44:55"),
    );
    b.image.ouvrir_octets("photo.png", &png(20, 10)).unwrap();
    let numero = b.analyser(&[source]).numero;
    let pdf = rapport::pdf(&document(&b, &demande(numero))).unwrap();
    let tout: String = pdf.pages.iter().map(|p| p.texte.as_str()).collect();
    for interdit in ["12345678", "Serial", "00:11:22", "MYIRO-1"] {
        assert!(!tout.contains(interdit), "{interdit} dans le rapport");
    }
}

#[test]
fn le_numero_n_est_pris_qu_apres_un_pdf_ecrit() {
    let (b, sources, numero) = prepare();
    let _ = sources;
    let reglage = ReglageAtelier::new(b.dossier.path().join("atelier"));
    reglage
        .enregistrer(&AtelierSaisi {
            nom: "Atelier Exemple".into(),
            ..Default::default()
        })
        .unwrap();
    let absent = b.dossier.path().join("absent").join("Rapport.pdf");
    let refus = b.ouverte.exporter_rapport_image(
        &b.image,
        &b.fichiers,
        &demande(numero),
        &reglage,
        &absent,
        JOUR,
    );
    assert_eq!(refus, Err("rapport.erreur.destination".to_string()));
    assert_eq!(reglage.prochain_numero(2026).unwrap(), "2026-0001");

    let sortie = b.dossier.path().join("Rapport.pdf");
    let pris = b
        .ouverte
        .exporter_rapport_image(
            &b.image,
            &b.fichiers,
            &demande(numero),
            &reglage,
            &sortie,
            JOUR,
        )
        .unwrap();
    assert_eq!(pris, "2026-0001");
    assert!(std::fs::read(&sortie).unwrap().starts_with(b"%PDF-"));
    assert_eq!(reglage.prochain_numero(2026).unwrap(), "2026-0002");
}

#[test]
fn une_analyse_perimee_est_refusee() {
    let (b, sources, numero) = prepare();
    b.analyser(&sources);
    let refus = b.ouverte.document_rapport_image(
        &b.image,
        &b.fichiers,
        &demande(numero),
        rapport::Atelier::default(),
        "2026-0001",
        JOUR,
    );
    assert_eq!(refus.unwrap_err(), "image.refus.sans_ecarts");
}

#[test]
fn un_rapport_anglais_n_a_aucun_texte_francais() {
    let (b, _, numero) = prepare();
    let d = document(
        &b,
        &DemandeRapportImage {
            langue: "en".into(),
            points: vec![(1, 1)],
            ..demande(numero)
        },
    );
    let tout = textes(&d);
    assert!(tout.contains("of the image is printable"), "{tout}");
    assert!(tout.contains("30 October 2026"), "{tout}");
    for francais in [
        "imprimable",
        "l’image",
        "Écart",
        "Points relevés",
        "Chiffres",
        "octobre",
        "Imprimé",
        "Profil de",
        "seuil",
        "Atelier",
        "Objet",
        "N° de rapport",
    ] {
        assert!(
            !tout.contains(francais),
            "« {francais} » dans le rapport anglais :\n{tout}"
        );
    }
    assert_eq!(
        texte(Langue::Anglais, "rapport.image.titre"),
        d.cartouche.titre
    );
}

#[test]
fn le_nom_par_defaut_reprend_l_image_et_le_numero() {
    use app::comparer::image::rapport::commandes::nom_par_defaut;
    assert_eq!(
        nom_par_defaut("Paysage final.jpg", "2026-0001"),
        "Rapport Paysage final 2026-0001.pdf"
    );
    assert_eq!(
        nom_par_defaut("a<b?.png", "2026-0002"),
        "Rapport a_b_ 2026-0002.pdf"
    );
}

/// Écrit un rapport d'image d'essai dans le dossier `APERCU_RAPPORT`, pour le
/// regarder.
#[test]
#[ignore]
fn ecrire_apercu_image() {
    let dossier = std::path::PathBuf::from(std::env::var("APERCU_RAPPORT").unwrap());
    let (b, _, numero) = prepare();
    let reglage = ReglageAtelier::new(b.dossier.path().join("atelier"));
    reglage
        .enregistrer(&AtelierSaisi {
            nom: "Atelier Exemple".into(),
            adresse: "12 rue des Presses\n75011 Paris".into(),
            courriel: "contact@exemple.fr".into(),
            telephone: "01 23 45 67 89".into(),
            site: "exemple.fr".into(),
        })
        .unwrap();
    let mut d = DemandeRapportImage {
        points: vec![(0, 0), (10, 5), (39, 29)],
        ..demande(numero)
    };
    d.parties = vec!["comparer.coupe".into()];
    d.graphiques = vec![graphique("comparer.coupe", SVG)];
    for langue in ["fr", "en"] {
        d.langue = langue.into();
        let sortie = dossier.join(format!("image-{langue}.pdf"));
        b.ouverte
            .exporter_rapport_image(&b.image, &b.fichiers, &d, &reglage, &sortie, JOUR)
            .unwrap();
    }
}

#[test]
fn une_partie_a_deux_graphiques_les_garde_tous() {
    let (b, _, numero) = prepare();
    let mut d = demande(numero);
    d.parties = vec!["comparer.chemin".into()];
    d.graphiques = vec![
        graphique("comparer.chemin", SVG),
        graphique("comparer.chemin", SVG),
    ];
    let doc = document(&b, &d);
    let chemin = doc
        .sections
        .iter()
        .find(|s| s.titre == "Courbes le long d’un chemin du cube RVB")
        .unwrap();
    assert_eq!(chemin.blocs.len(), 2);
}

/// Une partie garde le texte qui situe ses tableaux (« Écarts de B par
/// rapport à A », sous-titres) et la légende de ses graphiques.
#[test]
fn une_partie_garde_ses_textes_et_ses_legendes() {
    let (b, _, numero) = prepare();
    let mut d = demande(numero);
    d.parties = vec!["comparer.ecarts".into(), "comparer.chemin".into()];
    let texte = |t: &str| GraphiquePartie {
        partie: "comparer.ecarts".into(),
        svg: None,
        png_base64: None,
        tableau: None,
        texte: Some(t.into()),
        legende: None,
    };
    let tableau = GraphiquePartie {
        tableau: Some(TableauPage {
            entetes: vec!["Zone".into(), "ΔE00".into()],
            lignes: vec![vec!["Neutres".into(), "1,2".into()]],
        }),
        texte: None,
        ..texte("")
    };
    d.graphiques = vec![
        texte("Écarts de B par rapport à A"),
        tableau.clone(),
        texte("Zones"),
        tableau,
        GraphiquePartie {
            legende: Some("Du bleu au cyan".into()),
            ..graphique("comparer.chemin", SVG)
        },
    ];
    let doc = document(&b, &d);
    let ecarts = doc
        .sections
        .iter()
        .find(|s| s.titre == "Écarts au profil de référence")
        .unwrap();
    assert!(matches!(&ecarts.blocs[0], Bloc::Paragraphe(t) if t == "Écarts de B par rapport à A"));
    assert!(matches!(&ecarts.blocs[1], Bloc::Tableau { .. }));
    assert!(matches!(&ecarts.blocs[2], Bloc::Paragraphe(t) if t == "Zones"));
    assert_eq!(ecarts.blocs.len(), 4);
    let chemin = doc
        .sections
        .iter()
        .find(|s| s.titre == "Courbes le long d’un chemin du cube RVB")
        .unwrap();
    assert!(matches!(&chemin.blocs[0], Bloc::Svg { legende, .. } if legende == "Du bleu au cyan"));
}
