//! Pont MYIRO-1 : logique de session au-dessus de `FDXSDK.dll`.
//!
//! La session impose l'ordre des paliers et le plafond fixé au lancement,
//! avant tout appel à la DLL (ADR 0005, docs/abi/). Trois notions restent
//! séparées : le plafond (fixe), la progression des paliers (historique) et
//! l'état courant de l'instrument, seul à autoriser étalonnage et mesures.

pub mod dll;
pub mod serveur;

use fdx_sys::{
    lire_infos_instrument, ConditionCalcul, InfosInstrument, Port, Version, CONDITION_M0,
    CONDITION_M1, CONDITION_M2, LONGUEUR_BRUTES, LONGUEUR_LAB, LONGUEUR_SPECTRE, TAILLE_INFOS,
    TAILLE_TAMPON_INFOS,
};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, Echantillonnage, Empreinte, ErreurMesure,
    ErreurPont, Geometrie, Horodatage, Illuminant, Info, InstrumentMesurant, Observateur, Palier,
    Provenance, MODELE_MYIRO1,
};
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

/// Essais de désarmement au plus, quand l'instrument le refuse.
const ESSAIS_DESARMEMENT: usize = 3;

/// Code -9986 : opération interdite dans l'état actuel de l'instrument.
const CODE_ETAT_INCOMPATIBLE: i32 = -9986;

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
    /// `FDX_SetMeasureCondition({1, n})` : arme une lecture de bande ; la DLL
    /// compare le nombre de plages reconnues à `plages_attendues` (0 : aucun
    /// contrôle).
    fn armer_bande(&mut self, plages_attendues: u32) -> Result<i32, i32>;
    /// `FDX_StopMeasurement` : désarme et ramène l'instrument au repos.
    fn arreter_mesure(&mut self) -> Result<i32, i32>;
    /// `FDX_GetMeasureData` en deux temps, chaque résultat préparé à `longueur` valeurs.
    fn lire(&mut self, condition: &ConditionCalcul, longueur: usize) -> Result<Lecture, i32>;
    /// Prochain événement de l'instrument, ou `None` si le délai expire.
    fn attendre_evenement(&mut self, delai: Duration) -> Option<Evenement>;
    /// SHA-256 de la DLL chargée, pour la provenance des mesures.
    fn empreinte(&self) -> Option<String> {
        None
    }
}

/// Ce que rend une lecture de `FDX_GetMeasureData`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lecture {
    pub resultats: Vec<Vec<f32>>,
    /// Sens de passage rendu par la DLL (`FDX_eMeasureDirection`), brut.
    pub sens: u32,
}

/// Ce que l'instrument a mesuré sur une plage : les trois spectres (380 à 730 nm
/// par 10 nm), les données brutes (pilote libre, ADR 0006) et le Lab de la DLL.
#[derive(Clone, Debug, PartialEq)]
pub struct MesurePlage {
    pub m0: Vec<f32>,
    pub m1: Vec<f32>,
    pub m2: Vec<f32>,
    pub brutes: Vec<f32>,
    pub lab_dll: [Vec<f32>; 3],
}

/// Une lecture de bande : une mesure par plage, de gauche à droite selon le
/// manuel, et le sens de passage rendu par la DLL.
#[derive(Clone, Debug, PartialEq)]
pub struct MesureBande {
    pub plages: Vec<MesurePlage>,
    pub sens: u32,
    pub evenements: Vec<Evenement>,
    pub provenance: Provenance,
}

