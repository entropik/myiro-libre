//! Lecture d'un fichier CGATS.17 en mesure importée.

use std::collections::HashMap;

use pont_protocole::{ConditionMesure, Echantillonnage, Info, Lab, Spectre};

use crate::provenance::{self, condition_depuis, PREFIXE};
use crate::{erreur, ErreurCgats, MesureImportee, PlageImportee, EMPLACEMENTS, FORMAT_CGATS};

/// Morceau d'une ligne : mot nu ou chaîne entre guillemets.
#[derive(Debug, PartialEq)]
enum Jeton {
    Mot(String),
    Chaine(String),
}

impl Jeton {
    fn texte(&self) -> &str {
        match self {
            Jeton::Mot(t) | Jeton::Chaine(t) => t,
        }
    }
}

/// Découpe une ligne : blancs entre les jetons, `#` ouvre un commentaire hors
/// d'une chaîne, `""` dans une chaîne est un guillemet.
fn jetons(ligne: &str, numero: usize) -> Result<Vec<Jeton>, ErreurCgats> {
    let mut jetons = Vec::new();
    let mut car = ligne.chars().peekable();
    while let Some(&c) = car.peek() {
        if c.is_whitespace() {
            car.next();
        } else if c == '#' {
            break;
        } else if c == '"' {
            car.next();
            let mut texte = String::new();
            loop {
                match car.next() {
                    Some('"') if car.peek() == Some(&'"') => {
                        car.next();
                        texte.push('"');
                    }
                    Some('"') => break,
                    Some(c) => texte.push(c),
                    None => return erreur(format!("ligne {numero} : guillemet non fermé")),
                }
            }
            jetons.push(Jeton::Chaine(texte));
        } else {
            let mut texte = String::new();
            while let Some(&c) = car.peek() {
                if c.is_whitespace() {
                    break;
                }
                texte.push(c);
                car.next();
            }
            jetons.push(Jeton::Mot(texte));
        }
    }
    Ok(jetons)
}

/// Un tableau du fichier, tel qu'écrit.
#[derive(Default)]
struct Tableau {
    mots_cles: Vec<(String, String)>,
    champs: Vec<String>,
    valeurs: Vec<String>,
    annonce: Option<usize>,
}

impl Tableau {
    fn mot_cle(&self, nom: &str) -> Option<&str> {
        self.mots_cles
            .iter()
            .find(|(n, _)| n == nom)
            .map(|(_, v)| v.as_str())
    }

    fn lignes(&self) -> impl Iterator<Item = &[String]> {
        self.valeurs.chunks(self.champs.len())
    }
}

enum Etat {
    EnTete,
    Format,
    Donnees,
}

fn tableaux(texte: &str) -> Result<Vec<Tableau>, ErreurCgats> {
    let mut tableaux = Vec::new();
    let mut courant = Tableau::default();
    let mut commence = false;
    let mut etat = Etat::EnTete;
    for (i, ligne) in texte.lines().enumerate() {
        let numero = i + 1;
        let jetons = jetons(ligne, numero)?;
        let Some(premier) = jetons.first() else {
            continue;
        };
        let premier = premier.texte();
        match etat {
            Etat::EnTete => match premier {
                "BEGIN_DATA_FORMAT" => etat = Etat::Format,
                "BEGIN_DATA" => {
                    if courant.champs.is_empty() {
                        return erreur(format!("ligne {numero} : données sans BEGIN_DATA_FORMAT"));
                    }
                    etat = Etat::Donnees;
                }
                // Identifiant du fichier (CGATS.17, IT8.7/2…) en tête d'un tableau.
                _ if !commence && jetons.len() == 1 => commence = true,
                "KEYWORD" => commence = true,
                "NUMBER_OF_SETS" => {
                    commence = true;
                    let nombre = jetons.get(1).map(Jeton::texte).unwrap_or_default();
                    courant.annonce = Some(nombre.parse().map_err(|_| {
                        ErreurCgats(format!(
                            "ligne {numero} : NUMBER_OF_SETS « {nombre} » illisible"
                        ))
                    })?);
                }
                _ => {
                    commence = true;
                    let valeur: Vec<&str> = jetons[1..].iter().map(Jeton::texte).collect();
                    courant
                        .mots_cles
                        .push((premier.to_string(), valeur.join(" ")));
                }
            },
            Etat::Format => match premier {
                "END_DATA_FORMAT" => etat = Etat::EnTete,
                _ => courant
                    .champs
                    .extend(jetons.iter().map(|j| j.texte().to_string())),
            },
            Etat::Donnees => match premier {
                "END_DATA" => {
                    let fini = std::mem::take(&mut courant);
                    if fini.valeurs.len() % fini.champs.len() != 0 {
                        return erreur(format!(
                            "ligne {numero} : les données ne remplissent pas les {} colonnes annoncées",
                            fini.champs.len()
                        ));
                    }
                    let lignes = fini.valeurs.len() / fini.champs.len();
                    if fini.annonce.is_some_and(|n| n != lignes) {
                        return erreur(format!(
                            "ligne {numero} : {lignes} plages au lieu des {} annoncées",
                            fini.annonce.unwrap_or_default()
                        ));
                    }
                    tableaux.push(fini);
                    commence = false;
                    etat = Etat::EnTete;
                }
                _ => courant
                    .valeurs
                    .extend(jetons.iter().map(|j| j.texte().to_string())),
            },
        }
    }
    if !matches!(etat, Etat::EnTete) || commence {
        return erreur("fichier incomplet : il manque END_DATA_FORMAT ou END_DATA");
    }
    if tableaux.is_empty() {
        return erreur("aucun tableau de données (BEGIN_DATA … END_DATA)");
    }
    Ok(tableaux)
}

