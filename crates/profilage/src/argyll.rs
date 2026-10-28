//! ArgyllCMS en processus externe : `colprof` calcule le profil, `profcheck`
//! en mesure la précision (ADR 0002, version figée [`VERSION_ARGYLL`]).
//!
//! Le calcul se fait dans un dossier de travail temporaire, effacé à la fin,
//! réussie ou non. Les outils y sont lancés avec des noms de fichiers
//! relatifs : les commandes montrées sont exactement celles lancées.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::moteur::{
    etape_colprof, lire_avancement, Avancement, DemandeProfil, ErreurMoteur, Etape, EtapeCalcul,
    MoteurProfil, Precision, ProfilCalcule, Recette, VERSION_ARGYLL,
};

mod targen;

pub use targen::arguments_targen;
pub(crate) use targen::ligne_targen;

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

    /// Lance un outil d'ArgyllCMS dans le dossier de travail (voir
    /// [`executer`]).
    fn lancer(
        &self,
        nom: &str,
        arguments: &[OsString],
        dossier: &Path,
        etape: Etape,
        journal: &mut String,
        avancement: &mut dyn FnMut(Avancement),
    ) -> Result<String, ErreurMoteur> {
        executer(
            &self.outil(nom),
            nom,
            arguments,
            dossier,
            etape,
            journal,
            avancement,
        )
    }
}

/// Lance `programme` dans `dossier` et ajoute sa sortie au journal, sous la
/// ligne de commande `nom …`. Chaque ligne écrite est annoncée à
/// `avancement` dès qu'elle arrive (voir [`lire_avancement`]). Une lecture
/// interrompue est dite dans le journal. Rend stdout et stderr réunis.
fn executer(
    programme: &Path,
    nom: &str,
    arguments: &[OsString],
    dossier: &Path,
    etape: Etape,
    journal: &mut String,
    avancement: &mut dyn FnMut(Avancement),
) -> Result<String, ErreurMoteur> {
    let echec = |cause: String| ErreurMoteur::Echec {
        etape,
        cause: format!("{nom} : {cause}"),
    };
    let mut enfant = commande(programme)
        .args(arguments)
        .current_dir(dossier)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| echec(e.to_string()))?;
    let lu = lire_sorties(enfant.stdout.take(), enfant.stderr.take(), avancement);
    let (sortie, erreurs) = match lu {
        Ok(lu) => lu,
        Err(cause) => {
            // Plus personne ne lit l'outil : il est arrêté.
            let _ = enfant.kill();
            let _ = enfant.wait();
            journal.push_str(&format!("$ {}\n{cause}\n", ligne(nom, arguments)));
            return Err(echec(cause));
        }
    };
    let statut = enfant.wait().map_err(|e| echec(e.to_string()))?;
    let stdout = String::from_utf8_lossy(&sortie.octets);
    let stderr = String::from_utf8_lossy(&erreurs.octets);
    journal.push_str(&format!("$ {}\n{stdout}{stderr}", ligne(nom, arguments)));
    for (flux, lu) in [("sortie standard", &sortie), ("sortie d'erreur", &erreurs)] {
        if let Some(e) = &lu.erreur {
            journal.push_str(&format!("\nlecture interrompue ({flux}) : {e}\n"));
        }
    }
    if !statut.success() {
        // ArgyllCMS écrit ses erreurs « outil: Error - … » sur la sortie
        // d'erreur (relevé le 28 octobre 2026). Sans elle, l'outil a été
        // arrêté : ses pourcentages ou un avertissement ne sont pas une cause.
        return Err(match derniere_erreur(&stderr) {
            Some(cause) => ErreurMoteur::Echec { etape, cause },
            None => ErreurMoteur::Interrompu { etape },
        });
    }
    Ok(format!("{stdout}{stderr}"))
}

