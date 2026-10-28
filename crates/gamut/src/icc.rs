//! Lecture d'un profil ICC RVB : en-tête, table des balises, `wtpt`, `desc`,
//! `targ` et table A2B1 de type `mft2` (lut16).

use colorimetrie::{xyz_vers_lab, Xyz};

use crate::table::{lire_mft2, Lut};

/// Illuminant D50 du PCS, tel que l'écrivent les profils (échelle Y = 1).
pub(crate) const ILLUMINANT_D50: [f64; 3] = [0.9642, 1.0, 0.8249];

/// Raison pour laquelle un profil ne peut pas être lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurIcc {
    /// En-tête absent ou invalide (taille, signature `acsp`, PCS).
    EnTete(String),
    /// L'espace de données n'est pas RVB (CMJN par exemple).
    PasRvb { espace: String },
    /// Pas de table A2B1 (colorimétrique relative).
    SansTable,
    /// La table A2B1 a une structure que la crate ne lit pas (`mft1`, `mAB`…).
    StructureNonPriseEnCharge { structure: String },
    /// Une balise sort du fichier.
    Tronque { balise: String },
    /// Pas de blanc du support (`wtpt`) : la colorimétrie absolue est
    /// impossible, et aucun blanc D50 n'est supposé.
    SansBlanc,
}

impl ErreurIcc {
    /// Clé du catalogue de textes qui explique le refus.
    pub fn cle(&self) -> &'static str {
        match self {
            ErreurIcc::EnTete(_) => "comparer.refus.en_tete",
            ErreurIcc::PasRvb { .. } => "comparer.refus.pas_rvb",
            ErreurIcc::SansTable => "comparer.refus.sans_table",
            ErreurIcc::StructureNonPriseEnCharge { .. } => "comparer.refus.structure",
            ErreurIcc::Tronque { .. } => "comparer.refus.tronque",
            ErreurIcc::SansBlanc => "comparer.refus.sans_blanc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Pcs {
    Lab,
    Xyz,
}

/// Profil ICC RVB lu, prêt à donner ses couleurs en colorimétrie absolue.
#[derive(Debug, Clone)]
pub struct ProfilIcc {
    version: (u8, u8),
    description: Option<String>,
    pcs: Pcs,
    illuminant: [f64; 3],
    blanc: [f64; 3],
    lut: Lut,
    mesures: Option<Vec<u8>>,
}

impl ProfilIcc {
    /// Lit les octets d'un profil ICC RVB à table A2B1 `mft2`.
    pub fn lire(octets: &[u8]) -> Result<ProfilIcc, ErreurIcc> {
        if octets.len() < 132 {
            return Err(ErreurIcc::EnTete(format!(
                "fichier de {} octets, moins de 132 octets",
                octets.len()
            )));
        }
        if &octets[36..40] != b"acsp" {
            return Err(ErreurIcc::EnTete("signature acsp absente".into()));
        }
        let espace = &octets[16..20];
        if espace != b"RGB " {
            return Err(ErreurIcc::PasRvb {
                espace: signature(espace),
            });
        }
        let pcs = match &octets[20..24] {
            b"Lab " => Pcs::Lab,
            b"XYZ " => Pcs::Xyz,
            autre => {
                return Err(ErreurIcc::EnTete(format!(
                    "PCS {} non pris en charge",
                    signature(autre)
                )))
            }
        };
        let illuminant = [s15(octets, 68), s15(octets, 72), s15(octets, 76)];
        if illuminant.iter().any(|&v| v <= 0.0) {
            return Err(ErreurIcc::EnTete("illuminant du PCS invalide".into()));
        }
        let balises = Balises::lire(octets)?;

        let lut = match balises.trouver(b"A2B1")? {
            None => return Err(ErreurIcc::SansTable),
            Some(donnees) => lire_mft2(donnees)?,
        };
        let blanc = match balises.trouver(b"wtpt")? {
            Some(d) if d.len() >= 20 && &d[0..4] == b"XYZ " => [s15(d, 8), s15(d, 12), s15(d, 16)],
            Some(_) => {
                return Err(ErreurIcc::Tronque {
                    balise: "wtpt".into(),
                })
            }
            None => return Err(ErreurIcc::SansBlanc),
        };
        if blanc.iter().any(|&v| v <= 0.0) {
            return Err(ErreurIcc::SansBlanc);
        }
        let description = balises.trouver(b"desc")?.and_then(lire_desc);
        let mesures = balises.trouver(b"targ")?.and_then(lire_texte);

        Ok(ProfilIcc {
            version: (octets[8], octets[9] >> 4),
            description,
            pcs,
            illuminant,
            blanc,
            lut,
            mesures,
        })
    }

    /// Version ICC de l'en-tête, par exemple « 2.4 ».
    pub fn version(&self) -> String {
        format!("{}.{}", self.version.0, self.version.1)
    }

    /// Description du profil (balise `desc`), si elle se lit.
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// Points par arête de la table A2B1.
    pub fn grille(&self) -> u8 {
        u8::try_from(self.lut.grille).unwrap_or(u8::MAX)
    }

    /// L\*, a\*, b\* absolus de `rvb` (de 0 à 1, borné) par la table A2B1.
    pub fn lab_absolu(&self, rvb: [f64; 3]) -> [f64; 3] {
        let v = self.lut.evaluer(rvb);
        let relatif = match self.pcs {
            Pcs::Lab => {
                let w = v.map(|x| x * 65535.0);
                let lab = [w[0] / 652.80, w[1] / 256.0 - 128.0, w[2] / 256.0 - 128.0];
                lab_vers_xyz(lab, self.illuminant)
            }
            Pcs::Xyz => v.map(|x| x * 65535.0 / 32768.0),
        };
        let absolu = |k: usize| 100.0 * relatif[k] * self.blanc[k] / self.illuminant[k];
        let xyz = Xyz {
            x: absolu(0),
            y: absolu(1),
            z: absolu(2),
        };
        let blanc = Xyz {
            x: 100.0 * self.illuminant[0],
            y: 100.0 * self.illuminant[1],
            z: 100.0 * self.illuminant[2],
        };
        // Valeurs finies et illuminant positif, contrôlés à la lecture.
        xyz_vers_lab(xyz, blanc).map_or([f64::NAN; 3], |lab| [lab.l, lab.a, lab.b])
    }

