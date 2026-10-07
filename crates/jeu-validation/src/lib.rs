//! Jeu de validation du pilote libre (ADR 0006) : paires « données brutes →
//! spectres M0, M1, M2 » produites par la DLL du fabricant, et banc qui compare
//! un calcul candidat à ces paires.
//!
//! Indépendant de Windows et des ponts : la crate ne lit que du texte (les
//! sorties archivées) et ne touche à aucun instrument. Format décrit dans
//! `docs/pilote-libre/jeu-validation.md`.

use serde::{Deserialize, Serialize};

/// Valeurs de données brutes par plage pour le MYIRO-1.
pub const NOMBRE_BRUTES: usize = 152;
/// Spectre de 380 à 730 nm par pas de 10 nm.
pub const NOMBRE_LONGUEURS: usize = 36;
/// Identifiant du format, écrit en tête de chaque jeu.
pub const FORMAT: &str = "myiro-libre/jeu-validation/1";

/// Longueur d'onde, en nm, de la valeur n° `indice` d'un spectre.
pub fn longueur_onde(indice: usize) -> u32 {
    380 + 10 * indice as u32
}

/// Condition de mesure (ISO 13655) sous laquelle la DLL a calculé un spectre.
///
/// **Confirmé** : le pont demande les spectres avec `Illuminant` = 0, 1, 2, qui
/// donnent M0, M1, M2 d'après la fiche `docs/abi/FDX_GetMeasureData.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    M0,
    M1,
    M2,
}

impl Condition {
    pub const TOUTES: [Condition; 3] = [Condition::M0, Condition::M1, Condition::M2];

    fn indice(self) -> usize {
        self as usize
    }
}

/// Une sortie du pont archivée : son nom (celui du fichier, par exemple) et son texte.
pub struct SortieArchivee<'a> {
    pub nom: &'a str,
    pub contenu: &'a str,
}

/// Le jeu de validation : toutes les paires extraites, sans identifiant d'instrument.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JeuValidation {
    pub format: String,
    pub longueurs_onde: Vec<u32>,
    pub paires: Vec<Paire>,
}

/// Une plage mesurée : ses données brutes et les trois spectres calculés par la DLL.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paire {
    /// Numéro d'ordre de la sortie archivée d'où vient la plage (« sortie-1 »…),
    /// jamais son nom de fichier.
    pub source: String,
    /// Nom de la plage dans cette sortie (« papier », « 1A1 »…).
    pub plage: String,
    /// Instrument et chaîne logicielle, sans identifiant ; absent si la sortie
    /// ne les porte pas (CSV des tests sur instrument).
    pub instrument: Option<Origine>,
    pub brutes: Vec<f32>,
    /// Spectres M0, M1, M2 dans cet ordre (correspondance confirmée, voir [`Condition`]).
    pub spectres: [Vec<f32>; 3],
}

impl Paire {
    pub fn spectre(&self, condition: Condition) -> &[f32] {
        &self.spectres[condition.indice()]
    }
}

/// Spectres M0, M1, M2, dans cet ordre, de 380 à 730 nm par 10 nm.
pub type Spectres = [Vec<f32>; 3];

/// Un calcul « données brutes → spectres » à valider : un candidat synthétique
/// en test, le pilote libre ensuite.
pub trait CalculSpectres {
    fn calculer(&self, brutes: &[f32]) -> Result<Spectres, String>;
}

/// Résultat du banc : écarts par condition de mesure, et plages que le
/// candidat n'a pas su calculer (exclues des écarts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rapport {
    /// M0, M1, M2 dans cet ordre.
    pub conditions: [RapportCondition; 3],
    pub echecs: Vec<Echec>,
}

impl Rapport {
    pub fn condition(&self, condition: Condition) -> &RapportCondition {
        &self.conditions[condition.indice()]
    }
}

