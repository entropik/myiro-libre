//! Copie les jetons et les composants de `design-system/` dans l'interface
//! avant que Tauri ne l'embarque : `design-system/` reste la source unique.

use std::fs;
use std::path::Path;

fn main() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = racine.join("../design-system");
    let cible = racine.join("interface/systeme");
    fs::create_dir_all(&cible).expect("dossier interface/systeme");
    for fichier in ["tokens.css", "components.css"] {
        let de = source.join(fichier);
        println!("cargo:rerun-if-changed={}", de.display());
        let contenu = fs::read(&de).unwrap_or_else(|e| panic!("{}: {e}", de.display()));
        let vers = cible.join(fichier);
        if fs::read(&vers).ok().as_deref() != Some(contenu.as_slice()) {
            fs::write(&vers, contenu).expect("copie du système graphique");
        }
    }
    tauri_build::build();
}
