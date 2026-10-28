//! Pont FD-9 : logique de session au-dessus de `FD9SDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, fiches `docs/abi/FD9_*`). Le pont FD-9
//! déclare ses propres paliers : Version, Détection, Connexion,
//! Prénumérisation et Feuille. Il n'a ni étalonnage par l'utilisateur ni
//! mesure ponctuelle. Tout palier au-delà est refusé sans appel à la DLL.
//!
//! La session n'avance que sur un code de retour lu ou un événement reçu du
//! rappel ; elle n'invente aucun état.

mod attente;
mod connexion;
pub mod dll;
pub mod evenements;
mod feuille;
mod fichier;
mod plages;
pub mod serveur;

pub use attente::{Annulation, Delais};
pub use feuille::{ECART_ADMIS, TAILLE_IMAGE_MAX};

pub use plages::{CONDITIONS, LONGUEUR_SPECTRE};

use fd9_sys::{
    Appareil, CleOption, CoordonneesMire, DonneesQr, EtatSdk, ModeReconnaissance, NomApplication,
    PointMesure, PositionPlage, TamponInfosSysteme, DONNEES_BRUTES,
};
use pont_protocole::{ErreurPont, Identite, Palier, RemiseAuRepos};
use std::collections::VecDeque;
use std::time::Duration;

/// Dernier palier que ce pont sait franchir, quel que soit le plafond demandé.
pub const PALIER_MAX: Palier = Palier::Feuille;

/// Entrées passées à `FD9_GetDeviceList` : comme FD-S2w (la DLL admet 1 à 100).
pub const CAPACITE_DETECTION: usize = 10;

/// Nom sous lequel le pont se présente à l'instrument (`FD9_Connect`).
pub const NOM_APPLICATION: &str = "myiro-libre";

/// Les appels autorisés de `FD9SDK.dll`, une méthode par export de la liste
/// blanche. Le type ne permet aucun autre appel : ni `JIG_*`, ni écriture
/// dans l'instrument.
pub trait SdkFd9 {
    /// `FD9_GetLastError` : code interne de la dernière erreur. Ne parle pas à
    /// l'instrument ; au palier Version, un retour prouve que la DLL répond.
    fn dernier_code(&mut self) -> i32;
    /// `FD9_GetDeviceList` : remplit `tableau` (sa longueur est la capacité)
    /// et `trouves` ; rend le code public (0 = succès, même partiel).
    fn lister(&mut self, tableau: &mut [Appareil], trouves: &mut u32) -> i32;
    /// `FileVersion` de la ressource du fichier DLL, lue sans appel à la DLL.
    fn version_fichier(&self) -> Option<[u32; 4]>;
    /// SHA-256 du fichier DLL chargé, en hexadécimal minuscule.
    fn empreinte(&self) -> Option<String>;
    /// `FD9_RegisterDeviceEventHandler` : le rappel du pont si `actif`,
    /// sinon le pointeur nul (désinscription). Refusé pendant une connexion.
    fn inscrire_rappel(&mut self, actif: bool) -> i32;
    /// `FD9_Connect` : 0 ou 1901 = connecté (`connexion_etablie`).
    fn connecter(&mut self, appareil: &Appareil, nom: &NomApplication) -> i32;
    /// `FD9_GetSystemInfo`, dans un tampon mis à zéro par l'appelant.
    fn infos(&mut self, tampon: &mut TamponInfosSysteme) -> i32;
    /// `FD9_GetStatus` : 8 nombres, l'état du SDK en n° 4 (+0x10).
    fn statut(&mut self, tampon: &mut [u32; 8]) -> i32;
    /// `FD9_Disconnect`.
    fn deconnecter(&mut self) -> i32;
    /// Événements reçus par le rappel depuis le dernier appel, dans l'ordre.
    /// S'il n'y en a aucun, attend au plus `attente` le suivant. Ne parle pas
    /// à la DLL.
    fn evenements(&mut self, attente: Duration) -> Vec<Evenement>;
    /// Rappels reçus sans session pour les lire (avant l'inscription ou
    /// après la désinscription).
    fn rappels_orphelins(&self) -> u64;
    /// `FD9_StartCapture` : admis à l'état 3 seulement.
    fn capturer(&mut self) -> i32;
    /// `FD9_GetCaptureResult` : sans `image`, rend la taille de l'image dans
    /// `taille` ; avec, la remplit (tampon d'au moins `taille` octets).
    fn resultat_capture(
        &mut self,
        qr_resultat: &mut u32,
        qr: &mut DonneesQr,
        taille: &mut u32,
        image: Option<&mut [u8]>,
    ) -> i32;
    /// `FD9_StartPatchRecognition`, sans couleurs, rangées et colonnes à 0 :
    /// le type n'admet ni couleurs ni mode matrice.
    fn reconnaitre(&mut self, mode: ModeReconnaissance, coordonnees: &CoordonneesMire) -> i32;
    /// `FD9_GetPatchRecognitionResult2`, sans disposition.
    fn resultat_reconnaissance(
        &mut self,
        fiabilite: &mut u32,
        coordonnees: &mut CoordonneesMire,
    ) -> i32;
    /// `FD9_StopMeasurement` : admis aux états 1 à 8.
    fn arreter(&mut self, feuille_suivante: bool) -> i32;
    /// `FD9_ExitMeasurement(0, NULL)` : admis aux états 9, 10 et d'erreur.
    /// Le type n'envoie jamais d'image à l'instrument.
    fn sortir(&mut self) -> i32;
    /// `FD9_GetPremeasurementError` (`__cdecl`), sans texte : rend la cause.
    fn erreur_premesure(&mut self, erreur_systeme: &mut i32) -> i32;
    /// `FD9_GetMeasurePoint`, un point de 20 octets par plage reconnue.
    fn points_mesure(&mut self, points: &mut [PointMesure]) -> i32;
    /// `FD9_StartMeasurement2(1, 1)`, comme MYIROtools : le type n'admet pas
    /// d'autre mode.
    fn mesurer(&mut self) -> i32;
    /// `FD9_SetOption` : un réglage de calcul rangé dans la DLL, sans
    /// échange avec l'instrument.
    fn regler(&mut self, cle: CleOption, valeur: u32) -> i32;
    /// `FD9_GetData` : les 152 données brutes de la plage `demandee`, et la
    /// position que la DLL en déduit.
    fn donnees(
        &mut self,
        rendue: &mut PositionPlage,
        donnees: &mut [f32; DONNEES_BRUTES],
        demandee: &PositionPlage,
    ) -> i32;
    /// `FD9_CalculateData` depuis 152 données brutes, selon les réglages ;
    /// la longueur de sortie est celle du tableau.
    fn calculer(&mut self, sortie: &mut [f32], brutes: &[f32; DONNEES_BRUTES]) -> i32;
}

