//! Dessin d'une page MYIRO-1 à 300 ppp : plages, texte lisible et QR code.
//!
//! Le texte utilise la police bitmap `font8x8` (domaine public), table
//! latine pour les accents, agrandie 4 fois (2,7 mm par caractère). Le QR
//! code porte le tirage, la page et le début de l'empreinte : le MYIRO-1 ne
//! le lit pas, il sert à reconnaître la page ailleurs.

use font8x8::legacy::{BASIC_LEGACY, LATIN_LEGACY};
use qrcode::{Color, EcLevel, QrCode};

use crate::{ErreurMire, Gabarit, MiseEnPage};

/// Résolution des pages.
pub const RESOLUTION_PPP: u32 = 300;
/// Agrandissement de la police 8 × 8.
const ECHELLE_TEXTE: u32 = 4;
/// Pas entre deux lignes de texte, en pixels.
const INTERLIGNE_PX: u32 = 40;
/// Côté d'un module du QR code : 0,6 mm.
const MODULE_PX: u32 = 7;
/// Marge blanche du QR code, en modules.
const MARGE_QR_MODULES: u32 = 4;
/// Début du contenu de nos QR codes.
const PREFIXE_QR: &str = "MYIRO-LIBRE:T";
/// « … », absent de `font8x8` : trois points de 2 × 2.
const POINTS_DE_SUITE: [u8; 8] = [0, 0, 0, 0, 0, 0xDB, 0xDB, 0];

/// Image RVB entrelacée, fond blanc (255).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageRvb {
    pub largeur: u32,
    pub hauteur: u32,
    pub pixels: Vec<u8>,
}

impl ImageRvb {
    fn blanche(largeur: u32, hauteur: u32) -> ImageRvb {
        ImageRvb {
            largeur,
            hauteur,
            pixels: vec![255; largeur as usize * hauteur as usize * 3],
        }
    }

    /// Remplit le rectangle [x0, x1[ × [y0, y1[, rogné à l'image.
    fn remplir(&mut self, (x0, y0): (u32, u32), (x1, y1): (u32, u32), couleur: [u8; 3]) {
        let (x1, y1) = (x1.min(self.largeur), y1.min(self.hauteur));
        for y in y0..y1 {
            let debut = (y as usize * self.largeur as usize + x0 as usize) * 3;
            let fin = (y as usize * self.largeur as usize + x1 as usize) * 3;
            if debut < fin {
                for pixel in self.pixels[debut..fin].chunks_mut(3) {
                    pixel.copy_from_slice(&couleur);
                }
            }
        }
    }
}

/// Texte lisible d'une page et contenu de son QR code.
#[derive(Clone, Debug, PartialEq)]
pub struct Etiquette {
    pub lignes: Vec<String>,
    pub code: String,
}

/// Ce que dit le QR code d'une page MYIRO-1.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeTirage {
    pub tirage: i64,
    pub page: usize,
    pub pages: usize,
    /// Début de l'empreinte de la mise en page, en majuscules.
    pub empreinte: String,
}

/// « MYIRO-LIBRE:T42:P1/2:1A2B3C4D » : tout en caractères du mode
/// alphanumérique du QR code ; empreinte réduite à ses 8 premiers
/// caractères, en majuscules.
pub fn contenu_qr(code: &CodeTirage) -> String {
    let empreinte: String = code.empreinte.chars().take(8).collect();
    format!(
        "{PREFIXE_QR}{}:P{}/{}:{}",
        code.tirage,
        code.page,
        code.pages,
        empreinte.to_uppercase()
    )
}

