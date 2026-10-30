//! Adapter réel : `FD9SDK.dll` chargée dans le processus du pont.
//!
//! Seuls les exports de `fd9_sys::EXPORTS_AUTORISES` sont résolus ; toute
//! autre adresse n'est jamais demandée à la DLL. Convention d'appel :
//! `__stdcall` en x86, convention Windows en x64 (`extern "system"`), fiche
//! `docs/abi/FD9SDK.md`. La DLL de FD-S2w est 32 bits : elle demande le pont
//! compilé pour `i686-pc-windows-msvc`.

use crate::{evenements, Evenement, SdkFd9};
use fd9_sys::{
    Appareil, CleOption, CoordonneesMire, CouleursMire, DonneesQr, EnTeteCouleurs,
    GestionnaireEvenements, ImageFd9, ModeReconnaissance, NomApplication, PointMesure,
    PositionPlage, TamponInfosSysteme, DONNEES_BRUTES, EXPORTS_AUTORISES,
};
use libloading::Library;
use std::ffi::{c_char, c_void};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub use crate::fichier::{empreinte_fichier, version_du_fichier};

/// Exports dont l'adapter demande l'adresse au chargement : toute la liste
/// blanche, pour vérifier que la DLL est complète (fiche
/// `docs/abi/FD9_GetLastError.md`, palier Version). Seuls ceux des paliers
/// déclarés par ce pont reçoivent une forme d'appel.
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
type FnInscrire = unsafe extern "system" fn(GestionnaireEvenements) -> i32;
type FnConnecter = unsafe extern "system" fn(*const Appareil, *const u8) -> i32;
type FnInfos = unsafe extern "system" fn(*mut TamponInfosSysteme) -> i32;
type FnStatut = unsafe extern "system" fn(*mut [u32; 8]) -> i32;
type FnSansArgument = unsafe extern "system" fn() -> i32;
type FnResultatCapture =
    unsafe extern "system" fn(*mut u32, *mut DonneesQr, *mut u32, *mut ImageFd9) -> i32;
type FnReconnaitre = unsafe extern "system" fn(
    *const u32,
    *const CoordonneesMire,
    *const EnTeteCouleurs,
    *const u32,
    *const u32,
) -> i32;
type FnResultatReconnaissance =
    unsafe extern "system" fn(*mut u32, *mut c_void, *mut CoordonneesMire) -> i32;
type FnArreter = unsafe extern "system" fn(*const bool) -> i32;
type FnSortir = unsafe extern "system" fn(*const u32, *const c_void) -> i32;
/// Seul export en `__cdecl` de la liste blanche (fiche
/// FD9_GetPremeasurementError).
type FnErreurPremesure = unsafe extern "C" fn(*mut i32, *mut *const c_char) -> i32;
type FnPointsMesure = unsafe extern "system" fn(*mut PointMesure, *const u32) -> i32;
type FnImageFeuille = unsafe extern "system" fn(*mut ImageFd9, *mut PointMesure, *const u32) -> i32;
type FnDeuxNombres = unsafe extern "system" fn(*const u32, *const u32) -> i32;
type FnDonnees = unsafe extern "system" fn(
    *mut PositionPlage,
    *mut f32,
    *const u32,
    *const PositionPlage,
) -> i32;
type FnCalculer = unsafe extern "system" fn(*mut f32, *const u32, *const f32, *const u32) -> i32;

