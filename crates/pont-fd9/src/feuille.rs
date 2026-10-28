//! Déroulé d'une feuille au FD-9 : attente de la feuille, capture, QR,
//! reconnaissance des plages, puis arrêt (palier Prénumérisation). Séquence
//! de MYIROtools et états admis : fiche `docs/abi/FD9SDK.md`, section
//! « Mesurer une feuille ».
//!
//! Un **arrêt** est `FD9_StopMeasurement(faux)` (états 1 à 8), puis l'attente
//! de l'événement 10, puis `FD9_ExitMeasurement(0, NULL)`. Un événement
//! d'erreur sort sans arrêter. La session n'avance que sur un événement reçu
//! ou un code de retour lu.

use crate::attente::{Attente, Fin};
use crate::connexion::inattendue;
use crate::{SdkFd9, Session};
use fd9_sys::{CoordonneesMire, DonneesQr, EtatSdk, ResultatQr, ZONES_MAX};
use pont_protocole::{DefinitionFeuille, ErreurPont, Info, LectureFeuille, Palier, RemiseAuRepos};

/// Écart d'origine ou de pas admis entre la reconnaissance et la
/// définition : 3 mm, en 1/100 mm.
pub const ECART_ADMIS: u32 = 300;

/// Plus grande image acceptée de `FD9_GetCaptureResult`, en octets : une
/// taille au-delà est une réponse inattendue, pas une allocation.
pub const TAILLE_IMAGE_MAX: u32 = 256 << 20;

/// Échec d'un arrêt.
enum EchecArret {
    /// `FD9_StopMeasurement` refusé (code brut) : rien n'a été arrêté ni
    /// clos, par exemple hors des états 1 à 8.
    Refuse(i32),
    /// Arrêt admis, puis délai, incident ou sortie refusée.
    Apres(ErreurPont),
}

impl EchecArret {
    fn erreur(self) -> ErreurPont {
        match self {
            EchecArret::Refuse(code) => erreur_sdk(code),
            EchecArret::Apres(erreur) => erreur,
        }
    }
}

/// Codes publics de la fiche `FD9SDK.md` qui ont un nom dans le protocole.
fn erreur_sdk(code: i32) -> ErreurPont {
    match code {
        1001 => ErreurPont::ParametreRefuse {},
        1104 => ErreurPont::EtatIncompatible {},
        code => ErreurPont::Sdk { code },
    }
}

impl<S: SdkFd9> Session<S> {
    /// Capture la feuille glissée, lit son QR et reconnaît ses plages selon
    /// `mire`, puis la rend sans la mesurer.
    pub fn prenumeriser(&mut self, mire: &DefinitionFeuille) -> Result<LectureFeuille, ErreurPont> {
        self.autoriser(Palier::Prenumerisation, Some(Palier::Connexion))?;
        self.exiger_connexion()?;
        let coordonnees = CoordonneesMire::depuis_definition(mire).map_err(|e| {
            self.journal
                .push(format!("définition de feuille refusée : {e:?}"));
            ErreurPont::ParametreRefuse {}
        })?;
        self.vider_hors_requete();
        self.recus.clear();
        let lecture = self.lire_feuille(mire, &coordonnees)?;
        self.arreter_feuille().map_err(EchecArret::erreur)?;
        self.franchir(Palier::Prenumerisation);
        Ok(lecture)
    }

    fn exiger_connexion(&self) -> Result<(), ErreurPont> {
        if self.perdu {
            Err(ErreurPont::InstrumentPerdu {})
        } else if !self.connecte {
            Err(ErreurPont::EtatInvalide {
                attendu: Palier::Connexion,
            })
        } else {
            Ok(())
        }
    }

    /// Étapes communes aux paliers Prénumérisation et Feuille, jusqu'à la
    /// reconnaissance contrôlée. Toute sortie en erreur a déjà arrêté ou
    /// clos la feuille.
    fn lire_feuille(
        &mut self,
        mire: &DefinitionFeuille,
        coordonnees: &CoordonneesMire,
    ) -> Result<LectureFeuille, ErreurPont> {
        self.attendre_ou_conclure(EtatSdk::AlimentationFinie, Attente::Feuille)?;
        let code = self.sdk.capturer();
        self.verifier("FD9_StartCapture", code)?;
        self.attendre_ou_conclure(EtatSdk::CaptureFinie, Attente::Etape)?;
        let qr = self.lire_capture()?;
        if let Info::Confirmee(texte) = &qr {
            if !texte.starts_with(&mire.nom) {
                let raison = ErreurPont::AutreMire {
                    texte: texte.clone(),
                };
                return Err(self.abandonner(raison));
            }
        }
        let code = self.sdk.reconnaitre(self.reconnaissance, coordonnees);
        self.verifier("FD9_StartPatchRecognition", code)?;
        self.attendre_ou_conclure(EtatSdk::ReconnaissanceFinie, Attente::Etape)?;
        let mut lue = CoordonneesMire::vide();
        let mut fiabilite = 0u32;
        let code = self.sdk.resultat_reconnaissance(&mut fiabilite, &mut lue);
        self.verifier("FD9_GetPatchRecognitionResult2", code)?;
        let (conforme, ecart_max) = comparer(coordonnees, &lue);
        self.journal.push(format!(
            "reconnaissance : {} zone(s), {} plage(s), fiabilité {fiabilite}, écart {ecart_max}",
            lue.zones, lue.plages
        ));
        if !conforme || ecart_max > ECART_ADMIS {
            let raison = ErreurPont::MireNonReconnue {
                zones: lue.zones,
                plages: lue.plages,
                ecart_max,
            };
            return Err(self.abandonner(raison));
        }
        Ok(LectureFeuille {
            qr,
            zones: lue.zones,
            plages: lue.plages,
            fiabilite,
            ecart_max,
        })
    }

