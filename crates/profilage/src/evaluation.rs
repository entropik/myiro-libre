//! Interroger un profil ICC pour le vérifier (ticket #41), par `xicclu`
//! d'ArgyllCMS (version figée [`VERSION_ARGYLL`]) derrière le trait
//! [`MoteurVerification`] : [`ArgyllCms`] en vrai,
//! [`MoteurVerificationSimule`] pour les tests (ADR 0005).
//!
//! Pour chaque plage, trois valeurs distinctes : la **demande** (Lab
//! relatif des RVB lus comme du sRVB), la **valeur après transformation**
//! (calculée par ArgyllCMS avec le même profil, en colorimétrique relatif,
//! sans compensation du point noir) et la **prédiction** du profil pour
//! cette valeur, relative et absolue. Le blanc du support est la
//! prédiction absolue du blanc de l'appareil (1, 1, 1).
//!
//! Forme de sortie confirmée le 28 octobre 2026 avec ArgyllCMS 3.5.0 :
//! `xicclu -v0` lit une plage par ligne sur l'entrée standard (valeurs
//! d'appareil de 0 à 1) et écrit trois nombres par ligne, séparés par une
//! espace, à six décimales.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::thread;

use serde::{Deserialize, Serialize};

use crate::argyll::{
    commande, controler_version, derniere_erreur, ligne, ArgyllCms, DossierTravail,
};
use crate::moteur::{ErreurMoteur, Etape, VERSION_ARGYLL};

/// Noms des fichiers dans le dossier de travail.
const PROFIL: &str = "profil.icc";
const SRGB: &str = "sRGB.icm";

/// Comment la seconde mire a été imprimée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Impression {
    /// Sans gestion des couleurs : les RVB partent tels quels à
    /// l'imprimante ; seule la prédiction se calcule.
    SansGestion,
    /// À travers le profil : couleurs sRVB converties par le profil en
    /// colorimétrique relatif, sans compensation du point noir.
    ATraversLeProfil,
}

/// Ce qu'on demande au moteur : le profil, les RVB de la mire (de 0 à 1) et
/// la façon dont elle a été imprimée.
#[derive(Clone, Copy, Debug)]
pub struct DemandeEvaluation<'a> {
    pub icc: &'a [u8],
    pub rvb: &'a [[f64; 3]],
    pub impression: Impression,
}

/// Ce que le profil dit des plages, dans leur ordre (Lab en `[L, a, b]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    /// Lab relatif des RVB en sRVB ; `None` sans gestion des couleurs.
    pub demande: Option<Vec<[f64; 3]>>,
    /// Valeur après transformation, de 0 à 1.
    pub appareil: Vec<[f64; 3]>,
    pub prediction_rel: Vec<[f64; 3]>,
    pub prediction_abs: Vec<[f64; 3]>,
    /// Lab absolu prédit pour l'appareil (1, 1, 1).
    pub blanc_support: [f64; 3],
    pub version: String,
    /// Lignes de commande lancées, chemins réduits à leur nom.
    pub commandes: Vec<String>,
    pub journal: String,
}

/// Un moteur qui interroge un profil ICC pour sa vérification.
pub trait MoteurVerification: Send + Sync {
    /// Commandes équivalentes, sans rien lancer (section « Avancé »).
    fn commandes_verification(&self, impression: Impression) -> Vec<String>;
    /// Interroge le profil. En cas d'erreur, rien n'est rendu.
    fn evaluer(&self, demande: &DemandeEvaluation) -> Result<Evaluation, ErreurMoteur>;
}

/// Arguments de `xicclu` : `-v0` (valeurs seules), la fonction (`f` : vers
/// Lab, `b` : vers l'appareil), l'intention (`r` : relative, `a` : absolue),
/// `-pl` (Lab), puis le profil.
pub fn arguments_xicclu(fonction: &str, intention: &str, profil: &Path) -> Vec<OsString> {
    let mut a: Vec<OsString> = ["-v0".to_string(), format!("-f{fonction}")]
        .map(OsString::from)
        .into();
    a.push(format!("-i{intention}").into());
    a.push("-pl".into());
    a.push(profil.into());
    a
}