pub struct Fd9Dll {
    dernier_code: FnDernierCode,
    lister: FnLister,
    inscrire: FnInscrire,
    connecter: FnConnecter,
    infos: FnInfos,
    statut: FnStatut,
    deconnecter: FnSansArgument,
    capturer: FnSansArgument,
    resultat_capture: FnResultatCapture,
    reconnaitre: FnReconnaitre,
    resultat_reconnaissance: FnResultatReconnaissance,
    arreter: FnArreter,
    sortir: FnSortir,
    erreur_premesure: FnErreurPremesure,
    points_mesure: FnPointsMesure,
    image_feuille: FnImageFeuille,
    mesurer: FnDeuxNombres,
    regler: FnDeuxNombres,
    donnees: FnDonnees,
    calculer: FnCalculer,
    /// Événements du rappel, tant qu'il est inscrit.
    reception: Option<Receiver<Evenement>>,
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
            inscrire: resoudre(&bibliotheque, "FD9_RegisterDeviceEventHandler")?,
            connecter: resoudre(&bibliotheque, "FD9_Connect")?,
            infos: resoudre(&bibliotheque, "FD9_GetSystemInfo")?,
            statut: resoudre(&bibliotheque, "FD9_GetStatus")?,
            deconnecter: resoudre(&bibliotheque, "FD9_Disconnect")?,
            capturer: resoudre(&bibliotheque, "FD9_StartCapture")?,
            resultat_capture: resoudre(&bibliotheque, "FD9_GetCaptureResult")?,
            reconnaitre: resoudre(&bibliotheque, "FD9_StartPatchRecognition")?,
            resultat_reconnaissance: resoudre(&bibliotheque, "FD9_GetPatchRecognitionResult2")?,
            arreter: resoudre(&bibliotheque, "FD9_StopMeasurement")?,
            sortir: resoudre(&bibliotheque, "FD9_ExitMeasurement")?,
            erreur_premesure: resoudre(&bibliotheque, "FD9_GetPremeasurementError")?,
            points_mesure: resoudre(&bibliotheque, "FD9_GetMeasurePoint")?,
            image_feuille: resoudre(&bibliotheque, "FD9_GetChartImage")?,
            mesurer: resoudre(&bibliotheque, "FD9_StartMeasurement2")?,
            regler: resoudre(&bibliotheque, "FD9_SetOption")?,
            donnees: resoudre(&bibliotheque, "FD9_GetData")?,
            calculer: resoudre(&bibliotheque, "FD9_CalculateData")?,
            reception: None,
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

    fn inscrire_rappel(&mut self, actif: bool) -> i32 {
        if actif {
            // Le canal s'ouvre avant l'inscription : aucun événement perdu.
            self.reception = Some(evenements::ouvrir());
            let rappel: GestionnaireEvenements = Some(evenements::rappel);
            // SAFETY : fonction statique `extern "C"`, valide jusqu'au
            // déchargement ; la DLL en garde l'adresse jusqu'à la
            // désinscription (fiche FD9_RegisterDeviceEventHandler).
            let code = unsafe { (self.inscrire)(rappel) };
            if code != 0 {
                evenements::fermer();
                self.reception = None;
            }
            code
        } else {
            // SAFETY : le pointeur nul désinscrit.
            let code = unsafe { (self.inscrire)(None) };
            if code == 0 {
                evenements::fermer();
                self.reception = None;
            }
            code
        }
    }

    fn connecter(&mut self, appareil: &Appareil, nom: &NomApplication) -> i32 {
        // SAFETY : entrée `#[repr(C)]` de 44 octets et texte terminé par zéro,
        // lus pendant l'appel seulement (fiche FD9_Connect).
        unsafe { (self.connecter)(appareil, nom.octets().as_ptr()) }
    }

    fn infos(&mut self, tampon: &mut TamponInfosSysteme) -> i32 {
        // SAFETY : la DLL écrit 100 octets ; le tampon en a 256 (fiche
        // FD9_GetSystemInfo).
        unsafe { (self.infos)(tampon) }
    }

    fn statut(&mut self, tampon: &mut [u32; 8]) -> i32 {
        // SAFETY : 32 octets écrits pendant l'appel (fiche FD9_GetStatus).
        unsafe { (self.statut)(tampon) }
    }

    fn deconnecter(&mut self) -> i32 {
        // SAFETY : aucun argument (fiche FD9_Disconnect).
        unsafe { (self.deconnecter)() }
    }

