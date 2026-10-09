//! La page de l'application, lue comme un fichier : ce que le navigateur
//! ferait de ses attributs et du système graphique.

use std::collections::BTreeSet;
use std::path::Path;

fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif)).unwrap()
}

/// Classes du système graphique dont une règle à elles seules fixe `display`.
fn classes_qui_fixent_display(css: &str) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    for regle in css.split('}') {
        let Some((selecteurs, corps)) = regle.split_once('{') else {
            continue;
        };
        if !corps.contains("display:") {
            continue;
        }
        // Retire les commentaires qui précèdent la règle.
        let selecteurs = selecteurs.rsplit("*/").next().unwrap_or(selecteurs);
        for selecteur in selecteurs.split(',') {
            let s = selecteur.trim();
            if let Some(nom) = s.strip_prefix('.') {
                if nom.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    classes.insert(nom.to_string());
                }
            }
        }
    }
    classes
}

/// Un élément caché par l'attribut `hidden` doit vraiment l'être : une classe
/// qui fixe `display` (`.actions`, `.seg`…) l'emporte sur `hidden` dans le
/// navigateur, et l'élément resterait visible. On cache alors un enveloppant
/// sans classe.
#[test]
fn un_element_cache_ne_porte_pas_de_classe_qui_fixe_display() {
    let classes = classes_qui_fixent_display(&lire("../design-system/components.css"));
    assert!(classes.contains("actions"), "{classes:?}");
    let page = lire("interface/index.html");

    let mut fautes = Vec::new();
    for balise in page.split('<').skip(1) {
        let balise = balise.split('>').next().unwrap_or_default();
        let cachee = balise
            .split_whitespace()
            .any(|attribut| attribut == "hidden" || attribut == "hidden/");
        if !cachee {
            continue;
        }
        let Some((_, reste)) = balise.split_once("class=\"") else {
            continue;
        };
        let valeur = reste.split('"').next().unwrap_or_default();
        if valeur.split_whitespace().any(|c| classes.contains(c)) {
            fautes.push(balise.to_string());
        }
    }
    assert!(fautes.is_empty(), "visibles malgré hidden : {fautes:?}");
}
