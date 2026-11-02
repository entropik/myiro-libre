//! Réglage « Atelier » (ticket #89) : les coordonnées de l'atelier, son logo
//! et le numéro du dernier rapport, remplis une fois et repris sur chaque
//! rapport. Ils vivent dans `<données de l'application>/atelier`, à côté de
//! la bibliothèque mais hors d'elle : aucune migration, et la sauvegarde de
//! la bibliothèque ne les emporte pas (ADR 0008).
//!
//! Fichiers : `atelier.json` (coordonnées), `logo.png` (logo réduit et
//! réencodé), `rapports.json` (dernier numéro pris, par année). Chacun
//! s'écrit par un temporaire renommé.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};

use crate::ecriture::ecrire_atomique;

pub mod commandes;

/// Plus grand côté du logo gardé, en pixels.
pub const COTE_MAX_LOGO: u32 = 600;
/// Taille d'un fichier de logo lu au plus.
pub const OCTETS_MAX_LOGO: u64 = 64 << 20;
/// Plus grand côté d'un logo décodé (garde contre une image piégée).
const COTE_MAX_DECODE: u32 = 20_000;

const ATELIER: &str = "atelier.json";
const LOGO: &str = "logo.png";
const RAPPORTS: &str = "rapports.json";

/// Les coordonnées saisies dans le formulaire « Atelier ».
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AtelierSaisi {
    pub nom: String,
    pub adresse: String,
    pub courriel: String,
    pub telephone: String,
    pub site: String,
}

impl AtelierSaisi {
    /// Le premier champ trop long, en caractères : nom 120, adresse 400,
    /// autres 200.
    fn champ_trop_long(&self) -> Option<&'static str> {
        [
            ("nom", &self.nom, 120),
            ("adresse", &self.adresse, 400),
            ("courriel", &self.courriel, 200),
            ("telephone", &self.telephone, 200),
            ("site", &self.site, 200),
        ]
        .into_iter()
        .find(|(_, v, max)| v.chars().count() > *max)
        .map(|(champ, _, _)| champ)
    }
}

/// Pourquoi le réglage n'a pas pu être lu ou écrit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurAtelier {
    /// Le logo n'est ni un PNG ni un JPEG (un SVG, par exemple).
    LogoFormat,
    /// Le logo est un PNG ou un JPEG abîmé, ou trop grand.
    LogoIllisible,
    /// Le champ nommé dépasse sa longueur.
    TropLong(&'static str),
    Ecriture(String),
    Lecture(String),
}

impl ErreurAtelier {
    /// Clé du catalogue de textes.
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurAtelier::LogoFormat => "atelier.erreur.logo_format",
            ErreurAtelier::LogoIllisible => "atelier.erreur.logo_illisible",
            ErreurAtelier::TropLong(_) => "atelier.erreur.trop_long",
            ErreurAtelier::Ecriture(_) => "atelier.erreur.ecriture",
            ErreurAtelier::Lecture(_) => "atelier.erreur.lecture",
        }
    }
}

fn ecriture(e: std::io::Error) -> ErreurAtelier {
    ErreurAtelier::Ecriture(e.to_string())
}

fn lecture(e: impl std::fmt::Display) -> ErreurAtelier {
    ErreurAtelier::Lecture(e.to_string())
}

/// Le logo réduit à `COTE_MAX_LOGO` pixels de côté au plus, réencodé en
/// PNG. Seuls PNG et JPEG sont acceptés, reconnus par leur contenu.
fn preparer_logo(octets: &[u8]) -> Result<Vec<u8>, ErreurAtelier> {
    let format = image::guess_format(octets).map_err(|_| ErreurAtelier::LogoFormat)?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err(ErreurAtelier::LogoFormat);
    }
    let mut lecteur = ImageReader::with_format(Cursor::new(octets), format);
    let mut limites = Limits::default();
    limites.max_image_width = Some(COTE_MAX_DECODE);
    limites.max_image_height = Some(COTE_MAX_DECODE);
    limites.max_alloc = Some(512 << 20);
    lecteur.limits(limites);
    let mut logo = lecteur.decode().map_err(|_| ErreurAtelier::LogoIllisible)?;
    if logo.width().max(logo.height()) > COTE_MAX_LOGO {
        logo = logo.resize(COTE_MAX_LOGO, COTE_MAX_LOGO, FilterType::Triangle);
    }
    let mut png = Vec::new();
    logo.to_rgba8()
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|_| ErreurAtelier::LogoIllisible)?;
    Ok(png)
}

