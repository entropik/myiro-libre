//! Écarts ΔE00 plage par plage entre la mesure d'une seconde mire et ce que
//! le profil demande et prédit, couleurs hors gamut et statistiques.
//!
//! Trois valeurs ne sont jamais confondues pour une plage : la **demande**
//! (Lab relatif de la couleur sRVB voulue), la **valeur après
//! transformation** (envoyée à l'imprimante) et la **prédiction** (ce que le
//! profil annonce pour cette valeur). La mesure est gardée telle quelle et
//! ramenée au blanc du support pour la fidélité.

use std::fmt;

use colorimetrie::{blanc_d50, delta_e00, lab_vers_xyz, xyz_vers_lab, Inconnu, Lab, Xyz};
use serde::{Deserialize, Serialize};

use crate::serde_lab;

/// Nombre de pires plages gardées par famille.
pub const PIRES: usize = 10;

/// Une plage de la seconde mire : ce qui a été demandé (RVB, de 0 à 1) et ce
/// qui a été mesuré (Lab absolu).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlageAVerifier {
    pub id: String,
    pub rvb: [f64; 3],
    #[serde(with = "serde_lab::un")]
    pub mesure: Lab,
}

/// Ce que le profil dit des plages, dans leur ordre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValeursProfil {
    /// Lab relatif des RVB lus comme du sRVB ; `None` pour une mire
    /// imprimée sans gestion des couleurs.
    #[serde(with = "serde_lab::option_liste")]
    pub demande: Option<Vec<Lab>>,
    /// Valeur après transformation, de 0 à 1 (les RVB eux-mêmes sans
    /// gestion des couleurs).
    pub appareil: Vec<[f64; 3]>,
    /// Prédiction du profil pour la valeur après transformation, en
    /// colorimétrie relative puis absolue.
    #[serde(with = "serde_lab::liste")]
    pub prediction_rel: Vec<Lab>,
    #[serde(with = "serde_lab::liste")]
    pub prediction_abs: Vec<Lab>,
    /// Lab absolu que le profil prédit pour le blanc de l'appareil.
    #[serde(with = "serde_lab::un")]
    pub blanc_support: Lab,
}

/// Le résultat d'une plage : ses valeurs et ses trois écarts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatPlage {
    pub id: String,
    pub rvb: [f64; 3],
    #[serde(with = "serde_lab::option")]
    pub demande: Option<Lab>,
    pub appareil: [f64; 3],
    /// Prédiction absolue du profil pour la valeur après transformation.
    #[serde(with = "serde_lab::un")]
    pub prediction: Lab,
    /// Mesure telle quelle (absolue).
    #[serde(with = "serde_lab::un")]
    pub mesure: Lab,
    /// Mesure ramenée au blanc du support (relative).
    #[serde(with = "serde_lab::un")]
    pub mesure_rel: Lab,
    /// ΔE00 entre la demande et la mesure relative.
    pub ecart_fidelite: Option<f64>,
    /// ΔE00 entre la prédiction absolue et la mesure.
    pub ecart_prediction: f64,
    /// ΔE00 entre la demande et la prédiction relative : au-delà du seuil de
    /// gamut, la couleur demandée est hors gamut.
    pub ecart_gamut: Option<f64>,
}

/// L'écart d'une plage, pour les statistiques.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EcartPlage {
    pub id: String,
    pub de00: f64,
}

/// Moyenne, 95e centile (rang le plus proche) et maximum d'une famille, et
/// ses [`PIRES`] plus grands écarts, du plus grand au plus petit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Statistiques {
    pub plages: usize,
    pub moyenne: f64,
    pub p95: f64,
    pub max: f64,
    pub pires: Vec<EcartPlage>,
}

/// Les résultats d'une vérification, plage par plage et par famille.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resultats {
    pub plages: Vec<ResultatPlage>,
    /// Seuil de gamut qui a partagé les plages (ΔE00).
    pub seuil_gamut: f64,
    /// Fidélité des plages dans le gamut ; `None` sans demande ou sans plage
    /// dans le gamut.
    pub fidelite: Option<Statistiques>,
    /// Fidélité des plages hors gamut ; `None` sans plage hors gamut.
    pub hors_gamut: Option<Statistiques>,
    /// Accord avec la prédiction, toutes plages.
    pub prediction: Option<Statistiques>,
}

/// Pourquoi rien n'a été comparé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusVerification {
    /// Aucune plage.
    Vide,
    /// Les valeurs du profil ne sont pas aussi nombreuses que les plages.
    TaillesDifferentes,
    /// Une valeur de cette plage (ou du blanc du support) n'est pas un
    /// nombre utilisable.
    ValeurNonFinie { plage: String },
}

impl RefusVerification {
    /// Clé du texte affiché (`app/src/textes.rs`).
    pub fn cle(&self) -> &'static str {
        match self {
            RefusVerification::Vide => "verif.refus.vide",
            RefusVerification::TaillesDifferentes => "verif.refus.tailles",
            RefusVerification::ValeurNonFinie { .. } => "verif.refus.valeur",
        }
    }
}

impl fmt::Display for RefusVerification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefusVerification::Vide => f.write_str("aucune plage à vérifier"),
            RefusVerification::TaillesDifferentes => {
                f.write_str("le profil n'a pas rendu une valeur par plage")
            }
            RefusVerification::ValeurNonFinie { plage } => {
                write!(f, "une valeur de la plage {plage} est inutilisable")
            }
        }
    }
}

impl std::error::Error for RefusVerification {}

