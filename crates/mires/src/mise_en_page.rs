//! Placement des plages MYIRO-1 dans les bandes du [`Gabarit`].
//!
//! Une bande est lue d'un bout à l'autre : chaque plage doit trancher sur sa
//! voisine de gauche, et les plages du bout sur le blanc papier (ΔE*ab >
//! [`CONTRASTE_MIN_DE76`], manuel MYIRO-1). Les bandes sont lues une à une :
//! aucune règle avec la bande du dessus. Le Lab est **estimé** à partir des
//! valeurs RVB lues comme du sRGB : une estimation pour la mise en page,
//! jamais une mesure.

use std::collections::{HashMap, HashSet};

use colorimetrie::{xyz_vers_lab, Lab, Xyz};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ErreurMire, Gabarit, CONTRASTE_MIN_DE76, VERSION_GABARIT};

/// Une plage à sa place : rang dans les valeurs, bande et colonne.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlagePlacee {
    pub index: usize,
    pub rangee: usize,
    pub colonne: usize,
}

/// Une page, numérotée à partir de 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub numero: usize,
    pub plages: Vec<PlagePlacee>,
}

/// Plan de lecture : place de chaque plage, page par page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiseEnPage {
    pub gabarit: Gabarit,
    pub graine: u64,
    /// Version du gabarit ([`VERSION_GABARIT`]) au moment de la mise en
    /// page ; une mise en page rangée sans elle est de la version 1.
    #[serde(default = "version_1")]
    pub version_gabarit: u32,
    pub pages: Vec<Page>,
}

impl MiseEnPage {
    /// La mise en page est celle que le gabarit actuel produirait : même
    /// version du gabarit ([`VERSION_GABARIT`]), et chaque plage à la place
    /// que le gabarit lui donne, page après page. Une mise en page d'une
    /// version plus ancienne ne le suit pas : ses pages ne sont pas celles
    /// que le gabarit dessine aujourd'hui.
    pub fn suit_le_gabarit_actuel(&self) -> bool {
        if self.version_gabarit != VERSION_GABARIT {
            return false;
        }
        let colonnes = self.gabarit.colonnes();
        let par_page = self.gabarit.plages_par_page();
        if colonnes == 0 {
            return false;
        }
        let derniere = self.pages.len();
        self.pages.iter().enumerate().all(|(p, page)| {
            page.numero == p + 1
                && (page.plages.len() == par_page
                    || (p + 1 == derniere && page.plages.len() < par_page))
                && page
                    .plages
                    .iter()
                    .enumerate()
                    .all(|(i, plage)| (plage.rangee, plage.colonne) == (i / colonnes, i % colonnes))
        })
    }

    /// SHA-256 hexadécimal, en minuscules, du JSON de la mise en page.
    pub fn empreinte(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("{:x}", Sha256::digest(json.as_bytes()))
    }
}

/// Version d'une mise en page rangée sans version : la première, page en
/// paysage.
fn version_1() -> u32 {
    1
}

/// Ce que [`verifier`] reproche à une mise en page.
#[derive(Debug, PartialEq)]
pub enum Defaut {
    ContrasteVoisine {
        page: usize,
        rangee: usize,
        colonne: usize,
    },
    ContrasteBlanc {
        page: usize,
        rangee: usize,
        colonne: usize,
    },
    Nombre {
        attendu: usize,
        trouve: usize,
    },
    /// Valeur placée deux fois, ou rang inconnu des valeurs.
    Doublon {
        index: usize,
    },
    HorsGabarit {
        page: usize,
        rangee: usize,
        colonne: usize,
    },
}

