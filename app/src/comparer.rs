//! Comparer des profils ICC RVB (ticket #71) : de un à quatre profils, rangés
//! dans la bibliothèque ou ouverts depuis un fichier, analysés par la crate
//! `gamut` ; la limite du gamut est calculée par `iccgamut`
//! ([`MoteurGamut`]).
//!
//! Les méthodes de [`BibliothequeOuverte`] se testent sans Tauri, avec
//! `MoteurGamutSimule`. Les octets des profils sont lus sous le verrou ; tout
//! le calcul, `iccgamut` compris, se fait après : la page reste utilisable.
//! Ce qui n'est pas connu reste « inconnu », avec la clé de sa raison.

pub mod embarquees;
mod sources;

use gamut::{Analyse, Ecarts, ErreurIcc, MaillageAffiche, ProfilIcc, Repetabilite, Statistiques};
use pont_protocole::ConditionMesure;
use profilage::{ErreurMoteur, Etape, MoteurGamut};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

use crate::colonne::BibliothequeOuverte;
use crate::parc::si_choisi;
use crate::profiler::Moteur;
use embarquees::{mesures_embarquees, MesuresProfil, LAB_INCONNU, SANS_MESURES};
use sources::{
    ProfilLu, AUCUN_PROFIL, FICHIER_FERME, FICHIER_ILLISIBLE, FICHIER_TROP_GRAND, PROFIL_INCONNU,
    TROP_DE_PROFILS,
};

pub use sources::{
    FichierOuvertResume, FichiersOuverts, ProfilComparable, SourceComparaison, MAX_PROFILS,
};

const SANS_ARGYLL: &str = "comparer.limite.sans_argyll";
const SANS_ICCGAMUT: &str = "comparer.limite.sans_iccgamut";
const ARGYLL_VERSION: &str = "comparer.limite.argyll_version";
const ARGYLL_ILLISIBLE: &str = "comparer.limite.argyll_illisible";
const LIMITE_FICHIER: &str = "comparer.limite.fichier";
const LIMITE_ILLISIBLE: &str = "comparer.limite.illisible";
const LIMITE_INTERROMPUE: &str = "comparer.limite.interrompue";
/// Échec d'`iccgamut` lui-même (clé de `ErreurMoteur::cle`).
const ERREUR_LIMITE: &str = "comparer.erreur.limite";

/// Clés des refus et des limites indisponibles de la comparaison.
pub const CLES_COMPARAISON: [&str; 14] = [
    AUCUN_PROFIL,
    TROP_DE_PROFILS,
    PROFIL_INCONNU,
    FICHIER_FERME,
    FICHIER_ILLISIBLE,
    FICHIER_TROP_GRAND,
    SANS_ARGYLL,
    SANS_ICCGAMUT,
    ARGYLL_VERSION,
    ARGYLL_ILLISIBLE,
    LIMITE_FICHIER,
    LIMITE_ILLISIBLE,
    LIMITE_INTERROMPUE,
    ERREUR_LIMITE,
];

/// Clé du texte qui dit pourquoi la limite du gamut n'est pas calculée. Ce
/// sont des textes de la comparaison : ils ne parlent ni de profil rangé ni
/// de calcul de profil.
pub fn cle_limite(erreur: &ErreurMoteur) -> &'static str {
    match erreur {
        ErreurMoteur::Introuvable => SANS_ARGYLL,
        ErreurMoteur::VersionNonValidee { .. } => ARGYLL_VERSION,
        ErreurMoteur::OutilManquant { .. } => SANS_ICCGAMUT,
        ErreurMoteur::Echec {
            etape: Etape::Version,
            ..
        } => ARGYLL_ILLISIBLE,
        // Une recette ne concerne que le calcul d'un profil : jamais rendue ici.
        ErreurMoteur::Echec { .. } | ErreurMoteur::RecetteEnReglage => ERREUR_LIMITE,
        ErreurMoteur::Interrompu { .. } => LIMITE_INTERROMPUE,
        ErreurMoteur::Fichier(_) => LIMITE_FICHIER,
    }
}

/// Ce qu'ArgyllCMS ou le système a dit, montré après le texte de la clé ;
/// pour une autre version, la version trouvée (`{version}` du texte).
fn cause_limite(erreur: &ErreurMoteur) -> Option<String> {
    match erreur {
        ErreurMoteur::Echec { cause, .. } | ErreurMoteur::Fichier(cause) => Some(cause.clone()),
        ErreurMoteur::VersionNonValidee { trouvee } => Some(trouvee.clone()),
        ErreurMoteur::Introuvable
        | ErreurMoteur::OutilManquant { .. }
        | ErreurMoteur::RecetteEnReglage
        | ErreurMoteur::Interrompu { .. } => None,
    }
}

/// Limite du gamut d'un profil (iccgamut), ou pourquoi elle manque.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatLimite {
    Calculee {
        maillage: MaillageAffiche,
        /// Volume annoncé par iccgamut, s'il a pu être lu.
        volume: Option<f64>,
        /// Sortie d'iccgamut, sous sa ligne de commande.
        journal: String,
    },
    Indisponible {
        cle: &'static str,
        cause: Option<String>,
    },
}

