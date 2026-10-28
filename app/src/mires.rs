//! Préparer une mire à imprimer dans la vue d'une résolution (ticket #33).
//!
//! - **FD-9** : la mire RGB-2520, reprise telle quelle, est copiée octet pour
//!   octet dans le dossier des mires ;
//! - **MYIRO-1** : `targen` choisit les plages, la crate `mires` les place,
//!   dessine les pages et les écrit en TIFF.
//!
//! La mire et son tirage sont rangés dans la bibliothèque. Les méthodes de
//! [`BibliothequeOuverte`] se testent sans Tauri, avec le moteur simulé de la
//! crate `profilage`. `targen` tourne hors du verrou de la bibliothèque. Les
//! erreurs rendues sont des clés du catalogue de textes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bibliotheque::{IdCondition, NouvelleMire};
use mires::{
    lire_mire_fd9, ErreurMire, Format, Gabarit, Instrument, MireFd9, TailleMire, RGB2520_FICHIER,
    RGB2520_NOM,
};
use profilage::MoteurProfil;
use serde::{Deserialize, Serialize};

use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;

pub mod commandes;
mod depot;
mod myiro1;

use depot::ecrire_fichier;
use myiro1::controler_myiro1;

/// Bornes du nombre libre de plages MYIRO-1 (section « Avancé ») : des
/// bornes de saisie, pas un avis sur la qualité.
pub const NOMBRE_LIBRE: (usize, usize) = (24, 3000);
/// Dossier de la mire FD-9, à côté de l'application.
pub const DOSSIER_MIRE_FD9: &str = "mire-fd9";
/// Limites du côté des plages du FD-9 (manuel de FD-S2w, p. 35 à 37).
const COTE_FD9_MM: (f64, f64) = (6.0, 30.0);

/// Ce que l'imprimeur demande. Pour le FD-9, rien d'autre que l'instrument :
/// sa mire est RGB-2520.
#[derive(Clone, Debug, Deserialize)]
pub struct DemandeMire {
    pub instrument: Instrument,
    pub taille: Option<TailleMire>,
    /// Section « Avancé » : remplace la taille.
    pub nombre: Option<usize>,
    pub format: Option<Format>,
    /// Section « Avancé ».
    pub cote_mm: Option<f64>,
}

/// Ce que la section montre avant de préparer, sans rien lancer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApercuMire {
    pub plages: usize,
    pub pages: usize,
    pub cote_mm: f64,
    pub limites_cote: (f64, Option<f64>),
    /// Commande `targen` équivalente ; vide pour le FD-9.
    pub commande: String,
    /// Mire FD-9 absente : le fichier attendu, pour le dire avant le clic.
    pub mire_fd9_attendue: Option<PathBuf>,
}

/// Ce que la page reçoit après une préparation réussie.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MirePreparee {
    pub tirage: i64,
    pub fichiers: Vec<PathBuf>,
    pub dossier: PathBuf,
    pub plages: usize,
    pub pages: usize,
    pub commande: String,
}

/// Cherche `mire-fd9/RGB-2520 A3 - 1p.tif` à côté de l'exécutable (dans
/// `dossier_exe`), puis deux niveaux plus haut (racine du dépôt depuis
/// `target/debug`), comme `chercher_argyll`.
pub fn chercher_mire_fd9(dossier_exe: &Path) -> Option<PathBuf> {
    [
        dossier_exe.join(DOSSIER_MIRE_FD9),
        dossier_exe.join("..").join("..").join(DOSSIER_MIRE_FD9),
    ]
    .into_iter()
    .map(|d| d.join(RGB2520_FICHIER))
    .find(|f| f.is_file())
}