fn nombre(texte: &str, quoi: &str) -> Result<f32, ErreurCgats> {
    texte
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| ErreurCgats(format!("{quoi} : « {texte} » n'est pas un nombre")))
}

/// Pourcentage ramené à une réflectance de 0 à 1. Calculé en f64 : une valeur
/// écrite par [`crate::ecrire`] (f32 × 100, en f64) se relit à l'identique.
fn pourcent(texte: &str, quoi: &str) -> Result<f32, ErreurCgats> {
    texte
        .parse::<f64>()
        .ok()
        .map(|v| (v / 100.0) as f32)
        .filter(|v| v.is_finite())
        .ok_or_else(|| ErreurCgats(format!("{quoi} : « {texte} » n'est pas un nombre")))
}

/// Ce qu'un tableau apporte : un emplacement de spectre et ses plages.
struct Lu {
    emplacement: usize,
    condition: Info<ConditionMesure>,
    longueurs_onde: Info<Echantillonnage>,
    identifiants: Vec<String>,
    spectres: Vec<Info<Spectre>>,
    lab: Vec<Info<Lab>>,
}

/// Emplacement et condition d'un tableau, sans rien deviner : mot-clé de
/// myiro-libre, condition ISO 13655 écrite, ou source lumineuse du fabricant.
fn emplacement(t: &Tableau, rang: usize) -> Result<(usize, Info<ConditionMesure>), ErreurCgats> {
    let ecrite = match t.mot_cle("MEASUREMENT_CONDITION") {
        Some(c) => Some(condition_depuis(c).ok_or_else(|| {
            ErreurCgats(format!(
                "tableau {rang} : MEASUREMENT_CONDITION « {c} » inconnue"
            ))
        })?),
        None => None,
    };
    if let Some(e) = t.mot_cle(&format!("{PREFIXE}SPECTRE")) {
        let n = EMPLACEMENTS.iter().position(|x| *x == e).ok_or_else(|| {
            ErreurCgats(format!("tableau {rang} : {PREFIXE}SPECTRE « {e} » inconnu"))
        })?;
        return Ok((n, ecrite.map_or(Info::Inconnue, Info::Confirmee)));
    }
    if let Some(c) = ecrite {
        return Ok((c as usize, Info::Confirmee(c)));
    }
    let source = t.mot_cle("MEASUREMENT_SOURCE");
    let deduite = match source {
        Some("A") => Some(ConditionMesure::M0),
        Some("D50") => Some(ConditionMesure::M1),
        Some("UVCUT") => Some(ConditionMesure::M2),
        _ => None,
    };
    match deduite {
        Some(c) => Ok((c as usize, Info::Supposee(c))),
        None => erreur(format!(
            "tableau {rang} : condition de mesure introuvable (ni MEASUREMENT_CONDITION, ni MEASUREMENT_SOURCE A, D50 ou UVCUT) ; rien n'est deviné"
        )),
    }
}

