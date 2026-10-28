//! Jeux de tolérances versionnés et vérifications de profils (ticket #41),
//! immuables : modifier un jeu en range une nouvelle version, un recalcul
//! ajoute une vérification et garde l'originale. Critères, fiche
//! d'impression et calcul sont du JSON opaque, lu par l'application.

use pont_protocole::Horodatage;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::{
    instant, Bibliotheque, ErreurBibliotheque, EtatVerification, IdCondition, IdMesure, IdProfil,
    Resultat,
};

/// Numéro d'un jeu de tolérances (une version) dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdTolerances(pub i64);

/// Numéro d'une vérification dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdVerification(pub i64);

/// Une version d'un jeu de tolérances.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JeuTolerances {
    pub id: IdTolerances,
    pub nom: String,
    /// 1 pour la première, puis une de plus à chaque modification.
    pub version: u32,
    pub horodatage: String,
    /// Les critères, en JSON, tels qu'enregistrés.
    pub criteres: String,
    /// Au moins une vérification a été faite avec cette version.
    pub utilise: bool,
}

/// Une vérification à ranger.
#[derive(Clone, Copy, Debug)]
pub struct NouvelleVerification<'a> {
    pub profil: IdProfil,
    /// La mesure de la seconde mire.
    pub mesure: IdMesure,
    pub tolerances: IdTolerances,
    /// RFC 3339 UTC.
    pub horodatage: &'a str,
    /// Fiche d'impression confirmée, en JSON.
    pub fiche: &'a str,
    /// Calcul complet (valeurs plage par plage, contrôles), en JSON.
    pub calcul: &'a str,
    /// Accepté, hors tolérance ou incomplet ; jamais « non vérifié ».
    pub etat: EtatVerification,
    /// La vérification dont celle-ci est un recalcul sans remesure.
    pub recalcul_de: Option<IdVerification>,
}

/// Une vérification rangée, avec la version du jeu de tolérances employée.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VerificationRangee {
    pub id: IdVerification,
    pub profil: IdProfil,
    pub mesure: IdMesure,
    pub tolerances: JeuTolerances,
    pub horodatage: String,
    pub fiche: String,
    pub calcul: String,
    pub etat: EtatVerification,
    pub recalcul_de: Option<IdVerification>,
}

const COLONNES_JEU: &str = "t.id, t.nom, t.version, t.horodatage, t.criteres,
     EXISTS (SELECT 1 FROM verifications v WHERE v.tolerances = t.id)";

fn jeu_de_ligne(l: &Row<'_>, debut: usize) -> rusqlite::Result<JeuTolerances> {
    Ok(JeuTolerances {
        id: IdTolerances(l.get(debut)?),
        nom: l.get(debut + 1)?,
        version: l.get(debut + 2)?,
        horodatage: l.get(debut + 3)?,
        criteres: l.get(debut + 4)?,
        utilise: l.get(debut + 5)?,
    })
}

fn horodatage_valide(texte: &str) -> Resultat<()> {
    Horodatage::new(texte)
        .map(|_| ())
        .map_err(|_| ErreurBibliotheque::HorodatageInvalide(texte.to_string()))
}

