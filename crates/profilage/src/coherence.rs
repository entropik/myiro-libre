//! Contrôle de cohérence des mesures avant un calcul de profil (ticket #1).
//!
//! Vérifie que les mesures correspondent aux plages : dégradé gris, blanc et
//! noir, doublons, voisines en RVB. Le contrôle ne corrige jamais les
//! mesures : il ne remet aucune plage en ordre, n'en écarte aucune et
//! n'applique aucun décalage. Il refuse ou il laisse passer, et il le dit.
//!
//! Les seuils sont des choix de l'assistant, acceptés par le mainteneur le
//! 28 octobre 2026 : aucune norme ne fixe de seuil pour ce contrôle. Ils
//! restent à vérifier sur des mesures réelles.

use std::collections::HashMap;

use colorimetrie::{delta_e00, Lab};
use serde::{Deserialize, Serialize};

use crate::plages::PlagesProfil;

/// Recul de L* toléré d'un gris au suivant, RVB croissant (choix, à vérifier).
pub const RETOUR_GRIS_MAX_L: f64 = 3.0;
/// Écart de L* toléré entre le blanc RVB 100 et la plage la plus claire
/// (choix, à vérifier).
pub const ECART_BLANC_MAX_L: f64 = 2.0;
/// Écart de L* toléré entre le noir RVB 0 et la plage la plus foncée
/// (choix, à vérifier).
pub const ECART_NOIR_MAX_L: f64 = 5.0;
/// Écart ΔE00 toléré entre deux plages de même RVB (choix, à vérifier : entre
/// MYIRO-1 et FD-9, les écarts mesurés le 7 octobre 2026 restaient sous 0,6).
pub const ECART_DOUBLONS_MAX_DE00: f64 = 3.0;
/// Écart ΔE00 toléré entre une plage et la moyenne de ses voisines en RVB
/// (choix, à vérifier).
pub const ECART_VOISINES_MAX_DE00: f64 = 20.0;
/// Nombre de plus proches voisines en RVB comparées à chaque plage (choix,
/// à vérifier).
pub const VOISINES: usize = 6;
/// Décalage le plus grand essayé, en plages, dans chaque sens (choix, à
/// vérifier).
pub const DECALAGE_MAX: usize = 60;

/// Une plage est grise quand ses R, G et B (de 0 à 100) diffèrent de moins
/// de cette valeur.
const ECART_GRIS_RVB: f64 = 0.5;

/// Un des quatre contrôles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Critere {
    /// Les gris vont du foncé au clair.
    DegradeGris,
    /// Le blanc est la plage la plus claire, le noir parmi les plus foncées.
    BlancNoir,
    /// Les plages de même RVB ont des mesures proches.
    Doublons,
    /// Chaque plage ressemble à ses voisines en RVB.
    Voisines,
}

const CRITERES: [Critere; 4] = [
    Critere::DegradeGris,
    Critere::BlancNoir,
    Critere::Doublons,
    Critere::Voisines,
];

/// Une plage signalée par un contrôle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlageSuspecte {
    /// Rang de la plage, à partir de 1, dans l'ordre du fichier.
    pub rang: usize,
    pub critere: Critere,
    /// Écart constaté : L* pour le dégradé gris, le blanc et le noir, ΔE00
    /// pour les doublons et les voisines.
    pub ecart: f64,
}

/// Résultat du contrôle, gardé dans la provenance d'un profil calculé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControleCoherence {
    /// Contrôles faits.
    pub faits: Vec<Critere>,
    /// Contrôles impossibles sur cette mesure : moins de 3 gris, ni blanc ni
    /// noir, aucun doublon, moins de [`VOISINES`] voisines.
    pub non_faits: Vec<Critere>,
    /// Plages signalées, triées par rang.
    pub suspectes: Vec<PlageSuspecte>,
    /// Piste seulement, jamais appliquée : associer le RVB de la plage i au
    /// Lab de la plage i + décalage ne laisse aucune plage suspecte.
    pub decalage: Option<i64>,
}

impl ControleCoherence {
    /// Aucune plage suspecte : le calcul peut se faire.
    pub fn accepte(&self) -> bool {
        self.suspectes.is_empty()
    }
}

/// Ce qui ne dépend que des RVB : calculé une seule fois, quel que soit le
/// décalage essayé.
struct Analyse {
    labs: Vec<Lab>,
    /// Gris, RVB croissant.
    gris: Vec<usize>,
    blancs: Vec<usize>,
    noirs: Vec<usize>,
    /// Groupes de plages de même RVB, au centième près.
    doublons: Vec<Vec<usize>>,
    /// Plus proches voisines en RVB de chaque plage, avec leur distance ;
    /// rien si la mesure n'a pas assez de RVB différents.
    voisines: Option<Vec<Vec<(usize, f64)>>>,
}