/// Un appel du rappel d'événements : les quatre nombres de la DLL, l'état
/// nommé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evenement {
    pub etat: EtatSdk,
    pub travail: u32,
    pub page: u32,
    /// Plages déjà mesurées (état 11).
    pub compte: u32,
}

/// Ce que le palier Version établit, sans parler à l'instrument (fiche
/// `docs/abi/FD9_GetLastError.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionFd9 {
    pub version_fichier: Option<[u32; 4]>,
    pub empreinte: Option<String>,
    /// Ce que `FD9_GetLastError` a rendu (supposé 0 au chargement).
    pub code_interne: i32,
}

/// Résultat d'une fermeture. Une fois faite, elle n'est pas refaite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fermeture {
    /// Déconnecté, rappel désinscrit (ou rien n'était ouvert).
    Confirmee,
    /// `FD9_Disconnect` a été refusé (code brut) : la session de la DLL reste
    /// peut-être ouverte. Le pont s'arrête sans nouvel essai.
    Incertaine { remise_au_repos: RemiseAuRepos },
}

pub struct Session<S: SdkFd9> {
    sdk: S,
    plafond: Palier,
    progression: Option<Palier>,
    journal: Vec<String>,
    fermee: bool,
    fermeture: Option<Fermeture>,
    /// Instruments de la dernière détection.
    detectes: Vec<Appareil>,
    /// Rappel inscrit dans la DLL, session de la DLL ouverte.
    inscrit: bool,
    connecte: bool,
    /// Identité et version du SDK lues à la connexion, pour la provenance.
    identite: Option<Identite>,
    version_sdk: [u32; 3],
    /// Liaison perdue (état 103) : plus rien ne part vers l'instrument.
    perdu: bool,
    delais: Delais,
    annulation: Annulation,
    reconnaissance: ModeReconnaissance,
    /// Événements reçus et pas encore traités par la requête en cours.
    recus: VecDeque<Evenement>,
}

const APPAREIL_VIDE: Appareil = Appareil {
    code_liaison: 0,
    adresse: [0; 32],
    identifiant: [0; 8],
};

