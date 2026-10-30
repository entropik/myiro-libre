//! Banc commun des tests de préparation de mires (`mires.rs`, `mires_fd9.rs`) :
//! bibliothèque temporaire, `.ti1` fictifs et mire FD-9 fictive de même forme
//! que RGB-2520.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::mires::DemandeMire;
use app::textes::Langue;
use bibliotheque::IdCondition;
use mires::{
    ecrire_tiff, lire_mire_fd9_sans_empreinte, Format, GeometrieFd9, ImageRvb, Instrument, MireFd9,
    TailleMire,
};
use profilage::MoteurSimule;
use tempfile::TempDir;

/// Le 23 octobre 2026 à 9 h 15, UTC.
pub fn maintenant() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_746_900)
}

/// `.ti1` fictif de `n` plages : les premières valeurs d'un cube RVB 8 × 8 × 8.
pub fn ti1(n: usize) -> String {
    let niveau = |i: usize| i as f64 * 100.0 / 7.0;
    let mut lignes = String::new();
    for k in 0..n {
        let (r, g, b) = (k / 64 % 8, k / 8 % 8, k % 8);
        lignes.push_str(&format!(
            "{} {:.4} {:.4} {:.4} 0 0 0\n",
            k + 1,
            niveau(r),
            niveau(g),
            niveau(b)
        ));
    }
    format!(
        "CTI1\n\nCOLOR_REP \"iRGB\"\n\nNUMBER_OF_FIELDS 7\nBEGIN_DATA_FORMAT\n\
         SAMPLE_ID RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z\nEND_DATA_FORMAT\n\n\
         NUMBER_OF_SETS {n}\nBEGIN_DATA\n{lignes}END_DATA\n"
    )
}

/// `.ti1` de plages toutes proches du blanc papier.
pub fn ti1_clair(n: usize) -> String {
    let lignes: String = (0..n)
        .map(|k| format!("{} {} 100 {} 0 0 0\n", k + 1, 95 + k % 6, 96 + k % 5))
        .collect();
    format!(
        "CTI1\n\nBEGIN_DATA_FORMAT\nSAMPLE_ID RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z\n\
         END_DATA_FORMAT\n\nBEGIN_DATA\n{lignes}END_DATA\n"
    )
}

pub struct Banc {
    pub dossier: TempDir,
    pub ouverte: BibliothequeOuverte,
    pub condition: IdCondition,
}

/// Canon PRO-4100 → Hahnemühle Photo Rag 308 → 720 dpi.
pub fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        dossier,
        ouverte,
        condition,
    }
}

impl Banc {
    pub fn mires(&self) -> PathBuf {
        self.dossier.path().join("Mires")
    }

    pub fn fichiers(&self) -> Vec<String> {
        let mut noms: Vec<String> = std::fs::read_dir(self.mires())
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        noms.sort();
        noms
    }

    pub fn tirages(&self) -> usize {
        self.ouverte
            .detail_reglage(self.condition)
            .unwrap()
            .tirages
            .len()
    }

    /// Prépare la mire, texte imprimé en français.
    pub fn preparer(
        &self,
        demande: &DemandeMire,
        moteur: &MoteurSimule,
        mire_fd9: Option<&Path>,
    ) -> Result<app::mires::MirePreparee, String> {
        self.preparer_dans(Langue::Francais, demande, moteur, mire_fd9)
    }

    /// Prépare la mire, texte imprimé dans `langue`.
    pub fn preparer_dans(
        &self,
        langue: Langue,
        demande: &DemandeMire,
        moteur: &MoteurSimule,
        mire_fd9: Option<&Path>,
    ) -> Result<app::mires::MirePreparee, String> {
        self.ouverte.preparer_mire(
            self.condition,
            demande,
            moteur,
            mire_fd9,
            &self.mires(),
            maintenant(),
            langue,
        )
    }
}

pub fn myiro1(taille: TailleMire) -> DemandeMire {
    DemandeMire {
        instrument: Instrument::Myiro1,
        taille: Some(taille),
        nombre: None,
        format: Some(Format::A4),
        cote_mm: None,
    }
}

pub fn fd9() -> DemandeMire {
    DemandeMire {
        instrument: Instrument::Fd9,
        taille: None,
        nombre: None,
        format: None,
        cote_mm: None,
    }
}

pub fn moteur(n: usize) -> MoteurSimule {
    MoteurSimule::reussit(Vec::new()).avec_plages(ti1(n))
}

/// Mire FD-9 fictive de même forme que RGB-2520 : 6 × 4 plages de 5 pixels,
/// origine (3, 3), écrite en TIFF.
pub fn mire_fd9_fictive() -> (Vec<u8>, GeometrieFd9) {
    let g = GeometrieFd9 {
        largeur_px: 36,
        hauteur_px: 26,
        origine_px: (3, 3),
        pas_px: 5,
        colonnes: 6,
        rangees: 4,
    };
    let mut pixels = Vec::new();
    for y in 0..g.hauteur_px {
        for x in 0..g.largeur_px {
            let dans = (3..33).contains(&x) && (3..23).contains(&y);
            let k = ((y.saturating_sub(3)) / 5 * 6 + x.saturating_sub(3) / 5) as u8;
            pixels.extend(if dans {
                [k * 10, 200 - k, k * 7]
            } else {
                [255; 3]
            });
        }
    }
    let mut octets = Vec::new();
    ecrire_tiff(
        &ImageRvb {
            largeur: g.largeur_px,
            hauteur: g.hauteur_px,
            pixels,
        },
        &mut octets,
    )
    .unwrap();
    (octets, g)
}

pub fn lire_fictive() -> (Vec<u8>, MireFd9) {
    let (octets, g) = mire_fd9_fictive();
    let mire = lire_mire_fd9_sans_empreinte(octets.clone(), &g).unwrap();
    (octets, mire)
}
