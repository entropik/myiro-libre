//! Calage d'une feuille au FD-9 : ce que l'instrument va viser, et l'image de
//! la feuille qu'il a prise, réduite par le pont. L'application peut ainsi
//! montrer, comme FD-S2w (onglet « Measured Point »), un point sur chaque
//! plage avant de mesurer (fiche `docs/abi/FD9_GetMeasurePoint.md`).
//!
//! Ce n'est pas une donnée de la mesure conservée : le calage accompagne la
//! réponse (`prenumerisee`, `mesure`) et reste facultatif. L'image peut
//! montrer ce que l'opérateur a écrit à la main sur la feuille : elle ne va
//! dans aucun fichier versionné.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::mesure::ErreurMesure;

/// Points visés et image d'une feuille reconnue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalageFeuille {
    /// Décalage moyen de la feuille (x, y) sur la grille de la définition, en
    /// 1/100 mm.
    pub decalage: [i32; 2],
    /// Plus grand écart d'un point à la grille, décalage retiré, en 1/100 mm.
    pub ecart_max: u32,
    /// Un point par plage, dans l'ordre de la définition (zone, rangée,
    /// colonne).
    pub points: Vec<PointVise>,
    /// Image de la feuille ; absente si la DLL ne l'a pas rendue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageFeuille>,
}

/// Centre d'une plage trouvée sur l'image (`FD9_GetMeasurePoint`), au
/// 1/100 mm sur le papier, et sa place dans la mire, comptée depuis 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointVise {
    pub zone: u32,
    pub rangee: u32,
    pub colonne: u32,
    pub x: i32,
    pub y: i32,
}

/// Image de la feuille en gris sur 8 bits, une rangée après l'autre, de haut
/// en bas, sans bourrage. `ppp` : pixels par pouce (25,4 mm) ; un point
/// `(x, y)` en 1/100 mm tombe sur le pixel `(x × ppp / 2540, y × ppp / 2540)`
/// (supposé : même origine que les points, fiche `FD9_GetChartImage`).
/// Vérifiée à la construction comme à la lecture : `gris` a exactement
/// `largeur × hauteur` octets. Écrite en base64 (`gris`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ImageLue")]
pub struct ImageFeuille {
    largeur: u32,
    hauteur: u32,
    ppp: u32,
    #[serde(serialize_with = "ecrire_base64")]
    gris: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageLue {
    largeur: u32,
    hauteur: u32,
    ppp: u32,
    #[serde(deserialize_with = "lire_base64")]
    gris: Vec<u8>,
}

impl TryFrom<ImageLue> for ImageFeuille {
    type Error = ErreurMesure;
    fn try_from(i: ImageLue) -> Result<Self, ErreurMesure> {
        ImageFeuille::new(i.largeur, i.hauteur, i.ppp, i.gris)
    }
}

impl ImageFeuille {
    /// Image non vide, `ppp` non nul, `largeur × hauteur` octets.
    pub fn new(largeur: u32, hauteur: u32, ppp: u32, gris: Vec<u8>) -> Result<Self, ErreurMesure> {
        let attendu = u64::from(largeur) * u64::from(hauteur);
        if attendu == 0 || ppp == 0 {
            return Err(ErreurMesure(format!(
                "image de la feuille : {largeur} × {hauteur} à {ppp} ppp"
            )));
        }
        if gris.len() as u64 != attendu {
            return Err(ErreurMesure(format!(
                "image de la feuille : {} octets pour {largeur} × {hauteur}",
                gris.len()
            )));
        }
        Ok(ImageFeuille {
            largeur,
            hauteur,
            ppp,
            gris,
        })
    }

    pub fn largeur(&self) -> u32 {
        self.largeur
    }

    pub fn hauteur(&self) -> u32 {
        self.hauteur
    }

    pub fn ppp(&self) -> u32 {
        self.ppp
    }

    /// Les pixels, une rangée après l'autre.
    pub fn gris(&self) -> &[u8] {
        &self.gris
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 standard (RFC 4648), avec rembourrage `=`.
fn base64(octets: &[u8]) -> String {
    let mut texte = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let n = (u32::from(bloc[0]) << 16)
            | (u32::from(*bloc.get(1).unwrap_or(&0)) << 8)
            | u32::from(*bloc.get(2).unwrap_or(&0));
        for (i, decalage) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= bloc.len() {
                texte.push(char::from(ALPHABET[(n >> decalage) as usize & 63]));
            } else {
                texte.push('=');
            }
        }
    }
    texte
}

/// Inverse de [`base64`] ; refuse tout caractère hors de l'alphabet, une
/// longueur qui n'est pas un multiple de 4 et un rembourrage mal placé.
fn depuis_base64(texte: &str) -> Result<Vec<u8>, String> {
    let octets = texte.as_bytes();
    if !octets.len().is_multiple_of(4) {
        return Err(format!("base64 : longueur {}", octets.len()));
    }
    let valeur = |c: u8| ALPHABET.iter().position(|&a| a == c).map(|v| v as u32);
    let mut sortie = Vec::with_capacity(octets.len() / 4 * 3);
    let blocs = octets.len() / 4;
    for (k, bloc) in octets.chunks(4).enumerate() {
        let remplis = match (bloc[2], bloc[3]) {
            (b'=', b'=') if k + 1 == blocs => 1,
            (_, b'=') if k + 1 == blocs => 2,
            _ => 3,
        };
        let mut n = 0u32;
        for (i, &c) in bloc.iter().enumerate() {
            let v = if i > remplis {
                0
            } else {
                valeur(c).ok_or_else(|| format!("base64 : caractère « {} »", char::from(c)))?
            };
            n = (n << 6) | v;
        }
        sortie.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8][..remplis]);
    }
    Ok(sortie)
}

fn ecrire_base64<S: Serializer>(gris: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&base64(gris))
}

fn lire_base64<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let texte = String::deserialize(d)?;
    depuis_base64(&texte).map_err(serde::de::Error::custom)
}
