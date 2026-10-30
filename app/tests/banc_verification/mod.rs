//! Banc commun aux essais de la vérification (`verifier.rs`, `rapport_verification.rs`) :
//! une résolution, un profil calculé par le moteur simulé, le jeu
//! « Atelier », la mire de vérification FD-9 rangée et des exports FD-S2w
//! fictifs (numéros de série fictifs).
#![allow(dead_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::verifier::{DemandeVerification, FicheTirage};
use bibliotheque::{EtatVerification, IdCondition, IdMesure, IdProfil, IdTolerances};
use pont_protocole::ConditionMesure;
use profilage::{Impression, MoteurSimule, Recette};
use tempfile::TempDir;
use verification::{lire_xml_fds2w, Criteres, MireFd9, Seuils};

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

/// Le Lab que le profil simulé fidèle prédit pour des RVB sur 255
/// (L = 20 + 75 (r + g + b) / 3, a = 40 (r − g), b = 40 (g − b)), L
/// décalé de `decalage`.
pub fn lab_fidele(rvb: [f64; 3], decalage: f64) -> [f64; 3] {
    let [r, g, b] = rvb.map(|v| v / 255.0);
    [
        20.0 + 75.0 * (r + g + b) / 3.0 + decalage,
        40.0 * (r - g),
        40.0 * (g - b),
    ]
}

/// Une ligne de données : identifiant, RVB sur 255 et Lab.
pub fn ligne_lab(id: &str, rvb: [f64; 3], lab: [f64; 3]) -> String {
    format!(
        "{id}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}",
        rvb[0], rvb[1], rvb[2], lab[0], lab[1], lab[2]
    )
}

pub fn ligne(id: usize, rvb: [f64; 3], decalage: f64) -> String {
    ligne_lab(&id.to_string(), rvb, lab_fidele(rvb, decalage))
}

pub const MIRE_PROFIL: [[f64; 3]; 6] = [
    [255.0, 255.0, 255.0],
    [0.0, 0.0, 0.0],
    [255.0, 0.0, 0.0],
    [0.0, 255.0, 0.0],
    [0.0, 0.0, 255.0],
    [127.5, 127.5, 127.5],
];

/// Ligne `INSTRUMENTATION` d'un export MYIROtools du FD-9, numéro fictif.
pub const INSTRUMENT_AVEC_SERIE: &str = "KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678";

/// Export fictif à la manière de FD-S2w, en M1 (source D50), avec ces
/// lignes de données et cette ligne `INSTRUMENTATION`.
pub fn export_lignes(lignes: &[String], instrument: &str) -> String {
    let donnees: String = lignes.iter().map(|l| format!("{l}\r\n")).collect();
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n\
         INSTRUMENTATION\t\"{instrument}\"\r\nMEASUREMENT_SOURCE\t\"D50\"\r\nBEGIN_DATA_FORMAT\r\n\
         SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n{donnees}END_DATA\r\n",
        lignes.len()
    )
}

pub fn export(rvb: &[[f64; 3]], decalage: f64) -> String {
    let lignes: Vec<String> = rvb
        .iter()
        .enumerate()
        .map(|(i, c)| ligne(i + 1, *c, decalage))
        .collect();
    export_lignes(&lignes, "FD-9")
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
    /// La mire de vérification FD-9, telle que la bibliothèque l'a rangée.
    pub mire: MireFd9,
}

/// Epson SC-P5300 → Semigloss → 1440 dpi, un profil calculé (moteur
/// simulé) depuis la mire du profil, le jeu « Atelier » et la mire de
/// vérification FD-9 rangée.
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
    let rangee = ouverte.mire_verification_fd9(a(0)).unwrap();
    let mut banc = Banc {
        dossier,
        ouverte,
        condition,
        mesure_profil: IdMesure(0),
        profil: IdProfil(0),
        atelier: IdTolerances(0),
        mire: lire_xml_fds2w(&rangee.contenu).unwrap(),
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

    /// Lignes de la mire de vérification mesurée : ses ID, ses RVB, et le
    /// Lab que donne `lab` pour chaque RVB (sur 255).
    pub fn lignes_mire(&self, lab: impl Fn([f64; 3]) -> [f64; 3]) -> Vec<String> {
        self.mire
            .plages
            .iter()
            .map(|p| {
                let rvb = p.rvb.map(f64::from);
                ligne_lab(&p.id.to_string(), rvb, lab(rvb))
            })
            .collect()
    }

    /// Export de la mire de vérification FD-9 mesurée, Lab du profil fidèle
    /// décalé de `decalage` en L.
    pub fn seconde(&self, decalage: f64) -> String {
        export_lignes(&self.lignes_mire(|c| lab_fidele(c, decalage)), "FD-9")
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