    /// QR et image, en deux appels : la taille, puis l'image dans un tampon
    /// de cette taille. L'image n'est gardée que le temps de l'appel.
    fn lire_capture(&mut self) -> Result<Info<String>, ErreurPont> {
        let mut qr = DonneesQr::vide();
        let (mut resultat, mut taille) = (0u32, 0u32);
        let code = self
            .sdk
            .resultat_capture(&mut resultat, &mut qr, &mut taille, None);
        self.verifier("FD9_GetCaptureResult (taille)", code)?;
        if taille > TAILLE_IMAGE_MAX {
            let raison = inattendue(format!("image de {taille} octets annoncée"));
            return Err(self.abandonner(raison));
        }
        let mut image = vec![0u8; taille as usize];
        let code = self
            .sdk
            .resultat_capture(&mut resultat, &mut qr, &mut taille, Some(&mut image));
        self.verifier("FD9_GetCaptureResult (image)", code)?;
        drop(image);
        // Seul « FD-9 » compte : « FAIL » est rendu comme trouvé (fiche).
        let lu = ResultatQr::depuis(resultat) == ResultatQr::Trouve && qr.appareil() == "FD-9";
        self.journal.push(format!(
            "FD9_GetCaptureResult : QR {resultat}, appareil « {} », image de {taille} octets{}",
            qr.appareil(),
            if lu { "" } else { ", QR non lu" }
        ));
        Ok(if lu {
            Info::Confirmee(qr.texte())
        } else {
            Info::Inconnue
        })
    }

    /// Un code non nul arrête la feuille et rend l'erreur du SDK.
    fn verifier(&mut self, appel: &str, code: i32) -> Result<(), ErreurPont> {
        if code == 0 {
            return Ok(());
        }
        let interne = self.sdk.dernier_code();
        self.journal
            .push(format!("{appel} : code {code}, code interne {interne}"));
        Err(self.abandonner(erreur_sdk(code)))
    }

    /// Arrête la feuille, puis rend `raison`. Un arrêt refusé d'emblée n'a
    /// rien changé : la raison reste, et une annulation dit que l'arrêt a été
    /// refusé. Un arrêt admis qui échoue ensuite rend son échec.
    fn abandonner(&mut self, raison: ErreurPont) -> ErreurPont {
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
    fn arreter_feuille(&mut self) -> Result<(), EchecArret> {
        let code = self.sdk.arreter(false);
        self.journal
            .push(format!("FD9_StopMeasurement(faux) : {code}"));
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!("code interne {interne}"));
            return Err(EchecArret::Refuse(code));
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

    fn sortir(&mut self) -> Result<(), ErreurPont> {
        let code = self.sdk.sortir();
        self.journal
            .push(format!("FD9_ExitMeasurement(0, NULL) : {code}"));
        if code == 0 {
            Ok(())
        } else {
            Err(erreur_sdk(code))
        }
    }

    /// Attend `voulu`, ou conclut la feuille selon ce qui l'a interrompue.
    fn attendre_ou_conclure(&mut self, voulu: EtatSdk, attente: Attente) -> Result<(), ErreurPont> {
        match self.attendre(voulu, attente) {
            Ok(()) => Ok(()),
            Err(Fin::Delai) if attente == Attente::Feuille => {
                Err(self.abandonner(ErreurPont::FeuilleAbsente {}))
            }
            Err(Fin::Delai) => Err(self.abandonner(ErreurPont::Delai {})),
            Err(Fin::Annulee) => Err(self.abandonner(ErreurPont::MesureAnnulee {
                remise_au_repos: RemiseAuRepos::AuRepos {},
            })),
            Err(Fin::Incident(etat)) => Err(self.incident(etat)),
            Err(Fin::Inconnu(code)) => Err(self.etat_inconnu(code)),
        }
    }

    /// Événement d'erreur : la feuille sort sans arrêter (états de fin admis
    /// par `FD9_ExitMeasurement`), après la cause de l'échec de préparation
    /// pour 201.
    fn incident(&mut self, etat: EtatSdk) -> ErreurPont {
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
    fn etat_inconnu(&mut self, code: u32) -> ErreurPont {
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

/// Zones et plages égales à la définition (comme MYIROtools), et plus grand
/// écart d'origine ou de pas sur les zones communes.
fn comparer(definition: &CoordonneesMire, lue: &CoordonneesMire) -> (bool, u32) {
    let communes = (definition.zones.min(lue.zones) as usize).min(ZONES_MAX);
    let conforme = lue.zones == definition.zones
        && lue.plages == definition.plages
        && definition.compte_zones[..communes] == lue.compte_zones[..communes];
    let ecart_max = definition.geometries[..communes]
        .iter()
        .zip(&lue.geometries[..communes])
        .flat_map(|(d, l)| {
            [
                d.gauche.abs_diff(l.gauche),
                d.haut.abs_diff(l.haut),
                d.pas_x.abs_diff(l.pas_x),
                d.pas_y.abs_diff(l.pas_y),
            ]
        })
        .max()
        .unwrap_or(0);
    (conforme, ecart_max)
}
