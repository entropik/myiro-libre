//! Attente des événements du rappel pendant une feuille, délais et
//! annulation. La session n'avance que sur un événement reçu : un délai
//! dépassé ou une annulation sont rendus comme tels. Seule exception :
//! pendant la mesure des plages, un silence du rappel fait relire l'état
//! (`FD9_GetStatus`, sans question à l'instrument, essai E5).

use crate::{Evenement, SdkFd9, Session};
use fd9_sys::EtatSdk;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Pendant une attente, le pont regarde au moins aussi souvent si
/// l'annulation est demandée.
const PAS_ATTENTE: Duration = Duration::from_millis(100);

/// Après `FD9_Disconnect`, attente de l'état 1 ou du code 1102. MYIROtools
/// attend sans limite ; le pont s'arrête là.
pub const ATTENTE_APRES_DECONNEXION: Duration = Duration::from_secs(10);

/// Délais d'une feuille, et de la fin d'une déconnexion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delais {
    /// Attente de la feuille après la requête (`--attente-feuille`).
    pub feuille: Duration,
    /// Attente de chaque étape suivante (capture, reconnaissance, arrêt).
    pub etape: Duration,
    /// Attente de la fin de la mesure de toutes les plages.
    pub mesure: Duration,
    /// Pendant la mesure, silence du rappel au bout duquel l'état est relu
    /// (`FD9_GetStatus`), puis relu à chaque nouveau silence : à l'essai E5,
    /// la DLL a envoyé 976 événements 11 puis plus rien, ni état 9 ni
    /// erreur.
    pub silence: Duration,
    /// Attente, après la déconnexion, de l'état 1 ou du code 1102
    /// ([`ATTENTE_APRES_DECONNEXION`]).
    pub deconnexion: Duration,
}

impl Default for Delais {
    fn default() -> Self {
        Delais {
            feuille: Duration::from_secs(120),
            etape: Duration::from_secs(30),
            // Une RGB-2520 s'est mesurée en quelques minutes (essai E4) ;
            // 20 min laissent de la marge à une feuille lente.
            mesure: Duration::from_secs(20 * 60),
            silence: Duration::from_secs(60),
            deconnexion: ATTENTE_APRES_DECONNEXION,
        }
    }
}

/// Part de la garde d'une feuille au-delà de ses attentes : lecture des
/// plages, calculs et sortie (2 520 plages lues en peu de temps à l'essai E4,
/// supposé bien moins de 4 min).
pub const MARGE_GARDE_FEUILLE: Duration = Duration::from_secs(4 * 60);

impl Delais {
    /// Délai de la garde d'une feuille (`serveur::Garde::avec_feuille`) :
    /// attente de la feuille, quatre étapes (engagement, capture,
    /// reconnaissance, arrêt), mesure, plus [`MARGE_GARDE_FEUILLE`]. Par
    /// défaut 28 min : moins que les 30 min que l'application laisse à une
    /// feuille (`DELAI_FEUILLE`), qui reçoit ainsi la réponse de la garde
    /// avant de couper le pont elle-même.
    pub fn garde_feuille(&self) -> Duration {
        self.feuille + self.etape * 4 + self.mesure + MARGE_GARDE_FEUILLE
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
        // Dernier signe de l'instrument : un événement, ou une relecture.
        let mut dernier_signe = Instant::now();
        loop {
            if !self.recus.is_empty() {
                dernier_signe = Instant::now();
            }
            while let Some(evenement) = self.recus.pop_front() {
                self.journaliser(&evenement, "");
                match evenement.etat {
                    EtatSdk::Alimentation | EtatSdk::AlimentationFinie => {
                        self.annoncer_engagement()
                    }
                    EtatSdk::AvisPlages => self.annoncer_compte(evenement.compte),
                    _ => {}
                }
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
            if attente == Attente::Mesure && dernier_signe.elapsed() >= self.delais.silence {
                let silence = dernier_signe.elapsed();
                dernier_signe = Instant::now();
                if let Some(conclusion) = self.relire_pendant_la_mesure(voulu, silence) {
                    return conclusion;
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

    /// Le rappel se tait depuis `silence` pendant la mesure : l'état est relu
    /// (`FD9_GetStatus`, +0x10), sans question à l'instrument. `voulu`
    /// atteint : la mesure est finie ; état d'erreur : incident ; sinon
    /// (`None`), l'attente continue. Un refus de la lecture est noté, et
    /// l'attente continue aussi.
    fn relire_pendant_la_mesure(
        &mut self,
        voulu: EtatSdk,
        silence: Duration,
    ) -> Option<Result<(), Fin>> {
        let mut statut = [0u32; 8];
        let code = self.sdk.statut(&mut statut);
        let secondes = silence.as_secs();
        if code != 0 {
            self.journal.push(format!(
                "silence de {secondes} s pendant la mesure : FD9_GetStatus code {code} ; l'attente continue"
            ));
            return None;
        }
        let etat = EtatSdk::depuis(statut[4]);
        self.journal.push(format!(
            "silence de {secondes} s pendant la mesure : FD9_GetStatus code {code}, état {} ({statut:?})",
            etat.code()
        ));
        if etat == voulu {
            self.dernier_etat = Some(etat);
            self.journal
                .push("mesure finie sans événement du rappel : lecture des plages".into());
            Some(Ok(()))
        } else if etat.est_erreur() {
            self.dernier_etat = Some(etat);
            Some(Err(Fin::Incident(etat)))
        } else {
            None
        }
    }

    /// Note l'événement au journal (`contexte` après « événement ») et en
    /// garde l'état comme dernier état connu, sauf pour un avis de plages.
    pub(crate) fn journaliser(&mut self, e: &Evenement, contexte: &str) {
        if e.etat != EtatSdk::AvisPlages {
            self.dernier_etat = Some(e.etat);
        }
        let precision = match e.etat {
            // Essai E3 : 0 puis 1 juste après la connexion, sans autre
            // logiciel ouvert.
            EtatSdk::AttenteSansMain => {
                " : main pas encore obtenue (passage normal juste après la connexion ; s'il dure, un autre logiciel tient peut-être l'instrument)"
            }
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
