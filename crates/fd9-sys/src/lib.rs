//! Types et liste blanche des exports de `FD9SDK.dll` (FD-9).
//!
//! Tout ce qui est ici vient de l'analyse statique documentée dans `docs/abi/FD9_*`.
//! Aucune fonction de ce crate ne charge la DLL.

/// Seuls exports que le pont FD-9 a le droit de résoudre dans la DLL.
///
/// Un export absent de cette liste ne peut pas être appelé : le pont ne
/// demande jamais son adresse. Les `JIG_*` (maintenance usine et tests de
/// l'appareil) et les `FD9_Set*` n'y figurent pas et n'y entreront qu'avec un
/// contrat établi et l'accord explicite du responsable du projet. On n'ajoute
/// un export qu'avec sa fiche `docs/abi/`.
///
/// FD9SDK n'a pas d'export de version hors `JIG_GetSDKVersion` : le palier
/// Version lit la version dans la ressource du fichier DLL et n'appelle que
/// `FD9_GetLastError` (fiche `docs/abi/FD9_GetLastError.md`).
pub const EXPORTS_AUTORISES: &[&str] = &[
    "FD9_GetLastError",
    "FD9_GetDeviceList",
    // À appeler avant la connexion : refusé une fois connecté.
    "FD9_RegisterDeviceEventHandler",
    "FD9_Connect",
    "FD9_GetSystemInfo",
    "FD9_Disconnect",
];

/// Codes publics renvoyés par les exports (fiche `docs/abi/FD9SDK.md`).
/// `FD9_GetLastError`, lui, rend le code interne d'origine.
pub const CODE_SUCCES: i32 = 0;
pub const CODE_PARAMETRE_INVALIDE: i32 = 1001;
/// Connecté, mais l'instrument demande sa calibration périodique (interne 8011).
pub const CODE_CALIBRATION_PERIODIQUE_DUE: i32 = 1901;
/// Code interne absent de la table de regroupement.
pub const CODE_NON_CLASSE: i32 = 1999;

/// Vrai si `FD9_Connect` a ouvert la session : FD-S2w traite 1901 comme un
/// succès, avec un avertissement.
pub fn connexion_etablie(code: i32) -> bool {
    code == CODE_SUCCES || code == CODE_CALIBRATION_PERIODIQUE_DUE
}

/// Rappel de `FD9_RegisterDeviceEventHandler`, appelé par la DLL en `__cdecl`
/// avec l'état du SDK, le n° de travail, le n° de page et le nombre de plages
/// mesurées. `None` (pointeur nul) désinscrit.
pub type GestionnaireEvenements =
    Option<unsafe extern "C" fn(etat: u32, travail: u32, page: u32, mesurees: u32)>;

/// `tFD9_DeviceData` (44 octets en 32 et 64 bits) : une entrée remplie par
/// `FD9_GetDeviceList` et rendue à `FD9_Connect` (fiche
/// `docs/abi/FD9_GetDeviceList.md`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appareil {
    /// 0 = réseau (TCP), 1 = USB (port série virtuel).
    pub code_liaison: u32,
    /// Adresse IP `a.b.c.d` (ou nom d'hôte) en réseau, `COMn` en USB ;
    /// terminée par zéro quand elle fait moins de 32 octets.
    pub adresse: [u8; 32],
    /// 8 caractères sans zéro final : n° de série USB sans ses 4 premiers
    /// caractères, ou 4 octets de la réponse réseau en hexadécimal.
    /// `FD9_Connect` ne le lit pas.
    pub identifiant: [u8; 8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liaison {
    Reseau,
    Usb,
    /// Code que la DLL refuse à la connexion ; gardé pour le journal.
    Inconnue(u32),
}

fn texte_c(zone: &[u8]) -> String {
    let fin = zone.iter().position(|&o| o == 0).unwrap_or(zone.len());
    String::from_utf8_lossy(&zone[..fin]).into_owned()
}

/// Refus d'un paramètre de connexion avant tout appel à la DLL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErreurParametre {
    Vide,
    /// Nombre de caractères permis, zéro final non compris.
    TropLong {
        maximum: usize,
    },
    /// Zéro au milieu du texte ou caractère hors ASCII.
    CaractereInterdit,
}

/// Texte ASCII non vide, sans zéro, d'au plus `maximum` caractères.
fn verifier_texte(texte: &str, maximum: usize) -> Result<(), ErreurParametre> {
    if texte.is_empty() {
        Err(ErreurParametre::Vide)
    } else if !texte.bytes().all(|o| o.is_ascii() && o != 0) {
        Err(ErreurParametre::CaractereInterdit)
    } else if texte.len() > maximum {
        Err(ErreurParametre::TropLong { maximum })
    } else {
        Ok(())
    }
}

