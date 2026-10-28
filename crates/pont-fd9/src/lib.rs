//! Pont FD-9 : logique de session au-dessus de `FD9SDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, fiches `docs/abi/FD9_*`). Le pont FD-9
//! déclare ses propres paliers : Version, Détection et Connexion. Il n'a ni
//! étalonnage par l'utilisateur ni mesure ponctuelle. Tout palier au-delà est
//! refusé sans appel à la DLL.
//!
//! La session n'avance que sur un code de retour lu ou un événement reçu du
//! rappel ; elle n'invente aucun état.

pub mod dll;
pub mod evenements;
pub mod serveur;

use fd9_sys::{
    connexion_etablie, Appareil, EtatSdk, InfosSysteme, NomApplication, TamponInfosSysteme,
    CODE_RECERTIFICATION_DUE,
};
use pont_protocole::{AdresseReseau, ErreurPont, Identite, Palier, RemiseAuRepos};
use std::time::Duration;

/// Dernier palier que ce pont sait franchir, quel que soit le plafond demandé.
pub const PALIER_MAX: Palier = Palier::Connexion;

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
        }
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

    /// Connexion à l'instrument n° `instrument` de la dernière détection.
    pub fn connecter(&mut self, instrument: usize) -> Result<Identite, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Detection))?;
        let appareil = *self
            .detectes
            .get(instrument)
            .ok_or(ErreurPont::InstrumentInconnu {})?;
        self.ouvrir(&appareil)
    }

    /// Connexion directe à une adresse réseau, sans détection.
    pub fn connecter_adresse(&mut self, adresse: &AdresseReseau) -> Result<Identite, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Version))?;
        let appareil =
            Appareil::reseau(adresse.texte()).map_err(|_| ErreurPont::ParametreRefuse {})?;
        self.ouvrir(&appareil)
    }

    /// Inscrit le rappel, se connecte, lit l'identité et l'état. Un échec
    /// défait ce qui a été fait, dans l'ordre inverse.
    fn ouvrir(&mut self, appareil: &Appareil) -> Result<Identite, ErreurPont> {
        // Déjà connectée, la DLL rendrait 0 sans rien faire : succès trompeur.
        if self.connecte {
            return Err(ErreurPont::EtatIncompatible {});
        }
        let nom = NomApplication::nouveau(NOM_APPLICATION)
            .map_err(|e| inattendue(format!("nom d'application refusé : {e:?}")))?;
        self.vider_hors_requete();
        let code = self.sdk.inscrire_rappel(true);
        self.journal
            .push(format!("FD9_RegisterDeviceEventHandler : {code}"));
        if code != 0 {
            return Err(ErreurPont::Sdk { code });
        }
        self.inscrit = true;
        let code = self.sdk.connecter(appareil, &nom);
        if !connexion_etablie(code) {
            let interne = self.sdk.dernier_code();
            self.journal
                .push(format!("FD9_Connect : code {code}, code interne {interne}"));
            self.desinscrire();
            return Err(ErreurPont::Sdk { code });
        }
        self.connecte = true;
        let recertification_due = code == CODE_RECERTIFICATION_DUE;
        self.journal.push(if recertification_due {
            "FD9_Connect : 1901, connecté, recertification de l'instrument due".into()
        } else {
            format!("FD9_Connect : {code}")
        });
        match self.lire_identite(recertification_due) {
            Ok(identite) => {
                let mut statut = [0u32; 8];
                let code = self.sdk.statut(&mut statut);
                self.journal
                    .push(format!("FD9_GetStatus : code {code}, {statut:?}"));
                self.franchir(Palier::Connexion);
                Ok(identite)
            }
            Err(erreur) => {
                self.abandonner_connexion();
                Err(erreur)
            }
        }
    }

    fn lire_identite(&mut self, recertification_due: bool) -> Result<Identite, ErreurPont> {
        let mut tampon = TamponInfosSysteme::vide();
        let code = self.sdk.infos(&mut tampon);
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!(
                "FD9_GetSystemInfo : code {code}, code interne {interne}"
            ));
            return Err(ErreurPont::Sdk { code });
        }
        let infos = tampon.infos;
        if !infos.est_rempli() {
            return Err(inattendue("FD9_GetSystemInfo n'a rien écrit".into()));
        }
        // 8 chiffres supposés en BCD : lus en décimal, sans rien deviner.
        let texte = infos.numero_de_serie();
        let numero_serie = Some(&texte)
            .filter(|t| !t.is_empty() && t.bytes().all(|o| o.is_ascii_digit()))
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| inattendue(format!("n° de série « {texte} » non numérique")))?;
        self.journal.push(format!(
            "FD9_GetSystemInfo : 0, micrologiciel {}, produit {}",
            infos.micrologiciel(),
            infos.produit()
        ));
        Ok(Identite {
            numero_serie,
            micrologiciel: infos.micrologiciel(),
            code_produit: infos.code_produit(),
            adresse_mac: infos.mac(),
            // Le sens des dates du FD-9 n'est pas établi (fiche FD9_GetSystemInfo).
            date_initiale: None,
            anomalie_date_initiale: false,
            brute_hex: octets(&infos)
                .iter()
                .map(|octet| format!("{octet:02x}"))
                .collect(),
            recertification_due,
        })
    }

    /// Connecté, mais l'identité est inutilisable : déconnecter, puis
    /// désinscrire. Une déconnexion refusée laisse la session à fermer.
    fn abandonner_connexion(&mut self) {
        let code = self.sdk.deconnecter();
        self.journal.push(format!("FD9_Disconnect : {code}"));
        if code == 0 {
            self.connecte = false;
            self.desinscrire();
        }
    }

    fn desinscrire(&mut self) {
        let code = self.sdk.inscrire_rappel(false);
        self.journal
            .push(format!("FD9_RegisterDeviceEventHandler (nul) : {code}"));
        if code == 0 {
            self.inscrit = false;
        }
    }

    /// Événements arrivés hors de toute requête : journalisés, sans effet.
    fn vider_hors_requete(&mut self) {
        if !self.inscrit {
            return;
        }
        for e in self.sdk.evenements(Duration::ZERO) {
            self.journal.push(format!(
                "événement hors requête ignoré : état {} (travail {}, page {}, compte {})",
                e.etat.code(),
                e.travail,
                e.page,
                e.compte
            ));
        }
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

    /// Déconnecte puis désinscrit le rappel. Ensuite, plus rien n'atteint la
    /// DLL. Une déconnexion refusée n'est pas retentée.
    pub fn fermer(&mut self) -> Fermeture {
        if let Some(fermeture) = &self.fermeture {
            return fermeture.clone();
        }
        self.fermee = true;
        self.vider_hors_requete();
        let fermeture = if self.connecte {
            let code = self.sdk.deconnecter();
            self.journal.push(format!("FD9_Disconnect : {code}"));
            if code == 0 {
                self.connecte = false;
                self.desinscrire();
                Fermeture::Confirmee
            } else {
                Fermeture::Incertaine {
                    remise_au_repos: RemiseAuRepos::ArretRefuse { code },
                }
            }
        } else {
            if self.inscrit {
                self.desinscrire();
            }
            Fermeture::Confirmee
        };
        self.journal
            .push(format!("résultat de la fermeture : {fermeture:?}"));
        self.fermeture = Some(fermeture.clone());
        fermeture
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

fn inattendue(detail: String) -> ErreurPont {
    ErreurPont::ReponseInattendue { detail }
}

/// Les 100 octets de l'identité, dans l'ordre de la DLL.
fn octets(infos: &InfosSysteme) -> Vec<u8> {
    let mots =
        |valeurs: &[u32]| -> Vec<u8> { valeurs.iter().flat_map(|v| v.to_le_bytes()).collect() };
    [
        mots(&infos.version_sdk),
        mots(&infos.version_micrologiciel),
        infos.code_produit.to_vec(),
        infos.numero_serie.to_vec(),
        infos.adresse_mac.to_vec(),
        infos.remplissage.to_vec(),
        mots(&infos.date_etalonnage_usine),
        mots(&infos.date_premiere_mesure),
        infos.nom_produit.to_vec(),
    ]
    .concat()
}
