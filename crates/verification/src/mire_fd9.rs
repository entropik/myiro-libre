//! Mire de vérification du FD-9 (plan 7, tâche 10) : environ 200 couleurs,
//! différentes de la mire de profilage RGB-2520, tirées une fois d'une
//! graine figée et versionnée, et écrites en XML « ChartDefinition » pour
//! que FD-S2w l'ouvre (Fichier → Ouvrir le fichier de la charte) et en tire
//! le TIFF à imprimer, avec son QR code.
//!
//! Format relevé le 28 octobre 2026 sur les mires FD-9 RVB livrées avec
//! FD-S2w (version `00.00`), jamais copiées : `Data` → `Chart` → `Sheet` →
//! `Area` → `Patch`, dimensions en centièmes de millimètre, valeurs RVB de 0
//! à 255 multipliées par 100. `PatchWidthGap` et `PatchHeightGap` sont
//! écrits comme le pas d'une plage à la suivante (ils valent la largeur de
//! plage sur les mires jointives livrées). **Confirmé à l'ouverture** le
//! 30 octobre 2026 : FD-S2w ouvre la mire version 1 et montre ses 210
//! plages en 15 × 14. La lecture de la mire imprimée par le FD-9 reste à
//! faire.

use serde::{Deserialize, Serialize};

/// Version de la liste de couleurs et de la mise en page : en changer donne
/// une autre mire.
pub const MIRE_FD9_VERSION: u32 = 1;

/// Graine du tirage des couleurs (version 1).
const GRAINE: u64 = 0x4D59_524F_2D56_3031;

/// Niveaux RVB (0 à 255) employés par les 2 520 plages de la RGB-2520,
/// relevés sur un export de sa mesure du 26 octobre 2026. Une couleur dont
/// une composante est hors de ces niveaux n'est pas une plage de la RGB-2520.
pub const RGB2520_NIVEAUX: [u8; 66] = [
    0, 5, 10, 11, 20, 21, 26, 31, 32, 41, 43, 51, 53, 61, 62, 64, 71, 72, 74, 77, 81, 82, 85, 92,
    96, 102, 106, 112, 117, 122, 123, 127, 128, 132, 133, 138, 143, 149, 153, 159, 163, 170, 173,
    174, 179, 181, 183, 184, 191, 193, 194, 202, 204, 213, 214, 223, 224, 230, 234, 235, 244, 245,
    250, 252, 254, 255,
];

/// Gabarit : A4 portrait, 15 colonnes × 14 rangées de plages de 10 mm au
/// pas de 11 mm (espace de 1 mm, 10 % de la plage).
const COLONNES: usize = 15;
const RANGEES: usize = 14;
const PAGE_MM: (f64, f64) = (210.0, 297.0);
const PLAGE_MM: f64 = 10.0;
const PAS_MM: f64 = 11.0;
/// Zone centrée en largeur, et dans la hauteur laissée par les marges du
/// FD-9 (23 mm en tête, 33 mm en pied).
const ORIGINE_MM: (f64, f64) = (23.0, 67.0);
/// Date écrite dans la mire (celle de sa version), pour qu'elle reste
/// identique à chaque écriture.
const CREATION: &str = "2026/10/28 00:00:00";

/// Une plage de la mire, à sa place (colonne et rangée comptées de 1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlageMire {
    pub id: u32,
    pub colonne: usize,
    pub rangee: usize,
    pub rvb: [u8; 3],
}

/// Une mire FD-9 d'une seule zone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MireFd9 {
    pub nom: String,
    pub version: u32,
    pub page_mm: (f64, f64),
    pub plage_mm: f64,
    pub pas_mm: f64,
    /// Coin haut gauche de la zone de plages.
    pub origine_mm: (f64, f64),
    pub colonnes: usize,
    pub rangees: usize,
    /// Dans l'ordre des rangées, de gauche à droite et de haut en bas.
    pub plages: Vec<PlageMire>,
}

fn coin(c: &[u8; 3]) -> bool {
    c.iter().all(|v| *v == 0 || *v == 255)
}

fn hors_rgb2520(c: &[u8; 3]) -> bool {
    c.iter().any(|v| !RGB2520_NIVEAUX.contains(v))
}

