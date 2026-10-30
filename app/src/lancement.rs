//! Options de lancement qui choisissent la bibliothèque ouverte, lues sans
//! Tauri (ticket #86).

use std::path::PathBuf;

/// Bibliothèque à ouvrir au lancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibliothequeDemandee {
    /// Celle du poste, dans le dossier de données de l'application.
    Poste,
    /// Celle de démonstration (`--demo`).
    Demonstration { essai_ecarte: Option<PathBuf> },
    /// Une bibliothèque d'essai, dans ce dossier.
    Essai(PathBuf),
}

/// Lancement refusé à cause d'une option mal écrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusLancement {
    BibliothequeSansDossier,
}

impl std::fmt::Display for RefusLancement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefusLancement::BibliothequeSansDossier => write!(
                f,
                "option --bibliotheque sans dossier : écrire --bibliotheque <dossier> \
                 (la bibliothèque du poste n'est pas ouverte à la place)"
            ),
        }
    }
}

/// Option qui désigne la bibliothèque d'essai.
const OPTION_BIBLIOTHEQUE: &str = "--bibliotheque";

/// Bibliothèque demandée par les arguments du lancement : `--demo` l'emporte
/// sur `--bibliotheque <dossier>` ; sans l'une ni l'autre, celle du poste.
/// `--bibliotheque` sans dossier est refusée, jamais remplacée par celle du
/// poste.
pub fn bibliotheque_demandee(args: &[String]) -> Result<BibliothequeDemandee, RefusLancement> {
    let mut essai = None;
    for (i, arg) in args.iter().enumerate() {
        let valeur = if arg == OPTION_BIBLIOTHEQUE {
            args.get(i + 1).map(String::as_str)
        } else if let Some(valeur) = arg
            .strip_prefix(OPTION_BIBLIOTHEQUE)
            .and_then(|reste| reste.strip_prefix('='))
        {
            Some(valeur)
        } else {
            continue;
        };
        match valeur {
            Some(dossier) if !dossier.trim().is_empty() && !dossier.starts_with("--") => {
                essai.get_or_insert(PathBuf::from(dossier));
            }
            _ => return Err(RefusLancement::BibliothequeSansDossier),
        }
    }
    Ok(if args.iter().any(|a| a == "--demo") {
        BibliothequeDemandee::Demonstration {
            essai_ecarte: essai,
        }
    } else {
        essai.map_or(BibliothequeDemandee::Poste, BibliothequeDemandee::Essai)
    })
}
