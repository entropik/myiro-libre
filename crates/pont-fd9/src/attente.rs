//! Attente des événements du rappel pendant une feuille, délais et
//! annulation. La session n'avance que sur un événement reçu : un délai
//! dépassé ou une annulation sont rendus comme tels.

use crate::{Evenement, SdkFd9, Session};
use fd9_sys::EtatSdk;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Pendant une attente, le pont regarde au moins aussi souvent si
/// l'annulation est demandée.
const PAS_ATTENTE: Duration = Duration::from_millis(100);

/// Délais d'une feuille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delais {
    /// Attente de la feuille après la requête (`--attente-feuille`).
    pub feuille: Duration,
    /// Attente de chaque étape suivante (capture, reconnaissance, arrêt).
    pub etape: Duration,
    /// Attente de la fin de la mesure de toutes les plages.
    pub mesure: Duration,
}

impl Default for Delais {
    fn default() -> Self {
        Delais {
            feuille: Duration::from_secs(120),
            etape: Duration::from_secs(30),
            // Supposé : une RGB-2520 se mesure en quelques minutes (essai E4).
            mesure: Duration::from_secs(600),
        }
    }
}

/// Demande d'annulation, partagée entre le fil qui lit les requêtes et la
/// session qui attend l'instrument (ticket #26).
#[derive(Clone, Debug, Default)]
pub struct Annulation(Arc<Drapeaux>);

#[derive(Debug, Default)]
struct Drapeaux {
    demandee: AtomicBool,
    entree_fermee: AtomicBool,
}

impl Annulation {
    /// L'opérateur annule la feuille en cours.
    pub fn annuler(&self) {
        self.0.demandee.store(true, Ordering::SeqCst);
    }

    /// Plus personne n'attend de réponse : une feuille pas encore glissée
    /// n'est plus attendue.
    pub fn fermer_entree(&self) {
        self.0.entree_fermee.store(true, Ordering::SeqCst);
    }

    pub fn demandee(&self) -> bool {
        self.0.demandee.load(Ordering::SeqCst)
    }

    fn entree_fermee(&self) -> bool {
        self.0.entree_fermee.load(Ordering::SeqCst)
    }

    /// La requête visée est terminée : l'annulation ne vaut plus.
    pub fn effacer(&self) {
        self.0.demandee.store(false, Ordering::SeqCst);
    }
}

/// Ce qu'une attente peut annuler.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attente {
    /// La feuille : annulable, et abandonnée si l'entrée se ferme avant
    /// qu'elle s'engage.
    Feuille,
    /// Une étape : annulable.
    Etape,
    /// La mesure des plages : annulable, avec son propre délai.
    Mesure,
    /// La fin d'un arrêt : rien ne l'interrompt.
    Arret,
}

/// Pourquoi une attente a fini sans l'état voulu.
pub(crate) enum Fin {
    Delai,
    Annulee,
    Incident(EtatSdk),
    Inconnu(u32),
}

impl<S: SdkFd9> Session<S> {
    /// Lit les événements jusqu'à `voulu`. Les suivants restent pour la
    /// prochaine attente.
    pub(crate) fn attendre(&mut self, voulu: EtatSdk, attente: Attente) -> Result<(), Fin> {
        let delai = match attente {
            Attente::Feuille => self.delais.feuille,
            Attente::Etape | Attente::Arret => self.delais.etape,
            Attente::Mesure => self.delais.mesure,
        };
        let fin = Instant::now() + delai;
        let mut engagee = false;
        let mut rien_recu = false;
        loop {
            while let Some(evenement) = self.recus.pop_front() {
                self.journaliser(&evenement, "");
                match evenement.etat {
                    etat if etat == voulu => return Ok(()),
                    // L'arrêt a croisé la fin de la mesure : la feuille est
                    // finie, et `ExitMeasurement` est admis à l'état 9 comme
                    // à l'état 10.
                    EtatSdk::MesureFinie if attente == Attente::Arret => return Ok(()),
                    EtatSdk::Inconnu(code) => return Err(Fin::Inconnu(code)),
                    etat if etat.est_erreur() => return Err(Fin::Incident(etat)),
                    EtatSdk::Alimentation => engagee = true,
                    _ => {}
                }
            }
            if attente != Attente::Arret {
                let abandonnee = attente == Attente::Feuille
                    && !engagee
                    && rien_recu
                    && self.annulation.entree_fermee();
                if self.annulation.demandee() || abandonnee {
                    self.journal.push("annulation reçue".into());
                    return Err(Fin::Annulee);
                }
            }
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                self.journal.push(format!(
                    "délai dépassé en attendant l'état {}",
                    voulu.code()
                ));
                return Err(Fin::Delai);
            }
            let recus = self.sdk.evenements(reste.min(PAS_ATTENTE));
            rien_recu = recus.is_empty();
            self.recus.extend(recus);
        }
    }

    /// Note l'événement au journal (`contexte` après « événement ») et en
    /// garde l'état comme dernier état connu, sauf pour un avis de plages.
    pub(crate) fn journaliser(&mut self, e: &Evenement, contexte: &str) {
        if e.etat != EtatSdk::AvisPlages {
            self.dernier_etat = Some(e.etat);
        }
        let precision = match e.etat {
            EtatSdk::AttenteSansMain => " : un autre logiciel a la main sur l'instrument",
            _ => "",
        };
        self.journal.push(format!(
            "événement{contexte} : état {} (travail {}, page {}, compte {}){precision}",
            e.etat.code(),
            e.travail,
            e.page,
            e.compte
        ));
    }
}
