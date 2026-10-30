//! Options de lancement qui choisissent la bibliothèque ouverte, lues sans
//! Tauri (ticket #86).

use std::path::{Path, PathBuf};

/// Option qui désigne la bibliothèque d'essai.
const OPTION_BIBLIOTHEQUE: &str = "--bibliotheque";

/// Option qui ouvre la bibliothèque de démonstration.
const OPTION_DEMO: &str = "--demo";

/// Bibliothèque à ouvrir au lancement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibliothequeDemandee {
    /// Celle du poste, dans le dossier de données de l'application.
    Poste,
    /// Celle de démonstration (`--demo`) ; `essai_ecarte` est la bibliothèque
    /// d'essai demandée en même temps, qui n'est pas ouverte.
    Demonstration { essai_ecarte: Option<PathBuf> },
    /// Une bibliothèque d'essai, dans ce dossier.
    Essai(PathBuf),
}

/// Lancement refusé à cause d'une option mal écrite : l'application s'arrête
/// au lieu d'ouvrir la bibliothèque du poste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusLancement {
    /// `--bibliotheque` sans dossier.
    BibliothequeSansDossier,
    /// Option proche de `--bibliotheque` mais autrement écrite.
    OptionMalEcrite(String),
    /// `--bibliotheque` donnée plus d'une fois.
    BibliothequeRepetee,
    /// Le dossier demandé est celui de la bibliothèque du poste.
    BibliothequeDuPoste(PathBuf),
}

impl std::fmt::Display for RefusLancement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefusLancement::BibliothequeSansDossier => {
                write!(f, "option --bibliotheque sans dossier")?
            }
            RefusLancement::OptionMalEcrite(option) => write!(f, "option mal écrite : {option}")?,
            RefusLancement::BibliothequeRepetee => {
                write!(f, "option --bibliotheque donnée plusieurs fois")?
            }
            RefusLancement::BibliothequeDuPoste(dossier) => write!(
                f,
                "{} est la bibliothèque du poste, pas une copie",
                dossier.display()
            )?,
        }
        write!(
            f,
            " : écrire --bibliotheque <dossier> une seule fois, avec le dossier d'une copie \
             (la bibliothèque du poste n'est pas ouverte à la place)"
        )
    }
}

/// Un argument qui ressemble à l'option de la bibliothèque d'essai : sans ses
/// tirets de tête, en minuscules et sans accents, il commence par
/// « bibliotheque ».
fn ressemble_a_l_option(arg: &str) -> bool {
    let sans_accents: String = arg
        .trim_start_matches(['-', '–', '—'])
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            c => c,
        })
        .collect();
    sans_accents.starts_with("bibliotheque")
}

/// Bibliothèque demandée par les arguments du lancement (le premier est le
/// programme) : `--demo` l'emporte sur `--bibliotheque <dossier>` ; sans
/// l'une ni l'autre, celle du poste. `--bibliotheque` sans dossier, répétée
/// ou mal écrite est refusée, jamais remplacée par celle du poste.
pub fn bibliotheque_demandee(args: &[String]) -> Result<BibliothequeDemandee, RefusLancement> {
    let mut essai = None;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        let valeur = if arg == OPTION_BIBLIOTHEQUE {
            // Le dossier qui suit n'est pas une option, quel que soit son nom.
            i += 1;
            args.get(i - 1).map(String::as_str)
        } else if let Some(valeur) = arg
            .strip_prefix(OPTION_BIBLIOTHEQUE)
            .and_then(|reste| reste.strip_prefix('='))
        {
            Some(valeur)
        } else if ressemble_a_l_option(arg) {
            return Err(RefusLancement::OptionMalEcrite(arg.clone()));
        } else {
            continue;
        };
        let dossier = match valeur {
            Some(dossier) if !dossier.trim().is_empty() && !dossier.starts_with("--") => dossier,
            _ => return Err(RefusLancement::BibliothequeSansDossier),
        };
        if essai.replace(PathBuf::from(dossier)).is_some() {
            return Err(RefusLancement::BibliothequeRepetee);
        }
    }
    Ok(if args.iter().skip(1).any(|a| a == OPTION_DEMO) {
        BibliothequeDemandee::Demonstration {
            essai_ecarte: essai,
        }
    } else {
        essai.map_or(BibliothequeDemandee::Poste, BibliothequeDemandee::Essai)
    })
}

/// Refuse comme bibliothèque d'essai le dossier de la bibliothèque du poste,
/// même écrit autrement. Les deux chemins sont comparés une fois résolus,
/// quand ils existent tous deux : un dossier encore absent sera créé, ce
/// n'est donc pas celui du poste.
pub fn refuser_bibliotheque_du_poste(essai: &Path, poste: &Path) -> Result<(), RefusLancement> {
    match (std::fs::canonicalize(essai), std::fs::canonicalize(poste)) {
        (Ok(essai_resolu), Ok(poste_resolu)) if essai_resolu == poste_resolu => {
            Err(RefusLancement::BibliothequeDuPoste(essai.to_path_buf()))
        }
        _ => Ok(()),
    }
}
