//! Adapter réel : `FDXSDK.dll` chargée dans le processus du pont.
//!
//! Seuls les exports de `fdx_sys::EXPORTS_AUTORISES` sont résolus ; toute
//! autre adresse n'est jamais demandée à la DLL. Conventions d'appel :
//! `__stdcall` en x86, convention Windows en x64 (`extern "system"`).

use crate::{Evenement, Lecture, SdkMyiro1};
use fdx_sys::{
    ConditionCalcul, ConditionMesure, DescripteurResultat, Port, Version, EXPORTS_AUTORISES,
    MESURE_PONCTUELLE, TAILLE_TAMPON_INFOS,
};
use libloading::Library;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// Capacité maximale admise par `FDX_GetDevicePortList`.
const CAPACITE_MAX_PORTS: u32 = 100;

/// Nombre de résultats au plus par lecture : une bande en compte quelques dizaines.
const CAPACITE_MAX_RESULTATS: u32 = 1000;

/// Type d'étalonnage « blanc » de `FDX_Calibration` ; les types 1 et 2
/// (lumière ambiante, écran) ne sont jamais transmis.
const ETALONNAGE_BLANC: i32 = 0;

// La DLL appelle le rappel depuis son propre fil d'exécution, sans contexte
// utilisateur : les événements passent par cette file globale. La DLL ne
// gère qu'une session à la fois, le pont aussi.
static FILE_EVENEMENTS: Mutex<VecDeque<Evenement>> = Mutex::new(VecDeque::new());
static NOUVEL_EVENEMENT: Condvar = Condvar::new();

/// Rappel donné à la DLL : dépose l'événement et rend la main aussitôt,
/// sans jamais rappeler la DLL (fiche FDX_RegisterDeviceEventHandler).
extern "C" fn rappel(code: i32, nb_donnees_brutes: u32, erreur: i32) {
    if let Ok(mut file) = FILE_EVENEMENTS.lock() {
        file.push_back(Evenement {
            code,
            nb_donnees_brutes,
            erreur,
        });
    }
    NOUVEL_EVENEMENT.notify_all();
}

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
type FnRappel = extern "C" fn(i32, u32, i32);
type FnEnregistrer = unsafe extern "system" fn(Option<FnRappel>) -> i32;
type FnEtalonner = unsafe extern "system" fn(i32) -> i32;
type FnArmer = unsafe extern "system" fn(*const ConditionMesure) -> i32;
type FnLire = unsafe extern "system" fn(
    *mut DescripteurResultat,
    *mut u32,
    *mut u32,
    u32,
    *const ConditionCalcul,
) -> i32;

pub struct FdxDll {
    version: FnVersion,
    ports: FnPorts,
    connecter: FnConnecter,
    deconnecter: FnSansArgument,
    infos: FnInfos,
    enregistrer: FnEnregistrer,
    etalonner: FnEtalonner,
    armer: FnArmer,
    arreter: FnSansArgument,
    lire: FnLire,
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
        let dll = FdxDll {
            version: resoudre(&bibliotheque, "FDX_GetSDKVersion")?,
            ports: resoudre(&bibliotheque, "FDX_GetDevicePortList")?,
            connecter: resoudre(&bibliotheque, "FDX_Connect")?,
            deconnecter: resoudre(&bibliotheque, "FDX_Disconnect")?,
            infos: resoudre(&bibliotheque, "FDX_GetDeviceInfo")?,
            enregistrer: resoudre(&bibliotheque, "FDX_RegisterDeviceEventHandler")?,
            etalonner: resoudre(&bibliotheque, "FDX_Calibration")?,
            armer: resoudre(&bibliotheque, "FDX_SetMeasureCondition")?,
            arreter: resoudre(&bibliotheque, "FDX_StopMeasurement")?,
            lire: resoudre(&bibliotheque, "FDX_GetMeasureData")?,
            connecte: false,
            _bibliotheque: bibliotheque,
        };
        // Rappel enregistré avant toute connexion, comme EIZO et MYIRO tools.
        if let Ok(mut file) = FILE_EVENEMENTS.lock() {
            file.clear();
        }
        // SAFETY : `rappel` est une fonction statique, valide toute la session.
        unsafe { (dll.enregistrer)(Some(rappel)) };
        Ok(dll)
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

    fn etalonner_blanc(&mut self) -> Result<i32, i32> {
        // SAFETY : un entier, toujours le type 0 (fiche FDX_Calibration).
        verifier(unsafe { (self.etalonner)(ETALONNAGE_BLANC) })
    }

    fn armer_ponctuelle(&mut self) -> Result<i32, i32> {
        let condition = ConditionMesure {
            type_mesure: MESURE_PONCTUELLE,
            option: 0,
        };
        // SAFETY : pointeur vers 8 octets (fiche FDX_SetMeasureCondition).
        verifier(unsafe { (self.armer)(&condition) })
    }

    fn arreter_mesure(&mut self) -> Result<i32, i32> {
        // SAFETY : aucun argument (fiche FDX_StopMeasurement).
        verifier(unsafe { (self.arreter)() })
    }

    fn lire(&mut self, condition: &ConditionCalcul, longueur: usize) -> Result<Lecture, i32> {
        // Appel en deux temps, comme MYIRO tools (fiche FDX_GetMeasureData).
        let mut nombre = 0u32;
        let mut sens = 0u32;
        // SAFETY : tableau nul et capacité 0 pour connaître le nombre de résultats.
        verifier(unsafe {
            (self.lire)(std::ptr::null_mut(), &mut nombre, &mut sens, 0, condition)
        })?;
        let capacite = nombre.min(CAPACITE_MAX_RESULTATS);
        let mut tampons = vec![vec![0f32; longueur]; capacite as usize];
        let mut descripteurs: Vec<DescripteurResultat> = tampons
            .iter_mut()
            .map(|t| DescripteurResultat {
                valeurs: t.as_mut_ptr(),
                longueur: longueur as u32,
            })
            .collect();
        let mut rendus = 0u32;
        if capacite > 0 {
            // SAFETY : `capacite` descripteurs, chacun vers `longueur` float alloués ici,
            // vivants jusqu'à la fin de l'appel.
            verifier(unsafe {
                (self.lire)(
                    descripteurs.as_mut_ptr(),
                    &mut rendus,
                    &mut sens,
                    capacite,
                    condition,
                )
            })?;
        }
        // Plus de résultats que de place : la DLL n'a rien copié.
        if rendus > capacite {
            tampons.clear();
        }
        tampons.truncate(rendus as usize);
        Ok(Lecture {
            resultats: tampons,
            sens,
        })
    }

    fn attendre_evenement(&mut self, delai: Duration) -> Option<Evenement> {
        let file = FILE_EVENEMENTS.lock().ok()?;
        let (mut file, _) = NOUVEL_EVENEMENT
            .wait_timeout_while(file, delai, |f| f.is_empty())
            .ok()?;
        file.pop_front()
    }
}

impl Drop for FdxDll {
    fn drop(&mut self) {
        if self.connecte {
            // SAFETY : aucun argument ; ferme la session ouverte par FDX_Connect.
            unsafe { (self.deconnecter)() };
        }
        // SAFETY : retire le rappel avant que la DLL ne soit déchargée.
        unsafe { (self.enregistrer)(None) };
    }
}
