//! Commandes de la page pour l'image ouverte : les pixels et les codes
//! d'écart partent en binaire (`tauri::ipc::Response`), jamais en JSON.

use tauri::ipc::Response;
use tauri_plugin_dialog::DialogExt;

use super::{AnalyseEnCours, AnalyseProfil, ImageOuverte, ImageResume};
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

/// Analyse l'image face aux profils affichés. Hors du fil de la fenêtre ;
/// une demande d'annulation antérieure est oubliée.
#[tauri::command(async)]
pub fn image_analyser(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    image: tauri::State<'_, ImageOuverte>,
    fichiers: tauri::State<'_, FichiersOuverts>,
    annuler: tauri::State<'_, AnalyseEnCours>,
    sources: Vec<SourceComparaison>,
) -> Result<Vec<AnalyseProfil>, String> {
    annuler.reprendre();
    ouverte.analyser_image(&image, &sources, &fichiers, &annuler)
}

/// Codes d'écart du profil `index` de la dernière analyse, un octet par
/// pixel, en binaire.
#[tauri::command(async)]
pub fn image_ecarts(
    image: tauri::State<'_, ImageOuverte>,
    index: usize,
) -> Result<Response, String> {
    image.ecarts(index).map(Response::new)
}

#[tauri::command]
pub fn image_annuler(annuler: tauri::State<'_, AnalyseEnCours>) {
    annuler.annuler();
}
