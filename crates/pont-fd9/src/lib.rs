//! Pont FD-9 : logique de session au-dessus de `FD9SDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, fiches `docs/abi/FD9_*`). Le pont FD-9
//! déclare ses propres paliers : pour l'instant Version et Détection
//! seulement. Tout palier au-delà est refusé sans appel à la DLL.

pub mod dll;
pub mod serveur;

use fd9_sys::Appareil;
use pont_protocole::{ErreurPont, Palier};

/// Dernier palier que ce pont sait franchir, quel que soit le plafond demandé.
pub const PALIER_MAX: Palier = Palier::Detection;

/// Entrées passées à `FD9_GetDeviceList` : comme FD-S2w (la DLL admet 1 à 100).
pub const CAPACITE_DETECTION: usize = 10;

/// Les appels autorisés de `FD9SDK.dll` pour les paliers de ce pont. Le type ne
/// permet aucun autre appel : ni `JIG_*`, ni `FD9_Set*`, ni connexion tant que
/// le palier Connexion n'est pas déclaré.
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

pub struct Session<S: SdkFd9> {
    sdk: S,
    plafond: Palier,
    progression: Option<Palier>,
    journal: Vec<String>,
    fermee: bool,
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
        }
    }

    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    /// Appels à la DLL et codes rendus, dans l'ordre.
    pub fn journal(&self) -> &[String] {
        &self.journal
    }

    /// Plus haut palier franchi avec succès depuis le lancement.
    pub fn palier_atteint(&self) -> Option<Palier> {
        self.progression
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
        self.franchir(Palier::Detection);
        Ok(tableau[..trouves].to_vec())
    }

    /// Réponse à une demande d'un palier que ce pont ne déclare pas
    /// (connexion, étalonnage, mesures) : refusée sans appel à la DLL.
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

    /// Aucune session n'est jamais ouverte avec l'instrument : rien à
    /// désarmer ni à déconnecter. Ensuite, plus rien n'atteint la DLL.
    pub fn fermer(&mut self) {
        self.fermee = true;
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
