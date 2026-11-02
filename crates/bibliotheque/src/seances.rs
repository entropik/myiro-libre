//! Séances de mesure d'un tirage et leurs mesures sources (organisation 8,
//! ticket #35). Plan, fiche, association, compléments et composition y sont
//! du JSON opaque, lu par l'application (crate `seance-mesure`). Chaque
//! mesure source est une mesure importée de la résolution, gardée telle
//! quelle. Une séance terminée ne change plus : la base le refuse.

use pont_protocole::Horodatage;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{Bibliotheque, ErreurBibliotheque, IdCondition, IdMesure, IdTirage, Resultat};

/// Numéro d'une séance de mesure dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdSeance(pub i64);

/// Où en est une séance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatSeance {
    /// Des feuilles peuvent encore être mesurées ou importées.
    EnCours,
    /// Fermée avec toutes ses plages.
    Terminee,
    /// Fermée par un choix explicite, des plages manquant encore.
    Incomplete,
}

impl EtatSeance {
    fn texte(self) -> &'static str {
        match self {
            EtatSeance::EnCours => "en_cours",
            EtatSeance::Terminee => "terminee",
            EtatSeance::Incomplete => "incomplete",
        }
    }

    fn depuis(texte: &str) -> rusqlite::Result<Self> {
        match texte {
            "en_cours" => Ok(EtatSeance::EnCours),
            "terminee" => Ok(EtatSeance::Terminee),
            "incomplete" => Ok(EtatSeance::Incomplete),
            autre => Err(inconnu("état de séance", autre)),
        }
    }
}

/// D'où vient une mesure source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigineSource {
    /// Mesurée au FD-9 depuis l'application.
    Acquisition,
    /// Importée d'un fichier (FD-S2w par exemple).
    Import,
}

/// Ce qu'une mesure source apporte à la séance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleSource {
    /// La mesure d'un exemplaire de la mire.
    Exemplaire,
    /// Une mesure qui ne sert qu'aux compléments manuels.
    Complement,
}

fn inconnu(quoi: &str, texte: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("{quoi} « {texte} » inconnu").into(),
    )
}

impl OrigineSource {
    fn texte(self) -> &'static str {
        match self {
            OrigineSource::Acquisition => "acquisition",
            OrigineSource::Import => "import",
        }
    }

    fn depuis(texte: &str) -> rusqlite::Result<Self> {
        match texte {
            "acquisition" => Ok(OrigineSource::Acquisition),
            "import" => Ok(OrigineSource::Import),
            autre => Err(inconnu("origine", autre)),
        }
    }
}

impl RoleSource {
    fn texte(self) -> &'static str {
        match self {
            RoleSource::Exemplaire => "exemplaire",
            RoleSource::Complement => "complement",
        }
    }

    fn depuis(texte: &str) -> rusqlite::Result<Self> {
        match texte {
            "exemplaire" => Ok(RoleSource::Exemplaire),
            "complement" => Ok(RoleSource::Complement),
            autre => Err(inconnu("rôle", autre)),
        }
    }
}

/// Une séance à commencer.
#[derive(Clone, Copy, Debug)]
pub struct NouvelleSeance<'a> {
    pub condition: IdCondition,
    /// Le tirage mesuré, de cette résolution.
    pub tirage: IdTirage,
    /// Plan figé, en JSON.
    pub plan: &'a str,
    /// Fiche d'impression confirmée, en JSON.
    pub fiche: &'a str,
    /// RFC 3339.
    pub horodatage: &'a str,
}

/// Une mesure source à ranger dans une séance.
#[derive(Clone, Copy, Debug)]
pub struct NouvelleSource<'a> {
    /// Nom du fichier : seul le nom est gardé, pas son dossier.
    pub fichier: &'a str,
    /// Texte CGATS, gardé tel quel.
    pub texte: &'a str,
    pub origine: OrigineSource,
    pub role: RoleSource,
    /// Association de ses plages au plan, en JSON.
    pub association: &'a str,
    /// RFC 3339.
    pub horodatage: &'a str,
}

/// Une mesure source rangée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceRangee {
    pub mesure: IdMesure,
    pub fichier: String,
    pub origine: OrigineSource,
    pub role: RoleSource,
    pub association: String,
    /// Comptée dans la mesure moyennée ; une exclusion est un choix
    /// explicite de l'opérateur.
    pub retenue: bool,
    /// Le choix de l'opérateur (motif, heure), en JSON ; rien sans choix.
    pub selection: Option<String>,
    pub horodatage: String,
}

