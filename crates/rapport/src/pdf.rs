//! Compilation d'un [`Document`] en PDF, en mémoire, par Typst employé comme
//! bibliothèque. Le « monde » de Typst ne connaît que la source du document,
//! ses images et les deux polices Inter embarquées (SIL OFL 1.1,
//! `fontes/Inter-OFL.txt`) : aucun fichier du disque, aucune police du
//! système, aucun paquet téléchargé.

use std::sync::{LazyLock, Mutex};

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Abs, Frame, FrameItem};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

use crate::{source_typst, Document};

const POLICES: [&[u8]; 2] = [
    include_bytes!("../fontes/Inter-Regular.ttf"),
    include_bytes!("../fontes/Inter-SemiBold.ttf"),
];

/// Ce qui ne change pas d'un rapport à l'autre.
struct Commun {
    bibliotheque: LazyHash<Library>,
    livre: LazyHash<FontBook>,
    polices: Vec<Font>,
}

static COMMUN: LazyLock<Commun> = LazyLock::new(|| {
    let polices: Vec<Font> = POLICES
        .iter()
        .flat_map(|octets| Font::iter(Bytes::new(octets.to_vec())))
        .collect();
    Commun {
        bibliotheque: LazyHash::new(Library::default()),
        livre: LazyHash::new(FontBook::from_fonts(&polices)),
        polices,
    }
});

/// Une compilation à la fois : le cache de Typst est vidé après chacune.
static COMPILATION: Mutex<()> = Mutex::new(());

/// Pourquoi un PDF n'a pas pu être fait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurRapport {
    /// Typst a refusé la source ou une image (message de Typst).
    Compilation(String),
    /// L'écriture du PDF a échoué (message de Typst).
    Export(String),
}

/// Une page du PDF : son format et son texte, dans l'ordre de dessin.
#[derive(Clone, Debug, PartialEq)]
pub struct PagePdf {
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
    /// Morceaux de texte de la page, un par ligne.
    pub texte: String,
}

/// Un rapport en PDF.
#[derive(Clone, Debug, PartialEq)]
pub struct Pdf {
    pub octets: Vec<u8>,
    pub pages: Vec<PagePdf>,
}

fn identifiant(chemin: &str) -> Option<FileId> {
    let chemin = VirtualPath::new(chemin).ok()?;
    Some(RootedPath::new(VirtualRoot::Project, chemin).intern())
}

struct Monde {
    source: Source,
    fichiers: Vec<(FileId, Bytes)>,
}

impl World for Monde {
    fn library(&self) -> &LazyHash<Library> {
        &COMMUN.bibliotheque
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &COMMUN.livre
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.fichiers
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, octets)| octets.clone())
            .ok_or(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        COMMUN.polices.get(index).cloned()
    }

    /// La date vient du document, jamais de l'horloge.
    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn messages(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn textes(frame: &Frame, sortie: &mut Vec<String>) {
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(groupe) => textes(&groupe.frame, sortie),
            FrameItem::Text(t) => sortie.push(t.text.to_string()),
            _ => {}
        }
    }
}

fn millimetres(a: Abs) -> f64 {
    a.to_mm()
}

/// Le PDF du document, compilé en mémoire.
pub fn pdf(d: &Document) -> Result<Pdf, ErreurRapport> {
    let s = source_typst(d);
    let principal = identifiant("/rapport.typ").expect("chemin fixe valide");
    let mut fichiers = Vec::new();
    for (chemin, octets) in s.fichiers {
        let id = identifiant(&chemin)
            .ok_or_else(|| ErreurRapport::Compilation(format!("chemin {chemin}")))?;
        fichiers.push((id, Bytes::new(octets)));
    }
    let monde = Monde {
        source: Source::new(principal, s.texte),
        fichiers,
    };
    let _verrou = COMPILATION.lock().unwrap_or_else(|e| e.into_inner());
    let compile = typst::compile::<PagedDocument>(&monde).output;
    let resultat = compile
        .map_err(|e| ErreurRapport::Compilation(messages(&e)))
        .and_then(|document| {
            let octets = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .map_err(|e| ErreurRapport::Export(messages(&e)))?;
            let pages = document
                .pages()
                .iter()
                .map(|p| {
                    let mut morceaux = Vec::new();
                    textes(&p.frame, &mut morceaux);
                    PagePdf {
                        largeur_mm: millimetres(p.frame.size().x),
                        hauteur_mm: millimetres(p.frame.size().y),
                        texte: morceaux.join("\n"),
                    }
                })
                .collect();
            Ok(Pdf { octets, pages })
        });
    typst::comemo::evict(0);
    resultat
}
