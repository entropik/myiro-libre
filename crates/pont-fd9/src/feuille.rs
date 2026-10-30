//! Déroulé d'une feuille au FD-9 : attente de la feuille, capture, QR,
//! reconnaissance des plages, puis arrêt (palier Prénumérisation). Séquence
//! de MYIROtools et états admis : fiche `docs/abi/FD9SDK.md`, section
//! « Mesurer une feuille ».
//!
//! Un **arrêt** est `FD9_StopMeasurement(faux)` (états 1 à 8), puis l'attente
//! de l'événement 10, puis `FD9_ExitMeasurement(0, NULL)`. Un événement
//! d'erreur sort sans arrêter. Une attente de la feuille finie sans feuille
//! (délai, annulation) n'arrête rien si l'état relu est 0 ou 1, comme
//! MYIROtools (essai E2). La session n'avance que sur un événement reçu ou
//! un code de retour lu.

use crate::arret::EchecArret;
use crate::attente::{Attente, Fin};
use crate::calage::{caler, DECALAGE_ADMIS};
use crate::composants::{composant_qr, DossierDuPont, COMPOSANT_QR};
use crate::connexion::inattendue;
use crate::{SdkFd9, Session};
use fd9_sys::{
    CoordonneesMire, CouleursMire, DonneesQr, EtatSdk, ModeReconnaissance, PointMesure, ResultatQr,
    ZONES_MAX,
};
use pont_protocole::{DefinitionFeuille, ErreurPont, Info, LectureFeuille, Palier, RemiseAuRepos};

/// Écart de pas admis entre la reconnaissance et la définition, et écart
/// d'un point de mesure à la grille une fois le décalage retiré : 3 mm, en
/// 1/100 mm (la moitié d'une plage de la RGB-2520).
pub const ECART_ADMIS: u32 = 300;

/// Mode de reconnaissance du pont : COMBINE, avec les couleurs des plages,
/// comme MYIROtools et FD-S2w. La DLL trouve les plages sur l'image prise et
/// la mesure vise les points trouvés, jamais les seules coordonnées
/// théoriques (essai E4 bis : COORDINATE a mesuré à côté des plages).
pub const MODE_RECONNAISSANCE: ModeReconnaissance = ModeReconnaissance::Combine;

/// Définition convertie pour la DLL : coordonnées et couleurs.
pub(crate) struct MireConvertie {
    pub coordonnees: Box<CoordonneesMire>,
    pub couleurs: CouleursMire,
}

/// Plus grande image acceptée de `FD9_GetCaptureResult`, en octets : une
/// taille au-delà est une réponse inattendue, pas une allocation.
pub const TAILLE_IMAGE_MAX: u32 = 256 << 20;

