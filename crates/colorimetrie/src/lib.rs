//! Calculs de couleur de myiro-libre, partagés avec le futur RIP (ADR 0004).
//!
//! - spectre de réflexion (380 à 730 nm par 10 nm) vers XYZ puis Lab, pour
//!   l'illuminant D50 et l'observateur 2° ;
//! - Lab vers LCH ;
//! - écarts ΔE00 (CIEDE2000), ΔC et ΔH (CIELAB).
//!
//! Une valeur impossible à calculer est **inconnue** (voir `GLOSSARY.md`) :
//! les fonctions rendent alors `Err(Inconnu)`, jamais zéro.
//!
//! La crate ne dépend ni de Windows, ni de l'interface, ni des ponts : son
//! interface publique est un contrat.

mod tables;

use tables::PONDERATIONS;

/// Nombre de valeurs d'un spectre : 380 à 730 nm par pas de 10 nm.
pub const LONGUEUR_SPECTRE: usize = 36;

/// Raison pour laquelle une valeur est inconnue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inconnu {
    /// Le spectre n'a pas le nombre de valeurs attendu.
    LongueurSpectre { attendue: usize, recue: usize },
    /// Une valeur d'entrée n'est pas un nombre fini (NaN ou infini).
    ValeurNonFinie,
    /// Le blanc de référence a une composante nulle ou négative.
    BlancInvalide,
    /// La teinte d'une couleur sans chroma (un gris parfait) n'existe pas.
    TeinteSansChroma,
    /// Le calcul déborde (valeurs d'entrée extrêmes) : le résultat ne serait
    /// pas un nombre fini.
    ResultatNonFini,
}

/// Valeurs tristimulus, blanc de référence à Y = 100.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Couleur CIELAB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// Couleur en clarté, chroma et teinte (coordonnées polaires de Lab).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lch {
    pub l: f64,
    pub c: f64,
    /// Teinte en degrés, de 0 (inclus) à 360 (exclu) ; inconnue pour un gris
    /// parfait (chroma nulle).
    pub h: Result<f64, Inconnu>,
}

fn verifier_lab(lab: &Lab) -> Result<(), Inconnu> {
    if [lab.l, lab.a, lab.b].iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(Inconnu::ValeurNonFinie)
    }
}

/// Un résultat qui n'est pas un nombre fini est inconnu, jamais rendu tel quel.
fn fini(valeur: f64) -> Result<f64, Inconnu> {
    if valeur.is_finite() {
        Ok(valeur)
    } else {
        Err(Inconnu::ResultatNonFini)
    }
}

/// Angle de `atan2(b, a)` en degrés, ramené dans [0, 360) : un angle infime
/// négatif donnerait sinon exactement 360 par arrondi.
fn teinte_degres(b: f64, a: f64) -> f64 {
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    if h >= 360.0 {
        0.0
    } else {
        h
    }
}

/// Lab vers LCH.
pub fn lab_vers_lch(lab: Lab) -> Result<Lch, Inconnu> {
    verifier_lab(&lab)?;
    let c = fini(lab.a.hypot(lab.b))?;
    let h = if c == 0.0 {
        Err(Inconnu::TeinteSansChroma)
    } else {
        Ok(teinte_degres(lab.b, lab.a))
    };
    Ok(Lch { l: lab.l, c, h })
}

/// Écart de couleur CIEDE2000 (ΔE00), facteurs k_L = k_C = k_H = 1.
///
/// Suit les notes d'implémentation de Sharma, Wu et Dalal (2005) : teinte
/// moyenne et écart de teinte corrigés au-delà de 180°, teinte prise nulle
/// pour une chroma nulle (convention interne de la formule).
pub fn delta_e00(lab1: Lab, lab2: Lab) -> Result<f64, Inconnu> {
    verifier_lab(&lab1)?;
    verifier_lab(&lab2)?;
    const PUISSANCE_25_7: f64 = 6_103_515_625.0; // 25^7

    let c_moyen = (lab1.a.hypot(lab1.b) + lab2.a.hypot(lab2.b)) / 2.0;
    let c7 = c_moyen.powi(7);
    let g = 0.5 * (1.0 - (c7 / (c7 + PUISSANCE_25_7)).sqrt());
    let prime = |lab: &Lab| {
        let a = (1.0 + g) * lab.a;
        let c = a.hypot(lab.b);
        let h = if c == 0.0 {
            0.0
        } else {
            teinte_degres(lab.b, a)
        };
        (c, h)
    };
    let (c1, h1) = prime(&lab1);
    let (c2, h2) = prime(&lab2);

    let dl = lab2.l - lab1.l;
    let dc = c2 - c1;
    let produit_c = c1 * c2;
    let dh = if produit_c == 0.0 {
        0.0
    } else {
        let d = h2 - h1;
        if d > 180.0 {
            d - 360.0
        } else if d < -180.0 {
            d + 360.0
        } else {
            d
        }
    };
    let dh_grand = 2.0 * produit_c.sqrt() * (dh / 2.0).to_radians().sin();

    let l_moyen = (lab1.l + lab2.l) / 2.0;
    let c_moyen_prime = (c1 + c2) / 2.0;
    let h_moyen = if produit_c == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_moyen - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_moyen).to_radians().cos()
        + 0.32 * (3.0 * h_moyen + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_moyen - 63.0).to_radians().cos();
    let d_theta = 30.0 * (-((h_moyen - 275.0) / 25.0).powi(2)).exp();
    let c7p = c_moyen_prime.powi(7);
    let r_c = 2.0 * (c7p / (c7p + PUISSANCE_25_7)).sqrt();
    let l50 = (l_moyen - 50.0).powi(2);
    let s_l = 1.0 + 0.015 * l50 / (20.0 + l50).sqrt();
    let s_c = 1.0 + 0.045 * c_moyen_prime;
    let s_h = 1.0 + 0.015 * c_moyen_prime * t;
    let r_t = -(2.0 * d_theta).to_radians().sin() * r_c;

    let (tl, tc, th) = (dl / s_l, dc / s_c, dh_grand / s_h);
    fini((tl * tl + tc * tc + th * th + r_t * tc * th).sqrt())
}