/// Lit stdout et stderr d'un outil jusqu'au bout, un fil par flux : un flux
/// plein ne bloque pas l'outil. Les lignes reviennent sur le fil de
/// l'appelant, dans l'ordre d'arrivée, et sont annoncées à `avancement`. Un
/// fil de lecture qui panique donne une erreur, pas une sortie vide.
fn lire_sorties(
    sortie: Option<impl Read + Send>,
    erreurs: Option<impl Read + Send>,
    avancement: &mut dyn FnMut(Avancement),
) -> Result<(FluxLu, FluxLu), String> {
    let (envoi, reception) = mpsc::channel();
    thread::scope(|s| {
        let envoi_sortie = envoi.clone();
        let lecteur_sortie = s.spawn(move || lire_flux(sortie, &envoi_sortie));
        let lecteur_erreurs = s.spawn(move || lire_flux(erreurs, &envoi));
        for l in reception {
            // Une ligne qui ouvre une partie de colprof l'annonce d'abord.
            if let Some(e) = etape_colprof(&l) {
                avancement(Avancement::Etape(e));
            }
            if let Some(a) = lire_avancement(&l) {
                avancement(a);
            }
        }
        let arrete =
            |flux: &str| format!("lecture de la {flux} arrêtée net (fil de lecture en panique)");
        let sortie = lecteur_sortie
            .join()
            .map_err(|_| arrete("sortie standard"))?;
        let erreurs = lecteur_erreurs
            .join()
            .map_err(|_| arrete("sortie d'erreur"))?;
        Ok((sortie, erreurs))
    })
}

/// Ce qu'un flux a donné : tous ses octets, et l'erreur qui a arrêté sa
/// lecture s'il y en a eu une.
#[derive(Debug)]
struct FluxLu {
    octets: Vec<u8>,
    erreur: Option<String>,
}

