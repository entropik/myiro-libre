//! Parc du poste : imprimantes, papiers et résolutions d'impression. Une
//! condition d'impression est la feuille de l'arbre imprimante → papier →
//! résolution (GLOSSARY) ; une condition sans papier est « non classée ».

use std::time::SystemTime;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{
    replier, Bibliotheque, ConditionImpression, ErreurBibliotheque, IdCondition, Resultat,
    ResumeProfil,
};

/// Numéro d'une imprimante dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdImprimante(pub i64);

/// Numéro d'un papier dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdPapier(pub i64);

/// Une imprimante du parc.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Imprimante {
    pub id: IdImprimante,
    pub nom: String,
}

/// Un papier, propre à une imprimante : le même papier sur deux imprimantes
/// fait deux papiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Papier {
    pub id: IdPapier,
    pub imprimante: IdImprimante,
    pub nom: String,
}

/// Une résolution d'impression d'un papier, c'est-à-dire une condition
/// d'impression rangée, avec son profil actif.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reglage {
    pub condition: ConditionImpression,
    pub resolution: String,
    pub profil: Option<ResumeProfil>,
    /// Le profil actif a plus d'un an.
    pub profil_ancien: bool,
}

/// Un papier et ses résolutions, dans l'ordre alphabétique.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BranchePapier {
    pub papier: Papier,
    pub reglages: Vec<Reglage>,
}

/// Une imprimante et ses papiers, dans l'ordre alphabétique.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BrancheImprimante {
    pub imprimante: Imprimante,
    pub papiers: Vec<BranchePapier>,
}

/// Le parc entier : l'arbre imprimante → papier → résolution, et les
/// conditions d'impression encore sans papier (« non classées »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Parc {
    pub imprimantes: Vec<BrancheImprimante>,
    pub non_classees: Vec<ConditionImpression>,
}

/// Nom débarrassé de ses espaces de début et de fin, jamais vide.
fn nom_propre(nom: &str) -> Resultat<String> {
    let nom = nom.trim();
    if nom.is_empty() {
        Err(ErreurBibliotheque::NomVide)
    } else {
        Ok(nom.to_string())
    }
}

/// Refuse un nom déjà présent parmi `existants`, sans tenir compte des
/// majuscules.
fn nom_libre(nom: &str, existants: Vec<String>) -> Resultat<()> {
    let minuscules = nom.to_lowercase();
    match existants
        .into_iter()
        .find(|e| e.to_lowercase() == minuscules)
    {
        Some(porteur) => Err(ErreurBibliotheque::NomDejaPris(porteur)),
        None => Ok(()),
    }
}

/// Clé de tri alphabétique : sans accents ni majuscules, puis le nom exact.
fn cle(nom: &str) -> (String, String) {
    (replier(nom), nom.to_string())
}

impl Bibliotheque {
    /// Ajoute une imprimante au parc ; son nom doit être libre.
    pub fn creer_imprimante(&self, nom: &str) -> Resultat<Imprimante> {
        let nom = nom_propre(nom)?;
        nom_libre(
            &nom,
            self.imprimantes()?.into_iter().map(|i| i.nom).collect(),
        )?;
        self.base
            .execute("INSERT INTO imprimantes (nom) VALUES (?1)", params![nom])?;
        Ok(Imprimante {
            id: IdImprimante(self.base.last_insert_rowid()),
            nom,
        })
    }

    /// Imprimantes du parc, dans l'ordre alphabétique.
    pub fn imprimantes(&self) -> Resultat<Vec<Imprimante>> {
        let mut requete = self.base.prepare("SELECT id, nom FROM imprimantes")?;
        let lignes = requete.query_map([], |l| {
            Ok(Imprimante {
                id: IdImprimante(l.get(0)?),
                nom: l.get(1)?,
            })
        })?;
        let mut imprimantes: Vec<Imprimante> = lignes.collect::<Result<_, _>>()?;
        imprimantes.sort_by_cached_key(|i| cle(&i.nom));
        Ok(imprimantes)
    }

    /// Ajoute un papier à une imprimante ; son nom doit être libre sur cette
    /// imprimante.
    pub fn creer_papier(&self, imprimante: IdImprimante, nom: &str) -> Resultat<Papier> {
        let nom = nom_propre(nom)?;
        let existe: Option<i64> = self
            .base
            .query_row(
                "SELECT id FROM imprimantes WHERE id = ?1",
                params![imprimante.0],
                |l| l.get(0),
            )
            .optional()?;
        if existe.is_none() {
            return Err(ErreurBibliotheque::ImprimanteInconnue(imprimante));
        }
        let mut requete = self
            .base
            .prepare("SELECT nom FROM papiers WHERE imprimante = ?1")?;
        let existants = requete
            .query_map(params![imprimante.0], |l| l.get(0))?
            .collect::<Result<_, _>>()?;
        nom_libre(&nom, existants)?;
        self.base.execute(
            "INSERT INTO papiers (imprimante, nom) VALUES (?1, ?2)",
            params![imprimante.0, nom],
        )?;
        Ok(Papier {
            id: IdPapier(self.base.last_insert_rowid()),
            imprimante,
            nom,
        })
    }

