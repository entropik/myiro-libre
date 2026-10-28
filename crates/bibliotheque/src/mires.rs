//! Mires de profilage et leurs tirages, rangés avec leur condition
//! d'impression (organisation 6). La bibliothèque ne dépend pas de la crate
//! `mires` : valeurs, mise en page et génération y sont du JSON opaque,
//! conservé tel quel.

use pont_protocole::Horodatage;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{instant, Bibliotheque, ErreurBibliotheque, IdCondition, Resultat};

/// Numéro d'une mire dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdMire(pub i64);

/// Numéro d'un tirage : la trace d'une préparation de mire à imprimer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdTirage(pub i64);

/// Instruments dont la bibliothèque range les mires.
const INSTRUMENTS_MIRE: [&str; 2] = ["fd9", "myiro1"];

/// Une mire à ranger avec son premier tirage.
#[derive(Clone, Copy, Debug)]
pub struct NouvelleMire<'a> {
    pub nom: &'a str,
    /// `"fd9"` ou `"myiro1"`.
    pub instrument: &'a str,
    pub plages: usize,
    /// JSON : commande, version et journal (MYIRO-1) ; fichier repris tel
    /// quel (FD-9).
    pub generation: &'a str,
    /// JSON des valeurs RVB, de 0 à 100, dans l'ordre des plages.
    pub valeurs: &'a str,
    /// JSON de la mise en page (MYIRO-1) ou de la géométrie (FD-9).
    pub mise_en_page: &'a str,
    /// Empreinte de la mise en page (MYIRO-1) ou SHA-256 du TIFF (FD-9).
    pub empreinte: &'a str,
    /// Préparation, en RFC 3339.
    pub horodatage: &'a str,
}

/// Ce que la vue d'une résolution montre d'un tirage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeTirage {
    pub id: IdTirage,
    pub mire: IdMire,
    pub nom: String,
    pub instrument: String,
    pub plages: usize,
    pub pages: usize,
    pub horodatage: String,
}

/// Une mire rangée, telle qu'elle a été enregistrée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MireRangee {
    pub id: IdMire,
    pub nom: String,
    pub instrument: String,
    pub plages: usize,
    pub generation: String,
    pub valeurs: String,
    pub mise_en_page: String,
    pub empreinte: String,
}

impl Bibliotheque {
    /// Range la mire et un tirage de `pages` pages dans la condition, en une
    /// transaction. Une mire de même empreinte déjà rangée est reprise, pas
    /// dupliquée : chaque préparation de la mire FD-9 ajoute un tirage de la
    /// même mire RGB-2520.
    pub fn enregistrer_mire_et_tirage(
        &self,
        condition: IdCondition,
        mire: &NouvelleMire,
        pages: usize,
    ) -> Resultat<(IdMire, IdTirage)> {
        self.enregistrer_mire_et_tirage_si(condition, mire, pages, |_| {
            Ok::<(), std::convert::Infallible>(())
        })
        .map(|rangee| match rangee {
            Ok(ids) => ids,
            Err(jamais) => match jamais {},
        })
    }

