//! Palier Feuille : mesure des plages de la feuille reconnue, lecture des 152
//! données brutes de chaque plage, puis spectres et Lab M0, M1, M2 calculés
//! par la DLL (fiches `docs/abi/FD9_GetMeasurePoint.md`,
//! `FD9_StartMeasurement2.md`, `FD9_GetData.md`, `FD9_SetOption.md`,
//! `FD9_CalculateData.md`).
//!
//! La mesure finie (état 9), `FD9_StopMeasurement` est refusé : la feuille
//! est close par `FD9_ExitMeasurement` seul, après la lecture de toutes les
//! plages. Les calculs, sans échange avec l'instrument, viennent ensuite.

use crate::attente::Attente;
use crate::connexion::inattendue;
use crate::feuille::erreur_sdk;
use crate::{SdkFd9, Session};
use fd9_sys::{
    CleOption, EtatSdk, PositionPlage, DONNEES_BRUTES, DONNEES_BRUTES_TYPE, LAB_TYPE, SPECTRE_TYPE,
};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DefautPositions, DefinitionFeuille, DonneesBrutes,
    Echantillonnage, Empreinte, ErreurPont, Geometrie, Horodatage, Illuminant, Info,
    InstrumentMesurant, Lab, LectureFeuille, Mesure, Observateur, Palier, Plage, Provenance,
    Spectre, MODELE_FD9,
};
use std::collections::HashMap;

/// Valeur de la clé 0 de `FD9_SetOption` pour chaque condition de mesure,
/// relevée dans FD-S2w (supposé, à comparer à l'essai E4).
pub const CONDITIONS: [(ConditionMesure, u32); 3] = [
    (ConditionMesure::M0, 0),
    (ConditionMesure::M1, 2),
    (ConditionMesure::M2, 15),
];

/// Valeurs d'un spectre (fiche `FD9_CalculateData`, confirmé).
pub const LONGUEUR_SPECTRE: usize = 36;

/// Clés 1 et 2 de `FD9_SetOption` : D50 et 2°.
const ILLUMINANT_D50: u32 = 2;
const OBSERVATEUR_2_DEGRES: u32 = 0;

const LIBELLE_CALCUL: &str = "étalonnage automatique de l'instrument ; spectres : condition \
     0/2/15 = M0/M1/M2 (supposé), 380-730 nm par 10 nm (supposé) ; Lab : D50, 2° (supposé)";

/// Position de départ de `FD9_GetData` : aucune plage n'a cette position
/// (la DLL rend des rangées et colonnes d'au moins 0).
const POSITION_IMPOSSIBLE: PositionPlage = PositionPlage {
    zone_ou_index: u32::MAX,
    rangee: -2,
    colonne: -2,
};

/// Données brutes d'une plage, telles que `FD9_GetData` les rend.
type Brutes = [f32; DONNEES_BRUTES];

impl<S: SdkFd9> Session<S> {
    /// Mêmes étapes que la prénumérisation, puis mesure de toutes les plages.
    /// La plage n° i de la mesure est la plage n° i de `mire` : rangée par la
    /// position que rend la DLL avec ses données.
    pub fn mesurer_feuille(&mut self, mire: &DefinitionFeuille) -> Result<Mesure, ErreurPont> {
        let resultat = self.mesurer_toute_la_feuille(mire);
        // Une annulation ne vise que la feuille qui vient de finir.
        self.annulation.effacer();
        resultat
    }

    fn mesurer_toute_la_feuille(&mut self, mire: &DefinitionFeuille) -> Result<Mesure, ErreurPont> {
        let convertie = self.commencer_feuille(Palier::Feuille, mire)?;
        let lecture = self.lire_feuille(mire, &convertie)?;
        let code = self.sdk.mesurer();
        self.verifier("FD9_StartMeasurement2", code)?;
        self.attendre_ou_conclure(EtatSdk::MesureFinie, Attente::Mesure)?;
        let lues = self.lire_plages(mire);
        // Toutes les plages lues, ou la lecture refusée : la feuille est
        // close dans les deux cas.
        let sortie = self.sortir();
        let brutes = lues?;
        sortie?;
        let plages = self.calculer_plages(&brutes)?;
        self.controler_vraisemblance(mire, &plages)?;
        let provenance = self.provenance(lecture)?;
        let mesure = Mesure::new(plages, provenance).map_err(|e| inattendue(e.to_string()))?;
        self.franchir(Palier::Feuille);
        Ok(mesure)
    }

