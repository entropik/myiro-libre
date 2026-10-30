//! Commandes de la page pour le rapport d'une image : choisir où enregistrer
//! le PDF (par défaut `Documents\myiro-libre\Rapports`, « Rapport <image>
//! <numéro>.pdf »), l'écrire, puis montrer son dossier.

use std::path::{Path, PathBuf};

use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use super::{DemandeRapportImage, Jour};
use crate::atelier::ReglageAtelier;
use crate::colonne::BibliothequeOuverte;
use crate::comparer::image::ImageOuverte;
use crate::comparer::FichiersOuverts;

/// Le PDF enregistré : son nom et son chemin complet.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RapportEnregistre {
    pub nom: String,
    pub chemin: String,
    pub numero: String,
}

fn dossier_rapports(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dossier = app
        .path()
        .document_dir()
        .ok()?
        .join("myiro-libre")
        .join("Rapports");
    std::fs::create_dir_all(&dossier).ok()?;
    Some(dossier)
}

/// « Rapport photo 2026-0001.pdf » : le nom de l'image sans son extension,
/// sans caractère interdit dans un nom de fichier Windows.
pub fn nom_par_defaut(image: &str, numero: &str) -> String {
    let base = Path::new(image)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let propre: String = base
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    format!("Rapport {} {numero}.pdf", propre.trim())
}

/// Demande où enregistrer le rapport, puis l'écrit. `None` si l'opérateur
/// a annulé ; une erreur rend sa clé de texte.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn image_rapport_exporter(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    image: tauri::State<'_, ImageOuverte>,
    fichiers: tauri::State<'_, FichiersOuverts>,
    reglage: tauri::State<'_, ReglageAtelier>,
    demande: DemandeRapportImage,
    filtre: String,
) -> Result<Option<RapportEnregistre>, String> {
    let jour = Jour::aujourdhui();
    let numero = reglage
        .prochain_numero(jour.annee)
        .map_err(|e| e.cle().to_string())?;
    let nom = image.nom().map(|n| nom_par_defaut(&n, &numero))?;
    let mut fenetre = app
        .dialog()
        .file()
        .set_file_name(nom)
        .add_filter(filtre, &["pdf"]);
    if let Some(dossier) = dossier_rapports(&app) {
        fenetre = fenetre.set_directory(dossier);
    }
    let Some(sortie) = fenetre
        .blocking_save_file()
        .and_then(|f| f.into_path().ok())
    else {
        return Ok(None);
    };
    let numero =
        ouverte.exporter_rapport_image(&image, &fichiers, &demande, &reglage, &sortie, jour)?;
    Ok(Some(RapportEnregistre {
        nom: sortie
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        chemin: sortie.to_string_lossy().into_owned(),
        numero,
    }))
}

/// Montre le rapport enregistré dans l'Explorateur, sélectionné.
#[tauri::command]
pub fn rapport_ouvrir_dossier(chemin: String) -> Result<(), String> {
    let chemin = PathBuf::from(chemin);
    if !chemin.is_file() {
        return Err("rapport.erreur.destination".into());
    }
    #[allow(unused_mut)]
    let mut commande = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
    let mut selection = std::ffi::OsString::from("/select,");
    selection.push(chemin.as_os_str());
    commande
        .arg(selection)
        .spawn()
        .map(|_| ())
        .map_err(|_| "rapport.erreur.destination".into())
}
