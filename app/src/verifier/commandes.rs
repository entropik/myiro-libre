//! Commandes de la page pour la vérification d'un profil : elles ne font
//! qu'appeler les méthodes de [`BibliothequeOuverte`] (testées sans Tauri).

use std::time::SystemTime;

use bibliotheque::{IdCondition, IdProfil, IdTolerances, IdVerification, JeuTolerances};
use profilage::{ArgyllCms, Impression, MoteurVerification};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;
use verification::Criteres;

use super::{DemandeVerification, FicheTirage, ResultatVerification, ResumeVerification};
use crate::colonne::BibliothequeOuverte;
use crate::parc::si_choisi;
use crate::profiler::{ErreurProfil, Moteur, SourceProfil};
use crate::textes::Langue;

/// Ce que la section « Avancé » montre : les commandes `xicclu`, et la
/// raison qui empêchera la vérification si ArgyllCMS manque.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandesVerification {
    pub commandes: Vec<String>,
    pub moteur: Option<ErreurProfil>,
}

#[tauri::command]
pub fn verifier_sources(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    profil: i64,
) -> Result<Vec<SourceProfil>, ErreurProfil> {
    ouverte.sources_verification(IdProfil(profil))
}

/// Commandes équivalentes, en lecture seule, même sans ArgyllCMS.
#[tauri::command]
pub fn verifier_commandes(
    moteur: tauri::State<'_, Moteur>,
    impression: Impression,
) -> CommandesVerification {
    CommandesVerification {
        commandes: ArgyllCms::new(Default::default()).commandes_verification(impression),
        moteur: moteur.indisponible(),
    }
}

#[tauri::command]
pub fn verifier_fiche(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    condition: i64,
) -> Result<Option<FicheTirage>, ErreurProfil> {
    ouverte.derniere_fiche(IdCondition(condition))
}

/// Vérifie et range, hors du fil de la fenêtre.
#[tauri::command(async)]
pub fn verifier_lancer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    demande: DemandeVerification,
) -> Result<ResultatVerification, ErreurProfil> {
    let moteur = moteur
        .verification()
        .ok_or_else(|| ErreurProfil::sans_cause("profil.erreur.argyll_introuvable"))?;
    ouverte.verifier_profil(&demande, moteur, SystemTime::now())
}

#[tauri::command]
pub fn verifier_recalculer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    verification: i64,
    tolerances: i64,
) -> Result<ResultatVerification, ErreurProfil> {
    ouverte.recalculer_verification(
        IdVerification(verification),
        IdTolerances(tolerances),
        SystemTime::now(),
    )
}

#[tauri::command]
pub fn verifier_historique(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    profil: i64,
) -> Result<Vec<ResumeVerification>, ErreurProfil> {
    ouverte.historique_verifications(IdProfil(profil))
}

#[tauri::command]
pub fn tolerances_lister(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
) -> Result<Vec<JeuTolerances>, ErreurProfil> {
    ouverte.jeux_tolerances()
}

fn langue(code: &str) -> Langue {
    Langue::depuis_code(code).unwrap_or(Langue::Francais)
}

/// Nom proposé pour le rapport : celui du profil, suivi de `_verification`.
fn nom_rapport(profil: &str) -> String {
    let base = profil.strip_suffix(".icc").unwrap_or(profil);
    format!("{base}_verification.html")
}

/// Exporte le rapport là où l'opérateur le dit ; rend le nom du fichier
/// écrit, ou `None` s'il a annulé.
#[tauri::command(async)]
pub fn verifier_exporter_rapport(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    verification: i64,
    langue: &str,
    filtre: String,
) -> Result<Option<String>, String> {
    let v = IdVerification(verification);
    let d = ouverte.donnees_rapport(v).map_err(|e| e.cle.to_string())?;
    let fichier = app
        .dialog()
        .file()
        .set_file_name(nom_rapport(&d.profil.nom))
        .add_filter(filtre, &["html"])
        .blocking_save_file()
        .and_then(|f| f.into_path().ok());
    let langue = self::langue(langue);
    si_choisi(fichier, |f| {
        ouverte.exporter_rapport(v, langue, f)?;
        Ok(nom_fichier(f))
    })
}

/// Exporte le profil vérifié et, à côté, son rapport.
#[tauri::command(async)]
pub fn verifier_exporter_profil(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    verification: i64,
    langue: &str,
    filtre: String,
) -> Result<Option<String>, String> {
    let v = IdVerification(verification);
    let d = ouverte.donnees_rapport(v).map_err(|e| e.cle.to_string())?;
    let fichier = app
        .dialog()
        .file()
        .set_file_name(d.profil.nom)
        .add_filter(filtre, &["icc", "icm"])
        .blocking_save_file()
        .and_then(|f| f.into_path().ok());
    let langue = self::langue(langue);
    si_choisi(fichier, |f| {
        ouverte.exporter_profil_et_rapport(v, langue, f)?;
        Ok(nom_fichier(f))
    })
}

fn nom_fichier(f: &std::path::Path) -> String {
    f.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[tauri::command]
pub fn tolerances_enregistrer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    nom: &str,
    criteres: Criteres,
) -> Result<JeuTolerances, ErreurProfil> {
    let id = ouverte.enregistrer_tolerances(nom, &criteres, SystemTime::now())?;
    ouverte
        .jeux_tolerances()?
        .into_iter()
        .find(|j| j.id == id)
        .ok_or_else(|| ErreurProfil::sans_cause("bibliotheque.erreur.autre"))
}
