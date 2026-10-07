//! ΔE00 contre les 34 paires de test de Sharma, Wu et Dalal (2005).
//!
//! Le fichier de test n'est pas versionné : sa page de publication autorise
//! l'usage personnel et de recherche avec citation, sans droit de
//! redistribution explicite (voir `docs/references/icc-delta-e.md`). Pour lancer
//! ce test, télécharger `ciede2000testdata.txt` depuis
//! <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/> et le poser dans
//! `crates/colorimetrie/tests/donnees-locales/` (ignoré par git), ou donner son
//! chemin dans la variable d'environnement `SHARMA_CIEDE2000`. Le test est ignoré
//! par défaut (`cargo test -p colorimetrie -- --ignored` pour le lancer) et
//! échoue si le fichier manque.

use std::path::PathBuf;

use colorimetrie::{delta_e00, Lab};

fn chemin() -> PathBuf {
    std::env::var_os("SHARMA_CIEDE2000").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/donnees-locales/ciede2000testdata.txt")
        },
        PathBuf::from,
    )
}

#[test]
#[ignore = "lit le jeu de Sharma local (tests/donnees-locales/ ou SHARMA_CIEDE2000), non versionné"]
fn delta_e00_reproduit_les_34_paires_de_sharma() {
    let chemin = chemin();
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|_| {
        panic!(
            "jeu de Sharma absent ({}) : le télécharger (voir l'en-tête du fichier)",
            chemin.display()
        )
    });
    let mut paires = 0;
    for ligne in texte.lines().filter(|l| !l.trim().is_empty()) {
        let v: Vec<f64> = ligne
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 7, "ligne mal formée : {ligne}");
        let e = delta_e00(
            Lab {
                l: v[0],
                a: v[1],
                b: v[2],
            },
            Lab {
                l: v[3],
                a: v[4],
                b: v[5],
            },
        )
        .unwrap();
        assert!(
            (e - v[6]).abs() <= 5e-5,
            "paire {} : obtenu {e:.4}, publié {}",
            paires + 1,
            v[6]
        );
        paires += 1;
    }
    assert_eq!(paires, 34);
}
