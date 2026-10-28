//! Commandes de la page pour l'image ouverte : les pixels et les codes
//! d'écart partent en binaire (`tauri::ipc::Response`), jamais en JSON.

use tauri::ipc::Response;
use tauri_plugin_dialog::DialogExt;

use super::{Analyses, ImageOuverte, ImageResume};
use crate::colonne::BibliothequeOuverte;
use crate::comparer::{FichiersOuverts, SourceComparaison};
use crate::parc::si_choisi;

/// Ouvre une image choisie dans la fenêtre de Windows (JPEG, PNG, TIFF) à la
/// place de la précédente. `None` si l'opérateur a annulé.
#[tauri::command(async)]
pub fn image_ouvrir(
    app: tauri::AppHandle,
    image: tauri::State<'_, ImageOuverte>,
    filtre: String,
) -> Result<Option<ImageResume>, String> {
    let fichier = app
        .dialog()
        .file()
        .add_filter(filtre, &["jpg", "jpeg", "png", "tif", "tiff"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok());
    si_choisi(fichier, |f| image.ouvrir(f))
}

#[tauri::command]
pub fn image_traiter_comme_srvb(
    image: tauri::State<'_, ImageOuverte>,
) -> Result<ImageResume, String> {
    image.traiter_comme_srvb()
}

#[tauri::command]
pub fn image_fermer(image: tauri::State<'_, ImageOuverte>) {
    image.fermer();
}

/// Pixels RVBA de l'image d'analyse, en binaire (`ArrayBuffer` côté page).
#[tauri::command(async)]
pub fn image_apercu(image: tauri::State<'_, ImageOuverte>) -> Result<Response, String> {
    image.apercu().map(Response::new)
}

/// Analyse l'image face aux profils affichés, à la place de l'analyse en
/// cours. Hors du fil de la fenêtre.
#[tauri::command(async)]
pub fn image_analyser(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    image: tauri::State<'_, ImageOuverte>,
    fichiers: tauri::State<'_, FichiersOuverts>,
    sources: Vec<SourceComparaison>,
) -> Result<Analyses, String> {
    ouverte.analyser_image(&image, &sources, &fichiers)
}

/// Codes d'écart du profil `index` de l'analyse `analyse` (la dernière
/// terminée), un octet par pixel, en binaire.
#[tauri::command(async)]
pub fn image_ecarts(
    image: tauri::State<'_, ImageOuverte>,
    analyse: u64,
    index: usize,
) -> Result<Response, String> {
    image.ecarts(analyse, index).map(Response::new)
}

#[tauri::command]
pub fn image_annuler(image: tauri::State<'_, ImageOuverte>) {
    image.annuler_analyse();
}
