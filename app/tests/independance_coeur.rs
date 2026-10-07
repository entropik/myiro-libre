//! Le cœur ne dépend pas de Tauri (ADR 0003, ADR 0004) : seule la crate `app`
//! tire l'interface. Le graphe vient de `cargo metadata`, comme le voit Cargo.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// Pour chaque crate de l'espace de travail, dit si elle tire Tauri,
/// directement ou par une dépendance.
fn crates_qui_tirent_tauri() -> HashMap<String, bool> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let manifeste = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let sortie = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifeste)
        .output()
        .expect("cargo metadata se lance");
    assert!(sortie.status.success(), "cargo metadata a échoué");
    let meta: Value = serde_json::from_slice(&sortie.stdout).expect("JSON de cargo metadata");

    let noms: HashMap<&str, &str> = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    let deps: HashMap<&str, Vec<&str>> = meta["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            let id = n["id"].as_str().unwrap();
            let d = n["deps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["pkg"].as_str().unwrap())
                .collect();
            (id, d)
        })
        .collect();
    let est_tauri = |id: &str| {
        let nom = noms[id];
        nom == "tauri" || nom.starts_with("tauri-")
    };

    meta["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            let depart = m.as_str().unwrap();
            let mut vus = BTreeSet::new();
            let mut a_voir: Vec<&str> = deps[depart].clone();
            let mut tire = false;
            while let Some(id) = a_voir.pop() {
                if !vus.insert(id) {
                    continue;
                }
                if est_tauri(id) {
                    tire = true;
                    break;
                }
                a_voir.extend(deps[id].iter().copied());
            }
            (noms[depart].to_string(), tire)
        })
        .collect()
}

#[test]
fn seule_l_application_depend_de_tauri() {
    let crates = crates_qui_tirent_tauri();
    assert_eq!(crates.get("app"), Some(&true), "l'application tire Tauri");
    let coeur_avec_tauri: Vec<_> = crates
        .iter()
        .filter(|(nom, tire)| nom.as_str() != "app" && **tire)
        .map(|(nom, _)| nom.as_str())
        .collect();
    assert!(
        coeur_avec_tauri.is_empty(),
        "crates du cœur qui dépendent de Tauri : {coeur_avec_tauri:?}"
    );
    for coeur in ["pont-protocole", "fdx-sys", "pont-myiro1"] {
        assert_eq!(crates.get(coeur), Some(&false), "{coeur} est examinée");
    }
}
