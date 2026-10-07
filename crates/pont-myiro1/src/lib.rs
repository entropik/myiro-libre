//! Pont MYIRO-1 : logique de session au-dessus de `FDXSDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, docs/abi/).

pub mod dll;

use fdx_sys::{
    lire_infos_instrument, ConditionCalcul, InfosInstrument, Port, Version, CONDITION_M0,
    CONDITION_M1, CONDITION_M2, LONGUEUR_BRUTES, LONGUEUR_LAB, LONGUEUR_SPECTRE, TAILLE_INFOS,
    TAILLE_TAMPON_INFOS,
};
use pont_protocole::{ErreurPont, Palier};
use std::time::{Duration, Instant};

/// Délai passé à `FDX_Connect`, en secondes : valeur des logiciels officiels
/// (fiche `docs/abi/FDX_Connect.md`).
pub const DELAI_CONNEXION: u32 = 10;

/// Bit ajouté par `FDX_Connect` à son code quand le contrôle de la date
/// initiale de l'instrument a échoué.
const BIT_ANOMALIE_DATE_INITIALE: i32 = 4;

/// Durée maximale d'un étalonnage, événements compris. EIZO attend environ
/// 10 s ; marge pour un instrument lent (fiche `docs/abi/FDX_Calibration.md`).
pub const DELAI_ETALONNAGE: Duration = Duration::from_secs(30);

/// Temps laissé à l'opérateur pour poser l'instrument et appuyer sur le bouton.
pub const DELAI_APPUI: Duration = Duration::from_secs(120);

/// Délai maximal du retour au repos après un désarmement.
pub const DELAI_REPOS: Duration = Duration::from_secs(5);

/// Code -9983 : mesure armée sans étalonnage valable.
const CODE_NON_ETALONNE: i32 = -9983;

/// Codes d'événement (fiche `docs/abi/FDX_RegisterDeviceEventHandler.md`).
const EVENEMENT_REPOS: i32 = 0;
const EVENEMENT_MESURE_TERMINEE: i32 = 3;
const EVENEMENT_MESURE_ECHOUEE: i32 = 4;
const EVENEMENT_DECONNEXION: i32 = 6;
const EVENEMENT_ETALONNAGE_REUSSI: i32 = 8;
const EVENEMENT_ETALONNAGE_ECHOUE: i32 = 9;

/// Un appel du rappel d'événements de la DLL, tel quel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evenement {
    pub code: i32,
    pub nb_donnees_brutes: u32,
    pub erreur: i32,
}

/// Les appels autorisés de `FDXSDK.dll`, un par export de la liste blanche.
/// Les erreurs sont les codes négatifs bruts de la DLL.
pub trait SdkMyiro1 {
    fn version(&mut self) -> Result<Version, i32>;
    /// Liste complète des instruments vus (appel en deux temps côté DLL).
    fn ports(&mut self) -> Result<Vec<Port>, i32>;
    /// Renvoie le code positif ou nul de la DLL, bit 4 compris.
    fn connecter(&mut self, port: &Port, delai: u32) -> Result<i32, i32>;
    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32>;
    /// `FDX_Calibration(0)` : étalonnage sur le blanc, seul type autorisé.
    /// Le résultat arrive ensuite par les événements.
    fn etalonner_blanc(&mut self) -> Result<i32, i32>;
    /// `FDX_SetMeasureCondition({0, 0})` : arme une mesure ponctuelle.
    fn armer_ponctuelle(&mut self) -> Result<i32, i32>;
    /// `FDX_StopMeasurement` : désarme et ramène l'instrument au repos.
    fn arreter_mesure(&mut self) -> Result<i32, i32>;
    /// `FDX_GetMeasureData` en deux temps, chaque résultat préparé à `longueur` valeurs.
    fn lire(&mut self, condition: &ConditionCalcul, longueur: usize) -> Result<Lecture, i32>;
    /// Prochain événement de l'instrument, ou `None` si le délai expire.
    fn attendre_evenement(&mut self, delai: Duration) -> Option<Evenement>;
}

/// Ce que rend une lecture de `FDX_GetMeasureData`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lecture {
    pub resultats: Vec<Vec<f32>>,
    /// Sens de passage rendu par la DLL (`FDX_eMeasureDirection`), brut.
    pub sens: u32,
}

