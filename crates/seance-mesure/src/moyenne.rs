//! Moyennage de plusieurs exemplaires d'une même mire.

use std::collections::HashMap;

use cgats::{modele_instrument, MesureImportee, PlageImportee};
use colorimetrie::delta_e00;
use pont_protocole::{ConditionMesure, Info, Lab, Spectre};
use serde::{Deserialize, Serialize};

use crate::{associer, Plan};

/// Écart ΔE00 entre une mesure et la moyenne au-delà duquel une plage est
/// signalée avant acceptation. Proposé, en attente de validation du
/// mainteneur : aucune norme ne le fixe. C'est le seul endroit où cette
/// valeur est écrite.
pub const SEUIL_ECART_DE00: f64 = 1.0;

/// Noms des emplacements de spectre, comme dans les fichiers CGATS.
const EMPLACEMENTS: [&str; 3] = ["m0", "m1", "m2"];

/// Rôle d'une mesure dans une séance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// La mesure d'un exemplaire imprimé de la mire : toutes ses plages
    /// associées avec certitude comptent dans la moyenne.
    Exemplaire,
    /// Une mesure qui ne sert qu'aux compléments manuels qui la désignent.
    Complement,
}

/// Une mesure d'une séance, telle que la bibliothèque la garde.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// Numéro de la mesure dans la bibliothèque.
    pub id: i64,
    /// Empreinte de la mire de la séance où elle a été rangée.
    pub mire: String,
    pub role: Role,
    pub mesure: MesureImportee,
}

/// Un complément manuel : l'opérateur désigne la plage `identifiant` de la
/// mesure `source` pour la plage `plage` du plan, qu'aucun exemplaire ne
/// donne. Il reste distingué de la source dans la composition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Complement {
    pub plage: String,
    pub source: i64,
    pub identifiant: String,
}

/// Une mesure qui compte dans une plage du résultat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Apport {
    pub source: i64,
    /// Identifiant de la plage dans la mesure source.
    pub identifiant: String,
    /// Désignée à la main (complément manuel).
    pub manuel: bool,
}

/// Les mesures dont une plage du plan est la moyenne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionPlage {
    pub plage: String,
    pub apports: Vec<Apport>,
}

/// Écart entre une mesure et la moyenne, pour une plage mesurée au moins
/// deux fois : le plus grand ΔE00 des emplacements qui ont un Lab.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ecart {
    pub plage: String,
    pub source: i64,
    /// Inconnu sans Lab, ou s'il ne se calcule pas.
    pub de00: Option<f64>,
    /// Au-delà de [`SEUIL_ECART_DE00`] : à montrer avant acceptation. Rien
    /// n'est exclu pour autant.
    pub signale: bool,
}

/// Le résultat du moyennage.
#[derive(Clone, Debug, PartialEq)]
pub struct Moyenne {
    /// La mesure moyennée : plages du plan qui ont au moins un apport, dans
    /// l'ordre du plan, RVB de la mire confirmés. Sans provenance de pont,
    /// sans date ni numéro de série : l'instrument est le seul modèle.
    pub mesure: MesureImportee,
    /// Plages du plan qu'aucune mesure ne donne.
    pub manquantes: Vec<String>,
    /// Pour chaque plage de la mesure moyennée, ses apports.
    pub composition: Vec<CompositionPlage>,
    /// Tous les écarts, signalés ou non.
    pub ecarts: Vec<Ecart>,
}

impl Moyenne {
    pub fn complete(&self) -> bool {
        self.manquantes.is_empty()
    }

    /// Écarts au-delà de [`SEUIL_ECART_DE00`].
    pub fn ecarts_signales(&self) -> impl Iterator<Item = &Ecart> {
        self.ecarts.iter().filter(|e| e.signale)
    }
}

