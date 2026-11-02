//! Écriture d'un fichier sans jamais le laisser à moitié écrit : un fichier
//! temporaire à côté (`<nom>.tmp`), puis un renommage qui remplace l'ancien.
//! En cas d'échec, l'ancien fichier reste et le temporaire est effacé.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// Le fichier temporaire de `fichier` : `<nom>.tmp`, dans le même dossier.
pub fn temporaire(fichier: &Path) -> PathBuf {
    let mut nom = OsString::from(fichier.as_os_str());
    nom.push(".tmp");
    PathBuf::from(nom)
}

/// Écrit `octets` dans `fichier` par un temporaire renommé. Le dossier doit
/// exister.
pub fn ecrire_atomique(fichier: &Path, octets: &[u8]) -> io::Result<()> {
    let tmp = temporaire(fichier);
    let resultat = std::fs::write(&tmp, octets).and_then(|()| std::fs::rename(&tmp, fichier));
    if resultat.is_err() && tmp.is_file() {
        let _ = std::fs::remove_file(&tmp);
    }
    resultat
}
