//! Rapport de plantage, pour les essais sur l'instrument.
//!
//! À l'essai E3, la DLL a arrêté le pont sans un mot (lecture à l'adresse 0
//! dans son fil de reconnaissance). [`installer`] pose un filtre Windows des
//! exceptions non rattrapées : il écrit sur la sortie d'erreur le code de
//! l'exception, le module et le décalage de l'adresse fautive (à ajouter à
//! la base 0x10000000 de `FD9SDK.dll` x86 pour la retrouver dans les
//! désassemblages), l'accès fautif et le fil ; puis, sur demande, un vidage
//! mémoire (`--vidage`) ; puis il arrête le processus avec le code de
//! l'exception, sans fenêtre de Windows. Avec `--trace`, un second filtre
//! note aussi chaque exception grave au premier passage, même si un autre
//! filtre (celui de .NET, chargé par le QR) prend la place du premier.
//!
//! Le filtre n'alloue rien et n'utilise pas la sortie d'erreur de Rust : il
//! écrit dans un tampon fixe, puis directement à Windows. Le vidage contient
//! la mémoire du pont (adresse et identifiant de l'instrument compris) : il
//! reste sur le poste, jamais dans le dépôt.

use std::fmt::{self, Write};

/// Ce que le filtre a lu de l'exception.
pub struct Rapport<'a> {
    /// Vrai pour le filtre du premier passage (`--trace`) : l'exception sera
    /// peut-être rattrapée.
    pub premier_passage: bool,
    pub code: u32,
    /// Adresse de l'instruction fautive.
    pub adresse: usize,
    /// Chemin du module qui contient l'adresse (UTF-16) et sa base.
    pub module: Option<(&'a [u16], usize)>,
    /// Violation d'accès : sorte (0 lecture, 1 écriture, 8 exécution) et
    /// adresse visée.
    pub acces: Option<(usize, usize)>,
    pub fil: u32,
}

/// Nom en clair des codes d'exception les plus probables.
pub fn nom_exception(code: u32) -> &'static str {
    match code {
        0xc000_0005 => "accès mémoire refusé",
        0xc000_0006 => "page mémoire illisible",
        0xc000_001d => "instruction illégale",
        0xc000_0094 => "division entière par zéro",
        0xc000_0096 => "instruction privilégiée",
        0xc000_00fd => "pile épuisée",
        0xc000_0409 => "arrêt immédiat demandé par le programme",
        0xc000_0374 => "tas corrompu",
        0xe06d_7363 => "exception C++ non rattrapée",
        0xe043_4352 => "exception .NET non rattrapée",
        _ => "exception inconnue",
    }
}

/// Une ligne de rapport, terminée par un saut de ligne.
pub fn ecrire_rapport(w: &mut impl Write, r: &Rapport) -> fmt::Result {
    if r.premier_passage {
        w.write_str("pont-fd9 : exception (premier passage) : ")?;
    } else {
        w.write_str("pont-fd9 : plantage : ")?;
    }
    write!(
        w,
        "exception {:#010x} ({}) à ",
        r.code,
        nom_exception(r.code)
    )?;
    match r.module {
        Some((chemin, base)) => {
            // Le nom du fichier seul, sans son dossier.
            let debut = chemin
                .iter()
                .rposition(|&c| c == u16::from(b'\\') || c == u16::from(b'/'))
                .map_or(0, |i| i + 1);
            for c in char::decode_utf16(chemin[debut..].iter().copied()) {
                w.write_char(c.unwrap_or(char::REPLACEMENT_CHARACTER))?;
            }
            write!(
                w,
                "+{:#x} (adresse {:#x})",
                r.adresse.wrapping_sub(base),
                r.adresse
            )?;
        }
        None => write!(w, "{:#x} (module inconnu)", r.adresse)?,
    }
    if let Some((sorte, cible)) = r.acces {
        let sorte = match sorte {
            0 => "lecture",
            1 => "écriture",
            8 => "exécution",
            _ => "accès",
        };
        write!(w, ", {sorte} à l'adresse {cible:#x}")?;
    }
    writeln!(w, ", fil {}", r.fil)
}

/// Tampon fixe pour écrire sans allouer ; ce qui dépasse est perdu.
struct Tampon {
    octets: [u8; 1024],
    longueur: usize,
}

impl Write for Tampon {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let reste = self.octets.len() - self.longueur;
        let n = s.len().min(reste);
        self.octets[self.longueur..self.longueur + n].copy_from_slice(&s.as_bytes()[..n]);
        self.longueur += n;
        Ok(())
    }
}

#[cfg(windows)]
pub use windows::{fil_courant, installer};