    fn evenements(&mut self, attente: Duration) -> Vec<Evenement> {
        let Some(reception) = &self.reception else {
            std::thread::sleep(attente);
            return Vec::new();
        };
        let mut recus: Vec<Evenement> = reception.try_iter().collect();
        if recus.is_empty() && !attente.is_zero() {
            if let Ok(premier) = reception.recv_timeout(attente) {
                recus.push(premier);
                recus.extend(reception.try_iter());
            }
        }
        recus
    }

    fn rappels_orphelins(&self) -> u64 {
        evenements::orphelins()
    }

    fn capturer(&mut self) -> i32 {
        // SAFETY : aucun argument (fiche FD9_StartCapture).
        unsafe { (self.capturer)() }
    }

    fn resultat_capture(
        &mut self,
        qr_resultat: &mut u32,
        qr: &mut DonneesQr,
        taille: &mut u32,
        image: Option<&mut [u8]>,
    ) -> i32 {
        let Some(tampon) = image else {
            // SAFETY : trois sorties valides ; image nulle : la DLL n'écrit
            // que la taille (fiche FD9_GetCaptureResult).
            return unsafe {
                (self.resultat_capture)(qr_resultat, qr, taille, std::ptr::null_mut())
            };
        };
        // La DLL se fie à `*taille` : un tampon plus petit est refusé ici.
        if tampon.len() < *taille as usize {
            return fd9_sys::CODE_PARAMETRE_INVALIDE;
        }
        let mut image = ImageFd9 {
            largeur: 0,
            hauteur: 0,
            bits: 0,
            donnees: tampon.as_mut_ptr(),
        };
        // SAFETY : QR de 1 172 octets, tampon d'au moins `*taille` octets,
        // aucune adresse gardée après l'appel (fiche FD9_GetCaptureResult).
        unsafe { (self.resultat_capture)(qr_resultat, qr, taille, &mut image) }
    }

    fn reconnaitre(
        &mut self,
        mode: ModeReconnaissance,
        coordonnees: &CoordonneesMire,
        couleurs: Option<&CouleursMire>,
    ) -> i32 {
        // Sans image modèle, COMBINE fait lire à la DLL les couleurs dans son
        // fil de reconnaissance sans tester le pointeur (`fd9-x86`
        // 0x100a14f1, essai E3) : sans couleurs, l'appel n'est pas fait.
        if mode.lit_les_couleurs_sans_modele()
            && coordonnees.modele_present == 0
            && couleurs.is_none()
        {
            return fd9_sys::CODE_PARAMETRE_INVALIDE;
        }
        let tete = couleurs.map(CouleursMire::en_tete);
        let tete_ptr = tete
            .as_ref()
            .map_or(std::ptr::null(), |t| t as *const EnTeteCouleurs);
        let mode = mode as u32;
        let (rangees, colonnes) = (0u32, 0u32);
        // SAFETY : mode, rangées et colonnes jamais nuls (lus sans test par
        // la DLL), coordonnées de 0xd70 octets (0xd78 en 64 bits) ; en-tête
        // des couleurs et ses entrées vivants pendant l'appel (`couleurs`
        // emprunté) : tout est recopié avant le retour (fiche
        // FD9_StartPatchRecognition, 0x100ad370).
        unsafe { (self.reconnaitre)(&mode, coordonnees, tete_ptr, &rangees, &colonnes) }
    }

    fn resultat_reconnaissance(
        &mut self,
        fiabilite: &mut u32,
        coordonnees: &mut CoordonneesMire,
    ) -> i32 {
        // SAFETY : disposition nulle admise ; coordonnées à la taille de la
        // structure, écrites pendant l'appel (fiche
        // FD9_GetPatchRecognitionResult2).
        unsafe { (self.resultat_reconnaissance)(fiabilite, std::ptr::null_mut(), coordonnees) }
    }

    fn arreter(&mut self, feuille_suivante: bool) -> i32 {
        // SAFETY : un octet lu pendant l'appel (fiche FD9_StopMeasurement).
        unsafe { (self.arreter)(&feuille_suivante) }
    }

