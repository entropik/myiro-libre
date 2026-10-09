//! Nos densités confrontées à celles de `spotread -d` (ArgyllCMS) et à celles
//! d'un logiciel de référence, sur des relevés locaux non versionnés.
//!
//! Ignorés par défaut ; `cargo test -p colorimetrie --test densite_comparaisons
//! -- --ignored --nocapture` pour les lancer. Chacun échoue s'il ne trouve
//! aucun relevé.
//!
//! - `SPOTREAD_DENSITES` : fichier texte où l'on a recopié la sortie de
//!   `spotread -d -s` (une ou plusieurs lectures). Seules les lectures dont le
//!   spectre va de 380 à 730 nm en 36 pas sont comparées.
//! - `DENSITES_REFERENCE` : fichier `;`-séparé, une plage par ligne,
//!   `statut;cyan;magenta;jaune;visuel;nm380;…;nm730`, statut `E` ou `T`,
//!   densités absolues, spectre en facteurs de 0 à 1 (une ligne d'en-tête
//!   non numérique est sautée).

use colorimetrie::{densites, Densites, StatutDensite, LONGUEUR_SPECTRE};

/// Écart admis avec `spotread -d` : même méthode et mêmes tables, il ne
/// reste que l'arrondi de l'affichage (6 décimales).
const ECART_SPOTREAD: f64 = 1e-4;

/// Écart admis avec le logiciel de référence, dont la méthode n'est pas
/// connue : 0,02, l'ordre de l'accord entre deux densitomètres [supposé].
const ECART_REFERENCE: f64 = 0.02;

fn lire(variable: &str) -> String {
    let chemin = std::env::var_os(variable)
        .unwrap_or_else(|| panic!("donner le fichier des relevés dans {variable}"));
    std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("{} illisible : {e}", chemin.to_string_lossy()))
}

fn nombres(texte: &str, separateur: char) -> Vec<f64> {
    texte
        .split(separateur)
        .filter_map(|n| n.trim().parse().ok())
        .collect()
}

fn tableau(d: Densites) -> [f64; 4] {
    [d.cyan, d.magenta, d.jaune, d.visuel]
}

/// Écart maximal entre nos densités et celles attendues, plage par plage.
fn ecart_maximal(releves: &[(StatutDensite, [f64; 4], Vec<f64>)]) -> f64 {
    let mut maximal: f64 = 0.0;
    for (statut, attendues, spectre) in releves {
        let obtenues = tableau(densites(spectre, *statut).expect("densité calculable"));
        for (o, a) in obtenues.iter().zip(attendues) {
            maximal = maximal.max((o - a).abs());
        }
        eprintln!("{statut:?} attendu {attendues:?}, obtenu {obtenues:?}");
    }
    maximal
}

/// Lectures de `spotread -d -s` : spectre (en pourcentage dans sa sortie) et
/// densités CMYV des statuts T et E.
fn lectures_spotread(texte: &str) -> Vec<(StatutDensite, [f64; 4], Vec<f64>)> {
    let mut releves = Vec::new();
    let mut spectre: Option<Vec<f64>> = None;
    let mut lignes = texte.lines();
    while let Some(ligne) = lignes.next() {
        let ligne = ligne.trim();
        if ligne.starts_with("Spectrum from") {
            let bon = ligne.contains("380.000 to 730.000 nm in 36 steps");
            let valeurs = lignes.next().map(|l| nombres(l, ',')).unwrap_or_default();
            spectre = (bon && valeurs.len() == LONGUEUR_SPECTRE)
                .then(|| valeurs.iter().map(|v| v / 100.0).collect());
            continue;
        }
        let statut = if ligne.starts_with("Status T CMYV Density:") {
            StatutDensite::T
        } else if ligne.starts_with("Status E CMYV Density:") {
            StatutDensite::E
        } else {
            continue;
        };
        let (Some(s), Some((_, valeurs))) = (&spectre, ligne.split_once(':')) else {
            continue;
        };
        let d = nombres(valeurs, ' ');
        if let [c, m, j, v] = d[..] {
            releves.push((statut, [c, m, j, v], s.clone()));
        }
    }
    releves
}

#[test]
#[ignore = "lit une sortie locale de spotread -d -s (SPOTREAD_DENSITES), non versionnée"]
fn nos_densites_rejoignent_celles_de_spotread() {
    let releves = lectures_spotread(&lire("SPOTREAD_DENSITES"));
    assert!(!releves.is_empty(), "aucune lecture 380-730 nm en 36 pas");
    let maximal = ecart_maximal(&releves);
    eprintln!(
        "{} densités CMYV comparées : écart maximal {maximal:.6}",
        releves.len()
    );
    assert!(maximal <= ECART_SPOTREAD, "écart maximal {maximal:.6}");
}

#[test]
#[ignore = "lit des densités locales d'un logiciel de référence (DENSITES_REFERENCE), non versionnées"]
fn nos_densites_rejoignent_celles_du_logiciel_de_reference() {
    let releves: Vec<_> = lire("DENSITES_REFERENCE")
        .lines()
        .filter_map(|ligne| {
            let (statut, reste) = ligne.split_once(';')?;
            let statut = match statut.trim() {
                "E" => StatutDensite::E,
                "T" => StatutDensite::T,
                _ => return None,
            };
            let n = nombres(reste, ';');
            (n.len() == 4 + LONGUEUR_SPECTRE)
                .then(|| (statut, [n[0], n[1], n[2], n[3]], n[4..].to_vec()))
        })
        .collect();
    assert!(!releves.is_empty(), "aucune plage lisible");
    let maximal = ecart_maximal(&releves);
    eprintln!(
        "{} plages comparées : écart maximal {maximal:.4}",
        releves.len()
    );
    assert!(maximal <= ECART_REFERENCE, "écart maximal {maximal:.4}");
}

#[test]
fn la_sortie_de_spotread_est_lue() {
    // Extrait de la forme imprimée par spotread.c (ArgyllCMS 3.2.0) ;
    // valeurs fictives.
    let mut sortie =
        String::from(" Result is XYZ: 1 2 3\nSpectrum from 380.000 to 730.000 nm in 36 steps\n");
    sortie.push_str(&vec!["50"; 36].join(", "));
    sortie.push_str("\nPeak value 50.000000 at (aprox.) 380.0 nm\n");
    sortie.push_str("ISO Vis, Type 1, Type 2 Density: 0.3 0.3 0.3\n");
    sortie.push_str("Status T CMYV Density: 0.301030 0.301030 0.301030 0.301030\n");
    sortie.push_str("Status E CMYV Density: 0.301030 0.301030 0.301030 0.301030\n");
    sortie.push_str("Spectrum from 400.000 to 700.000 nm in 31 steps\n");
    sortie.push_str(&vec!["50"; 31].join(", "));
    sortie.push_str("\nStatus T CMYV Density: 1 1 1 1\n");
    let releves = lectures_spotread(&sortie);
    assert_eq!(releves.len(), 2, "la lecture 400-700 nm est sautée");
    assert_eq!(releves[0].0, StatutDensite::T);
    assert_eq!(releves[1].0, StatutDensite::E);
    assert_eq!(releves[0].2, vec![0.5; 36]);
    assert!(ecart_maximal(&releves) <= ECART_SPOTREAD);
}
