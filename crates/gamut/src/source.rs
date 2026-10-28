//! Profil de l'image (ticket #80) : la couleur demandée de chaque pixel, en
//! L\*a\*b\* relatifs D50. Profil RVB à table `mft2` (A2B1, puis A2B0) ou
//! matrice et courbes, dans l'ordre de l'ICC, ou gris (`kTRC`) ; sRVB quand
//! l'image n'en embarque pas.

use crate::icc::{
    lire_desc, s15, signature, u16_at, u32_at, vers_lab_pcs, Balises, ErreurIcc, ILLUMINANT_D50,
};
use crate::table::{lab16_vers_lab, lire_mft2, Lut, XYZ16};

/// Conversions écartées : la balise et la raison pour laquelle elle ne se
/// lit pas.
type Ecartees = Vec<(String, ErreurIcc)>;

/// Profil RVB, dans l'ordre de l'ICC : table colorimétrique A2B1, puis
/// perceptuelle A2B0, puis matrice et courbes. Une conversion présente mais
/// illisible est écartée avec sa raison, et la suivante est essayée ; si
/// aucune ne se lit, la raison de la première écartée est rendue.
fn lire_rvb(balises: &Balises, pcs: &[u8]) -> Result<(Genre, Ecartees), ErreurIcc> {
    let mut ecartees = Ecartees::new();
    for (sig, balise) in [(b"A2B1", "A2B1"), (b"A2B0", "A2B0")] {
        let table = match balises.trouver(sig) {
            Ok(None) => continue,
            Ok(Some(d)) => lire_pcs(pcs).and_then(|pcs| {
                Ok(Genre::Table {
                    pcs,
                    lut: lire_mft2(d, balise)?,
                })
            }),
            Err(erreur) => Err(erreur),
        };
        match table {
            Ok(genre) => return Ok((genre, ecartees)),
            Err(erreur) => ecartees.push((balise.into(), erreur)),
        }
    }
    match lire_matrice(balises) {
        Ok(Some(genre)) => return Ok((genre, ecartees)),
        Ok(None) => {}
        Err(erreur) => ecartees.push(("rXYZ".into(), erreur)),
    }
    Err(ecartees
        .into_iter()
        .next()
        .map_or(ErreurIcc::SansTable, |(_, erreur)| erreur))
}

/// Matrice et courbes : `None` si aucune des six balises n'y est.
fn lire_matrice(balises: &Balises) -> Result<Option<Genre>, ErreurIcc> {
    let paires = [(b"rXYZ", b"rTRC"), (b"gXYZ", b"gTRC"), (b"bXYZ", b"bTRC")];
    let mut colonnes = [[0.0; 3]; 3];
    let mut courbes = Vec::new();
    for (k, (xyz, trc)) in paires.into_iter().enumerate() {
        match (balises.trouver(xyz)?, balises.trouver(trc)?) {
            (Some(x), Some(t)) => {
                colonnes[k] = lire_xyz(x, xyz)?;
                courbes.push(lire_courbe(t, trc)?);
            }
            (None, None) if courbes.is_empty() && k == 0 => return Ok(None),
            _ => {
                return Err(ErreurIcc::StructureNonPriseEnCharge {
                    structure: "matrice et courbes incomplètes".into(),
                })
            }
        }
    }
    let courbes = <[Courbe; 3]>::try_from(courbes).map_err(|_| ErreurIcc::SansTable)?;
    Ok(Some(Genre::Matrice { colonnes, courbes }))
}

fn lire_pcs(pcs: &[u8]) -> Result<Pcs, ErreurIcc> {
    match pcs {
        b"Lab " => Ok(Pcs::Lab),
        b"XYZ " => Ok(Pcs::Xyz),
        autre => Err(ErreurIcc::EnTete(format!(
            "PCS {} non pris en charge",
            signature(autre)
        ))),
    }
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
    /// Gris (ICC.1, annexe F.2) : la courbe donne Y sur le blanc D50 au PCS
    /// XYZ, et L\* (de 0 à 1 pour 0 à 100) au PCS Lab.
    Gris { pcs: Pcs, courbe: Courbe },
}

/// Profil de l'image, prêt à donner la couleur demandée d'un pixel.
#[derive(Debug, Clone)]
pub struct ProfilSource {
    nom: String,
    illuminant: [f64; 3],
    genre: Genre,
    ecartees: Ecartees,
}

/// Primaires sRVB adaptées D50 par Bradford, à 4 décimales, telles que les
/// écrit un profil ICC sRVB. `srvb` les ramène sur le D50 du PCS ; l'écart
/// au `sRGB.icm` d'ArgyllCMS est de 0,0116 ΔE00 au plus (confirmé par le
/// test `srvb_concorde_avec_le_srgb_d_argyll`).
const PRIMAIRES_SRVB: [[f64; 3]; 3] = [
    [0.4361, 0.2225, 0.0139],
    [0.3851, 0.7169, 0.0971],
    [0.1431, 0.0606, 0.7141],
];

impl ProfilSource {
    /// sRVB (IEC 61966-2-1) : primaires `PRIMAIRES_SRVB` ramenées pour que
    /// leur somme soit exactement le D50 du PCS (le blanc de l'image est le
    /// blanc D50) ; courbe `para` de type 3.
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
            ecartees: Ecartees::new(),
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
        let (genre, ecartees) = match &octets[16..20] {
            b"RGB " => lire_rvb(&balises, &octets[20..24])?,
            b"GRAY" => match balises.trouver(b"kTRC")? {
                Some(t) => (
                    Genre::Gris {
                        pcs: lire_pcs(&octets[20..24])?,
                        courbe: lire_courbe(t, b"kTRC")?,
                    },
                    Ecartees::new(),
                ),
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
            ecartees,
        })
    }

    /// Conversions du profil écartées parce qu'illisibles, avec leur raison
    /// (une A2B1 `mAB` quand l'A2B0 se lit, par exemple) : rien n'est
    /// remplacé sans le dire.
    pub fn ecartees(&self) -> &[(String, ErreurIcc)] {
        &self.ecartees
    }

    /// Clé du catalogue de textes qui explique le refus du profil d'une
    /// image, et non d'un profil d'imprimante.
    pub fn cle_refus(erreur: &ErreurIcc) -> &'static str {
        match erreur {
            ErreurIcc::EnTete(_) => "comparer.image.refus.en_tete",
            ErreurIcc::PasRvb { .. } => "comparer.image.refus.pas_rvb",
            ErreurIcc::StructureNonPriseEnCharge { .. } => "comparer.image.refus.structure",
            ErreurIcc::Tronque { .. } => "comparer.image.refus.tronque",
            // Ni blanc ni table inverse ne sont demandés au profil d'une
            // image : il n'en reste que l'absence de conversion.
            ErreurIcc::SansTable | ErreurIcc::SansBlanc | ErreurIcc::SansTableInverse => {
                "comparer.image.refus.sans_conversion"
            }
        }
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
            Genre::Gris { pcs, courbe } => {
                let v = courbe.appliquer(rvb[0]);
                match pcs {
                    Pcs::Lab => [100.0 * v, 0.0, 0.0],
                    Pcs::Xyz => vers_lab_pcs(self.illuminant.map(|w| w * v), self.illuminant),
                }
            }
        }
    }
}