/// Une mesure ponctuelle : les trois spectres (380 à 730 nm par 10 nm) et les
/// données brutes de l'instrument, gardées pour le pilote libre (ADR 0006).
#[derive(Clone, Debug, PartialEq)]
pub struct MesurePonctuelle {
    pub m0: Vec<f32>,
    pub m1: Vec<f32>,
    pub m2: Vec<f32>,
    pub brutes: Vec<f32>,
    /// L*a*b* D50/2° calculés par la DLL pour M0, M1, M2 : référence pour
    /// valider notre propre colorimétrie.
    pub lab_dll: [Vec<f32>; 3],
    pub evenements: Vec<Evenement>,
}

/// Résultat d'un étalonnage réussi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etalonnage {
    /// Événements reçus pendant l'étalonnage, pour le journal.
    pub evenements: Vec<Evenement>,
}

/// Résultat d'une connexion réussie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connexion {
    pub infos: InfosInstrument,
    /// La DLL n'a pas pu lire ou poser la date initiale de l'instrument
    /// (bit 4 du code de `FDX_Connect`) : à journaliser, pas bloquant.
    pub anomalie_date_initiale: bool,
    /// Les 40 octets écrits par `FDX_GetDeviceInfo`, à archiver pour pouvoir
    /// réinterpréter plus tard les champs encore inconnus.
    pub identite_brute: Vec<u8>,
}

pub struct Session<S: SdkMyiro1> {
    sdk: S,
    plafond: Palier,
    atteint: Option<Palier>,
    ports: Vec<Port>,
}

impl<S: SdkMyiro1> Session<S> {
    pub fn new(sdk: S, plafond: Palier) -> Self {
        Session {
            sdk,
            plafond,
            atteint: None,
            ports: Vec::new(),
        }
    }

    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    pub fn sdk_mut(&mut self) -> &mut S {
        &mut self.sdk
    }

    /// Dernier palier franchi avec succès.
    pub fn palier_atteint(&self) -> Option<Palier> {
        self.atteint
    }

    pub fn version(&mut self) -> Result<Version, ErreurPont> {
        self.autoriser(Palier::Version, None)?;
        let version = self.sdk.version().map_err(traduire)?;
        self.atteint = Some(Palier::Version);
        Ok(version)
    }

    /// Une liste vide veut dire « aucun instrument branché », pas une erreur.
    pub fn detecter(&mut self) -> Result<Vec<Port>, ErreurPont> {
        self.autoriser(Palier::Detection, Some(Palier::Version))?;
        self.ports = self.sdk.ports().map_err(traduire)?;
        self.atteint = Some(Palier::Detection);
        Ok(self.ports.clone())
    }