/// RVB au centième près, pour reconnaître des plages de même RVB.
fn cle_rvb(rvb: [f64; 3]) -> [i64; 3] {
    rvb.map(|c| (c * 100.0).round() as i64)
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Un écart incalculable rend la plage suspecte : le doute fait refuser.
fn ecart_de00(a: Lab, b: Lab) -> f64 {
    delta_e00(a, b).unwrap_or(f64::INFINITY)
}

impl Analyse {
    fn new(mesure: &PlagesProfil) -> Analyse {
        let rvb: Vec<[f64; 3]> = mesure.plages.iter().map(|p| p.rvb).collect();
        let labs = mesure
            .plages
            .iter()
            .map(|p| Lab {
                l: p.lab[0],
                a: p.lab[1],
                b: p.lab[2],
            })
            .collect();
        let cles: Vec<[i64; 3]> = rvb.iter().map(|&c| cle_rvb(c)).collect();

        let mut gris: Vec<usize> = (0..rvb.len())
            .filter(|&i| {
                let [r, v, b] = rvb[i];
                r.max(v).max(b) - r.min(v).min(b) < ECART_GRIS_RVB
            })
            .collect();
        let somme = |i: usize| rvb[i].iter().sum::<f64>();
        gris.sort_by(|&a, &b| somme(a).total_cmp(&somme(b)));
        let blancs = (0..rvb.len()).filter(|&i| cles[i] == [10_000; 3]).collect();
        let noirs = (0..rvb.len()).filter(|&i| cles[i] == [0; 3]).collect();

        let mut groupes: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
        for (i, cle) in cles.iter().enumerate() {
            groupes.entry(*cle).or_default().push(i);
        }
        let differents = groupes.len();
        let mut doublons: Vec<Vec<usize>> = groupes.into_values().filter(|g| g.len() > 1).collect();
        doublons.sort();

        let voisines = (differents > VOISINES).then(|| {
            (0..rvb.len())
                .map(|i| plus_proches(&rvb, &cles, i))
                .collect()
        });
        Analyse {
            labs,
            gris,
            blancs,
            noirs,
            doublons,
            voisines,
        }
    }

    /// Lab associé au RVB de la plage `i` quand les mesures sont décalées de
    /// `decalage` ; rien hors de la mesure.
    fn lab(&self, i: usize, decalage: i64) -> Option<Lab> {
        let j = usize::try_from(i as i64 + decalage).ok()?;
        self.labs.get(j).copied()
    }

    /// Les quatre contrôles, le RVB de la plage i associé au Lab de la plage
    /// i + `decalage`. `premiere` arrête à la première plage suspecte.
    fn evaluer(&self, decalage: i64, premiere: bool) -> ControleCoherence {
        let mut r = ControleCoherence {
            faits: Vec::new(),
            non_faits: Vec::new(),
            suspectes: Vec::new(),
            decalage: None,
        };
        for critere in CRITERES {
            let fait = match critere {
                Critere::DegradeGris => self.gris(decalage, &mut r.suspectes),
                Critere::BlancNoir => self.blanc_noir(decalage, &mut r.suspectes),
                Critere::Doublons => self.doublons(decalage, &mut r.suspectes),
                Critere::Voisines => self.voisines(decalage, premiere, &mut r.suspectes),
            };
            if fait {
                r.faits.push(critere);
            } else {
                r.non_faits.push(critere);
            }
            if premiere && !r.suspectes.is_empty() {
                return r;
            }
        }
        // Une plage signalée deux fois par le même contrôle n'est gardée
        // qu'une fois, avec son plus grand écart.
        r.suspectes.sort_by(|a, b| {
            (a.rang, a.critere)
                .cmp(&(b.rang, b.critere))
                .then(b.ecart.total_cmp(&a.ecart))
        });
        r.suspectes
            .dedup_by(|b, a| (a.rang, a.critere) == (b.rang, b.critere));
        r
    }

    fn gris(&self, decalage: i64, suspectes: &mut Vec<PlageSuspecte>) -> bool {
        let gris: Vec<(usize, Lab)> = self
            .gris
            .iter()
            .filter_map(|&i| self.lab(i, decalage).map(|l| (i, l)))
            .collect();
        if gris.len() < 3 {
            return false;
        }
        for paire in gris.windows(2) {
            let ((i, avant), (j, apres)) = (paire[0], paire[1]);
            let recul = avant.l - apres.l;
            if recul > RETOUR_GRIS_MAX_L {
                for rang in [i + 1, j + 1] {
                    suspectes.push(PlageSuspecte {
                        rang,
                        critere: Critere::DegradeGris,
                        ecart: recul,
                    });
                }
            }
        }
        true
    }

    fn blanc_noir(&self, decalage: i64, suspectes: &mut Vec<PlageSuspecte>) -> bool {
        if self.blancs.is_empty() && self.noirs.is_empty() {
            return false;
        }
        let clartes = (0..self.labs.len()).filter_map(|i| self.lab(i, decalage).map(|l| l.l));
        let (min, max) = clartes.fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), l| {
            (min.min(l), max.max(l))
        });
        let mut signaler = |plages: &[usize], ecart: &dyn Fn(f64) -> f64, seuil: f64| {
            for &i in plages {
                let Some(lab) = self.lab(i, decalage) else {
                    continue;
                };
                let e = ecart(lab.l);
                if e > seuil {
                    suspectes.push(PlageSuspecte {
                        rang: i + 1,
                        critere: Critere::BlancNoir,
                        ecart: e,
                    });
                }
            }
        };
        signaler(&self.blancs, &|l| max - l, ECART_BLANC_MAX_L);
        signaler(&self.noirs, &|l| l - min, ECART_NOIR_MAX_L);
        true
    }

    fn doublons(&self, decalage: i64, suspectes: &mut Vec<PlageSuspecte>) -> bool {
        if self.doublons.is_empty() {
            return false;
        }
        for groupe in &self.doublons {
            let membres: Vec<(usize, Lab)> = groupe
                .iter()
                .filter_map(|&i| self.lab(i, decalage).map(|l| (i, l)))
                .collect();
            for &(i, lab) in &membres {
                let ecart = membres
                    .iter()
                    .filter(|(j, _)| *j != i)
                    .map(|&(_, autre)| ecart_de00(lab, autre))
                    .fold(0.0, f64::max);
                if ecart > ECART_DOUBLONS_MAX_DE00 {
                    suspectes.push(PlageSuspecte {
                        rang: i + 1,
                        critere: Critere::Doublons,
                        ecart,
                    });
                }
            }
        }
        true
    }

    fn voisines(&self, decalage: i64, premiere: bool, suspectes: &mut Vec<PlageSuspecte>) -> bool {
        let Some(voisines) = &self.voisines else {
            return false;
        };
        for (i, proches) in voisines.iter().enumerate() {
            let Some(lab) = self.lab(i, decalage) else {
                continue;
            };
            let (mut somme, mut poids) = ([0.0; 3], 0.0);
            for &(j, d) in proches {
                if let Some(v) = self.lab(j, decalage) {
                    let w = 1.0 / d;
                    somme[0] += w * v.l;
                    somme[1] += w * v.a;
                    somme[2] += w * v.b;
                    poids += w;
                }
            }
            if poids == 0.0 {
                continue;
            }
            let moyenne = Lab {
                l: somme[0] / poids,
                a: somme[1] / poids,
                b: somme[2] / poids,
            };
            let ecart = ecart_de00(lab, moyenne);
            if ecart > ECART_VOISINES_MAX_DE00 {
                suspectes.push(PlageSuspecte {
                    rang: i + 1,
                    critere: Critere::Voisines,
                    ecart,
                });
                if premiere {
                    break;
                }
            }
        }
        true
    }
}

