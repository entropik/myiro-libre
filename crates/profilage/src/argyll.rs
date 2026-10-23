//! ArgyllCMS en processus externe : `colprof` calcule le profil, `profcheck`
//! en mesure la précision (ADR 0002, version figée [`VERSION_ARGYLL`]).
//!
//! Le calcul se fait dans un dossier de travail temporaire, effacé à la fin,
//! réussie ou non. Les outils y sont lancés avec des noms de fichiers
//! relatifs : les commandes montrées sont exactement celles lancées.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::moteur::{
    DemandeProfil, ErreurMoteur, Etape, MoteurProfil, Precision, ProfilCalcule, Recette,
    VERSION_ARGYLL,
};

/// Noms des fichiers dans le dossier de travail.
const SRGB: &str = "sRGB.icm";
const PROFIL: &str = "profil.icc";
const BASE: &str = "mesure";
const TI3: &str = "mesure.ti3";

/// ArgyllCMS posé dans un dossier qui contient `bin/` et `ref/`.
#[derive(Clone, Debug)]
pub struct ArgyllCms {
    racine: PathBuf,
}

impl ArgyllCms {
    pub fn new(racine: PathBuf) -> Self {
        ArgyllCms { racine }
    }

    /// Les trois fichiers dont le calcul a besoin sont là.
    fn complet(&self) -> bool {
        self.outil("colprof").is_file()
            && self.outil("profcheck").is_file()
            && self.racine.join("ref").join(SRGB).is_file()
    }

    fn outil(&self, nom: &str) -> PathBuf {
        self.racine
            .join("bin")
            .join(format!("{nom}{}", std::env::consts::EXE_SUFFIX))
    }

    /// Version lue dans l'aide de `colprof` lancé sans argument.
    pub fn version(&self) -> Result<String, ErreurMoteur> {
        let sortie = commande(&self.outil("colprof"))
            .output()
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => ErreurMoteur::Introuvable,
                _ => ErreurMoteur::Echec {
                    etape: Etape::Version,
                    cause: e.to_string(),
                },
            })?;
        // colprof sans argument écrit son aide et rend un code d'erreur :
        // seul le texte compte ici.
        let texte = format!(
            "{}{}",
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr)
        );
        lire_version(&texte).ok_or_else(|| ErreurMoteur::Echec {
            etape: Etape::Version,
            cause: derniere_ligne(&texte),
        })
    }

    /// Lance un outil dans le dossier de travail et ajoute sa sortie au
    /// journal. Rend stdout et stderr réunis.
    fn lancer(
        &self,
        nom: &str,
        arguments: &[OsString],
        dossier: &Path,
        etape: Etape,
        journal: &mut String,
    ) -> Result<String, ErreurMoteur> {
        let sortie = commande(&self.outil(nom))
            .args(arguments)
            .current_dir(dossier)
            .output()
            .map_err(|e| ErreurMoteur::Echec {
                etape,
                cause: format!("{nom} : {e}"),
            })?;
        let stdout = String::from_utf8_lossy(&sortie.stdout);
        let stderr = String::from_utf8_lossy(&sortie.stderr);
        journal.push_str(&format!("$ {}\n{stdout}{stderr}", ligne(nom, arguments)));
        if !sortie.status.success() {
            let cause = if stderr.trim().is_empty() {
                derniere_ligne(&stdout)
            } else {
                derniere_ligne(&stderr)
            };
            return Err(ErreurMoteur::Echec { etape, cause });
        }
        Ok(format!("{stdout}{stderr}"))
    }
}

impl MoteurProfil for ArgyllCms {
    fn commandes(&self, demande: &DemandeProfil) -> Vec<String> {
        lignes_commandes(demande.recette, demande.description)
    }

