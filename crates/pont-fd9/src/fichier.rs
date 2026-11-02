//! Ce que le pont lit du fichier DLL sans le charger : empreinte et version.

use std::path::Path;

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
        // `mots` fait 16 octets : i + 3 < 16 pour i ≤ 12.
        let mot = |i: usize| u32::from_le_bytes([mots[i], mots[i + 1], mots[i + 2], mots[i + 3]]);
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
