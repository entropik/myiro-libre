//! Bibliothèque locale de myiro-libre (ADR 0001) : conditions d'impression,
//! instruments et mesures, dans une base unique sur le poste.
//!
//! Indépendante de Tauri, de Windows et des ponts (ADR 0004). Les mesures y
//! sont conservées dans leur format versionné (`docs/formats/mesure.md`), sans
//! rien recalculer ni compléter.

use std::fmt;
use std::path::Path;

use cgats::MesureImportee;
use pont_protocole::{ecrire_mesure, lire_mesure, Geometrie, Info, Mesure};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior, MAIN_DB};
use serde::Serialize;

/// Nom du fichier de la base, dans le dossier de la bibliothèque.
pub const FICHIER_BASE: &str = "bibliotheque.sqlite";

/// Début du nom de la copie de la bibliothèque faite juste avant une
/// restauration, dans son dossier, suivi de la date et de l'heure UTC
/// (`bibliotheque.sqlite.avant-restauration-2026-10-09-133405Z`) : remise en
/// place si la restauration échoue, et gardée ensuite.
pub const FICHIER_SECOURS: &str = "bibliotheque.sqlite.avant-restauration";

/// Date et heure UTC d'un instant, pour un nom de fichier :
/// `2026-10-09-133405Z`. Calendrier grégorien.
fn date_heure_utc(instant: std::time::SystemTime) -> String {
    let secondes = instant
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (jours, reste) = (secondes / 86_400, secondes % 86_400);
    // Jours depuis 1970 vers année, mois, jour (algorithme des ères de 400 ans).
    let z = jours as i64 + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z - ere * 146_097;
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let m = (5 * jour_annee + 2) / 153;
    let jour = jour_annee - (153 * m + 2) / 5 + 1;
    let mois = if m < 10 { m + 3 } else { m - 9 };
    let annee = annee_ere + ere * 400 + i64::from(mois <= 2);
    format!(
        "{annee:04}-{mois:02}-{jour:02}-{:02}{:02}{:02}Z",
        reste / 3600,
        reste % 3600 / 60,
        reste % 60
    )
}

/// Le chemin s'il est libre, sinon le même suivi de `-2`, `-3`…
fn fichier_libre(chemin: &Path) -> std::path::PathBuf {
    if !chemin.exists() {
        return chemin.to_path_buf();
    }
    (2..)
        .map(|n| {
            let mut nom = chemin.as_os_str().to_owned();
            nom.push(format!("-{n}"));
            std::path::PathBuf::from(nom)
        })
        .find(|c| !c.exists())
        .expect("un nom libre finit par se trouver")
}

/// Version de l'organisation de la base ; une base plus récente est refusée.
const VERSION_BASE: i32 = 3;

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
    /// Le fichier à importer n'est pas un CGATS lisible ; rien n'est importé.
    ImportIllisible(String),
    /// Une mesure conservée ne se relit plus (format, contenu).
    MesureIllisible {
        id: IdMesure,
        detail: String,
    },
    /// Le fichier à restaurer n'est pas une sauvegarde utilisable ; la
    /// bibliothèque n'a pas été touchée.
    SauvegardeInvalide(String),
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
            Self::ImportIllisible(detail) => {
                write!(f, "fichier CGATS illisible, rien n'a été importé : {detail}")
            }
            Self::SauvegardeInvalide(detail) => {
                write!(f, "sauvegarde inutilisable, rien n'a été restauré : {detail}")
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
    /// Nom donné par l'utilisateur ; aucun pour une mesure enregistrée sans
    /// nom (bande, bibliothèque d'avant les noms).
    pub nom: Option<String>,
}

/// Ce que la colonne de gauche montre d'une mesure, sans la relire en entier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeMesure {
    pub id: IdMesure,
    /// Nom donné par l'utilisateur, s'il y en a un.
    pub nom: Option<String>,
    /// Fin de la mesure, telle que le pont l'a datée.
    pub horodatage: String,
    pub instrument: Instrument,
    pub geometrie: Geometrie,
    /// Nombre de plages lues.
    pub plages: usize,
}

/// Une mesure importée d'un fichier CGATS, telle que la bibliothèque la
/// conserve : le texte du fichier, relu à chaque fois. Ce n'est pas une
/// mesure attestée par un pont (GLOSSARY : Mesure importée).
#[derive(Clone, Debug, PartialEq)]
pub struct MesureImporteeEnregistree {
    pub id: IdMesure,
    pub condition: IdCondition,
    /// Nom du fichier d'origine, sans son dossier.
    pub fichier: String,
    pub mesure: MesureImportee,
}