    fn sortir(&mut self) -> i32 {
        let afficher = 0u32;
        // SAFETY : un nombre lu pendant l'appel ; image nulle avec 0 (fiche
        // FD9_ExitMeasurement).
        unsafe { (self.sortir)(&afficher, std::ptr::null()) }
    }

    fn erreur_premesure(&mut self, erreur_systeme: &mut i32) -> i32 {
        // SAFETY : `__cdecl` ; texte nul admis, rien de la DLL n'est gardé
        // (fiche FD9_GetPremeasurementError).
        unsafe { (self.erreur_premesure)(erreur_systeme, std::ptr::null_mut()) }
    }

    fn points_mesure(&mut self, points: &mut [PointMesure]) -> i32 {
        // Un nombre nul serait refusé (1001) ; le pont n'en demande jamais.
        let nombre = points.len() as u32;
        // SAFETY : `nombre` points de 20 octets, remplis pendant l'appel
        // (fiche FD9_GetMeasurePoint).
        unsafe { (self.points_mesure)(points.as_mut_ptr(), &nombre) }
    }

    fn image_feuille(&mut self, largeur: &mut u32, hauteur: &mut u32, gris: &mut [u8]) -> i32 {
        // La DLL se fie à `largeur × hauteur` : une place plus grande que le
        // tampon est refusée ici.
        if u64::from(*largeur) * u64::from(*hauteur) > gris.len() as u64 {
            return fd9_sys::CODE_PARAMETRE_INVALIDE;
        }
        // 8 bits toujours : sous 8, la DLL diviserait par zéro (fiche
        // FD9_GetChartImage, 0x100a0816).
        let mut image = ImageFd9 {
            largeur: *largeur,
            hauteur: *hauteur,
            bits: 8,
            donnees: gris.as_mut_ptr(),
        };
        let nombre = 0u32;
        // SAFETY : image de 16 octets (24 en x64) dont les données ont au
        // moins `largeur × hauteur` octets, écrites pendant l'appel ; points
        // nuls avec un nombre nul (lu sans test si le journal du SDK est
        // actif) ; aucune adresse gardée (fiche FD9_GetChartImage).
        let code = unsafe { (self.image_feuille)(&mut image, std::ptr::null_mut(), &nombre) };
        *largeur = image.largeur;
        *hauteur = image.hauteur;
        code
    }

    fn mesurer(&mut self) -> i32 {
        let (mode, avis) = (1u32, 1u32);
        // SAFETY : deux nombres lus pendant l'appel, jamais nuls (fiche
        // FD9_StartMeasurement2).
        unsafe { (self.mesurer)(&mode, &avis) }
    }

    fn regler(&mut self, cle: CleOption, valeur: u32) -> i32 {
        let cle = cle as u32;
        // SAFETY : deux nombres lus pendant l'appel, jamais nuls ; aucun
        // échange avec l'instrument (fiche FD9_SetOption).
        unsafe { (self.regler)(&cle, &valeur) }
    }

    fn donnees(
        &mut self,
        rendue: &mut PositionPlage,
        donnees: &mut [f32; DONNEES_BRUTES],
        demandee: &PositionPlage,
    ) -> i32 {
        let longueur = DONNEES_BRUTES as u32;
        // SAFETY : la DLL écrit au plus 152 flottants et une position de
        // 12 octets, pendant l'appel (fiche FD9_GetData).
        unsafe { (self.donnees)(rendue, donnees.as_mut_ptr(), &longueur, demandee) }
    }

    fn calculer(&mut self, sortie: &mut [f32], brutes: &[f32; DONNEES_BRUTES]) -> i32 {
        let longueur_sortie = sortie.len() as u32;
        let longueur_entree = DONNEES_BRUTES as u32;
        // SAFETY : sortie de `longueur_sortie` flottants (la DLL refuse 1005
        // une sortie trop courte pour le type), entrée de 152 recopiée avant
        // le calcul (fiche FD9_CalculateData).
        unsafe {
            (self.calculer)(
                sortie.as_mut_ptr(),
                &longueur_sortie,
                brutes.as_ptr(),
                &longueur_entree,
            )
        }
    }
}
