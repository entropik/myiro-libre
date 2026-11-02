//! Commandes de l'écran « Mesurer une feuille au FD-9 » (ticket #34) :
//! l'état du bouton, la mesure et son avancement (événement `feuille_fd9`),
//! l'annulation, et le calage montré dans l'onglet « Points mesurés »,
//! dont l'image part en binaire (ADR 0003). Avec `--demo`, un FD-9 simulé
//! (`demonstration_fd9`) remplace l'instrument et la mire.
//!
//! La même mesure sert à la séance de mesure d'un tirage (ticket #35) :
//! `seance_mesurer_exemplaire` mesure une feuille de la séance avec la mire
//! de son plan figé et la range comme mesure source.
//!
//! Le reste (définition de la mire, avancement, rangement, calage gardé) est
//! dans `instrument::feuille`, `mesurer_feuille` et `seance_fd9`, sans Tauri.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pont_protocole::CalageFeuille;
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, State};

use crate::colonne::BibliothequeOuverte;
use crate::demonstration_fd9::{self, IssueDemonstration, PontDemonstrationFd9};
use crate::instrument::choix::{Modele, Operation, VueSelection};
use crate::instrument::feuille::{EtapeFeuille, VueCalage};
use crate::instrument::Instrument;
use crate::mesurer_feuille::{
    self, etat_feuille_fd9, EchecFeuille, EtatFeuille, FeuilleRangee, RAISON_OCCUPE,
    RAISON_SANS_FD9,
};
use crate::pont::{self, Panne, Pont};
use crate::seance_fd9::SourceAjoutee;
use crate::{DossierBibliotheque, Instruments};
use mires::MireFd9;

/// Annulation de la feuille en cours, utilisable pendant qu'elle tient
/// l'instrument.
#[derive(Default)]
pub(crate) struct AnnulationFeuille(Mutex<Option<pont::Annulation>>);

/// Calage montré dans l'onglet « Points mesurés » : celui de la feuille en
/// cours, ou le dernier de la résolution affichée. Numéroté : la page
/// demande l'image du calage qu'elle a reçu, jamais celle d'un autre.
#[derive(Default)]
pub(crate) struct CalageAffiche(Mutex<(u64, Option<CalageFeuille>)>);

impl CalageAffiche {
    fn montrer(&self, calage: CalageFeuille) -> u64 {
        let mut courant = self.0.lock().unwrap_or_else(|e| e.into_inner());
        courant.0 += 1;
        courant.1 = Some(calage);
        courant.0
    }
}

/// Mire RGB-2520 lue une fois pour toutes, dès qu'elle l'a été sans erreur.
#[derive(Default)]
pub(crate) struct MireFeuille(Mutex<Option<Arc<MireFd9>>>);

impl MireFeuille {
    fn lue(&self) -> Result<Arc<MireFd9>, String> {
        let mut garde = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mire) = garde.as_ref() {
            return Ok(mire.clone());
        }
        let dossier_exe = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(PathBuf::from))
            .unwrap_or_default();
        let mire = Arc::new(mesurer_feuille::lire_rgb2520(
            crate::mires::chercher_mire_fd9(&dossier_exe).as_deref(),
        )?);
        *garde = Some(mire.clone());
        Ok(mire)
    }
}

/// FD-9 de démonstration (`--demo`), et l'issue de ses feuilles
/// (`--demo-feuille`). Absent hors démonstration.
pub(crate) struct DemonstrationFd9(Option<Mutex<Instrument<PontDemonstrationFd9>>>);

impl DemonstrationFd9 {
    pub(crate) fn au_lancement() -> Self {
        if !std::env::args().any(|a| a == "--demo") {
            return DemonstrationFd9(None);
        }
        let issue = IssueDemonstration::du_lancement();
        DemonstrationFd9(Some(Mutex::new(demonstration_fd9::instrument(
            issue,
            demonstration_fd9::RYTHME,
        ))))
    }