/// Une séance, telle qu'elle est rangée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeanceRangee {
    pub id: IdSeance,
    pub condition: IdCondition,
    pub tirage: IdTirage,
    pub plan: String,
    pub fiche: String,
    pub horodatage: String,
    pub etat: EtatSeance,
    /// Compléments manuels, en JSON.
    pub complements: String,
    /// Fermeture, en RFC 3339 ; rien tant qu'elle est en cours.
    pub fin: Option<String>,
    /// La mesure moyennée rangée à la fermeture.
    pub resultat: Option<IdMesure>,
    /// Composition plage par plage, écarts et exclusions, en JSON.
    pub composition: Option<String>,
    /// Dans l'ordre où elles ont été rangées.
    pub sources: Vec<SourceRangee>,
}

/// La fermeture d'une séance.
#[derive(Clone, Copy, Debug)]
pub struct FinSeance<'a> {
    /// [`EtatSeance::Terminee`] ou [`EtatSeance::Incomplete`].
    pub etat: EtatSeance,
    /// RFC 3339.
    pub horodatage: &'a str,
    /// Nom et texte CGATS de la mesure moyennée, rangée comme mesure
    /// importée de la résolution.
    pub resultat: Option<(&'a str, &'a str)>,
    /// Composition, en JSON.
    pub composition: &'a str,
}

/// Ce qu'une mesure est dans une séance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "place", rename_all = "snake_case")]
pub enum PlaceEnSeance {
    /// Une mesure source, avec son association au plan (JSON).
    Source { association: String },
    /// La mesure moyennée de la séance.
    Resultat,
}

/// La séance où une mesure a sa place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SituationSeance {
    pub seance: IdSeance,
    pub etat: EtatSeance,
    pub place: PlaceEnSeance,
}

fn horodatage_valide(texte: &str) -> Resultat<()> {
    Horodatage::new(texte)
        .map(|_| ())
        .map_err(|_| ErreurBibliotheque::HorodatageInvalide(texte.to_string()))
}

impl Bibliotheque {
    /// Commence une séance sur un tirage de la résolution.
    pub fn creer_seance(&self, s: &NouvelleSeance) -> Resultat<IdSeance> {
        self.verifier_condition(s.condition)?;
        horodatage_valide(s.horodatage)?;
        let condition: Option<i64> = self
            .base
            .query_row(
                "SELECT condition FROM tirages WHERE id = ?1",
                params![s.tirage.0],
                |l| l.get(0),
            )
            .optional()?;
        if condition != Some(s.condition.0) {
            return Err(ErreurBibliotheque::TirageInconnu(s.tirage));
        }
        self.base.execute(
            "INSERT INTO seances (condition, tirage, plan, fiche, horodatage)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![s.condition.0, s.tirage.0, s.plan, s.fiche, s.horodatage],
        )?;
        Ok(IdSeance(self.base.last_insert_rowid()))
    }

    /// Relit une séance et ses mesures sources.
    pub fn seance(&self, id: IdSeance) -> Resultat<SeanceRangee> {
        let mut seance = self
            .base
            .query_row(
                "SELECT condition, tirage, plan, fiche, horodatage, etat, complements, fin,
                        resultat, composition
                 FROM seances WHERE id = ?1",
                params![id.0],
                |l| {
                    Ok(SeanceRangee {
                        id,
                        condition: IdCondition(l.get(0)?),
                        tirage: IdTirage(l.get(1)?),
                        plan: l.get(2)?,
                        fiche: l.get(3)?,
                        horodatage: l.get(4)?,
                        etat: EtatSeance::depuis(&l.get::<_, String>(5)?)?,
                        complements: l.get(6)?,
                        fin: l.get(7)?,
                        resultat: l.get::<_, Option<i64>>(8)?.map(IdMesure),
                        composition: l.get(9)?,
                        sources: Vec::new(),
                    })
                },
            )
            .optional()?
            .ok_or(ErreurBibliotheque::SeanceInconnue(id))?;
        let mut requete = self.base.prepare(
            "SELECT s.mesure, m.fichier, s.origine, s.role, s.association, s.retenue,
                    s.selection, s.horodatage
             FROM seance_sources s JOIN mesures m ON m.id = s.mesure
             WHERE s.seance = ?1 ORDER BY s.id",
        )?;
        let sources = requete.query_map(params![id.0], |l| {
            Ok(SourceRangee {
                mesure: IdMesure(l.get(0)?),
                fichier: l.get(1)?,
                origine: OrigineSource::depuis(&l.get::<_, String>(2)?)?,
                role: RoleSource::depuis(&l.get::<_, String>(3)?)?,
                association: l.get(4)?,
                retenue: l.get(5)?,
                selection: l.get(6)?,
                horodatage: l.get(7)?,
            })
        })?;
        seance.sources = sources.collect::<Result<_, _>>()?;
        Ok(seance)
    }

