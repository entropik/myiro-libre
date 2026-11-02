//! Commandes de la page pour la séance de mesure d'un tirage FD-9 (ticket
//! #35) : elles ne font qu'appeler les méthodes de [`BibliothequeOuverte`]
//! (testées sans Tauri). La feuille mesurée au FD-9 passe par
//! `ecran_feuille::seance_mesurer_exemplaire`, qui tient l'instrument comme
//! « Mesurer une feuille au FD-9 ». Aucun écran ne les appelle encore.

use std::time::SystemTime;

use bibliotheque::{IdCondition, IdMesure, IdSeance, IdTirage};
use seance_mesure::{Complement, Role, VuePlage};
use tauri_plugin_dialog::DialogExt;

use super::{EtatSeanceVue, FinDemandee, RefusSeance, SourceAjoutee};
use crate::colonne::BibliothequeOuverte;
use crate::verifier::FicheTirage;

/// Commence la séance du tirage, ou reprend celle qui est en cours.
#[tauri::command]
pub fn seance_commencer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    condition: i64,
    tirage: i64,
    fiche: FicheTirage,
) -> Result<IdSeance, RefusSeance> {
    ouverte.commencer_seance_fd9(
        IdCondition(condition),
        IdTirage(tirage),
        &fiche,
        SystemTime::now(),
    )
}

/// Les séances en cours de la résolution, à reprendre.
#[tauri::command]
pub fn seance_en_cours(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    condition: i64,
) -> Result<Vec<IdSeance>, String> {
    ouverte.seances_en_cours(IdCondition(condition))
}

/// La séance, ses mesures sources, l'aperçu de la moyenne ou son résultat.
#[tauri::command(async)]
pub fn seance_etat(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
) -> Result<EtatSeanceVue, RefusSeance> {
    ouverte.etat_seance(IdSeance(seance))
}

/// Importe un fichier CGATS (export FD-S2w) choisi par l'opérateur dans
/// la séance ; `None` s'il a annulé.
#[tauri::command(async)]
pub fn seance_importer(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
    role: Role,
    filtre: String,
) -> Result<Option<SourceAjoutee>, RefusSeance> {
    let Some(fichier) = app
        .dialog()
        .file()
        .add_filter(filtre, &["txt", "cgats", "it8", "ti3"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok())
    else {
        return Ok(None);
    };
    let octets = std::fs::read(&fichier).map_err(|e| {
        eprintln!("{} : {e}", fichier.display());
        super::cle::CGATS_ILLISIBLE
    })?;
    let nom = fichier
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    ouverte
        .importer_dans_seance(IdSeance(seance), &nom, &octets, role, SystemTime::now())
        .map(Some)
}

/// Chaque plage du plan, l'attendu à côté du mesuré, pour une mesure source.
#[tauri::command(async)]
pub fn seance_inspecter(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
    mesure: i64,
) -> Result<Vec<VuePlage>, RefusSeance> {
    ouverte.plages_source(IdSeance(seance), IdMesure(mesure))
}

/// Remplace les compléments manuels de la séance.
#[tauri::command]
pub fn seance_completer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
    complements: Vec<Complement>,
) -> Result<(), RefusSeance> {
    ouverte.completer_seance(IdSeance(seance), complements)
}

/// Retient ou écarte une feuille, avec le motif de l'opérateur.
#[tauri::command]
pub fn seance_retenir(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
    mesure: i64,
    retenue: bool,
    motif: String,
) -> Result<(), RefusSeance> {
    ouverte.retenir_source(
        IdSeance(seance),
        IdMesure(mesure),
        retenue,
        &motif,
        SystemTime::now(),
    )
}

/// Ferme la séance et range la mesure moyennée.
#[tauri::command(async)]
pub fn seance_terminer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    seance: i64,
    fin: FinDemandee,
) -> Result<IdMesure, RefusSeance> {
    ouverte.terminer_seance(IdSeance(seance), fin, SystemTime::now())
}
