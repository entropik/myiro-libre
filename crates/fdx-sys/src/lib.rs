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

/// Entrée de `FDX_GetDevicePortList` (44 octets, fiche
/// `docs/abi/FDX_GetDevicePortList.md`). Le pont la rend telle quelle à
/// `FDX_Connect` ; les méthodes ne font que la lire.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    pub code_liaison: i32,
    /// Nom du port (33 octets au plus), remplissage, puis n° de série.
    pub opaque: [u8; 40],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liaison {
    Reseau,
    Usb,
    /// Code que la DLL refuserait à la connexion ; gardé pour le journal.
    Inconnue(i32),
}

const LONGUEUR_MAX_NOM: usize = 33;
const POSITION_NUMERO: usize = 0x28 - 4;

impl Port {
    pub fn liaison(&self) -> Liaison {
        match self.code_liaison {
            0 => Liaison::Reseau,
            1 => Liaison::Usb,
            code => Liaison::Inconnue(code),
        }
    }

    /// `COMn`, `/dev/cu.usbmodem…` ou adresse IP.
    pub fn nom(&self) -> String {
        let zone = &self.opaque[..LONGUEUR_MAX_NOM];
        let fin = zone.iter().position(|&o| o == 0).unwrap_or(zone.len());
        String::from_utf8_lossy(&zone[..fin]).into_owned()
    }

    pub fn numero_serie(&self) -> u32 {
        u32::from_le_bytes(
            self.opaque[POSITION_NUMERO..POSITION_NUMERO + 4]
                .try_into()
                .unwrap(),
        )
    }
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

/// Taille du tampon passé à `FDX_GetDeviceInfo`. La structure fait 40 octets
/// dans les versions étudiées ; la marge protège la mémoire du pont si une
/// autre version écrivait plus loin.
pub const TAILLE_TAMPON_INFOS: usize = 256;

/// Nombre d'octets réellement écrits par `FDX_GetDeviceInfo` (versions étudiées).
pub const TAILLE_INFOS: usize = 40;

/// Date initiale d'usine : l'instrument n'a jamais reçu de date de mise en service.
pub const DATE_INITIALE_USINE: u32 = 20190101;

/// Champs confirmés de la réponse de `FDX_GetDeviceInfo`
/// (fiche `docs/abi/FDX_GetDeviceInfo.md`). Les octets 0x10 et 0x14, de sens
/// encore inconnu, ne sont pas interprétés.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfosInstrument {
    /// N° de série, 8 chiffres.
    pub numero: u32,
    /// Format `%u.%02d.%04d`, comme MY-CT1.
    pub micrologiciel: String,
    pub adresse_mac: String,
    /// `ACJ1` ou `9C1D` pour la famille MYIRO-1.
    pub code_produit: String,
    /// AAAAMMJJ ; `None` si l'instrument porte encore la date d'usine.
    pub date_initiale: Option<u32>,
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
        code_produit: String::from_utf8_lossy(&tampon[0x1e..0x22]).into_owned(),
        date_initiale: Some(mot(0x24)).filter(|&date| date != DATE_INITIALE_USINE),
    }
}
