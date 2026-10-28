//! Mesures embarquées dans un profil (balise `targ`), lues pour la fidélité
//! et la répétabilité. Elles sont lues par `cgats::lire_sans_condition`, qui suit ses règles
//! d'échelle RVB : rien n'est deviné.

use cgats::MesureImportee;
use pont_protocole::{ConditionMesure, Echantillonnage, Info};

/// Clés des raisons pour lesquelles les mesures d'un profil ne servent pas.
pub const CLES_FIDELITE: [&str; 6] = [
    SANS_MESURES,
    ILLISIBLE,
    ECHELLE_SPECTRE,
    RVB_INCONNUS,
    RVB_HORS_ECHELLE,
    LAB_INCONNU,
];

/// Le profil n'embarque aucune mesure.
pub const SANS_MESURES: &str = "comparer.fidelite.sans_mesures";
const ILLISIBLE: &str = "comparer.fidelite.illisible";
/// Spectres `SPECTRAL_NM_` dont l'échelle ne se décide pas
/// (`docs/formats/cgats.md`).
const ECHELLE_SPECTRE: &str = "comparer.fidelite.echelle_spectre";
const RVB_INCONNUS: &str = "comparer.fidelite.rvb_inconnus";
const RVB_HORS_ECHELLE: &str = "comparer.fidelite.rvb_hors_echelle";
pub const LAB_INCONNU: &str = "comparer.fidelite.lab_inconnu";

/// Échantillonnage des spectres que `colorimetrie` calcule : 380 à 730 nm
/// par 10 nm.
const SPECTRE_CALCULABLE: Echantillonnage = Echantillonnage {
    debut_nm: 380,
    pas_nm: 10,
};

/// Les mesures d'un profil, avec ce qui n'est que supposé ou calculé.
#[derive(Clone, Debug, PartialEq)]
pub struct MesuresProfil {
    pub mesures: Vec<gamut::Mesure>,
    /// Condition de mesure ; inconnue quand le fichier ne la dit pas (mesures
    /// qu'ArgyllCMS embarque, venues du `.ti3` de myiro-libre).
    pub condition: Option<ConditionMesure>,
    /// La condition est déduite d'une convention du fabricant.
    pub condition_supposee: bool,
    /// L'échelle des RVB est déduite d'une convention de logiciel.
    pub rvb_supposes: bool,
    /// Un Lab au moins a été calculé par myiro-libre depuis le spectre (D50,
    /// 2°), faute de Lab dans le fichier.
    pub lab_calcule: bool,
}

/// Lit les mesures embarquées `targ` d'un profil. La fidélité et « mêmes
/// mesures » n'ont besoin que des RVB et des Lab : un fichier sans condition
/// de mesure est lu, sous une condition inconnue ([`cgats::lire_sans_condition`]).
/// Sinon, la condition est la seule présente, ou M1 si le fichier en donne
/// plusieurs. Les RVB suivent l'échelle du fichier. Le Lab est pris dans
/// le fichier ; à défaut, il est calculé depuis le spectre. Les erreurs
/// sont des clés du catalogue de textes.
pub fn mesures_embarquees(targ: &[u8]) -> Result<MesuresProfil, &'static str> {
    let mesure = cgats::lire_sans_condition(&cgats::decoder(targ)).map_err(|e| {
        eprintln!("mesures embarquées : {e}");
        if e.echelle_spectre_indecidable() {
            ECHELLE_SPECTRE
        } else {
            ILLISIBLE
        }
    })?;
    if mesure.plages.is_empty() {
        return Err(SANS_MESURES);
    }
    let presentes: Vec<usize> = (0..3)
        .filter(|&e| mesure.conditions[e].valeur().is_some())
        .collect();
    let e = presentes
        .iter()
        .copied()
        .find(|&e| mesure.conditions[e].valeur() == Some(&ConditionMesure::M1))
        .or(presentes.first().copied())
        // Sans condition écrite, les plages sont dans l'emplacement `m0`.
        .unwrap_or(0);
    let condition = mesure.conditions[e].valeur().copied();

    let mut lab_calcule = false;
    let mut mesures = Vec::with_capacity(mesure.plages.len());
    for p in &mesure.plages {
        let rvb = p.rvb.valeur().ok_or(RVB_INCONNUS)?;
        if rvb.iter().any(|c| !(0.0..=1.0).contains(c)) {
            return Err(RVB_HORS_ECHELLE);
        }
        let lab = match p.lab[e].valeur() {
            Some(lab) => lab.valeurs().map(f64::from),
            None => {
                lab_calcule = true;
                lab_du_spectre(&mesure, p, e).ok_or(LAB_INCONNU)?
            }
        };
        mesures.push(gamut::Mesure {
            rvb: rvb.map(f64::from),
            lab,
        });
    }
    Ok(MesuresProfil {
        mesures,
        condition,
        condition_supposee: matches!(mesure.conditions[e], Info::Supposee(_)),
        rvb_supposes: mesure
            .plages
            .iter()
            .any(|p| matches!(p.rvb, Info::Supposee(_))),
        lab_calcule,
    })
}

/// Lab calculé depuis le spectre de l'emplacement `e`, si le fichier en
/// nomme les longueurs d'onde et qu'elles sont celles du calcul.
fn lab_du_spectre(
    mesure: &MesureImportee,
    plage: &cgats::PlageImportee,
    e: usize,
) -> Option<[f64; 3]> {
    if mesure.longueurs_onde.valeur() != Some(&SPECTRE_CALCULABLE) {
        return None;
    }
    let spectre: Vec<f64> = plage.spectres[e]
        .valeur()?
        .iter()
        .map(|v| f64::from(*v))
        .collect();
    let lab = colorimetrie::spectre_vers_lab(&spectre).ok()?;
    Some([lab.l, lab.a, lab.b])
}
