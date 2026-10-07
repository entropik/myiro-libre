//! Application de bureau myiro-libre : cadre de l'interface (ADR 0003).
//!
//! Seule cette crate dépend de Tauri ; le cœur (ponts, protocole, et plus tard
//! colorimétrie, mires, bibliothèque) n'en dépend pas. Dans cette crate, les
//! modules `instrument` et `pont` n'en dépendent pas non plus : seul ce
//! fichier fait le lien avec la fenêtre.

pub mod instrument;
pub mod pont;
pub mod textes;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use instrument::{Instrument, Vue};
use pont::PontProcessus;
use tauri::{AppHandle, Manager, State};
use textes::Langue;

/// Instrument ouvert par l'application, s'il y en a un. Le remplacer ferme
/// l'ancien pont.
#[derive(Default)]
struct Instruments(Mutex<Option<Instrument<PontProcessus>>>);

/// Nom du fichier où l'emplacement du SDK indiqué est retenu.
const FICHIER_SDK: &str = "emplacement-sdk.txt";

/// Textes de l'interface dans une langue, pour la page web.
#[tauri::command]
fn catalogue(langue: &str) -> BTreeMap<&'static str, &'static str> {
    let langue = Langue::depuis_code(langue).unwrap_or(Langue::Francais);
    textes::cles()
        .map(|c| (c, textes::texte(langue, c)))
        .collect()
}

/// Valeur d'une option de lancement (`--langue en`, `--pont <chemin>`).
fn option_de_lancement(nom: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2).find(|a| a[0] == nom).map(|a| a[1].clone())
}

/// Langue demandée au lancement (`--langue fr` ou `--langue en`), si elle l'est.
#[tauri::command]
fn langue_demandee() -> Option<&'static str> {
    option_de_lancement("--langue")
        .and_then(|code| Langue::depuis_code(&code))
        .map(Langue::code)
}

/// Le programme `pont-myiro1` livré à côté de l'application, ou celui donné
/// au lancement par `--pont <chemin>` (par exemple le pont 32 bits).
fn programme_pont() -> PathBuf {
    if let Some(chemin) = option_de_lancement("--pont") {
        return PathBuf::from(chemin);
    }
    let nom = format!("pont-myiro1{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join(&nom)))
        .unwrap_or_else(|| PathBuf::from(nom))
}

fn fichier_sdk(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(FICHIER_SDK))
}

/// Emplacement du SDK indiqué lors d'une utilisation précédente.
#[tauri::command]
fn emplacement_sdk(app: AppHandle) -> Option<String> {
    let texte = std::fs::read_to_string(fichier_sdk(&app)?).ok()?;
    let texte = texte.trim();
    (!texte.is_empty()).then(|| texte.to_string())
}

/// Ferme l'instrument en cours, puis en ouvre un nouveau avec cet emplacement.
fn ouvrir(instruments: &Instruments, sdk: Option<String>) -> Vue {
    let mut courant = instruments.0.lock().unwrap_or_else(|e| e.into_inner());
    *courant = None;
    let programme = programme_pont();
    let instrument =
        Instrument::ouvrir(sdk.as_deref().map(std::path::Path::new), |dll, plafond| {
            PontProcessus::lancer(&programme, dll, plafond)
        });
    let vue = instrument.vue();
    *courant = Some(instrument);
    vue
}

/// Ouvre l'instrument avec l'emplacement du SDK retenu (au lancement, ou
/// « Réessayer »). Hors du fil de la fenêtre : la connexion peut être longue.
#[tauri::command(async)]
fn ouvrir_instrument(app: AppHandle, instruments: State<'_, Instruments>) -> Vue {
    ouvrir(&instruments, emplacement_sdk(app))
}

/// Retient l'emplacement du SDK indiqué par l'opérateur, puis ouvre
/// l'instrument avec lui.
#[tauri::command(async)]
fn indiquer_sdk(app: AppHandle, instruments: State<'_, Instruments>, chemin: String) -> Vue {
    // Un chemin copié depuis l'Explorateur arrive souvent entre guillemets droits.
    let chemin = chemin.trim().trim_matches('"').trim().to_string();
    if let Some(fichier) = fichier_sdk(&app) {
        if let Some(dossier) = fichier.parent() {
            let _ = std::fs::create_dir_all(dossier);
        }
        let _ = std::fs::write(&fichier, &chemin);
    }
    ouvrir(&instruments, (!chemin.is_empty()).then_some(chemin))
}

/// Ouvre la fenêtre principale.
pub fn lancer() {
    tauri::Builder::default()
        .manage(Instruments::default())
        .invoke_handler(tauri::generate_handler![
            catalogue,
            langue_demandee,
            emplacement_sdk,
            ouvrir_instrument,
            indiquer_sdk
        ])
        .run(tauri::generate_context!())
        .expect("lancement de l'application");
}