/// Fidélité d'un profil à ses mesures embarquées, ou pourquoi elle est
/// inconnue (jamais 0).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatFidelite {
    Connue {
        statistiques: Statistiques,
        plages: usize,
        /// Inconnue quand les mesures embarquées ne la disent pas.
        condition: Option<ConditionMesure>,
        condition_supposee: bool,
        rvb_supposes: bool,
        lab_calcule: bool,
    },
    Inconnue {
        cle: &'static str,
    },
}

/// Un profil analysé, ou refusé avec la structure qui en est la cause.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatProfil {
    Analyse {
        /// En boîte : bien plus grande que la raison d'un refus.
        analyse: Box<Analyse>,
        limite: EtatLimite,
        fidelite: EtatFidelite,
        repetabilite: Option<Repetabilite>,
    },
    Refuse {
        cle: &'static str,
        detail: Option<String>,
    },
}

/// Un profil de la comparaison et d'où il vient.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProfilCompare {
    pub source: SourceComparaison,
    pub nom: String,
    /// « Imprimante › Papier › Résolution » d'un profil de la bibliothèque,
    /// nom du fichier sinon (textes `comparer.origine.*`).
    pub origine: String,
    /// Date du profil de la bibliothèque (JJ/MM/AAAA) ; rien pour un fichier.
    pub date: Option<String>,
    pub etat: EtatProfil,
}

/// La comparaison : `ecarts[i]` et `memes_mesures[i]` comparent le profil i
/// au profil 0 (`None` et `false` pour 0 et pour un profil refusé).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Comparaison {
    pub profils: Vec<ProfilCompare>,
    pub ecarts: Vec<Option<Ecarts>>,
    pub memes_mesures: Vec<bool>,
    /// Commande d'iccgamut, sans ArgyllCMS rien.
    pub commande_limite: Option<String>,
}

/// Courbe (L\*, C\*) d'un profil le long d'un chemin du cube RVB, et ses
/// plages mesurées sur ce chemin (position 0–255, L\*, C\*).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheminProfil {
    pub points: Vec<[f64; 2]>,
    pub mesures: Vec<[f64; 3]>,
}

/// Ce qui précise un refus de lecture : structure, espace, balise.
fn detail_refus(erreur: &ErreurIcc) -> Option<String> {
    match erreur {
        ErreurIcc::EnTete(d) => Some(d.clone()),
        ErreurIcc::PasRvb { espace } => Some(espace.clone()),
        ErreurIcc::StructureNonPriseEnCharge { structure } => Some(structure.clone()),
        ErreurIcc::Tronque { balise } => Some(balise.clone()),
        ErreurIcc::SansTable | ErreurIcc::SansBlanc | ErreurIcc::SansTableInverse => None,
    }
}

/// Un profil lu, avec ses mesures embarquées (ou la clé de leur absence).
struct Lu {
    profil: ProfilIcc,
    mesures: Result<MesuresProfil, &'static str>,
}

fn lire(l: &ProfilLu) -> Result<Lu, ErreurIcc> {
    let profil = ProfilIcc::lire(&l.icc)?;
    let mesures = profil
        .mesures_embarquees()
        .map_or(Err(SANS_MESURES), mesures_embarquees);
    Ok(Lu { profil, mesures })
}

fn limite(icc: &[u8], moteur: Option<&dyn MoteurGamut>) -> EtatLimite {
    let Some(moteur) = moteur else {
        return EtatLimite::Indisponible {
            cle: SANS_ARGYLL,
            cause: None,
        };
    };
    match moteur.limite(icc) {
        Ok(l) => match gamut::lire_gam(&l.gam) {
            Ok(m) => EtatLimite::Calculee {
                maillage: gamut::maillage_affiche(&m),
                volume: l.volume,
                journal: l.journal,
            },
            Err(cause) => EtatLimite::Indisponible {
                cle: LIMITE_ILLISIBLE,
                cause: Some(cause),
            },
        },
        Err(e) => {
            eprintln!("{e}");
            EtatLimite::Indisponible {
                cle: cle_limite(&e),
                cause: cause_limite(&e),
            }
        }
    }
}

fn etat(icc: &[u8], lu: &Result<Lu, ErreurIcc>, moteur: Option<&dyn MoteurGamut>) -> EtatProfil {
    let lu = match lu {
        Ok(lu) => lu,
        Err(e) => {
            return EtatProfil::Refuse {
                cle: e.cle(),
                detail: detail_refus(e),
            }
        }
    };
    let fidelite = match &lu.mesures {
        Ok(m) => match gamut::fidelite(&lu.profil, &m.mesures) {
            Some(statistiques) => EtatFidelite::Connue {
                statistiques,
                plages: m.mesures.len(),
                condition: m.condition,
                condition_supposee: m.condition_supposee,
                rvb_supposes: m.rvb_supposes,
                lab_calcule: m.lab_calcule,
            },
            None => EtatFidelite::Inconnue { cle: LAB_INCONNU },
        },
        Err(cle) => EtatFidelite::Inconnue { cle },
    };
    EtatProfil::Analyse {
        analyse: Box::new(gamut::analyser(&lu.profil)),
        limite: limite(icc, moteur),
        fidelite,
        repetabilite: lu
            .mesures
            .as_ref()
            .ok()
            .and_then(|m| gamut::repetabilite(&m.mesures)),
    }
}