/// Les trois premières valeurs de chaque ligne non vide de `xicclu -v0` ;
/// une ligne illisible ou un nombre de lignes autre que `attendues` est
/// refusé, avec ce qui ne va pas.
pub fn lire_sortie_xicclu(sortie: &str, attendues: usize) -> Result<Vec<[f64; 3]>, String> {
    let mut valeurs = Vec::with_capacity(attendues);
    for l in sortie.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let nombres: Vec<f64> = l
            .split_whitespace()
            .take(3)
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| format!("ligne illisible : {l}"))?;
        match nombres[..] {
            [a, b, c] => valeurs.push([a, b, c]),
            _ => return Err(format!("ligne incomplète : {l}")),
        }
    }
    if valeurs.len() != attendues {
        return Err(format!(
            "{} valeurs rendues pour {attendues} attendues",
            valeurs.len()
        ));
    }
    Ok(valeurs)
}

/// Les appels de `xicclu`, dans l'ordre : (fichier, fonction, intention).
fn appels(impression: Impression) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut a = Vec::new();
    if impression == Impression::ATraversLeProfil {
        a.extend([(SRGB, "f", "r"), (PROFIL, "b", "r")]);
    }
    a.extend([(PROFIL, "f", "r"), (PROFIL, "f", "a")]);
    a
}

fn lignes_verification(impression: Impression) -> Vec<String> {
    appels(impression)
        .into_iter()
        .map(|(p, f, i)| ligne("xicclu", &arguments_xicclu(f, i, Path::new(p))))
        .collect()
}

/// Une valeur par ligne, telle que `xicclu` la lit.
fn entree(valeurs: &[[f64; 3]]) -> String {
    valeurs
        .iter()
        .map(|[a, b, c]| format!("{a} {b} {c}\n"))
        .collect()
}

impl ArgyllCms {
    /// Lance `xicclu` dans le dossier de travail sur ces valeurs, et lit
    /// autant de valeurs en sortie. L'entrée est écrite par un autre fil :
    /// une sortie pleine ne bloque pas l'écriture.
    fn xicclu(
        &self,
        arguments: &[OsString],
        dossier: &Path,
        valeurs: &[[f64; 3]],
        journal: &mut String,
    ) -> Result<Vec<[f64; 3]>, ErreurMoteur> {
        let echec = |cause: String| ErreurMoteur::Echec {
            etape: Etape::Verification,
            cause: format!("xicclu : {cause}"),
        };
        let mut enfant = commande(&self.outil("xicclu"))
            .args(arguments)
            .current_dir(dossier)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| echec(e.to_string()))?;
        let texte = entree(valeurs);
        let ecriture = enfant
            .stdin
            .take()
            .map(|mut flux| thread::spawn(move || flux.write_all(texte.as_bytes())));
        let sortie = enfant
            .wait_with_output()
            .map_err(|e| echec(e.to_string()))?;
        if let Some(fil) = ecriture {
            match fil.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(echec(format!("entrée standard : {e}"))),
                Err(_) => return Err(echec("écriture de l'entrée arrêtée net".into())),
            }
        }
        let stdout = String::from_utf8_lossy(&sortie.stdout);
        let stderr = String::from_utf8_lossy(&sortie.stderr);
        journal.push_str(&format!(
            "$ {}\n({} valeurs sur l'entrée standard)\n{stderr}",
            ligne("xicclu", arguments),
            valeurs.len()
        ));
        if !sortie.status.success() {
            return Err(match derniere_erreur(&stderr) {
                Some(cause) => ErreurMoteur::Echec {
                    etape: Etape::Verification,
                    cause,
                },
                None => ErreurMoteur::Interrompu {
                    etape: Etape::Verification,
                },
            });
        }
        lire_sortie_xicclu(&stdout, valeurs.len()).map_err(echec)
    }
}

impl MoteurVerification for ArgyllCms {
    fn commandes_verification(&self, impression: Impression) -> Vec<String> {
        lignes_verification(impression)
    }