/// RVB de 0 à 100 lu comme du sRGB, Lab sous D65 : estimation pour la mise
/// en page seulement. Une valeur non finie donne un Lab inconnu (NaN), qui ne
/// tranche sur rien.
pub fn estimer_lab(rvb: [f64; 3]) -> Lab {
    let lineaire = rvb.map(|v| {
        let c = v / 100.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    // Matrice sRGB (IEC 61966-2-1), D65 ; le blanc est la somme des lignes.
    const M: [[f64; 3]; 3] = [
        [0.4124, 0.3576, 0.1805],
        [0.2126, 0.7152, 0.0722],
        [0.0193, 0.1192, 0.9505],
    ];
    let produit = |l: [f64; 3]| {
        let [x, y, z] =
            M.map(|ligne| 100.0 * (ligne[0] * l[0] + ligne[1] * l[1] + ligne[2] * l[2]));
        Xyz { x, y, z }
    };
    xyz_vers_lab(produit(lineaire), produit([1.0; 3])).unwrap_or(Lab {
        l: f64::NAN,
        a: f64::NAN,
        b: f64::NAN,
    })
}

fn tranche(a: &Lab, b: &Lab) -> bool {
    let de76 = ((a.l - b.l).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt();
    de76 > CONTRASTE_MIN_DE76
}

/// Générateur splitmix64, déterministe pour une graine donnée.
struct Generateur(u64);

impl Generateur {
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier de 0 à `n` exclu (`n` > 0).
    fn sous(&mut self, n: usize) -> usize {
        (self.suivant() % n as u64) as usize
    }
}

/// Emplacements remplis dans l'ordre de lecture : rangée par rangée, puis
/// page par page. Les contraintes d'un emplacement ne regardent que sa
/// bande : voisine de gauche, et blanc papier aux deux bouts.
struct Emplacements<'a> {
    labs: Vec<Lab>,
    colonnes: usize,
    n: usize,
    blanc: Lab,
    ordre: &'a mut [usize],
}

impl Emplacements<'_> {
    fn debut_de_bande(&self, s: usize) -> bool {
        s.is_multiple_of(self.colonnes)
    }

    fn fin_de_bande(&self, s: usize) -> bool {
        s % self.colonnes == self.colonnes - 1 || s + 1 == self.n
    }

    fn lab(&self, s: usize) -> &Lab {
        &self.labs[self.ordre[s]]
    }

    /// Contraintes qui touchent l'emplacement `s` : (s, 0) pour la voisine de
    /// gauche, (s, 1) pour le blanc au début, (s, 2) pour le blanc à la fin.
    fn contraintes(&self, s: usize, liste: &mut Vec<(usize, u8)>) {
        if self.debut_de_bande(s) {
            liste.push((s, 1));
        } else {
            liste.push((s, 0));
        }
        if self.fin_de_bande(s) {
            liste.push((s, 2));
        } else {
            liste.push((s + 1, 0));
        }
    }

    fn en_defaut(&self, (s, genre): (usize, u8)) -> bool {
        match genre {
            0 => !tranche(self.lab(s - 1), self.lab(s)),
            _ => !tranche(self.lab(s), &self.blanc),
        }
    }

    fn defauts_de(&self, liste: &[(usize, u8)]) -> usize {
        liste.iter().filter(|c| self.en_defaut(**c)).count()
    }

    fn defaut_en(&self, s: usize) -> bool {
        let mut liste = Vec::new();
        self.contraintes(s, &mut liste);
        self.defauts_de(&liste) > 0
    }
}

/// Place les plages : ordre mélangé par la graine, bandes remplies dans
/// l'ordre de lecture, puis réparation par échanges tant qu'un échange réduit
/// le nombre de défauts (50 × n tentatives au plus). S'il reste un défaut, la
/// mise en page est refusée.
pub fn mettre_en_page(
    valeurs: &[[f64; 3]],
    gabarit: &Gabarit,
    graine: u64,
) -> Result<MiseEnPage, ErreurMire> {
    let par_page = gabarit.plages_par_page();
    if par_page == 0 {
        let (min, max) = Gabarit::limites_cote();
        return Err(ErreurMire::CoteHorsLimites { min, max });
    }
    let n = valeurs.len();
    let mut generateur = Generateur(graine);
    let mut ordre: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = generateur.sous(i + 1);
        ordre.swap(i, j);
    }
    let e = Emplacements {
        labs: valeurs.iter().map(|v| estimer_lab(*v)).collect(),
        colonnes: gabarit.colonnes(),
        n,
        blanc: estimer_lab([100.0; 3]),
        ordre: &mut ordre,
    };
    let mut tentatives = 0;
    'reparation: while tentatives < 50 * n {
        let mut reste = false;
        for a in 0..n {
            if !e.defaut_en(a) {
                continue;
            }
            reste = true;
            let b = generateur.sous(n);
            tentatives += 1;
            let mut liste = Vec::new();
            e.contraintes(a, &mut liste);
            e.contraintes(b, &mut liste);
            liste.sort_unstable();
            liste.dedup();
            let avant = e.defauts_de(&liste);
            e.ordre.swap(a, b);
            if e.defauts_de(&liste) >= avant {
                e.ordre.swap(a, b);
            }
            if tentatives >= 50 * n {
                break 'reparation;
            }
        }
        if !reste {
            break;
        }
    }
    let defauts = (0..n)
        .flat_map(|s| {
            let mut liste = Vec::new();
            e.contraintes(s, &mut liste);
            liste
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .filter(|c| e.en_defaut(*c))
        .count();
    if defauts > 0 {
        return Err(ErreurMire::Contraste { defauts });
    }

    let colonnes = gabarit.colonnes();
    let mut pages: Vec<Page> = Vec::new();
    for (s, &index) in ordre.iter().enumerate() {
        let numero = s / par_page + 1;
        if pages.last().map(|p| p.numero) != Some(numero) {
            pages.push(Page {
                numero,
                plages: Vec::new(),
            });
        }
        if let Some(page) = pages.last_mut() {
            page.plages.push(PlagePlacee {
                index,
                rangee: (s % par_page) / colonnes,
                colonne: s % colonnes,
            });
        }
    }
    Ok(MiseEnPage {
        gabarit: gabarit.clone(),
        graine,
        version_gabarit: VERSION_GABARIT,
        pages,
    })
}

/// Défauts d'une mise en page, sans rien corriger : nombre, doublons,
/// plages hors gabarit, contraste avec la voisine de gauche, et avec le
/// blanc papier là où la plage n'a pas de voisine (bout de bande ou trou).
pub fn verifier(mise: &MiseEnPage, valeurs: &[[f64; 3]]) -> Vec<Defaut> {
    let mut defauts = Vec::new();
    let place: usize = mise.pages.iter().map(|p| p.plages.len()).sum();
    if place != valeurs.len() {
        defauts.push(Defaut::Nombre {
            attendu: valeurs.len(),
            trouve: place,
        });
    }
    let mut vues = HashSet::new();
    let (colonnes, rangees) = (mise.gabarit.colonnes(), mise.gabarit.rangees());
    let blanc = estimer_lab([100.0; 3]);
    for page in &mise.pages {
        let mut cases: HashMap<(usize, usize), Lab> = HashMap::new();
        for p in &page.plages {
            if !vues.insert(p.index) || p.index >= valeurs.len() {
                defauts.push(Defaut::Doublon { index: p.index });
                continue;
            }
            if p.rangee >= rangees || p.colonne >= colonnes {
                defauts.push(Defaut::HorsGabarit {
                    page: page.numero,
                    rangee: p.rangee,
                    colonne: p.colonne,
                });
                continue;
            }
            cases.insert((p.rangee, p.colonne), estimer_lab(valeurs[p.index]));
        }
        let mut positions: Vec<_> = cases.keys().copied().collect();
        positions.sort_unstable();
        for (rangee, colonne) in positions {
            let lab = &cases[&(rangee, colonne)];
            let ou = (page.numero, rangee, colonne);
            let gauche = colonne.checked_sub(1).and_then(|c| cases.get(&(rangee, c)));
            let droite = cases.get(&(rangee, colonne + 1));
            if let Some(g) = gauche {
                if !tranche(g, lab) {
                    defauts.push(Defaut::ContrasteVoisine {
                        page: ou.0,
                        rangee,
                        colonne,
                    });
                }
            }
            if (gauche.is_none() || droite.is_none()) && !tranche(lab, &blanc) {
                defauts.push(Defaut::ContrasteBlanc {
                    page: ou.0,
                    rangee,
                    colonne,
                });
            }
        }
    }
    defauts
}
