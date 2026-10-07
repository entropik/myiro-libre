//! Adapter réel : `FDXSDK.dll` chargée dans le processus du pont.
//!
//! Seuls les exports de `fdx_sys::EXPORTS_AUTORISES` sont résolus ; toute
//! autre adresse n'est jamais demandée à la DLL. Conventions d'appel :
//! `__stdcall` en x86, convention Windows en x64 (`extern "system"`).

use crate::SdkMyiro1;
use fdx_sys::{Port, Version, EXPORTS_AUTORISES, TAILLE_TAMPON_INFOS};
use libloading::Library;
use std::path::Path;

/// Capacité maximale admise par `FDX_GetDevicePortList`.
const CAPACITE_MAX_PORTS: u32 = 100;

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

type FnVersion = unsafe extern "system" fn(*mut Version) -> i32;
type FnPorts = unsafe extern "system" fn(*mut Port, *mut u32, u32) -> i32;
type FnConnecter = unsafe extern "system" fn(*const Port, u32) -> i32;
type FnSansArgument = unsafe extern "system" fn() -> i32;
type FnInfos = unsafe extern "system" fn(*mut u8) -> i32;

pub struct FdxDll {
    version: FnVersion,
    ports: FnPorts,
    connecter: FnConnecter,
    deconnecter: FnSansArgument,
    infos: FnInfos,
    connecte: bool,
    // Déclarée en dernier : la DLL reste chargée tant que les pointeurs vivent.
    _bibliotheque: Library,
}

impl FdxDll {
    pub fn charger(chemin: impl AsRef<Path>) -> Result<Self, ErreurChargement> {
        let chemin = chemin.as_ref();
        // SAFETY : charger la DLL exécute son code d'initialisation. FDXSDK
        // n'importe que des bibliothèques système et n'ouvre aucune liaison
        // avec l'instrument avant FDX_GetDevicePortList ou FDX_Connect.
        let bibliotheque = unsafe { Library::new(chemin) }
            .map_err(|e| ErreurChargement::DllIntrouvable(format!("{}: {e}", chemin.display())))?;
        Ok(FdxDll {
            version: resoudre(&bibliotheque, "FDX_GetSDKVersion")?,
            ports: resoudre(&bibliotheque, "FDX_GetDevicePortList")?,
            connecter: resoudre(&bibliotheque, "FDX_Connect")?,
            deconnecter: resoudre(&bibliotheque, "FDX_Disconnect")?,
            infos: resoudre(&bibliotheque, "FDX_GetDeviceInfo")?,
            connecte: false,
            _bibliotheque: bibliotheque,
        })
    }
}

fn resoudre<T: Copy>(bibliotheque: &Library, nom: &str) -> Result<T, ErreurChargement> {
    autoriser_export(nom)?;
    let mut symbole = nom.as_bytes().to_vec();
    symbole.push(0);
    // SAFETY : le type T est la signature établie dans docs/abi/ pour ce nom.
    unsafe { bibliotheque.get::<T>(&symbole) }
        .map(|s| *s)
        .map_err(|_| ErreurChargement::ExportAbsent(nom.to_string()))
}

fn verifier(code: i32) -> Result<i32, i32> {
    if code < 0 {
        Err(code)
    } else {
        Ok(code)
    }
}

impl SdkMyiro1 for FdxDll {
    fn version(&mut self) -> Result<Version, i32> {
        let mut version = Version::default();
        // SAFETY : un pointeur vers 12 octets (fiche FDX_GetSDKVersion).
        verifier(unsafe { (self.version)(&mut version) })?;
        Ok(version)
    }

    fn ports(&mut self) -> Result<Vec<Port>, i32> {
        // Appel en deux temps, comme MY-CT1 (fiche FDX_GetDevicePortList).
        let mut nombre = 0u32;
        // SAFETY : tableau nul et capacité 0 admis ; compteur valide.
        verifier(unsafe { (self.ports)(std::ptr::null_mut(), &mut nombre, 0) })?;
        if nombre == 0 {
            return Ok(Vec::new());
        }
        let capacite = nombre.min(CAPACITE_MAX_PORTS);
        let vide = Port {
            code_liaison: 0,
            opaque: [0; 40],
        };
        let mut liste = vec![vide; capacite as usize];
        let mut trouves = 0u32;
        // SAFETY : `liste` contient `capacite` entrées de 44 octets.
        verifier(unsafe { (self.ports)(liste.as_mut_ptr(), &mut trouves, capacite) })?;
        // Si un instrument est apparu entre les deux appels, la DLL n'a rien
        // copié (capacité insuffisante) : on ne garde que ce qui est sûr.
        if trouves > capacite {
            return Ok(Vec::new());
        }
        liste.truncate(trouves as usize);
        Ok(liste)
    }

    fn connecter(&mut self, port: &Port, delai: u32) -> Result<i32, i32> {
        // SAFETY : entrée de 44 octets reçue de la détection, rendue intacte.
        let code = verifier(unsafe { (self.connecter)(port, delai) })?;
        self.connecte = true;
        Ok(code)
    }

    fn infos(&mut self) -> Result<[u8; TAILLE_TAMPON_INFOS], i32> {
        let mut tampon = [0u8; TAILLE_TAMPON_INFOS];
        // SAFETY : tampon plus large que les 40 octets écrits par la DLL.
        verifier(unsafe { (self.infos)(tampon.as_mut_ptr()) })?;
        Ok(tampon)
    }
}

impl Drop for FdxDll {
    fn drop(&mut self) {
        if self.connecte {
            // SAFETY : aucun argument ; ferme la session ouverte par FDX_Connect.
            unsafe { (self.deconnecter)() };
        }
    }
}