    /// Les séances en cours de la résolution, de la plus ancienne à la
    /// plus récente : celles qu'on peut reprendre.
    pub fn seances_en_cours(&self, condition: IdCondition) -> Resultat<Vec<IdSeance>> {
        let mut requete = self.base.prepare(
            "SELECT id FROM seances WHERE condition = ?1 AND etat = 'en_cours' ORDER BY id",
        )?;
        let ids = requete.query_map(params![condition.0], |l| Ok(IdSeance(l.get(0)?)))?;
        Ok(ids.collect::<Result<_, _>>()?)
    }

    /// La séance en cours, ou refusée si elle est fermée.
    fn seance_en_cours(&self, id: IdSeance) -> Resultat<IdCondition> {
        let ligne: Option<(i64, String)> = self
            .base
            .query_row(
                "SELECT condition, etat FROM seances WHERE id = ?1",
                params![id.0],
                |l| Ok((l.get(0)?, l.get(1)?)),
            )
            .optional()?;
        match ligne {
            None => Err(ErreurBibliotheque::SeanceInconnue(id)),
            Some((condition, etat)) if etat == "en_cours" => Ok(IdCondition(condition)),
            Some(_) => Err(ErreurBibliotheque::SeanceFermee(id)),
        }
    }

    /// Range une mesure source dans une séance en cours : la mesure est
    /// importée dans la résolution de la séance, puis rattachée à la séance,
    /// en une transaction. Un fichier illisible ne range rien.
    pub fn ajouter_source(&self, seance: IdSeance, s: &NouvelleSource) -> Resultat<IdMesure> {
        let condition = self.seance_en_cours(seance)?;
        horodatage_valide(s.horodatage)?;
        let transaction = self.base.unchecked_transaction()?;
        let mesure = self.importer_mesure(condition, s.fichier, s.texte)?;
        transaction.execute(
            "INSERT INTO seance_sources (seance, mesure, origine, role, association, horodatage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                seance.0,
                mesure.0,
                s.origine.texte(),
                s.role.texte(),
                s.association,
                s.horodatage
            ],
        )?;
        transaction.commit()?;
        Ok(mesure)
    }

    /// Retient ou écarte une mesure source de la moyenne, avec le choix de
    /// l'opérateur (motif, heure, en JSON).
    pub fn retenir_source(
        &self,
        seance: IdSeance,
        mesure: IdMesure,
        retenue: bool,
        selection: Option<&str>,
    ) -> Resultat<()> {
        self.seance_en_cours(seance)?;
        let changees = self.base.execute(
            "UPDATE seance_sources SET retenue = ?1, selection = ?2
             WHERE seance = ?3 AND mesure = ?4",
            params![retenue, selection, seance.0, mesure.0],
        )?;
        if changees == 0 {
            return Err(ErreurBibliotheque::MesureInconnue(mesure));
        }
        Ok(())
    }

    /// Remplace les compléments manuels d'une séance en cours (JSON).
    pub fn changer_complements(&self, seance: IdSeance, complements: &str) -> Resultat<()> {
        self.seance_en_cours(seance)?;
        self.base.execute(
            "UPDATE seances SET complements = ?1 WHERE id = ?2",
            params![complements, seance.0],
        )?;
        Ok(())
    }

    /// Ferme une séance en cours : la mesure moyennée, s'il y en a une, est
    /// rangée comme mesure importée de la résolution, avec la composition,
    /// en une transaction. Rend le numéro de la mesure moyennée.
    pub fn terminer_seance(&self, seance: IdSeance, fin: &FinSeance) -> Resultat<Option<IdMesure>> {
        let condition = self.seance_en_cours(seance)?;
        if fin.etat == EtatSeance::EnCours {
            return Err(ErreurBibliotheque::Base(
                "une séance se ferme terminée ou incomplète".into(),
            ));
        }
        horodatage_valide(fin.horodatage)?;
        let transaction = self.base.unchecked_transaction()?;
        let resultat = fin
            .resultat
            .map(|(fichier, texte)| self.importer_mesure(condition, fichier, texte))
            .transpose()?;
        transaction.execute(
            "UPDATE seances SET etat = ?1, fin = ?2, resultat = ?3, composition = ?4
             WHERE id = ?5",
            params![
                fin.etat.texte(),
                fin.horodatage,
                resultat.map(|m| m.0),
                fin.composition,
                seance.0
            ],
        )?;
        transaction.commit()?;
        Ok(resultat)
    }

    /// La séance où la mesure a sa place, comme source ou comme résultat ;
    /// rien pour une mesure hors séance.
    pub fn situation_en_seance(&self, mesure: IdMesure) -> Resultat<Option<SituationSeance>> {
        let source: Option<(i64, String, String)> = self
            .base
            .query_row(
                "SELECT s.id, s.etat, x.association
                 FROM seance_sources x JOIN seances s ON s.id = x.seance
                 WHERE x.mesure = ?1",
                params![mesure.0],
                |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?)),
            )
            .optional()?;
        if let Some((seance, etat, association)) = source {
            return Ok(Some(SituationSeance {
                seance: IdSeance(seance),
                etat: EtatSeance::depuis(&etat)?,
                place: PlaceEnSeance::Source { association },
            }));
        }
        let resultat: Option<(i64, String)> = self
            .base
            .query_row(
                "SELECT id, etat FROM seances WHERE resultat = ?1",
                params![mesure.0],
                |l| Ok((l.get(0)?, l.get(1)?)),
            )
            .optional()?;
        Ok(match resultat {
            Some((seance, etat)) => Some(SituationSeance {
                seance: IdSeance(seance),
                etat: EtatSeance::depuis(&etat)?,
                place: PlaceEnSeance::Resultat,
            }),
            None => None,
        })
    }
}