/// Pourquoi des mesures ne se moyennent pas. Une source est nommée par son
/// numéro dans la bibliothèque, jamais par le numéro de série de son
/// instrument.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "refus", rename_all = "snake_case")]
pub enum Incompatible {
    AucunExemplaire,
    AutreMire { source: i64 },
    AssociationIncertaine { source: i64 },
    ModeleInconnu { source: i64 },
    ModelesDifferents { source: i64 },
    InstrumentsDifferents { source: i64 },
    ConditionsDifferentes { source: i64 },
    LongueursOndeDifferentes { source: i64 },
    GeometriesDifferentes { source: i64 },
    SpectresMeles { emplacement: String },
    LabMeles { emplacement: String },
    ComplementInvalide { plage: String },
}

/// Moyenne les mesures d'une séance, plage par plage.
///
/// Règles de compatibilité, contrôlées avant tout calcul, dans cet ordre ;
/// la première qui manque refuse tout, rien n'est écarté en silence :
///
/// 1. au moins un exemplaire ([`Role::Exemplaire`]) ;
/// 2. toutes les mesures viennent de la mire du plan (même empreinte) ;
/// 3. chaque exemplaire est associé au plan avec certitude ([`associer`]) ;
///    un exemplaire incomplet compte pour les plages qu'il donne ;
/// 4. le modèle d'instrument est connu, le même partout : jamais de
///    mélange MYIRO-1/FD-9 ;
/// 5. même instrument : deux numéros de série connus sont égaux ;
/// 6. mêmes conditions de mesure, emplacement par emplacement ;
/// 7. mêmes longueurs d'onde, quand elles sont connues des deux côtés ;
/// 8. même géométrie de mesure, quand elle est connue des deux côtés ;
/// 9. chaque complément manuel désigne une plage du plan qu'aucun
///    exemplaire ne donne, une seule fois, dans une mesure de rôle
///    [`Role::Complement`] de la liste qui a cet identifiant une fois ;
/// 10. pour chaque emplacement, toutes les mesures d'une plage ont un
///     spectre (de même longueur), ou aucune ; de même pour le Lab. Pas de
///     moyenne mêlant des mesures avec et sans spectre.
///
/// Les spectres sont moyennés valeur par valeur et les Lab composante par
/// composante, sans recalcul : un seul exemplaire est rendu tel quel. Les
/// écarts à la moyenne sont tous écrits, signalés au-delà de
/// [`SEUIL_ECART_DE00`] ; aucune mesure n'est exclue d'office.
pub fn moyenner(
    plan: &Plan,
    sources: &[Source],
    complements: &[Complement],
) -> Result<Moyenne, Incompatible> {
    if !sources.iter().any(|s| s.role == Role::Exemplaire) {
        return Err(Incompatible::AucunExemplaire);
    }
    let mut apports: Vec<Vec<(usize, usize, bool)>> = vec![Vec::new(); plan.plages.len()];
    for (n, s) in sources.iter().enumerate() {
        if s.mire != plan.mire.empreinte {
            return Err(Incompatible::AutreMire { source: s.id });
        }
        if s.role == Role::Exemplaire {
            let association = associer(plan, &s.mesure);
            if !association.certaine() {
                return Err(Incompatible::AssociationIncertaine { source: s.id });
            }
            for (i, rang) in association.rangs.iter().enumerate() {
                if let Some(rang) = rang {
                    apports[i].push((n, *rang, false));
                }
            }
        }
    }
    let modele = meme_instrument(sources)?;
    completer(plan, sources, complements, &mut apports)?;
    verifier_melanges(sources, &apports)?;

    let premiere = &sources[0].mesure;
    let mut plages = Vec::new();
    let mut manquantes = Vec::new();
    let mut composition = Vec::new();
    let mut ecarts = Vec::new();
    for (attendue, apports) in plan.plages.iter().zip(&apports) {
        if apports.is_empty() {
            manquantes.push(attendue.identifiant.clone());
            continue;
        }
        let mesurees: Vec<&PlageImportee> = apports
            .iter()
            .map(|&(n, rang, _)| &sources[n].mesure.plages[rang])
            .collect();
        let plage = PlageImportee {
            identifiant: attendue.identifiant.clone(),
            spectres: std::array::from_fn(|e| moyenne_spectres(&mesurees, e)),
            lab: std::array::from_fn(|e| moyenne_lab(&mesurees, e)),
            rvb: Info::Confirmee(attendue.rvb.map(|c| (c / 100.0) as f32)),
        };
        if apports.len() > 1 {
            for (&(n, _, _), mesuree) in apports.iter().zip(&mesurees) {
                let de00 = ecart(mesuree, &plage);
                ecarts.push(Ecart {
                    plage: attendue.identifiant.clone(),
                    source: sources[n].id,
                    de00,
                    signale: de00.is_some_and(|d| d > SEUIL_ECART_DE00),
                });
            }
        }
        composition.push(CompositionPlage {
            plage: attendue.identifiant.clone(),
            apports: apports
                .iter()
                .map(|&(n, rang, manuel)| Apport {
                    source: sources[n].id,
                    identifiant: sources[n].mesure.plages[rang].identifiant.clone(),
                    manuel,
                })
                .collect(),
        });
        plages.push(plage);
    }
    Ok(Moyenne {
        mesure: MesureImportee {
            provenance: Info::Inconnue,
            instrument: Info::Confirmee(modele),
            numero_serie: Info::Inconnue,
            date: Info::Inconnue,
            conditions: std::array::from_fn(|e| condition_commune(sources, e)),
            longueurs_onde: sources
                .iter()
                .find(|s| s.mesure.longueurs_onde != Info::Inconnue)
                .map_or(premiere.longueurs_onde.clone(), |s| {
                    s.mesure.longueurs_onde.clone()
                }),
            plages,
        },
        manquantes,
        composition,
        ecarts,
    })
}