/// Codes publics de la fiche `FD9SDK.md` qui ont un nom dans le protocole.
pub(crate) fn erreur_sdk(code: i32) -> ErreurPont {
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
        let resultat = self.prenumeriser_feuille(mire);
        // Une annulation ne vise que la feuille qui vient de finir.
        self.annulation.effacer();
        resultat
    }

    fn prenumeriser_feuille(
        &mut self,
        mire: &DefinitionFeuille,
    ) -> Result<LectureFeuille, ErreurPont> {
        let convertie = self.commencer_feuille(Palier::Prenumerisation, mire)?;
        let lecture = self.lire_feuille(mire, &convertie)?;
        self.arreter_feuille().map_err(EchecArret::erreur)?;
        self.franchir(Palier::Prenumerisation);
        Ok(lecture)
    }

    /// Début commun des paliers Prénumérisation et Feuille, sans appel à la
    /// DLL : palier, connexion, définition convertie (une mire sans couleurs
    /// est refusée : COMBINE en a besoin), événements anciens écartés.
    pub(crate) fn commencer_feuille(
        &mut self,
        palier: Palier,
        mire: &DefinitionFeuille,
    ) -> Result<MireConvertie, ErreurPont> {
        // Le calage d'une feuille précédente ne passe jamais pour celui-ci.
        self.calage = None;
        self.autoriser(palier, Some(Palier::Connexion))?;
        self.exiger_connexion()?;
        self.exiger_composant_qr()?;
        let coordonnees = CoordonneesMire::depuis_definition(mire).map_err(|e| {
            self.journal
                .push(format!("définition de feuille refusée : {e:?}"));
            ErreurPont::ParametreRefuse {}
        })?;
        let Some(couleurs) = CouleursMire::depuis_definition(mire) else {
            // Essai E3 : sans couleurs ni image modèle, COMBINE fait lire un
            // pointeur nul à la DLL (fiche FD9_StartPatchRecognition).
            self.journal.push(
                "mire refusée : sans les couleurs de ses plages (COMBINE), la feuille ne peut pas être recalée"
                    .into(),
            );
            return Err(ErreurPont::ParametreRefuse {});
        };
        self.vider_hors_requete();
        self.recus.clear();
        Ok(MireConvertie {
            coordonnees,
            couleurs,
        })
    }

    /// Le composant de lecture du QR à côté du pont, sans quoi la DLL
    /// arrêterait le pont à la lecture du QR, feuille engagée (ticket #87).
    fn exiger_composant_qr(&mut self) -> Result<(), ErreurPont> {
        let dossier = match self.dossier_du_pont.clone() {
            DossierDuPont::NonVerifie => return Ok(()),
            DossierDuPont::Connu(dossier) => dossier,
            DossierDuPont::Illisible => {
                self.journal.push(format!(
                    "composant de lecture du QR introuvable : dossier du pont illisible, {COMPOSANT_QR} non cherché ailleurs ; feuille non lancée"
                ));
                return Err(ErreurPont::ComposantQrIntrouvable {
                    chemin: COMPOSANT_QR.into(),
                });
            }
        };
        composant_qr(&dossier).map(drop).map_err(|chemin| {
            let chemin = chemin.display().to_string();
            self.journal.push(format!(
                "composant de lecture du QR introuvable : {chemin} (la DLL le charge depuis le dossier du pont) ; feuille non lancée"
            ));
            ErreurPont::ComposantQrIntrouvable { chemin }
        })
    }

    pub(crate) fn exiger_connexion(&self) -> Result<(), ErreurPont> {
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
    /// reconnaissance contrôlée et aux points de mesure calés. Toute sortie
    /// en erreur a déjà arrêté ou clos la feuille.
    pub(crate) fn lire_feuille(
        &mut self,
        mire: &DefinitionFeuille,
        convertie: &MireConvertie,
    ) -> Result<LectureFeuille, ErreurPont> {
        let coordonnees = &*convertie.coordonnees;
        self.attendre_feuille()?;
        let code = self.sdk.capturer();
        self.verifier("FD9_StartCapture", code)?;
        self.attendre_ou_conclure(EtatSdk::CaptureFinie, Attente::Etape)?;
        let (qr, taille_capture) = self.lire_capture()?;
        if let Info::Confirmee(texte) = &qr {
            if !texte.starts_with(&mire.nom) {
                let raison = ErreurPont::AutreMire {
                    texte: texte.clone(),
                };
                return Err(self.abandonner(raison));
            }
        }
        let code =
            self.sdk
                .reconnaitre(MODE_RECONNAISSANCE, coordonnees, Some(&convertie.couleurs));
        self.verifier("FD9_StartPatchRecognition", code)?;
        self.attendre_ou_conclure(EtatSdk::ReconnaissanceFinie, Attente::Etape)?;
        let mut lue = CoordonneesMire::vide();
        let mut fiabilite = 0u32;
        let code = self.sdk.resultat_reconnaissance(&mut fiabilite, &mut lue);
        self.verifier("FD9_GetPatchRecognitionResult2", code)?;
        let (conforme, ecart_pas) = comparer(coordonnees, &lue);
        self.journal.push(format!(
            "reconnaissance : {} zone(s), {} plage(s), fiabilité {fiabilite:#x}, écart de pas {ecart_pas}",
            lue.zones, lue.plages
        ));
        if fiabilite != 0 {
            // Comme FD-S2w (0x41f750) : des drapeaux d'alerte, pas de mesure.
            return Err(self.abandonner(ErreurPont::ReconnaissanceDouteuse { fiabilite }));
        }
        let non_reconnue = |ecart_max| ErreurPont::MireNonReconnue {
            zones: lue.zones,
            plages: lue.plages,
            ecart_max,
        };
        if !conforme || ecart_pas > ECART_ADMIS {
            return Err(self.abandonner(non_reconnue(ecart_pas)));
        }
        let mut points = vec![PointMesure::default(); lue.plages as usize];
        let code = self.sdk.points_mesure(&mut points);
        self.verifier("FD9_GetMeasurePoint", code)?;
        let calage = match caler(mire, &points) {
            Ok(calage) => calage,
            Err(defaut) => {
                self.journal
                    .push(format!("points de mesure refusés : {defaut:?}"));
                return Err(self.abandonner(ErreurPont::PositionsPlages { defaut }));
            }
        };
        let (dx, dy) = calage.decalage;
        self.journal.push(format!(
            "points de mesure : {} point(s), décalage de la feuille ({dx} ; {dy}), écart restant {}",
            points.len(),
            calage.ecart_max
        ));
        let decalage = dx.unsigned_abs().max(dy.unsigned_abs());
        if decalage > DECALAGE_ADMIS {
            return Err(self.abandonner(non_reconnue(decalage)));
        }
        if calage.ecart_max > ECART_ADMIS {
            return Err(self.abandonner(non_reconnue(calage.ecart_max)));
        }
        // État 7, avant l'arrêt ou la mesure : l'image de la feuille pour
        // l'application, avec les points visés.
        self.retenir_calage(&calage, &points, taille_capture);
        Ok(LectureFeuille {
            qr,
            zones: lue.zones,
            plages: lue.plages,
            fiabilite,
            ecart_max: ecart_pas.max(calage.ecart_max),
        })
    }

    /// Lit l'état courant (`FD9_GetStatus`, +0x10, sans question à
    /// l'instrument) : une feuille glissée avant la requête n'est pas perdue.
    /// État 3 : feuille engagée, la capture suit ; état 2 : engagement en
    /// cours ; état 1 : attente de la feuille ; autre : refus, rien n'est
    /// lancé.
    fn attendre_feuille(&mut self) -> Result<(), ErreurPont> {
        let mut statut = [0u32; 8];
        let code = self.sdk.statut(&mut statut);
        self.journal
            .push(format!("FD9_GetStatus : code {code}, {statut:?}"));
        if code != 0 {
            return Err(erreur_sdk(code));
        }
        let etat = EtatSdk::depuis(statut[4]);
        self.dernier_etat = Some(etat);
        match etat {
            EtatSdk::AlimentationFinie => Ok(()),
            EtatSdk::Alimentation => {
                self.attendre_ou_conclure(EtatSdk::AlimentationFinie, Attente::Etape)
            }
            EtatSdk::Attente => {
                self.attendre_ou_conclure(EtatSdk::AlimentationFinie, Attente::Feuille)
            }
            etat => {
                self.journal.push(format!(
                    "l'instrument n'attend pas de feuille (état {})",
                    etat.code()
                ));
                Err(ErreurPont::InstrumentIndisponible { etat: etat.code() })
            }
        }
    }

    /// QR et image, en deux appels : la taille, puis l'image dans un tampon
    /// de cette taille. L'image n'est gardée que le temps de l'appel ; sa
    /// taille est rendue avec le QR.
    fn lire_capture(&mut self) -> Result<(Info<String>, u32), ErreurPont> {
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
        let qr = if lu {
            Info::Confirmee(qr.texte())
        } else {
            Info::Inconnue
        };
        Ok((qr, taille))
    }

    /// Un code non nul arrête la feuille et rend l'erreur du SDK.
    pub(crate) fn verifier(&mut self, appel: &str, code: i32) -> Result<(), ErreurPont> {
        if code == 0 {
            return Ok(());
        }
        let interne = self.sdk.dernier_code();
        self.journal
            .push(format!("{appel} : code {code}, code interne {interne}"));
        Err(self.abandonner(erreur_sdk(code)))
    }

    /// Attend `voulu`, ou conclut la feuille selon ce qui l'a interrompue.
    pub(crate) fn attendre_ou_conclure(
        &mut self,
        voulu: EtatSdk,
        attente: Attente,
    ) -> Result<(), ErreurPont> {
        let annulee = ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {},
        };
        match self.attendre(voulu, attente) {
            Ok(()) => Ok(()),
            Err(Fin::Delai) if attente == Attente::Feuille => {
                Err(self.conclure_sans_feuille(ErreurPont::FeuilleAbsente {}))
            }
            Err(Fin::Annulee) if attente == Attente::Feuille => {
                Err(self.conclure_sans_feuille(annulee))
            }
            Err(Fin::Delai) => Err(self.abandonner(ErreurPont::Delai {})),
            Err(Fin::Annulee) => Err(self.abandonner(annulee)),
            Err(Fin::Incident(etat)) => Err(self.incident(etat)),
            Err(Fin::Inconnu(code)) => Err(self.etat_inconnu(code)),
        }
    }
}