#[cfg(not(windows))]
pub fn fil_courant() -> u32 {
    0
}

#[cfg(not(windows))]
pub fn installer(_vidage: Option<&std::path::Path>, _premier_passage: bool) {}

#[cfg(windows)]
mod windows {
    use super::{ecrire_rapport, Rapport, Tampon};
    use std::ffi::c_void;
    use std::fmt::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::OnceLock;

    #[repr(C)]
    struct EnregistrementException {
        code: u32,
        drapeaux: u32,
        suivant: *mut EnregistrementException,
        adresse: *mut c_void,
        nombre: u32,
        infos: [usize; 15],
    }

    #[repr(C)]
    struct PointeursException {
        enregistrement: *mut EnregistrementException,
        contexte: *mut c_void,
    }

    /// `MINIDUMP_EXCEPTION_INFORMATION`, aligné sur 4 dans `dbghelp.h`.
    #[repr(C, packed(4))]
    struct InfoVidage {
        fil: u32,
        pointeurs: *const PointeursException,
        pointeurs_du_client: i32,
    }

    type Filtre = unsafe extern "system" fn(*const PointeursException) -> i32;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetUnhandledExceptionFilter(filtre: Option<Filtre>) -> Option<Filtre>;
        fn AddVectoredExceptionHandler(premier: u32, filtre: Filtre) -> *mut c_void;
        fn GetCurrentThreadId() -> u32;
        fn GetStdHandle(numero: u32) -> *mut c_void;
        fn WriteFile(
            fichier: *mut c_void,
            octets: *const u8,
            nombre: u32,
            ecrits: *mut u32,
            recouvrement: *mut c_void,
        ) -> i32;
        fn GetModuleHandleExW(drapeaux: u32, adresse: *const u16, module: *mut *mut c_void) -> i32;
        fn GetModuleFileNameW(module: *mut c_void, nom: *mut u16, taille: u32) -> u32;
        fn CreateFileW(
            nom: *const u16,
            acces: u32,
            partage: u32,
            securite: *const c_void,
            creation: u32,
            attributs: u32,
            modele: *mut c_void,
        ) -> *mut c_void;
        fn CloseHandle(objet: *mut c_void) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
        fn GetCurrentProcessId() -> u32;
    }

    #[link(name = "dbghelp")]
    extern "system" {
        fn MiniDumpWriteDump(
            processus: *mut c_void,
            identifiant: u32,
            fichier: *mut c_void,
            sorte: u32,
            exception: *const InfoVidage,
            flux: *const c_void,
            rappel: *const c_void,
        ) -> i32;
    }

    const SORTIE_ERREUR: u32 = -12i32 as u32;
    const DEPUIS_ADRESSE: u32 = 0x4;
    const SANS_COMPTE: u32 = 0x2;
    const ECRITURE: u32 = 0x4000_0000;
    const CREER_TOUJOURS: u32 = 2;
    const FICHIER_NORMAL: u32 = 0x80;
    /// `MiniDumpWithDataSegs | MiniDumpWithIndirectlyReferencedMemory` :
    /// piles, variables globales des modules, mémoire désignée par les piles.
    const SORTE_VIDAGE: u32 = 0x1 | 0x40;
    /// `EXCEPTION_EXECUTE_HANDLER` : arrête le processus avec le code de
    /// l'exception, sans fenêtre.
    const ARRETER: i32 = 1;
    const CONTINUER: i32 = 0;
    /// Lignes du premier passage, au plus : une boucle d'exceptions ne noie
    /// pas le journal.
    const PREMIERS_PASSAGES_MAX: u32 = 20;

    static VIDAGE: OnceLock<Vec<u16>> = OnceLock::new();
    static DEJA: AtomicBool = AtomicBool::new(false);
    static PREMIERS_PASSAGES: AtomicU32 = AtomicU32::new(0);

    /// N° du fil Windows courant, comme dans le journal du SDK.
    pub fn fil_courant() -> u32 {
        // SAFETY : aucun argument.
        unsafe { GetCurrentThreadId() }
    }

    /// Pose le filtre des exceptions non rattrapées ; `vidage` : fichier du
    /// vidage mémoire à écrire au plantage ; `premier_passage` : note aussi
    /// les exceptions graves dès qu'elles sont levées.
    pub fn installer(vidage: Option<&Path>, premier_passage: bool) {
        if let Some(chemin) = vidage {
            let chemin = crate::dll::chemin_pour_chargement(chemin);
            let _ = VIDAGE.set(chemin.as_os_str().encode_wide().chain([0]).collect());
        }
        // SAFETY : fonctions `extern "system"` statiques, valides jusqu'à la
        // fin du processus.
        unsafe {
            SetUnhandledExceptionFilter(Some(filtre_final));
            if premier_passage {
                AddVectoredExceptionHandler(1, filtre_premier_passage);
            }
        }
    }

    fn ecrire(octets: &[u8]) {
        let mut ecrits = 0u32;
        // SAFETY : tampon valide pendant l'appel ; la poignée est celle de la
        // sortie d'erreur (nulle ou invalide : WriteFile échoue, sans effet).
        unsafe {
            WriteFile(
                GetStdHandle(SORTIE_ERREUR),
                octets.as_ptr(),
                octets.len() as u32,
                &mut ecrits,
                std::ptr::null_mut(),
            );
        }
    }

    /// Lit l'exception et écrit son rapport.
    ///
    /// # Safety
    ///
    /// `pointeurs` vient de Windows : valable pendant le filtre.
    unsafe fn rapporter(pointeurs: *const PointeursException, premier_passage: bool) {
        let Some(enregistrement) = pointeurs.as_ref().and_then(|p| p.enregistrement.as_ref())
        else {
            return;
        };
        let adresse = enregistrement.adresse as usize;
        let mut module = std::ptr::null_mut();
        let mut nom = [0u16; 260];
        let module_lu = if GetModuleHandleExW(
            DEPUIS_ADRESSE | SANS_COMPTE,
            enregistrement.adresse as *const u16,
            &mut module,
        ) != 0
        {
            let n = GetModuleFileNameW(module, nom.as_mut_ptr(), nom.len() as u32) as usize;
            Some((&nom[..n.min(nom.len())], module as usize))
        } else {
            None
        };
        let acces = (matches!(enregistrement.code, 0xc000_0005 | 0xc000_0006)
            && enregistrement.nombre >= 2)
            .then(|| (enregistrement.infos[0], enregistrement.infos[1]));
        let rapport = Rapport {
            premier_passage,
            code: enregistrement.code,
            adresse,
            module: module_lu,
            acces,
            fil: GetCurrentThreadId(),
        };
        let mut tampon = Tampon {
            octets: [0; 1024],
            longueur: 0,
        };
        let _ = ecrire_rapport(&mut tampon, &rapport);
        ecrire(&tampon.octets[..tampon.longueur]);
    }

    unsafe fn ecrire_vidage(pointeurs: *const PointeursException) {
        let Some(chemin) = VIDAGE.get() else {
            return;
        };
        let fichier = CreateFileW(
            chemin.as_ptr(),
            ECRITURE,
            0,
            std::ptr::null(),
            CREER_TOUJOURS,
            FICHIER_NORMAL,
            std::ptr::null_mut(),
        );
        if fichier as isize == -1 {
            ecrire("pont-fd9 : vidage mémoire impossible : fichier non créé\n".as_bytes());
            return;
        }
        let info = InfoVidage {
            fil: GetCurrentThreadId(),
            pointeurs,
            pointeurs_du_client: 0,
        };
        let fait = MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            fichier,
            SORTE_VIDAGE,
            &info,
            std::ptr::null(),
            std::ptr::null(),
        );
        CloseHandle(fichier);
        let mut tampon = Tampon {
            octets: [0; 1024],
            longueur: 0,
        };
        let _ = if fait != 0 {
            tampon.write_str("pont-fd9 : vidage mémoire écrit : ")
        } else {
            tampon.write_str("pont-fd9 : vidage mémoire incomplet : ")
        };
        for c in char::decode_utf16(chemin.iter().copied().take_while(|&c| c != 0)) {
            let _ = tampon.write_char(c.unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        let _ = tampon.write_char('\n');
        ecrire(&tampon.octets[..tampon.longueur]);
    }

    unsafe extern "system" fn filtre_final(pointeurs: *const PointeursException) -> i32 {
        // Une seule fois : un plantage dans le filtre n'y revient pas.
        if !DEJA.swap(true, Ordering::SeqCst) {
            rapporter(pointeurs, false);
            ecrire_vidage(pointeurs);
        }
        ARRETER
    }

    unsafe extern "system" fn filtre_premier_passage(pointeurs: *const PointeursException) -> i32 {
        let grave = pointeurs
            .as_ref()
            .and_then(|p| p.enregistrement.as_ref())
            .is_some_and(|e| {
                matches!(
                    e.code,
                    0xc000_0005
                        | 0xc000_0006
                        | 0xc000_001d
                        | 0xc000_0094
                        | 0xc000_0096
                        | 0xc000_00fd
                )
            });
        if grave && PREMIERS_PASSAGES.fetch_add(1, Ordering::SeqCst) < PREMIERS_PASSAGES_MAX {
            rapporter(pointeurs, true);
        }
        CONTINUER
    }
}
