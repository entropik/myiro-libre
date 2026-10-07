//! Formes binaires établies dans docs/abi/ : une erreur ici corromprait la mémoire du pont.

use fdx_sys::{lire_infos_instrument, Port, Version, TAILLE_TAMPON_INFOS};
use std::mem::size_of;

#[test]
fn une_entree_de_la_liste_des_ports_fait_44_octets() {
    assert_eq!(size_of::<Port>(), 44);
}

#[test]
fn la_version_du_sdk_fait_12_octets() {
    assert_eq!(size_of::<Version>(), 12);
}

// La DLL écrit au moins jusqu'à l'octet 0x21 ; on prévoit large (fiche FDX_GetDeviceInfo).
// Vérifié à la compilation : un tampon trop petit empêche de construire les tests.
const _: () = assert!(TAILLE_TAMPON_INFOS >= 256);

fn tampon_exemple() -> [u8; TAILLE_TAMPON_INFOS] {
    let mut t = [0u8; TAILLE_TAMPON_INFOS];
    t[0..4].copy_from_slice(&10002006u32.to_le_bytes());
    t[4..8].copy_from_slice(&1u32.to_le_bytes());
    t[8..12].copy_from_slice(&2u32.to_le_bytes());
    t[12..16].copy_from_slice(&3u32.to_le_bytes());
    t[0x18..0x1e].copy_from_slice(&[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]);
    t
}

#[test]
fn le_numero_est_lu_a_l_octet_0() {
    assert_eq!(lire_infos_instrument(&tampon_exemple()).numero, 10002006);
}

#[test]
fn le_micrologiciel_est_affiche_comme_my_ct1() {
    // Format "%u.%02d.%04d" relevé dans MY-CT1.
    assert_eq!(
        lire_infos_instrument(&tampon_exemple()).micrologiciel,
        "1.02.0003"
    );
}

#[test]
fn l_adresse_mac_est_affiche_comme_my_ct1() {
    assert_eq!(
        lire_infos_instrument(&tampon_exemple()).adresse_mac,
        "00:1A:2B:3C:4D:5E"
    );
}
