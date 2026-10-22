//! Installer un profil ICC dans Windows, sans jamais écraser un autre profil
//! du même nom. Même modèle que le pare-feu (ADR 0005) : le trait
//! [`InstallationProfils`] a deux adapters, [`InstallationWindows`] (le vrai
//! dossier des couleurs, par `mscms`) et [`InstallationSimulee`] (un dossier
//! ordinaire), seul utilisé par les tests.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Pourquoi un profil n'a pas été installé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurInstallation {
    /// Un autre profil du même nom, au contenu différent, est déjà installé.
    DejaPresentDifferent,
    /// Windows a refusé (accès refusé : droits d'administrateur sans doute).
    Refusee,
    /// Le nom n'est pas un simple nom de fichier `.icc` ou `.icm`.
    NomInvalide,
    /// Autre échec, avec son détail technique (console seulement).
    Systeme(String),
}

/// Résultat d'une installation réussie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatInstallation {
    /// Le profil vient d'être installé.
    Installe,
    /// Le même profil, octet pour octet, était déjà installé : rien n'a changé.
    DejaInstalle,
}

/// Le dossier des couleurs d'un système et l'installation d'un profil.
pub trait InstallationProfils {
    /// Dossier où les profils installés se trouvent.
    fn dossier(&self) -> Result<PathBuf, ErreurInstallation>;
    /// Installe le fichier : il est copié dans [`Self::dossier`] sous son nom.
    fn installer(&self, fichier: &Path) -> Result<(), ErreurInstallation>;
}

/// Un nom de profil installable : un simple nom de fichier (ni dossier, ni
/// lecteur, ni `..`), qui finit par `.icc` ou `.icm` sans tenir compte des
/// majuscules. Le nom vient de la bibliothèque, mais rien n'est écrit sur
/// le disque avec un nom qui sortirait de son dossier.
fn nom_valide(nom: &str) -> bool {
    let simple = Path::new(nom).file_name() == Some(std::ffi::OsStr::new(nom))
        && !nom.contains(['/', '\\', ':']);
    let extension = nom.rsplit_once('.').is_some_and(|(base, ext)| {
        !base.is_empty() && (ext.eq_ignore_ascii_case("icc") || ext.eq_ignore_ascii_case("icm"))
    });
    simple && extension
}

/// Le profil `nom` est-il installé avec exactement ce contenu ?
pub fn est_installe(systeme: &dyn InstallationProfils, nom: &str, icc: &[u8]) -> bool {
    nom_valide(nom)
        && systeme
            .dossier()
            .ok()
            .and_then(|d| std::fs::read(d.join(nom)).ok())
            .is_some_and(|present| present == icc)
}

/// Installe le profil `icc` sous le nom `nom`. Déjà présent à l'identique :
/// rien à faire. Présent avec un autre contenu : refusé, l'autre profil
/// reste. Sinon, le fichier est écrit dans `temporaire`, puis installé.
pub fn installer_profil(
    systeme: &dyn InstallationProfils,
    nom: &str,
    icc: &[u8],
    temporaire: &Path,
) -> Result<EtatInstallation, ErreurInstallation> {
    if !nom_valide(nom) {
        return Err(ErreurInstallation::NomInvalide);
    }
    let installe = systeme.dossier()?.join(nom);
    match std::fs::read(&installe) {
        Ok(present) if present == icc => return Ok(EtatInstallation::DejaInstalle),
        Ok(_) => return Err(ErreurInstallation::DejaPresentDifferent),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(ErreurInstallation::Systeme(format!(
                "{} : {e}",
                installe.display()
            )))
        }
    }
    // Entre cette vérification et `InstallColorProfileW`, un autre programme
    // pourrait poser un profil du même nom, que l'installation remplacerait
    // peut-être (comportement de Windows non établi). L'instant est court :
    // on l'accepte plutôt que de verrouiller le dossier des couleurs.
    let fichier = temporaire.join(nom);
    std::fs::write(&fichier, icc)
        .map_err(|e| ErreurInstallation::Systeme(format!("{} : {e}", fichier.display())))?;
    let resultat = systeme.installer(&fichier);
    // Windows a copié le profil dans son dossier : le fichier temporaire ne sert plus.
    let _ = std::fs::remove_file(&fichier);
    resultat.map(|()| EtatInstallation::Installe)
}

