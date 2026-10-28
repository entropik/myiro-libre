//! Profil de l'image (ticket #80) : la couleur demandée de chaque pixel, en
//! L\*a\*b\* relatifs D50. Profil RVB matrice et courbes, gris (`kTRC`), ou
//! RVB à table `mft2` ; sRVB quand l'image n'en embarque pas.

use crate::icc::{
    lire_desc, s15, signature, u16_at, u32_at, vers_lab_pcs, Balises, ErreurIcc, ILLUMINANT_D50,
};
use crate::table::{lab16_vers_lab, lire_mft2, Lut, XYZ16};

/// Profil RVB : matrice et courbes si les six balises y sont, sinon table
/// `mft2` colorimétrique (A2B1), sinon perceptuelle (A2B0).
fn lire_rvb(balises: &Balises, pcs: &[u8]) -> Result<Genre, ErreurIcc> {
    let mut colonnes = [[0.0; 3]; 3];
    let mut courbes = Vec::new();
    for (xyz, trc) in [(b"rXYZ", b"rTRC"), (b"gXYZ", b"gTRC"), (b"bXYZ", b"bTRC")] {
        let (Some(x), Some(t)) = (balises.trouver(xyz)?, balises.trouver(trc)?) else {
            break;
        };
        colonnes[courbes.len()] = lire_xyz(x, xyz)?;
        courbes.push(lire_courbe(t, trc)?);
    }
    if let Ok(courbes) = <[Courbe; 3]>::try_from(courbes) {
        return Ok(Genre::Matrice { colonnes, courbes });
    }
    let pcs = match pcs {
        b"Lab " => Pcs::Lab,
        b"XYZ " => Pcs::Xyz,
        autre => {
            return Err(ErreurIcc::EnTete(format!(
                "PCS {} non pris en charge",
                signature(autre)
            )))
        }
    };
    for (sig, balise) in [(b"A2B1", "A2B1"), (b"A2B0", "A2B0")] {
        if let Some(d) = balises.trouver(sig)? {
            return Ok(Genre::Table {
                pcs,
                lut: lire_mft2(d, balise)?,
            });
        }
    }
    Err(ErreurIcc::SansTable)
}

/// Balise `XYZ ` : trois s15Fixed16.
fn lire_xyz(d: &[u8], balise: &[u8; 4]) -> Result<[f64; 3], ErreurIcc> {
    if d.len() < 20 || &d[0..4] != b"XYZ " {
        return Err(ErreurIcc::StructureNonPriseEnCharge {
            structure: format!(
                "{} de type {}",
                signature(balise),
                signature(&d[..d.len().min(4)])
            ),
        });
    }
    Ok([s15(d, 8), s15(d, 12), s15(d, 16)])
}

