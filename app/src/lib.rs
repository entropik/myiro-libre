//! Application de bureau myiro-libre : cadre de l'interface (ADR 0003).
//!
//! Seule cette crate dépend de Tauri ; le cœur (ponts, protocole, colorimétrie,
//! bibliothèque, et plus tard mires) n'en dépend pas. Dans cette crate, les
//! modules `instrument` et `pont` n'en dépendent pas non plus : seul ce
//! fichier fait le lien avec la fenêtre.

pub mod colonne;
pub mod demonstration;
pub mod instrument;
pub mod mesurer;
pub mod pont;
pub mod textes;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};

use instrument::choix::{self, Modele, Occupation, Operation, Ouverture, Selection, VueSelection};
use instrument::parefeu::{AutorisationPareFeu, PareFeuWindows};
use instrument::{emplacements_a_essayer, fd9, Accord, Geste, Gestes};
use mesurer::{choisir_condition, FicheMesure, RefusMesure, Seance};
use pont::Panne;
use pont::{chercher_ponts, chercher_ponts_nommes, PontProcessus};
use pont_protocole::Palier;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use textes::Langue;

/// Instrument actif parmi ceux du poste, une fois cherché. Un seul pont est
/// ouvert à la fois : la sélection ferme l'ancien avant d'ouvrir l'autre.
#[derive(Default)]
struct Instruments {
    selection: Mutex<Option<Selection<PontProcessus>>>,
    /// Opération en cours sur l'instrument : une demande pendant une autre
    /// est refusée avec sa raison (module `instrument`), sans attendre sa fin.
    occupation: Occupation,
}

/// Règle du pare-feu pour le FD-9 : demandée d'elle-même une seule fois
/// pendant l'utilisation de l'application (ticket #47).
struct PareFeuFd9(Mutex<AutorisationPareFeu<PareFeuWindows>>);

impl Default for PareFeuFd9 {
    fn default() -> Self {
        PareFeuFd9(Mutex::new(AutorisationPareFeu::new(
            PareFeuWindows::default(),
        )))
    }
}

/// Fichier où la DLL du fabricant trouvée est retenue pour la fois suivante.
const FICHIER_SDK: &str = "emplacement-sdk.txt";

/// Fichier où le dernier instrument choisi est retenu (ticket #49).
const FICHIER_CHOIX: &str = "instrument-choisi.txt";

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

fn fichier_choix(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(FICHIER_CHOIX))
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

/// `HWND` de la fenêtre de l'application, 0 si elle est introuvable.
fn fenetre_principale(app: &AppHandle) -> isize {
    #[cfg(windows)]
    if let Some(fenetre) = app.webview_windows().values().next() {
        if let Ok(hwnd) = fenetre.hwnd() {
            return hwnd.0 as isize;
        }
    }
    #[cfg(not(windows))]
    let _ = app;
    0
}

/// Lancement d'un pont, tel que le module `instrument` le demande.
type Lanceur = fn(&Path, &Path, Palier) -> Result<PontProcessus, Panne>;

