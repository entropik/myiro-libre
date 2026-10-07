//! Bibliothèque locale de myiro-libre (ADR 0001) : conditions d'impression,
//! instruments et mesures, dans une base unique sur le poste.
//!
//! Indépendante de Tauri, de Windows et des ponts (ADR 0004). Les mesures y
//! sont conservées dans leur format versionné (`docs/formats/mesure.md`), sans
//! rien recalculer ni compléter.

use std::fmt;
use std::path::Path;

use pont_protocole::{ecrire_mesure, lire_mesure, Geometrie, Mesure};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

/// Nom du fichier de la base, dans le dossier de la bibliothèque.
pub const FICHIER_BASE: &str = "bibliotheque.sqlite";

/// Version de l'organisation de la base ; une base plus récente est refusée.
const VERSION_BASE: i32 = 1;

/// Ce qui peut empêcher une opération sur la bibliothèque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurBibliotheque {
    /// Le nom d'une condition d'impression est vide.
    NomVide,
    /// Une autre condition d'impression porte déjà ce nom.
    NomDejaPris(String),
    ConditionInconnue(IdCondition),
    MesureInconnue(IdMesure),
    /// La base a été écrite par une version plus récente de myiro-libre.
    VersionBase(i32),
    /// Une mesure conservée ne se relit plus (format, contenu).
    MesureIllisible {
        id: IdMesure,
        detail: String,
    },
    /// Erreur du fichier ou de la base elle-même.
    Base(String),
}

impl fmt::Display for ErreurBibliotheque {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NomVide => f.write_str("nom de condition d'impression vide"),
            Self::NomDejaPris(nom) => {
                write!(f, "une condition d'impression s'appelle déjà « {nom} »")
            }
            Self::ConditionInconnue(id) => write!(f, "condition d'impression n° {} inconnue", id.0),
            Self::MesureInconnue(id) => write!(f, "mesure n° {} inconnue", id.0),
            Self::VersionBase(v) => write!(
                f,
                "bibliothèque écrite par une version plus récente (organisation {v}, attendu {VERSION_BASE} au plus)"
            ),
            Self::MesureIllisible { id, detail } => {
                write!(f, "mesure n° {} illisible : {detail}", id.0)
            }
            Self::Base(detail) => write!(f, "bibliothèque : {detail}"),
        }
    }
}

impl std::error::Error for ErreurBibliotheque {}

impl From<rusqlite::Error> for ErreurBibliotheque {
    fn from(e: rusqlite::Error) -> Self {
        ErreurBibliotheque::Base(e.to_string())
    }
}

pub type Resultat<T> = Result<T, ErreurBibliotheque>;

/// Numéro d'une condition d'impression dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdCondition(pub i64);

/// Numéro d'une mesure dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdMesure(pub i64);

/// Condition d'impression (GLOSSARY) : pour l'instant, son nom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConditionImpression {
    pub id: IdCondition,
    pub nom: String,
}

/// Instrument (GLOSSARY) : un spectrophotomètre précis, reconnu à son modèle
/// et à son numéro de série tels que le pont les a posés dans la provenance.
/// Le micrologiciel, qui peut changer, reste dans la provenance de chaque mesure.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Instrument {
    pub modele: String,
    pub numero_serie: u32,
}

/// Une mesure telle que la bibliothèque la conserve.
#[derive(Clone, Debug, PartialEq)]
pub struct MesureEnregistree {
    pub id: IdMesure,
    pub condition: IdCondition,
    pub instrument: Instrument,
    /// La mesure du pont, relue à l'identique.
    pub mesure: Mesure,
}

/// Ce que la colonne de gauche montre d'une mesure, sans la relire en entier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeMesure {
    pub id: IdMesure,
    /// Fin de la mesure, telle que le pont l'a datée.
    pub horodatage: String,
    pub instrument: Instrument,
    pub geometrie: Geometrie,
    /// Nombre de plages lues.
    pub plages: usize,
}

/// Une condition d'impression et ses mesures, les plus récentes d'abord.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Branche {
    pub condition: ConditionImpression,
    pub mesures: Vec<ResumeMesure>,
}

/// La bibliothèque ouverte : une base dans un dossier du poste.
pub struct Bibliotheque {
    base: Connection,
}

impl Bibliotheque {
    /// Ouvre la bibliothèque du dossier, en la créant si besoin (dossier
    /// compris).
    pub fn ouvrir(dossier: &Path) -> Resultat<Self> {
        std::fs::create_dir_all(dossier)
            .map_err(|e| ErreurBibliotheque::Base(format!("{} : {e}", dossier.display())))?;
        let base = Connection::open(dossier.join(FICHIER_BASE))?;
        base.pragma_update(None, "foreign_keys", true)?;
        let version: i32 = base.pragma_query_value(None, "user_version", |l| l.get(0))?;
        if version > VERSION_BASE {
            return Err(ErreurBibliotheque::VersionBase(version));
        }
        if version < VERSION_BASE {
            base.execute_batch(SCHEMA)?;
            base.pragma_update(None, "user_version", VERSION_BASE)?;
        }
        Ok(Bibliotheque { base })
    }