    /// Comme [`Self::enregistrer_mire_et_tirage`], mais la transaction n'est
    /// validée qu'après `ecrire`, qui reçoit le numéro du tirage (il figure
    /// dans le QR code et le nom des fichiers) : si `ecrire` échoue, rien
    /// n'est rangé, ni tirage ni mire nouvelle, et son erreur est rendue
    /// (résultat intérieur). Un tirage n'apparaît jamais sans ses fichiers.
    pub fn enregistrer_mire_et_tirage_si<E>(
        &self,
        condition: IdCondition,
        mire: &NouvelleMire,
        pages: usize,
        ecrire: impl FnOnce(IdTirage) -> Result<(), E>,
    ) -> Resultat<Result<(IdMire, IdTirage), E>> {
        self.verifier_condition(condition)?;
        let invalide = |cause: &str| Err(ErreurBibliotheque::MireInvalide(cause.to_string()));
        if mire.nom.trim().is_empty() {
            return Err(ErreurBibliotheque::NomVide);
        }
        if !INSTRUMENTS_MIRE.contains(&mire.instrument) {
            return invalide(&format!("instrument « {} » inconnu", mire.instrument));
        }
        if mire.plages == 0 || pages == 0 {
            return invalide("une mire a au moins une plage et une page");
        }
        if mire.empreinte.is_empty() {
            return invalide("empreinte vide");
        }
        if Horodatage::new(mire.horodatage).is_err() {
            return Err(ErreurBibliotheque::HorodatageInvalide(
                mire.horodatage.to_string(),
            ));
        }
        let entier =
            |n: usize| i64::try_from(n).map_err(|e| ErreurBibliotheque::Base(e.to_string()));
        let transaction = self.base.unchecked_transaction()?;
        let existante: Option<i64> = transaction
            .query_row(
                "SELECT id FROM mires WHERE empreinte = ?1",
                params![mire.empreinte],
                |l| l.get(0),
            )
            .optional()?;
        let id_mire = match existante {
            Some(id) => id,
            None => {
                transaction.execute(
                    "INSERT INTO mires
                         (nom, instrument, plages, generation, valeurs, mise_en_page, empreinte, horodatage)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        mire.nom.trim(),
                        mire.instrument,
                        entier(mire.plages)?,
                        mire.generation,
                        mire.valeurs,
                        mire.mise_en_page,
                        mire.empreinte,
                        mire.horodatage
                    ],
                )?;
                transaction.last_insert_rowid()
            }
        };
        transaction.execute(
            "INSERT INTO tirages (condition, mire, pages, horodatage) VALUES (?1, ?2, ?3, ?4)",
            params![condition.0, id_mire, entier(pages)?, mire.horodatage],
        )?;
        let id_tirage = IdTirage(transaction.last_insert_rowid());
        if let Err(e) = ecrire(id_tirage) {
            // La transaction est annulée en sortant de portée.
            return Ok(Err(e));
        }
        transaction.commit()?;
        Ok(Ok((IdMire(id_mire), id_tirage)))
    }

    /// Les tirages de la condition, du plus récent au plus ancien.
    pub fn tirages(&self, condition: IdCondition) -> Resultat<Vec<ResumeTirage>> {
        let mut requete = self.base.prepare(
            "SELECT t.id, m.id, m.nom, m.instrument, m.plages, t.pages, t.horodatage
             FROM tirages t JOIN mires m ON m.id = t.mire
             WHERE t.condition = ?1",
        )?;
        let lignes = requete.query_map(params![condition.0], |l| {
            Ok(ResumeTirage {
                id: IdTirage(l.get(0)?),
                mire: IdMire(l.get(1)?),
                nom: l.get(2)?,
                instrument: l.get(3)?,
                plages: l.get::<_, i64>(4)?.try_into().unwrap_or(0),
                pages: l.get::<_, i64>(5)?.try_into().unwrap_or(0),
                horodatage: l.get(6)?,
            })
        })?;
        let mut tirages = lignes.collect::<Result<Vec<_>, _>>()?;
        tirages.sort_by_cached_key(|t| std::cmp::Reverse((instant(&t.horodatage), t.id)));
        Ok(tirages)
    }

    /// La condition et la mire d'un tirage.
    pub fn mire_du_tirage(&self, tirage: IdTirage) -> Resultat<(IdCondition, MireRangee)> {
        self.base
            .query_row(
                "SELECT t.condition, m.id, m.nom, m.instrument, m.plages, m.generation,
                        m.valeurs, m.mise_en_page, m.empreinte
                 FROM tirages t JOIN mires m ON m.id = t.mire
                 WHERE t.id = ?1",
                params![tirage.0],
                |l| {
                    Ok((
                        IdCondition(l.get(0)?),
                        MireRangee {
                            id: IdMire(l.get(1)?),
                            nom: l.get(2)?,
                            instrument: l.get(3)?,
                            plages: l.get::<_, i64>(4)?.try_into().unwrap_or(0),
                            generation: l.get(5)?,
                            valeurs: l.get(6)?,
                            mise_en_page: l.get(7)?,
                            empreinte: l.get(8)?,
                        },
                    ))
                },
            )
            .optional()?
            .ok_or(ErreurBibliotheque::TirageInconnu(tirage))
    }
}
