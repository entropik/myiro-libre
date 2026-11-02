//! Exports synthétiques à la manière de FD-S2w (numéro de série fictif),
//! et plan de trois plages qui leur correspond.

#![allow(dead_code)]

use cgats::MesureImportee;
use seance_mesure::{IdentiteMire, Plan};

/// RVB de la mire de test, de 0 à 100.
pub const VALEURS: [[f64; 3]; 3] = [[100.0, 100.0, 100.0], [0.0, 50.0, 25.0], [20.0, 40.0, 60.0]];

pub const EMPREINTE: &str = "empreinte-fictive";

pub fn plan() -> Plan {
    Plan::figer(
        IdentiteMire {
            id: 7,
            nom: "Mire de test".into(),
            empreinte: EMPREINTE.into(),
        },
        &VALEURS,
        r#"{"confirmee":true}"#,
        true,
    )
    .unwrap()
}

/// Une ligne d'un export : identifiant, RVB sur 255, Lab, réflectance
/// constante du spectre.
#[derive(Clone, Debug)]
pub struct Ligne {
    pub id: String,
    pub rvb: [f64; 3],
    pub lab: [f32; 3],
    pub reflectance: f32,
}

/// Un export de FD-S2w fictif.
#[derive(Clone, Debug)]
pub struct Export {
    pub instrument: String,
    pub serie: String,
    /// `MEASUREMENT_SOURCE` : « D50 » donne la condition M1 supposée.
    pub source: String,
    pub rvb: bool,
    pub lab: bool,
    pub spectres: bool,
    /// Premier nm du spectre (380 en temps normal).
    pub debut_nm: u32,
    pub lignes: Vec<Ligne>,
}

/// Les trois plages de [`VALEURS`], RVB sur 255, Lab décalé de `decalage`
/// sur L.
pub fn lignes(decalage: f32) -> Vec<Ligne> {
    VALEURS
        .iter()
        .enumerate()
        .map(|(i, v)| Ligne {
            id: (i + 1).to_string(),
            rvb: v.map(|c| c * 2.55),
            lab: [90.0 - 30.0 * i as f32 + decalage, 1.0, -2.0],
            reflectance: 0.8 - 0.2 * i as f32,
        })
        .collect()
}

impl Export {
    pub fn fd9(lignes: Vec<Ligne>) -> Export {
        Export {
            instrument: "KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678".into(),
            serie: "12345678".into(),
            source: "D50".into(),
            rvb: true,
            lab: true,
            spectres: true,
            debut_nm: 380,
            lignes,
        }
    }

    pub fn texte(&self) -> String {
        let mut champs = vec!["SAMPLE_ID".to_string()];
        if self.rvb {
            champs.extend(["RGB_R", "RGB_G", "RGB_B"].map(String::from));
        }
        if self.lab {
            champs.extend(["LAB_L", "LAB_A", "LAB_B"].map(String::from));
        }
        if self.spectres {
            champs.extend((0..36).map(|i| format!("nm{}", self.debut_nm + 10 * i)));
        }
        let mut t = String::from("CGATS.17\r\n");
        t += "ORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n";
        t += &format!("INSTRUMENTATION\t\"{}\"\r\n", self.instrument);
        t += &format!("SERIAL\t\"{}\"\r\n", self.serie);
        t += &format!("MEASUREMENT_SOURCE\t\"{}\"\r\n", self.source);
        t += &format!("NUMBER_OF_FIELDS\t{}\r\n", champs.len());
        t += "BEGIN_DATA_FORMAT\r\n";
        t += &champs.join("\t");
        t += "\r\nEND_DATA_FORMAT\r\n";
        t += &format!("NUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n", self.lignes.len());
        for l in &self.lignes {
            let mut v = vec![l.id.clone()];
            if self.rvb {
                v.extend(l.rvb.iter().map(|c| c.to_string()));
            }
            if self.lab {
                v.extend(l.lab.iter().map(|c| c.to_string()));
            }
            if self.spectres {
                v.extend((0..36).map(|_| l.reflectance.to_string()));
            }
            t += &v.join("\t");
            t += "\r\n";
        }
        t += "END_DATA\r\n";
        t
    }

    pub fn mesure(&self) -> MesureImportee {
        cgats::lire(&self.texte()).unwrap()
    }
}
