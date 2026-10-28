//! Banc commun aux essais de la vérification (`verifier.rs`, `rapport.rs`) :
//! une résolution, un profil calculé par le moteur simulé, le jeu
//! « Atelier » et des exports FD-S2w fictifs.
#![allow(dead_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::verifier::{DemandeVerification, FicheTirage};
use bibliotheque::{EtatVerification, IdCondition, IdMesure, IdProfil, IdTolerances};
use pont_protocole::ConditionMesure;
use profilage::{Impression, MoteurSimule, Recette};
use tempfile::TempDir;
use verification::{Criteres, Seuils};

pub fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

/// Le 28 octobre 2026 à 9 h 00, UTC, plus `minutes`.
pub fn a(minutes: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_793_178_000 + 60 * minutes)
}

/// Une plage : RVB sur 255 et le Lab que le profil simulé fidèle prédit
/// (L = 20 + 75 (r + g + b) / 3, a = 40 (r − g), b = 40 (g − b)), L
/// décalé de `decalage`.
pub fn ligne(id: usize, rvb: [f64; 3], decalage: f64) -> String {
    let [r, g, b] = rvb.map(|v| v / 255.0);
    let l = 20.0 + 75.0 * (r + g + b) / 3.0 + decalage;
    let (a, bb) = (40.0 * (r - g), 40.0 * (g - b));
    format!(
        "{id}\t{}\t{}\t{}\t{l:.4}\t{a:.4}\t{bb:.4}",
        rvb[0], rvb[1], rvb[2]
    )
}

pub const MIRE_PROFIL: [[f64; 3]; 6] = [
    [255.0, 255.0, 255.0],
    [0.0, 0.0, 0.0],
    [255.0, 0.0, 0.0],
    [0.0, 255.0, 0.0],
    [0.0, 0.0, 255.0],
    [127.5, 127.5, 127.5],
];

pub const SECONDE_MIRE: [[f64; 3]; 6] = [
    [255.0, 255.0, 255.0],
    [51.0, 51.0, 51.0],
    [204.0, 102.0, 51.0],
    [51.0, 153.0, 102.0],
    [102.0, 51.0, 204.0],
    [178.5, 178.5, 178.5],
];

/// Export fictif à la manière de FD-S2w, en M1 (source D50).
pub fn export(rvb: &[[f64; 3]], decalage: f64) -> String {
    let donnees: String = rvb
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}\r\n", ligne(i + 1, *c, decalage)))
        .collect();
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n\
         INSTRUMENTATION\t\"FD-9\"\r\nMEASUREMENT_SOURCE\t\"D50\"\r\nBEGIN_DATA_FORMAT\r\n\
         SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n{donnees}END_DATA\r\n",
        rvb.len()
    )
}

pub fn atelier() -> Criteres {
    Criteres {
        fidelite: Seuils {
            moyenne: Some(2.0),
            p95: Some(4.0),
            max: Some(6.0),
        },
        prediction: Seuils {
            moyenne: Some(1.5),
            p95: Some(3.0),
            max: Some(5.0),
        },
        seuil_gamut: 1.0,
    }
}

pub fn fiche(impression: Impression) -> FicheTirage {
    FicheTirage {
        impression,
        logiciel: "Logiciel de l'atelier".into(),
        support_pilote: "Premium Semigloss".into(),
        qualite: "1440 dpi".into(),
        encres: "UltraChrome Pro".into(),
        remarques: String::new(),
        confirmee: true,
    }
}

pub struct Banc {
    pub dossier: TempDir,
    pub ouverte: BibliothequeOuverte,
    pub condition: IdCondition,
    pub mesure_profil: IdMesure,
    pub profil: IdProfil,
    pub atelier: IdTolerances,
}

/// Epson SC-P5300 → Semigloss → 1440 dpi, un profil calculé (moteur
/// simulé) depuis la mire du profil, et le jeu « Atelier ».
pub fn banc() -> Banc {
    banc_papier("Semigloss 250")
}

/// Le même banc, avec ce nom de papier.
pub fn banc_papier(papier: &str) -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let epson = ouverte.ajouter_imprimante("Epson SC-P5300").unwrap();
    let condition = ouverte
        .ajouter_papier(epson.id, papier, "1440 dpi")
        .unwrap()
        .id;
    let mut banc = Banc {
        dossier,
        ouverte,
        condition,
        mesure_profil: IdMesure(0),
        profil: IdProfil(0),
        atelier: IdTolerances(0),
    };
    banc.mesure_profil = banc.importer("mire-profil_M1.txt", &export(&MIRE_PROFIL, 0.0));
    banc.profil = banc.calculer(banc.mesure_profil, a(0));
    banc.atelier = banc
        .ouverte
        .enregistrer_tolerances("Atelier", &atelier(), a(1))
        .unwrap();
    banc
}

impl Banc {
    pub fn importer(&self, nom: &str, texte: &str) -> IdMesure {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, texte).unwrap();
        let apercu = self.ouverte.apercu_cgats(&chemin).unwrap();
        self.ouverte
            .ranger_apercu(apercu.numero, self.condition)
            .unwrap()
    }

    pub fn calculer(&self, mesure: IdMesure, quand: SystemTime) -> IdProfil {
        self.ouverte
            .calculer_profil(
                self.condition,
                mesure,
                ConditionMesure::M1,
                Recette::Standard,
                &MoteurSimule::reussit(icc()),
                quand,
                &mut |_| {},
            )
            .unwrap()
            .profil
    }

    pub fn demande(&self, mesure: IdMesure, impression: Impression) -> DemandeVerification {
        DemandeVerification {
            profil: self.profil,
            mesure,
            mesure_condition: ConditionMesure::M1,
            tolerances: self.atelier,
            fiche: fiche(impression),
        }
    }

    pub fn etat(&self, profil: IdProfil) -> EtatVerification {
        self.ouverte
            .detail_reglage(self.condition)
            .unwrap()
            .historique
            .iter()
            .find(|p| p.id == profil)
            .unwrap()
            .etat
    }
}
