//! Types et liste blanche des exports de `FDXSDK.dll` (MYIRO-1).
//!
//! Tout ce qui est ici vient de l'analyse statique documentée dans `docs/abi/`.
//! Aucune fonction de ce crate ne charge la DLL.

/// Seuls exports que le pont a le droit de résoudre dans la DLL.
///
/// Un export absent de cette liste ne peut pas être appelé : le pont ne
/// demande jamais son adresse. Les `FDX_JIG_*` (maintenance usine) et les
/// `FDX_Set*` (écriture dans l'instrument) n'y figurent pas et n'y entreront
/// qu'avec un contrat établi et l'accord explicite du responsable du projet.
/// On n'ajoute un export qu'avec sa fiche `docs/abi/`.
pub const EXPORTS_AUTORISES: &[&str] = &[
    "FDX_GetSDKVersion",
    "FDX_GetDevicePortList",
    "FDX_Connect",
    "FDX_Disconnect",
    "FDX_GetDeviceInfo",
    "FDX_GetError",
];

/// Entrée de `FDX_GetDevicePortList` : 44 octets dont seul le premier champ
/// est compris. Le pont la rend telle quelle à `FDX_Connect`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    /// 0 ou 1 (confirmé) ; moyen de liaison USB/réseau (supposé).
    pub code_liaison: i32,
    pub opaque: [u8; 40],
}

/// Sortie de `FDX_GetSDKVersion` : trois entiers écrits en dur dans la DLL,
/// `{1, 1, 0}` en 1.0.1 et `{1, 3, 0}` en 1.0.3. À archiver tels quels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Version {
    pub partie0: u32,
    pub partie1: u32,
    pub partie2: u32,
}

/// Taille du tampon passé à `FDX_GetDeviceInfo`. La structure réelle n'est
/// pas connue (au moins 34 octets) : un tampon trop petit corromprait la
/// mémoire du pont, d'où cette marge.
pub const TAILLE_TAMPON_INFOS: usize = 256;

/// Champs lus dans la réponse de `FDX_GetDeviceInfo`. Leur sens est supposé
/// (fiche `docs/abi/FDX_GetDeviceInfo.md`) ; seuls les formats d'affichage
/// de MY-CT1 sont confirmés.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfosInstrument {
    /// Octet 0 : numéro, supposé être le n° de série.
    pub numero: u32,
    /// Octets 4, 8, 12 au format `%u.%02d.%04d` : supposé être le micrologiciel.
    pub micrologiciel: String,
    /// Octets 0x18 à 0x1d : supposé être l'adresse MAC.
    pub adresse_mac: String,
}

pub fn lire_infos_instrument(tampon: &[u8; TAILLE_TAMPON_INFOS]) -> InfosInstrument {
    let mot =
        |position: usize| u32::from_le_bytes(tampon[position..position + 4].try_into().unwrap());
    let mac = &tampon[0x18..0x1e];
    InfosInstrument {
        numero: mot(0),
        micrologiciel: format!("{}.{:02}.{:04}", mot(4), mot(8) as i32, mot(12) as i32),
        adresse_mac: mac
            .iter()
            .map(|octet| format!("{octet:02X}"))
            .collect::<Vec<_>>()
            .join(":"),
    }
}