    /// Crée une condition d'impression ; le nom est débarrassé de ses espaces
    /// de début et de fin, et doit être libre.
    pub fn creer_condition(&self, nom: &str) -> Resultat<ConditionImpression> {
        let nom = self.nom_libre(nom, None)?;
        self.base
            .execute("INSERT INTO conditions (nom) VALUES (?1)", params![nom])?;
        Ok(ConditionImpression {
            id: IdCondition(self.base.last_insert_rowid()),
            nom,
        })
    }

    /// Renomme une condition d'impression ; ses mesures la suivent.
    pub fn renommer_condition(&self, id: IdCondition, nom: &str) -> Resultat<()> {
        self.verifier_condition(id)?;
        let nom = self.nom_libre(nom, Some(id))?;
        self.base.execute(
            "UPDATE conditions SET nom = ?1 WHERE id = ?2",
            params![nom, id.0],
        )?;
        Ok(())
    }

    fn verifier_condition(&self, id: IdCondition) -> Resultat<()> {
        let existe: Option<i64> = self
            .base
            .query_row(
                "SELECT id FROM conditions WHERE id = ?1",
                params![id.0],
                |l| l.get(0),
            )
            .optional()?;
        existe
            .map(|_| ())
            .ok_or(ErreurBibliotheque::ConditionInconnue(id))
    }