    /// Les 152 données brutes de chaque plage, demandée par son numéro, puis
    /// rangées dans l'ordre de la mire d'après la position que la DLL rend
    /// avec elles. L'instrument rend ses plages dans son ordre de mesure (en
    /// serpentin à l'essai E4) : aucun ordre n'est supposé. Chaque position
    /// de la mire doit être rendue une fois et une seule, sinon tout est
    /// refusé (`PositionsPlages`).
    fn lire_plages(&mut self, mire: &DefinitionFeuille) -> Result<Vec<Brutes>, ErreurPont> {
        let attendues = positions(mire);
        let mut lues = Vec::with_capacity(attendues.len());
        for i in 0..attendues.len() {
            self.regler(CleOption::TypeDonnees, DONNEES_BRUTES_TYPE)?;
            let mut brutes = [0f32; DONNEES_BRUTES];
            // Valeur impossible : une position que la DLL n'écrit pas ne
            // peut pas passer pour une position de la mire.
            let mut rendue = POSITION_IMPOSSIBLE;
            // Bornée par PLAGES_MAX : le numéro tient dans un u32.
            let demandee = PositionPlage::numero(i as u32);
            let code = self.sdk.donnees(&mut rendue, &mut brutes, &demandee);
            if code != 0 {
                // Après un GetData, GetLastError ne dit rien de lui (fiche).
                self.journal
                    .push(format!("FD9_GetData plage lue n° {} : code {code}", i + 1));
                return Err(erreur_sdk(code));
            }
            lues.push((rendue, brutes));
        }
        let desordre = lues
            .iter()
            .zip(&attendues)
            .filter(|((rendue, _), attendue)| rendue != *attendue)
            .count();
        let rangees = ranger(&attendues, lues).inspect_err(|e| {
            self.journal
                .push(format!("plages lues impossibles à ranger : {e:?}"));
        })?;
        self.journal.push(format!(
            "{} plage(s) lue(s), rangées par leur position dans l'ordre de la mire              ({desordre} hors de l'ordre de la mire à la lecture)",
            rangees.len()
        ));
        Ok(rangees)
    }

    /// Spectres et Lab M0, M1, M2 de chaque plage, réglages reposés avant
    /// chaque calcul.
    fn calculer_plages(&mut self, toutes: &[Brutes]) -> Result<Vec<Plage>, ErreurPont> {
        let mut plages = Vec::with_capacity(toutes.len());
        for (i, brutes) in toutes.iter().enumerate() {
            let mut spectres = Vec::with_capacity(3);
            let mut labs = Vec::with_capacity(3);
            for (condition, valeur) in CONDITIONS {
                self.regler(CleOption::Condition, valeur)?;
                self.regler(CleOption::TypeDonnees, SPECTRE_TYPE)?;
                let mut spectre = [0f32; LONGUEUR_SPECTRE];
                let code = self.sdk.calculer(&mut spectre, brutes);
                if code != 0 {
                    return Err(inattendue(format!(
                        "plage n° {}, {condition:?} : spectre de {LONGUEUR_SPECTRE} valeurs \
                         refusé par la DLL (code {code})",
                        i + 1
                    )));
                }
                self.regler(CleOption::TypeDonnees, LAB_TYPE)?;
                self.regler(CleOption::Illuminant, ILLUMINANT_D50)?;
                self.regler(CleOption::Observateur, OBSERVATEUR_2_DEGRES)?;
                let mut lab = [0f32; 3];
                let code = self.sdk.calculer(&mut lab, brutes);
                if code != 0 {
                    return Err(inattendue(format!(
                        "plage n° {}, {condition:?} : Lab refusé par la DLL (code {code})",
                        i + 1
                    )));
                }
                let invalide = |e: pont_protocole::ErreurMesure| {
                    inattendue(format!("plage n° {}, {condition:?} : {e}", i + 1))
                };
                spectres.push(Spectre::new(spectre.to_vec()).map_err(invalide)?);
                labs.push(Lab::new(lab).map_err(invalide)?);
            }
            let invalide =
                |e: pont_protocole::ErreurMesure| inattendue(format!("plage n° {} : {e}", i + 1));
            let brutes = DonneesBrutes::new(brutes.to_vec()).map_err(invalide)?;
            let (Ok(spectres), Ok(labs)) = (spectres.try_into(), labs.try_into()) else {
                return Err(inattendue(format!("plage n° {} incomplète", i + 1)));
            };
            plages.push(Plage::new(spectres, brutes, labs).map_err(invalide)?);
        }
        Ok(plages)
    }

    /// Les L* M1 mesurés doivent suivre la clarté des couleurs imprimées
    /// (`calage::CORRELATION_MIN`) : une mesure à côté des plages ne les
    /// suit pas (essai E4 bis). Rien n'est rendu sinon.
    fn controler_vraisemblance(
        &mut self,
        mire: &DefinitionFeuille,
        plages: &[Plage],
    ) -> Result<(), ErreurPont> {
        let Some(couleurs) = &mire.couleurs else {
            return Ok(());
        };
        let l_m1: Vec<f32> = plages.iter().map(|p| p.lab()[1].valeurs()[0]).collect();
        let Some(r) = crate::calage::correlation(&l_m1, couleurs) else {
            self.journal.push(
                "vraisemblance non contrôlée : clartés imprimées ou mesurées toutes égales".into(),
            );
            return Ok(());
        };
        let correlation_milliemes = (r * 1000.0).round() as i32;
        self.journal.push(format!(
            "vraisemblance : corrélation des L* M1 à la clarté imprimée {correlation_milliemes} pour mille (minimum {})",
            (crate::calage::CORRELATION_MIN * 1000.0) as i32
        ));
        if r < crate::calage::CORRELATION_MIN {
            return Err(ErreurPont::MesureInvraisemblable {
                correlation_milliemes,
            });
        }
        Ok(())
    }