impl Bibliotheque {
    /// Range une nouvelle version du jeu `nom` (débarrassé de ses espaces de
    /// bord) : la première porte le numéro 1, chaque suivante un de plus.
    /// Les versions précédentes ne changent pas.
    pub fn enregistrer_tolerances(
        &self,
        nom: &str,
        criteres: &str,
        horodatage: &str,
    ) -> Resultat<IdTolerances> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(ErreurBibliotheque::NomVide);
        }
        horodatage_valide(horodatage)?;
        let transaction = self.base.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO tolerances (nom, version, horodatage, criteres)
             VALUES (?1, (SELECT COALESCE(MAX(version), 0) + 1 FROM tolerances WHERE nom = ?1),
                     ?2, ?3)",
            params![nom, horodatage, criteres],
        )?;
        let id = IdTolerances(transaction.last_insert_rowid());
        transaction.commit()?;
        Ok(id)
    }

    /// Toutes les versions de tous les jeux, par nom puis de la plus récente
    /// à la plus ancienne.
    pub fn jeux_tolerances(&self) -> Resultat<Vec<JeuTolerances>> {
        let mut requete = self.base.prepare(&format!(
            "SELECT {COLONNES_JEU} FROM tolerances t ORDER BY t.nom, t.version DESC"
        ))?;
        let jeux = requete.query_map([], |l| jeu_de_ligne(l, 0))?;
        Ok(jeux.collect::<Result<_, _>>()?)
    }

    pub fn jeu_tolerances(&self, id: IdTolerances) -> Resultat<JeuTolerances> {
        self.base
            .query_row(
                &format!("SELECT {COLONNES_JEU} FROM tolerances t WHERE t.id = ?1"),
                params![id.0],
                |l| jeu_de_ligne(l, 0),
            )
            .optional()?
            .ok_or(ErreurBibliotheque::ToleranceInconnue(id))
    }

    /// Range une vérification et donne son état au profil, d'un bloc. Le
    /// profil reste actif (ou dans l'historique) quel que soit l'état.
    pub fn enregistrer_verification(&self, v: &NouvelleVerification) -> Resultat<IdVerification> {
        if v.etat == EtatVerification::NonVerifie {
            return Err(ErreurBibliotheque::EtatVerificationInvalide);
        }
        horodatage_valide(v.horodatage)?;
        self.condition_profil(v.profil)?;
        self.jeu_tolerances(v.tolerances)?;
        let mesure: Option<i64> = self
            .base
            .query_row(
                "SELECT id FROM mesures WHERE id = ?1",
                params![v.mesure.0],
                |l| l.get(0),
            )
            .optional()?;
        if mesure.is_none() {
            return Err(ErreurBibliotheque::MesureInconnue(v.mesure));
        }
        if let Some(origine) = v.recalcul_de {
            self.verification(origine)?;
        }
        let transaction = self.base.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO verifications
                 (profil, mesure, tolerances, horodatage, fiche, calcul, etat, recalcul_de)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                v.profil.0,
                v.mesure.0,
                v.tolerances.0,
                v.horodatage,
                v.fiche,
                v.calcul,
                v.etat.en_base(),
                v.recalcul_de.map(|r| r.0)
            ],
        )?;
        let id = IdVerification(transaction.last_insert_rowid());
        // L'état du profil est celui de sa vérification la plus récente, dans
        // la même transaction.
        let derniere = self
            .verifications(v.profil)?
            .first()
            .map_or(v.etat, |d| d.etat);
        self.marquer_verification(v.profil, derniere)?;
        transaction.commit()?;
        Ok(id)
    }

    /// Les vérifications d'un profil, la plus récente d'abord.
    pub fn verifications(&self, profil: IdProfil) -> Resultat<Vec<VerificationRangee>> {
        let mut liste = self.lire_verifications("v.profil = ?1", profil.0)?;
        liste.sort_by_cached_key(|v| std::cmp::Reverse((instant(&v.horodatage), v.id)));
        Ok(liste)
    }

    pub fn verification(&self, id: IdVerification) -> Resultat<VerificationRangee> {
        self.lire_verifications("v.id = ?1", id.0)?
            .pop()
            .ok_or(ErreurBibliotheque::VerificationInconnue(id))
    }

    fn lire_verifications(&self, filtre: &str, valeur: i64) -> Resultat<Vec<VerificationRangee>> {
        let mut requete = self.base.prepare(&format!(
            "SELECT v.id, v.profil, v.mesure, v.horodatage, v.fiche, v.calcul, v.etat,
                    v.recalcul_de, {COLONNES_JEU}
             FROM verifications v JOIN tolerances t ON t.id = v.tolerances
             WHERE {filtre}"
        ))?;
        let lignes = requete.query_map(params![valeur], |l| {
            Ok((
                VerificationRangee {
                    id: IdVerification(l.get(0)?),
                    profil: IdProfil(l.get(1)?),
                    mesure: IdMesure(l.get(2)?),
                    horodatage: l.get(3)?,
                    fiche: l.get(4)?,
                    calcul: l.get(5)?,
                    etat: EtatVerification::NonVerifie,
                    recalcul_de: l.get::<_, Option<i64>>(7)?.map(IdVerification),
                    tolerances: jeu_de_ligne(l, 8)?,
                },
                l.get::<_, String>(6)?,
            ))
        })?;
        let mut liste = Vec::new();
        for ligne in lignes {
            let (mut v, etat) = ligne?;
            v.etat = EtatVerification::depuis_base(&etat).ok_or_else(|| {
                ErreurBibliotheque::Base(format!("vérification n° {} : état inconnu", v.id.0))
            })?;
            liste.push(v);
        }
        Ok(liste)
    }

    /// Les mesures sources d'un profil (aucune pour un profil rangé tel quel).
    pub fn mesures_profil(&self, profil: IdProfil) -> Resultat<Vec<IdMesure>> {
        let texte: String = self
            .base
            .query_row(
                "SELECT mesures FROM profils WHERE id = ?1",
                params![profil.0],
                |l| l.get(0),
            )
            .optional()?
            .ok_or(ErreurBibliotheque::ProfilInconnu(profil))?;
        let ids: Vec<i64> = serde_json::from_str(&texte).map_err(|e| {
            ErreurBibliotheque::Base(format!("profil n° {} : mesures {e}", profil.0))
        })?;
        Ok(ids.into_iter().map(IdMesure).collect())
    }

    /// La résolution (condition d'impression) où le profil est rangé.
    pub fn condition_profil(&self, profil: IdProfil) -> Resultat<IdCondition> {
        self.base
            .query_row(
                "SELECT condition FROM profils WHERE id = ?1",
                params![profil.0],
                |l| l.get(0).map(IdCondition),
            )
            .optional()?
            .ok_or(ErreurBibliotheque::ProfilInconnu(profil))
    }
}

/// Numéro d'une mire de vérification dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdMireVerification(pub i64);