impl<S: SdkFd9> Session<S> {
    /// Le plafond est ramené au dernier palier déclaré par ce pont.
    pub fn new(sdk: S, plafond: Palier) -> Self {
        Session {
            sdk,
            plafond: plafond.min(PALIER_MAX),
            progression: None,
            journal: Vec::new(),
            fermee: false,
            fermeture: None,
            detectes: Vec::new(),
            inscrit: false,
            connecte: false,
            identite: None,
            version_sdk: [0; 3],
            perdu: false,
            delais: Delais::default(),
            annulation: Annulation::default(),
            reconnaissance: ModeReconnaissance::Combine,
            recus: VecDeque::new(),
        }
    }

    /// Délais d'attente de la feuille et de chaque étape.
    pub fn avec_delais(mut self, delais: Delais) -> Self {
        self.delais = delais;
        self
    }

    /// Mode de reconnaissance des plages : COMBINE par défaut, COORDINATE en
    /// repli, sur réglage explicite.
    pub fn avec_reconnaissance(mut self, mode: ModeReconnaissance) -> Self {
        self.reconnaissance = mode;
        self
    }

    /// Poignée d'annulation de la requête en cours, à donner au fil qui lit
    /// les requêtes.
    pub fn annulation(&self) -> Annulation {
        self.annulation.clone()
    }

    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    /// Pour les tests : agir sur l'instrument simulé pendant la session.
    pub fn sdk_mut(&mut self) -> &mut S {
        &mut self.sdk
    }

    /// Appels à la DLL, codes rendus et événements reçus, dans l'ordre.
    pub fn journal(&self) -> &[String] {
        &self.journal
    }

    /// Plus haut palier franchi avec succès depuis le lancement.
    pub fn palier_atteint(&self) -> Option<Palier> {
        self.progression
    }

    /// Rappels de la DLL arrivés sans session pour les lire.
    pub fn rappels_orphelins(&self) -> u64 {
        self.sdk.rappels_orphelins()
    }

    pub fn version(&mut self) -> Result<VersionFd9, ErreurPont> {
        self.autoriser(Palier::Version, None)?;
        let code_interne = self.sdk.dernier_code();
        self.journal
            .push(format!("FD9_GetLastError : {code_interne}"));
        let version = VersionFd9 {
            version_fichier: self.sdk.version_fichier(),
            empreinte: self.sdk.empreinte(),
            code_interne,
        };
        self.franchir(Palier::Version);
        Ok(version)
    }

    /// Instruments vus en USB puis sur le réseau. Une liste vide veut dire
    /// « aucun FD-9 visible », pas une erreur.
    pub fn detecter(&mut self) -> Result<Vec<Appareil>, ErreurPont> {
        self.autoriser(Palier::Detection, Some(Palier::Version))?;
        let mut tableau = [APPAREIL_VIDE; CAPACITE_DETECTION];
        let mut trouves = 0u32;
        let code = self.sdk.lister(&mut tableau, &mut trouves);
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!(
                "FD9_GetDeviceList : code {code}, code interne {interne}"
            ));
            return Err(ErreurPont::Sdk { code });
        }
        self.journal
            .push(format!("FD9_GetDeviceList : 0, {trouves} trouvé(s)"));
        let trouves = trouves as usize;
        if trouves > CAPACITE_DETECTION {
            return Err(ErreurPont::ReponseInattendue {
                detail: format!("{trouves} instruments annoncés pour {CAPACITE_DETECTION} places"),
            });
        }
        for appareil in &tableau[..trouves] {
            if let fd9_sys::Liaison::Inconnue(code) = appareil.liaison() {
                self.journal
                    .push(format!("FD9_GetDeviceList : liaison inconnue {code}"));
            }
        }
        self.detectes = tableau[..trouves].to_vec();
        self.franchir(Palier::Detection);
        Ok(self.detectes.clone())
    }

    /// Réponse à une demande d'un palier que ce pont ne déclare pas
    /// (étalonnage, mesures) : refusée sans appel à la DLL.
    pub fn hors_paliers(&self, palier: Palier) -> ErreurPont {
        if self.fermee {
            ErreurPont::SessionFermee {}
        } else {
            ErreurPont::PalierNonAutorise {
                demande: palier,
                plafond: self.plafond,
            }
        }
    }

    fn franchir(&mut self, palier: Palier) {
        self.progression = self.progression.max(Some(palier));
    }

    fn autoriser(&self, demande: Palier, prealable: Option<Palier>) -> Result<(), ErreurPont> {
        if self.fermee {
            return Err(ErreurPont::SessionFermee {});
        }
        if demande > self.plafond {
            return Err(ErreurPont::PalierNonAutorise {
                demande,
                plafond: self.plafond,
            });
        }
        if let Some(attendu) = prealable {
            if self.progression < Some(attendu) {
                return Err(ErreurPont::EtatInvalide { attendu });
            }
        }
        Ok(())
    }
}
