//! Commandes de la page pour préparer une mire.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use bibliotheque::IdCondition;
use mires::{ErreurMire, Instrument, RGB2520_FICHIER};
use profilage::ArgyllCms;

use super::{chercher_mire_fd9, ApercuMire, DemandeMire, MirePreparee, DOSSIER_MIRE_FD9};
use crate::colonne::BibliothequeOuverte;
use crate::profiler::Moteur;
use crate::textes::Langue;

/// Dossier de l'exécutable ; vide s'il est introuvable.
fn dossier_exe() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_default()
}

/// ArgyllCMS trouvé au lancement, ou à sa place attendue à côté de
/// l'application : absent, il refuse alors de choisir les plages
/// (« ArgyllCMS introuvable ») mais montre la commande.
fn argyll(moteur: &Moteur) -> ArgyllCms {
    moteur
        .0
        .clone()
        .unwrap_or_else(|| ArgyllCms::new(dossier_exe().join("argyll")))
}

/// Dossier des fichiers à imprimer : `Documents\myiro-libre\Mires`.
fn dossier_mires(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    app.path()
        .document_dir()
        .map(|d| d.join("myiro-libre").join("Mires"))
        .map_err(|e| {
            eprintln!("dossier Documents : {e}");
            ErreurMire::Ecriture(e.to_string()).cle().to_string()
        })
}

/// Aperçu de la mire demandée, sans rien lancer ; pour le FD-9, la page
/// apprend aussi si la mire RGB-2520 manque, avant le clic.
#[tauri::command(async)]
pub fn mire_apercu(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    demande: DemandeMire,
) -> Result<ApercuMire, String> {
    let mut apercu = ouverte.apercu_mire(&demande, &argyll(&moteur))?;
    if demande.instrument == Instrument::Fd9 {
        let exe = dossier_exe();
        if chercher_mire_fd9(&exe).is_none() {
            apercu.mire_fd9_attendue = Some(exe.join(DOSSIER_MIRE_FD9).join(RGB2520_FICHIER));
        }
    }
    Ok(apercu)
}

/// Prépare la mire et range son tirage. Hors du fil de la fenêtre : `targen`
/// peut prendre quelques secondes. Le texte imprimé sur les pages MYIRO-1
/// est dans la langue de la page (`fr` ou `en`, français sinon).
#[tauri::command(async)]
pub fn mire_preparer(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    condition: i64,
    demande: DemandeMire,
    langue: &str,
) -> Result<MirePreparee, String> {
    let mire_fd9 = chercher_mire_fd9(&dossier_exe());
    ouverte.preparer_mire(
        IdCondition(condition),
        &demande,
        &argyll(&moteur),
        mire_fd9.as_deref(),
        &dossier_mires(&app)?,
        SystemTime::now(),
        Langue::depuis_code(langue).unwrap_or(Langue::Francais),
    )
}

/// Montre le dossier des mires dans l'Explorateur, en le créant au besoin.
#[tauri::command]
pub fn mire_ouvrir_dossier(app: tauri::AppHandle) -> Result<(), String> {
    let dossier = dossier_mires(&app)?;
    let echec = |e: std::io::Error| {
        eprintln!("{} : {e}", dossier.display());
        ErreurMire::Ecriture(e.to_string()).cle().to_string()
    };
    fs::create_dir_all(&dossier).map_err(echec)?;
    #[allow(unused_mut)]
    let mut commande = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
    commande.arg(&dossier).spawn().map(|_| ()).map_err(echec)
}