    /// Avec `--demo`, ce que la barre et la liste des instruments montrent :
    /// le FD-9 simulé seul, en service ; aucun instrument du poste n'est
    /// cherché ni ouvert. Pendant une feuille, il est dit occupé.
    pub(crate) fn vue(&self) -> Option<Result<VueSelection, String>> {
        let demo = self.0.as_ref()?;
        Some(match demo.try_lock() {
            Ok(fd9) => Ok(VueSelection::seul(Modele::Fd9, fd9.vue())),
            Err(_) => Err("refus.occupe.mesure".into()),
        })
    }
}

/// Ce que la page reçoit à chaque étape d'une feuille : la résolution où
/// elle sera rangée, l'étape, et le numéro du calage annoncé (image à
/// demander par `feuille_fd9_image`).
#[derive(Clone, Serialize)]
struct EvenementFeuille<'a> {
    condition: i64,
    numero: Option<u64>,
    #[serde(flatten)]
    etape: &'a EtapeFeuille,
}

/// Ce que la page reçoit d'une mesure de feuille : la feuille rangée, ou
/// pourquoi elle ne l'est pas, et l'état de l'instrument (absent en
/// démonstration).
#[derive(Serialize)]
pub(crate) struct EcranFeuille {
    rangee: Option<FeuilleRangee>,
    /// La feuille d'une séance, rangée comme mesure source (#35).
    source: Option<SourceAjoutee>,
    echec: Option<EchecFeuille>,
    instrument: Option<VueSelection>,
}

/// Ce qui est mesuré : la RGB-2520 rangée dans une résolution (#34), ou
/// une feuille d'une séance, avec la mire de son plan (#35).
enum Demande {
    Feuille {
        condition: bibliotheque::IdCondition,
        mire: Result<MireFd9, String>,
    },
    Exemplaire {
        seance: bibliotheque::IdSeance,
    },
}

/// La feuille rangée, selon la demande.
enum Rangement {
    Feuille(FeuilleRangee),
    Exemplaire(SourceAjoutee),
}

/// Mesure et range, avec l'instrument et le lanceur de ponts donnés.
fn mesurer_avec<P: Pont>(
    bibliotheque: &BibliothequeOuverte,
    demande: Demande,
    instrument: &mut Instrument<P>,
    dossier: &Path,
    lancer: &mut impl FnMut(&Path, &Path, pont_protocole::Palier) -> Result<P, Panne>,
    suivi: &mut impl FnMut(EtapeFeuille),
) -> Result<Rangement, EchecFeuille> {
    match demande {
        Demande::Feuille { condition, mire } => bibliotheque
            .mesurer_feuille_fd9(condition, instrument, mire, dossier, lancer, suivi)
            .map(Rangement::Feuille),
        Demande::Exemplaire { seance } => bibliotheque
            .mesurer_exemplaire_fd9(seance, instrument, dossier, lancer, suivi)
            .map(Rangement::Exemplaire),
    }
}

/// Le bouton « Mesurer une feuille au FD-9 » : actif, ou les raisons qui
/// l'en empêchent, et la mire pour l'onglet « Mire ». Ne bloque jamais :
/// pendant une autre opération, l'instrument est dit occupé.
#[tauri::command(async)]
pub(crate) fn feuille_fd9_etat(
    instruments: State<'_, Instruments>,
    demonstration: State<'_, DemonstrationFd9>,
    mire: State<'_, MireFeuille>,
) -> EtatFeuille {
    fn raison<P: pont::Pont>(instrument: Option<&Instrument<P>>) -> Option<&'static str> {
        match instrument {
            None => Some(RAISON_OCCUPE),
            Some(i) if i.feuille_possible() => None,
            Some(_) => Some(RAISON_SANS_FD9),
        }
    }
    if let Some(demo) = &demonstration.0 {
        let fd9 = raison(demo.try_lock().ok().as_deref());
        return etat_feuille_fd9(fd9, Ok(demonstration_fd9::mire()), true);
    }
    let fd9 = match instruments.selection.try_lock() {
        Ok(selection) => match selection.as_ref() {
            Some(selection) => raison(Some(selection.instrument())),
            None => Some(RAISON_OCCUPE),
        },
        Err(_) => Some(RAISON_OCCUPE),
    };
    etat_feuille_fd9(fd9, mire.lue(), false)
}

