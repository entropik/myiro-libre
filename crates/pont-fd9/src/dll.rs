//! Adapter réel : `FD9SDK.dll` chargée dans le processus du pont.
//!
//! Seuls les exports de `fd9_sys::EXPORTS_AUTORISES` sont résolus ; toute
//! autre adresse n'est jamais demandée à la DLL. Convention d'appel :
//! `__stdcall` en x86, convention Windows en x64 (`extern "system"`), fiche
//! `docs/abi/FD9SDK.md`. La DLL de FD-S2w est 32 bits : elle demande le pont
//! compilé pour `i686-pc-windows-msvc`.

use crate::SdkFd9;
use fd9_sys::{Appareil, EXPORTS_AUTORISES};
use libloading::Library;
use std::path::Path;

/// Exports dont l'adapter demande l'adresse au chargement : toute la liste
/// blanche, pour vérifier que la DLL est complète (fiche
/// `docs/abi/FD9_GetLastError.md`, palier Version). Seuls `FD9_GetLastError`
/// et `FD9_GetDeviceList` sont appelés par les paliers de ce pont.
pub const EXPORTS_RESOLUS: &[&str] = EXPORTS_AUTORISES;

#[derive(Debug, PartialEq, Eq)]
pub enum ErreurChargement {
    DllIntrouvable(String),
    /// Export absent de la liste blanche : son adresse n'est pas demandée.
    ExportRefuse(String),
    /// Export autorisé mais absent de cette DLL.
    ExportAbsent(String),
}

/// Vérifie qu'un export figure dans la liste blanche, avant toute résolution.
pub fn autoriser_export(nom: &str) -> Result<(), ErreurChargement> {
    if EXPORTS_AUTORISES.contains(&nom) {
        Ok(())
    } else {
        Err(ErreurChargement::ExportRefuse(nom.to_string()))
    }
}

type FnDernierCode = unsafe extern "system" fn() -> i32;
type FnLister = unsafe extern "system" fn(*mut Appareil, *mut u32, *const u32) -> i32;

pub struct Fd9Dll {
    dernier_code: FnDernierCode,
    lister: FnLister,
    version_fichier: Option<[u32; 4]>,
    empreinte: Option<String>,
    // Déclarée en dernier : la DLL reste chargée tant que les pointeurs vivent.
    _bibliotheque: Library,
}

impl Fd9Dll {
    pub fn charger(chemin: impl AsRef<Path>) -> Result<Self, ErreurChargement> {
        let chemin = chemin.as_ref();
        // Lus avant le chargement, sur le fichier seul.
        let version_fichier = version_du_fichier(chemin);
        let empreinte = empreinte_fichier(chemin).ok();
        let bibliotheque = ouvrir(chemin).map_err(|e| {
            // libloading garde l'erreur Windows (126 : module introuvable,
            // 193 : mauvaise architecture) dans la source de son erreur.
            let cause = std::error::Error::source(&e)
                .map(|s| format!(" ({s})"))
                .unwrap_or_default();
            ErreurChargement::DllIntrouvable(format!("{}: {e}{cause}", chemin.display()))
        })?;
        for nom in EXPORTS_RESOLUS {
            verifier_presence(&bibliotheque, nom)?;
        }
        Ok(Fd9Dll {
            dernier_code: resoudre(&bibliotheque, "FD9_GetLastError")?,
            lister: resoudre(&bibliotheque, "FD9_GetDeviceList")?,
            version_fichier,
            empreinte,
            _bibliotheque: bibliotheque,
        })
    }
}