/// Écarts absolus entre le candidat et la DLL pour une condition de mesure,
/// en facteur de réflexion (0 à 1). Inconnus (`None`) si aucune paire n'a été
/// comparée : jamais remplacés par zéro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RapportCondition {
    pub condition: Condition,
    /// Nombre de paires comparées.
    pub paires: usize,
    pub ecart_moyen: Option<f64>,
    pub ecart_maximal: Option<f64>,
    pub par_longueur_onde: Vec<EcartLongueurOnde>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EcartLongueurOnde {
    /// En nm.
    pub longueur_onde: u32,
    pub ecart_moyen: Option<f64>,
    pub ecart_maximal: Option<f64>,
}

/// Plage que le candidat n'a pas calculée, ou a rendue de forme imprévue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Echec {
    pub source: String,
    pub plage: String,
    pub detail: String,
}

/// Compare le candidat à chaque paire du jeu, condition par condition.
pub fn comparer(jeu: &JeuValidation, candidat: &impl CalculSpectres) -> Rapport {
    // Somme et maximum des écarts absolus, par condition et longueur d'onde.
    let mut sommes = [[0.0f64; NOMBRE_LONGUEURS]; 3];
    let mut maximums = [[0.0f64; NOMBRE_LONGUEURS]; 3];
    let mut comparees = 0usize;
    let mut echecs = Vec::new();
    for paire in &jeu.paires {
        let echec = |detail: String| Echec {
            source: paire.source.clone(),
            plage: paire.plage.clone(),
            detail,
        };
        // Un jeu relu depuis un fichier peut être abîmé : il est revérifié ici,
        // car une valeur manquante ou non finie ne doit jamais compter comme un écart nul.
        if let Some(defaut) = defaut_de_forme("données brutes", &paire.brutes, NOMBRE_BRUTES)
            .or_else(|| defaut_des_spectres(&paire.spectres))
        {
            echecs.push(echec(format!("paire du jeu : {defaut}")));
            continue;
        }
        let calcule = match candidat.calculer(&paire.brutes) {
            Ok(spectres) => spectres,
            Err(detail) => {
                echecs.push(echec(detail));
                continue;
            }
        };
        if let Some(defaut) = defaut_des_spectres(&calcule) {
            echecs.push(echec(format!("candidat : {defaut}")));
            continue;
        }
        comparees += 1;
        for c in 0..3 {
            for (l, (a, b)) in calcule[c].iter().zip(&paire.spectres[c]).enumerate() {
                let ecart = (f64::from(*a) - f64::from(*b)).abs();
                sommes[c][l] += ecart;
                maximums[c][l] = maximums[c][l].max(ecart);
            }
        }
    }
    let connu = |v: f64| (comparees > 0).then_some(v);
    let n = comparees as f64;
    let conditions = Condition::TOUTES.map(|condition| {
        let c = condition.indice();
        RapportCondition {
            condition,
            paires: comparees,
            ecart_moyen: connu(sommes[c].iter().sum::<f64>() / (n * NOMBRE_LONGUEURS as f64)),
            ecart_maximal: connu(maximums[c].iter().copied().fold(0.0, f64::max)),
            par_longueur_onde: (0..NOMBRE_LONGUEURS)
                .map(|l| EcartLongueurOnde {
                    longueur_onde: longueur_onde(l),
                    ecart_moyen: connu(sommes[c][l] / n),
                    ecart_maximal: connu(maximums[c][l]),
                })
                .collect(),
        }
    });
    Rapport { conditions, echecs }
}

/// Décrit le défaut d'une suite de valeurs : mauvaise longueur ou valeur non finie.
fn defaut_de_forme(quoi: &str, valeurs: &[f32], attendu: usize) -> Option<String> {
    if valeurs.len() != attendu {
        return Some(format!(
            "{quoi} : {} valeurs au lieu de {attendu}",
            valeurs.len()
        ));
    }
    valeurs
        .iter()
        .position(|v| !v.is_finite())
        .map(|i| format!("{quoi} : valeur n° {} non finie", i + 1))
}

fn defaut_des_spectres(spectres: &Spectres) -> Option<String> {
    Condition::TOUTES.iter().find_map(|&condition| {
        defaut_de_forme(
            &format!("spectre {condition:?}"),
            &spectres[condition.indice()],
            NOMBRE_LONGUEURS,
        )
    })
}

