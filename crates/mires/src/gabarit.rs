//! Gabarit des pages en bandes du MYIRO-1. Le FD-9 n'a pas de gabarit : sa
//! mire est RGB-2520, reprise telle quelle.
//!
//! La page est en portrait (demande du mainteneur, 28 octobre 2026). Un
//! bandeau de texte et le QR code occupent le haut ; les bandes traversent
//! la largeur, se lisent de gauche à droite et sont centrées, séparées par
//! un blanc étroit.

use serde::{Deserialize, Serialize};

use crate::{ErreurMire, Format};

/// Distance minimale entre un texte et une plage (manuel de FD-S2w, p. 35 à
/// 37, appliquée aussi au MYIRO-1).
pub const DISTANCE_TEXTE_MM: f64 = 2.0;
/// Côté minimal d'une plage lue en bande (manuel MYIRO-1).
pub const MYIRO1_COTE_MIN_MM: f64 = 7.0;
/// Côté par défaut : choix, à l'intérieur des limites du manuel, à vérifier à
/// l'essai (seule la plage de 20 mm a été mesurée, le 7 octobre 2026).
pub const MYIRO1_COTE_PAR_DEFAUT_MM: f64 = 8.0;
/// Blanc papier aux deux bouts d'une bande (manuel MYIRO-1).
pub const MYIRO1_BLANC_BOUT_MM: f64 = 10.0;
/// Longueur maximale d'une bande, blancs compris (manuel MYIRO-1).
pub const MYIRO1_BANDE_MAX_MM: f64 = 257.0;
/// Blanc entre deux bandes : le manuel ne fixe aucun écart entre bandes.
/// À confirmer à l'impression avec la règle MY-A02 ; repli à 5 mm.
pub const MYIRO1_ECART_BANDES_MM: f64 = 3.0;
/// Marge technique d'impression en feuille, sur les quatre bords : donnée
/// par le mainteneur (0,5 cm, 28 octobre 2026).
pub const MYIRO1_MARGE_PAGE_MM: f64 = 5.0;
/// Bandeau du haut (texte et QR code), écart au texte compris. Choix, à
/// vérifier.
pub const MYIRO1_BANDEAU_MM: f64 = 22.0;
/// Écart ΔE*ab minimal entre deux plages voisines d'une bande (manuel
/// MYIRO-1 : strictement supérieur).
pub const CONTRASTE_MIN_DE76: f64 = 10.0;

/// Rectangle sur la page, en millimètres depuis le coin haut gauche.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// Gabarit des pages en bandes du MYIRO-1 pour un format et un côté de
/// plage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gabarit {
    pub format: Format,
    pub cote_mm: f64,
}

impl Gabarit {
    /// Gabarit au côté par défaut ([`MYIRO1_COTE_PAR_DEFAUT_MM`]).
    pub fn par_defaut(format: Format) -> Gabarit {
        Gabarit {
            format,
            cote_mm: MYIRO1_COTE_PAR_DEFAUT_MM,
        }
    }

    /// Gabarit au côté donné. Refusé sous le minimum du manuel, ou si aucune
    /// bande ne tient sur la page.
    pub fn nouveau(format: Format, cote_mm: f64) -> Result<Gabarit, ErreurMire> {
        let (min, max) = Gabarit::limites_cote();
        let gabarit = Gabarit { format, cote_mm };
        if !cote_mm.is_finite() || cote_mm < min || gabarit.plages_par_page() == 0 {
            return Err(ErreurMire::CoteHorsLimites { min, max });
        }
        Ok(gabarit)
    }

    /// Limites du côté de plage : le manuel donne un minimum, pas de maximum.
    pub fn limites_cote() -> (f64, Option<f64>) {
        (MYIRO1_COTE_MIN_MM, None)
    }

    /// Largeur et hauteur de la page en portrait, en millimètres.
    pub fn page_mm(&self) -> (f64, f64) {
        self.format.dimensions()
    }

    /// Plages par bande : la bande, blancs compris, tient en 257 mm et dans
    /// la largeur de la page.
    pub fn colonnes(&self) -> usize {
        if !self.cote_mm.is_finite() || self.cote_mm <= 0.0 {
            return 0;
        }
        let bande = MYIRO1_BANDE_MAX_MM - 2.0 * MYIRO1_BLANC_BOUT_MM;
        let page = self.page_mm().0 - 2.0 * MYIRO1_MARGE_PAGE_MM - 2.0 * MYIRO1_BLANC_BOUT_MM;
        (bande.min(page) / self.cote_mm).floor().max(0.0) as usize
    }

    /// Bandes par page : le plus grand n tel que n × côté + (n − 1) × écart
    /// tienne sous le bandeau, marges comprises.
    pub fn rangees(&self) -> usize {
        if !self.cote_mm.is_finite() || self.cote_mm <= 0.0 {
            return 0;
        }
        let place = self.page_mm().1 - 2.0 * MYIRO1_MARGE_PAGE_MM - MYIRO1_BANDEAU_MM;
        ((place + MYIRO1_ECART_BANDES_MM) / (self.cote_mm + MYIRO1_ECART_BANDES_MM))
            .floor()
            .max(0.0) as usize
    }

    pub fn plages_par_page(&self) -> usize {
        self.colonnes() * self.rangees()
    }

    /// Nombre de pages pour `nombre` plages.
    pub fn pages(&self, nombre: usize) -> usize {
        nombre.div_ceil(self.plages_par_page().max(1))
    }

    /// Coin haut gauche d'une plage, en millimètres. Les plages d'une bande
    /// sont jointives ; la bande, blancs de bout compris, est centrée dans la
    /// largeur de la page.
    pub fn origine_plage(&self, rangee: usize, colonne: usize) -> (f64, f64) {
        let debut = (self.page_mm().0 - self.colonnes() as f64 * self.cote_mm) / 2.0;
        let x = debut + colonne as f64 * self.cote_mm;
        let y = MYIRO1_MARGE_PAGE_MM
            + MYIRO1_BANDEAU_MM
            + rangee as f64 * (self.cote_mm + MYIRO1_ECART_BANDES_MM);
        (x, y)
    }

    /// Bandeau du haut, pour le texte et le QR code : il s'arrête à
    /// [`DISTANCE_TEXTE_MM`] des plages.
    pub fn zone_texte(&self) -> Rectangle {
        Rectangle {
            x: MYIRO1_MARGE_PAGE_MM,
            y: MYIRO1_MARGE_PAGE_MM,
            largeur: self.page_mm().0 - 2.0 * MYIRO1_MARGE_PAGE_MM,
            hauteur: MYIRO1_BANDEAU_MM - DISTANCE_TEXTE_MM,
        }
    }
}