/// Une mire de vérification rangée : son contenu (XML de FD-S2w pour le
/// FD-9), tel qu'écrit la première fois.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MireVerificationRangee {
    pub id: IdMireVerification,
    pub nom: String,
    /// `fd9` ou `myiro1`.
    pub instrument: String,
    pub version: u32,
    /// SHA-256 du contenu, en hexadécimal minuscule.
    pub empreinte: String,
    pub contenu: String,
    /// Première fois qu'elle a été rangée.
    pub horodatage: String,
}

impl Bibliotheque {
    /// Range une mire de vérification une seule fois : un contenu déjà rangé
    /// est rendu tel qu'il a été rangé la première fois, sans rien changer.
    pub fn ranger_mire_verification(
        &self,
        nom: &str,
        instrument: &str,
        version: u32,
        contenu: &str,
        horodatage: &str,
    ) -> Resultat<MireVerificationRangee> {
        horodatage_valide(horodatage)?;
        let empreinte = crate::profils::empreinte(contenu.as_bytes());
        self.base.execute(
            "INSERT OR IGNORE INTO mires_verification
                 (nom, instrument, version, empreinte, contenu, horodatage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![nom, instrument, version, empreinte, contenu, horodatage],
        )?;
        self.mires_verification()?
            .into_iter()
            .find(|m| m.empreinte == empreinte)
            .ok_or_else(|| {
                ErreurBibliotheque::Base(format!("mire {empreinte} absente après rangement"))
            })
    }

    /// La mire de vérification rangée pour cet instrument et cette version,
    /// s'il y en a une.
    pub fn mire_verification(
        &self,
        instrument: &str,
        version: u32,
    ) -> Resultat<Option<MireVerificationRangee>> {
        Ok(self
            .mires_verification()?
            .into_iter()
            .find(|m| m.instrument == instrument && m.version == version))
    }

    /// Les mires de vérification, dans l'ordre où elles ont été rangées.
    pub fn mires_verification(&self) -> Resultat<Vec<MireVerificationRangee>> {
        let mut requete = self.base.prepare(
            "SELECT id, nom, instrument, version, empreinte, contenu, horodatage
             FROM mires_verification ORDER BY id",
        )?;
        let mires = requete.query_map([], |l| {
            Ok(MireVerificationRangee {
                id: IdMireVerification(l.get(0)?),
                nom: l.get(1)?,
                instrument: l.get(2)?,
                version: l.get(3)?,
                empreinte: l.get(4)?,
                contenu: l.get(5)?,
                horodatage: l.get(6)?,
            })
        })?;
        Ok(mires.collect::<Result<_, _>>()?)
    }
}

/// Organisation 7 (ticket #41), par-dessus l'organisation 6 (mires, #33) :
/// les jeux de tolérances versionnés, les vérifications de profils et les
/// mires de vérification (rangées une fois par contenu ; elles rejoindront
/// les mires de #33 à sa fusion). La base elle-même refuse de modifier ou
/// d'effacer un jeu ou une vérification. Rien n'est réécrit ni effacé.
pub(crate) const ORGANISATION_7: &str = "
CREATE TABLE mires_verification (
    id INTEGER PRIMARY KEY,
    nom TEXT NOT NULL,
    instrument TEXT NOT NULL CHECK (instrument IN ('fd9', 'myiro1')),
    version INTEGER NOT NULL CHECK (version > 0),
    empreinte TEXT NOT NULL UNIQUE,
    contenu TEXT NOT NULL,
    horodatage TEXT NOT NULL
);
CREATE TABLE tolerances (
    id INTEGER PRIMARY KEY,
    nom TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version > 0),
    horodatage TEXT NOT NULL,
    criteres TEXT NOT NULL,
    UNIQUE (nom, version)
);
CREATE TABLE verifications (
    id INTEGER PRIMARY KEY,
    profil INTEGER NOT NULL REFERENCES profils (id),
    mesure INTEGER NOT NULL REFERENCES mesures (id),
    tolerances INTEGER NOT NULL REFERENCES tolerances (id),
    horodatage TEXT NOT NULL,
    fiche TEXT NOT NULL,
    calcul TEXT NOT NULL,
    etat TEXT NOT NULL CHECK (etat IN ('accepte', 'hors_tolerance', 'incomplet')),
    recalcul_de INTEGER REFERENCES verifications (id)
);
CREATE INDEX verifications_par_profil ON verifications (profil);
CREATE TRIGGER tolerances_immuables BEFORE UPDATE ON tolerances
    BEGIN SELECT RAISE(ABORT, 'jeu de tolérances immuable'); END;
CREATE TRIGGER tolerances_gardees BEFORE DELETE ON tolerances
    BEGIN SELECT RAISE(ABORT, 'jeu de tolérances immuable'); END;
CREATE TRIGGER verifications_immuables BEFORE UPDATE ON verifications
    BEGIN SELECT RAISE(ABORT, 'vérification immuable'); END;
CREATE TRIGGER verifications_gardees BEFORE DELETE ON verifications
    BEGIN SELECT RAISE(ABORT, 'vérification immuable'); END;
";
