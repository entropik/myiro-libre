//! Commandes de la page pour la vérification d'un profil : elles ne font
//! qu'appeler les méthodes de [`BibliothequeOuverte`] (testées sans Tauri).

use std::time::SystemTime;

use bibliotheque::{IdCondition, IdProfil, IdTolerances, IdVerification, JeuTolerances};
use profilage::{ArgyllCms, Impression, MoteurVerification};
use serde::Serialize;
use verification::Criteres;

use super::{DemandeVerification, FicheTirage, ResultatVerification, ResumeVerification};
use crate::colonne::BibliothequeOuverte;
use crate::profiler::{ErreurProfil, Moteur, SourceProfil};

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