/// Règles 4 à 8 : rend le modèle commun.
fn meme_instrument(sources: &[Source]) -> Result<String, Incompatible> {
    let mut modele: Option<String> = None;
    let premiere = &sources[0].mesure;
    for s in sources {
        let m = &s.mesure;
        let Some(ce_modele) = m.instrument.valeur().and_then(|i| modele_instrument(i)) else {
            return Err(Incompatible::ModeleInconnu { source: s.id });
        };
        match &modele {
            Some(commun) if *commun != ce_modele => {
                return Err(Incompatible::ModelesDifferents { source: s.id })
            }
            _ => modele = Some(ce_modele),
        }
        let source = s.id;
        if let (Some(a), Some(b)) = (premiere.numero_serie.valeur(), m.numero_serie.valeur()) {
            if a.trim() != b.trim() {
                return Err(Incompatible::InstrumentsDifferents { source });
            }
        }
        if (0..3).any(|e| premiere.conditions[e].valeur() != m.conditions[e].valeur()) {
            return Err(Incompatible::ConditionsDifferentes { source });
        }
        if let (Some(a), Some(b)) = (premiere.longueurs_onde.valeur(), m.longueurs_onde.valeur()) {
            if a != b {
                return Err(Incompatible::LongueursOndeDifferentes { source });
            }
        }
        let geometrie = |m: &MesureImportee| m.provenance.valeur().map(|p| p.geometrie.clone());
        if let (Some(a), Some(b)) = (geometrie(premiere), geometrie(m)) {
            if a != b {
                return Err(Incompatible::GeometriesDifferentes { source });
            }
        }
    }
    Ok(modele.expect("au moins une source"))
}

/// Règle 9 : ajoute les compléments manuels aux apports.
fn completer(
    plan: &Plan,
    sources: &[Source],
    complements: &[Complement],
    apports: &mut [Vec<(usize, usize, bool)>],
) -> Result<(), Incompatible> {
    let index: HashMap<&str, usize> = plan
        .plages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.identifiant.as_str(), i))
        .collect();
    for c in complements {
        let invalide = || Incompatible::ComplementInvalide {
            plage: c.plage.clone(),
        };
        let i = *index.get(c.plage.as_str()).ok_or_else(invalide)?;
        let n = sources
            .iter()
            .position(|s| s.id == c.source && s.role == Role::Complement)
            .ok_or_else(invalide)?;
        let rangs: Vec<usize> = sources[n]
            .mesure
            .plages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.identifiant == c.identifiant)
            .map(|(r, _)| r)
            .collect();
        let [rang] = rangs[..] else {
            return Err(invalide());
        };
        if !apports[i].is_empty() {
            return Err(invalide());
        }
        apports[i].push((n, rang, true));
    }
    Ok(())
}

