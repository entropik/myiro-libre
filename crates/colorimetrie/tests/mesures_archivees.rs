//! Notre Lab confronté au Lab calculé par la DLL du MYIRO-1, sur les mesures
//! réelles archivées localement.
//!
//! Les mesures ne sont pas versionnées (`Archivage/donnees/`). Le test lit les
//! fichiers `.csv` écrits par les tests `--ignored` du pont
//! (`plage;donnees;L;a;b;nm380;…;nm730`), sous `Archivage/donnees/` à la racine
//! du dépôt ou dans le dossier donné par la variable `MESURES_ARCHIVEES`. Sans
//! mesures, il s'arrête en le signalant.

use std::path::{Path, PathBuf};

use colorimetrie::{delta_e00, spectre_vers_lab, Lab, LONGUEUR_SPECTRE};

/// Écart maximal admis entre notre Lab et celui de la DLL.
const ECART_MAXIMAL: f64 = 0.05;

fn dossier() -> PathBuf {
    std::env::var_os("MESURES_ARCHIVEES").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Archivage/donnees"),
        PathBuf::from,
    )
}

fn fichiers_csv(dossier: &Path, trouves: &mut Vec<PathBuf>) {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            fichiers_csv(&chemin, trouves);
        } else if chemin.extension().is_some_and(|e| e == "csv") {
            trouves.push(chemin);
        }
    }
}

/// Lab de la DLL et spectre de chaque ligne M0, M1 ou M2 d'un fichier du pont.
fn lignes_du_pont(texte: &str) -> Vec<(Lab, Vec<f64>)> {
    let mut lignes = texte.lines();
    let Some(entete) = lignes.next() else {
        return Vec::new();
    };
    let colonnes: Vec<&str> = entete.split(';').collect();
    if colonnes.get(1) != Some(&"donnees") || colonnes.get(5) != Some(&"nm380") {
        return Vec::new();
    }
    lignes
        .filter_map(|ligne| {
            let champs: Vec<&str> = ligne.split(';').collect();
            if !matches!(champs.get(1), Some(&"M0" | &"M1" | &"M2")) {
                return None;
            }
            let nombres: Vec<f64> = champs[2..].iter().filter_map(|c| c.parse().ok()).collect();
            if nombres.len() != 3 + LONGUEUR_SPECTRE {
                return None;
            }
            let lab = Lab {
                l: nombres[0],
                a: nombres[1],
                b: nombres[2],
            };
            Some((lab, nombres[3..].to_vec()))
        })
        .collect()
}

#[test]
fn notre_lab_rejoint_celui_de_la_dll_sur_les_mesures_archivees() {
    let mut fichiers = Vec::new();
    fichiers_csv(&dossier(), &mut fichiers);
    let mesures: Vec<(Lab, Vec<f64>)> = fichiers
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .flat_map(|texte| lignes_du_pont(&texte))
        .collect();
    if mesures.is_empty() {
        eprintln!(
            "aucune mesure archivée sous {} : test non joué",
            dossier().display()
        );
        return;
    }
    let ecarts: Vec<f64> = mesures
        .iter()
        .map(|(dll, spectre)| delta_e00(*dll, spectre_vers_lab(spectre).unwrap()).unwrap())
        .collect();
    let moyen = ecarts.iter().sum::<f64>() / ecarts.len() as f64;
    let maximal = ecarts.iter().copied().fold(0.0, f64::max);
    eprintln!(
        "{} spectres ({} fichiers) : ΔE00 moyen {moyen:.4}, maximal {maximal:.4}",
        ecarts.len(),
        fichiers.len()
    );
    assert!(maximal <= ECART_MAXIMAL, "ΔE00 maximal {maximal:.4}");
}