/// Charge la DLL en cherchant ses dépendances dans son propre dossier :
/// FD9SDK importe des DLL livrées à côté d'elle (`DIColor.dll`,
/// `FD9BarcodeManager.dll`, `opencv_*247.dll`, `VCOMP*.DLL`).
fn ouvrir(chemin: &Path) -> Result<Library, libloading::Error> {
    #[cfg(windows)]
    {
        use libloading::os::windows::{Library as LibWin, LOAD_WITH_ALTERED_SEARCH_PATH};
        let chemin = chemin_pour_chargement(chemin);
        // SAFETY : charger la DLL exécute son code d'initialisation et celui de
        // ses dépendances. FD9SDK n'ouvre aucune liaison avec l'instrument
        // avant FD9_GetDeviceList ou FD9_Connect (fiches docs/abi/FD9_*).
        unsafe { LibWin::load_with_flags(&chemin, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into)
    }
    #[cfg(not(windows))]
    {
        // SAFETY : idem ; hors Windows, aucune DLL du fabricant n'existe.
        unsafe { Library::new(chemin) }
    }
}

/// Chemin donné à `LoadLibraryExW` : absolu et écrit avec des « \ ». Avec
/// `LOAD_WITH_ALTERED_SEARCH_PATH`, Windows ne cherche les dépendances dans le
/// dossier de la DLL que pour un tel chemin ; avec des « / », la DLL de FD-S2w
/// ne se charge pas (constaté sur le poste le 9 octobre 2026).
#[cfg(windows)]
pub fn chemin_pour_chargement(chemin: &Path) -> std::path::PathBuf {
    let absolu = std::path::absolute(chemin).unwrap_or_else(|_| chemin.to_path_buf());
    std::path::PathBuf::from(absolu.to_string_lossy().replace('/', "\\"))
}

/// Vérifie qu'un export autorisé existe, sans lui donner de forme d'appel :
/// son adresse n'est lue que comme une adresse, jamais appelée.
fn verifier_presence(bibliotheque: &Library, nom: &str) -> Result<(), ErreurChargement> {
    autoriser_export(nom)?;
    // SAFETY : le symbole est lu comme une simple adresse, ni appelé ni lu.
    unsafe { bibliotheque.get::<*const std::ffi::c_void>(nom.as_bytes()) }
        .map(|_| ())
        .map_err(|_| ErreurChargement::ExportAbsent(nom.to_string()))
}

fn resoudre<T: Copy>(bibliotheque: &Library, nom: &str) -> Result<T, ErreurChargement> {
    autoriser_export(nom)?;
    // SAFETY : le nom est dans la liste blanche et le type de pointeur suit sa
    // fiche docs/abi/.
    unsafe { bibliotheque.get::<T>(nom.as_bytes()) }
        .map(|symbole| *symbole)
        .map_err(|_| ErreurChargement::ExportAbsent(nom.to_string()))
}

impl SdkFd9 for Fd9Dll {
    fn dernier_code(&mut self) -> i32 {
        // SAFETY : aucun argument, relit une variable globale de la DLL.
        unsafe { (self.dernier_code)() }
    }

    fn lister(&mut self, tableau: &mut [Appareil], trouves: &mut u32) -> i32 {
        // La DLL écrit au plus `capacite` entrées de 44 octets et ne garde
        // aucune adresse après le retour (fiche FD9_GetDeviceList).
        let capacite = tableau.len() as u32;
        // SAFETY : tableau de `capacite` entrées `#[repr(C)]` de 44 octets,
        // compteur et capacité valides pendant l'appel.
        unsafe { (self.lister)(tableau.as_mut_ptr(), trouves, &capacite) }
    }

    fn version_fichier(&self) -> Option<[u32; 4]> {
        self.version_fichier
    }

    fn empreinte(&self) -> Option<String> {
        self.empreinte.clone()
    }
}

/// SHA-256 d'un fichier, en hexadécimal minuscule.
pub fn empreinte_fichier(chemin: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let contenu = std::fs::read(chemin)?;
    Ok(Sha256::digest(&contenu)
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect())
}

/// Version binaire du fichier (`VS_FIXEDFILEINFO`, majeur, mineur, révision,
/// build), lue par l'outil de version de Windows sans exécuter le fichier.
/// `None` si le fichier n'a pas de ressource de version.
#[cfg(windows)]
pub fn version_du_fichier(chemin: &Path) -> Option<[u32; 4]> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "version")]
    extern "system" {
        fn GetFileVersionInfoSizeW(nom: *const u16, poignee: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            nom: *const u16,
            poignee: u32,
            taille: u32,
            donnees: *mut c_void,
        ) -> i32;
        fn VerQueryValueW(
            bloc: *const c_void,
            chemin: *const u16,
            valeur: *mut *mut c_void,
            taille: *mut u32,
        ) -> i32;
    }

    let nom: Vec<u16> = chemin.as_os_str().encode_wide().chain([0]).collect();
    let racine: Vec<u16> = "\\".encode_utf16().chain([0]).collect();
    let mut poignee = 0;
    // SAFETY : textes terminés par zéro ; tampon de la taille annoncée ; la
    // valeur rendue pointe dans ce tampon, lue seulement sur `taille` octets.
    unsafe {
        let taille = GetFileVersionInfoSizeW(nom.as_ptr(), &mut poignee);
        if taille == 0 {
            return None;
        }
        let mut tampon = vec![0u8; taille as usize];
        if GetFileVersionInfoW(nom.as_ptr(), 0, taille, tampon.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let mut valeur: *mut c_void = std::ptr::null_mut();
        let mut longueur = 0u32;
        if VerQueryValueW(
            tampon.as_ptr().cast(),
            racine.as_ptr(),
            &mut valeur,
            &mut longueur,
        ) == 0
            || valeur.is_null()
            || longueur < 16
        {
            return None;
        }
        let mots = std::slice::from_raw_parts(valeur.cast::<u8>(), 16);
        let mot = |i: usize| u32::from_le_bytes(mots[i..i + 4].try_into().unwrap());
        // Signature de VS_FIXEDFILEINFO.
        if mot(0) != 0xFEEF_04BD {
            return None;
        }
        let (haut, bas) = (mot(8), mot(12));
        Some([haut >> 16, haut & 0xffff, bas >> 16, bas & 0xffff])
    }
}

#[cfg(not(windows))]
pub fn version_du_fichier(_chemin: &Path) -> Option<[u32; 4]> {
    None
}
