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

/// Espace sans chasse : une coupure de ligne permise, invisible.
const COUPURE: char = '\u{200b}';
/// Lettres et chiffres d'affilée au-delà desquels une coupure est posée.
const SUITE_MAX: usize = 16;

/// Un texte du cartouche en littéral Typst, avec une coupure permise après
/// chaque séparateur d'un nom de fichier, d'une adresse électronique ou
/// d'un site (`_ - . / \ @ › + = &`), et toutes les 16 lettres ou chiffres
/// d'affilée sans séparateur : un nom long passe à la ligne au lieu de
/// déborder de sa case. Rien n'est ajouté de visible.
pub fn coupable(t: &str) -> String {
    let mut sortie = String::with_capacity(t.len() + t.len() / 8);
    let mut suite = 0;
    for c in t.chars() {
        if c.is_alphanumeric() {
            if suite == SUITE_MAX {
                sortie.push(COUPURE);
                suite = 0;
            }
            suite += 1;
            sortie.push(c);
        } else {
            suite = 0;
            sortie.push(c);
            if matches!(
                c,
                '_' | '-' | '.' | '/' | '\\' | '@' | '›' | '+' | '=' | '&'
            ) {
                sortie.push(COUPURE);
            }
        }
    }
    chaine_typst(&sortie)
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
        // Un tableau sans colonne ne se met pas en page : il est laissé de
        // côté plutôt que d'arrêter tout le rapport.
        Bloc::Tableau { entetes, .. } if entetes.is_empty() => Ok(()),
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
                let extension = if f.image.starts_with(&[0xff, 0xd8, 0xff]) {
                    "jpg"
                } else {
                    "png"
                };
                let chemin = fichiers.ajouter(extension, f.image.clone());
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
    // Le gabarit peut avoir été extrait avec des fins de ligne Windows.
    let mut s = GABARIT.replace("\r\n", "\n");
    s.push('\n');
    s.push_str(SEPARATION);
    let _ = writeln!(
        s,
        "#set document(title: {})",
        chaine_typst(&format!("{} {}", c.titre, c.numero))
    );
    let _ = writeln!(s, "#set text(lang: \"{langue}\")");
    // Le cartouche du document, avec le numéro de page qu'on lui donne.
    let _ = writeln!(s, "#let cartouche-document(numero-page) = cartouche(");
    let _ = writeln!(
        s,
        "  (nom: {}, adresse: {}, courriel: {}, telephone: {}, site: {}, logo: {logo}),",
        coupable(&a.nom),
        coupable(&a.adresse),
        coupable(&a.courriel),
        coupable(&a.telephone),
        coupable(&a.site),
    );
    let _ = writeln!(
        s,
        "  {}, {}, {}, {}, {},",
        coupable(&c.client),
        coupable(&c.objet),
        liste(&c.lignes, |(k, v)| format!(
            "({}, {})",
            chaine_typst(k),
            coupable(v)
        )),
        chaine_typst(&c.numero),
        coupable(&c.titre),
    );
    let _ = writeln!(
        s,
        "  (atelier: {}, client: {}, objet: {}, numero: {}),\n  numero-page)",
        chaine_typst(&l.atelier),
        chaine_typst(&l.client),
        chaine_typst(&l.objet),
        chaine_typst(&l.numero),
    );
    // La marge du bas est la hauteur du cartouche, mesurée avec le plus long
    // numéro de page plausible, plus 6 mm au-dessus de lui et 12 mm sous lui.
    let page = chaine_typst(&l.page);
    let _ = writeln!(
        s,
        "#show: corps => context {{\n  \
         let haut = measure(block(width: 180mm, cartouche-document({page}.replace(\"{{n}}\", \"999\").replace(\"{{total}}\", \"999\")))).height\n  \
         set page(paper: \"a4\", margin: (top: 15mm, x: 15mm, bottom: haut + 18mm), footer-descent: 6mm,\n    \
         footer: context cartouche-document({page}.replace(\"{{n}}\", str(counter(page).get().first())).replace(\"{{total}}\", str(counter(page).final().first()))))\n  \
         corps\n}}"
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