/// Le réglage « Atelier », dans son dossier.
pub struct ReglageAtelier {
    dossier: PathBuf,
}

impl ReglageAtelier {
    pub fn new(dossier: PathBuf) -> Self {
        Self { dossier }
    }

    pub fn dossier(&self) -> &Path {
        &self.dossier
    }

    fn ecrire(&self, nom: &str, octets: &[u8]) -> Result<(), ErreurAtelier> {
        std::fs::create_dir_all(&self.dossier).map_err(ecriture)?;
        ecrire_atomique(&self.dossier.join(nom), octets).map_err(ecriture)
    }

    /// Le contenu d'un fichier du réglage ; `None` s'il n'existe pas.
    fn lire_fichier(&self, nom: &str) -> Result<Option<Vec<u8>>, ErreurAtelier> {
        match std::fs::read(self.dossier.join(nom)) {
            Ok(octets) => Ok(Some(octets)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(lecture(e)),
        }
    }

    /// Les coordonnées saisies ; vides si rien n'a été enregistré.
    pub fn saisi(&self) -> Result<AtelierSaisi, ErreurAtelier> {
        match self.lire_fichier(ATELIER)? {
            Some(octets) => serde_json::from_slice(&octets).map_err(lecture),
            None => Ok(AtelierSaisi::default()),
        }
    }

    pub fn a_un_logo(&self) -> bool {
        self.dossier.join(LOGO).is_file()
    }

    /// L'atelier tel qu'il part dans un rapport, logo compris.
    pub fn lire(&self) -> Result<rapport::Atelier, ErreurAtelier> {
        let s = self.saisi()?;
        Ok(rapport::Atelier {
            nom: s.nom,
            adresse: s.adresse,
            courriel: s.courriel,
            telephone: s.telephone,
            site: s.site,
            logo_png: self.lire_fichier(LOGO)?,
        })
    }

    /// Enregistre les coordonnées, après contrôle des longueurs.
    pub fn enregistrer(&self, a: &AtelierSaisi) -> Result<(), ErreurAtelier> {
        if let Some(champ) = a.champ_trop_long() {
            return Err(ErreurAtelier::TropLong(champ));
        }
        let json =
            serde_json::to_vec_pretty(a).map_err(|e| ErreurAtelier::Ecriture(e.to_string()))?;
        self.ecrire(ATELIER, &json)
    }

    /// Lit le fichier choisi, le contrôle, le réduit et le garde en
    /// `logo.png`. Un logo refusé laisse l'ancien en place.
    pub fn poser_logo(&self, fichier: &Path) -> Result<(), ErreurAtelier> {
        let mut octets = Vec::new();
        std::fs::File::open(fichier)
            .and_then(|f| f.take(OCTETS_MAX_LOGO + 1).read_to_end(&mut octets))
            .map_err(lecture)?;
        if octets.len() as u64 > OCTETS_MAX_LOGO {
            return Err(ErreurAtelier::LogoIllisible);
        }
        let png = preparer_logo(&octets)?;
        self.ecrire(LOGO, &png)
    }

    pub fn retirer_logo(&self) -> Result<(), ErreurAtelier> {
        match std::fs::remove_file(self.dossier.join(LOGO)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(ecriture(e)),
            _ => Ok(()),
        }
    }

    fn numeros(&self) -> Result<BTreeMap<String, u32>, ErreurAtelier> {
        match self.lire_fichier(RAPPORTS)? {
            Some(octets) => serde_json::from_slice(&octets).map_err(lecture),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Le numéro que prendrait le prochain rapport de l'année, sans le
    /// prendre : `2026-0001`, puis `2026-0002`…
    pub fn prochain_numero(&self, annee: i32) -> Result<String, ErreurAtelier> {
        let dernier = self
            .numeros()?
            .get(&annee.to_string())
            .copied()
            .unwrap_or(0);
        Ok(format!("{annee}-{:04}", dernier + 1))
    }

    /// Prend le prochain numéro de l'année et le retient. Appelé seulement
    /// une fois le PDF écrit.
    pub fn prendre_numero(&self, annee: i32) -> Result<String, ErreurAtelier> {
        let mut numeros = self.numeros()?;
        let dernier = numeros.entry(annee.to_string()).or_insert(0);
        *dernier += 1;
        let numero = format!("{annee}-{:04}", *dernier);
        let json = serde_json::to_vec_pretty(&numeros)
            .map_err(|e| ErreurAtelier::Ecriture(e.to_string()))?;
        self.ecrire(RAPPORTS, &json)?;
        Ok(numero)
    }
}