impl BibliothequeOuverte {
    /// Compare de un à quatre profils ; le premier sert de référence aux
    /// écarts. Un profil d'une structure non lue est refusé sans empêcher
    /// les autres. Sans `moteur` (ArgyllCMS absent), la limite du gamut est
    /// indisponible et tout le reste est calculé.
    pub fn comparer(
        &self,
        sources: &[SourceComparaison],
        fichiers: &FichiersOuverts,
        moteur: Option<&dyn MoteurGamut>,
    ) -> Result<Comparaison, String> {
        let lus = self.lire_sources(sources, fichiers)?;
        // Hors de tout verrou à partir d'ici.
        let profils: Vec<_> = lus.iter().map(|l| (l, lire(l))).collect();
        let reference = profils.first().and_then(|(_, p)| p.as_ref().ok());
        let mut ecarts = Vec::with_capacity(profils.len());
        let mut memes_mesures = Vec::with_capacity(profils.len());
        for (i, (_, p)) in profils.iter().enumerate() {
            let paire = reference.zip(p.as_ref().ok()).filter(|_| i > 0);
            ecarts.push(paire.map(|(r, p)| gamut::comparer(&r.profil, &p.profil)));
            memes_mesures.push(paire.is_some_and(|(r, p)| {
                matches!((&r.mesures, &p.mesures), (Ok(a), Ok(b)) if gamut::memes_mesures(&a.mesures, &b.mesures))
            }));
        }
        let profils = profils
            .iter()
            .map(|(l, p)| ProfilCompare {
                source: l.source,
                nom: l.nom.clone(),
                origine: l.origine.clone(),
                date: l.date.clone(),
                etat: etat(&l.icc, p, moteur),
            })
            .collect();
        Ok(Comparaison {
            profils,
            ecarts,
            memes_mesures,
            commande_limite: moteur.map(|m| m.commande_limite()),
        })
    }

    /// Courbes des profils le long du segment du cube RVB de `depart` à
    /// `arrivee`, avec leurs plages mesurées sur ce chemin. Un profil refusé
    /// donne une courbe vide.
    pub fn chemin_comparaison(
        &self,
        sources: &[SourceComparaison],
        fichiers: &FichiersOuverts,
        depart: [u8; 3],
        arrivee: [u8; 3],
    ) -> Result<Vec<CheminProfil>, String> {
        let lus = self.lire_sources(sources, fichiers)?;
        Ok(lus
            .iter()
            .map(|l| match lire(l) {
                Ok(lu) => CheminProfil {
                    points: gamut::chemin(&lu.profil, depart, arrivee),
                    mesures: lu.mesures.map_or_else(
                        |_| Vec::new(),
                        |m| gamut::points_du_chemin(&m.mesures, depart, arrivee),
                    ),
                },
                Err(_) => CheminProfil {
                    points: Vec::new(),
                    mesures: Vec::new(),
                },
            })
            .collect())
    }
}

// ---- Commandes de la page ----

#[tauri::command(async)]
pub fn comparer_profils_comparables(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
) -> Result<Vec<ProfilComparable>, String> {
    ouverte.profils_comparables()
}

/// Ouvre un profil choisi dans la fenêtre de Windows (`.icc`, `.icm`) pour
/// la séance, sans le ranger. `None` si l'opérateur a annulé.
#[tauri::command(async)]
pub fn comparer_ouvrir_fichier(
    app: tauri::AppHandle,
    fichiers: tauri::State<'_, FichiersOuverts>,
    filtre: String,
) -> Result<Option<FichierOuvertResume>, String> {
    let fichier = app
        .dialog()
        .file()
        .add_filter(filtre, &["icc", "icm"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok());
    si_choisi(fichier, |f| fichiers.ouvrir(f))
}

/// Analyse et compare les profils choisis. Hors du fil de la fenêtre :
/// iccgamut peut prendre quelques secondes par profil.
#[tauri::command(async)]
pub fn comparer_analyser(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    fichiers: tauri::State<'_, FichiersOuverts>,
    moteur: tauri::State<'_, Moteur>,
    sources: Vec<SourceComparaison>,
) -> Result<Comparaison, String> {
    ouverte.comparer(&sources, &fichiers, moteur.gamut())
}

#[tauri::command(async)]
pub fn comparer_chemin(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    fichiers: tauri::State<'_, FichiersOuverts>,
    sources: Vec<SourceComparaison>,
    depart: [u8; 3],
    arrivee: [u8; 3],
) -> Result<Vec<CheminProfil>, String> {
    ouverte.chemin_comparaison(&sources, &fichiers, depart, arrivee)
}
