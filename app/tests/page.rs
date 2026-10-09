//! La page de l'application, lue comme un fichier : ce que le navigateur
//! ferait de ses attributs et du système graphique.

use std::collections::BTreeSet;
use std::io::Write;
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
                if nom
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    classes.insert(nom.to_string());
                }
            }
        }
    }
    classes
}

/// Les modules de la page testés par leurs données (`tests/interface/*.test.js`),
/// avec le lanceur de tests de Node, sans npm ni dépendance. Sans Node sur le
/// poste, le contrôle est sauté et annoncé ; en intégration continue (variable
/// `CI`), Node est exigé et son absence fait échouer le test.
#[test]
fn les_modules_de_la_page_passent_leurs_tests() {
    let dossier = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interface");
    let mut fichiers: Vec<_> = std::fs::read_dir(&dossier)
        .unwrap()
        .map(|f| f.unwrap().path())
        .filter(|f| f.to_string_lossy().ends_with(".test.js"))
        .collect();
    fichiers.sort();
    assert!(!fichiers.is_empty());
    let sortie = match std::process::Command::new("node")
        .arg("--test")
        .args(&fichiers)
        .output()
    {
        Ok(sortie) => sortie,
        Err(e) => {
            let avis = sans_node(std::env::var_os("CI").is_some(), &e.to_string())
                .unwrap_or_else(|faute| panic!("{faute}"));
            // Écrit hors de la capture de `cargo test` : le saut se voit.
            let _ = writeln!(std::io::stderr(), "{avis}");
            return;
        }
    };
    assert!(
        sortie.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );
}

/// Dans Mesurer, la courbe de spectre vient après la liste des mesures de la
/// séance : elle ne la pousse jamais hors de la zone (règle d'écran du
/// mainteneur).
#[test]
fn la_courbe_de_spectre_ne_coupe_pas_la_liste_de_mesurer() {
    let page = lire("interface/index.html");
    let position = |attribut: &str| {
        page.find(attribut)
            .unwrap_or_else(|| panic!("{attribut} absent"))
    };
    assert!(position("data-mesurer-liste") < position("data-mesurer-spectre"));
}

/// Sans Node : l'avis de saut sur le poste, l'échec en intégration continue.
fn sans_node(ci: bool, erreur: &str) -> Result<String, String> {
    if ci {
        Err(format!(
            "Node absent en intégration continue ({erreur}) : les tests des modules de la page doivent tourner."
        ))
    } else {
        Ok(format!(
            "SAUTÉ : Node absent ({erreur}), tests des modules de la page non lancés."
        ))
    }
}

#[test]
fn sans_node_le_poste_annonce_le_saut_et_la_ci_echoue() {
    assert!(sans_node(false, "introuvable")
        .unwrap()
        .starts_with("SAUTÉ"));
    assert!(sans_node(true, "introuvable").is_err());
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