fn lire_tableau(t: &Tableau, rang: usize) -> Result<Lu, ErreurCgats> {
    let (emplacement, condition) = emplacement(t, rang)?;
    let colonne = |nom: &str| t.champs.iter().position(|c| c.eq_ignore_ascii_case(nom));
    let id = colonne("SAMPLE_ID");
    let lab = [colonne("LAB_L"), colonne("LAB_A"), colonne("LAB_B")];

    // Colonnes de spectre, chacune avec son échelle, jamais devinée :
    // `nm380` du fabricant (réflectance de 0 à 1), `SPEC_380` d'ArgyllCMS
    // (pourcentage), `MYIRO_LIBRE_SPECTRE_1` de myiro-libre quand les
    // longueurs d'onde sont inconnues (pourcentage).
    let mut nm: Vec<(u32, usize)> = Vec::new();
    let mut spec: Vec<(u32, usize)> = Vec::new();
    let mut rangs: Vec<(u32, usize)> = Vec::new();
    for (i, champ) in t.champs.iter().enumerate() {
        let minuscule = champ.to_ascii_lowercase();
        if let Some(l) = minuscule.strip_prefix("nm").and_then(|l| l.parse().ok()) {
            nm.push((l, i));
        } else if let Some(l) = minuscule.strip_prefix("spec_").and_then(|l| l.parse().ok()) {
            spec.push((l, i));
        } else if let Some(r) = champ
            .strip_prefix(&format!("{PREFIXE}SPECTRE_"))
            .and_then(|r| r.parse().ok())
        {
            rangs.push((r, i));
        }
    }
    let sortes = [!nm.is_empty(), !spec.is_empty(), !rangs.is_empty()];
    if sortes.iter().filter(|s| **s).count() > 1 {
        return erreur(format!(
            "tableau {rang} : deux sortes de colonnes de spectre"
        ));
    }
    let pourcentage = nm.is_empty();
    let nm = if nm.is_empty() { spec } else { nm };
    let (longueurs_onde, colonnes_spectre) = if !nm.is_empty() {
        let mut nm = nm;
        nm.sort();
        let debut = nm[0].0;
        let pas = nm.get(1).map_or(0, |(l, _)| l - debut);
        // En 64 bits : des longueurs d'onde démesurées ne débordent pas.
        let reguliere = nm
            .iter()
            .enumerate()
            .all(|(i, (l, _))| u64::from(*l) == u64::from(debut) + u64::from(pas) * i as u64);
        if nm.len() > 1 && (pas == 0 || !reguliere) {
            return erreur(format!(
                "tableau {rang} : longueurs d'onde irrégulières ou en double"
            ));
        }
        let echantillonnage = if nm.len() > 1 {
            Info::Confirmee(Echantillonnage {
                debut_nm: debut,
                pas_nm: pas,
            })
        } else {
            Info::Inconnue
        };
        (
            echantillonnage,
            nm.iter().map(|(_, i)| *i).collect::<Vec<_>>(),
        )
    } else {
        rangs.sort();
        if rangs
            .iter()
            .enumerate()
            .any(|(i, (r, _))| *r as usize != i + 1)
        {
            return erreur(format!("tableau {rang} : colonnes de spectre incomplètes"));
        }
        (Info::Inconnue, rangs.iter().map(|(_, i)| *i).collect())
    };

    let mut lu = Lu {
        emplacement,
        condition,
        longueurs_onde,
        identifiants: Vec::new(),
        spectres: Vec::new(),
        lab: Vec::new(),
    };
    for (n, ligne) in t.lignes().enumerate() {
        let quoi = |champ: &str| format!("tableau {rang}, plage {}, {champ}", n + 1);
        lu.identifiants
            .push(id.map_or_else(|| (n + 1).to_string(), |i| ligne[i].clone()));
        lu.lab.push(match lab {
            [Some(l), Some(a), Some(b)] => {
                let mut v = [0.0; 3];
                for (k, colonne) in [l, a, b].into_iter().enumerate() {
                    v[k] = nombre(&ligne[colonne], &quoi(&t.champs[colonne]))?;
                }
                Info::Confirmee(
                    Lab::new(v).map_err(|e| ErreurCgats(format!("{} : {e}", quoi("Lab"))))?,
                )
            }
            _ => Info::Inconnue,
        });
        lu.spectres.push(if colonnes_spectre.is_empty() {
            Info::Inconnue
        } else {
            let valeurs = colonnes_spectre
                .iter()
                .map(|&c| {
                    if pourcentage {
                        pourcent(&ligne[c], &quoi(&t.champs[c]))
                    } else {
                        nombre(&ligne[c], &quoi(&t.champs[c]))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Info::Confirmee(
                Spectre::new(valeurs)
                    .map_err(|e| ErreurCgats(format!("{} : {e}", quoi("spectre"))))?,
            )
        });
    }
    Ok(lu)
}

fn declare(t: &Tableau, nom: &str) -> Info<String> {
    match t.mot_cle(nom) {
        Some(v) if !v.trim().is_empty() => Info::Confirmee(v.to_string()),
        _ => Info::Inconnue,
    }
}

/// Lit un fichier CGATS.17 (de myiro-libre ou d'un autre logiciel) en mesure
/// importée. Plusieurs tableaux sont lus comme les emplacements `m0`, `m1`,
/// `m2` des mêmes plages ; un emplacement absent du fichier reste inconnu.
///
/// Refusé en clair : fichier mal formé, nombre illisible, condition de mesure
/// introuvable, tableaux qui ne portent pas les mêmes plages, fichier de
/// myiro-libre d'une autre version ou à la provenance incomplète.
pub fn lire(texte: &str) -> Result<MesureImportee, ErreurCgats> {
    let tableaux = tableaux(texte)?;
    let premier = &tableaux[0];
    let mots_cles: HashMap<String, String> = premier.mots_cles.iter().cloned().collect();
    let provenance = match mots_cles.get(&format!("{PREFIXE}FORMAT")) {
        Some(f) if f == FORMAT_CGATS => Info::Confirmee(provenance::lire(&mots_cles)?),
        Some(f) => {
            return erreur(format!(
                "fichier myiro-libre « {f} » non pris en charge (attendu {FORMAT_CGATS})"
            ))
        }
        None => Info::Inconnue,
    };

    let mut conditions = [Info::Inconnue, Info::Inconnue, Info::Inconnue];
    let mut longueurs_onde = Info::Inconnue;
    let mut plages: Vec<PlageImportee> = Vec::new();
    let mut vus = [false; 3];
    for (i, t) in tableaux.iter().enumerate() {
        let lu = lire_tableau(t, i + 1)?;
        let e = lu.emplacement;
        if std::mem::replace(&mut vus[e], true) {
            return erreur(format!(
                "tableau {} : deux tableaux pour l'emplacement {}",
                i + 1,
                EMPLACEMENTS[e]
            ));
        }
        conditions[e] = lu.condition;
        if lu.longueurs_onde != Info::Inconnue {
            if longueurs_onde != Info::Inconnue && longueurs_onde != lu.longueurs_onde {
                return erreur(format!("tableau {} : autres longueurs d'onde", i + 1));
            }
            longueurs_onde = lu.longueurs_onde;
        }
        if i == 0 {
            plages = lu
                .identifiants
                .iter()
                .map(|id| PlageImportee {
                    identifiant: id.clone(),
                    spectres: [Info::Inconnue, Info::Inconnue, Info::Inconnue],
                    lab: [Info::Inconnue, Info::Inconnue, Info::Inconnue],
                })
                .collect();
        } else if lu.identifiants.len() != plages.len()
            || lu
                .identifiants
                .iter()
                .zip(&plages)
                .any(|(id, p)| *id != p.identifiant)
        {
            return erreur(format!(
                "tableau {} : pas les mêmes plages que le premier tableau",
                i + 1
            ));
        }
        for ((plage, spectre), lab) in plages.iter_mut().zip(lu.spectres).zip(lu.lab) {
            plage.spectres[e] = spectre;
            plage.lab[e] = lab;
        }
    }

    Ok(MesureImportee {
        provenance,
        instrument: declare(premier, "INSTRUMENTATION"),
        numero_serie: declare(premier, "SERIAL"),
        date: declare(premier, "CREATED"),
        conditions,
        longueurs_onde,
        plages,
    })
}