    /// Ajoute une résolution d'impression à un papier : crée la condition
    /// d'impression « imprimante · papier · résolution ». La résolution doit
    /// être libre sur ce papier.
    pub fn creer_reglage(
        &self,
        papier: IdPapier,
        resolution: &str,
    ) -> Resultat<ConditionImpression> {
        let resolution = nom_propre(resolution)?;
        let noms: Option<(String, String)> = self
            .base
            .query_row(
                "SELECT i.nom, p.nom FROM papiers p JOIN imprimantes i ON i.id = p.imprimante
                 WHERE p.id = ?1",
                params![papier.0],
                |l| Ok((l.get(0)?, l.get(1)?)),
            )
            .optional()?;
        let (imprimante, nom_papier) = noms.ok_or(ErreurBibliotheque::PapierInconnu(papier))?;
        let mut requete = self
            .base
            .prepare("SELECT resolution FROM conditions WHERE papier = ?1")?;
        let existantes = requete
            .query_map(params![papier.0], |l| l.get(0))?
            .collect::<Result<_, _>>()?;
        nom_libre(&resolution, existantes)?;
        let nom = self.nom_libre(&format!("{imprimante} · {nom_papier} · {resolution}"), None)?;
        self.base.execute(
            "INSERT INTO conditions (nom, papier, resolution) VALUES (?1, ?2, ?3)",
            params![nom, papier.0, resolution],
        )?;
        Ok(ConditionImpression {
            id: IdCondition(self.base.last_insert_rowid()),
            nom,
        })
    }

    /// Le parc, réduit par une recherche sans accents ni majuscules. Une
    /// recherche vide rend tout. Sinon, une imprimante trouvée garde tous ses
    /// papiers, un papier trouvé toutes ses résolutions, une résolution
    /// trouvée son papier et son imprimante ; une condition non classée est
    /// gardée si son nom correspond. `maintenant` sert à signaler un profil
    /// actif de plus d'un an.
    pub fn parc(&self, recherche: &str, maintenant: SystemTime) -> Resultat<Parc> {
        let cherche = replier(recherche.trim());
        let trouve = |nom: &str| cherche.is_empty() || replier(nom).contains(&cherche);

        let mut requete = self.base.prepare(
            "SELECT i.id, i.nom, p.id, p.nom, c.id, c.nom, c.resolution
             FROM imprimantes i
             LEFT JOIN papiers p ON p.imprimante = i.id
             LEFT JOIN conditions c ON c.papier = p.id",
        )?;
        let lignes = requete.query_map([], |l| {
            Ok((
                Imprimante {
                    id: IdImprimante(l.get(0)?),
                    nom: l.get(1)?,
                },
                l.get::<_, Option<i64>>(2)?
                    .zip(l.get::<_, Option<String>>(3)?),
                l.get::<_, Option<i64>>(4)?,
                l.get::<_, Option<String>>(5)?,
                l.get::<_, Option<String>>(6)?,
            ))
        })?;

        let mut imprimantes: Vec<BrancheImprimante> = Vec::new();
        for ligne in lignes {
            let (imprimante, papier, condition, nom_condition, resolution) = ligne?;
            let position = match imprimantes
                .iter()
                .position(|b| b.imprimante.id == imprimante.id)
            {
                Some(position) => position,
                None => {
                    imprimantes.push(BrancheImprimante {
                        imprimante: imprimante.clone(),
                        papiers: Vec::new(),
                    });
                    imprimantes.len() - 1
                }
            };
            let Some((id_papier, nom_papier)) = papier else {
                continue;
            };
            let papiers = &mut imprimantes[position].papiers;
            let position = match papiers.iter().position(|b| b.papier.id.0 == id_papier) {
                Some(position) => position,
                None => {
                    papiers.push(BranchePapier {
                        papier: Papier {
                            id: IdPapier(id_papier),
                            imprimante: imprimante.id,
                            nom: nom_papier,
                        },
                        reglages: Vec::new(),
                    });
                    papiers.len() - 1
                }
            };
            if let (Some(id), Some(nom), Some(resolution)) = (condition, nom_condition, resolution)
            {
                let condition = ConditionImpression {
                    id: IdCondition(id),
                    nom,
                };
                let (profil, profil_ancien) = self.profil_du_reglage(condition.id, maintenant)?;
                papiers[position].reglages.push(Reglage {
                    condition,
                    resolution,
                    profil,
                    profil_ancien,
                });
            }
        }

        // Filtrage du haut vers le bas, puis tri à chaque niveau.
        let mut gardees = Vec::new();
        for mut branche in imprimantes {
            if !trouve(&branche.imprimante.nom) {
                branche.papiers.retain_mut(|p| {
                    if !trouve(&p.papier.nom) {
                        p.reglages.retain(|r| trouve(&r.resolution));
                        !p.reglages.is_empty()
                    } else {
                        true
                    }
                });
                if branche.papiers.is_empty() {
                    continue;
                }
            }
            for papier in &mut branche.papiers {
                papier.reglages.sort_by_cached_key(|r| cle(&r.resolution));
            }
            branche.papiers.sort_by_cached_key(|p| cle(&p.papier.nom));
            gardees.push(branche);
        }
        gardees.sort_by_cached_key(|b| cle(&b.imprimante.nom));

        let mut requete = self
            .base
            .prepare("SELECT id, nom FROM conditions WHERE papier IS NULL ORDER BY id")?;
        let non_classees = requete
            .query_map([], |l| {
                Ok(ConditionImpression {
                    id: IdCondition(l.get(0)?),
                    nom: l.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|c| trouve(&c.nom))
            .collect();

        Ok(Parc {
            imprimantes: gardees,
            non_classees,
        })
    }

    /// Profil actif d'une condition rangée et s'il a plus d'un an.
    fn profil_du_reglage(
        &self,
        _condition: IdCondition,
        _maintenant: SystemTime,
    ) -> Resultat<(Option<ResumeProfil>, bool)> {
        Ok((None, false))
    }
}