    /// Ouvre la session avec l'instrument n° `index` de la dernière détection
    /// et renvoie son identité.
    ///
    /// Attention : sur un instrument qui n'a jamais reçu de date de mise en
    /// service, la DLL y inscrit la date de l'ordinateur pendant cet appel.
    pub fn connecter(&mut self, index: usize) -> Result<Connexion, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Detection))?;
        let port = *self.ports.get(index).ok_or(ErreurPont::InstrumentInconnu)?;
        let code = self
            .sdk
            .connecter(&port, DELAI_CONNEXION)
            .map_err(traduire)?;
        self.atteint = Some(Palier::Connexion);
        let tampon = self.sdk.infos().map_err(traduire)?;
        Ok(Connexion {
            infos: lire_infos_instrument(&tampon),
            identite_brute: tampon[..TAILLE_INFOS].to_vec(),
            anomalie_date_initiale: code & BIT_ANOMALIE_DATE_INITIALE != 0,
        })
    }

    /// Étalonnage sur le blanc : l'instrument doit être posé sur son capuchon.
    /// Un échec laisse la session au palier Connexion.
    pub fn etalonner(&mut self) -> Result<Etalonnage, ErreurPont> {
        self.autoriser(Palier::Etalonnage, Some(Palier::Connexion))?;
        self.sdk.etalonner_blanc().map_err(traduire)?;
        let echeance = Instant::now() + DELAI_ETALONNAGE;
        let mut evenements = Vec::new();
        loop {
            let reste = echeance.saturating_duration_since(Instant::now());
            let evenement = self
                .sdk
                .attendre_evenement(reste)
                .ok_or(ErreurPont::Delai)?;
            evenements.push(evenement);
            match evenement.code {
                EVENEMENT_ETALONNAGE_REUSSI => break,
                EVENEMENT_ETALONNAGE_ECHOUE => {
                    return Err(ErreurPont::EtalonnageEchoue {
                        erreur: evenement.erreur,
                    })
                }
                EVENEMENT_DECONNEXION => return Err(ErreurPont::InstrumentPerdu),
                _ => {}
            }
        }
        self.atteint = Some(Palier::Etalonnage);
        Ok(Etalonnage { evenements })
    }

    /// Mesure ponctuelle : arme l'instrument, attend l'appui sur son bouton,
    /// lit M0, M1, M2 et les données brutes, puis désarme dans tous les cas.
    pub fn mesurer_ponctuelle(&mut self) -> Result<MesurePonctuelle, ErreurPont> {
        self.autoriser(Palier::MesurePonctuelle, Some(Palier::Etalonnage))?;
        if let Err(code) = self.sdk.armer_ponctuelle() {
            if code == CODE_NON_ETALONNE {
                self.atteint = Some(Palier::Connexion);
            }
            return Err(traduire(code));
        }
        let resultat = self.attendre_mesure().and_then(|evenements| {
            Ok(MesurePonctuelle {
                m0: self.lire_un(&ConditionCalcul::spectre(CONDITION_M0), LONGUEUR_SPECTRE)?,
                m1: self.lire_un(&ConditionCalcul::spectre(CONDITION_M1), LONGUEUR_SPECTRE)?,
                m2: self.lire_un(&ConditionCalcul::spectre(CONDITION_M2), LONGUEUR_SPECTRE)?,
                brutes: self.lire_un(&ConditionCalcul::brutes(), LONGUEUR_BRUTES)?,
                lab_dll: [
                    self.lire_un(&ConditionCalcul::lab(CONDITION_M0), LONGUEUR_LAB)?,
                    self.lire_un(&ConditionCalcul::lab(CONDITION_M1), LONGUEUR_LAB)?,
                    self.lire_un(&ConditionCalcul::lab(CONDITION_M2), LONGUEUR_LAB)?,
                ],
                evenements,
            })
        });
        // Désarmer même après un échec, puis attendre que l'instrument soit
        // revenu au repos : sinon un nouvel armement est refusé (-9986).
        if self.sdk.arreter_mesure().is_ok() {
            self.attendre_repos();
        }
        resultat
    }

    /// Attend l'événement 0 (retour au repos), au plus `DELAI_REPOS`.
    fn attendre_repos(&mut self) {
        let echeance = Instant::now() + DELAI_REPOS;
        while let Some(evenement) = self
            .sdk
            .attendre_evenement(echeance.saturating_duration_since(Instant::now()))
        {
            if evenement.code == EVENEMENT_REPOS || evenement.code == EVENEMENT_DECONNEXION {
                break;
            }
        }
    }

    fn attendre_mesure(&mut self) -> Result<Vec<Evenement>, ErreurPont> {
        let echeance = Instant::now() + DELAI_APPUI;
        let mut evenements = Vec::new();
        loop {
            let reste = echeance.saturating_duration_since(Instant::now());
            let evenement = self
                .sdk
                .attendre_evenement(reste)
                .ok_or(ErreurPont::Delai)?;
            evenements.push(evenement);
            match evenement.code {
                EVENEMENT_MESURE_TERMINEE => return Ok(evenements),
                EVENEMENT_MESURE_ECHOUEE => {
                    return Err(ErreurPont::MesureEchouee {
                        erreur: evenement.erreur,
                    })
                }
                EVENEMENT_DECONNEXION => return Err(ErreurPont::InstrumentPerdu),
                _ => {}
            }
        }
    }

    /// Lit un seul résultat de `longueur` valeurs ; toute autre forme est refusée.
    fn lire_un(
        &mut self,
        condition: &ConditionCalcul,
        longueur: usize,
    ) -> Result<Vec<f32>, ErreurPont> {
        let lecture = self.sdk.lire(condition, longueur).map_err(traduire)?;
        match lecture.resultats.as_slice() {
            [valeurs] if valeurs.len() == longueur => Ok(valeurs.clone()),
            autre => Err(ErreurPont::ReponseInattendue(format!(
                "{} résultat(s) au lieu d'un seul de {longueur} valeurs",
                autre.len()
            ))),
        }
    }

    fn autoriser(&self, demande: Palier, prealable: Option<Palier>) -> Result<(), ErreurPont> {
        if demande > self.plafond {
            return Err(ErreurPont::PalierNonAutorise {
                demande,
                plafond: self.plafond,
            });
        }
        if let Some(attendu) = prealable {
            if self.atteint < Some(attendu) {
                return Err(ErreurPont::EtatInvalide { attendu });
            }
        }
        Ok(())
    }
}

fn traduire(code: i32) -> ErreurPont {
    match code {
        -9992 => ErreurPont::ParametreRefuse,
        -9986 => ErreurPont::EtatIncompatible,
        CODE_NON_ETALONNE => ErreurPont::NonEtalonne,
        code => ErreurPont::Sdk { code },
    }
}