#[derive(Clone, Copy)]
enum Mode {
    Ponctuelle,
    /// Nombre de plages attendu, 0 si inconnu.
    Bande(u32),
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
    pub provenance: Provenance,
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

/// État courant de l'instrument vu par la session. Lui seul autorise
/// l'étalonnage et les mesures : un palier franchi dans le passé n'y suffit
/// jamais. L'identité et la date d'étalonnage n'existent qu'à l'intérieur de
/// l'état qui les garantit, et disparaissent avec lui.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EtatInstrument {
    /// Aucune connexion exploitable : jamais connecté, ou tentative en cours
    /// ou échouée.
    NonConnecte,
    /// `FDX_Connect` a réussi mais l'identité n'a pas pu être lue : une
    /// nouvelle connexion est nécessaire.
    Inexploitable,
    /// Connecté et identifié, sans étalonnage utilisable.
    Connecte { identite: InfosInstrument },
    /// Étalonné sur le blanc pendant cette connexion ; `date` au format RFC 3339.
    Etalonne {
        identite: InfosInstrument,
        date: String,
    },
    /// Liaison perdue (événement 6) : une nouvelle connexion est nécessaire.
    Perdu,
}

pub struct Session<S: SdkMyiro1> {
    sdk: S,
    plafond: Palier,
    /// Plus haut palier franchi : historique, n'autorise aucune mesure.
    progression: Option<Palier>,
    etat: EtatInstrument,
    ports: Vec<Port>,
    journal: Vec<String>,
    version: Option<Version>,
}

impl<S: SdkMyiro1> Session<S> {
    pub fn new(sdk: S, plafond: Palier) -> Self {
        Session {
            sdk,
            plafond,
            progression: None,
            etat: EtatInstrument::NonConnecte,
            ports: Vec::new(),
            journal: Vec::new(),
            version: None,
        }
    }

    pub fn sdk(&self) -> &S {
        &self.sdk
    }

    pub fn sdk_mut(&mut self) -> &mut S {
        &mut self.sdk
    }

    /// Étapes de la mesure, codes de retour et événements, dans l'ordre.
    pub fn journal(&self) -> &[String] {
        &self.journal
    }

    /// Plus haut palier franchi avec succès depuis le lancement. C'est un
    /// historique : il ne dit pas si l'instrument est encore connecté ni
    /// étalonné (voir [`Session::etat`]).
    pub fn palier_atteint(&self) -> Option<Palier> {
        self.progression
    }

    /// État courant de l'instrument.
    pub fn etat(&self) -> &EtatInstrument {
        &self.etat
    }

    fn franchir(&mut self, palier: Palier) {
        self.progression = self.progression.max(Some(palier));
    }

    pub fn version(&mut self) -> Result<Version, ErreurPont> {
        self.autoriser(Palier::Version, None)?;
        let version = self.sdk.version().map_err(traduire)?;
        self.version = Some(version);
        self.franchir(Palier::Version);
        Ok(version)
    }

    /// Une liste vide veut dire « aucun instrument branché », pas une erreur.
    pub fn detecter(&mut self) -> Result<Vec<Port>, ErreurPont> {
        self.autoriser(Palier::Detection, Some(Palier::Version))?;
        self.ports = self.sdk.ports().map_err(traduire)?;
        self.franchir(Palier::Detection);
        Ok(self.ports.clone())
    }

