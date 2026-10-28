//! Mesure d'une feuille au FD-9 : description de la mire envoyée au pont, et
//! ce que le pont a lu de la feuille (QR, reconnaissance des plages).
//!
//! Unité : 1/100 mm, origine en haut à gauche de la feuille, comme le SDK
//! (fiche `docs/abi/FD9_StartPatchRecognition.md`).

use serde::{Deserialize, Serialize};

use crate::mesure::{ErreurMesure, Info};

/// Zones qu'une feuille peut porter : borne du SDK.
pub const ZONES_FEUILLE_MAX: usize = 50;

/// Une feuille à mesurer au FD-9. Vérifiée à la lecture comme par
/// [`DefinitionFeuille::verifier`] : une définition incohérente n'atteint
/// jamais le pont.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DefinitionLue")]
pub struct DefinitionFeuille {
    /// Début attendu du texte du QR (« RGB-2520 - 1P MY9 »).
    pub nom: String,
    pub largeur: u32,
    pub hauteur: u32,
    /// 1 à 50 zones, dans l'ordre de lecture des plages.
    pub zones: Vec<ZoneFeuille>,
}

/// Une grille de plages : origine de la première plage, taille d'une plage,
/// pas d'une plage à la suivante. Plages lues rangée par rangée, de haut en
/// bas, de gauche à droite dans chaque rangée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneFeuille {
    pub gauche: u32,
    pub haut: u32,
    pub largeur_plage: u32,
    pub hauteur_plage: u32,
    pub pas_x: u32,
    pub pas_y: u32,
    pub rangees: u32,
    pub colonnes: u32,
}

/// Forme lue avant vérification.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionLue {
    nom: String,
    largeur: u32,
    hauteur: u32,
    zones: Vec<ZoneFeuille>,
}

impl TryFrom<DefinitionLue> for DefinitionFeuille {
    type Error = ErreurMesure;
    fn try_from(d: DefinitionLue) -> Result<Self, ErreurMesure> {
        let definition = DefinitionFeuille {
            nom: d.nom,
            largeur: d.largeur,
            hauteur: d.hauteur,
            zones: d.zones,
        };
        definition.verifier()?;
        Ok(definition)
    }
}

fn refus(detail: String) -> Result<(), ErreurMesure> {
    Err(ErreurMesure(detail))
}

impl DefinitionFeuille {
    /// Nom non vide, 1 à 50 zones, chaque zone non vide et tenue dans la
    /// feuille : `gauche + pas × colonnes ≤ largeur`, comme le contrôle du SDK,
    /// et de même en hauteur.
    pub fn verifier(&self) -> Result<(), ErreurMesure> {
        if self.nom.trim().is_empty() {
            return refus("feuille : nom vide".into());
        }
        if self.zones.is_empty() || self.zones.len() > ZONES_FEUILLE_MAX {
            return refus(format!(
                "feuille : {} zones (de 1 à {ZONES_FEUILLE_MAX})",
                self.zones.len()
            ));
        }
        for (n, z) in self.zones.iter().enumerate() {
            let n = n + 1;
            for (quoi, valeur) in [
                ("rangées", z.rangees),
                ("colonnes", z.colonnes),
                ("pas horizontal", z.pas_x),
                ("pas vertical", z.pas_y),
                ("largeur de plage", z.largeur_plage),
                ("hauteur de plage", z.hauteur_plage),
            ] {
                if valeur == 0 {
                    return refus(format!("feuille : zone {n}, {quoi} nul"));
                }
            }
            // En u64 : aucun nombre reçu ne peut faire déborder le calcul.
            let droite = u64::from(z.gauche) + u64::from(z.pas_x) * u64::from(z.colonnes);
            let bas = u64::from(z.haut) + u64::from(z.pas_y) * u64::from(z.rangees);
            if droite > u64::from(self.largeur) || bas > u64::from(self.hauteur) {
                return refus(format!(
                    "feuille : zone {n} jusqu'à ({droite} ; {bas}), hors de la feuille {} × {}",
                    self.largeur, self.hauteur
                ));
            }
        }
        Ok(())
    }

    /// Nombre total de plages, toutes zones comprises.
    pub fn plages(&self) -> usize {
        self.zones
            .iter()
            .map(|z| z.rangees as usize * z.colonnes as usize)
            .sum()
    }
}

/// Ce que le pont a lu d'une feuille avant de la mesurer : réponse de la
/// prénumérisation, et champ `feuille` de la provenance d'une mesure en
/// feuille.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LectureFeuille {
    /// Texte libre du QR ; inconnu si le QR est absent ou illisible.
    pub qr: Info<String>,
    /// Zones et plages reconnues par le SDK.
    pub zones: u32,
    pub plages: u32,
    /// Fiabilité rendue par le SDK, brute : son sens n'est pas établi.
    pub fiabilite: u32,
    /// Plus grand écart d'origine ou de pas entre la reconnaissance et la
    /// définition, en 1/100 mm.
    pub ecart_max: u32,
}