    /// Outils présents, version contrôlée, puis `xicclu` lancé dans un
    /// dossier de travail effacé à la fin, réussite ou échec. Sans
    /// `colprof`, ArgyllCMS est introuvable ; sans `xicclu`, seul cet outil
    /// manque.
    fn evaluer(&self, demande: &DemandeEvaluation) -> Result<Evaluation, ErreurMoteur> {
        if !self.outil("colprof").is_file() {
            return Err(ErreurMoteur::Introuvable);
        }
        if !self.outil("xicclu").is_file() {
            return Err(ErreurMoteur::OutilManquant {
                outil: "xicclu".to_string(),
            });
        }
        let version = self.version()?;
        controler_version(&version)?;

        let dossier = DossierTravail::creer()?;
        let chemin = dossier.chemin();
        fs::write(chemin.join(PROFIL), demande.icc)
            .map_err(|e| ErreurMoteur::Fichier(format!("{PROFIL} : {e}")))?;
        let mut journal = String::new();
        let args = |p: &str, f: &str, i: &str| arguments_xicclu(f, i, Path::new(p));

        let (lab_demande, appareil) = match demande.impression {
            Impression::SansGestion => (None, demande.rvb.to_vec()),
            Impression::ATraversLeProfil => {
                fs::copy(self.racine().join("ref").join(SRGB), chemin.join(SRGB))
                    .map_err(|e| ErreurMoteur::Fichier(format!("ref/{SRGB} d'ArgyllCMS : {e}")))?;
                let lab = self.xicclu(&args(SRGB, "f", "r"), chemin, demande.rvb, &mut journal)?;
                let appareil = self.xicclu(&args(PROFIL, "b", "r"), chemin, &lab, &mut journal)?;
                (Some(lab), appareil)
            }
        };
        let prediction_rel =
            self.xicclu(&args(PROFIL, "f", "r"), chemin, &appareil, &mut journal)?;
        let mut avec_blanc = appareil.clone();
        avec_blanc.push([1.0, 1.0, 1.0]);
        let mut prediction_abs =
            self.xicclu(&args(PROFIL, "f", "a"), chemin, &avec_blanc, &mut journal)?;
        let blanc_support = prediction_abs.pop().ok_or_else(|| ErreurMoteur::Echec {
            etape: Etape::Verification,
            cause: "blanc du support absent".into(),
        })?;
        Ok(Evaluation {
            demande: lab_demande,
            appareil,
            prediction_rel,
            prediction_abs,
            blanc_support,
            version,
            commandes: lignes_verification(demande.impression),
            journal,
        })
    }
}

/// Moteur des tests : rend une évaluation calculée par une fonction simple
/// des RVB, une évaluation donnée, ou une erreur.
pub struct MoteurVerificationSimule {
    resultat: Option<Result<Evaluation, ErreurMoteur>>,
}

/// Lab d'une fonction simple et régulière des RVB (de 0 à 1), celle de la
/// mire synthétique du plan 3.
fn lab_simule([r, g, b]: [f64; 3]) -> [f64; 3] {
    [
        20.0 + 75.0 * (r + g + b) / 3.0,
        40.0 * (r - g),
        40.0 * (g - b),
    ]
}

impl MoteurVerificationSimule {
    /// Un profil fidèle sur un support parfaitement blanc : la valeur après
    /// transformation est le RVB lui-même, demande et prédictions sont la
    /// même fonction simple des RVB.
    pub fn fidele() -> Self {
        MoteurVerificationSimule { resultat: None }
    }

    /// Un moteur qui rend toujours cette évaluation.
    pub fn rend(evaluation: Evaluation) -> Self {
        MoteurVerificationSimule {
            resultat: Some(Ok(evaluation)),
        }
    }

    /// Un moteur qui échoue toujours ainsi.
    pub fn echoue(erreur: ErreurMoteur) -> Self {
        MoteurVerificationSimule {
            resultat: Some(Err(erreur)),
        }
    }
}

impl MoteurVerification for MoteurVerificationSimule {
    fn commandes_verification(&self, impression: Impression) -> Vec<String> {
        lignes_verification(impression)
    }

    fn evaluer(&self, demande: &DemandeEvaluation) -> Result<Evaluation, ErreurMoteur> {
        if let Some(resultat) = &self.resultat {
            return resultat.clone();
        }
        let appareil = demande.rvb.to_vec();
        let lab: Vec<[f64; 3]> = appareil.iter().copied().map(lab_simule).collect();
        Ok(Evaluation {
            demande: (demande.impression == Impression::ATraversLeProfil).then(|| lab.clone()),
            appareil,
            prediction_rel: lab.clone(),
            prediction_abs: lab,
            blanc_support: [100.0, 0.0, 0.0],
            version: VERSION_ARGYLL.to_string(),
            commandes: lignes_verification(demande.impression),
            journal: "moteur simulé\n".to_string(),
        })
    }
}