/// Inverse de [`contenu_qr`] ; rien pour un texte qui n'est pas à nous.
pub fn lire_qr(texte: &str) -> Option<CodeTirage> {
    let reste = texte.strip_prefix(PREFIXE_QR)?;
    let (tirage, reste) = reste.split_once(":P")?;
    let (pages, empreinte) = reste.split_once(':')?;
    let (page, pages) = pages.split_once('/')?;
    let chiffres = |t: &str| !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit());
    if !chiffres(tirage) || !chiffres(page) || !chiffres(pages) {
        return None;
    }
    let (page, pages): (usize, usize) = (page.parse().ok()?, pages.parse().ok()?);
    let hexadecimal = empreinte.len() == 8
        && empreinte
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(&c));
    if page == 0 || page > pages || !hexadecimal {
        return None;
    }
    Some(CodeTirage {
        tirage: tirage.parse().ok()?,
        page,
        pages,
        empreinte: empreinte.to_string(),
    })
}

/// La ligne si elle tient en `caracteres`, sinon son début suivi de « … ».
pub fn couper_ligne(ligne: &str, caracteres: usize) -> String {
    if ligne.chars().count() <= caracteres {
        return ligne.to_string();
    }
    match caracteres.checked_sub(1) {
        Some(garde) => ligne.chars().take(garde).chain(['…']).collect(),
        None => String::new(),
    }
}

/// Millimètres vers pixels, arrondis : appliqué à chaque bord, deux plages
/// voisines se touchent sans trou.
fn px(mm: f64) -> u32 {
    (mm * RESOLUTION_PPP as f64 / 25.4).round().max(0.0) as u32
}

/// Valeur de 0 à 100 vers octet.
fn octet(v: f64) -> u8 {
    (v * 2.55).round().clamp(0.0, 255.0) as u8
}

/// Le bandeau du haut d'une page : QR code à droite, texte à gauche.
struct Bandeau {
    qr: QrCode,
    /// Colonne, en pixels, du premier module du QR code.
    x_qr: u32,
    /// Caractères d'une ligne de texte avant la marge du QR code ; au-delà,
    /// la ligne est coupée par « … » ([`couper_ligne`]).
    caracteres: usize,
}

fn bandeau(g: &Gabarit, code: &str) -> Result<Bandeau, ErreurMire> {
    let erreur = |cause: String| ErreurMire::Ecriture(cause);
    let zone = g.zone_texte();
    let (zx, zy) = (px(zone.x), px(zone.y));
    let (zx1, zy1) = (px(zone.x + zone.largeur), px(zone.y + zone.hauteur));
    let qr = QrCode::with_error_correction_level(code, EcLevel::M)
        .map_err(|e| erreur(format!("QR code : {e}")))?;
    let cote_qr = qr.width() as u32 * MODULE_PX;
    if cote_qr > zy1 - zy || cote_qr + MARGE_QR_MODULES * MODULE_PX > zx1 - zx {
        return Err(erreur("QR code trop grand pour le bandeau".to_string()));
    }
    let x_qr = zx1 - cote_qr;
    let fin_texte = x_qr - MARGE_QR_MODULES * MODULE_PX;
    Ok(Bandeau {
        qr,
        x_qr,
        caracteres: ((fin_texte - zx) / (8 * ECHELLE_TEXTE)) as usize,
    })
}

/// Nombre de caractères qu'une ligne du texte d'une page peut avoir sans
/// être coupée, pour ce gabarit et ce contenu de QR code (plus le QR code
/// est grand, moins il reste de place). C'est le calcul de [`rendre_page`].
pub fn caracteres_par_ligne(gabarit: &Gabarit, code: &str) -> Result<usize, ErreurMire> {
    bandeau(gabarit, code).map(|b| b.caracteres)
}

