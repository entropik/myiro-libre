//! Écriture d'un [`Document`] en source Typst : le gabarit fixe
//! (`gabarit.typ`), puis le document, fait d'appels aux fonctions du gabarit
//! dont chaque texte est un littéral de chaîne. Les images sont des fichiers
//! virtuels (`/images/…`) rendus avec la source : rien d'autre n'est lu.

use std::fmt::Write;

use crate::{nettoyer_svg, Bloc, Document, PaireCouleurs, Section};

const GABARIT: &str = include_str!("gabarit.typ");
/// Sépare le gabarit du document dans la source.
const SEPARATION: &str = "// ---- document ----\n";

/// Une source Typst et les fichiers qu'elle appelle (chemin virtuel,
/// octets).
#[derive(Clone, Debug, PartialEq)]
pub struct SourceTypst {
    pub texte: String,
    pub fichiers: Vec<(String, Vec<u8>)>,
}

/// Un texte en littéral de chaîne Typst : guillemets et barres obliques
/// inverses échappés, saut de ligne et tabulation écrits `\n` et `\t`, autres
/// caractères de contrôle retirés.
pub fn chaine_typst(t: &str) -> String {
    let mut s = String::with_capacity(t.len() + 2);
    s.push('"');
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            c if c.is_control() => {}
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

/// Une liste Typst : `()` vide, sinon chaque élément suivi d'une virgule.
fn liste<T>(elements: &[T], ecrire: impl Fn(&T) -> String) -> String {
    if elements.is_empty() {
        return "()".into();
    }
    let mut s = String::from("(");
    for e in elements {
        s.push_str(&ecrire(e));
        s.push_str(", ");
    }
    s.pop();
    s.push(')');
    s
}

fn chaines(elements: &[String]) -> String {
    liste(elements, |e| chaine_typst(e))
}

fn couple((k, v): &(String, String)) -> String {
    format!("({}, {})", chaine_typst(k), chaine_typst(v))
}

/// `#rrggbb` en minuscules, ou `none` pour toute autre forme.
fn couleur(c: &str) -> String {
    let hexa = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit());
    if hexa {
        chaine_typst(&c.to_ascii_lowercase())
    } else {
        "none".into()
    }
}

fn paire(p: &PaireCouleurs) -> String {
    format!(
        "({}, {}, {}, {})",
        couleur(&p.image),
        couleur(&p.imprime),
        chaine_typst(&p.titre),
        chaines(&p.lignes)
    )
}

/// Les fichiers d'images de la source : le logo, puis les autres, numérotés
/// dans l'ordre.
#[derive(Default)]
struct Fichiers {
    liste: Vec<(String, Vec<u8>)>,
    numeros: usize,
}

impl Fichiers {
    fn ajouter(&mut self, extension: &str, octets: Vec<u8>) -> String {
        let chemin = format!("/images/{}.{extension}", self.numeros);
        self.numeros += 1;
        self.liste.push((chemin.clone(), octets));
        chemin
    }
}

