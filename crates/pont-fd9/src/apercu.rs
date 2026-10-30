//! Calage rendu à l'application (ticket #34) : les points que le FD-9 va
//! viser, et l'image de la feuille qu'il a prise, réduite à environ 1 000
//! pixels de haut. L'application peut ainsi montrer, comme l'onglet
//! « Measured Point » de FD-S2w, un point sur chaque plage (fiches
//! `docs/abi/FD9_GetMeasurePoint.md` et `FD9_GetChartImage.md`).
//!
//! L'image n'est qu'un aperçu : si la DLL ne la rend pas, la feuille continue
//! et le calage part sans elle. Elle peut montrer ce que l'opérateur a écrit
//! sur la feuille : le pont ne l'écrit nulle part, il la donne seulement dans
//! sa réponse.

use crate::calage::Calage;
use crate::{SdkFd9, Session};
use fd9_sys::{PointMesure, IMAGE_FEUILLE_HAUTEUR_MAX, IMAGE_FEUILLE_LARGEUR, IMAGE_FEUILLE_PPP};
use pont_protocole::{CalageFeuille, ImageFeuille, PointVise};

/// Hauteur visée pour l'image réduite, en pixels.
pub const HAUTEUR_APERCU: u32 = 1_000;

/// Diviseurs de 300, dans l'ordre : l'image réduite garde une résolution
/// entière en ppp.
const FACTEURS: [u32; 18] = [
    1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 25, 30, 50, 60, 75, 100, 150, 300,
];

impl<S: SdkFd9> Session<S> {
    /// Calage de la dernière feuille reconnue, rendu une seule fois (la
    /// réponse au client le prend).
    pub fn prendre_calage(&mut self) -> Option<CalageFeuille> {
        self.calage.take()
    }

    /// Retient les points visés et l'image de la feuille, lue à l'état 7.
    /// `taille_capture` : taille de l'image prise (`FD9_GetCaptureResult`),
    /// qui borne le tampon de l'image du papier.
    pub(crate) fn retenir_calage(
        &mut self,
        calage: &Calage,
        points: &[PointMesure],
        taille_capture: u32,
    ) {
        let image = self.lire_image_feuille(taille_capture);
        let (dx, dy) = calage.decalage;
        self.calage = Some(CalageFeuille {
            decalage: [dx, dy],
            ecart_max: calage.ecart_max,
            points: points
                .iter()
                .map(|p| PointVise {
                    zone: p.zone,
                    rangee: p.rangee,
                    colonne: p.colonne,
                    x: p.x,
                    y: p.y,
                })
                .collect(),
            image,
        });
    }

    /// `FD9_GetChartImage` dans un tampon de 3 638 pixels de large, assez
    /// haut pour l'image prise, puis réduction. Tout échec est journalisé et
    /// rend `None`.
    fn lire_image_feuille(&mut self, taille_capture: u32) -> Option<ImageFeuille> {
        let mut largeur = IMAGE_FEUILLE_LARGEUR;
        let mut hauteur = taille_capture
            .div_ceil(largeur)
            .clamp(1, IMAGE_FEUILLE_HAUTEUR_MAX);
        let place = largeur as usize * hauteur as usize;
        let mut gris = vec![0u8; place];
        let code = self
            .sdk
            .image_feuille(&mut largeur, &mut hauteur, &mut gris);
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!(
                "FD9_GetChartImage : code {code}, code interne {interne} ; calage rendu sans image"
            ));
            return None;
        }
        let pixels = u64::from(largeur) * u64::from(hauteur);
        if pixels == 0 || pixels > place as u64 {
            self.journal.push(format!(
                "FD9_GetChartImage : image de {largeur} × {hauteur} annoncée pour un tampon de {place} octets ; ignorée"
            ));
            return None;
        }
        gris.truncate(pixels as usize);
        let Some(image) = reduire(largeur, hauteur, &gris) else {
            self.journal.push(format!(
                "FD9_GetChartImage : image de {largeur} × {hauteur} impossible à réduire à {HAUTEUR_APERCU} pixels de haut ; ignorée"
            ));
            return None;
        };
        self.journal.push(format!(
            "FD9_GetChartImage : image de {largeur} × {hauteur}, réduite à {} × {} ({} ppp)",
            image.largeur(),
            image.hauteur(),
            image.ppp()
        ));
        Some(image)
    }
}

/// Plus petit diviseur de 300 qui ramène `hauteur` à `HAUTEUR_APERCU`
/// pixels au plus ; `None` si même le plus grand n'y suffit pas.
fn facteur(hauteur: u32) -> Option<u32> {
    FACTEURS
        .into_iter()
        .find(|&k| hauteur.div_ceil(k) <= HAUTEUR_APERCU)
}

/// Réduit une image grise de `largeur × hauteur` octets (non vide, à
/// 300 ppp) : chaque pixel est la moyenne, arrondie, d'un carré de k × k
/// pixels (moins au bord droit et en bas). `None` si l'image est vide, ne
/// fait pas `largeur × hauteur` octets, ou ne tient pas dans
/// `HAUTEUR_APERCU` pixels de haut.
fn reduire(largeur: u32, hauteur: u32, gris: &[u8]) -> Option<ImageFeuille> {
    if largeur == 0 || gris.len() != largeur as usize * hauteur as usize {
        return None;
    }
    let k = facteur(hauteur)?;
    let (l, h) = (largeur.div_ceil(k), hauteur.div_ceil(k));
    let (largeur, k_us) = (largeur as usize, k as usize);
    let mut sortie = Vec::with_capacity(l as usize * h as usize);
    let mut sommes = vec![0u64; l as usize];
    for rangees in gris.chunks(largeur * k_us) {
        sommes.iter_mut().for_each(|s| *s = 0);
        for rangee in rangees.chunks(largeur) {
            for (x, &v) in rangee.iter().enumerate() {
                sommes[x / k_us] += u64::from(v);
            }
        }
        let haut = (rangees.len() / largeur) as u64;
        for (bx, somme) in sommes.iter().enumerate() {
            let large = (largeur - bx * k_us).min(k_us) as u64;
            let n = haut * large;
            sortie.push(((somme + n / 2) / n) as u8);
        }
    }
    ImageFeuille::new(l, h, IMAGE_FEUILLE_PPP / k, sortie).ok()
}
