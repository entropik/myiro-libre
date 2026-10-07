//! Formes binaires établies dans docs/abi/ : une erreur ici corromprait la mémoire du pont.

use fdx_sys::{lire_infos_instrument, Liaison, Port, Version, TAILLE_TAMPON_INFOS};
use std::mem::size_of;

#[test]
fn une_entree_de_la_liste_des_ports_fait_44_octets() {
    assert_eq!(size_of::<Port>(), 44);
}

#[test]
fn la_version_du_sdk_fait_12_octets() {
    assert_eq!(size_of::<Version>(), 12);
}

// La structure fait 40 octets ; on prévoit large (fiche FDX_GetDeviceInfo).
// Vérifié à la compilation : un tampon trop petit empêche de construire les tests.
const _: () = assert!(TAILLE_TAMPON_INFOS >= 256);

fn tampon_exemple() -> [u8; TAILLE_TAMPON_INFOS] {
    let mut t = [0u8; TAILLE_TAMPON_INFOS];
    t[0..4].copy_from_slice(&12345678u32.to_le_bytes());
    t[4..8].copy_from_slice(&1u32.to_le_bytes());
    t[8..12].copy_from_slice(&2u32.to_le_bytes());
    t[12..16].copy_from_slice(&3u32.to_le_bytes());
    t[0x18..0x1e].copy_from_slice(&[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]);
    t[0x1e..0x22].copy_from_slice(b"ACJ1");
    t[0x24..0x28].copy_from_slice(&20231015u32.to_le_bytes());
    t
}

#[test]
fn le_code_produit_est_lu_a_l_octet_0x1e() {
    assert_eq!(
        lire_infos_instrument(&tampon_exemple()).code_produit,
        "ACJ1"
    );
}

#[test]
fn la_date_initiale_est_lue_a_l_octet_0x24() {
    assert_eq!(
        lire_infos_instrument(&tampon_exemple()).date_initiale,
        Some(20231015)
    );
}

#[test]
fn la_date_d_usine_veut_dire_jamais_posee() {
    let mut t = tampon_exemple();
    t[0x24..0x28].copy_from_slice(&20190101u32.to_le_bytes());
    assert_eq!(lire_infos_instrument(&t).date_initiale, None);
}

fn port(liaison: i32, nom: &[u8], numero: u32) -> Port {
    let mut opaque = [0u8; 40];
    opaque[..nom.len()].copy_from_slice(nom);
    opaque[36..40].copy_from_slice(&numero.to_le_bytes());
    Port {
        code_liaison: liaison,
        opaque,
    }
}

#[test]
fn une_entree_usb_donne_son_port_et_son_numero() {
    let p = port(1, b"COM3", 12345678);
    assert_eq!(p.liaison(), Liaison::Usb);
    assert_eq!(p.nom(), "COM3");
    assert_eq!(p.numero_serie(), 12345678);
}

#[test]
fn une_entree_reseau_donne_son_adresse() {
    let p = port(0, b"192.168.1.40", 12345678);
    assert_eq!(p.liaison(), Liaison::Reseau);
    assert_eq!(p.nom(), "192.168.1.40");
}

#[test]
fn un_code_de_liaison_inattendu_reste_visible() {
    assert_eq!(port(7, b"", 0).liaison(), Liaison::Inconnue(7));
}

#[test]
fn un_nom_de_port_sans_zero_final_ne_deborde_pas_sur_le_numero() {
    let p = port(1, &[b'A'; 36], 0x31313131);
    assert_eq!(p.nom(), "A".repeat(33));
}

#[test]
fn le_numero_est_lu_a_l_octet_0() {
    assert_eq!(lire_infos_instrument(&tampon_exemple()).numero, 12345678);
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

#[test]
fn la_condition_de_mesure_fait_8_octets() {
    assert_eq!(size_of::<fdx_sys::ConditionMesure>(), 8);
}

#[test]
fn la_condition_de_calcul_fait_0x31c_octets() {
    assert_eq!(size_of::<fdx_sys::ConditionCalcul>(), 0x31c);
}

#[test]
fn un_descripteur_de_resultat_fait_8_octets_en_32_bits_et_16_en_64() {
    let attendu = if cfg!(target_pointer_width = "32") {
        8
    } else {
        16
    };
    assert_eq!(size_of::<fdx_sys::DescripteurResultat>(), attendu);
}

#[test]
fn la_condition_de_calcul_place_ses_champs_comme_la_dll() {
    let c = fdx_sys::ConditionCalcul::spectre(fdx_sys::CONDITION_M2);
    let octets: &[u8] =
        unsafe { std::slice::from_raw_parts(&c as *const _ as *const u8, size_of_val(&c)) };
    assert_eq!(
        octets[0..4],
        2i32.to_le_bytes(),
        "Illuminant = condition de mesure"
    );
    assert_eq!(octets[4..8], 2i32.to_le_bytes(), "ObsIlluminant = D50");
    assert_eq!(octets[8..12], 0i32.to_le_bytes(), "Observer = 2°");
    assert_eq!(octets[16..20], 10i32.to_le_bytes(), "DataType = spectre");
    assert!(octets[20..].iter().all(|&o| o == 0), "le reste est à zéro");
}