fn bloc(s: &mut String, b: &Bloc, fichiers: &mut Fichiers) {
    let _ = match b {
        Bloc::Verdict { texte, detail } => writeln!(
            s,
            "#verdict({}, {})",
            chaine_typst(texte),
            chaine_typst(detail)
        ),
        Bloc::Paragraphe(t) => writeln!(s, "#paragraphe({})", chaine_typst(t)),
        Bloc::Couples(c) => writeln!(s, "#couples({})", liste(c, couple)),
        Bloc::Tableau {
            entetes,
            lignes,
            nombres,
        } => {
            // Chaque ligne a autant de cellules que d'en-têtes.
            let lignes: Vec<Vec<String>> = lignes
                .iter()
                .map(|l| {
                    let mut l = l.clone();
                    l.resize(entetes.len(), String::new());
                    l
                })
                .collect();
            writeln!(
                s,
                "#tableau({}, {}, {})",
                chaines(entetes),
                liste(&lignes, |l| chaines(l)),
                liste(nombres, |n| n.to_string())
            )
        }
        Bloc::Images(figures) => {
            let mut appels = Vec::new();
            for f in figures {
                let chemin = fichiers.ajouter("png", f.png.clone());
                let largeur = if f.largeur_mm.is_finite() {
                    f.largeur_mm.clamp(10.0, 180.0)
                } else {
                    80.0
                };
                appels.push(format!(
                    "({}, {largeur:.1}mm, {})",
                    chaine_typst(&chemin),
                    chaine_typst(&f.legende)
                ));
            }
            writeln!(s, "#images({})", liste(&appels, String::clone))
        }
        Bloc::Paires(p) => writeln!(s, "#paires({})", liste(p, paire)),
        Bloc::Svg { svg, legende } => match nettoyer_svg(svg) {
            Ok(propre) => {
                let chemin = fichiers.ajouter("svg", propre.into_bytes());
                writeln!(
                    s,
                    "#graphique({}, {})",
                    chaine_typst(&chemin),
                    chaine_typst(legende)
                )
            }
            Err(_) => writeln!(s, "#legende({})", chaine_typst(legende)),
        },
        Bloc::Pre(t) => writeln!(s, "#pre({})", chaine_typst(t)),
    };
}

fn section(s: &mut String, section: &Section, fichiers: &mut Fichiers) {
    let _ = writeln!(s, "#section({})", chaine_typst(&section.titre));
    for b in &section.blocs {
        bloc(s, b, fichiers);
    }
}

/// La source Typst d'un document et ses fichiers d'images.
pub fn source_typst(d: &Document) -> SourceTypst {
    let c = &d.cartouche;
    let a = &c.atelier;
    let l = &d.libelles;
    let langue = if d.langue == "en" { "en" } else { "fr" };
    let mut fichiers = Fichiers::default();
    let logo = match &a.logo_png {
        Some(png) => {
            fichiers
                .liste
                .push(("/images/logo.png".into(), png.clone()));
            chaine_typst("/images/logo.png")
        }
        None => "none".into(),
    };
    let mut s = String::from(GABARIT);
    s.push('\n');
    s.push_str(SEPARATION);
    let _ = writeln!(
        s,
        "#set document(title: {})",
        chaine_typst(&format!("{} {}", c.titre, c.numero))
    );
    let _ = writeln!(s, "#set text(lang: \"{langue}\")");
    let _ = writeln!(
        s,
        "#set page(paper: \"a4\", margin: (top: 15mm, x: 15mm, bottom: 56mm), footer-descent: 6mm, footer: context cartouche("
    );
    let _ = writeln!(
        s,
        "  (nom: {}, adresse: {}, courriel: {}, telephone: {}, site: {}, logo: {logo}),",
        chaine_typst(&a.nom),
        chaine_typst(&a.adresse),
        chaine_typst(&a.courriel),
        chaine_typst(&a.telephone),
        chaine_typst(&a.site),
    );
    let _ = writeln!(
        s,
        "  {}, {}, {}, {}, {},",
        chaine_typst(&c.client),
        chaine_typst(&c.objet),
        liste(&c.lignes, couple),
        chaine_typst(&c.numero),
        chaine_typst(&c.titre),
    );
    let _ = writeln!(
        s,
        "  (page: {}, atelier: {}, client: {}, objet: {}, numero: {}),\n))",
        chaine_typst(&l.page),
        chaine_typst(&l.atelier),
        chaine_typst(&l.client),
        chaine_typst(&l.objet),
        chaine_typst(&l.numero),
    );
    let _ = writeln!(s, "#titre-document({})", chaine_typst(&c.titre));
    for sec in &d.sections {
        section(&mut s, sec, &mut fichiers);
    }
    SourceTypst {
        texte: s,
        fichiers: fichiers.liste,
    }
}