/// Lit un flux jusqu'au bout et le rend en entier (journal), avec l'erreur
/// qui a arrêté la lecture s'il y en a eu une. Chaque ligne, coupée sur `\r`
/// ou `\n`, est envoyée dès qu'elle est complète : un outil qui réécrit sa
/// ligne (« 45% » puis « 46% ») est suivi pas à pas.
fn lire_flux(flux: Option<impl Read>, envoi: &Sender<String>) -> FluxLu {
    let mut tout = Vec::new();
    let mut erreur = None;
    let Some(mut flux) = flux else {
        return FluxLu {
            octets: tout,
            erreur,
        };
    };
    let mut debut = 0;
    let mut tampon = [0u8; 4096];
    loop {
        match flux.read(&mut tampon) {
            Ok(0) => break,
            Ok(n) => tout.extend_from_slice(&tampon[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                erreur = Some(e.to_string());
                break;
            }
        }
        while let Some(i) = tout[debut..].iter().position(|&c| c == b'\r' || c == b'\n') {
            let _ = envoi.send(String::from_utf8_lossy(&tout[debut..debut + i]).into_owned());
            debut += i + 1;
        }
    }
    if debut < tout.len() {
        let _ = envoi.send(String::from_utf8_lossy(&tout[debut..]).into_owned());
    }
    FluxLu {
        octets: tout,
        erreur,
    }
}

impl MoteurProfil for ArgyllCms {
    fn commandes(&self, demande: &DemandeProfil) -> Vec<String> {
        lignes_commandes(demande.recette, demande.description)
    }

    fn calculer(
        &self,
        demande: &DemandeProfil,
        avancement: &mut dyn FnMut(Avancement),
    ) -> Result<ProfilCalcule, ErreurMoteur> {
        // Avant toute recherche : une recette en réglage ne lance rien.
        let colprof = arguments_colprof(
            demande.recette,
            demande.description,
            Path::new(SRGB),
            Path::new(PROFIL),
            Path::new(BASE),
        )
        .ok_or(ErreurMoteur::RecetteEnReglage)?;
        let version = self.version()?;
        controler_version(&version)?;

        let dossier = DossierTravail::creer()?;
        let chemin = dossier.chemin();
        fs::write(chemin.join(TI3), demande.ti3)
            .map_err(|e| ErreurMoteur::Fichier(format!("{TI3} : {e}")))?;
        fs::copy(self.racine.join("ref").join(SRGB), chemin.join(SRGB))
            .map_err(|e| ErreurMoteur::Fichier(format!("ref/{SRGB} d'ArgyllCMS : {e}")))?;

        let mut journal = String::new();
        avancement(Avancement::Etape(EtapeCalcul::Profil));
        self.lancer(
            "colprof",
            &colprof,
            chemin,
            Etape::Profil,
            &mut journal,
            avancement,
        )?;
        avancement(Avancement::Etape(EtapeCalcul::Controle));
        let controle = self.lancer(
            "profcheck",
            &arguments_profcheck(),
            chemin,
            Etape::Controle,
            &mut journal,
            avancement,
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

    fn commande_plages(&self, nombre: usize) -> String {
        ligne_targen(nombre)
    }

    fn generer_plages(&self, nombre: usize) -> Result<crate::moteur::PlagesGenerees, ErreurMoteur> {
        self.lancer_targen(nombre)
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

/// Description telle qu'elle part à `colprof -D` : ArgyllCMS 3.5.0 n'écrit
/// que de l'ASCII dans la description du profil (un accent y devient « ? »).
/// Les accents français et allemands courants sont donc retirés, œ, æ, ß et ẞ
/// écrits oe, ae, ss et SS ; tout autre caractère non ASCII donne « ? ». La
/// bibliothèque, elle, garde le nom d'origine.
pub fn description_argyll(description: &str) -> String {
    let mut ascii = String::with_capacity(description.len());
    for c in description.chars() {
        let remplacement = match c {
            'à' | 'â' | 'ä' => "a",
            'À' | 'Â' | 'Ä' => "A",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'É' | 'È' | 'Ê' | 'Ë' => "E",
            'î' | 'ï' => "i",
            'Î' | 'Ï' => "I",
            'ô' | 'ö' => "o",
            'Ô' | 'Ö' => "O",
            'ù' | 'û' | 'ü' => "u",
            'Ù' | 'Û' | 'Ü' => "U",
            'ÿ' => "y",
            'Ÿ' => "Y",
            'ç' => "c",
            'Ç' => "C",
            'œ' => "oe",
            'Œ' => "OE",
            'æ' => "ae",
            'Æ' => "AE",
            'ß' => "ss",
            'ẞ' => "SS",
            c if c.is_ascii() => {
                ascii.push(c);
                continue;
            }
            _ => "?",
        };
        ascii.push_str(remplacement);
    }
    ascii
}

/// Arguments de `colprof` : `-v` (journal), la qualité de la recette, une
/// table perceptuelle à partir de sRGB (`-S … -cmt -dpp`), la description
/// sans accents (`-D`, voir [`description_argyll`]), le profil produit
/// (`-O`) et la base du `.ti3` lu. Chaque valeur reste un seul argument,
/// quels que soient ses espaces ou guillemets. Rien pour une recette en
/// cours de réglage.
pub fn arguments_colprof(
    recette: Recette,
    description: &str,
    srgb: &Path,
    sortie: &Path,
    base: &Path,
) -> Option<Vec<OsString>> {
    let qualite = recette.qualite()?;
    let mut a: Vec<OsString> = ["-v", qualite, "-S"].map(OsString::from).into();
    a.push(srgb.into());
    a.extend(["-cmt", "-dpp", "-D"].map(OsString::from));
    a.push(description_argyll(description).into());
    a.push("-O".into());
    a.push(sortie.into());
    a.push(base.into());
    Some(a)
}

/// `profcheck -k` : écarts en ΔE00 entre le `.ti3` et le profil.
fn arguments_profcheck() -> Vec<OsString> {
    ["-k", TI3, PROFIL].map(OsString::from).into()
}

/// Les deux commandes du calcul, telles qu'elles sont lancées dans le
/// dossier de travail. Rien n'est lancé : elles se construisent même sans
/// ArgyllCMS. Aucune pour une recette en cours de réglage.
pub fn lignes_commandes(recette: Recette, description: &str) -> Vec<String> {
    let Some(colprof) = arguments_colprof(
        recette,
        description,
        Path::new(SRGB),
        Path::new(PROFIL),
        Path::new(BASE),
    ) else {
        return Vec::new();
    };
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

/// Précision lue dans la ligne de bilan de `profcheck -k` (confirmée avec
/// ArgyllCMS 3.5.0) :
/// `Profile check complete, errors(CIEDE2000): max. = 0.003447, avg. = 0.001526, RMS = 0.001680`.
/// Une ligne en CIE94 (`errors(CIE94)`, option `-c`) ou en CIE76
/// (`errors:`, sans option) n'est pas en ΔE00 : elle ne donne rien.
pub fn lire_precision(sortie_profcheck: &str) -> Option<Precision> {
    sortie_profcheck.lines().find_map(|l| {
        let bilan = l
            .split_once("Profile check complete, errors(CIEDE2000):")?
            .1;
        Some(Precision {
            max_de00: valeur_apres(bilan, "max. =")?,
            moyenne_de00: valeur_apres(bilan, "avg. =")?,
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

/// Dernière ligne d'erreur d'ArgyllCMS (« … Error - … ») d'une sortie.
fn derniere_erreur(texte: &str) -> Option<String> {
    texte
        .lines()
        .map(str::trim)
        .rfind(|l| l.contains("Error - "))
        .map(str::to_string)
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
        assert_eq!(
            moteur.calculer(&demande, &mut |_| {}),
            Err(ErreurMoteur::Introuvable)
        );
    }

    /// Un flux est rendu en entier pour le journal, et chaque ligne, coupée
    /// sur `\r` ou `\n`, est envoyée dès qu'elle est complète.
    #[test]
    fn un_flux_se_lit_ligne_a_ligne_et_reste_entier() {
        let (envoi, reception) = std::sync::mpsc::channel();
        let texte = b"Creating B to A tables\r\n 45%\r 46%\rfin sans retour";
        let lu = lire_flux(Some(&texte[..]), &envoi);
        assert_eq!(lu.erreur, None);
        let tout = lu.octets;
        drop(envoi);
        assert_eq!(tout, texte);
        let lignes: Vec<String> = reception.iter().collect();
        assert_eq!(
            lignes,
            [
                "Creating B to A tables",
                "",
                " 45%",
                " 46%",
                "fin sans retour"
            ]
        );
    }

    /// Un flux qui rend quelques octets puis une erreur de lecture.
    struct FluxCoupe(Option<&'static [u8]>);

    impl Read for FluxCoupe {
        fn read(&mut self, tampon: &mut [u8]) -> io::Result<usize> {
            match self.0.take() {
                Some(octets) => {
                    tampon[..octets.len()].copy_from_slice(octets);
                    Ok(octets.len())
                }
                None => Err(io::Error::other("tuyau rompu")),
            }
        }
    }

    /// Une erreur de lecture n'est pas avalée : elle est rendue avec ce qui a
    /// été lu avant elle, pour le journal.
    #[test]
    fn une_erreur_de_lecture_est_gardee() {
        let (envoi, reception) = std::sync::mpsc::channel();
        let lu = lire_flux(Some(FluxCoupe(Some(b"debut\r\n"))), &envoi);
        drop(envoi);
        assert_eq!(lu.octets, b"debut\r\n");
        assert_eq!(lu.erreur.as_deref(), Some("tuyau rompu"));
        assert_eq!(reception.iter().collect::<Vec<_>>(), ["debut", ""]);
    }

    /// Un flux qui fait paniquer son fil de lecture.
    struct FluxQuiPanique;

    impl Read for FluxQuiPanique {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("lecteur en panique (essai)")
        }
    }

    /// Un fil de lecture qui panique ne passe pas pour une sortie vide : la
    /// lecture rend une erreur claire.
    #[test]
    fn un_fil_de_lecture_en_panique_donne_une_erreur() {
        let resultat = lire_sorties(Some(FluxQuiPanique), Some(&b"ok\n"[..]), &mut |_| {});
        let erreur = resultat.unwrap_err();
        assert!(erreur.contains("sortie standard"), "{erreur}");
        let resultat = lire_sorties(Some(&b"ok\n"[..]), Some(FluxQuiPanique), &mut |_| {});
        assert!(resultat.unwrap_err().contains("sortie d'erreur"));
    }

    /// Script `cmd` écrit dans un dossier temporaire.
    #[cfg(windows)]
    fn script(dossier: &Path, nom: &str, lignes: &[&str]) {
        let texte: String = lignes.iter().map(|l| format!("{l}\r\n")).collect();
        std::fs::write(dossier.join(nom), texte).unwrap();
    }

    /// Un vrai processus court, sans ArgyllCMS, qui écrit beaucoup sur ses
    /// deux sorties à la fois (bien plus qu'un tuyau n'en tient) : rien ne
    /// bloque, le journal est complet et chaque ligne est annoncée.
    #[cfg(windows)]
    #[test]
    fn un_outil_bavard_sur_ses_deux_sorties_ne_bloque_pas() {
        let dossier = tempfile::tempdir().unwrap();
        script(
            dossier.path(),
            "bavard.cmd",
            &[
                "@echo off",
                "for /L %%i in (1,1,3000) do (",
                "  echo ligne %%i",
                "  echo erreur %%i 1>&2",
                ")",
            ],
        );
        let mut journal = String::new();
        let mut lignes = 0;
        let sortie = executer(
            Path::new("cmd"),
            "bavard",
            &["/c".into(), r".\bavard.cmd".into()],
            dossier.path(),
            Etape::Profil,
            &mut journal,
            &mut |a| {
                if matches!(a, Avancement::Ligne(_)) {
                    lignes += 1;
                }
            },
        )
        .unwrap();
        assert_eq!(lignes, 6000);
        assert_eq!(sortie.matches("ligne ").count(), 3000);
        assert_eq!(sortie.matches("erreur ").count(), 3000);
        assert!(
            journal.starts_with("$ bavard /c .\\bavard.cmd\n"),
            "{}",
            &journal[..40]
        );
        for i in [1, 1500, 3000] {
            assert!(journal.contains(&format!("ligne {i}\r\n")), "ligne {i}");
            assert!(journal.contains(&format!("erreur {i} ")), "erreur {i}");
        }
    }

    /// Un outil qui échoue : la cause est sa dernière ligne « Error - »
    /// (forme des erreurs d'ArgyllCMS, relevée le 28 octobre 2026 sur colprof
    /// et profcheck 3.5.0), et le journal garde tout ce qu'il a écrit.
    #[cfg(windows)]
    #[test]
    fn un_outil_qui_echoue_donne_sa_derniere_ligne_d_erreur() {
        let dossier = tempfile::tempdir().unwrap();
        script(
            dossier.path(),
            "echec.cmd",
            &[
                "@echo off",
                "echo debut du calcul",
                "echo colprof: Error - trop peu de plages 1>&2",
                "echo   45%%",
                "exit /b 1",
            ],
        );
        let mut journal = String::new();
        let erreur = executer(
            Path::new("cmd"),
            "echec",
            &["/c".into(), r".\echec.cmd".into()],
            dossier.path(),
            Etape::Profil,
            &mut journal,
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(
            erreur,
            ErreurMoteur::Echec {
                etape: Etape::Profil,
                cause: "colprof: Error - trop peu de plages".into()
            }
        );
        assert!(journal.contains("debut du calcul") && journal.contains("trop peu de plages"));
    }

    /// Un outil arrêté de l'extérieur (programme fermé, poste en veille)
    /// n'écrit pas d'erreur d'ArgyllCMS : le calcul est dit interrompu, sans
    /// présenter ses pourcentages ni un avertissement comme une cause.
    #[cfg(windows)]
    #[test]
    fn un_outil_arrete_sans_erreur_est_dit_interrompu() {
        let dossier = tempfile::tempdir().unwrap();
        script(
            dossier.path(),
            "arret.cmd",
            &[
                "@echo off",
                "echo Creating B to A tables",
                "echo   0%%",
                "echo  40%%",
                "echo Warning: un avertissement 1>&2",
                "exit /b 1",
            ],
        );
        let mut journal = String::new();
        let erreur = executer(
            Path::new("cmd"),
            "arret",
            &["/c".into(), r".\arret.cmd".into()],
            dossier.path(),
            Etape::Profil,
            &mut journal,
            &mut |_| {},
        )
        .unwrap_err();
        assert_eq!(
            erreur,
            ErreurMoteur::Interrompu {
                etape: Etape::Profil
            }
        );
        assert!(journal.contains("40%") && journal.contains("un avertissement"));
    }

    #[test]
    fn la_cause_est_la_derniere_erreur_d_argyll() {
        assert_eq!(
            derniere_erreur("Warning: x\ncolprof: Error - a\nprofcheck: Error - b\n 40%\n"),
            Some("profcheck: Error - b".to_string())
        );
        assert_eq!(derniere_erreur("Warning: x\n  0%\n 40%\n"), None);
        assert_eq!(derniere_erreur(""), None);
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
