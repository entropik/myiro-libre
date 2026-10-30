//! Les parties de l'accordéon reprises dans le rapport : les chiffres clés
//! des profils (toujours), puis les parties cochées, dans l'ordre de la vue.
//! Leur graphique vient de la page : SVG (passé par la liste blanche de la
//! crate `rapport`), PNG (vue 3D) ou tableau de textes. Une partie inconnue
//! est ignorée ; un graphique refusé est remplacé par une phrase.

use base64::Engine;
use rapport::{Bloc, Section};
use serde::Deserialize;

use super::{t, DemandeRapportImage};
use crate::textes::Langue;

/// Parties reprises, dans l'ordre de la vue « Comparer ». La première (les
/// chiffres clés) est toujours reprise ; les autres seulement cochées.
pub const PARTIES: [&str; 7] = [
    "comparer.chiffres",
    "comparer.volume",
    "comparer.teintes",
    "comparer.coupe",
    "comparer.chemin",
    "comparer.primaires",
    "comparer.ecarts",
];

/// Taille d'un PNG reçu au plus, en base 64.
const BASE64_MAX: usize = 24 << 20;
/// Bornes d'un tableau reçu : colonnes, lignes, caractères par cellule.
const COLONNES_MAX: usize = 8;
const LIGNES_MAX: usize = 60;
const CARACTERES_MAX: usize = 400;

/// Un tableau de la page, en textes.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TableauPage {
    pub entetes: Vec<String>,
    pub lignes: Vec<Vec<String>>,
}

/// Un morceau d'une partie, tel que la page l'a dessiné : un texte qui situe
/// ce qui suit (« Écarts de B par rapport à A », sous-titre), un tableau, un
/// graphique SVG ou une image PNG, avec sa légende.
#[derive(Clone, Debug, Deserialize)]
pub struct GraphiquePartie {
    pub partie: String,
    pub svg: Option<String>,
    pub png_base64: Option<String>,
    pub tableau: Option<TableauPage>,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub legende: Option<String>,
}

fn cellule(c: &str) -> String {
    c.chars().take(CARACTERES_MAX).collect()
}

fn png(base64: &str) -> Option<Vec<u8>> {
    if base64.len() > BASE64_MAX {
        return None;
    }
    let octets = base64::engine::general_purpose::STANDARD
        .decode(base64.trim_start_matches("data:image/png;base64,"))
        .ok()?;
    image::load_from_memory_with_format(&octets, image::ImageFormat::Png).ok()?;
    Some(octets)
}

fn blocs(g: &GraphiquePartie, langue: Langue) -> Vec<Bloc> {
    let refuse = || vec![Bloc::Paragraphe(t(langue, "rapport.graphique_refuse"))];
    let legende = g.legende.as_deref().map(cellule).unwrap_or_default();
    if let Some(texte) = &g.texte {
        return vec![Bloc::Paragraphe(cellule(texte))];
    }
    if let Some(tableau) = &g.tableau {
        let entetes: Vec<String> = tableau
            .entetes
            .iter()
            .take(COLONNES_MAX)
            .map(|c| cellule(c))
            .collect();
        let lignes = tableau
            .lignes
            .iter()
            .take(LIGNES_MAX)
            .map(|l| l.iter().take(COLONNES_MAX).map(|c| cellule(c)).collect())
            .collect();
        let mut nombres = vec![true; entetes.len()];
        if let Some(premiere) = nombres.first_mut() {
            *premiere = false;
        }
        return vec![Bloc::Tableau {
            entetes,
            lignes,
            nombres,
        }];
    }
    if let Some(svg) = &g.svg {
        return match rapport::nettoyer_svg(svg) {
            Ok(svg) => vec![Bloc::Svg { svg, legende }],
            Err(_) => refuse(),
        };
    }
    match g.png_base64.as_deref().and_then(png) {
        Some(png) => vec![Bloc::Images(vec![rapport::Figure {
            png,
            legende,
            largeur_mm: 140.0,
        }])],
        None => refuse(),
    }
}

/// Les sections des parties reprises, dans l'ordre de [`PARTIES`].
pub(super) fn sections(d: &DemandeRapportImage, langue: Langue) -> Vec<Section> {
    PARTIES
        .iter()
        .enumerate()
        .filter(|(i, cle)| *i == 0 || d.parties.iter().any(|p| p == *cle))
        .filter_map(|(_, cle)| {
            // Une partie peut avoir plusieurs graphiques ou tableaux (chemin, écarts).
            let blocs: Vec<Bloc> = d
                .graphiques
                .iter()
                .filter(|g| g.partie == *cle)
                .flat_map(|g| blocs(g, langue))
                .collect();
            (!blocs.is_empty()).then(|| Section {
                titre: t(langue, cle),
                blocs,
            })
        })
        .collect()
}