    fn regler(&mut self, cle: CleOption, valeur: u32) -> Result<(), ErreurPont> {
        let code = self.sdk.regler(cle, valeur);
        if code == 0 {
            return Ok(());
        }
        self.journal.push(format!(
            "FD9_SetOption({}, {valeur}) : code {code}",
            cle as u32
        ));
        Err(erreur_sdk(code))
    }

    /// Ce que le pont atteste : instrument et SDK lus à la connexion, pas
    /// d'étalonnage par l'utilisateur, conditions, illuminant et observateur
    /// supposés tant que l'essai E4 ne les a pas comparés à une mesure de
    /// référence.
    fn provenance(&self, lecture: LectureFeuille) -> Result<Provenance, ErreurPont> {
        let identite = self
            .identite
            .clone()
            .ok_or_else(|| inattendue("identité de l'instrument absente".into()))?;
        let maintenant = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
        let horodatage = Horodatage::new(maintenant).map_err(|e| inattendue(e.to_string()))?;
        Ok(Provenance {
            instrument: InstrumentMesurant {
                modele: MODELE_FD9.into(),
                numero_serie: identite.numero_serie,
                micrologiciel: identite.micrologiciel,
                code_produit: identite.code_produit,
            },
            version_sdk: self.version_sdk,
            empreinte_dll: self
                .sdk
                .empreinte()
                .and_then(|texte| Empreinte::new(texte).ok())
                .map_or(Info::Inconnue, Info::Confirmee),
            version_pont: env!("CARGO_PKG_VERSION").into(),
            architecture: std::env::consts::ARCH.into(),
            horodatage,
            etalonnage: Info::Inconnue,
            geometrie: Geometrie::Feuille {},
            calcul: Calcul {
                libelle: LIBELLE_CALCUL.into(),
                demande: Info::Confirmee(ConditionsCalcul {
                    conditions_spectres: CONDITIONS.map(|(c, _)| Info::Supposee(c)),
                    longueurs_onde: Info::Supposee(Echantillonnage {
                        debut_nm: 380,
                        pas_nm: 10,
                    }),
                    // Valeurs 2 et 0 d'une table de FD-S2w, que la DLL ne
                    // permet pas de relire (fiche FD9_SetOption) : supposées.
                    illuminant_lab: Info::Supposee(Illuminant::D50),
                    observateur_lab: Info::Supposee(Observateur::DeuxDegres),
                }),
                observe: Info::Inconnue,
            },
            feuille: Some(lecture),
        })
    }
}

/// Range les plages lues (position rendue, données) dans l'ordre de
/// `attendues`. Une position rendue deux fois, absente de la mire, ou jamais
/// rendue refuse tout ; `lue` compte depuis 1, dans l'ordre de lecture.
fn ranger(
    attendues: &[PositionPlage],
    lues: Vec<(PositionPlage, Brutes)>,
) -> Result<Vec<Brutes>, ErreurPont> {
    let rang: HashMap<PositionPlage, usize> =
        attendues.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    let mut rangees: Vec<Option<Brutes>> = vec![None; attendues.len()];
    for (i, (rendue, brutes)) in lues.into_iter().enumerate() {
        // Bornée par PLAGES_MAX : le rang tient dans un u32.
        let lue = i as u32 + 1;
        let (zone, rangee, colonne) = (rendue.zone_ou_index, rendue.rangee, rendue.colonne);
        let Some(&n) = rang.get(&rendue) else {
            return Err(positions_plages(DefautPositions::HorsMire {
                lue,
                zone,
                rangee,
                colonne,
            }));
        };
        if rangees[n].replace(brutes).is_some() {
            return Err(positions_plages(DefautPositions::Doublon {
                lue,
                zone,
                rangee,
                colonne,
            }));
        }
    }
    if let Some(n) = rangees.iter().position(Option::is_none) {
        let p = attendues[n];
        return Err(positions_plages(DefautPositions::Manquante {
            zone: p.zone_ou_index,
            rangee: p.rangee,
            colonne: p.colonne,
        }));
    }
    Ok(rangees.into_iter().flatten().collect())
}

fn positions_plages(defaut: DefautPositions) -> ErreurPont {
    ErreurPont::PositionsPlages { defaut }
}

/// Position de chaque plage de la mire, dans l'ordre de la définition (celui
/// des SAMPLE_ID) : zones dans l'ordre, puis rangées de haut en bas, puis
/// colonnes de gauche à droite.
fn positions(mire: &DefinitionFeuille) -> Vec<PositionPlage> {
    mire.zones
        .iter()
        .enumerate()
        .flat_map(|(zone, z)| {
            (0..z.rangees).flat_map(move |rangee| {
                (0..z.colonnes).map(move |colonne| PositionPlage {
                    // Bornés par ZONES_MAX, RANGEES_MAX et COLONNES_MAX.
                    zone_ou_index: zone as u32,
                    rangee: rangee as i32,
                    colonne: colonne as i32,
                })
            })
        })
        .collect()
}
