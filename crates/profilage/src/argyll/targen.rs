//! `targen` choisit les plages RVB d'une mire MYIRO-1, avec le même déroulé
//! que le calcul d'un profil : version contrôlée, dossier de travail
//! temporaire, outil lancé sans fenêtre avec des noms de fichiers relatifs.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use super::{ligne, ArgyllCms, DossierTravail};
use crate::moteur::{ErreurMoteur, Etape, PlagesGenerees};

/// Base des fichiers de la mire dans le dossier de travail.
const BASE_MIRE: &str = "mire";

/// Arguments de `targen` : `-v` (journal), `-d2` (imprimante RVB), `-G`
/// (répartition plus régulière, plus lente : temps à mesurer à l'essai) et
/// le nombre de plages, blanches et noires comprises (`-f`, confirmé avec
/// ArgyllCMS 3.5.0 le 28 octobre 2026).
pub fn arguments_targen(nombre: usize, base: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = ["-v", "-d2", "-G"].map(OsString::from).into();
    a.push(format!("-f{nombre}").into());
    a.push(base.into());
    a
}

/// Ligne de commande de `targen` telle qu'elle est lancée, sans rien lancer.
pub(crate) fn ligne_targen(nombre: usize) -> String {
    ligne("targen", &arguments_targen(nombre, Path::new(BASE_MIRE)))
}

impl ArgyllCms {
    /// Lance `targen` et lit le `.ti1` produit.
    pub(super) fn lancer_targen(&self, nombre: usize) -> Result<PlagesGenerees, ErreurMoteur> {
        if !self.outil("targen").is_file() {
            return Err(ErreurMoteur::Introuvable);
        }
        let version = self.version()?;
        super::controler_version(&version)?;

        let dossier = DossierTravail::creer()?;
        let chemin = dossier.chemin();
        let mut journal = String::new();
        self.lancer(
            "targen",
            &arguments_targen(nombre, Path::new(BASE_MIRE)),
            chemin,
            Etape::Plages,
            &mut journal,
            &mut |_| {},
        )?;
        let ti1_nom = format!("{BASE_MIRE}.ti1");
        let ti1 = fs::read_to_string(chemin.join(&ti1_nom))
            .map_err(|e| ErreurMoteur::Fichier(format!("{ti1_nom} : {e}")))?;
        Ok(PlagesGenerees {
            ti1,
            version,
            commande: ligne_targen(nombre),
            journal,
        })
    }
}