/// Ce que la provenance du pont dit de l'instrument et de la chaîne logicielle,
/// une fois le numéro de série remplacé par un pseudonyme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origine {
    /// « instrument-1 », « instrument-2 »… : un par numéro de série distinct
    /// rencontré dans l'extraction, dans l'ordre d'apparition.
    pub pseudonyme: String,
    pub modele: String,
    pub micrologiciel: String,
    pub version_sdk: [u32; 3],
    pub empreinte_dll: Option<String>,
    pub calcul: String,
}

/// Extrait les paires de sorties archivées du pont : CSV des tests sur
/// instrument (`plage;donnees;L;a;b;nm380…`) ou lignes JSON du protocole.
/// Toute plage incomplète ou de forme imprévue est refusée, sans deviner.
pub fn extraire(sorties: &[SortieArchivee]) -> Result<JeuValidation, String> {
    let mut paires = Vec::new();
    // Numéros de série rencontrés, dans l'ordre : leur rang donne le pseudonyme.
    let mut series: Vec<u32> = Vec::new();
    for (k, sortie) in sorties.iter().enumerate() {
        // Un nom de fichier peut contenir un numéro de série : le jeu ne garde
        // qu'un numéro d'ordre ; le nom ne sert qu'aux messages d'erreur.
        let source = format!("sortie-{}", k + 1);
        if sortie.contenu.trim_start().starts_with('{') {
            paires.extend(extraire_json(sortie, &source, &mut series)?);
        } else {
            paires.extend(extraire_csv(sortie, &source)?);
        }
    }
    Ok(JeuValidation {
        format: FORMAT.into(),
        longueurs_onde: (0..NOMBRE_LONGUEURS).map(longueur_onde).collect(),
        paires,
    })
}

/// Lignes JSON du protocole (`pont-protocole`) : seules les réponses `mesure`
/// portent des paires ; le numéro de série de leur provenance est remplacé, et
/// les autres identifiants (code produit, adresse MAC des réponses `connecte`)
/// ne sont pas repris.
fn extraire_json(
    sortie: &SortieArchivee,
    source: &str,
    series: &mut Vec<u32>,
) -> Result<Vec<Paire>, String> {
    use pont_protocole::{lire_reponse, Reponse};
    let mut paires = Vec::new();
    let mut mesures = 0;
    for (n, ligne) in sortie.contenu.lines().enumerate() {
        if ligne.trim().is_empty() {
            continue;
        }
        let lieu = || format!("{}, ligne {}", sortie.nom, n + 1);
        let reponse = lire_reponse(ligne).map_err(|e| format!("{} : {e}", lieu()))?;
        // Une mesure relue est déjà vérifiée (nombres finis, plages cohérentes,
        // format initial ou courant) ; restent les dimensions du jeu.
        let Reponse::Mesure { mesure } = reponse else {
            continue;
        };
        mesures += 1;
        let provenance = mesure.provenance();
        let serie = provenance.instrument.numero_serie;
        let rang = match series.iter().position(|&s| s == serie) {
            Some(rang) => rang,
            None => {
                series.push(serie);
                series.len() - 1
            }
        };
        let origine = Origine {
            pseudonyme: format!("instrument-{}", rang + 1),
            modele: provenance.instrument.modele.clone(),
            micrologiciel: provenance.instrument.micrologiciel.clone(),
            version_sdk: provenance.version_sdk,
            empreinte_dll: provenance
                .empreinte_dll
                .valeur()
                .map(|e| e.texte().to_string()),
            calcul: provenance.calcul.libelle.clone(),
        };
        for (k, plage) in mesure.plages().iter().enumerate() {
            let nom = format!("mesure-{mesures}/plage-{}", k + 1);
            for (quoi, valeurs, attendu) in [
                ("données brutes", &plage.brutes()[..], NOMBRE_BRUTES),
                ("spectre M0", &plage.m0()[..], NOMBRE_LONGUEURS),
                ("spectre M1", &plage.m1()[..], NOMBRE_LONGUEURS),
                ("spectre M2", &plage.m2()[..], NOMBRE_LONGUEURS),
            ] {
                if valeurs.len() != attendu {
                    return Err(format!(
                        "{} : {nom}, {quoi} : {} valeurs au lieu de {attendu}",
                        lieu(),
                        valeurs.len()
                    ));
                }
            }
            paires.push(Paire {
                source: source.into(),
                plage: nom,
                instrument: Some(origine.clone()),
                brutes: plage.brutes().to_vec(),
                spectres: [
                    plage.m0().to_vec(),
                    plage.m1().to_vec(),
                    plage.m2().to_vec(),
                ],
            });
        }
    }
    Ok(paires)
}