    /// Conditions d'impression, dans l'ordre de création.
    pub fn conditions(&self) -> Resultat<Vec<ConditionImpression>> {
        let mut requete = self
            .base
            .prepare("SELECT id, nom FROM conditions ORDER BY id")?;
        let lignes = requete.query_map([], |l| {
            Ok(ConditionImpression {
                id: IdCondition(l.get(0)?),
                nom: l.get(1)?,
            })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    /// Enregistre une mesure dans une condition d'impression. L'instrument est
    /// celui de sa provenance ; la mesure est conservée dans son format
    /// versionné, sans rien recalculer ni compléter.
    pub fn enregistrer_mesure(
        &self,
        condition: IdCondition,
        mesure: &Mesure,
    ) -> Resultat<IdMesure> {
        self.verifier_condition(condition)?;
        let provenance = mesure.provenance();
        let instrument = &provenance.instrument;
        let transaction = self.base.unchecked_transaction()?;
        transaction.execute(
            "INSERT OR IGNORE INTO instruments (modele, numero_serie) VALUES (?1, ?2)",
            params![instrument.modele, instrument.numero_serie],
        )?;
        let id_instrument: i64 = transaction.query_row(
            "SELECT id FROM instruments WHERE modele = ?1 AND numero_serie = ?2",
            params![instrument.modele, instrument.numero_serie],
            |l| l.get(0),
        )?;
        transaction.execute(
            "INSERT INTO mesures (condition, instrument, horodatage, geometrie, plages, contenu)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                condition.0,
                id_instrument,
                provenance.horodatage.texte(),
                serde_json::to_string(&provenance.geometrie)
                    .expect("une géométrie se sérialise toujours"),
                mesure.plages().len(),
                ecrire_mesure(mesure)
            ],
        )?;
        let id = IdMesure(transaction.last_insert_rowid());
        transaction.commit()?;
        Ok(id)
    }

    /// Relit une mesure enregistrée.
    pub fn mesure(&self, id: IdMesure) -> Resultat<MesureEnregistree> {
        let ligne = self
            .base
            .query_row(
                "SELECT m.condition, i.modele, i.numero_serie, m.contenu
                 FROM mesures m JOIN instruments i ON i.id = m.instrument
                 WHERE m.id = ?1",
                params![id.0],
                |l| {
                    Ok((
                        IdCondition(l.get(0)?),
                        Instrument {
                            modele: l.get(1)?,
                            numero_serie: l.get(2)?,
                        },
                        l.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let (condition, instrument, contenu) =
            ligne.ok_or(ErreurBibliotheque::MesureInconnue(id))?;
        let mesure = lire_mesure(&contenu).map_err(|e| ErreurBibliotheque::MesureIllisible {
            id,
            detail: e.to_string(),
        })?;
        Ok(MesureEnregistree {
            id,
            condition,
            instrument,
            mesure,
        })
    }

    /// Arborescence condition d'impression → mesures, réduite par une
    /// recherche. Une recherche vide rend tout, conditions vides comprises.
    ///
    /// Chaque mot cherché doit se trouver, sans tenir compte des majuscules ni
    /// des accents, dans le nom de la condition, le modèle ou le numéro de
    /// série de l'instrument, ou la date de la mesure (`2026-10-07`). Une
    /// condition dont le nom suffit garde toutes ses mesures.
    pub fn arborescence(&self, recherche: &str) -> Resultat<Vec<Branche>> {
        let mots: Vec<String> = recherche.split_whitespace().map(replier).collect();
        let trouve = |texte: &str| {
            let texte = replier(texte);
            mots.iter().all(|m| texte.contains(m.as_str()))
        };
        let mut requete = self.base.prepare(
            "SELECT m.id, m.condition, m.horodatage, m.geometrie, m.plages, i.modele, i.numero_serie
             FROM mesures m JOIN instruments i ON i.id = m.instrument
             ORDER BY m.horodatage DESC, m.id DESC",
        )?;
        let lignes = requete.query_map([], |l| {
            Ok((
                IdMesure(l.get(0)?),
                IdCondition(l.get(1)?),
                l.get::<_, String>(2)?,
                l.get::<_, String>(3)?,
                l.get::<_, usize>(4)?,
                Instrument {
                    modele: l.get(5)?,
                    numero_serie: l.get(6)?,
                },
            ))
        })?;
        let mut mesures = Vec::new();
        for ligne in lignes {
            let (id, condition, horodatage, geometrie, plages, instrument) = ligne?;
            let geometrie = serde_json::from_str(&geometrie).map_err(|e| {
                ErreurBibliotheque::MesureIllisible {
                    id,
                    detail: e.to_string(),
                }
            })?;
            let resume = ResumeMesure {
                id,
                horodatage,
                instrument,
                geometrie,
                plages,
            };
            mesures.push((condition, resume));
        }

        let mut branches = Vec::new();
        for condition in self.conditions()? {
            let siennes = mesures
                .iter()
                .filter(|(c, _)| *c == condition.id)
                .map(|(_, m)| m);
            let retenues: Vec<ResumeMesure> = if trouve(&condition.nom) {
                siennes.cloned().collect()
            } else {
                siennes
                    .filter(|m| {
                        trouve(&format!(
                            "{} {} {} {}",
                            condition.nom,
                            m.instrument.modele,
                            m.instrument.numero_serie,
                            m.horodatage
                        ))
                    })
                    .cloned()
                    .collect()
            };
            if mots.is_empty() || trouve(&condition.nom) || !retenues.is_empty() {
                branches.push(Branche {
                    condition,
                    mesures: retenues,
                });
            }
        }
        Ok(branches)
    }

    /// Instruments qui ont produit au moins une mesure de la bibliothèque.
    pub fn instruments(&self) -> Resultat<Vec<Instrument>> {
        let mut requete = self.base.prepare(
            "SELECT modele, numero_serie FROM instruments ORDER BY modele, numero_serie",
        )?;
        let lignes = requete.query_map([], |l| {
            Ok(Instrument {
                modele: l.get(0)?,
                numero_serie: l.get(1)?,
            })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    /// Nom nettoyé, s'il n'est ni vide ni porté par une autre condition.
    fn nom_libre(&self, nom: &str, sauf: Option<IdCondition>) -> Resultat<String> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(ErreurBibliotheque::NomVide);
        }
        let porteur: Option<i64> = self
            .base
            .query_row(
                "SELECT id FROM conditions WHERE nom = ?1",
                params![nom],
                |l| l.get(0),
            )
            .optional()?;
        match porteur {
            Some(id) if Some(IdCondition(id)) != sauf => {
                Err(ErreurBibliotheque::NomDejaPris(nom.to_string()))
            }
            _ => Ok(nom.to_string()),
        }
    }
}

/// Texte ramené à une forme de comparaison : minuscules, sans accents, avec
/// l'apostrophe droite.
fn replier(texte: &str) -> String {
    let mut plie = String::with_capacity(texte.len());
    for c in texte.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => plie.push('a'),
            'ç' => plie.push('c'),
            'é' | 'è' | 'ê' | 'ë' => plie.push('e'),
            'î' | 'ï' | 'í' | 'ì' => plie.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => plie.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => plie.push('u'),
            'ÿ' => plie.push('y'),
            'ñ' => plie.push('n'),
            'œ' => plie.push_str("oe"),
            'æ' => plie.push_str("ae"),
            '’' | '‘' => plie.push('\''),
            autre => plie.push(autre),
        }
    }
    plie
}

const SCHEMA: &str = "
CREATE TABLE conditions (
    id INTEGER PRIMARY KEY,
    nom TEXT NOT NULL UNIQUE
);
CREATE TABLE instruments (
    id INTEGER PRIMARY KEY,
    modele TEXT NOT NULL,
    numero_serie INTEGER NOT NULL,
    UNIQUE (modele, numero_serie)
);
CREATE TABLE mesures (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    instrument INTEGER NOT NULL REFERENCES instruments (id),
    horodatage TEXT NOT NULL,
    geometrie TEXT NOT NULL,
    plages INTEGER NOT NULL,
    contenu TEXT NOT NULL
);
CREATE INDEX mesures_par_condition ON mesures (condition);
";