    fn calculer(&self, demande: &DemandeProfil) -> Result<ProfilCalcule, ErreurMoteur> {
        let version = self.version()?;
        controler_version(&version)?;

        let dossier = DossierTravail::creer()?;
        let chemin = dossier.chemin();
        fs::write(chemin.join(TI3), demande.ti3)
            .map_err(|e| ErreurMoteur::Fichier(format!("{TI3} : {e}")))?;
        fs::copy(self.racine.join("ref").join(SRGB), chemin.join(SRGB))
            .map_err(|e| ErreurMoteur::Fichier(format!("ref/{SRGB} d'ArgyllCMS : {e}")))?;

        let mut journal = String::new();
        let colprof = arguments_colprof(
            demande.recette,
            demande.description,
            Path::new(SRGB),
            Path::new(PROFIL),
            Path::new(BASE),
        );
        self.lancer("colprof", &colprof, chemin, Etape::Profil, &mut journal)?;
        let controle = self.lancer(
            "profcheck",
            &arguments_profcheck(),
            chemin,
            Etape::Controle,
            &mut journal,
        )?;
        // Un profil sans précision n'est pas présenté comme terminé.
        let precision = lire_precision(&controle).ok_or_else(|| ErreurMoteur::Echec {
            etape: Etape::Controle,
            cause: format!("précision illisible : {}", derniere_ligne(&controle)),
        })?;
        let icc = fs::read(chemin.join(PROFIL))
            .map_err(|e| ErreurMoteur::Fichier(format!("{PROFIL} : {e}")))?;
        Ok(ProfilCalcule {
            icc,
            version,
            commandes: lignes_commandes(demande.recette, demande.description),
            journal,
            precision,
        })
    }
}