/// Ce que la colonne de gauche montre d'une mesure importée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeImportee {
    pub id: IdMesure,
    /// Nom du fichier d'origine, sans son dossier.
    pub fichier: String,
    /// Date et instrument tels que le fichier les déclare.
    pub date: Info<String>,
    pub instrument: Info<String>,
    pub plages: usize,
}

/// Une condition d'impression et ses mesures, les plus récentes d'abord ;
/// les mesures importées à part, les dernières importées d'abord.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Branche {
    pub condition: ConditionImpression,
    pub mesures: Vec<ResumeMesure>,
    pub importees: Vec<ResumeImportee>,
}

/// La bibliothèque ouverte : une base dans un dossier du poste.
pub struct Bibliotheque {
    base: Connection,
    /// Dossier de la bibliothèque, où va la copie de secours d'une restauration.
    dossier: std::path::PathBuf,
}

impl Bibliotheque {
    /// Ouvre la bibliothèque du dossier, en la créant si besoin (dossier
    /// compris).
    pub fn ouvrir(dossier: &Path) -> Resultat<Self> {
        std::fs::create_dir_all(dossier)
            .map_err(|e| ErreurBibliotheque::Base(format!("{} : {e}", dossier.display())))?;
        let mut base = Connection::open(dossier.join(FICHIER_BASE))?;
        base.pragma_update(None, "foreign_keys", true)?;
        migrer(&mut base)?;
        Ok(Bibliotheque {
            base,
            dossier: dossier.to_path_buf(),
        })
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
        self.enregistrer(condition, mesure, None)
    }

    /// Enregistre une mesure avec son nom, débarrassé de ses espaces de début
    /// et de fin ; un nom vide est refusé et rien n'est enregistré. Deux
    /// mesures peuvent porter le même nom.
    pub fn enregistrer_mesure_nommee(
        &self,
        condition: IdCondition,
        mesure: &Mesure,
        nom: &str,
    ) -> Resultat<IdMesure> {
        let nom = nom_de_mesure(nom)?;
        self.enregistrer(condition, mesure, Some(nom))
    }

    /// Renomme une mesure du pont ; la mesure elle-même n'est pas touchée.
    /// Une mesure importée n'a pas de nom de mesure (elle est désignée par
    /// son fichier, ADR 0001) : elle est refusée comme inconnue.
    pub fn renommer_mesure(&self, id: IdMesure, nom: &str) -> Resultat<()> {
        let nom = nom_de_mesure(nom)?;
        let modifiees = self.base.execute(
            "UPDATE mesures SET nom = ?1 WHERE id = ?2 AND origine = 'pont'",
            params![nom, id.0],
        )?;
        if modifiees == 0 {
            return Err(ErreurBibliotheque::MesureInconnue(id));
        }
        Ok(())
    }