/// Écart de chroma CIELAB ΔC*ab = C₂ − C₁, signé : positif quand la seconde
/// couleur est plus saturée.
pub fn delta_c(lab1: Lab, lab2: Lab) -> Result<f64, Inconnu> {
    verifier_lab(&lab1)?;
    verifier_lab(&lab2)?;
    fini(lab2.a.hypot(lab2.b) - lab1.a.hypot(lab1.b))
}

/// Écart de teinte CIELAB ΔH*ab = 2 √(C₁ C₂) sin(Δh / 2), signé : positif
/// quand la teinte de la seconde couleur tourne dans le sens direct (de a*
/// vers b*). Sa valeur absolue vaut √(ΔE*ab² − ΔL² − ΔC²) ; elle est nulle si
/// l'une des couleurs est un gris parfait.
pub fn delta_h(lab1: Lab, lab2: Lab) -> Result<f64, Inconnu> {
    verifier_lab(&lab1)?;
    verifier_lab(&lab2)?;
    let (c1, c2) = (fini(lab1.a.hypot(lab1.b))?, fini(lab2.a.hypot(lab2.b))?);
    if c1 == 0.0 || c2 == 0.0 {
        return Ok(0.0);
    }
    let dh = (lab2.b.atan2(lab2.a) - lab1.b.atan2(lab1.a)).to_degrees();
    let dh = (dh + 180.0).rem_euclid(360.0) - 180.0;
    fini(2.0 * (c1 * c2).sqrt() * (dh / 2.0).to_radians().sin())
}

/// Facteur de réflexion de la longueur d'onde de rang `i` des tables
/// (380 à 780 nm). Au-delà de 730 nm, le spectre mesuré s'arrête : on
/// prolonge sa dernière valeur, comme le recommande l'ASTM E308.
fn reflexion(spectre: &[f64], i: usize) -> f64 {
    spectre[i.min(LONGUEUR_SPECTRE - 1)]
}

fn integrer(spectre: &[f64]) -> Xyz {
    let (mut x, mut y, mut z, mut norme) = (0.0, 0.0, 0.0, 0.0);
    for (i, [s, xb, yb, zb]) in PONDERATIONS.iter().enumerate() {
        let r = reflexion(spectre, i);
        x += s * xb * r;
        y += s * yb * r;
        z += s * zb * r;
        norme += s * yb;
    }
    let k = 100.0 / norme;
    Xyz {
        x: k * x,
        y: k * y,
        z: k * z,
    }
}

/// Blanc de référence D50, 2°, calculé avec les mêmes tables que les
/// spectres (diffuseur parfait).
pub fn blanc_d50() -> Xyz {
    integrer(&[1.0; LONGUEUR_SPECTRE])
}

/// Spectre de réflexion (facteurs de 0 à 1, 380 à 730 nm par 10 nm) vers XYZ,
/// D50 et 2°.
pub fn spectre_vers_xyz(spectre: &[f64]) -> Result<Xyz, Inconnu> {
    if spectre.len() != LONGUEUR_SPECTRE {
        return Err(Inconnu::LongueurSpectre {
            attendue: LONGUEUR_SPECTRE,
            recue: spectre.len(),
        });
    }
    if spectre.iter().any(|v| !v.is_finite()) {
        return Err(Inconnu::ValeurNonFinie);
    }
    let xyz = integrer(spectre);
    Ok(Xyz {
        x: fini(xyz.x)?,
        y: fini(xyz.y)?,
        z: fini(xyz.z)?,
    })
}

/// XYZ vers Lab, rapporté au blanc donné.
pub fn xyz_vers_lab(xyz: Xyz, blanc: Xyz) -> Result<Lab, Inconnu> {
    let valeurs = [xyz.x, xyz.y, xyz.z, blanc.x, blanc.y, blanc.z];
    if valeurs.iter().any(|v| !v.is_finite()) {
        return Err(Inconnu::ValeurNonFinie);
    }
    if [blanc.x, blanc.y, blanc.z].iter().any(|&v| v <= 0.0) {
        return Err(Inconnu::BlancInvalide);
    }
    const EPSILON: f64 = 216.0 / 24389.0;
    const KAPPA: f64 = 24389.0 / 27.0;
    let f = |t: f64| {
        if t > EPSILON {
            t.cbrt()
        } else {
            (KAPPA * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(xyz.x / blanc.x), f(xyz.y / blanc.y), f(xyz.z / blanc.z));
    Ok(Lab {
        l: fini(116.0 * fy - 16.0)?,
        a: fini(500.0 * (fx - fy))?,
        b: fini(200.0 * (fy - fz))?,
    })
}

/// Spectre de réflexion vers Lab, D50 et 2°.
pub fn spectre_vers_lab(spectre: &[f64]) -> Result<Lab, Inconnu> {
    xyz_vers_lab(spectre_vers_xyz(spectre)?, blanc_d50())
}
