//! Instrument simulé partagé par les tests : répond comme FDXSDK d'après docs/abi/.

#![allow(dead_code)]

use fdx_sys::{ConditionCalcul, Port, Version, TAILLE_TAMPON_INFOS};
use pont_myiro1::{Evenement, Lecture, SdkMyiro1};
use std::collections::VecDeque;
use std::time::Duration;

/// SDK simulé : répond comme FDXSDK d'après docs/abi/, et note chaque appel.
#[derive(Default)]
pub struct SdkSimule {
    pub ports: Vec<Port>,
    pub code_connexion: i32,
    pub code_etalonnage: i32,
    pub code_armement: i32,
    /// Code négatif : `FDX_GetDeviceInfo` échoue avec ce code.
    pub code_infos: i32,
    /// N° de série rendu par `FDX_GetDeviceInfo`.
    pub numero_serie: u32,
    /// Nombre de résultats rendus par chaque lecture (1 en ponctuelle).
    pub resultats_par_lecture: usize,
    /// Événements que l'instrument simulé émettra, dans l'ordre.
    pub evenements: VecDeque<Evenement>,
    /// Événements émis à chaque armement réussi, une salve par armement.
    pub salves: VecDeque<Vec<Evenement>>,
    /// Comme le vrai MYIRO-1 : désarmement refusé (-9986) tant que l'instrument
    /// est dans l'état « mesure réussie », jusqu'à son réarmement automatique.
    pub arret_refuse_apres_mesure: bool,
    pub dernier_evenement: Option<i32>,
    /// Sens de passage rendu par les lectures.
    pub sens: u32,
    pub appels: Vec<String>,
}

pub fn evenement(code: i32) -> Evenement {
    Evenement {
        code,
        nb_donnees_brutes: 0,
        erreur: 0,
    }
}

impl SdkSimule {
    pub fn avec_un_myiro1() -> Self {
        SdkSimule {
            ports: vec![Port {
                code_liaison: 0,
                opaque: [0; 40],
            }],
            resultats_par_lecture: 1,
            numero_serie: 12345678,
            ..Default::default()
        }
    }
}

impl SdkMyiro1 for SdkSimule {
    fn empreinte(&self) -> Option<String> {
        Some("empreinte simulée".into())
    }
    fn version(&mut self) -> Result<Version, i32> {
        self.appels.push("version".into());
        Ok(Version {
            partie0: 1,
            partie1: 1,
            partie2: 0,
        })
    }
    fn ports(&mut self) -> Result<Vec<Port>, i32> {
        self.appels.push("ports".into());
        Ok(self.ports.clone())
    }
    /// SUPPOSÉ : accepte un second `FDX_Connect` sans `FDX_Disconnect`
    /// entre les deux (reconnexion après une perte de liaison ou une identité
    /// illisible). La fiche `docs/abi/FDX_Connect.md` ne décrit pas ce cas ;
    /// la vraie DLL peut le refuser. Ces tests vérifient la logique de la
    /// session, pas ce comportement de la DLL.
    fn connecter(&mut self, _port: &Port, delai: u32) -> Result<i32, i32> {
        self.appels.push(format!("connecter {delai}"));
        if self.code_connexion < 0 {
            Err(self.code_connexion)
        } else {
            Ok(self.code_connexion)
        }
    }
    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32> {
        self.appels.push("infos".into());
        if self.code_infos < 0 {
            return Err(self.code_infos);
        }
        let mut t = [0u8; TAILLE_TAMPON_INFOS];
        t[0..4].copy_from_slice(&self.numero_serie.to_le_bytes());
        Ok(t)
    }
    fn etalonner_blanc(&mut self) -> Result<i32, i32> {
        self.appels.push("etalonner blanc".into());
        if self.code_etalonnage < 0 {
            Err(self.code_etalonnage)
        } else {
            Ok(self.code_etalonnage)
        }
    }
    fn armer_ponctuelle(&mut self) -> Result<i32, i32> {
        self.appels.push("armer ponctuelle".into());
        if self.code_armement < 0 {
            return Err(self.code_armement);
        }
        if let Some(salve) = self.salves.pop_front() {
            self.evenements.extend(salve);
        }
        Ok(0)
    }
    fn armer_bande(&mut self, plages_attendues: u32) -> Result<i32, i32> {
        self.appels.push(format!("armer bande {plages_attendues}"));
        if self.code_armement < 0 {
            return Err(self.code_armement);
        }
        if let Some(salve) = self.salves.pop_front() {
            self.evenements.extend(salve);
        }
        Ok(0)
    }
    fn arreter_mesure(&mut self) -> Result<i32, i32> {
        self.appels.push("arreter".into());
        if self.arret_refuse_apres_mesure && self.dernier_evenement == Some(3) {
            return Err(-9986);
        }
        Ok(0)
    }
    /// Valeurs repérables : condition × 10 + type de données.
    fn lire(&mut self, condition: &ConditionCalcul, longueur: usize) -> Result<Lecture, i32> {
        self.appels.push(format!(
            "lire {} {}",
            condition.illuminant, condition.type_donnees
        ));
        let valeur = (condition.illuminant * 10 + condition.type_donnees) as f32;
        Ok(Lecture {
            resultats: vec![vec![valeur; longueur]; self.resultats_par_lecture],
            sens: self.sens,
        })
    }
    /// Sans événement en attente, simule l'expiration du délai.
    fn attendre_evenement(&mut self, _delai: Duration) -> Option<Evenement> {
        let evenement = self.evenements.pop_front()?;
        self.dernier_evenement = Some(evenement.code);
        Some(evenement)
    }
}