/// Clé du catalogue qui explique un échec d'installation à l'opérateur.
pub fn cle_erreur(erreur: &ErreurInstallation) -> &'static str {
    match erreur {
        ErreurInstallation::DejaPresentDifferent => "installation.deja_present",
        ErreurInstallation::Refusee => "installation.refusee",
        ErreurInstallation::NomInvalide => "installation.nom_invalide",
        ErreurInstallation::Systeme(detail) => {
            eprintln!("installation du profil : {detail}");
            "installation.echec"
        }
    }
}

/// Un dossier ordinaire tenu pour le dossier des couleurs : l'installation
/// y copie le fichier, ou refuse tout si `refuse`.
#[derive(Clone, Debug)]
pub struct InstallationSimulee {
    pub dossier: PathBuf,
    pub refuse: bool,
}

impl InstallationProfils for InstallationSimulee {
    fn dossier(&self) -> Result<PathBuf, ErreurInstallation> {
        Ok(self.dossier.clone())
    }

    fn installer(&self, fichier: &Path) -> Result<(), ErreurInstallation> {
        if self.refuse {
            return Err(ErreurInstallation::Refusee);
        }
        let nom = fichier.file_name().ok_or_else(|| {
            ErreurInstallation::Systeme(format!("{} : sans nom", fichier.display()))
        })?;
        std::fs::copy(fichier, self.dossier.join(nom))
            .map(|_| ())
            .map_err(|e| ErreurInstallation::Systeme(format!("{} : {e}", fichier.display())))
    }
}

/// Le vrai dossier des couleurs de Windows (`GetColorDirectoryW`) et
/// l'installation par `InstallColorProfileW` (`mscms.dll`). Jamais appelé
/// par les tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct InstallationWindows;

#[cfg(windows)]
mod mscms {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    use super::ErreurInstallation;

    #[link(name = "mscms")]
    extern "system" {
        fn GetColorDirectoryW(machine: *const u16, dossier: *mut u16, taille: *mut u32) -> i32;
        fn InstallColorProfileW(machine: *const u16, profil: *const u16) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
    }

    const ERROR_ACCESS_DENIED: u32 = 5;
    /// Caractères du chemin du dossier des couleurs, zéro final compris.
    const LONGUEUR_MAX: usize = 1024;

    fn echec(quoi: &str) -> ErreurInstallation {
        // SAFETY : lecture du code d'erreur du fil courant, sans argument.
        let code = unsafe { GetLastError() };
        if code == ERROR_ACCESS_DENIED {
            ErreurInstallation::Refusee
        } else {
            ErreurInstallation::Systeme(format!("{quoi} : erreur Windows {code}"))
        }
    }

    pub(super) fn dossier() -> Result<PathBuf, ErreurInstallation> {
        let mut tampon = vec![0u16; LONGUEUR_MAX];
        let mut taille = u32::try_from(tampon.len() * 2).unwrap_or(u32::MAX);
        // SAFETY : `tampon` fait `taille` octets ; machine nulle = ce poste.
        let ok = unsafe { GetColorDirectoryW(std::ptr::null(), tampon.as_mut_ptr(), &mut taille) };
        if ok == 0 {
            return Err(echec("GetColorDirectoryW"));
        }
        let fin = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
        Ok(PathBuf::from(OsString::from_wide(&tampon[..fin])))
    }

    pub(super) fn installer(fichier: &Path) -> Result<(), ErreurInstallation> {
        let chemin: Vec<u16> = fichier.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY : chemin UTF-16 terminé par zéro, vivant pendant l'appel.
        let ok = unsafe { InstallColorProfileW(std::ptr::null(), chemin.as_ptr()) };
        if ok == 0 {
            return Err(echec("InstallColorProfileW"));
        }
        Ok(())
    }
}

#[cfg(windows)]
impl InstallationProfils for InstallationWindows {
    fn dossier(&self) -> Result<PathBuf, ErreurInstallation> {
        mscms::dossier()
    }

    fn installer(&self, fichier: &Path) -> Result<(), ErreurInstallation> {
        mscms::installer(fichier)
    }
}

#[cfg(not(windows))]
impl InstallationProfils for InstallationWindows {
    fn dossier(&self) -> Result<PathBuf, ErreurInstallation> {
        Err(ErreurInstallation::Systeme("pas sous Windows".into()))
    }

    fn installer(&self, _: &Path) -> Result<(), ErreurInstallation> {
        Err(ErreurInstallation::Systeme("pas sous Windows".into()))
    }
}
