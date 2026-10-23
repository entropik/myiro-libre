//! Profils ICC d'une condition d'impression : un profil actif, l'historique
//! des précédents, leur état de vérification et leur provenance.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use pont_protocole::Horodatage;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{instant, Bibliotheque, ErreurBibliotheque, IdCondition, IdMesure, Resultat};

/// Numéro d'un profil dans la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct IdProfil(pub i64);

/// Espace de données d'un profil ICC accepté par la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EspaceProfil {
    Rvb,
    Cmjn,
}

impl EspaceProfil {
    fn en_base(self) -> &'static str {
        match self {
            Self::Rvb => "rvb",
            Self::Cmjn => "cmjn",
        }
    }

    fn depuis_base(texte: &str) -> Option<Self> {
        match texte {
            "rvb" => Some(Self::Rvb),
            "cmjn" => Some(Self::Cmjn),
            _ => None,
        }
    }
}

/// État de vérification d'un profil.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatVerification {
    NonVerifie,
    Accepte,
    HorsTolerance,
    Incomplet,
}

impl EtatVerification {
    fn en_base(self) -> &'static str {
        match self {
            Self::NonVerifie => "non_verifie",
            Self::Accepte => "accepte",
            Self::HorsTolerance => "hors_tolerance",
            Self::Incomplet => "incomplet",
        }
    }

    fn depuis_base(texte: &str) -> Option<Self> {
        match texte {
            "non_verifie" => Some(Self::NonVerifie),
            "accepte" => Some(Self::Accepte),
            "hors_tolerance" => Some(Self::HorsTolerance),
            "incomplet" => Some(Self::Incomplet),
            _ => None,
        }
    }
}

/// Ce que l'arbre et l'historique montrent d'un profil, sans son fichier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResumeProfil {
    pub id: IdProfil,
    pub nom: String,
    /// Création du profil, en RFC 3339 UTC.
    pub horodatage: String,
    pub espace: EspaceProfil,
    pub etat: EtatVerification,
    /// SHA-256 du fichier, en hexadécimal minuscule.
    pub empreinte: String,
    pub actif: bool,
}

/// Un profil à enregistrer dans une condition d'impression.
#[derive(Clone, Copy, Debug)]
pub struct NouveauProfil<'a> {
    pub condition: IdCondition,
    pub nom: &'a str,
    /// Le fichier ICC, conservé tel quel.
    pub icc: &'a [u8],
    /// Création du profil, en RFC 3339 UTC.
    pub horodatage: &'a str,
    /// Commande et réglages qui ont produit le profil, conservés tels quels.
    pub parametres: &'a str,
    /// Mesures sources.
    pub mesures: &'a [IdMesure],
}

/// Espace de données d'un fichier ICC, après un contrôle de son en-tête :
/// au moins 132 octets, signature `acsp`, taille annoncée égale à celle du
/// fichier, espace RVB ou CMJN. L'erreur dit ce qui ne va pas.
pub fn espace_icc(icc: &[u8]) -> Result<EspaceProfil, String> {
    if icc.len() < 132 {
        return Err(format!(
            "fichier de {} octets, moins de 132 octets (en-tête ICC incomplet)",
            icc.len()
        ));
    }
    if icc.get(36..40) != Some(b"acsp".as_slice()) {
        return Err("signature acsp absente".into());
    }
    let annoncee = icc
        .get(0..4)
        .and_then(|o| <[u8; 4]>::try_from(o).ok())
        .map_or(0, u32::from_be_bytes);
    if usize::try_from(annoncee).ok() != Some(icc.len()) {
        return Err(format!(
            "taille annoncée {annoncee}, fichier de {} octets",
            icc.len()
        ));
    }
    match icc.get(16..20) {
        Some(b"RGB ") => Ok(EspaceProfil::Rvb),
        Some(b"CMYK") => Ok(EspaceProfil::Cmjn),
        autre => {
            let nom = autre.map_or_else(String::new, |o| {
                String::from_utf8_lossy(o).trim_end().to_string()
            });
            Err(format!("espace de données {nom} non pris en charge"))
        }
    }
}

/// Nom de fichier proposé pour un profil, valide sous Windows et lisible :
/// `Imprimante_Papier_Résolution_Date.icc`. Les caractères interdits par
/// Windows, les guillemets « » et les caractères de contrôle sont retirés,
/// les espaces deviennent des tirets ; les accents restent.
pub fn nom_profil_propose(imprimante: &str, papier: &str, resolution: &str, date: &str) -> String {
    let partie = |texte: &str| {
        let mut propre = String::with_capacity(texte.len());
        for c in texte.chars() {
            if "<>:\"/\\|?*«»".contains(c) || c.is_control() {
                continue;
            }
            let c = if c.is_whitespace() { '-' } else { c };
            if !(c == '-' && propre.ends_with('-')) {
                propre.push(c);
            }
        }
        propre.trim_matches('-').to_string()
    };
    let parties: Vec<String> = [imprimante, papier, resolution, date]
        .into_iter()
        .map(partie)
        .filter(|p| !p.is_empty())
        .collect();
    format!("{}.icc", parties.join("_"))
}

/// Vrai si le profil daté `horodatage` (RFC 3339) a plus d'un an (365
/// jours) à l'instant `maintenant`.
pub fn profil_ancien(horodatage: &str, maintenant: SystemTime) -> bool {
    let maintenant = maintenant
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    maintenant.saturating_sub(instant(horodatage).0) > 365 * 86_400
}

