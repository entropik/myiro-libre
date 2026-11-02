//! Commandes de la page pour le réglage « Atelier ». Un refus rend sa clé
//! de texte et, pour un champ trop long, le champ, pour que la page dise
//! l'erreur sous lui.

use serde::Serialize;
use tauri::ipc::Response;
use tauri_plugin_dialog::DialogExt;

use super::{AtelierSaisi, ErreurAtelier, ReglageAtelier};

/// Ce que la page reçoit : les coordonnées, et si un logo est posé (ses
/// octets se demandent à part, en binaire).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueAtelier {
    pub saisi: AtelierSaisi,
    pub logo: bool,
}

/// Un refus, dit à la page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefusAtelier {
    pub cle: &'static str,
    pub champ: Option<&'static str>,
}

impl From<ErreurAtelier> for RefusAtelier {
    fn from(e: ErreurAtelier) -> Self {
        let champ = match e {
            ErreurAtelier::TropLong(champ) => Some(champ),
            ErreurAtelier::LogoFormat | ErreurAtelier::LogoIllisible => Some("logo"),
            _ => None,
        };
        if let ErreurAtelier::Ecriture(detail) | ErreurAtelier::Lecture(detail) = &e {
            eprintln!("atelier : {detail}");
        }
        RefusAtelier {
            cle: e.cle(),
            champ,
        }
    }
}

/// La vue du réglage.
pub fn vue(reglage: &ReglageAtelier) -> Result<VueAtelier, RefusAtelier> {
    Ok(VueAtelier {
        saisi: reglage.saisi()?,
        logo: reglage.a_un_logo(),
    })
}

#[tauri::command]
pub fn atelier_lire(reglage: tauri::State<'_, ReglageAtelier>) -> Result<VueAtelier, RefusAtelier> {
    vue(&reglage)
}

/// Le logo en PNG, en binaire ; vide sans logo.
#[tauri::command(async)]
pub fn atelier_logo(reglage: tauri::State<'_, ReglageAtelier>) -> Result<Response, RefusAtelier> {
    let atelier = reglage.lire()?;
    Ok(Response::new(atelier.logo_png.unwrap_or_default()))
}

#[tauri::command]
pub fn atelier_enregistrer(
    reglage: tauri::State<'_, ReglageAtelier>,
    saisi: AtelierSaisi,
) -> Result<VueAtelier, RefusAtelier> {
    reglage.enregistrer(&saisi)?;
    vue(&reglage)
}

/// Choisit un logo dans la fenêtre de Windows (PNG, JPEG) ; `None` si
/// l'opérateur a annulé.
#[tauri::command(async)]
pub fn atelier_choisir_logo(
    app: tauri::AppHandle,
    reglage: tauri::State<'_, ReglageAtelier>,
    filtre: String,
) -> Result<Option<VueAtelier>, RefusAtelier> {
    let fichier = app
        .dialog()
        .file()
        .add_filter(filtre, &["png", "jpg", "jpeg"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok());
    match fichier {
        Some(f) => {
            reglage.poser_logo(&f)?;
            vue(&reglage).map(Some)
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn atelier_retirer_logo(
    reglage: tauri::State<'_, ReglageAtelier>,
) -> Result<VueAtelier, RefusAtelier> {
    reglage.retirer_logo()?;
    vue(&reglage)
}