/// Les [`VOISINES`] plus proches voisines en RVB de la plage `i`, sans elle
/// ni ses doublons, de la plus proche à la plus lointaine.
fn plus_proches(rvb: &[[f64; 3]], cles: &[[i64; 3]], i: usize) -> Vec<(usize, f64)> {
    let mut proches: Vec<(usize, f64)> = Vec::with_capacity(VOISINES + 1);
    for j in 0..rvb.len() {
        if cles[j] == cles[i] {
            continue;
        }
        let d = distance(rvb[i], rvb[j]);
        if proches.len() == VOISINES && proches[VOISINES - 1].1 <= d {
            continue;
        }
        let place = proches.partition_point(|&(_, autre)| autre <= d);
        proches.insert(place, (j, d));
        proches.truncate(VOISINES);
    }
    proches
}

/// Contrôle que les mesures correspondent aux plages.
///
/// Si des plages sont suspectes, essaie de décaler les mesures de 1 à
/// [`DECALAGE_MAX`] plages, dans les deux sens : le premier décalage qui ne
/// laisse aucune plage suspecte, avec les mêmes contrôles faits, est rendu
/// comme piste. Rien n'est recalculé avec ce décalage.
pub fn controler_coherence(plages: &PlagesProfil) -> ControleCoherence {
    let analyse = Analyse::new(plages);
    let mut controle = analyse.evaluer(0, false);
    if controle.accepte() {
        return controle;
    }
    let n = plages.plages.len();
    controle.decalage = (1..=DECALAGE_MAX.min(n.saturating_sub(1)))
        .flat_map(|k| [k as i64, -(k as i64)])
        .find(|&s| {
            let essai = analyse.evaluer(s, true);
            // Un contrôle devenu impossible sur les plages communes ne prouve rien.
            essai.accepte() && essai.faits == controle.faits
        });
    controle
}