/// Empreinte SHA-256 en hexadécimal minuscule.
fn empreinte(octets: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for o in Sha256::digest(octets) {
        let _ = write!(hex, "{o:02x}");
    }
    hex
}

impl Bibliotheque {
    /// Enregistre un profil ICC RVB ou CMJN dans sa condition d'impression ;
    /// il devient le profil actif, et le précédent passe à l'historique. Un
    /// fichier qui n'est pas un profil utilisable est refusé et rien ne change.
    pub fn enregistrer_profil(&self, profil: &NouveauProfil) -> Resultat<IdProfil> {
        self.verifier_condition(profil.condition)?;
        let nom = profil.nom.trim();
        if nom.is_empty() {
            return Err(ErreurBibliotheque::NomVide);
        }
        // Une date illisible serait lue comme l'an 0 : profil faussement
        // ancien, historique mal trié.
        if Horodatage::new(profil.horodatage).is_err() {
            return Err(ErreurBibliotheque::HorodatageInvalide(
                profil.horodatage.to_string(),
            ));
        }
        let espace = espace_icc(profil.icc).map_err(ErreurBibliotheque::ProfilIllisible)?;
        let mesures = serde_json::to_string(profil.mesures)
            .map_err(|e| ErreurBibliotheque::Base(e.to_string()))?;
        let transaction = self.base.unchecked_transaction()?;
        transaction.execute(
            "UPDATE profils SET actif = 0 WHERE condition = ?1 AND actif = 1",
            params![profil.condition.0],
        )?;
        transaction.execute(
            "INSERT INTO profils
                 (condition, nom, horodatage, espace, etat, actif, empreinte, parametres, mesures, icc)
             VALUES (?1, ?2, ?3, ?4, 'non_verifie', 1, ?5, ?6, ?7, ?8)",
            params![
                profil.condition.0,
                nom,
                profil.horodatage,
                espace.en_base(),
                empreinte(profil.icc),
                profil.parametres,
                mesures,
                profil.icc
            ],
        )?;
        let id = IdProfil(transaction.last_insert_rowid());
        transaction.commit()?;
        Ok(id)
    }

    /// Le profil actif de la condition, s'il y en a un.
    pub fn profil_actif(&self, condition: IdCondition) -> Resultat<Option<ResumeProfil>> {
        Ok(self
            .resumes_profils(condition)?
            .into_iter()
            .find(|p| p.actif))
    }

    /// Tous les profils de la condition, le plus récent d'abord, actif
    /// compris.
    pub fn historique_profils(&self, condition: IdCondition) -> Resultat<Vec<ResumeProfil>> {
        let mut profils = self.resumes_profils(condition)?;
        profils.sort_by_cached_key(|p| std::cmp::Reverse((instant(&p.horodatage), p.id)));
        Ok(profils)
    }

    /// Le fichier ICC d'un profil, octet pour octet.
    pub fn fichier_profil(&self, id: IdProfil) -> Resultat<Vec<u8>> {
        self.base
            .query_row(
                "SELECT icc FROM profils WHERE id = ?1",
                params![id.0],
                |l| l.get(0),
            )
            .optional()?
            .ok_or(ErreurBibliotheque::ProfilInconnu(id))
    }

    /// Les paramètres qui ont produit le profil, tels qu'ils ont été
    /// enregistrés (commande et réglages, ou origine d'un profil rangé).
    pub fn parametres_profil(&self, id: IdProfil) -> Resultat<String> {
        self.base
            .query_row(
                "SELECT parametres FROM profils WHERE id = ?1",
                params![id.0],
                |l| l.get(0),
            )
            .optional()?
            .ok_or(ErreurBibliotheque::ProfilInconnu(id))
    }

    /// Note le résultat de la vérification d'un profil.
    pub fn marquer_verification(&self, id: IdProfil, etat: EtatVerification) -> Resultat<()> {
        let modifies = self.base.execute(
            "UPDATE profils SET etat = ?1 WHERE id = ?2",
            params![etat.en_base(), id.0],
        )?;
        if modifies == 0 {
            return Err(ErreurBibliotheque::ProfilInconnu(id));
        }
        Ok(())
    }

    fn resumes_profils(&self, condition: IdCondition) -> Resultat<Vec<ResumeProfil>> {
        let mut requete = self.base.prepare(
            "SELECT id, nom, horodatage, espace, etat, empreinte, actif FROM profils
             WHERE condition = ?1",
        )?;
        let lignes = requete.query_map(params![condition.0], |l| {
            Ok((
                IdProfil(l.get(0)?),
                l.get::<_, String>(1)?,
                l.get::<_, String>(2)?,
                l.get::<_, String>(3)?,
                l.get::<_, String>(4)?,
                l.get::<_, String>(5)?,
                l.get::<_, bool>(6)?,
            ))
        })?;
        let mut profils = Vec::new();
        for ligne in lignes {
            let (id, nom, horodatage, espace, etat, empreinte, actif) = ligne?;
            let illisible = |quoi: &str| {
                ErreurBibliotheque::Base(format!("profil n° {} : {quoi} inconnu", id.0))
            };
            profils.push(ResumeProfil {
                id,
                nom,
                horodatage,
                espace: EspaceProfil::depuis_base(&espace).ok_or_else(|| illisible("espace"))?,
                etat: EtatVerification::depuis_base(&etat).ok_or_else(|| illisible("état"))?,
                empreinte,
                actif,
            });
        }
        Ok(profils)
    }
}