/// Règle 10, emplacement par emplacement, sur toutes les plages.
fn verifier_melanges(
    sources: &[Source],
    apports: &[Vec<(usize, usize, bool)>],
) -> Result<(), Incompatible> {
    for (e, nom) in EMPLACEMENTS.iter().enumerate() {
        let mesurees = || {
            apports
                .iter()
                .flatten()
                .map(|&(n, rang, _)| (n, &sources[n].mesure.plages[rang]))
        };
        let avec_spectre = mesurees()
            .filter(|(_, p)| p.spectres[e].valeur().is_some())
            .count();
        let total = mesurees().count();
        if avec_spectre != 0 && avec_spectre != total {
            return Err(Incompatible::SpectresMeles {
                emplacement: (*nom).into(),
            });
        }
        let mut longueur = None;
        for (n, p) in mesurees() {
            if let Some(sp) = p.spectres[e].valeur() {
                match longueur {
                    Some(l) if l != sp.len() => {
                        return Err(Incompatible::LongueursOndeDifferentes {
                            source: sources[n].id,
                        })
                    }
                    _ => longueur = Some(sp.len()),
                }
            }
        }
        let avec_lab = mesurees()
            .filter(|(_, p)| p.lab[e].valeur().is_some())
            .count();
        if avec_lab != 0 && avec_lab != total {
            return Err(Incompatible::LabMeles {
                emplacement: (*nom).into(),
            });
        }
    }
    Ok(())
}

fn moyenne_spectres(mesurees: &[&PlageImportee], e: usize) -> Info<Spectre> {
    let spectres: Option<Vec<&Spectre>> = mesurees.iter().map(|p| p.spectres[e].valeur()).collect();
    let Some(spectres) = spectres else {
        return Info::Inconnue;
    };
    let n = spectres.len() as f64;
    let valeurs = (0..spectres[0].len())
        .map(|i| (spectres.iter().map(|s| f64::from(s[i])).sum::<f64>() / n) as f32)
        .collect();
    Spectre::new(valeurs).map_or(Info::Inconnue, Info::Confirmee)
}

fn moyenne_lab(mesurees: &[&PlageImportee], e: usize) -> Info<Lab> {
    let labs: Option<Vec<[f32; 3]>> = mesurees
        .iter()
        .map(|p| p.lab[e].valeur().map(Lab::valeurs))
        .collect();
    let Some(labs) = labs else {
        return Info::Inconnue;
    };
    let n = labs.len() as f64;
    let v = std::array::from_fn(|k| (labs.iter().map(|l| f64::from(l[k])).sum::<f64>() / n) as f32);
    Lab::new(v).map_or(Info::Inconnue, Info::Confirmee)
}

fn vers_colorimetrie(lab: &Lab) -> colorimetrie::Lab {
    let [l, a, b] = lab.valeurs().map(f64::from);
    colorimetrie::Lab { l, a, b }
}

/// Le plus grand ΔE00 entre une mesure et la moyenne, sur les emplacements
/// où les deux ont un Lab.
fn ecart(mesuree: &PlageImportee, moyenne: &PlageImportee) -> Option<f64> {
    (0..3)
        .filter_map(|e| {
            let a = mesuree.lab[e].valeur()?;
            let b = moyenne.lab[e].valeur()?;
            delta_e00(vers_colorimetrie(a), vers_colorimetrie(b)).ok()
        })
        .reduce(f64::max)
}

/// Condition d'un emplacement, la même partout (règle 6) : confirmée si
/// toutes les mesures la confirment, supposée sinon.
fn condition_commune(sources: &[Source], e: usize) -> Info<ConditionMesure> {
    let conditions: Vec<&Info<ConditionMesure>> =
        sources.iter().map(|s| &s.mesure.conditions[e]).collect();
    match conditions[0].valeur() {
        None => Info::Inconnue,
        Some(c) if conditions.iter().all(|i| matches!(i, Info::Confirmee(_))) => {
            Info::Confirmee(*c)
        }
        Some(c) => Info::Supposee(*c),
    }
}
