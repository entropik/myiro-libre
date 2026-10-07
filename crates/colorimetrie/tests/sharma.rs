//! ΔE00 contre les 34 paires de test de Sharma, Wu et Dalal (2005).
//!
//! Le fichier de test n'est pas versionné : sa page de publication autorise
//! l'usage personnel et de recherche avec citation, sans droit de
//! redistribution explicite (voir `docs/references/icc-delta-e.md`). Pour lancer
//! ce test, télécharger `ciede2000testdata.txt` depuis
//! <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/> et le poser dans
//! `crates/colorimetrie/tests/donnees-locales/` (ignoré par git), ou donner son
//! chemin dans la variable d'environnement `SHARMA_CIEDE2000`. Sans fichier, le
//! test s'arrête en le signalant.

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
fn delta_e00_reproduit_les_34_paires_de_sharma() {
    let chemin = chemin();
    let Ok(texte) = std::fs::read_to_string(&chemin) else {
        eprintln!(
            "jeu de Sharma absent ({}) : test non joué",
            chemin.display()
        );
        return;
    };
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