/// Ramène une mesure absolue au blanc du support (règle ICC de la
/// colorimétrie relative) : chaque composante XYZ est multipliée par le
/// rapport du blanc D50 au blanc du support. Le blanc du support devient
/// L = 100, a = b = 0.
pub fn vers_relatif(mesure: Lab, blanc_support: Lab) -> Result<Lab, Inconnu> {
    let d50 = blanc_d50();
    let m = lab_vers_xyz(mesure, d50)?;
    let s = lab_vers_xyz(blanc_support, d50)?;
    if [s.x, s.y, s.z].iter().any(|&v| v <= 0.0) {
        return Err(Inconnu::BlancInvalide);
    }
    let relatif = Xyz {
        x: m.x * d50.x / s.x,
        y: m.y * d50.y / s.y,
        z: m.z * d50.z / s.z,
    };
    xyz_vers_lab(relatif, d50)
}

/// Statistiques d'une liste d'écarts ; rien pour une liste vide.
pub fn statistiques(ecarts: &[EcartPlage]) -> Option<Statistiques> {
    if ecarts.is_empty() {
        return None;
    }
    let n = ecarts.len();
    let mut tries: Vec<&EcartPlage> = ecarts.iter().collect();
    // Du plus grand au plus petit ; à égalité, l'ordre de la mire.
    tries.sort_by(|a, b| b.de00.total_cmp(&a.de00));
    let moyenne = ecarts.iter().map(|e| e.de00).sum::<f64>() / n as f64;
    // Rang le plus proche, compté à partir de 1 dans l'ordre croissant.
    let rang = ((0.95 * n as f64).ceil() as usize).clamp(1, n);
    let p95 = tries[n - rang].de00;
    Some(Statistiques {
        plages: n,
        moyenne,
        p95,
        max: tries[0].de00,
        pires: tries.into_iter().take(PIRES).cloned().collect(),
    })
}

fn fini(valeurs: &[f64]) -> bool {
    valeurs.iter().all(|v| v.is_finite())
}

fn lab_fini(lab: &Lab) -> bool {
    fini(&[lab.l, lab.a, lab.b])
}

/// Compare chaque plage mesurée à ce que le profil demande et prédit, puis
/// partage les plages selon `seuil_gamut` (voir [`reclasser`]).
pub fn comparer(
    plages: &[PlageAVerifier],
    valeurs: &ValeursProfil,
    seuil_gamut: f64,
) -> Result<Resultats, RefusVerification> {
    if plages.is_empty() {
        return Err(RefusVerification::Vide);
    }
    let n = plages.len();
    if valeurs.appareil.len() != n
        || valeurs.prediction_rel.len() != n
        || valeurs.prediction_abs.len() != n
        || valeurs.demande.as_ref().is_some_and(|d| d.len() != n)
    {
        return Err(RefusVerification::TaillesDifferentes);
    }
    if !lab_fini(&valeurs.blanc_support) {
        return Err(RefusVerification::ValeurNonFinie {
            plage: "blanc_support".to_string(),
        });
    }
    let mut resultats = Vec::with_capacity(n);
    for (i, plage) in plages.iter().enumerate() {
        let refus = || RefusVerification::ValeurNonFinie {
            plage: plage.id.clone(),
        };
        let demande = valeurs.demande.as_ref().map(|d| d[i]);
        let appareil = valeurs.appareil[i];
        let (rel, abs) = (valeurs.prediction_rel[i], valeurs.prediction_abs[i]);
        if !fini(&plage.rvb)
            || !fini(&appareil)
            || !lab_fini(&plage.mesure)
            || !lab_fini(&rel)
            || !lab_fini(&abs)
            || demande.is_some_and(|d| !lab_fini(&d))
        {
            return Err(refus());
        }
        let mesure_rel = vers_relatif(plage.mesure, valeurs.blanc_support).map_err(|_| refus())?;
        let ecart = |a: Lab, b: Lab| delta_e00(a, b).map_err(|_| refus());
        resultats.push(ResultatPlage {
            id: plage.id.clone(),
            rvb: plage.rvb,
            demande,
            appareil,
            prediction: abs,
            mesure: plage.mesure,
            mesure_rel,
            ecart_fidelite: demande.map(|d| ecart(d, mesure_rel)).transpose()?,
            ecart_prediction: ecart(abs, plage.mesure)?,
            ecart_gamut: demande.map(|d| ecart(d, rel)).transpose()?,
        });
    }
    Ok(classer(resultats, seuil_gamut))
}

/// Les mêmes écarts, partagés selon un autre seuil de gamut.
pub fn reclasser(resultats: &Resultats, seuil_gamut: f64) -> Resultats {
    classer(resultats.plages.clone(), seuil_gamut)
}

/// Une plage est hors gamut quand son écart au gamut dépasse le seuil.
fn classer(plages: Vec<ResultatPlage>, seuil_gamut: f64) -> Resultats {
    let fidelite = |hors: bool| -> Vec<EcartPlage> {
        plages
            .iter()
            .filter_map(|p| {
                let (de00, gamut) = (p.ecart_fidelite?, p.ecart_gamut?);
                ((gamut > seuil_gamut) == hors).then(|| EcartPlage {
                    id: p.id.clone(),
                    de00,
                })
            })
            .collect()
    };
    let prediction: Vec<EcartPlage> = plages
        .iter()
        .map(|p| EcartPlage {
            id: p.id.clone(),
            de00: p.ecart_prediction,
        })
        .collect();
    Resultats {
        fidelite: statistiques(&fidelite(false)),
        hors_gamut: statistiques(&fidelite(true)),
        prediction: statistiques(&prediction),
        seuil_gamut,
        plages,
    }
}