impl Appareil {
    /// Paramètre de connexion réseau du FD-9 : l'adresse IP (ou le nom
    /// d'hôte) suffit, sans passer par la détection. Le SDK joint
    /// l'instrument en TCP sur le port 49152 (fiche `docs/abi/FD9_Connect.md`).
    /// 23 caractères au plus : la DLL n'en recopie que 24 octets, sans zéro
    /// final garanti.
    pub fn reseau(adresse: &str) -> Result<Self, ErreurParametre> {
        verifier_texte(adresse, 23)?;
        let mut zone = [0u8; 32];
        zone[..adresse.len()].copy_from_slice(adresse.as_bytes());
        Ok(Appareil {
            code_liaison: 0,
            adresse: zone,
            identifiant: [0; 8],
        })
    }

    pub fn liaison(&self) -> Liaison {
        match self.code_liaison {
            0 => Liaison::Reseau,
            1 => Liaison::Usb,
            code => Liaison::Inconnue(code),
        }
    }

    pub fn adresse(&self) -> String {
        texte_c(&self.adresse)
    }

    pub fn identifiant(&self) -> String {
        texte_c(&self.identifiant)
    }
}

/// `tFD9_SystemInfo` : ce que `FD9_GetSystemInfo` écrit (100 octets en 32 et
/// 64 bits, aucune adresse dedans). Fiche `docs/abi/FD9_GetSystemInfo.md`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InfosSysteme {
    /// Écrite en dur dans la DLL : `{1, 0x20, 3}` en 1.3.2.3, `{1, 0x1f, 5}`
    /// en 1.3.1.5. À archiver telle quelle.
    pub version_sdk: [u32; 3],
    /// Trois nombres décodés par le SDK depuis la réponse de l'instrument.
    pub version_micrologiciel: [u32; 3],
    /// 4 caractères sans zéro final.
    pub code_produit: [u8; 4],
    /// 8 chiffres hexadécimaux sans zéro final.
    pub numero_serie: [u8; 8],
    pub adresse_mac: [u8; 6],
    pub remplissage: [u8; 2],
    /// Année, mois, jour. Nom tiré du journal du SDK (supposé).
    pub date_etalonnage_usine: [u32; 3],
    /// Année, mois, jour. Nom tiré du journal du SDK (supposé).
    pub date_premiere_mesure: [u32; 3],
    /// Texte constant de la DLL (`KONICA MINOLTA FD-9`), terminé par zéro.
    pub nom_produit: [u8; 32],
}

impl InfosSysteme {
    /// Faux tant que la DLL n'a rien écrit : `FD9_GetSystemInfo` rend 0 sans
    /// toucher au tampon quand aucun instrument n'est connecté.
    pub fn est_rempli(&self) -> bool {
        self.version_sdk[0] != 0
    }

    /// Format `%u.%02u.%04u` du journal du SDK.
    pub fn micrologiciel(&self) -> String {
        let [a, b, c] = self.version_micrologiciel;
        format!("{a}.{b:02}.{c:04}")
    }

    pub fn code_produit(&self) -> String {
        texte_c(&self.code_produit)
    }

    pub fn numero_de_serie(&self) -> String {
        texte_c(&self.numero_serie)
    }

    pub fn mac(&self) -> String {
        self.adresse_mac
            .iter()
            .map(|octet| format!("{octet:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    pub fn produit(&self) -> String {
        texte_c(&self.nom_produit)
    }
}

/// Tampon passé à `FD9_GetSystemInfo`. La DLL écrit 100 octets dans les
/// versions étudiées ; la marge protège la mémoire du pont si une autre version
/// écrivait plus loin (le journal du SDK Mac laisse penser à 104).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TamponInfosSysteme {
    pub infos: InfosSysteme,
    pub marge: [u8; 156],
}

impl TamponInfosSysteme {
    /// Tampon à zéro, à repasser à la DLL avant chaque appel.
    pub fn vide() -> Self {
        TamponInfosSysteme {
            infos: InfosSysteme {
                version_sdk: [0; 3],
                version_micrologiciel: [0; 3],
                code_produit: [0; 4],
                numero_serie: [0; 8],
                adresse_mac: [0; 6],
                remplissage: [0; 2],
                date_etalonnage_usine: [0; 3],
                date_premiere_mesure: [0; 3],
                nom_produit: [0; 32],
            },
            marge: [0; 156],
        }
    }
}

/// Second argument de `FD9_Connect` : nom sous lequel le pont se présente à
/// l'instrument. La DLL en recopie 20 octets ; on garde la place du zéro final.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NomApplication(Vec<u8>);

impl NomApplication {
    pub fn nouveau(nom: &str) -> Result<Self, ErreurParametre> {
        verifier_texte(nom, 19)?;
        let mut octets = nom.as_bytes().to_vec();
        octets.push(0);
        Ok(NomApplication(octets))
    }

    /// Texte terminé par zéro, à passer tel quel (pointeur sur le premier octet).
    pub fn octets(&self) -> &[u8] {
        &self.0
    }
}