impl<S: SdkFd9> Session<S> {
    /// L'attente de la feuille a fini sans elle (délai ou annulation). Comme
    /// MYIROtools, l'état est relu (`FD9_GetStatus`, +0x10) : à l'état 0 ou
    /// 1, rien n'est engagé, et ni `FD9_StopMeasurement` ni
    /// `FD9_ExitMeasurement` ne sont appelés. À l'essai E2, un arrêt depuis
    /// l'état 1 n'a jamais été suivi de l'événement 10. Aux autres états, la
    /// feuille est arrêtée comme avant. Une lecture refusée rend son erreur,
    /// sans arrêt.
    fn conclure_sans_feuille(&mut self, raison: ErreurPont) -> ErreurPont {
        let mut statut = [0u32; 8];
        let code = self.sdk.statut(&mut statut);
        self.journal
            .push(format!("FD9_GetStatus : code {code}, {statut:?}"));
        if code != 0 {
            return erreur_sdk(code);
        }
        match statut[4] {
            etat @ (0 | 1) => {
                self.dernier_etat = Some(EtatSdk::depuis(etat));
                self.journal.push(format!(
                    "aucune feuille engagée (état {etat}) : ni arrêt ni sortie"
                ));
                raison
            }
            _ => self.abandonner(raison),
        }
    }
}

/// Zones et plages égales à la définition (comme MYIROtools et FD-S2w), et
/// plus grand écart de pas sur les zones communes. L'origine n'est pas
/// comparée : COMBINE recale la feuille, et son décalage se lit sur les
/// points de mesure.
fn comparer(definition: &CoordonneesMire, lue: &CoordonneesMire) -> (bool, u32) {
    let communes = (definition.zones.min(lue.zones) as usize).min(ZONES_MAX);
    let conforme = lue.zones == definition.zones
        && lue.plages == definition.plages
        && definition.compte_zones[..communes] == lue.compte_zones[..communes];
    let ecart_max = definition.geometries[..communes]
        .iter()
        .zip(&lue.geometries[..communes])
        .flat_map(|(d, l)| [d.pas_x.abs_diff(l.pas_x), d.pas_y.abs_diff(l.pas_y)])
        .max()
        .unwrap_or(0);
    (conforme, ecart_max)
}