/// Mesure la RGB-2520 au FD-9 et la range dans la résolution `condition`.
/// Chaque étape est annoncée à la page par l'événement `feuille_fd9`, le
/// calage sans ses pixels. Refusé (clé du catalogue) pendant une autre
/// opération.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn mesurer_feuille_fd9(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    bibliotheque: State<'_, BibliothequeOuverte>,
    dossier: State<'_, DossierBibliotheque>,
    annulation: State<'_, AnnulationFeuille>,
    calage: State<'_, CalageAffiche>,
    demonstration: State<'_, DemonstrationFd9>,
    mire: State<'_, MireFeuille>,
    condition: i64,
) -> Result<EcranFeuille, String> {
    let etats = Etats {
        instruments: &instruments,
        bibliotheque: &bibliotheque,
        dossier: &dossier,
        annulation: &annulation,
        calage: &calage,
        demonstration: &demonstration,
        mire: &mire,
    };
    mesurer_au_fd9(&app, &etats, condition, None)
}

/// Mesure une feuille de la séance `seance` (ticket #35), de la résolution
/// `condition`, au FD-9, avec la mire du plan figé de la séance ; elle est
/// rangée comme mesure source. Mêmes événements `feuille_fd9`, même
/// annulation et même calage que « Mesurer une feuille au FD-9 ».
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn seance_mesurer_exemplaire(
    app: AppHandle,
    instruments: State<'_, Instruments>,
    bibliotheque: State<'_, BibliothequeOuverte>,
    dossier: State<'_, DossierBibliotheque>,
    annulation: State<'_, AnnulationFeuille>,
    calage: State<'_, CalageAffiche>,
    demonstration: State<'_, DemonstrationFd9>,
    mire: State<'_, MireFeuille>,
    condition: i64,
    seance: i64,
) -> Result<EcranFeuille, String> {
    let etats = Etats {
        instruments: &instruments,
        bibliotheque: &bibliotheque,
        dossier: &dossier,
        annulation: &annulation,
        calage: &calage,
        demonstration: &demonstration,
        mire: &mire,
    };
    mesurer_au_fd9(
        &app,
        &etats,
        condition,
        Some(bibliotheque::IdSeance(seance)),
    )
}

/// Les états de l'application dont une feuille a besoin.
struct Etats<'a> {
    instruments: &'a Instruments,
    bibliotheque: &'a BibliothequeOuverte,
    dossier: &'a DossierBibliotheque,
    annulation: &'a AnnulationFeuille,
    calage: &'a CalageAffiche,
    demonstration: &'a DemonstrationFd9,
    mire: &'a MireFeuille,
}