/// Les couleurs de la mire de vérification, toujours les mêmes : les huit
/// coins du cube (blanc du papier, noir, primaires, secondaires), une rampe
/// de seize gris, douze tons chair, puis des couleurs tirées de la graine.
/// Hors des coins, chacune a une composante hors des niveaux de la RGB-2520.
pub fn couleurs_verification_fd9() -> Vec<[u8; 3]> {
    let mut c: Vec<[u8; 3]> = Vec::with_capacity(COLONNES * RANGEES);
    for r in [255, 0] {
        for g in [255, 0] {
            for b in [255, 0] {
                c.push([r, g, b]);
            }
        }
    }
    for v in [
        8, 16, 24, 36, 47, 56, 67, 89, 109, 135, 146, 166, 188, 208, 228, 248,
    ] {
        c.push([v, v, v]);
    }
    c.extend([
        [248, 218, 198],
        [240, 198, 166],
        [228, 188, 156],
        [218, 166, 135],
        [208, 156, 119],
        [198, 146, 109],
        [188, 135, 99],
        [177, 119, 89],
        [166, 109, 78],
        [146, 99, 67],
        [135, 89, 56],
        [119, 78, 47],
    ]);
    let mut etat = GRAINE;
    let mut suivant = || {
        etat = etat
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (etat >> 33) as u8
    };
    while c.len() < COLONNES * RANGEES {
        let nouvelle = [suivant(), suivant(), suivant()];
        if !coin(&nouvelle) && hors_rgb2520(&nouvelle) && !c.contains(&nouvelle) {
            c.push(nouvelle);
        }
    }
    c
}

/// Clarté approchée d'un RVB, pour la mise en page seulement.
fn luma(c: &[u8; 3]) -> f64 {
    0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2])
}

/// La mire de vérification : les couleurs placées en damier, claires et
/// foncées en alternance, pour que deux voisines se distinguent au scan.
pub fn mire_verification_fd9() -> MireFd9 {
    let mut couleurs = couleurs_verification_fd9();
    couleurs.sort_by(|a, b| luma(a).total_cmp(&luma(b)));
    let moitie = couleurs.len() / 2;
    let (foncees, claires) = couleurs.split_at(moitie);
    let (mut f, mut cl) = (foncees.iter(), claires.iter());
    let mut plages = Vec::with_capacity(COLONNES * RANGEES);
    for rangee in 1..=RANGEES {
        for colonne in 1..=COLONNES {
            let suivante = if (rangee + colonne) % 2 == 0 {
                cl.next()
            } else {
                f.next()
            };
            if let Some(rvb) = suivante {
                plages.push(PlageMire {
                    id: plages.len() as u32 + 1,
                    colonne,
                    rangee,
                    rvb: *rvb,
                });
            }
        }
    }
    MireFd9 {
        nom: format!("myiro-libre verification FD-9 v{MIRE_FD9_VERSION}"),
        version: MIRE_FD9_VERSION,
        page_mm: PAGE_MM,
        plage_mm: PLAGE_MM,
        pas_mm: PAS_MM,
        origine_mm: ORIGINE_MM,
        colonnes: COLONNES,
        rangees: RANGEES,
        plages,
    }
}