/// Organisation 8 (ticket #35), par-dessus l'organisation 7 : les séances de
/// mesure et leurs mesures sources. Rien n'est réécrit ni effacé. Une
/// séance fermée ne change plus, ni ses sources ; une séance ou une source
/// ne s'efface pas. Un tirage est « mesuré » dès qu'une séance fermée le
/// mesure ([`Bibliotheque::tirages_prepares`]).
pub(crate) const ORGANISATION_8: &str = "
CREATE TABLE seances (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    tirage INTEGER NOT NULL REFERENCES tirages (id),
    plan TEXT NOT NULL,
    fiche TEXT NOT NULL,
    horodatage TEXT NOT NULL,
    etat TEXT NOT NULL DEFAULT 'en_cours'
        CHECK (etat IN ('en_cours', 'terminee', 'incomplete')),
    complements TEXT NOT NULL DEFAULT '[]',
    fin TEXT,
    resultat INTEGER REFERENCES mesures (id),
    composition TEXT,
    CHECK ((etat = 'en_cours') = (fin IS NULL))
);
CREATE INDEX seances_par_condition ON seances (condition);
CREATE INDEX seances_par_tirage ON seances (tirage);
CREATE TABLE seance_sources (
    id INTEGER PRIMARY KEY,
    seance INTEGER NOT NULL REFERENCES seances (id),
    mesure INTEGER NOT NULL UNIQUE REFERENCES mesures (id),
    origine TEXT NOT NULL CHECK (origine IN ('acquisition', 'import')),
    role TEXT NOT NULL CHECK (role IN ('exemplaire', 'complement')),
    association TEXT NOT NULL,
    retenue INTEGER NOT NULL DEFAULT 1 CHECK (retenue IN (0, 1)),
    selection TEXT,
    horodatage TEXT NOT NULL
);
CREATE INDEX seance_sources_par_seance ON seance_sources (seance);
CREATE TRIGGER seances_fermees BEFORE UPDATE ON seances WHEN OLD.etat <> 'en_cours'
    BEGIN SELECT RAISE(ABORT, 'séance fermée'); END;
CREATE TRIGGER seances_gardees BEFORE DELETE ON seances
    BEGIN SELECT RAISE(ABORT, 'séance gardée'); END;
CREATE TRIGGER sources_fermees BEFORE UPDATE ON seance_sources
    WHEN (SELECT etat FROM seances WHERE id = OLD.seance) <> 'en_cours'
    BEGIN SELECT RAISE(ABORT, 'séance fermée'); END;
CREATE TRIGGER sources_fixes
    BEFORE UPDATE OF seance, mesure, origine, role, association, horodatage ON seance_sources
    BEGIN SELECT RAISE(ABORT, 'mesure source immuable'); END;
CREATE TRIGGER sources_gardees BEFORE DELETE ON seance_sources
    BEGIN SELECT RAISE(ABORT, 'mesure source gardée'); END;
";