#[derive(Default)]
struct PlageEnCours {
    nom: String,
    spectres: [Option<Vec<f32>>; 3],
    brutes: Option<Vec<f32>>,
}

fn extraire_csv(sortie: &SortieArchivee, source: &str) -> Result<Vec<Paire>, String> {
    let mut lignes = sortie.contenu.lines().enumerate();
    match lignes.next() {
        Some((_, entete)) if entete.starts_with("plage;donnees;") => {}
        _ => return Err(format!("{} : en-tête CSV du pont absente", sortie.nom)),
    }
    let mut plages: Vec<PlageEnCours> = Vec::new();
    for (n, ligne) in lignes {
        if ligne.trim().is_empty() {
            continue;
        }
        let lieu = || format!("{}, ligne {}", sortie.nom, n + 1);
        let champs: Vec<&str> = ligne.split(';').collect();
        // Plage, données, L, a, b, puis au moins une valeur.
        if champs.len() < 6 {
            return Err(format!("{} : ligne trop courte", lieu()));
        }
        let valeurs = champs[5..]
            .iter()
            .map(|v| v.trim().parse::<f32>())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(|e| format!("{} : valeur illisible ({e})", lieu()))?;
        // NaN et infinis se lisent comme des nombres : ce ne sont pas des mesures.
        if valeurs.iter().any(|v| !v.is_finite()) {
            return Err(format!("{} : valeur non finie", lieu()));
        }
        let nom = champs[0];
        if plages.last().map(|p| p.nom.as_str()) != Some(nom) {
            if plages.iter().any(|p| p.nom == nom) {
                return Err(format!(
                    "{} : la plage {nom} revient après une autre plage",
                    lieu()
                ));
            }
            plages.push(PlageEnCours {
                nom: nom.into(),
                ..Default::default()
            });
        }
        let plage = plages.last_mut().expect("plage ajoutée ci-dessus");
        let (case, attendu) = match champs[1] {
            "M0" => (&mut plage.spectres[0], NOMBRE_LONGUEURS),
            "M1" => (&mut plage.spectres[1], NOMBRE_LONGUEURS),
            "M2" => (&mut plage.spectres[2], NOMBRE_LONGUEURS),
            "brutes" => (&mut plage.brutes, NOMBRE_BRUTES),
            autre => return Err(format!("{} : type de données inconnu « {autre} »", lieu())),
        };
        if valeurs.len() != attendu {
            return Err(format!(
                "{} : {} valeurs au lieu de {attendu}",
                lieu(),
                valeurs.len()
            ));
        }
        if case.replace(valeurs).is_some() {
            return Err(format!("{} : données en double pour {nom}", lieu()));
        }
    }
    plages
        .into_iter()
        .map(|p| {
            let manque = |quoi: &str| format!("{} : plage {} sans {quoi}", sortie.nom, p.nom);
            let [m0, m1, m2] = p.spectres;
            Ok(Paire {
                source: source.into(),
                instrument: None,
                brutes: p.brutes.ok_or_else(|| manque("données brutes"))?,
                spectres: [
                    m0.ok_or_else(|| manque("spectre M0"))?,
                    m1.ok_or_else(|| manque("spectre M1"))?,
                    m2.ok_or_else(|| manque("spectre M2"))?,
                ],
                plage: p.nom,
            })
        })
        .collect()
}