    /// Ouvre la session avec l'instrument n° `index` de la dernière détection
    /// et renvoie son identité.
    ///
    /// Attention : sur un instrument qui n'a jamais reçu de date de mise en
    /// service, la DLL y inscrit la date de l'ordinateur pendant cet appel.
    ///
    /// Toute tentative efface l'identité et l'étalonnage de la connexion
    /// précédente ; la connexion n'est exploitable qu'une fois l'identité lue.
    pub fn connecter(&mut self, index: usize) -> Result<Connexion, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Detection))?;
        let port = *self.ports.get(index).ok_or(ErreurPont::InstrumentInconnu)?;
        self.etat = EtatInstrument::NonConnecte;
        let code = self
            .sdk
            .connecter(&port, DELAI_CONNEXION)
            .map_err(traduire)?;
        self.etat = EtatInstrument::Inexploitable;
        let tampon = self.sdk.infos().map_err(traduire)?;
        let infos = lire_infos_instrument(&tampon);
        self.etat = EtatInstrument::Connecte {
            identite: infos.clone(),
        };
        self.franchir(Palier::Connexion);
        Ok(Connexion {
            infos,
            identite_brute: tampon[..TAILLE_INFOS].to_vec(),
            anomalie_date_initiale: code & BIT_ANOMALIE_DATE_INITIALE != 0,
        })
    }

    /// Étalonnage sur le blanc : l'instrument doit être posé sur son capuchon.
    /// L'étalonnage précédent cesse d'être utilisable dès le début de la
    /// tentative : après un échec, un délai ou une perte, il faut réétalonner.
    pub fn etalonner(&mut self) -> Result<Etalonnage, ErreurPont> {
        self.autoriser(Palier::Etalonnage, None)?;
        self.exiger(false)?;
        self.invalider_etalonnage();
        let resultat = self.attendre_etalonnage();
        match &resultat {
            Ok(_) => {
                if let EtatInstrument::Connecte { identite } = &self.etat {
                    self.etat = EtatInstrument::Etalonne {
                        identite: identite.clone(),
                        date: maintenant(),
                    };
                }
                self.franchir(Palier::Etalonnage);
            }
            Err(ErreurPont::InstrumentPerdu) => self.perdre_liaison(),
            Err(_) => {}
        }
        resultat
    }

    fn attendre_etalonnage(&mut self) -> Result<Etalonnage, ErreurPont> {
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
        Ok(Etalonnage { evenements })
    }

    /// Mesure ponctuelle : arme l'instrument, attend l'appui sur son bouton,
    /// lit M0, M1, M2 et les données brutes, puis désarme dans tous les cas.
    pub fn mesurer_ponctuelle(&mut self) -> Result<MesurePonctuelle, ErreurPont> {
        self.autoriser(Palier::MesurePonctuelle, None)?;
        let garantie = self.garantie()?;
        let (mut plages, _, evenements) = self.mesurer(Mode::Ponctuelle)?;
        if plages.len() != 1 {
            return Err(inattendue(format!(
                "{} résultats pour une mesure ponctuelle",
                plages.len()
            )));
        }
        let plage = plages.remove(0);
        let provenance = self.provenance(garantie, Geometrie::Ponctuelle {})?;
        Ok(MesurePonctuelle {
            provenance,
            m0: plage.m0,
            m1: plage.m1,
            m2: plage.m2,
            brutes: plage.brutes,
            lab_dll: plage.lab_dll,
            evenements,
        })
    }

    /// Lecture de bande : l'opérateur fait glisser l'instrument le long d'une
    /// rangée de plages ; la DLL les reconnaît et rend une mesure par plage.
    /// Avec `plages_attendues`, la DLL et le pont refusent une lecture qui n'en
    /// compte pas autant (par exemple le blanc du départ pris pour une plage).
    pub fn mesurer_bande(
        &mut self,
        plages_attendues: Option<u32>,
    ) -> Result<MesureBande, ErreurPont> {
        self.autoriser(Palier::Bande, None)?;
        let garantie = self.garantie()?;
        let (plages, sens, evenements) =
            self.mesurer(Mode::Bande(plages_attendues.unwrap_or(0)))?;
        if let Some(attendues) = plages_attendues {
            if plages.len() != attendues as usize {
                return Err(inattendue(format!(
                    "{} plages lues au lieu de {attendues}",
                    plages.len()
                )));
            }
        }
        Ok(MesureBande {
            plages,
            sens,
            evenements,
            provenance: self.provenance(garantie, Geometrie::Bande { sens })?,
        })
    }

    /// Identité et date d'étalonnage qui garantissent la mesure demandée,
    /// relevées avant armement ; refus sans appel à la DLL sinon.
    fn garantie(&self) -> Result<(InfosInstrument, String), ErreurPont> {
        self.exiger(true)?;
        match &self.etat {
            EtatInstrument::Etalonne { identite, date } => Ok((identite.clone(), date.clone())),
            _ => Err(ErreurPont::EtalonnageRequis),
        }
    }

    /// Refuse, sans appel à la DLL, ce que l'état courant ne permet pas :
    /// une connexion identifiée, et un étalonnage utilisable si `etalonne`.
    fn exiger(&self, etalonne: bool) -> Result<(), ErreurPont> {
        match &self.etat {
            EtatInstrument::Etalonne { .. } => Ok(()),
            EtatInstrument::Connecte { .. } if !etalonne => Ok(()),
            EtatInstrument::Connecte { .. } => Err(ErreurPont::EtalonnageRequis),
            EtatInstrument::Perdu => Err(ErreurPont::InstrumentPerdu),
            EtatInstrument::Inexploitable => Err(ErreurPont::SessionInexploitable),
            EtatInstrument::NonConnecte => Err(ErreurPont::EtatInvalide {
                attendu: Palier::Connexion,
            }),
        }
    }

    /// Liaison perdue (événement 6) : identité et étalonnage disparaissent,
    /// tout est refusé jusqu'à une nouvelle connexion. Seul chemin vers `Perdu`.
    fn perdre_liaison(&mut self) {
        self.journal
            .push("liaison perdue : reconnexion nécessaire".into());
        self.etat = EtatInstrument::Perdu;
    }

    /// L'étalonnage cesse d'être utilisable ; l'identité reste.
    fn invalider_etalonnage(&mut self) {
        if let EtatInstrument::Etalonne { identite, .. } = &self.etat {
            self.etat = EtatInstrument::Connecte {
                identite: identite.clone(),
            };
        }
    }

    /// Ce que le pont atteste sur la mesure qui vient de se terminer, à partir
    /// de la garantie relevée avant armement.
    fn provenance(
        &self,
        (infos, etalonnage): (InfosInstrument, String),
        geometrie: Geometrie,
    ) -> Result<Provenance, ErreurPont> {
        let Some(version) = &self.version else {
            return Err(inattendue("mesure sans version du SDK".into()));
        };
        let invalide = |erreur: ErreurMesure| inattendue(erreur.to_string());
        let empreinte_dll = match self.sdk.empreinte() {
            Some(texte) => Info::Confirmee(Empreinte::new(texte).map_err(invalide)?),
            None => Info::Inconnue,
        };
        // Relevée avant armement : l'étalonnage qui garantit cette mesure.
        let etalonnage = Info::Confirmee(Horodatage::new(etalonnage).map_err(invalide)?);
        Ok(Provenance {
            instrument: InstrumentMesurant {
                modele: MODELE_MYIRO1.into(),
                numero_serie: infos.numero,
                micrologiciel: infos.micrologiciel.clone(),
                code_produit: infos.code_produit.clone(),
            },
            version_sdk: [version.partie0, version.partie1, version.partie2],
            empreinte_dll,
            version_pont: env!("CARGO_PKG_VERSION").into(),
            architecture: std::env::consts::ARCH.into(),
            horodatage: Horodatage::new(maintenant()).map_err(invalide)?,
            etalonnage,
            geometrie,
            calcul: conditions_de_calcul(),
        })
    }

    /// Arme, attend la fin de la mesure, lit tout, puis désarme dans tous les cas.
    fn mesurer(
        &mut self,
        mode: Mode,
    ) -> Result<(Vec<MesurePlage>, u32, Vec<Evenement>), ErreurPont> {
        // Comme MYIRO tools : désarmer avant d'armer, l'instrument se réarmant
        // seul après une mesure.
        self.desarmer("avant armement");
        if self.etat == EtatInstrument::Perdu {
            return Err(ErreurPont::InstrumentPerdu);
        }
        let armement = match mode {
            Mode::Ponctuelle => self.sdk.armer_ponctuelle(),
            Mode::Bande(attendues) => self.sdk.armer_bande(attendues),
        };
        self.journal
            .push(format!("armement : code {}", code_de(armement)));
        if let Err(code) = armement {
            if code == CODE_NON_ETALONNE {
                self.invalider_etalonnage();
            }
            return Err(traduire(code));
        }
        let resultat = self.attendre_mesure().and_then(|evenements| {
            let (plages, sens) = self.lire_plages()?;
            Ok((plages, sens, evenements))
        });
        if resultat.as_ref().err() == Some(&ErreurPont::InstrumentPerdu) {
            self.perdre_liaison();
        }
        // Désarmer même après un échec.
        self.desarmer("après lecture");
        resultat
    }

    /// Lit spectres, données brutes et Lab de toutes les plages de la dernière
    /// mesure ; chaque lecture doit rendre le même nombre de plages, au moins une.
    fn lire_plages(&mut self) -> Result<(Vec<MesurePlage>, u32), ErreurPont> {
        let (m0, sens) =
            self.lire_tout(&ConditionCalcul::spectre(CONDITION_M0), LONGUEUR_SPECTRE)?;
        let nombre = m0.len();
        if nombre == 0 {
            return Err(inattendue("aucune plage dans la mesure".into()));
        }
        let m1 = self.lire_n(
            &ConditionCalcul::spectre(CONDITION_M1),
            LONGUEUR_SPECTRE,
            nombre,
        )?;
        let m2 = self.lire_n(
            &ConditionCalcul::spectre(CONDITION_M2),
            LONGUEUR_SPECTRE,
            nombre,
        )?;
        let brutes = self.lire_n(&ConditionCalcul::brutes(), LONGUEUR_BRUTES, nombre)?;
        let lab0 = self.lire_n(&ConditionCalcul::lab(CONDITION_M0), LONGUEUR_LAB, nombre)?;
        let lab1 = self.lire_n(&ConditionCalcul::lab(CONDITION_M1), LONGUEUR_LAB, nombre)?;
        let lab2 = self.lire_n(&ConditionCalcul::lab(CONDITION_M2), LONGUEUR_LAB, nombre)?;
        let plages = (0..nombre)
            .map(|i| MesurePlage {
                m0: m0[i].clone(),
                m1: m1[i].clone(),
                m2: m2[i].clone(),
                brutes: brutes[i].clone(),
                lab_dll: [lab0[i].clone(), lab1[i].clone(), lab2[i].clone()],
            })
            .collect();
        Ok((plages, sens))
    }

    /// Lit tous les résultats et exige qu'il y en ait `nombre`.
    fn lire_n(
        &mut self,
        condition: &ConditionCalcul,
        longueur: usize,
        nombre: usize,
    ) -> Result<Vec<Vec<f32>>, ErreurPont> {
        let (valeurs, _) = self.lire_tout(condition, longueur)?;
        if valeurs.len() == nombre {
            Ok(valeurs)
        } else {
            Err(inattendue(format!(
                "{} plages au lieu de {nombre}",
                valeurs.len()
            )))
        }
    }

    /// `FDX_StopMeasurement`, puis attente du retour au repos s'il a été accepté :
    /// sans cela, un nouvel armement est refusé (-9986). Juste après une mesure,
    /// le MYIRO-1 refuse aussi le désarmement (état « mesure réussie ») jusqu'à
    /// son réarmement automatique : on attend alors son prochain événement et
    /// on réessaie.
    fn desarmer(&mut self, moment: &str) {
        for _ in 0..ESSAIS_DESARMEMENT {
            let arret = self.sdk.arreter_mesure();
            self.journal
                .push(format!("désarmement {moment} : code {}", code_de(arret)));
            match arret {
                Ok(_) => {
                    self.attendre_repos();
                    return;
                }
                Err(CODE_ETAT_INCOMPATIBLE) => match self.sdk.attendre_evenement(DELAI_REPOS) {
                    Some(evenement) => {
                        self.journal.push(format!(
                            "événement {} (avant nouvel essai de désarmement)",
                            evenement.code
                        ));
                        if evenement.code == EVENEMENT_DECONNEXION {
                            self.perdre_liaison();
                            return;
                        }
                    }
                    // Aucun événement : l'instrument est déjà au repos.
                    None => return,
                },
                Err(_) => return,
            }
        }
    }

    /// Attend l'événement 0 (retour au repos), au plus `DELAI_REPOS`.
    fn attendre_repos(&mut self) {
        let echeance = Instant::now() + DELAI_REPOS;
        while let Some(evenement) = self
            .sdk
            .attendre_evenement(echeance.saturating_duration_since(Instant::now()))
        {
            self.journal
                .push(format!("événement {} (attente du repos)", evenement.code));
            if evenement.code == EVENEMENT_DECONNEXION {
                self.perdre_liaison();
                break;
            }
            if evenement.code == EVENEMENT_REPOS {
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
            // Les événements 2 se répètent à chaque donnée brute : un seul suffit.
            if evenements.last().map(|e: &Evenement| e.code) != Some(evenement.code) {
                self.journal.push(format!(
                    "événement {} (erreur {})",
                    evenement.code, evenement.erreur
                ));
            }
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

    /// Lit tous les résultats ; chacun doit compter exactement `longueur` valeurs.
    fn lire_tout(
        &mut self,
        condition: &ConditionCalcul,
        longueur: usize,
    ) -> Result<(Vec<Vec<f32>>, u32), ErreurPont> {
        let lecture = self.sdk.lire(condition, longueur).map_err(traduire)?;
        if let Some(mauvais) = lecture.resultats.iter().find(|v| v.len() != longueur) {
            return Err(inattendue(format!(
                "{} valeurs au lieu de {longueur}",
                mauvais.len()
            )));
        }
        Ok((lecture.resultats, lecture.sens))
    }

    fn autoriser(&self, demande: Palier, prealable: Option<Palier>) -> Result<(), ErreurPont> {
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

fn code_de(resultat: Result<i32, i32>) -> i32 {
    resultat.unwrap_or_else(|code| code)
}

fn traduire(code: i32) -> ErreurPont {
    match code {
        -9992 => ErreurPont::ParametreRefuse,
        CODE_ETAT_INCOMPATIBLE => ErreurPont::EtatIncompatible,
        CODE_NON_ETALONNE => ErreurPont::NonEtalonne,
        code => ErreurPont::Sdk { code },
    }
}

/// Ce que le pont demande à la DLL, pour la provenance, qualifié d'après la
/// fiche `docs/abi/FDX_GetMeasureData.md`. Rien n'est relu sur l'instrument.
fn conditions_de_calcul() -> Calcul {
    Calcul {
        libelle: "spectres : Illuminant 0/1/2 = M0/M1/M2, 380-730 nm par 10 nm ; Lab : D50, 2°"
            .into(),
        demande: Info::Confirmee(ConditionsCalcul {
            conditions_spectres: [
                Info::Confirmee(ConditionMesure::M0),
                Info::Confirmee(ConditionMesure::M1),
                Info::Confirmee(ConditionMesure::M2),
            ],
            longueurs_onde: Info::Confirmee(Echantillonnage {
                debut_nm: 380,
                pas_nm: 10,
            }),
            illuminant_lab: Info::Confirmee(Illuminant::D50),
            // « 0 = 2° » : supposé fort, pas confirmé.
            observateur_lab: Info::Supposee(Observateur::DeuxDegres),
        }),
        observe: Info::Inconnue,
    }
}

/// Heure de l'ordinateur, RFC 3339 à la seconde, avec fuseau.
fn maintenant() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn inattendue(detail: String) -> ErreurPont {
    ErreurPont::ReponseInattendue { detail }
}