/// Prépare ce dont le module `instrument` a besoin pour ouvrir un instrument
/// (emplacements des logiciels, ponts, pare-feu), puis lui confie `faire`.
/// `dossier` : le dossier choisi par l'opérateur, essayé en premier.
fn avec_ouverture<R>(
    app: &AppHandle,
    dossier: Option<PathBuf>,
    faire: impl FnOnce(&mut Ouverture<'_, Lanceur, PareFeuWindows>) -> R,
) -> R {
    let pare_feu = app.state::<PareFeuFd9>();
    let mut autorisation = pare_feu.0.lock().unwrap_or_else(|e| e.into_inner());
    // La fenêtre de contrôle de compte s'ouvre au premier plan, sur celle de
    // l'application.
    autorisation.pare_feu_mut().fenetre = fenetre_principale(app);
    let dossier_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_default();
    // Les ressources du paquet (DLL embarquées) sont installées à côté de
    // l'exécutable sous Windows.
    let dossier_app = app.path().resource_dir().unwrap_or(dossier_exe.clone());
    let emplacements = emplacements_a_essayer(&dossier_app, sdk_retenu(app), dossier.clone());
    let ponts = chercher_ponts(&dossier_exe);
    let emplacements_fd9 = fd9::emplacements_fd9(dossier);
    let ponts_fd9 = chercher_ponts_nommes(&dossier_exe, fd9::NOM_PONT_FD9);
    faire(&mut Ouverture {
        myiro1: choix::Recherche {
            emplacements: &emplacements,
            ponts: &ponts,
        },
        fd9: choix::Recherche {
            emplacements: &emplacements_fd9,
            ponts: &ponts_fd9,
        },
        lancer: PontProcessus::lancer as Lanceur,
        pare_feu: &mut autorisation,
    })
}

/// Vue de la sélection, après avoir retenu la DLL du MYIRO-1 trouvée.
fn vue_retenue(app: &AppHandle, selection: &Selection<PontProcessus>) -> VueSelection {
    if let Some(dll) = selection.instrument().sdk() {
        retenir_sdk(app, dll);
    }
    selection.vue()
}

/// Au lancement, ouvre l'instrument choisi la fois précédente (ou celui qui
/// est trouvé) ; ensuite (« Réessayer », dossier choisi, pare-feu), rouvre
/// l'instrument actif. Refusé pendant une autre opération.
fn ouvrir(
    app: &AppHandle,
    instruments: &Instruments,
    dossier: Option<PathBuf>,
) -> Result<VueSelection, String> {
    let jeton = instruments
        .occupation
        .commencer(Operation::Recherche)
        .map_err(|refus| refus.cle())?;
    let mut courant = instruments
        .selection
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    avec_ouverture(app, dossier, |o| match courant.as_mut() {
        Some(selection) => selection.rouvrir(&jeton, o),
        None => {
            let retenu = fichier_choix(app).and_then(|f| choix::lire_choix(&f));
            *courant = Some(Selection::lancer(retenu, o));
        }
    });
    let selection = courant.as_ref().expect("sélection ouverte");
    Ok(vue_retenue(app, selection))
}

/// Ouvre l'instrument (au lancement, ou « Réessayer »). Hors du fil de la
/// fenêtre : la connexion peut être longue.
#[tauri::command(async)]
fn ouvrir_instrument(
    app: AppHandle,
    instruments: State<'_, Instruments>,
) -> Result<VueSelection, String> {
    ouvrir(&app, &instruments, None)
}

/// Bouton « Autoriser le FD-9 dans le pare-feu » : la règle sera redemandée
/// une fois (fenêtre de contrôle de compte de Windows), puis l'instrument rouvert.
#[tauri::command(async)]
fn autoriser_pare_feu_fd9(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    pare_feu: State<'_, PareFeuFd9>,
) -> Result<VueSelection, String> {
    pare_feu
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .autoriser_a_nouveau();
    ouvrir(&app, &instruments, None)
}

/// Ouvre le sélecteur de dossier de Windows, puis ouvre l'instrument avec le
/// dossier choisi. `None` si l'opérateur a annulé.
#[tauri::command(async)]
fn choisir_dossier(
    app: AppHandle,
    instruments: State<'_, Instruments>,
) -> Result<Option<VueSelection>, String> {
    let Some(dossier) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let Ok(dossier) = dossier.into_path() else {
        return Ok(None);
    };
    ouvrir(&app, &instruments, Some(dossier)).map(Some)
}

/// Liste des instruments : l'opérateur choisit `code` (`myiro1`, `fd9`). Le
/// pont en cours est fermé, l'autre ouvert, et le choix retenu pour le
/// lancement suivant. Un refus rend la clé du catalogue qui l'explique.
#[tauri::command(async)]
fn choisir_instrument(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    code: &str,
) -> Result<VueSelection, String> {
    let vers = Modele::depuis_code(code).ok_or_else(|| "refus.absent".to_string())?;
    let jeton = instruments
        .occupation
        .commencer(Operation::Changement)
        .map_err(|refus| refus.cle())?;
    let mut courant = instruments
        .selection
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let selection = courant
        .as_mut()
        .ok_or_else(|| "refus.occupe.recherche".to_string())?;
    avec_ouverture(&app, None, |o| selection.changer(&jeton, vers, o))
        .map_err(|refus| refus.cle())?;
    if let Some(fichier) = fichier_choix(&app) {
        let _ = choix::retenir_choix(&fichier, vers);
    }
    Ok(vue_retenue(&app, selection))
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

/// Temps laissé à l'opérateur pour répondre à un geste ; au-delà, il est
/// réputé avoir renoncé et l'écran d'étalonnage se ferme.
const DELAI_GESTE: std::time::Duration = std::time::Duration::from_secs(10 * 60);

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
        // Sans réponse (page fermée, opérateur parti), l'opérateur a renoncé :
        // l'attente est bornée, car l'instrument reste verrouillé pendant ce temps.
        let accord = reponse.recv_timeout(DELAI_GESTE).unwrap_or(Accord::Annule);
        self.attente
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        accord
    }
}

/// Étalonne l'instrument actif : l'écran demande d'abord de le poser sur
/// son blanc. Rend la nouvelle vue ; refusé (clé du catalogue) pendant une
/// autre opération ou avant la recherche de l'instrument.
#[tauri::command(async)]
fn etalonner(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    attente: State<'_, GesteEnAttente>,
) -> Result<VueSelection, String> {
    let _jeton = instruments
        .occupation
        .commencer(Operation::Etalonnage)
        .map_err(|refus| refus.cle())?;
    let mut courant = instruments
        .selection
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let selection = courant
        .as_mut()
        .ok_or_else(|| "refus.occupe.recherche".to_string())?;
    selection.instrument_mut().etalonner(&mut GestesEcran {
        app: &app,
        attente: &attente,
    });
    Ok(selection.vue())
}