/// Une feuille au FD-9, d'une résolution (`seance` absente) ou d'une séance.
fn mesurer_au_fd9(
    app: &AppHandle,
    etats: &Etats,
    condition: i64,
    seance: Option<bibliotheque::IdSeance>,
) -> Result<EcranFeuille, String> {
    let Etats {
        instruments,
        bibliotheque,
        dossier,
        annulation,
        calage,
        demonstration,
        mire,
    } = *etats;
    let _jeton = instruments
        .occupation
        .commencer(Operation::Mesure)
        .map_err(|refus| refus.cle())?;
    let mut suivi = |etape: EtapeFeuille| {
        let numero = match &etape {
            EtapeFeuille::CalagePret { calage: c } => Some(calage.montrer(c.clone())),
            _ => None,
        };
        let _ = app.emit(
            "feuille_fd9",
            EvenementFeuille {
                condition,
                numero,
                etape: &etape,
            },
        );
    };
    let poser = |a: pont::Annulation| {
        *annulation.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(a);
    };
    let condition_id = bibliotheque::IdCondition(condition);
    let demande = |mire: &dyn Fn() -> Result<MireFd9, String>| match seance {
        Some(seance) => Demande::Exemplaire { seance },
        None => Demande::Feuille {
            condition: condition_id,
            mire: mire(),
        },
    };
    let (resultat, instrument) = match &demonstration.0 {
        Some(demo) => {
            let mut fd9 = demo.lock().unwrap_or_else(|e| e.into_inner());
            poser(fd9.annulation());
            let issue = IssueDemonstration::du_lancement();
            let resultat = mesurer_avec(
                bibliotheque,
                demande(&|| Ok(demonstration_fd9::mire())),
                &mut fd9,
                &dossier.0,
                &mut demonstration_fd9::lanceur(issue, demonstration_fd9::RYTHME),
                &mut suivi,
            );
            (resultat, None)
        }
        None => {
            let mut courant = instruments
                .selection
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let selection = courant
                .as_mut()
                .ok_or_else(|| "refus.occupe.recherche".to_string())?;
            poser(selection.instrument().annulation());
            let resultat = mesurer_avec(
                bibliotheque,
                demande(&|| mire.lue().map(|m| MireFd9::clone(&m))),
                selection.instrument_mut(),
                &dossier.0,
                // Le pont de la feuille trace dans `mesures-fd9/traces/`.
                &mut mesurer_feuille::lanceur_feuille(&dossier.0),
                &mut suivi,
            );
            (resultat, Some(selection.vue()))
        }
    };
    annulation
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    let (rangee, source, echec) = match resultat {
        Ok(Rangement::Feuille(rangee)) => (Some(rangee), None, None),
        Ok(Rangement::Exemplaire(source)) => (None, Some(source), None),
        Err(echec) => (None, None, Some(echec)),
    };
    Ok(EcranFeuille {
        rangee,
        source,
        echec,
        instrument,
    })
}

/// Bouton « Annuler », de l'attente de la feuille à la fin de la mesure des
/// plages (la feuille est alors perdue) ; sans
/// feuille en cours, rien ne se passe.
#[tauri::command]
pub(crate) fn annuler_feuille_fd9(annulation: State<'_, AnnulationFeuille>) -> bool {
    annulation
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(pont::Annulation::annuler)
}

/// Le dernier calage gardé dans la résolution `condition`, devenu celui de
/// l'onglet « Points mesurés » : son numéro, la mesure et le calage sans ses
/// pixels. `None` si la résolution n'en a pas.
#[tauri::command(async)]
pub(crate) fn feuille_fd9_dernier_calage(
    bibliotheque: State<'_, BibliothequeOuverte>,
    dossier: State<'_, DossierBibliotheque>,
    calage: State<'_, CalageAffiche>,
    condition: i64,
) -> Option<serde_json::Value> {
    let (mesure, dernier) =
        bibliotheque.dernier_calage_fd9(bibliotheque::IdCondition(condition), &dossier.0)?;
    let vue = serde_json::to_value(VueCalage::de(&dernier)).ok()?;
    let numero = calage.montrer(dernier);
    Some(serde_json::json!({ "numero": numero, "mesure": mesure.0, "calage": vue }))
}

/// Pixels de l'image du calage n° `numero`, en binaire : largeur, hauteur
/// et ppp (trois entiers de 32 bits, petit-boutiste), puis un octet de gris
/// par pixel, rangée après rangée. Vide si ce calage n'est plus celui
/// montré, ou s'il n'a pas d'image.
#[tauri::command]
pub(crate) fn feuille_fd9_image(calage: State<'_, CalageAffiche>, numero: u64) -> Response {
    let courant = calage.0.lock().unwrap_or_else(|e| e.into_inner());
    let image = match &*courant {
        (n, Some(c)) if *n == numero => c.image.as_ref(),
        _ => None,
    };
    Response::new(image.map_or_else(Vec::new, |image| {
        let mut octets = Vec::with_capacity(12 + image.gris().len());
        for n in [image.largeur(), image.hauteur(), image.ppp()] {
            octets.extend_from_slice(&n.to_le_bytes());
        }
        octets.extend_from_slice(image.gris());
        octets
    }))
}
