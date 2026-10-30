//! Moteur de rapport de myiro-libre (ticket #89, ADR 0008).
//!
//! Un rapport est un [`Document`] : un cartouche d'architecte (atelier,
//! client, objet, papier, profil, date, numéro), puis des sections faites de
//! blocs. Il s'écrit en source Typst ([`source_typst`]), où chaque texte est
//! un littéral de chaîne et jamais du balisage, puis se compile en PDF en
//! mémoire. La crate ne dépend ni de Windows, ni de Tauri, ni des ponts : le
//! même moteur sert au rapport d'une image, à la vérification d'un profil,
//! au contrôle et aux mesures.
//!
//! Un rapport est dans une seule langue, ne porte aucun identifiant
//! d'instrument, et ne lit aucun fichier hors de ceux qu'on lui donne.

mod pdf;
mod source;
mod svg;

pub use pdf::{pdf, ErreurRapport, PagePdf, Pdf};
pub use source::{chaine_typst, source_typst, SourceTypst};
pub use svg::{nettoyer_svg, RefusSvg};

/// Coordonnées de l'atelier, reprises dans le cartouche de chaque rapport.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Atelier {
    pub nom: String,
    /// Sur plusieurs lignes.
    pub adresse: String,
    pub courriel: String,
    pub telephone: String,
    pub site: String,
    /// Logo déjà contrôlé et réencodé en PNG.
    pub logo_png: Option<Vec<u8>>,
}

/// Le cartouche d'architecte, en pied de chaque page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cartouche {
    pub atelier: Atelier,
    pub client: String,
    pub objet: String,
    /// Papier, profil, date… : libellé et valeur, déjà traduits.
    pub lignes: Vec<(String, String)>,
    pub numero: String,
    pub titre: String,
}

/// Libellés fixes du cartouche, dans la langue du rapport.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Libelles {
    /// Avec `{n}` (page) et `{total}` (pages) : « Page {n} / {total} ».
    pub page: String,
    pub atelier: String,
    pub client: String,
    pub objet: String,
    pub numero: String,
}

/// Un rapport complet, dans une seule langue.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    /// `"fr"` ou `"en"` ; toute autre valeur est lue comme `"fr"`.
    pub langue: String,
    pub cartouche: Cartouche,
    pub sections: Vec<Section>,
    pub libelles: Libelles,
}

/// Une partie du rapport : un titre, puis ses blocs.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub titre: String,
    pub blocs: Vec<Bloc>,
}

/// Un bloc de contenu. Tous sont insécables sauf le tableau, le paragraphe
/// et le texte préformaté.
#[derive(Clone, Debug, PartialEq)]
pub enum Bloc {
    /// La phrase de verdict en grand, son détail en petit dessous.
    Verdict {
        texte: String,
        detail: String,
    },
    Paragraphe(String),
    /// Libellé et valeur.
    Couples(Vec<(String, String)>),
    /// `nombres[i]` : la colonne `i` est alignée à droite, en chiffres
    /// tabulaires.
    Tableau {
        entetes: Vec<String>,
        lignes: Vec<Vec<String>>,
        nombres: Vec<bool>,
    },
    /// Images côte à côte.
    Images(Vec<Figure>),
    /// Pastilles Image / Imprimé.
    Paires(Vec<PaireCouleurs>),
    /// Graphique SVG ; repassé par [`nettoyer_svg`], un graphique refusé ne
    /// garde que sa légende.
    Svg {
        svg: String,
        legende: String,
    },
    /// Texte gardé tel quel, ligne par ligne.
    Pre(String),
}

/// Une image PNG et sa légende.
#[derive(Clone, Debug, PartialEq)]
pub struct Figure {
    pub png: Vec<u8>,
    pub legende: String,
    /// Largeur sur la page, bornée de 10 à 180 mm.
    pub largeur_mm: f32,
}

/// Une couleur de l'image et sa couleur imprimée, côte à côte.
#[derive(Clone, Debug, PartialEq)]
pub struct PaireCouleurs {
    /// `#rrggbb` ; une autre forme laisse le carré vide.
    pub image: String,
    pub imprime: String,
    pub titre: String,
    pub lignes: Vec<String>,
}
