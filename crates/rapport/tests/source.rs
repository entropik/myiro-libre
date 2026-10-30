//! La source Typst d'un document : textes toujours en littéraux de chaîne,
//! fichiers virtuels seulement, cartouche en pied de chaque page, style sobre.

mod commun;

use commun::*;
use rapport::*;

fn texte(d: &Document) -> String {
    source_typst(d).texte
}

/// Partie fixe de la source (le gabarit), avant le document.
fn gabarit(s: &str) -> &str {
    s.split("// ---- document ----").next().unwrap()
}

/// Partie propre au document.
fn corps(s: &str) -> &str {
    s.split("// ---- document ----").nth(1).unwrap()
}

#[test]
fn la_source_n_appelle_aucun_fichier_hors_des_siens() {
    let s = source_typst(&document_complet());
    for interdit in [
        "#import", "#include", "read(", "json(", "csv(", "plugin(", "http",
    ] {
        assert!(!s.texte.contains(interdit), "{interdit} dans la source");
    }
    // Chaque image appelée est un fichier fourni avec la source.
    let appelees: Vec<&str> = s
        .texte
        .split("\"/images/")
        .skip(1)
        .map(|m| &m[..m.find('"').unwrap()])
        .collect();
    assert_eq!(appelees, ["0.png", "1.png", "2.svg"]);
    for nom in appelees {
        let chemin = format!("/images/{nom}");
        assert!(s.fichiers.iter().any(|(c, _)| *c == chemin), "{chemin}");
    }
    assert_eq!(s.fichiers.len(), 3);
    assert!(s.fichiers.iter().all(|(c, _)| c.starts_with("/images/")));
}

#[test]
fn les_textes_saisis_sont_echappes() {
    let mut d = document(vec![]);
    d.cartouche.client = "<script>&\"' \\ \") #panic(\"x\") [*gras*] $x$ #let".into();
    let s = texte(&d);
    assert!(s.contains(r#""<script>&\"' \\ \") #panic(\"x\") [*gras*] $x$ #let""#));
    assert_eq!(chaine_typst("a\"b\\c\nd\te\u{0}f\r"), r#""a\"b\\c\nd\tef""#);
}

#[test]
fn le_cartouche_porte_atelier_client_numero() {
    let s = texte(&document(vec![]));
    for attendu in [
        "\"Atelier Exemple\"",
        "\"12 rue des Presses\\n75011 Paris\"",
        "\"contact@exemple.fr\"",
        "\"01 23 45 67 89\"",
        "\"exemple.fr\"",
        "\"Galerie Dupont\"",
        "\"Tirage Paysage 60 × 40\"",
        "(\"Papier\", \"Hahnemühle Photo Rag\")",
        "\"2026-0001\"",
        "\"Rapport d'image\"",
        "\"Page {n} / {total}\"",
        "\"N° de rapport\"",
    ] {
        assert!(s.contains(attendu), "{attendu} absent");
    }
}

#[test]
fn le_logo_est_une_image_png_fournie() {
    let sans = source_typst(&document(vec![]));
    assert!(!sans.texte.contains("logo.png"));
    assert!(sans.fichiers.is_empty());
    let mut d = document(vec![]);
    d.cartouche.atelier.logo_png = Some(PNG_1X1.to_vec());
    let avec = source_typst(&d);
    assert!(avec.texte.contains("logo: \"/images/logo.png\""));
    assert_eq!(
        avec.fichiers,
        vec![("/images/logo.png".to_string(), PNG_1X1.to_vec())]
    );
}

#[test]
fn le_cartouche_est_en_pied_de_chaque_page() {
    let s = texte(&document(vec![]));
    assert!(s.contains("#set page(paper: \"a4\""));
    assert!(s.contains("footer: context cartouche("));
    assert!(s.contains("counter(page).final()"));
}

#[test]
fn les_blocs_ne_se_coupent_pas_et_le_pied_garde_sa_place() {
    let s = texte(&document_complet());
    for appel in [
        "#verdict(",
        "#images(",
        "#paires(",
        "#graphique(",
        "#couples(",
    ] {
        assert!(corps(&s).contains(appel), "{appel} absent");
    }
    // Chaque bloc d'images, de paires, de verdict, de couples ou de
    // graphique est insécable.
    for f in ["verdict", "images", "paires", "graphique", "couples"] {
        let def = gabarit(&s).split(&format!("#let {f}(")).nth(1).unwrap();
        let def = def.split("\n#let ").next().unwrap();
        assert!(def.contains("breakable: false"), "{f} sécable");
    }
    // Le cartouche a une hauteur fixe, rognée, dans la marge du bas.
    let def = gabarit(&s).split("#let cartouche(").nth(1).unwrap();
    assert!(def.contains("clip: true"));
}

#[test]
fn le_style_est_sobre() {
    let s = texte(&document_complet());
    for interdit in [
        "radius", "gradient", "shadow", "cmyk(", "red", "blue", "green",
    ] {
        assert!(
            !gabarit(&s).contains(interdit),
            "{interdit} dans le gabarit"
        );
    }
    // Les seules couleurs du document sont celles des paires, en #rrggbb
    // contrôlés, passées comme chaînes au seul carré de couleur.
    assert_eq!(gabarit(&s).matches("rgb(").count(), 1);
    assert!(gabarit(&s).contains("rgb(c)"));
    assert!(!corps(&s).contains("rgb("));
}

#[test]
fn une_paire_garde_ses_deux_couleurs_et_son_texte() {
    let s = texte(&document_complet());
    assert!(s.contains(r##"("#c03020", "#a84030", "Point 1", ("ΔE00 4,1", "au-delà du seuil",))"##));
}

#[test]
fn une_couleur_mal_formee_n_est_pas_posee() {
    let d = document(vec![Section {
        titre: "x".into(),
        blocs: vec![Bloc::Paires(vec![PaireCouleurs {
            image: "red\") #panic(".into(),
            imprime: "#ABCDEF".into(),
            titre: "P".into(),
            lignes: vec![],
        }])],
    }]);
    let s = texte(&d);
    assert!(s.contains(r##"(none, "#abcdef", "P", ())"##));
    assert!(!s.contains("panic"));
}

#[test]
fn la_langue_est_fr_ou_en_seulement() {
    let mut d = document(vec![]);
    d.langue = "en".into();
    assert!(texte(&d).contains("lang: \"en\""));
    d.langue = "\") #x".into();
    assert!(texte(&d).contains("lang: \"fr\""));
}

#[test]
fn un_graphique_refuse_ne_garde_que_sa_legende() {
    let d = document(vec![Section {
        titre: "x".into(),
        blocs: vec![Bloc::Svg {
            svg: "<svg><script>alert(1)</script></svg>".into(),
            legende: "Coupe".into(),
        }],
    }]);
    let s = source_typst(&d);
    assert!(s.fichiers.is_empty());
    assert!(!corps(&s.texte).contains("#graphique("));
    assert!(s.texte.contains("\"Coupe\""));
}

/// La source d'un document qui emploie chaque bloc est comparée à
/// `tests/or/document.typ`. `REGENERER_OR=1` la réécrit.
#[test]
fn la_source_d_un_document_complet_est_celle_de_reference() {
    let s = texte(&document_complet());
    let chemin = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/or/document.typ");
    if std::env::var_os("REGENERER_OR").is_some() {
        std::fs::write(chemin, &s).unwrap();
    }
    let attendu = std::fs::read_to_string(chemin)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(s, attendu);
}
