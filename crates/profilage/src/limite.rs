//! Limite du gamut d'un profil ICC, calculée par `iccgamut` (ArgyllCMS,
//! version figée [`VERSION_ARGYLL`]) derrière le trait [`MoteurGamut`] :
//! [`ArgyllCms`] en vrai, [`MoteurGamutSimule`] pour les tests (sur le modèle
//! de l'ADR 0005).
//!
//! Commande confirmée le 26 octobre 2026 avec ArgyllCMS 3.5.0 :
//! `iccgamut -v -ia -pl profil.icc` écrit `profil.gam` et affiche
//! `Total volume of gamut is 467316.128099 cubic colorspace units`.

use std::ffi::OsString;
use std::fs;

use crate::argyll::{controler_version, ligne, ArgyllCms, DossierTravail};
use crate::moteur::{ErreurMoteur, Etape, VERSION_ARGYLL};

/// Noms des fichiers dans le dossier de travail.
const PROFIL: &str = "profil.icc";
const GAM: &str = "profil.gam";

/// La limite du gamut d'un profil, telle qu'`iccgamut` l'a écrite.
#[derive(Clone, Debug, PartialEq)]
pub struct LimiteCalculee {
    /// Texte du fichier `.gam` (sommets Lab et triangles), à lire par
    /// `gamut::lire_gam`.
    pub gam: String,
    /// Volume annoncé par `iccgamut`, s'il a pu être lu dans sa sortie.
    pub volume: Option<f64>,
    /// Version d'ArgyllCMS qui a calculé la limite.
    pub version: String,
}

/// Un moteur qui calcule la limite du gamut d'un profil ICC.
pub trait MoteurGamut: Send + Sync {
    /// Commande équivalente, sans rien lancer (section « Avancé »).
    fn commande_limite(&self) -> String;
    /// Calcule la limite du gamut du profil `icc`. En cas d'erreur, rien
    /// n'est rendu.
    fn limite(&self, icc: &[u8]) -> Result<LimiteCalculee, ErreurMoteur>;
}

/// Arguments d'`iccgamut` : `-v` (volume dans la sortie), `-ia` (table
/// colorimétrique absolue), `-pl` (limite en Lab), puis le profil.
pub fn arguments_iccgamut() -> Vec<OsString> {
    ["-v", "-ia", "-pl", PROFIL].map(OsString::from).into()
}

/// Volume lu dans la ligne `Total volume of gamut is <x> cubic colorspace
/// units` d'`iccgamut -v` ; rien si la ligne manque ou si le nombre est
/// illisible.
pub fn lire_volume_iccgamut(sortie: &str) -> Option<f64> {
    sortie.lines().find_map(|l| {
        l.split_once("Total volume of gamut is")?
            .1
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

impl MoteurGamut for ArgyllCms {
    fn commande_limite(&self) -> String {
        ligne("iccgamut", &arguments_iccgamut())
    }

    /// Version contrôlée, puis `iccgamut` lancé dans un dossier de travail
    /// effacé à la fin, réussite ou échec. Un ArgyllCMS sans `iccgamut` est
    /// dit introuvable.
    fn limite(&self, icc: &[u8]) -> Result<LimiteCalculee, ErreurMoteur> {
        let version = self.version()?;
        controler_version(&version)?;
        if !self.outil("iccgamut").is_file() {
            return Err(ErreurMoteur::Introuvable);
        }

        let dossier = DossierTravail::creer()?;
        let chemin = dossier.chemin();
        fs::write(chemin.join(PROFIL), icc)
            .map_err(|e| ErreurMoteur::Fichier(format!("{PROFIL} : {e}")))?;
        let mut journal = String::new();
        let sortie = self.lancer(
            "iccgamut",
            &arguments_iccgamut(),
            chemin,
            Etape::Gamut,
            &mut journal,
            &mut |_| {},
        )?;
        let octets = fs::read(chemin.join(GAM))
            .map_err(|e| ErreurMoteur::Fichier(format!("{GAM} : {e}")))?;
        Ok(LimiteCalculee {
            gam: String::from_utf8_lossy(&octets).into_owned(),
            volume: lire_volume_iccgamut(&sortie),
            version,
        })
    }
}

/// Moteur des tests : rend toujours la même limite, ou la même erreur. Le
/// volume n'est pas inventé : il reste inconnu.
pub struct MoteurGamutSimule {
    resultat: Result<String, ErreurMoteur>,
}

impl MoteurGamutSimule {
    /// Un moteur qui rend ce texte `.gam`.
    pub fn reussit(gam: String) -> Self {
        MoteurGamutSimule { resultat: Ok(gam) }
    }

    /// Un moteur qui échoue toujours ainsi.
    pub fn echoue(erreur: ErreurMoteur) -> Self {
        MoteurGamutSimule {
            resultat: Err(erreur),
        }
    }
}

impl MoteurGamut for MoteurGamutSimule {
    fn commande_limite(&self) -> String {
        ligne("iccgamut", &arguments_iccgamut())
    }

    fn limite(&self, _icc: &[u8]) -> Result<LimiteCalculee, ErreurMoteur> {
        Ok(LimiteCalculee {
            gam: self.resultat.clone()?,
            volume: None,
            version: VERSION_ARGYLL.to_string(),
        })
    }
}
