//! Rappel d'événements de `FD9SDK.dll` (fiche
//! `docs/abi/FD9_RegisterDeviceEventHandler.md`).
//!
//! La DLL appelle [`rappel`] depuis un de ses fils, en `__cdecl`. Le rappel ne
//! parle jamais à la DLL : il pose l'événement dans un canal et rend la main.
//! Toute la séquence se joue dans le fil de la session. Un rappel reçu sans
//! canal ouvert (avant l'inscription ou après la désinscription) est compté,
//! jamais perdu en silence.

use crate::{trace, Evenement};
use fd9_sys::EtatSdk;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

/// Canal vers la session ouverte ; `None` sans session.
static CANAL: Mutex<Option<Sender<Evenement>>> = Mutex::new(None);

/// Rappels reçus sans canal ouvert.
static ORPHELINS: AtomicU64 = AtomicU64::new(0);

/// Trace de chaque entrée et sortie du rappel (`--trace`).
static TRACE: AtomicBool = AtomicBool::new(false);

/// Écrit sur la sortie d'erreur, au fil de l'eau, chaque entrée et sortie du
/// rappel, avec son fil (celui de la DLL).
pub fn tracer(actif: bool) {
    TRACE.store(actif, Ordering::SeqCst);
}

/// Une ligne de trace, sans jamais paniquer : une panique ici traverserait la
/// DLL et arrêterait le pont.
fn ecrire_trace(texte: std::fmt::Arguments) {
    if TRACE.load(Ordering::SeqCst) {
        let _ = writeln!(std::io::stderr(), "pont-fd9 : {} {texte}", trace::entete());
    }
}

/// Fonction donnée à `FD9_RegisterDeviceEventHandler`. Statique : la DLL
/// garde son adresse jusqu'à la désinscription.
///
/// # Safety
///
/// Appelée par la DLL avec quatre entiers de 32 bits ; elle ne lit aucun
/// pointeur.
pub unsafe extern "C" fn rappel(etat: u32, travail: u32, page: u32, compte: u32) {
    ecrire_trace(format_args!(
        "→ rappel(état {etat}, travail {travail}, page {page}, compte {compte})"
    ));
    let evenement = Evenement {
        etat: EtatSdk::depuis(etat),
        travail,
        page,
        compte,
    };
    // Un verrou empoisonné reste utilisable : rien ne doit paniquer ici, la
    // panique traverserait la DLL.
    let canal = CANAL.lock().unwrap_or_else(|e| e.into_inner());
    let recu = canal
        .as_ref()
        .is_some_and(|envoi| envoi.send(evenement).is_ok());
    drop(canal);
    if !recu {
        ORPHELINS.fetch_add(1, Ordering::SeqCst);
    }
    ecrire_trace(format_args!(
        "← rappel(état {etat}){}",
        if recu { "" } else { " : sans session" }
    ));
}

/// Ouvre un canal neuf, avant l'inscription du rappel. Un canal précédent est
/// fermé : ses événements non lus sont perdus pour la session close.
pub fn ouvrir() -> Receiver<Evenement> {
    let (envoi, reception) = channel();
    *CANAL.lock().unwrap_or_else(|e| e.into_inner()) = Some(envoi);
    reception
}

/// Ferme le canal, après la désinscription du rappel.
pub fn fermer() {
    *CANAL.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Rappels reçus sans canal ouvert depuis le lancement du pont.
pub fn orphelins() -> u64 {
    ORPHELINS.load(Ordering::SeqCst)
}