/// Mesures ponctuelles faites depuis le lancement (tâche Mesurer).
#[derive(Default)]
struct SeanceMesures(Mutex<Seance>);

/// Ce que la feuille Mesurer reçoit : l'état de l'instrument, s'il a pu
/// changer (avec la liste des instruments du poste), et les mesures de la
/// séance, la plus récente d'abord.
#[derive(serde::Serialize)]
struct EcranMesurer {
    instrument: Option<VueSelection>,
    mesures: Vec<FicheMesure>,
}

fn langue(code: &str) -> Langue {
    Langue::depuis_code(code).unwrap_or(Langue::Francais)
}

/// Mesure ponctuelle : l'écran demande de poser le MYIRO-1 sur la couleur,
/// le pont attend l'appui sur son bouton, puis la mesure est rangée dans la
/// condition d'impression `condition`. Les refus sont des clés du catalogue.
#[tauri::command(async)]
fn mesurer(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    attente: State<'_, GesteEnAttente>,
    bibliotheque: State<'_, colonne::BibliothequeOuverte>,
    seance: State<'_, SeanceMesures>,
    condition: i64,
    langue: &str,
) -> Result<EcranMesurer, String> {
    let langue = self::langue(langue);
    // Refus avant toute mesure : un code `mesurer.erreur.*` (cause et action).
    let condition = choisir_condition(bibliotheque.conditions(), condition)
        .map_err(|refus| refus.code().to_string())?;
    // Pas de mesure pendant un étalonnage ou un changement d'instrument (#49).
    let _jeton = instruments
        .occupation
        .commencer(Operation::Mesure)
        .map_err(|refus| refus.cle())?;
    let mut courant = instruments
        .selection
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let selection = courant
        .as_mut()
        .ok_or(RefusMesure::SansInstrument.code().to_string())?;
    let mut seance = seance.0.lock().unwrap_or_else(|e| e.into_inner());
    seance.mesurer(
        selection.instrument_mut(),
        &mut GestesEcran {
            app: &app,
            attente: &attente,
        },
        &condition,
        |mesure, nom| bibliotheque.enregistrer_mesure_nommee(condition.id, mesure, nom),
        langue,
    );
    Ok(EcranMesurer {
        instrument: Some(selection.vue()),
        mesures: seance.fiches(langue),
    })
}

/// Range à nouveau les mesures que la bibliothèque avait refusées.
#[tauri::command]
fn ranger_a_nouveau(
    seance: State<'_, SeanceMesures>,
    bibliotheque: State<'_, colonne::BibliothequeOuverte>,
    langue: &str,
) -> EcranMesurer {
    let mut seance = seance.0.lock().unwrap_or_else(|e| e.into_inner());
    seance.ranger_a_nouveau(|condition, mesure, nom| {
        bibliotheque.enregistrer_mesure_nommee(condition.id, mesure, nom)
    });
    EcranMesurer {
        instrument: None,
        mesures: seance.fiches(self::langue(langue)),
    }
}

/// Mesures de la séance, dans la langue de l'écran.
#[tauri::command]
fn mesures_seance(seance: State<'_, SeanceMesures>, langue: &str) -> EcranMesurer {
    let seance = seance.0.lock().unwrap_or_else(|e| e.into_inner());
    EcranMesurer {
        instrument: None,
        mesures: seance.fiches(self::langue(langue)),
    }
}

/// Renomme une mesure de la séance, et dans la bibliothèque si elle y est rangée.
#[tauri::command]
fn renommer_mesure(
    seance: State<'_, SeanceMesures>,
    bibliotheque: State<'_, colonne::BibliothequeOuverte>,
    numero: usize,
    nom: &str,
    langue: &str,
) -> Result<EcranMesurer, &'static str> {
    let mut seance = seance.0.lock().unwrap_or_else(|e| e.into_inner());
    seance.renommer(numero, nom, |id, nom| bibliotheque.renommer_mesure(id, nom))?;
    Ok(EcranMesurer {
        instrument: None,
        mesures: seance.fiches(self::langue(langue)),
    })
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
        .manage(PareFeuFd9::default())
        .setup(ouvrir_bibliotheque)
        .manage(GesteEnAttente::default())
        .manage(SeanceMesures::default())
        .invoke_handler(tauri::generate_handler![
            mesurer,
            mesures_seance,
            renommer_mesure,
            ranger_a_nouveau,
            catalogue,
            langue_demandee,
            ouvrir_instrument,
            autoriser_pare_feu_fd9,
            choisir_dossier,
            choisir_instrument,
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