/// Nom de fichier d'une page : « Canon PRO-4100 - Baryta - 600 dpi -
/// 2026-10-23 - T42 - page 1 sur 2.tif ». Comme pour les profils
/// (`nom_profil_propose`), les caractères interdits par Windows, les
/// guillemets « » et les caractères de contrôle sont retirés ; les accents
/// restent, et les espaces en trop sont réduits.
pub fn nom_fichier_page(
    imprimante: &str,
    papier: &str,
    resolution: &str,
    date: &str,
    tirage: i64,
    page: usize,
    pages: usize,
) -> String {
    let partie = |texte: &str| {
        let propre: String = texte
            .chars()
            .filter(|c| !"<>:\"/\\|?*«»".contains(*c) && !c.is_control())
            .collect();
        propre.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let parties: Vec<String> = [imprimante, papier, resolution, date]
        .into_iter()
        .map(partie)
        .filter(|p| !p.is_empty())
        .collect();
    format!(
        "{} - T{tirage} - page {page} sur {pages}.tif",
        parties.join(" - ")
    )
}

/// La mire FD-9 est RGB-2520 : ni taille, ni nombre, ni format, ni côté.
fn controler_fd9(demande: &DemandeMire) -> Result<(), String> {
    let fixe = demande.taille.is_none()
        && demande.nombre.is_none()
        && demande.format.is_none()
        && demande.cote_mm.is_none();
    fixe.then_some(())
        .ok_or_else(|| "mire.erreur.fd9_fixe".to_string())
}

impl BibliothequeOuverte {
    /// Plages, pages, côté et commande de la mire demandée, sans rien lancer.
    pub fn apercu_mire(
        &self,
        demande: &DemandeMire,
        moteur: &dyn MoteurProfil,
    ) -> Result<ApercuMire, String> {
        match demande.instrument {
            Instrument::Fd9 => {
                controler_fd9(demande)?;
                let g = mires::RGB2520_GEOMETRIE;
                Ok(ApercuMire {
                    plages: g.colonnes * g.rangees,
                    pages: 1,
                    cote_mm: (g.pas_px as f64 * 25.4 / mires::RESOLUTION_PPP as f64 * 100.0)
                        .round()
                        / 100.0,
                    limites_cote: (COTE_FD9_MM.0, Some(COTE_FD9_MM.1)),
                    commande: String::new(),
                    mire_fd9_attendue: None,
                })
            }
            Instrument::Myiro1 => {
                let d = controler_myiro1(demande)?;
                Ok(ApercuMire {
                    plages: d.nombre,
                    pages: d.gabarit.pages(d.nombre),
                    cote_mm: d.gabarit.cote_mm,
                    limites_cote: Gabarit::limites_cote(),
                    commande: moteur.commande_plages(d.nombre),
                    mire_fd9_attendue: None,
                })
            }
        }
    }

    /// Prépare la mire demandée, range la mire et son tirage, et écrit ses
    /// pages dans `dossier`. Rien n'est rangé si la mire ne peut pas être
    /// préparée, ni si une page ne peut pas être écrite : un tirage
    /// n'apparaît jamais sans ses fichiers (voir `ranger_et_deposer`).
    pub fn preparer_mire(
        &self,
        condition: IdCondition,
        demande: &DemandeMire,
        moteur: &dyn MoteurProfil,
        mire_fd9: Option<&Path>,
        dossier: &Path,
        maintenant: SystemTime,
    ) -> Result<MirePreparee, String> {
        match demande.instrument {
            Instrument::Fd9 => {
                controler_fd9(demande)?;
                self.detail_reglage(condition)?;
                let fichier = mire_fd9.ok_or_else(|| {
                    eprintln!("mire RGB-2520 introuvable ({DOSSIER_MIRE_FD9}/{RGB2520_FICHIER})");
                    ErreurMire::MireFd9Introuvable.cle().to_string()
                })?;
                let octets = fs::read(fichier).map_err(|e| {
                    eprintln!("{} : {e}", fichier.display());
                    ErreurMire::MireFd9Illisible(e.to_string())
                        .cle()
                        .to_string()
                })?;
                let mire = lire_mire_fd9(octets).map_err(|e| {
                    eprintln!("{} : {e}", fichier.display());
                    e.cle().to_string()
                })?;
                self.preparer_mire_fd9(condition, mire, dossier, maintenant)
            }
            Instrument::Myiro1 => {
                self.preparer_myiro1(condition, demande, moteur, dossier, maintenant)
            }
        }
    }

    /// Range une mire FD-9 déjà lue et vérifiée, et la copie telle quelle
    /// dans `dossier`, sans texte ni QR code ajoutés : elle garde les siens.
    pub fn preparer_mire_fd9(
        &self,
        condition: IdCondition,
        mire: MireFd9,
        dossier: &Path,
        maintenant: SystemTime,
    ) -> Result<MirePreparee, String> {
        let detail = self.detail_reglage(condition)?;
        let horodatage = instant_utc(maintenant);
        let json = |v: serde_json::Result<String>| {
            v.map_err(|e| {
                eprintln!("{e}");
                ErreurMire::Ecriture(e.to_string()).cle().to_string()
            })
        };
        let generation = json(serde_json::to_string(&serde_json::json!({
            "origine": "reprise",
            "fichier": RGB2520_FICHIER,
            "nom": RGB2520_NOM,
        })))?;
        let valeurs = json(serde_json::to_string(&mire.valeurs))?;
        let geometrie = json(serde_json::to_string(&mire.geometrie))?;
        let date = horodatage.get(..10).unwrap_or(&horodatage);
        let nouvelle = NouvelleMire {
            nom: "RGB-2520",
            instrument: "fd9",
            plages: mire.valeurs.len(),
            generation: &generation,
            valeurs: &valeurs,
            mise_en_page: &geometrie,
            empreinte: &mire.empreinte,
            horodatage: &horodatage,
        };
        let (tirage, fichiers) =
            self.ranger_et_deposer(condition, &nouvelle, 1, dossier, |tirage, travail| {
                let nom = nom_fichier_page(
                    &detail.imprimante,
                    &detail.papier,
                    &detail.resolution,
                    date,
                    tirage.0,
                    1,
                    1,
                );
                ecrire_fichier(&travail.join(&nom), |f| {
                    std::io::Write::write_all(f, &mire.octets)
                        .map_err(|e| ErreurMire::Ecriture(e.to_string()))
                })?;
                Ok(vec![nom])
            })?;
        Ok(MirePreparee {
            tirage: tirage.0,
            fichiers,
            dossier: dossier.to_path_buf(),
            plages: mire.valeurs.len(),
            pages: 1,
            commande: String::new(),
        })
    }
}