/// Dessine la page `page` (numérotée à partir de 1) de la mise en page.
pub fn rendre_page(
    mise: &MiseEnPage,
    valeurs: &[[f64; 3]],
    page: usize,
    etiquette: &Etiquette,
) -> Result<ImageRvb, ErreurMire> {
    let erreur = |cause: String| ErreurMire::Ecriture(cause);
    let contenu = mise
        .pages
        .iter()
        .find(|p| p.numero == page)
        .ok_or_else(|| erreur(format!("page {page} absente de la mise en page")))?;
    let g = &mise.gabarit;
    let (largeur, hauteur) = g.page_mm();
    let mut image = ImageRvb::blanche(px(largeur), px(hauteur));
    for plage in &contenu.plages {
        let valeur = valeurs
            .get(plage.index)
            .ok_or_else(|| erreur(format!("plage {} sans valeur", plage.index)))?;
        let (x, y) = g.origine_plage(plage.rangee, plage.colonne);
        image.remplir(
            (px(x), px(y)),
            (px(x + g.cote_mm), px(y + g.cote_mm)),
            valeur.map(octet),
        );
    }

    let Bandeau {
        qr,
        x_qr: qx,
        caracteres,
    } = bandeau(g, &etiquette.code)?;
    let (zx, zy, zy1) = {
        let zone = g.zone_texte();
        (px(zone.x), px(zone.y), px(zone.y + zone.hauteur))
    };
    // Modules sombres dans le bandeau, en haut à droite ; sa marge blanche
    // tombe dans la marge de la page et l'écart aux plages.
    let modules = qr.width() as u32;
    for (i, couleur) in qr.to_colors().into_iter().enumerate() {
        if couleur == Color::Dark {
            let (mx, my) = (i as u32 % modules, i as u32 / modules);
            let (x, y) = (qx + mx * MODULE_PX, zy + my * MODULE_PX);
            image.remplir((x, y), (x + MODULE_PX, y + MODULE_PX), [0; 3]);
        }
    }

    let cellule = 8 * ECHELLE_TEXTE;
    for (rang, ligne) in etiquette.lignes.iter().enumerate() {
        let y = zy + rang as u32 * INTERLIGNE_PX;
        if y + cellule > zy1 {
            break;
        }
        for (k, c) in couper_ligne(ligne, caracteres).chars().enumerate() {
            dessiner_caractere(&mut image, c, (zx + k as u32 * cellule, y));
        }
    }
    Ok(image)
}

/// Motif 8 × 8 d'un caractère (ligne par ligne, bit 0 à gauche), s'il est
/// dans la police : ASCII, table latine (accents, « · », espace insécable
/// U+00A0) et « … ».
fn motif_police(c: char) -> Option<[u8; 8]> {
    match c as u32 {
        n @ 0..=0x7F => Some(BASIC_LEGACY[n as usize]),
        n @ 0xA0..=0xFF => Some(LATIN_LEGACY[(n - 0xA0) as usize]),
        0x2026 => Some(POINTS_DE_SUITE),
        _ => None,
    }
}

/// Le caractère est dessiné sur les pages tel quel ; sinon il sort en « ? ».
/// Une espace fine insécable (U+202F) ou une apostrophe courbe, par
/// exemple, ne le sont pas.
pub fn dessinable(c: char) -> bool {
    !c.is_control() && motif_police(c).is_some_and(|m| m != [0; 8] || c == ' ' || c == '\u{a0}')
}

/// Motif 8 × 8 d'un caractère (ligne par ligne, bit 0 à gauche) ; « ? » pour
/// un caractère que la police n'a pas.
fn motif(c: char) -> [u8; 8] {
    motif_police(c).unwrap_or(BASIC_LEGACY['?' as usize])
}

fn dessiner_caractere(image: &mut ImageRvb, c: char, (x, y): (u32, u32)) {
    for (rangee, bits) in motif(c).iter().enumerate() {
        for colonne in 0..8 {
            if bits & (1 << colonne) != 0 {
                let x0 = x + colonne * ECHELLE_TEXTE;
                let y0 = y + rangee as u32 * ECHELLE_TEXTE;
                image.remplir((x0, y0), (x0 + ECHELLE_TEXTE, y0 + ECHELLE_TEXTE), [0; 3]);
            }
        }
    }
}