fn echapper(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn restaurer(t: &str) -> String {
    t.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Millimètres en centièmes de millimètre, unité de FD-S2w.
fn centiemes(mm: f64) -> i64 {
    (mm * 100.0).round() as i64
}

/// La mire en XML « ChartDefinition » de FD-S2w, en UTF-8, fins de ligne
/// CRLF comme les fichiers de FD-S2w.
pub fn ecrire_xml_fds2w(m: &MireFd9) -> String {
    let n = m.plages.len();
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n");
    x.push_str(
        "<Data Identifier=\"ChartDefinition\" Version=\"00.00\" Application=\"FD-S2w\">\r\n",
    );
    x.push_str(&format!(
        "  <Chart CreateTime=\"{CREATION}\" Kind=\"Normal\" SheetCount=\"1\" UUID=\"\" UNIT=\"0\" ChartPatchCount=\"{n}\" ChartName=\"{}\">\r\n",
        echapper(&m.nom)
    ));
    x.push_str(&format!(
        "    <Sheet Width=\"{}\" Page=\"1\" Height=\"{}\" AreaCount=\"1\" PatchCount=\"{n}\">\r\n",
        centiemes(m.page_mm.0),
        centiemes(m.page_mm.1)
    ));
    let (plage, pas) = (centiemes(m.plage_mm), centiemes(m.pas_mm));
    x.push_str(&format!(
        "      <Area LeftTopX=\"{}\" LeftTopY=\"{}\" ColumnCount=\"{}\" Number=\"1\" RowCount=\"{}\" PatchWidthGap=\"{pas}\" PatchCount=\"{n}\" PatchHeight=\"{plage}\" PatchHeightGap=\"{pas}\" PatchWidth=\"{plage}\">\r\n",
        centiemes(m.origine_mm.0),
        centiemes(m.origine_mm.1),
        m.colonnes,
        m.rangees
    ));
    for p in &m.plages {
        let [r, g, b] = p.rvb.map(|v| u32::from(v) * 100);
        x.push_str(&format!(
            "        <Patch Column=\"{}\" ColorValue1=\"{r}\" ColorValue2=\"{g}\" ColorValue3=\"{b}\" ColorFormat=\"RGB\" Row=\"{}\" ID=\"{}\"/>\r\n",
            p.colonne, p.rangee, p.id
        ));
    }
    x.push_str("      </Area>\r\n    </Sheet>\r\n  </Chart>\r\n</Data>\r\n");
    x
}

/// Attributs `nom="valeur"` d'une balise ouvrante, valeurs restaurées.
fn attributs(balise: &str) -> Vec<(String, String)> {
    let mut a = Vec::new();
    let mut reste = balise;
    while let Some(egal) = reste.find("=\"") {
        let nom = reste[..egal]
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("");
        let apres = &reste[egal + 2..];
        let Some(fin) = apres.find('"') else { break };
        a.push((nom.to_string(), restaurer(&apres[..fin])));
        reste = &apres[fin + 1..];
    }
    a
}

/// Relit une mire écrite par [`ecrire_xml_fds2w`] (une feuille, une zone,
/// des plages RVB) ; ce qui ne va pas est dit.
pub fn lire_xml_fds2w(texte: &str) -> Result<MireFd9, String> {
    let mut nom = None;
    let (mut page, mut zone) = (None, None);
    let mut plages = Vec::new();
    for morceau in texte.split('<').skip(1) {
        let balise = morceau.split('>').next().unwrap_or("");
        let a = attributs(balise);
        let champ = |n: &str| a.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        let nombre = |n: &str| -> Result<f64, String> {
            champ(n)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| format!("attribut {n} absent ou illisible"))
        };
        match balise.split_whitespace().next() {
            Some("Chart") => nom = champ("ChartName"),
            Some("Sheet") => page = Some((nombre("Width")? / 100.0, nombre("Height")? / 100.0)),
            Some("Area") => {
                zone = Some((
                    (nombre("LeftTopX")? / 100.0, nombre("LeftTopY")? / 100.0),
                    nombre("PatchWidth")? / 100.0,
                    nombre("PatchWidthGap")? / 100.0,
                    nombre("ColumnCount")? as usize,
                    nombre("RowCount")? as usize,
                ))
            }
            Some("Patch") => {
                if champ("ColorFormat").as_deref() != Some("RGB") {
                    return Err("plage qui n'est pas en RVB".into());
                }
                let v = |n: &str| -> Result<u8, String> {
                    let x = nombre(n)? / 100.0;
                    (0.0..=255.0)
                        .contains(&x)
                        .then_some(x.round() as u8)
                        .ok_or_else(|| format!("{n} hors de 0 à 255"))
                };
                plages.push(PlageMire {
                    id: nombre("ID")? as u32,
                    colonne: nombre("Column")? as usize,
                    rangee: nombre("Row")? as usize,
                    rvb: [v("ColorValue1")?, v("ColorValue2")?, v("ColorValue3")?],
                });
            }
            _ => {}
        }
    }
    let nom = nom.ok_or("balise Chart absente")?;
    let page_mm = page.ok_or("balise Sheet absente")?;
    let (origine_mm, plage_mm, pas_mm, colonnes, rangees) = zone.ok_or("balise Area absente")?;
    let version = nom
        .rsplit_once(" v")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(MIRE_FD9_VERSION);
    Ok(MireFd9 {
        nom,
        version,
        page_mm,
        plage_mm,
        pas_mm,
        origine_mm,
        colonnes,
        rangees,
        plages,
    })
}
