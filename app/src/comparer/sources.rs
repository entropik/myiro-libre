//! Profils qu'on peut comparer : ceux de la bibliothèque, et les fichiers
//! `.icc` ouverts pour la séance, jamais rangés ni écrits sur le disque.

use std::path::Path;
use std::sync::{Arc, Mutex};

use bibliotheque::{Bibliotheque, ErreurBibliotheque, IdProfil};
use serde::{Deserialize, Serialize};

use crate::colonne::BibliothequeOuverte;
use crate::parc::lire_profil;

/// Nombre de profils comparés au plus.
pub const MAX_PROFILS: usize = 4;
pub(crate) const TROP_DE_PROFILS: &str = "comparer.refus.trop_de_profils";
pub(crate) const FICHIER_FERME: &str = "comparer.refus.fichier_ferme";

/// Un profil choisi par la page : rangé dans la bibliothèque, ou fichier
/// ouvert pour la séance ([`FichiersOuverts`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceComparaison {
    Bibliotheque { profil: i64 },
    Fichier { jeton: u32 },
}

/// Un fichier `.icc` ouvert pour la comparaison : il reste en mémoire le
/// temps de la séance.
struct FichierOuvert {
    jeton: u32,
    nom: String,
    icc: Arc<Vec<u8>>,
}

/// Ce que la page reçoit d'un fichier ouvert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FichierOuvertResume {
    pub jeton: u32,
    pub nom: String,
}

/// Les fichiers ouverts pour la comparaison pendant la séance.
#[derive(Default)]
pub struct FichiersOuverts(Mutex<Vec<FichierOuvert>>);

impl FichiersOuverts {
    /// Lit le fichier (borné comme un profil à ranger) et le garde en
    /// mémoire sous un nouveau jeton. Rien n'est écrit, rien n'est rangé ;
    /// le verrou n'est pris qu'une fois le fichier lu.
    pub fn ouvrir(&self, chemin: &Path) -> Result<FichierOuvertResume, String> {
        let icc = Arc::new(lire_profil(chemin)?);
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut ouverts = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let jeton = ouverts.iter().map(|f| f.jeton).max().unwrap_or(0) + 1;
        ouverts.push(FichierOuvert {
            jeton,
            nom: nom.clone(),
            icc,
        });
        Ok(FichierOuvertResume { jeton, nom })
    }

    fn lire(&self, jeton: u32) -> Option<(String, Arc<Vec<u8>>)> {
        let ouverts = self.0.lock().unwrap_or_else(|e| e.into_inner());
        ouverts
            .iter()
            .find(|f| f.jeton == jeton)
            .map(|f| (f.nom.clone(), Arc::clone(&f.icc)))
    }
}

/// Un profil rangé, tel que la liste de la comparaison le montre.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProfilComparable {
    pub profil: IdProfil,
    pub imprimante: String,
    pub papier: String,
    pub resolution: String,
    pub nom: String,
    pub horodatage: String,
    /// Profil actif de sa résolution.
    pub actif: bool,
}

/// Tous les profils du parc : chaque résolution, son historique du plus
/// récent au plus ancien.
fn comparables(b: &Bibliotheque) -> Result<Vec<ProfilComparable>, ErreurBibliotheque> {
    let parc = b.parc("", std::time::SystemTime::now())?;
    let mut liste = Vec::new();
    for i in &parc.imprimantes {
        for p in &i.papiers {
            for r in &p.reglages {
                for profil in b.historique_profils(r.condition.id)? {
                    liste.push(ProfilComparable {
                        profil: profil.id,
                        imprimante: i.imprimante.nom.clone(),
                        papier: p.papier.nom.clone(),
                        resolution: r.resolution.clone(),
                        nom: profil.nom,
                        horodatage: profil.horodatage,
                        actif: profil.actif,
                    });
                }
            }
        }
    }
    Ok(liste)
}

/// Un profil choisi, lu : ses octets et de quoi dire d'où il vient.
pub(crate) struct ProfilLu {
    pub source: SourceComparaison,
    pub nom: String,
    /// « Imprimante › Papier › Résolution », ou le nom du fichier.
    pub origine: String,
    /// Date de rangement (JJ/MM/AAAA) d'un profil de la bibliothèque.
    pub date: Option<String>,
    pub icc: Arc<Vec<u8>>,
}

/// `AAAA-MM-JJ…` vers `JJ/MM/AAAA` ; tel quel s'il n'a pas cette forme.
fn date_affichee(horodatage: &str) -> String {
    match (
        horodatage.get(0..4),
        horodatage.get(5..7),
        horodatage.get(8..10),
    ) {
        (Some(a), Some(m), Some(j)) => format!("{j}/{m}/{a}"),
        _ => horodatage.to_string(),
    }
}

impl BibliothequeOuverte {
    /// Les profils rangés qu'on peut comparer, résolution par résolution.
    pub fn profils_comparables(&self) -> Result<Vec<ProfilComparable>, String> {
        self.avec(comparables)
    }

    /// Lit les octets des profils choisis : la bibliothèque n'est verrouillée
    /// que le temps de cette lecture. Au-delà de [`MAX_PROFILS`], un profil
    /// inconnu ou un fichier fermé, rien n'est lu.
    pub(crate) fn lire_sources(
        &self,
        sources: &[SourceComparaison],
        fichiers: &FichiersOuverts,
    ) -> Result<Vec<ProfilLu>, String> {
        if sources.len() > MAX_PROFILS {
            return Err(TROP_DE_PROFILS.to_string());
        }
        let ranges = self.avec(|b| {
            let liste = comparables(b)?;
            sources
                .iter()
                .filter_map(|s| match s {
                    SourceComparaison::Bibliotheque { profil } => Some(IdProfil(*profil)),
                    SourceComparaison::Fichier { .. } => None,
                })
                .map(|id| {
                    let trouve = liste.iter().find(|p| p.profil == id).cloned();
                    trouve
                        .map(|p| b.fichier_profil(id).map(|icc| (p, icc)))
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()
        })?;
        let mut ranges = ranges.into_iter();
        sources
            .iter()
            .map(|&source| match source {
                SourceComparaison::Bibliotheque { .. } => {
                    let (p, icc) = ranges
                        .next()
                        .flatten()
                        .ok_or_else(|| "parc.erreur.profil_inconnu".to_string())?;
                    Ok(ProfilLu {
                        source,
                        origine: format!("{} › {} › {}", p.imprimante, p.papier, p.resolution),
                        date: Some(date_affichee(&p.horodatage)),
                        nom: p.nom,
                        icc: Arc::new(icc),
                    })
                }
                SourceComparaison::Fichier { jeton } => {
                    let (nom, icc) = fichiers
                        .lire(jeton)
                        .ok_or_else(|| FICHIER_FERME.to_string())?;
                    Ok(ProfilLu {
                        source,
                        origine: nom.clone(),
                        date: None,
                        nom,
                        icc,
                    })
                }
            })
            .collect()
    }
}
