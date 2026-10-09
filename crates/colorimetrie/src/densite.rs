//! Densités par réflexion depuis un spectre : statuts E et T (cyan, magenta,
//! jaune) et densité visuelle ISO, absolues ou relatives au papier.
//!
//! Méthode d'ArgyllCMS (`xsp_density`, ce que `spotread -d` affiche) : les
//! pondérations et le spectre sont lus tous les 1 nm par interpolation de
//! Lagrange d'ordre 3 sur leurs points au pas de 10 nm (bornes prolongées par
//! leur dernière valeur), le spectre pondéré est divisé par la somme des
//! pondérations, et la densité est moins son logarithme décimal. Le spectre
//! doit être mesuré en condition M0 (ISO 13655, compatible avec l'ISO 5-3) :
//! c'est à l'appelant de le vérifier.
//!
//! Différence voulue avec ArgyllCMS : un facteur de réflexion pondéré nul ou
//! négatif rend une densité inconnue, et non une densité plafonnée à 8.

use crate::tables_densite::{
    Ponderation, E_BLEU, E_ROUGE, E_VERT, T_BLEU, T_ROUGE, T_VERT, VISUELLE_ISO,
};
use crate::{fini, Inconnu, LONGUEUR_SPECTRE};

/// Statut de densité (ISO 5-3) : il fixe les filtres qui pèsent le spectre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatutDensite {
    /// Le plus courant en Europe.
    E,
    /// Le plus courant aux États-Unis.
    T,
}

/// Densités d'une plage. Le cyan se lit derrière le filtre rouge, le magenta
/// derrière le vert, le jaune derrière le bleu ; la densité visuelle ISO est
/// la même pour les deux statuts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Densites {
    pub cyan: f64,
    pub magenta: f64,
    pub jaune: f64,
    pub visuel: f64,
}

/// Premier point du spectre mesuré et pas, en nm.
const DEBUT_SPECTRE_NM: f64 = 380.0;
const PAS_NM: f64 = 10.0;

/// Valeur au point `nm` d'une suite au pas de 10 nm commençant à `debut`,
/// interpolée comme le fait ArgyllCMS (`getval_raw_xspec_poly3`) : Lagrange
/// sur quatre points, `nm` ramené dans les bornes de la suite.
fn lagrange(valeurs: &[f64], debut: f64, nm: f64) -> f64 {
    let n = valeurs.len();
    let fin = debut + PAS_NM * (n - 1) as f64;
    let nm = nm.clamp(debut, fin);
    let f = (nm - debut) / PAS_NM;
    let i = (f.floor() as usize).clamp(1, n - 3);
    let x = |k: usize| debut + (i - 1 + k) as f64 * PAS_NM;
    (0..4)
        .map(|a| {
            (0..4)
                .filter(|&b| b != a)
                .fold(valeurs[i - 1 + a], |produit, b| {
                    produit * (nm - x(b)) / (x(a) - x(b))
                })
        })
        .sum()
}

/// Facteur de réflexion pondéré par un filtre, relatif au blanc parfait.
fn reflexion_ponderee(spectre: &[f64], filtre: &Ponderation) -> f64 {
    let points = filtre.log10.len() as u32;
    let (mut somme, mut poids_total) = (0.0, 0.0);
    // Tous les 1 nm, du premier au dernier point du filtre.
    for k in 0..=(points - 1) * 10 {
        let nm = filtre.debut_nm + f64::from(k);
        let poids = 10f64.powf(lagrange(filtre.log10, filtre.debut_nm, nm));
        poids_total += poids;
        somme += poids * lagrange(spectre, DEBUT_SPECTRE_NM, nm);
    }
    somme / poids_total
}

fn densite(spectre: &[f64], filtre: &Ponderation) -> Result<f64, Inconnu> {
    let r = reflexion_ponderee(spectre, filtre);
    if r <= 0.0 {
        return Err(Inconnu::ResultatNonFini);
    }
    fini(-r.log10())
}

fn verifier_spectre(spectre: &[f64]) -> Result<(), Inconnu> {
    if spectre.len() != LONGUEUR_SPECTRE {
        return Err(Inconnu::LongueurSpectre {
            attendue: LONGUEUR_SPECTRE,
            recue: spectre.len(),
        });
    }
    if spectre.iter().any(|v| !v.is_finite()) {
        return Err(Inconnu::ValeurNonFinie);
    }
    Ok(())
}

/// Densités absolues (rapportées au blanc parfait) d'un spectre de réflexion
/// mesuré en M0 (facteurs de 0 à 1, 380 à 730 nm par 10 nm).
pub fn densites(spectre: &[f64], statut: StatutDensite) -> Result<Densites, Inconnu> {
    verifier_spectre(spectre)?;
    let [rouge, vert, bleu] = match statut {
        StatutDensite::E => [&E_ROUGE, &E_VERT, &E_BLEU],
        StatutDensite::T => [&T_ROUGE, &T_VERT, &T_BLEU],
    };
    Ok(Densites {
        cyan: densite(spectre, rouge)?,
        magenta: densite(spectre, vert)?,
        jaune: densite(spectre, bleu)?,
        visuel: densite(spectre, &VISUELLE_ISO)?,
    })
}

/// Densités relatives au papier : celles du spectre moins celles du papier,
/// avec le même statut. Le papier doit avoir été mesuré dans la même
/// condition que la plage : c'est à l'appelant de le vérifier.
pub fn densites_relatives(
    spectre: &[f64],
    papier: &[f64],
    statut: StatutDensite,
) -> Result<Densites, Inconnu> {
    let d = densites(spectre, statut)?;
    let p = densites(papier, statut)?;
    Ok(Densites {
        cyan: d.cyan - p.cyan,
        magenta: d.magenta - p.magenta,
        jaune: d.jaune - p.jaune,
        visuel: d.visuel - p.visuel,
    })
}