/// Commande sans fenêtre de console sous Windows.
fn commande(programme: &Path) -> Command {
    #[allow(unused_mut)]
    let mut commande = Command::new(programme);
    #[cfg(windows)]
    {
        // Pas de fenêtre de console derrière l'application.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
    commande
}

/// Cherche ArgyllCMS : `<dossier de l'exécutable>/argyll` (installation),
/// puis deux niveaux plus haut (`target/debug` → racine du dépôt, en
/// développement). Un dossier compte s'il contient `bin/colprof`,
/// `bin/profcheck` et `ref/sRGB.icm` : un ArgyllCMS incomplet est dit
/// introuvable avant tout calcul.
pub fn chercher_argyll(dossier_exe: &Path) -> Option<PathBuf> {
    [
        dossier_exe.join("argyll"),
        dossier_exe.join("..").join("..").join("argyll"),
    ]
    .into_iter()
    .find(|racine| ArgyllCms::new(racine.clone()).complet())
}

/// Arguments de `colprof` : `-v` (journal), la qualité de la recette, une
/// table perceptuelle à partir de sRGB (`-S … -cmt -dpp`), la description
/// (`-D`), le profil produit (`-O`) et la base du `.ti3` lu. Chaque valeur
/// reste un seul argument, quels que soient ses espaces ou guillemets.
pub fn arguments_colprof(
    recette: Recette,
    description: &str,
    srgb: &Path,
    sortie: &Path,
    base: &Path,
) -> Vec<OsString> {
    let qualite = match recette {
        Recette::Rapide => "-qm",
        Recette::Standard => "-qh",
        Recette::Approfondi => "-qu",
    };
    let mut a: Vec<OsString> = ["-v", qualite, "-S"].map(OsString::from).into();
    a.push(srgb.into());
    a.extend(["-cmt", "-dpp", "-D", description, "-O"].map(OsString::from));
    a.push(sortie.into());
    a.push(base.into());
    a
}

/// `profcheck -k` : écarts en ΔE00 entre le `.ti3` et le profil.
fn arguments_profcheck() -> Vec<OsString> {
    ["-k", TI3, PROFIL].map(OsString::from).into()
}

/// Les deux commandes du calcul, telles qu'elles sont lancées dans le
/// dossier de travail.
pub(crate) fn lignes_commandes(recette: Recette, description: &str) -> Vec<String> {
    let colprof = arguments_colprof(
        recette,
        description,
        Path::new(SRGB),
        Path::new(PROFIL),
        Path::new(BASE),
    );
    vec![
        ligne("colprof", &colprof),
        ligne("profcheck", &arguments_profcheck()),
    ]
}

/// Ligne de commande lisible : un argument avec espace ou guillemet est mis
/// entre guillemets, ses guillemets précédés de `\`.
fn ligne(outil: &str, arguments: &[OsString]) -> String {
    let mut ligne = outil.to_string();
    for a in arguments {
        let a = a.to_string_lossy();
        ligne.push(' ');
        if a.is_empty() || a.contains(|c: char| c.is_whitespace() || c == '"') {
            ligne.push_str(&format!("\"{}\"", a.replace('"', "\\\"")));
        } else {
            ligne.push_str(&a);
        }
    }
    ligne
}

/// Précision lue dans la ligne de bilan de `profcheck` :
/// `Profile check complete, peak err = 2.51, avg err = 0.45, RMS = 0.6`.
pub fn lire_precision(sortie_profcheck: &str) -> Option<Precision> {
    sortie_profcheck.lines().find_map(|l| {
        Some(Precision {
            max_de00: valeur_apres(l, "peak err =")?,
            moyenne_de00: valeur_apres(l, "avg err =")?,
        })
    })
}

fn valeur_apres(ligne: &str, cle: &str) -> Option<f64> {
    let reste = &ligne[ligne.find(cle)? + cle.len()..];
    reste
        .trim_start()
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?
        .parse()
        .ok()
}

/// Version lue dans l'aide de `colprof` : « Version 3.5.0 » → `"3.5.0"`.
/// Seul un numéro en trois parties compte (« AGPL Version 3 » n'en est pas
/// un). Deux numéros différents ne donnent rien : mieux vaut refuser que
/// retenir le mauvais.
pub fn lire_version(sortie_colprof: &str) -> Option<String> {
    let mots: Vec<&str> = sortie_colprof.split_whitespace().collect();
    let mut versions = mots.windows(2).filter_map(|m| {
        let v = m[1].trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        (m[0] == "Version" && numero_en_trois_parties(v)).then_some(v)
    });
    let premiere = versions.next()?;
    versions
        .all(|v| v == premiere)
        .then(|| premiere.to_string())
}

/// « 3.5.0 » : trois nombres séparés par des points.
fn numero_en_trois_parties(texte: &str) -> bool {
    let parties: Vec<&str> = texte.split('.').collect();
    parties.len() == 3
        && parties
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Refuse toute version autre que [`VERSION_ARGYLL`].
pub fn controler_version(trouvee: &str) -> Result<(), ErreurMoteur> {
    if trouvee == VERSION_ARGYLL {
        Ok(())
    } else {
        Err(ErreurMoteur::VersionNonValidee {
            trouvee: trouvee.to_string(),
        })
    }
}

/// Dernière ligne non vide d'une sortie, ou une phrase si elle est vide.
fn derniere_ligne(texte: &str) -> String {
    texte
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("aucun message")
        .to_string()
}

/// Dossier de travail unique sous le dossier temporaire du système, effacé
/// quand la garde sort de portée (calcul réussi ou non).
struct DossierTravail(PathBuf);

impl DossierTravail {
    fn creer() -> Result<Self, ErreurMoteur> {
        static RANG: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let nom = format!(
            "myiro-libre-profil-{}-{nanos}-{}",
            std::process::id(),
            RANG.fetch_add(1, Ordering::Relaxed)
        );
        let chemin = std::env::temp_dir().join(nom);
        fs::create_dir(&chemin)
            .map_err(|e| ErreurMoteur::Fichier(format!("dossier de travail : {e}")))?;
        Ok(DossierTravail(chemin))
    }

    fn chemin(&self) -> &Path {
        &self.0
    }
}

impl Drop for DossierTravail {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.0) {
            eprintln!("dossier de travail non effacé ({}) : {e}", self.0.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_dossier_de_travail_est_efface_apres_un_echec() {
        let chemin = {
            let d = DossierTravail::creer().unwrap();
            std::fs::write(d.chemin().join("mesure.ti3"), "x").unwrap();
            let _: Result<(), ()> = Err(()); // un échec du calcul fait sortir la garde de portée
            d.chemin().to_path_buf()
        };
        assert!(!chemin.exists());
    }

    #[test]
    fn deux_dossiers_de_travail_sont_distincts() {
        let a = DossierTravail::creer().unwrap();
        let b = DossierTravail::creer().unwrap();
        assert_ne!(a.chemin(), b.chemin());
    }

    #[test]
    fn un_argyll_absent_est_introuvable() {
        let vide = tempfile::tempdir().unwrap();
        let moteur = ArgyllCms::new(vide.path().join("argyll"));
        assert_eq!(moteur.version(), Err(ErreurMoteur::Introuvable));
        let demande = DemandeProfil {
            ti3: "CTI3\r\n",
            description: "Essai",
            recette: Recette::Standard,
        };
        assert_eq!(moteur.calculer(&demande), Err(ErreurMoteur::Introuvable));
    }

    #[test]
    fn la_cause_est_la_derniere_ligne_non_vide() {
        assert_eq!(
            derniere_ligne("colprof: Error - x\n  \nDiagnostic : trop peu de plages\n\n"),
            "Diagnostic : trop peu de plages"
        );
        assert_eq!(derniere_ligne(""), "aucun message");
    }
}