/// Balise `curv` ou `para`.
fn lire_courbe(d: &[u8], balise: &[u8; 4]) -> Result<Courbe, ErreurIcc> {
    let tronque = || ErreurIcc::Tronque {
        balise: signature(balise),
    };
    match d.get(0..4).ok_or_else(tronque)? {
        b"curv" => {
            let n = u32_at(d, 8).ok_or_else(tronque)? as usize;
            let points = (0..n)
                .map(|i| u16_at(d, 12 + 2 * i).map(f64::from))
                .collect::<Option<Vec<f64>>>()
                .ok_or_else(tronque)?;
            Ok(match points.as_slice() {
                [] => Courbe::Para {
                    genre: 0,
                    p: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                },
                [gamma] => Courbe::Para {
                    genre: 0,
                    p: [gamma / 256.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                },
                _ => Courbe::Table(points.iter().map(|v| v / 65535.0).collect()),
            })
        }
        b"para" => {
            let genre = u16_at(d, 8).ok_or_else(tronque)?;
            let nombre = match genre {
                0 => 1,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 7,
                _ => {
                    return Err(ErreurIcc::StructureNonPriseEnCharge {
                        structure: format!("para de type {genre}"),
                    })
                }
            };
            if d.len() < 12 + 4 * nombre {
                return Err(tronque());
            }
            let mut p = [0.0; 7];
            for (i, v) in p.iter_mut().enumerate().take(nombre) {
                *v = s15(d, 12 + 4 * i);
            }
            Ok(Courbe::Para { genre, p })
        }
        autre => Err(ErreurIcc::StructureNonPriseEnCharge {
            structure: format!("{} de type {}", signature(balise), signature(autre)),
        }),
    }
}

/// Courbe de transfert d'un canal, de 0 à 1 vers 0 à 1 (linéaire).
#[derive(Debug, Clone)]
enum Courbe {
    /// `para` de l'ICC, types 0 à 4 : g, a, b, c, d, e, f (absents à 0).
    Para { genre: u16, p: [f64; 7] },
    /// `curv` de deux points ou plus, interpolée linéairement.
    Table(Vec<f64>),
}

impl Courbe {
    fn appliquer(&self, x: f64) -> f64 {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let y = match self {
            Courbe::Table(t) => {
                let xf = x * (t.len() - 1) as f64;
                // Borné de 0 à n − 2 : la conversion est exacte.
                let i = (xf.floor() as usize).min(t.len() - 2);
                let f = xf - i as f64;
                t[i] * (1.0 - f) + t[i + 1] * f
            }
            Courbe::Para { genre, p } => {
                let [g, a, b, c, d, e, f] = *p;
                let puissance = |v: f64| if v > 0.0 { v.powf(g) } else { 0.0 };
                // Seuil des types 1 et 2 : x ≥ −b/a.
                let au_dessus = a * x + b >= 0.0;
                match genre {
                    0 => puissance(x),
                    1 if au_dessus => puissance(a * x + b),
                    1 => 0.0,
                    2 if au_dessus => puissance(a * x + b) + c,
                    2 => c,
                    3 if x >= d => puissance(a * x + b),
                    3 => c * x,
                    _ if x >= d => puissance(a * x + b) + e,
                    _ => c * x + f,
                }
            }
        };
        y.clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
enum Pcs {
    Lab,
    Xyz,
}

#[derive(Debug, Clone)]
enum Genre {
    /// XYZ = colonnes rXYZ, gXYZ, bXYZ × canaux linéaires.
    Matrice {
        colonnes: [[f64; 3]; 3],
        courbes: [Courbe; 3],
    },
    /// Table `mft2` : RVB vers PCS Lab « hérité » 16 bits ou XYZ.
    Table { pcs: Pcs, lut: Lut },
    /// Y = courbe(gris), sur le blanc D50 du PCS.
    Gris(Courbe),
}

/// Profil de l'image, prêt à donner la couleur demandée d'un pixel.
#[derive(Debug, Clone)]
pub struct ProfilSource {
    nom: String,
    illuminant: [f64; 3],
    genre: Genre,
}

/// Primaires sRVB adaptées D50, telles que les écrit un profil ICC sRVB.
const PRIMAIRES_SRVB: [[f64; 3]; 3] = [
    [0.4361, 0.2225, 0.0139],
    [0.3851, 0.7169, 0.0971],
    [0.1431, 0.0606, 0.7141],
];

impl ProfilSource {
    /// sRVB (IEC 61966-2-1) : primaires adaptées D50 d'un profil ICC sRVB,
    /// ramenées pour que leur somme soit exactement le D50 du PCS (le blanc
    /// de l'image est le blanc D50) ; courbe `para` de type 3.
    pub fn srvb() -> ProfilSource {
        let somme = [0, 1, 2].map(|k| PRIMAIRES_SRVB.iter().map(|p| p[k]).sum::<f64>());
        let colonnes =
            PRIMAIRES_SRVB.map(|p| [0, 1, 2].map(|k| p[k] * ILLUMINANT_D50[k] / somme[k]));
        let courbe = Courbe::Para {
            genre: 3,
            p: [
                2.4,
                1.0 / 1.055,
                0.055 / 1.055,
                1.0 / 12.92,
                0.04045,
                0.0,
                0.0,
            ],
        };
        ProfilSource {
            nom: "sRVB".into(),
            illuminant: ILLUMINANT_D50,
            genre: Genre::Matrice {
                colonnes,
                courbes: [courbe.clone(), courbe.clone(), courbe],
            },
        }
    }

    /// Profil embarqué dans l'image : RVB matrice et courbes (`rXYZ`…,
    /// `rTRC`… en `curv` ou `para` de types 0 à 4), gris (`kTRC`), ou RVB à
    /// table `mft2` (A2B1, sinon A2B0). Autre chose est refusé avec sa
    /// raison : CMJN par `PasRvb`, un autre espace de données par `EnTete`.
    pub fn lire(octets: &[u8]) -> Result<ProfilSource, ErreurIcc> {
        if octets.len() < 132 {
            return Err(ErreurIcc::EnTete(format!(
                "fichier de {} octets, moins de 132 octets",
                octets.len()
            )));
        }
        if &octets[36..40] != b"acsp" {
            return Err(ErreurIcc::EnTete("signature acsp absente".into()));
        }
        let illuminant = [s15(octets, 68), s15(octets, 72), s15(octets, 76)];
        if illuminant.iter().any(|&v| v <= 0.0) {
            return Err(ErreurIcc::EnTete("illuminant du PCS invalide".into()));
        }
        let balises = Balises::lire(octets)?;
        let genre = match &octets[16..20] {
            b"RGB " => lire_rvb(&balises, &octets[20..24])?,
            b"GRAY" => match balises.trouver(b"kTRC")? {
                Some(t) => Genre::Gris(lire_courbe(t, b"kTRC")?),
                None => return Err(ErreurIcc::SansTable),
            },
            b"CMYK" => {
                return Err(ErreurIcc::PasRvb {
                    espace: "CMYK".into(),
                })
            }
            autre => {
                return Err(ErreurIcc::EnTete(format!(
                    "espace de données {} non pris en charge",
                    signature(autre)
                )))
            }
        };
        let nom = balises
            .trouver(b"desc")?
            .and_then(lire_desc)
            .unwrap_or_default();
        Ok(ProfilSource {
            nom,
            illuminant,
            genre,
        })
    }

    /// Nom lu dans `desc` (vide s'il ne se lit pas), ou « sRVB » pour
    /// [`ProfilSource::srvb`].
    pub fn nom(&self) -> String {
        self.nom.clone()
    }

    /// Couleur demandée : L\*a\*b\* relatifs D50 de `rvb` (de 0 à 1, borné ;
    /// un gris passe r = v = b).
    pub fn demande(&self, rvb: [f64; 3]) -> [f64; 3] {
        match &self.genre {
            Genre::Matrice { colonnes, courbes } => {
                let lineaire = [0, 1, 2].map(|k| courbes[k].appliquer(rvb[k]));
                let xyz = [0, 1, 2].map(|i| (0..3).map(|k| colonnes[k][i] * lineaire[k]).sum());
                vers_lab_pcs(xyz, self.illuminant)
            }
            Genre::Table { pcs, lut } => {
                let v = lut.evaluer(rvb);
                match pcs {
                    Pcs::Lab => lab16_vers_lab(v),
                    Pcs::Xyz => vers_lab_pcs(v.map(|x| x * XYZ16), self.illuminant),
                }
            }
            Genre::Gris(courbe) => {
                let y = courbe.appliquer(rvb[0]);
                vers_lab_pcs(self.illuminant.map(|w| w * y), self.illuminant)
            }
        }
    }
}
