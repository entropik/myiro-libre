//! FD-9 simulé partagé par les tests : répond comme FD9SDK d'après docs/abi/FD9_*.
//! Il valide la logique du pont, pas l'ABI : seul l'instrument tranche.

#![allow(dead_code)]

use fd9_sys::Appareil;
use pont_fd9::SdkFd9;

/// SDK simulé, à échecs injectables ; note chaque appel.
#[derive(Default)]
pub struct Fd9Simule {
    /// Instruments que `FD9_GetDeviceList` trouve.
    pub appareils: Vec<Appareil>,
    /// Code public rendu par `FD9_GetDeviceList` quand une étape échoue.
    pub code_liste: i32,
    /// Code interne rendu ensuite par `FD9_GetLastError`.
    pub code_interne: i32,
    /// Compteur écrit par la DLL, s'il faut en simuler un faux.
    pub trouves_force: Option<u32>,
    pub version_fichier: Option<[u32; 4]>,
    pub empreinte: Option<String>,
    pub appels: Vec<String>,
}

/// Motif répété 32 fois : empreinte SHA-256 fictive de la DLL simulée.
pub const EMPREINTE_SIMULEE: &str = "5e";

/// FD-9 en réseau à l'adresse fictive 192.168.1.40, identifiant fictif.
pub fn fd9_reseau() -> Appareil {
    let mut appareil = Appareil::reseau("192.168.1.40").unwrap();
    appareil.identifiant.copy_from_slice(b"12345678");
    appareil
}

/// FD-9 branché en USB sur COM4, identifiant fictif.
pub fn fd9_usb() -> Appareil {
    let mut adresse = [0u8; 32];
    adresse[..4].copy_from_slice(b"COM4");
    Appareil {
        code_liaison: 1,
        adresse,
        identifiant: *b"87654321",
    }
}

impl Fd9Simule {
    /// La DLL de FD-S2w (1.3.2.3) et un FD-9 sur le réseau.
    pub fn avec_un_fd9() -> Self {
        Fd9Simule {
            appareils: vec![fd9_reseau()],
            version_fichier: Some([1, 3, 2, 3]),
            empreinte: Some(EMPREINTE_SIMULEE.repeat(32)),
            ..Default::default()
        }
    }

    pub fn appels_dll(&self) -> Vec<&str> {
        self.appels.iter().map(String::as_str).collect()
    }
}

impl SdkFd9 for Fd9Simule {
    fn dernier_code(&mut self) -> i32 {
        self.appels.push("FD9_GetLastError".into());
        self.code_interne
    }

    fn lister(&mut self, tableau: &mut [Appareil], trouves: &mut u32) -> i32 {
        self.appels
            .push(format!("FD9_GetDeviceList {}", tableau.len()));
        // Comme la DLL : capacité de 1 à 100, compteur remis à zéro, tableau
        // effacé, au plus `capacité` entrées.
        if tableau.is_empty() || tableau.len() > 100 {
            return 1001;
        }
        *trouves = 0;
        for (place, appareil) in tableau.iter_mut().zip(&self.appareils) {
            *place = *appareil;
            *trouves += 1;
        }
        if let Some(force) = self.trouves_force {
            *trouves = force;
        }
        // Succès partiel : une étape en échec n'empêche pas de rendre ce qui
        // a été vu (fiche FD9_GetDeviceList).
        if self.code_liste != 0 && *trouves == 0 {
            self.code_liste
        } else {
            0
        }
    }

    fn version_fichier(&self) -> Option<[u32; 4]> {
        self.version_fichier
    }

    fn empreinte(&self) -> Option<String> {
        self.empreinte.clone()
    }
}