    /// Blanc du support (`wtpt`) en L\*, a\*, b\*, rapporté à l'illuminant
    /// du PCS.
    pub fn blanc_support(&self) -> [f64; 3] {
        let blanc = |v: [f64; 3]| Xyz {
            x: 100.0 * v[0],
            y: 100.0 * v[1],
            z: 100.0 * v[2],
        };
        xyz_vers_lab(blanc(self.blanc), blanc(self.illuminant))
            .map_or([f64::NAN; 3], |lab| [lab.l, lab.a, lab.b])
    }

    /// Mesures embarquées (balise `targ`), octets tels quels.
    pub fn mesures_embarquees(&self) -> Option<&[u8]> {
        self.mesures.as_deref()
    }
}

/// Table des balises : signature, position et taille.
struct Balises<'a> {
    octets: &'a [u8],
    entrees: Vec<([u8; 4], usize, usize)>,
}

impl<'a> Balises<'a> {
    fn lire(octets: &'a [u8]) -> Result<Balises<'a>, ErreurIcc> {
        let nombre = u32_at(octets, 128).unwrap_or(0) as usize;
        let fin = nombre
            .checked_mul(12)
            .and_then(|n| n.checked_add(132))
            .filter(|&fin| fin <= octets.len())
            .ok_or_else(|| ErreurIcc::EnTete("table des balises tronquée".into()))?;
        let entrees = octets[132..fin]
            .as_chunks::<12>()
            .0
            .iter()
            .map(|e| {
                let mut sig = [0u8; 4];
                sig.copy_from_slice(&e[0..4]);
                let position = u32_at(e, 4).unwrap_or(0) as usize;
                let taille = u32_at(e, 8).unwrap_or(0) as usize;
                (sig, position, taille)
            })
            .collect();
        Ok(Balises { octets, entrees })
    }

    /// Données d'une balise ; `Tronque` si elle sort du fichier.
    fn trouver(&self, sig: &[u8; 4]) -> Result<Option<&'a [u8]>, ErreurIcc> {
        let Some(&(_, position, taille)) = self.entrees.iter().find(|e| &e.0 == sig) else {
            return Ok(None);
        };
        position
            .checked_add(taille)
            .and_then(|fin| self.octets.get(position..fin))
            .map(Some)
            .ok_or_else(|| ErreurIcc::Tronque {
                balise: signature(sig),
            })
    }
}

/// Texte d'un `desc` (ASCII) ou premier enregistrement d'un `mluc`.
fn lire_desc(d: &[u8]) -> Option<String> {
    match d.get(0..4)? {
        b"desc" => {
            let n = u32_at(d, 8)? as usize;
            let texte = d.get(12..12usize.checked_add(n)?)?;
            let texte = texte.split(|&o| o == 0).next()?;
            Some(String::from_utf8_lossy(texte).into_owned())
        }
        b"mluc" => {
            if u32_at(d, 8)? == 0 {
                return None;
            }
            let longueur = u32_at(d, 20)? as usize;
            let position = u32_at(d, 24)? as usize;
            let unites: Vec<u16> = d
                .get(position..position.checked_add(longueur)?)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|m| u16::from_be_bytes([m[0], m[1]]))
                .collect();
            Some(String::from_utf16_lossy(&unites))
        }
        _ => None,
    }
}

/// Octets d'un type `text` après 8 octets, sans les NUL de fin.
fn lire_texte(d: &[u8]) -> Option<Vec<u8>> {
    if d.get(0..4)? != b"text" {
        return None;
    }
    let mut texte = d.get(8..)?;
    while let Some((&0, reste)) = texte.split_last() {
        texte = reste;
    }
    Some(texte.to_vec())
}

pub(crate) fn signature(octets: &[u8]) -> String {
    String::from_utf8_lossy(octets).trim_end().to_string()
}

pub(crate) fn u32_at(d: &[u8], position: usize) -> Option<u32> {
    let o = d.get(position..position.checked_add(4)?)?;
    Some(u32::from_be_bytes([o[0], o[1], o[2], o[3]]))
}

pub(crate) fn u16_at(d: &[u8], position: usize) -> Option<u16> {
    let o = d.get(position..position.checked_add(2)?)?;
    Some(u16::from_be_bytes([o[0], o[1]]))
}

/// s15Fixed16 ; zéro si hors du fichier (la longueur est contrôlée avant).
fn s15(d: &[u8], position: usize) -> f64 {
    f64::from(u32_at(d, position).unwrap_or(0) as i32) / 65536.0
}

/// L\*, a\*, b\* vers XYZ, rapporté au blanc donné (même échelle que lui).
pub(crate) fn lab_vers_xyz(lab: [f64; 3], blanc: [f64; 3]) -> [f64; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let f = [fy + lab[1] / 500.0, fy, fy - lab[2] / 200.0];
    let inverse = |t: f64| {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            (116.0 * t - 16.0) / (24389.0 / 27.0)
        }
    };
    [
        blanc[0] * inverse(f[0]),
        blanc[1] * inverse(f[1]),
        blanc[2] * inverse(f[2]),
    ]
}