    fn enregistrer(
        &self,
        condition: IdCondition,
        mesure: &Mesure,
        nom: Option<&str>,
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
            "INSERT INTO mesures (condition, instrument, horodatage, geometrie, plages, contenu, nom)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                condition.0,
                id_instrument,
                provenance.horodatage.texte(),
                serde_json::to_string(&provenance.geometrie)
                    .expect("une géométrie se sérialise toujours"),
                mesure.plages().len(),
                ecrire_mesure(mesure),
                nom
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
                "SELECT m.condition, i.modele, i.numero_serie, m.contenu, m.nom
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
                        l.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        let (condition, instrument, contenu, nom) =
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
            nom,
        })
    }

    /// Arborescence condition d'impression → mesures, réduite par une
    /// recherche. Une recherche vide rend tout, conditions vides comprises.
    ///
    /// Chaque mot cherché doit se trouver, sans tenir compte des majuscules ni
    /// des accents, dans le nom de la condition, le nom de la mesure, le modèle
    /// ou le numéro de série de l'instrument, ou la date de la mesure
    /// (`2026-10-07`). Une
    /// condition dont le nom suffit garde toutes ses mesures.
    pub fn arborescence(&self, recherche: &str) -> Resultat<Vec<Branche>> {
        let mots: Vec<String> = recherche.split_whitespace().map(replier).collect();
        let trouve = |texte: &str| {
            let texte = replier(texte);
            mots.iter().all(|m| texte.contains(m.as_str()))
        };
        let mut requete = self.base.prepare(
            "SELECT m.id, m.condition, m.horodatage, m.geometrie, m.plages, i.modele, i.numero_serie,
                    m.nom
             FROM mesures m JOIN instruments i ON i.id = m.instrument",
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
                l.get::<_, Option<String>>(7)?,
            ))
        })?;
        let mut mesures = Vec::new();
        for ligne in lignes {
            let (id, condition, horodatage, geometrie, plages, instrument, nom) = ligne?;
            let geometrie = serde_json::from_str(&geometrie).map_err(|e| {
                ErreurBibliotheque::MesureIllisible {
                    id,
                    detail: e.to_string(),
                }
            })?;
            let resume = ResumeMesure {
                id,
                nom,
                horodatage,
                instrument,
                geometrie,
                plages,
            };
            mesures.push((condition, resume));
        }
        // Les plus récentes d'abord, sur l'instant réel : deux fuseaux
        // différents ne se comparent pas sur le texte de la date.
        mesures.sort_by_key(|(_, m)| std::cmp::Reverse((instant(&m.horodatage), m.id)));

        let importees = self.resumes_importees()?;

        let mut branches = Vec::new();
        for condition in self.conditions()? {
            let tout = trouve(&condition.nom);
            let retenues: Vec<ResumeMesure> = mesures
                .iter()
                .filter(|(c, _)| *c == condition.id)
                .map(|(_, m)| m)
                .filter(|m| {
                    tout || trouve(&format!(
                        "{} {} {} {} {}",
                        condition.nom,
                        m.nom.as_deref().unwrap_or_default(),
                        m.instrument.modele,
                        m.instrument.numero_serie,
                        m.horodatage
                    ))
                })
                .cloned()
                .collect();
            let declare = |info: &Info<String>| info.valeur().cloned().unwrap_or_default();
            let importees_retenues: Vec<ResumeImportee> = importees
                .iter()
                .filter(|(c, _)| *c == condition.id)
                .map(|(_, m)| m)
                .filter(|m| {
                    tout || trouve(&format!(
                        "{} {} {} {}",
                        condition.nom,
                        m.fichier,
                        declare(&m.instrument),
                        declare(&m.date)
                    ))
                })
                .cloned()
                .collect();
            if mots.is_empty() || tout || !retenues.is_empty() || !importees_retenues.is_empty() {
                branches.push(Branche {
                    condition,
                    mesures: retenues,
                    importees: importees_retenues,
                });
            }
        }
        Ok(branches)
    }

    /// Mesures importées, les dernières importées d'abord, avec leur condition.
    fn resumes_importees(&self) -> Resultat<Vec<(IdCondition, ResumeImportee)>> {
        let mut requete = self.base.prepare(
            "SELECT id, condition, fichier, contenu FROM mesures
             WHERE origine = 'importee' ORDER BY id DESC",
        )?;
        let lignes = requete.query_map([], |l| {
            Ok((
                IdMesure(l.get(0)?),
                IdCondition(l.get(1)?),
                l.get::<_, String>(2)?,
                l.get::<_, String>(3)?,
            ))
        })?;
        let mut resumes = Vec::new();
        for ligne in lignes {
            let (id, condition, fichier, contenu) = ligne?;
            let mesure = lire_importee(id, &contenu)?;
            resumes.push((
                condition,
                ResumeImportee {
                    id,
                    fichier,
                    date: mesure.date,
                    instrument: mesure.instrument,
                    plages: mesure.plages.len(),
                },
            ));
        }
        Ok(resumes)
    }

    /// Range dans une condition d'impression la mesure lue dans un fichier
    /// CGATS. Le texte du fichier est conservé tel quel, et relu à chaque
    /// fois : ce qu'il ne dit pas reste inconnu. Seul le nom du fichier est
    /// gardé, pas son dossier. Un fichier illisible n'importe rien.
    pub fn importer_mesure(
        &self,
        condition: IdCondition,
        fichier: &str,
        texte: &str,
    ) -> Resultat<IdMesure> {
        self.verifier_condition(condition)?;
        let mesure =
            cgats::lire(texte).map_err(|e| ErreurBibliotheque::ImportIllisible(e.to_string()))?;
        let nom = fichier.rsplit(['/', '\\']).next().unwrap_or(fichier);
        self.base.execute(
            "INSERT INTO mesures (condition, origine, plages, fichier, contenu)
             VALUES (?1, 'importee', ?2, ?3, ?4)",
            params![condition.0, mesure.plages.len(), nom, texte],
        )?;
        Ok(IdMesure(self.base.last_insert_rowid()))
    }

    /// Relit une mesure importée.
    pub fn mesure_importee(&self, id: IdMesure) -> Resultat<MesureImporteeEnregistree> {
        let ligne = self
            .base
            .query_row(
                "SELECT condition, fichier, contenu FROM mesures
                 WHERE id = ?1 AND origine = 'importee'",
                params![id.0],
                |l| {
                    Ok((
                        IdCondition(l.get(0)?),
                        l.get::<_, String>(1)?,
                        l.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let (condition, fichier, contenu) = ligne.ok_or(ErreurBibliotheque::MesureInconnue(id))?;
        Ok(MesureImporteeEnregistree {
            id,
            condition,
            fichier,
            mesure: lire_importee(id, &contenu)?,
        })
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

    /// Sauvegarde complète de la bibliothèque dans un seul fichier : une base
    /// SQLite, lisible par tout outil SQLite (ADR 0001). Un fichier déjà
    /// présent est remplacé ; il ne l'est qu'une fois la copie terminée.
    pub fn sauvegarder(&self, fichier: &Path) -> Resultat<()> {
        let mut nom = fichier.as_os_str().to_owned();
        nom.push(".partiel");
        let partiel = std::path::PathBuf::from(nom);
        let erreur_fichier =
            |e: std::io::Error| ErreurBibliotheque::Base(format!("{} : {e}", fichier.display()));
        match std::fs::remove_file(&partiel) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(erreur_fichier(e)),
            _ => {}
        }
        if let Err(e) = self.base.backup(MAIN_DB, &partiel, None) {
            let _ = std::fs::remove_file(&partiel);
            return Err(e.into());
        }
        std::fs::rename(&partiel, fichier).map_err(erreur_fichier)
    }

    /// Remplace tout le contenu de la bibliothèque par une sauvegarde faite
    /// par [`Bibliotheque::sauvegarder`]. La sauvegarde est d'abord examinée
    /// en lecture seule : organisation connue, tables présentes, liens
    /// cohérents, chaque mesure relisible. Au moindre défaut, rien n'est
    /// touché ([`ErreurBibliotheque::SauvegardeInvalide`]).
    pub fn restaurer(&mut self, fichier: &Path) -> Resultat<()> {
        examiner_sauvegarde(fichier).map_err(ErreurBibliotheque::SauvegardeInvalide)?;
        // Copie de secours d'abord : si la suite échoue (mise à niveau
        // impossible, disque plein…), la bibliothèque d'avant est remise.
        // Une copie par restauration, datée : la suivante n'écrase pas celle-ci.
        let secours = fichier_libre(&self.dossier.join(format!(
            "{FICHIER_SECOURS}-{}",
            date_heure_utc(std::time::SystemTime::now())
        )));
        self.sauvegarder(&secours)?;
        match self.remplacer_par(fichier) {
            Ok(()) => Ok(()),
            Err(echec) => match self.remplacer_par(&secours) {
                Ok(()) => Err(echec),
                // Les deux erreurs sont gardées : la copie reste sur le disque.
                Err(remise) => Err(ErreurBibliotheque::Base(format!(
                    "restauration impossible ({echec}), et remise de la copie de secours {} impossible ({remise})",
                    secours.display()
                ))),
            },
        }
    }

    /// Remplace tout le contenu de la base par celui du fichier, puis le met
    /// à niveau.
    fn remplacer_par(&mut self, fichier: &Path) -> Resultat<()> {
        self.base
            .restore(MAIN_DB, fichier, None::<fn(rusqlite::backup::Progress)>)?;
        self.base.pragma_update(None, "foreign_keys", true)?;
        migrer(&mut self.base)
    }

    /// Nom nettoyé, s'il n'est ni vide ni porté par une autre condition.
    fn nom_libre(&self, nom: &str, sauf: Option<IdCondition>) -> Resultat<String> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(ErreurBibliotheque::NomVide);
        }
        // Comparaison sans les majuscules : « Offset » et « offset » sont la
        // même condition pour l'utilisateur.
        let minuscules = nom.to_lowercase();
        match self
            .conditions()?
            .into_iter()
            .find(|c| c.nom.to_lowercase() == minuscules && Some(c.id) != sauf)
        {
            Some(porteuse) => Err(ErreurBibliotheque::NomDejaPris(porteuse.nom)),
            None => Ok(nom.to_string()),
        }
    }
}

/// Vérifie, en lecture seule, qu'un fichier est une sauvegarde restaurable.
fn examiner_sauvegarde(fichier: &Path) -> Result<(), String> {
    if !fichier.is_file() {
        return Err(format!("{} introuvable", fichier.display()));
    }
    let base = Connection::open_with_flags(fichier, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let version: i32 = base
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .map_err(|e| format!("pas une base SQLite ({e})"))?;
    if version > VERSION_BASE {
        return Err(format!(
            "écrite par une version plus récente (organisation {version}, attendu {VERSION_BASE} au plus)"
        ));
    }
    let tables: i64 = base
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table'
             AND name IN ('conditions', 'instruments', 'mesures')",
            [],
            |l| l.get(0),
        )
        .map_err(|e| e.to_string())?;
    if version < 1 || tables != 3 {
        return Err("pas une sauvegarde de bibliothèque myiro-libre".into());
    }
    let liens_rompus = base
        .prepare("PRAGMA foreign_key_check")
        .and_then(|mut r| r.exists([]))
        .map_err(|e| e.to_string())?;
    if liens_rompus {
        return Err("mesures rattachées à une condition ou un instrument absent".into());
    }
    // Avant l'organisation 3, toutes les mesures venaient d'un pont.
    let origine = if version >= 3 { "origine" } else { "'pont'" };
    let mut requete = base
        .prepare(&format!("SELECT id, {origine}, contenu FROM mesures"))
        .map_err(|e| e.to_string())?;
    let mesures = requete
        .query_map([], |l| {
            Ok((
                l.get::<_, i64>(0)?,
                l.get::<_, String>(1)?,
                l.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for mesure in mesures {
        let (id, origine, contenu) = mesure.map_err(|e| e.to_string())?;
        let lue = if origine == "importee" {
            cgats::lire(&contenu).map(|_| ()).map_err(|e| e.to_string())
        } else {
            lire_mesure(&contenu).map(|_| ()).map_err(|e| e.to_string())
        };
        lue.map_err(|e| format!("mesure n° {id} illisible : {e}"))?;
    }
    Ok(())
}

/// Relit le texte CGATS conservé d'une mesure importée.
fn lire_importee(id: IdMesure, contenu: &str) -> Resultat<MesureImportee> {
    cgats::lire(contenu).map_err(|e| ErreurBibliotheque::MesureIllisible {
        id,
        detail: e.to_string(),
    })
}

/// Instant UTC d'un horodatage RFC 3339 déjà vérifié par `pont-protocole` :
/// secondes depuis 1970 et nanosecondes (au-delà de 9 chiffres, la fraction
/// est tronquée).
fn instant(texte: &str) -> (i64, u32) {
    let o = texte.as_bytes();
    let nombre = |debut: usize, fin: usize| -> i64 {
        texte
            .get(debut..fin)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    let (annee, mois, jour) = (nombre(0, 4), nombre(5, 7), nombre(8, 10));
    let (h, m, s) = (nombre(11, 13), nombre(14, 16), nombre(17, 19));
    // Jours depuis le 1er janvier 1970 (calendrier grégorien proleptique).
    let (a, mo) = if mois <= 2 {
        (annee - 1, mois + 9)
    } else {
        (annee, mois - 3)
    };
    let ere = a.div_euclid(400);
    let annee_ere = a - ere * 400;
    let jour_annee = (153 * mo + 2) / 5 + jour - 1;
    let jour_ere = annee_ere * 365 + annee_ere / 4 - annee_ere / 100 + jour_annee;
    let jours = ere * 146_097 + jour_ere - 719_468;

    let mut i = 19;
    let mut nanos = 0u32;
    if o.get(i) == Some(&b'.') {
        i += 1;
        let mut echelle = 100_000_000u32;
        while let Some(c) = o.get(i).filter(|c| c.is_ascii_digit()) {
            nanos += u32::from(c - b'0') * echelle;
            echelle /= 10;
            i += 1;
        }
    }
    let decalage = match o.get(i) {
        Some(b'+') => nombre(i + 1, i + 3) * 3600 + nombre(i + 4, i + 6) * 60,
        Some(b'-') => -(nombre(i + 1, i + 3) * 3600 + nombre(i + 4, i + 6) * 60),
        _ => 0,
    };
    (jours * 86_400 + h * 3600 + m * 60 + s - decalage, nanos)
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

/// Amène la base à [`VERSION_BASE`], une organisation après l'autre, dans une
/// seule transaction : un arrêt en cours de route n'écrit rien, et la base
/// garde son ancien numéro. Une base plus récente est refusée sans être
/// touchée.
fn migrer(base: &mut Connection) -> Resultat<()> {
    let transaction = base.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut version: i32 = transaction.pragma_query_value(None, "user_version", |l| l.get(0))?;
    if version > VERSION_BASE {
        return Err(ErreurBibliotheque::VersionBase(version));
    }
    while version < VERSION_BASE {
        match version {
            0 => transaction.execute_batch(ORGANISATION_1)?,
            1 => transaction.execute_batch(ORGANISATION_2)?,
            2 => transaction.execute_batch(ORGANISATION_3)?,
            autre => unreachable!("aucune migration depuis l'organisation {autre}"),
        }
        version += 1;
        transaction.pragma_update(None, "user_version", version)?;
    }
    transaction.commit()?;
    Ok(())
}

/// Organisation 2 (ticket #7) : le nom d'une mesure. Les mesures déjà
/// enregistrées restent sans nom ; rien n'est réécrit ni effacé.
const ORGANISATION_2: &str = "ALTER TABLE mesures ADD COLUMN nom TEXT;";

/// Nom de mesure débarrassé de ses espaces de début et de fin, jamais vide.
fn nom_de_mesure(nom: &str) -> Resultat<&str> {
    let nom = nom.trim();
    if nom.is_empty() {
        Err(ErreurBibliotheque::NomVide)
    } else {
        Ok(nom)
    }
}

/// Organisation 1. `IF NOT EXISTS` : la toute première version de cette
/// crate créait les tables puis écrivait le numéro en deux temps, et pouvait
/// laisser des tables complètes sous le numéro 0.
const ORGANISATION_1: &str = "
CREATE TABLE IF NOT EXISTS conditions (
    id INTEGER PRIMARY KEY,
    nom TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS instruments (
    id INTEGER PRIMARY KEY,
    modele TEXT NOT NULL,
    numero_serie INTEGER NOT NULL,
    UNIQUE (modele, numero_serie)
);
CREATE TABLE IF NOT EXISTS mesures (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    instrument INTEGER NOT NULL REFERENCES instruments (id),
    horodatage TEXT NOT NULL,
    geometrie TEXT NOT NULL,
    plages INTEGER NOT NULL,
    contenu TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS mesures_par_condition ON mesures (condition);
";

/// Organisation 3 : les mesures importées (ticket #9), par-dessus
/// l'organisation 2. La table des mesures est refaite pour accueillir une
/// mesure sans instrument attesté, sans date ni géométrie connues, avec son
/// origine et le nom de son fichier. Toutes les mesures existantes sont
/// recopiées telles quelles, comme mesures d'un pont, avec leur numéro et
/// leur nom ; rien n'est effacé avant d'être recopié, et le tout se fait
/// dans la transaction de [`migrer`].
const ORGANISATION_3: &str = "
CREATE TABLE mesures_3 (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    origine TEXT NOT NULL DEFAULT 'pont' CHECK (origine IN ('pont', 'importee')),
    instrument INTEGER REFERENCES instruments (id),
    horodatage TEXT,
    geometrie TEXT,
    plages INTEGER NOT NULL,
    fichier TEXT,
    contenu TEXT NOT NULL,
    nom TEXT,
    CHECK (origine = 'importee' OR (instrument IS NOT NULL AND horodatage IS NOT NULL
                                    AND geometrie IS NOT NULL)),
    CHECK (origine = 'pont' OR fichier IS NOT NULL)
);
INSERT INTO mesures_3 (id, condition, origine, instrument, horodatage, geometrie, plages, contenu, nom)
    SELECT id, condition, 'pont', instrument, horodatage, geometrie, plages, contenu, nom FROM mesures;
DROP TABLE mesures;
ALTER TABLE mesures_3 RENAME TO mesures;
CREATE INDEX mesures_par_condition ON mesures (condition);
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn la_date_d_une_copie_de_secours_est_juste() {
        let a = |s: u64| date_heure_utc(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(a(0), "1970-01-01-000000Z");
        assert_eq!(a(1_791_552_845), "2026-10-09-133405Z");
        assert_eq!(a(1_709_251_199), "2024-02-29-235959Z");
    }
}
