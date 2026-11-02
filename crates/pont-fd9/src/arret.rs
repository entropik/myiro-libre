//! Arrêt et clôture d'une feuille au FD-9 (fiches
//! `docs/abi/FD9_StopMeasurement.md` et `FD9_ExitMeasurement.md`). Un arrêt
//! est `FD9_StopMeasurement(faux)` (états 1 à 8), puis l'attente de
//! l'événement 10, puis `FD9_ExitMeasurement(0, NULL)`. Un événement d'erreur
//! sort sans arrêter.

use crate::attente::{Attente, Fin};
use crate::connexion::inattendue;
use crate::feuille::erreur_sdk;
use crate::{Evenement, SdkFd9, Session};
use fd9_sys::EtatSdk;
use pont_protocole::{ErreurPont, RemiseAuRepos};
use std::time::{Duration, Instant};

/// Pas de lecture des événements après un arrêt refusé.
const PAS_ARRET_REFUSE: Duration = Duration::from_millis(100);

/// Échec d'un arrêt.
pub(crate) enum EchecArret {
    /// `FD9_StopMeasurement` refusé (code brut) : rien n'a été arrêté ni
    /// clos, par exemple hors des états 1 à 8.
    Refuse(i32),
    /// Arrêt admis, puis délai, incident ou sortie refusée.
    Apres(ErreurPont),
}

impl EchecArret {
    pub(crate) fn erreur(self) -> ErreurPont {
        match self {
            EchecArret::Refuse(code) => erreur_sdk(code),
            EchecArret::Apres(erreur) => erreur,
        }
    }
}

/// Après un arrêt refusé, temps laissé au rappel pour annoncer l'état où la
/// DLL est déjà passée (fin de mesure, incident), avant de conclure.
pub const ATTENTE_APRES_ARRET_REFUSE: Duration = Duration::from_secs(2);

impl<S: SdkFd9> Session<S> {
    /// Arrête la feuille, puis rend `raison`. Un arrêt refusé d'emblée n'a
    /// rien changé : la raison reste, et une annulation dit que l'arrêt a été
    /// refusé. Un arrêt admis qui échoue ensuite rend son échec.
    pub(crate) fn abandonner(&mut self, raison: ErreurPont) -> ErreurPont {
        match self.arreter_feuille() {
            Ok(()) => raison,
            Err(EchecArret::Refuse(code)) => {
                self.journal
                    .push(format!("arrêt refusé ({code}) après {raison:?}"));
                match raison {
                    ErreurPont::MesureAnnulee { .. } => ErreurPont::MesureAnnulee {
                        remise_au_repos: RemiseAuRepos::ArretRefuse { code },
                    },
                    raison => raison,
                }
            }
            Err(EchecArret::Apres(echec)) => {
                self.journal
                    .push(format!("arrêt après {raison:?} : {echec:?}"));
                echec
            }
        }
    }

    /// `FD9_StopMeasurement(faux)`, événement 10, puis `FD9_ExitMeasurement`.
    /// Sans événement 10 : `Delai`, sans sortir.
    pub(crate) fn arreter_feuille(&mut self) -> Result<(), EchecArret> {
        let code = self.sdk.arreter(false);
        self.journal
            .push(format!("FD9_StopMeasurement(faux) : {code}"));
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!("code interne {interne}"));
            return self.apres_arret_refuse(code);
        }
        let fin = match self.attendre(EtatSdk::AnnulationFinie, Attente::Arret) {
            Ok(()) => self.sortir(),
            Err(Fin::Incident(etat)) => Err(self.incident(etat)),
            Err(Fin::Inconnu(code)) => Err(inattendue(format!(
                "état {code} du SDK inconnu pendant l'arrêt"
            ))),
            Err(Fin::Delai | Fin::Annulee) => Err(ErreurPont::Delai {}),
        };
        fin.map_err(EchecArret::Apres)
    }

    /// L'arrêt a pu croiser la fin de la feuille : les événements pas encore
    /// traités, puis ceux qui arrivent pendant [`ATTENTE_APRES_ARRET_REFUSE`],
    /// disent où elle en est. Mesure finie ou arrêt fini (9, 10) : la feuille
    /// est close par `ExitMeasurement`. Incident : il est traité et rendu.
    /// Sinon, rien n'a changé.
    pub(crate) fn apres_arret_refuse(&mut self, code: i32) -> Result<(), EchecArret> {
        let fin = Instant::now() + ATTENTE_APRES_ARRET_REFUSE;
        let mut recus: Vec<Evenement> = self.recus.drain(..).collect();
        loop {
            for e in recus {
                self.journaliser(&e, " reçu pendant l'arrêt");
            }
            let conclu = matches!(
                self.dernier_etat,
                Some(EtatSdk::MesureFinie | EtatSdk::AnnulationFinie)
            ) || self.dernier_etat.is_some_and(|e| e.est_erreur());
            let reste = fin.saturating_duration_since(Instant::now());
            if conclu || reste.is_zero() {
                break;
            }
            recus = self.sdk.evenements(reste.min(PAS_ARRET_REFUSE));
        }
        match self.dernier_etat {
            Some(EtatSdk::MesureFinie | EtatSdk::AnnulationFinie) => {
                self.sortir().map_err(EchecArret::Apres)
            }
            Some(etat) if etat.est_erreur() => Err(EchecArret::Apres(self.incident(etat))),
            _ => Err(EchecArret::Refuse(code)),
        }
    }

    pub(crate) fn sortir(&mut self) -> Result<(), ErreurPont> {
        let code = self.sdk.sortir();
        self.journal
            .push(format!("FD9_ExitMeasurement(0, NULL) : {code}"));
        if code == 0 {
            Ok(())
        } else {
            Err(erreur_sdk(code))
        }
    }

    /// Événement d'erreur : la feuille sort sans arrêter (états de fin admis
    /// par `FD9_ExitMeasurement`), après la cause de l'échec de préparation
    /// pour 201.
    pub(crate) fn incident(&mut self, etat: EtatSdk) -> ErreurPont {
        let premesure = (etat == EtatSdk::Premesure).then(|| {
            let mut systeme = 0i32;
            let cause = self.sdk.erreur_premesure(&mut systeme);
            self.journal.push(format!(
                "FD9_GetPremeasurementError : cause {cause}, erreur système {systeme}"
            ));
            cause
        });
        if let Err(erreur) = self.sortir() {
            self.journal
                .push(format!("sortie après l'état {} : {erreur:?}", etat.code()));
        }
        if etat == EtatSdk::Communication {
            self.perdu = true;
            return ErreurPont::InstrumentPerdu {};
        }
        ErreurPont::IncidentFeuille {
            etat: etat.code(),
            premesure,
        }
    }

    /// État absent de la table : arrêt si le SDK l'admet, sinon sortie, sans
    /// rien en déduire.
    pub(crate) fn etat_inconnu(&mut self, code: u32) -> ErreurPont {
        let arret = self.sdk.arreter(false);
        self.journal
            .push(format!("FD9_StopMeasurement(faux) : {arret}"));
        if arret == 0 {
            if self
                .attendre(EtatSdk::AnnulationFinie, Attente::Arret)
                .is_ok()
            {
                let _ = self.sortir();
            }
        } else {
            let _ = self.sortir();
        }
        inattendue(format!("état {code} du SDK inconnu"))
    }
}
