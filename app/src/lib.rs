//! Application de bureau myiro-libre : cadre de l'interface (ADR 0003).
//!
//! Seule cette crate dépend de Tauri ; le cœur (ponts, protocole, colorimétrie,
//! bibliothèque, et plus tard mires) n'en dépend pas. Dans cette crate, les
//! modules `instrument` et `pont` n'en dépendent pas non plus : seul ce
//! fichier fait le lien avec la fenêtre.

pub mod colonne;
pub mod demonstration;
pub mod instrument;
pub mod pont;
pub mod textes;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};

use instrument::{emplacements_a_essayer, Accord, Geste, Gestes, Instrument, Vue};
use pont::{chercher_ponts, PontProcessus};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use textes::Langue;

/// Instrument ouvert par l'application, s'il y en a un. Le remplacer ferme
/// l'ancien pont.
#[derive(Default)]
struct Instruments(Mutex<Option<Instrument<PontProcessus>>>);

/// Fichier où la DLL du fabricant trouvée est retenue pour la fois suivante.
const FICHIER_SDK: &str = "emplacement-sdk.txt";

/// Textes de l'interface dans une langue, pour la page web.
#[tauri::command]
fn catalogue(langue: &str) -> BTreeMap<&'static str, &'static str> {
    let langue = Langue::depuis_code(langue).unwrap_or(Langue::Francais);
    textes::cles()
        .map(|c| (c, textes::texte(langue, c)))
        .collect()
}

/// Langue demandée au lancement (`--langue fr` ou `--langue en`), si elle l'est.
#[tauri::command]
fn langue_demandee() -> Option<&'static str> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|a| a[0] == "--langue")
        .and_then(|a| Langue::depuis_code(&a[1]))
        .map(Langue::code)
}

fn fichier_sdk(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(FICHIER_SDK))
}

/// DLL du fabricant retenue lors d'une utilisation précédente.
fn sdk_retenu(app: &AppHandle) -> Option<PathBuf> {
    let texte = std::fs::read_to_string(fichier_sdk(app)?).ok()?;
    let texte = texte.trim();
    (!texte.is_empty()).then(|| PathBuf::from(texte))
}

fn retenir_sdk(app: &AppHandle, dll: &std::path::Path) {
    if let Some(fichier) = fichier_sdk(app) {
        if let Some(dossier) = fichier.parent() {
            let _ = std::fs::create_dir_all(dossier);
        }
        let _ = std::fs::write(&fichier, dll.display().to_string());
    }
}

/// Ferme l'instrument en cours, puis en ouvre un nouveau, en essayant les
/// emplacements dans l'ordre de `emplacements_a_essayer`.
fn ouvrir(app: &AppHandle, instruments: &Instruments, choisi: Option<PathBuf>) -> Vue {
    let mut courant = instruments.0.lock().unwrap_or_else(|e| e.into_inner());
    *courant = None;
    let dossier_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_default();
    // Les ressources du paquet (DLL embarquées) sont installées à côté de
    // l'exécutable sous Windows.
    let dossier_app = app.path().resource_dir().unwrap_or(dossier_exe.clone());
    let emplacements = emplacements_a_essayer(&dossier_app, sdk_retenu(app), choisi);
    let ponts = chercher_ponts(&dossier_exe);
    let instrument = Instrument::ouvrir(&emplacements, &ponts, PontProcessus::lancer);
    if let Some(dll) = instrument.sdk() {
        retenir_sdk(app, dll);
    }
    let vue = instrument.vue();
    *courant = Some(instrument);
    vue
}

/// Ouvre l'instrument (au lancement, ou « Réessayer »). Hors du fil de la
/// fenêtre : la connexion peut être longue.
#[tauri::command(async)]
fn ouvrir_instrument(app: AppHandle, instruments: State<'_, Instruments>) -> Vue {
    ouvrir(&app, &instruments, None)
}

/// Ouvre le sélecteur de dossier de Windows, puis ouvre l'instrument avec le
/// dossier choisi. `None` si l'opérateur a annulé.
#[tauri::command(async)]
fn choisir_dossier(app: AppHandle, instruments: State<'_, Instruments>) -> Option<Vue> {
    let dossier = app.dialog().file().blocking_pick_folder()?;
    let dossier = dossier.into_path().ok()?;
    Some(ouvrir(&app, &instruments, Some(dossier)))
}

/// Ouvre la bibliothèque du poste, ou celle de démonstration avec `--demo`.
fn ouvrir_bibliotheque(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let ouverte = if std::env::args().any(|a| a == "--demo") {
        colonne::BibliothequeOuverte::demonstration(&demonstration::dossier())
    } else {
        colonne::BibliothequeOuverte::ouvrir(&colonne::emplacement(&app.path().app_data_dir()?))
    };
    app.manage(ouverte);
    Ok(())
}

/// Geste montré à l'écran, en attente de la réponse de l'opérateur.
#[derive(Default)]
struct GesteEnAttente(Mutex<Option<mpsc::Sender<Accord>>>);

/// Adapter réel du trait `Gestes` : l'écran. Il envoie le geste à la page
/// (événement `geste`), puis attend la réponse de l'opérateur, donnée par la
/// commande `repondre_geste`.
struct GestesEcran<'a> {
    app: &'a AppHandle,
    attente: &'a GesteEnAttente,
}

impl Gestes for GestesEcran<'_> {
    fn demander(&mut self, geste: Geste) -> Accord {
        let (envoi, reponse) = mpsc::channel();
        *self.attente.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(envoi);
        if self.app.emit("geste", geste).is_err() {
            return Accord::Annule;
        }
        // Sans réponse possible (page fermée), l'opérateur a renoncé.
        reponse.recv().unwrap_or(Accord::Annule)
    }
}

/// Étalonne l'instrument ouvert : l'écran demande d'abord de le poser sur
/// son blanc. Rend la nouvelle vue, ou `None` sans instrument ouvert.
#[tauri::command(async)]
fn etalonner(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    attente: State<'_, GesteEnAttente>,
) -> Option<Vue> {
    let mut courant = instruments.0.lock().unwrap_or_else(|e| e.into_inner());
    let instrument = courant.as_mut()?;
    instrument.etalonner(&mut GestesEcran {
        app: &app,
        attente: &attente,
    });
    Some(instrument.vue())
}

/// Réponse de l'opérateur au geste en attente : fait, ou annulé. Sans geste
/// en attente, rien ne se passe.
#[tauri::command]
fn repondre_geste(fait: bool, attente: State<'_, GesteEnAttente>) {
    let envoi = attente.0.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(envoi) = envoi {
        let _ = envoi.send(if fait { Accord::Fait } else { Accord::Annule });
    }
}

/// Ouvre la fenêtre principale.
pub fn lancer() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Instruments::default())
        .setup(ouvrir_bibliotheque)
        .manage(GesteEnAttente::default())
        .invoke_handler(tauri::generate_handler![
            catalogue,
            langue_demandee,
            ouvrir_instrument,
            choisir_dossier,
            colonne::version_application,
            colonne::bibliotheque_demonstration,
            colonne::bibliotheque_arborescence,
            colonne::bibliotheque_detail_mesure,
            colonne::bibliotheque_creer_condition,
            colonne::bibliotheque_renommer_condition,
            etalonner,
            repondre_geste
        ])
        .run(tauri::generate_context!())
        .expect("lancement de l'application");
}
