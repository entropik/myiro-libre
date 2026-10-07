//! Formes binaires établies dans docs/abi/FD9_* : une erreur ici corromprait la mémoire du pont.
//! Ces tests tournent en 64 bits (`cargo test`) et en 32 bits
//! (`cargo test --target i686-pc-windows-msvc`).

use fd9_sys::{
    connexion_etablie, Appareil, ErreurParametre, GestionnaireEvenements, InfosSysteme, Liaison,
    NomApplication, TamponInfosSysteme,
};
use std::mem::{offset_of, size_of};

#[test]
fn une_entree_de_la_liste_des_appareils_fait_44_octets() {
    assert_eq!(size_of::<Appareil>(), 44);
    assert_eq!(offset_of!(Appareil, code_liaison), 0);
    assert_eq!(offset_of!(Appareil, adresse), 4);
    assert_eq!(offset_of!(Appareil, identifiant), 0x24);
}

fn appareil(code: u32, adresse: &[u8], identifiant: &[u8; 8]) -> Appareil {
    let mut a = [0u8; 32];
    a[..adresse.len()].copy_from_slice(adresse);
    Appareil {
        code_liaison: code,
        adresse: a,
        identifiant: *identifiant,
    }
}

#[test]
fn une_entree_reseau_donne_son_adresse_ip_et_son_identifiant() {
    let a = appareil(0, b"192.168.1.40", b"0a1b2c3d");
    assert_eq!(a.liaison(), Liaison::Reseau);
    assert_eq!(a.adresse(), "192.168.1.40");
    assert_eq!(a.identifiant(), "0a1b2c3d");
}

#[test]
fn une_entree_usb_donne_son_port_com() {
    let a = appareil(1, b"COM4", b"12345678");
    assert_eq!(a.liaison(), Liaison::Usb);
    assert_eq!(a.adresse(), "COM4");
    assert_eq!(a.identifiant(), "12345678");
}

#[test]
fn un_code_de_liaison_inattendu_reste_visible() {
    assert_eq!(
        appareil(7, b"", b"\0\0\0\0\0\0\0\0").liaison(),
        Liaison::Inconnue(7)
    );
}

#[test]
fn une_connexion_reseau_directe_se_prepare_sans_detection() {
    let a = Appareil::reseau("192.168.1.40").unwrap();
    assert_eq!(a.code_liaison, 0);
    assert_eq!(a.adresse(), "192.168.1.40");
    assert_eq!(a.identifiant, [0; 8]);
}

#[test]
fn une_adresse_reseau_garde_toujours_son_zero_final() {
    // FD9_Connect mesure l'adresse avec strlen : 31 caractères au plus.
    assert!(Appareil::reseau(&"a".repeat(31)).is_ok());
    assert_eq!(
        Appareil::reseau(&"a".repeat(32)),
        Err(ErreurParametre::TropLong { maximum: 31 })
    );
}

#[test]
fn une_adresse_reseau_vide_ou_avec_un_zero_est_refusee() {
    assert_eq!(Appareil::reseau(""), Err(ErreurParametre::Vide));
    assert_eq!(
        Appareil::reseau("192.168\0.1.40"),
        Err(ErreurParametre::CaractereInterdit)
    );
}

#[test]
fn le_nom_d_application_tient_en_20_octets_avec_son_zero_final() {
    let nom = NomApplication::nouveau("myiro-libre").unwrap();
    assert_eq!(nom.octets(), b"myiro-libre\0");
    // La DLL n'en garde que 20 octets : on refuse au-delà de 19 caractères
    // pour que l'instrument reçoive exactement le nom choisi.
    assert!(NomApplication::nouveau(&"n".repeat(19)).is_ok());
    assert_eq!(
        NomApplication::nouveau(&"n".repeat(20)),
        Err(ErreurParametre::TropLong { maximum: 19 })
    );
    assert_eq!(NomApplication::nouveau(""), Err(ErreurParametre::Vide));
    assert_eq!(
        NomApplication::nouveau("myiro\0libre"),
        Err(ErreurParametre::CaractereInterdit)
    );
}

#[test]
fn le_nom_d_application_est_en_ascii() {
    assert_eq!(
        NomApplication::nouveau("myiro-libre é"),
        Err(ErreurParametre::CaractereInterdit)
    );
}

#[test]
fn les_infos_systeme_font_100_octets_aux_memes_positions_en_32_et_64_bits() {
    assert_eq!(size_of::<InfosSysteme>(), 100);
    assert_eq!(offset_of!(InfosSysteme, version_sdk), 0);
    assert_eq!(offset_of!(InfosSysteme, version_micrologiciel), 0x0c);
    assert_eq!(offset_of!(InfosSysteme, code_produit), 0x18);
    assert_eq!(offset_of!(InfosSysteme, numero_serie), 0x1c);
    assert_eq!(offset_of!(InfosSysteme, adresse_mac), 0x24);
    assert_eq!(offset_of!(InfosSysteme, date_etalonnage_usine), 0x2c);
    assert_eq!(offset_of!(InfosSysteme, date_premiere_mesure), 0x38);
    assert_eq!(offset_of!(InfosSysteme, nom_produit), 0x44);
}

#[test]
fn le_tampon_des_infos_systeme_laisse_une_marge() {
    assert_eq!(size_of::<TamponInfosSysteme>(), 256);
    assert_eq!(offset_of!(TamponInfosSysteme, infos), 0);
}

/// Réponse fictive, octet par octet, comme la DLL l'écrirait.
fn tampon_exemple() -> TamponInfosSysteme {
    let mut octets = [0u8; 256];
    let mut mots = |position: usize, valeurs: &[u32]| {
        for (k, v) in valeurs.iter().enumerate() {
            octets[position + 4 * k..position + 4 * k + 4].copy_from_slice(&v.to_le_bytes());
        }
    };
    mots(0, &[1, 0x20, 3]);
    mots(0x0c, &[1, 2, 3]);
    mots(0x2c, &[2019, 4, 9]);
    mots(0x38, &[2021, 6, 15]);
    octets[0x18..0x1c].copy_from_slice(b"9C1A");
    octets[0x1c..0x24].copy_from_slice(b"12345678");
    octets[0x24..0x2a].copy_from_slice(&[0x00, 0x20, 0x6b, 0x01, 0x02, 0x03]);
    octets[0x44..0x44 + 19].copy_from_slice(b"KONICA MINOLTA FD-9");
    unsafe { std::mem::transmute::<[u8; 256], TamponInfosSysteme>(octets) }
}

#[test]
fn un_tampon_neuf_n_est_pas_rempli() {
    // FD9_GetSystemInfo rend 0 sans rien écrire si aucun instrument n'est connecté.
    assert!(!TamponInfosSysteme::vide().infos.est_rempli());
    assert!(tampon_exemple().infos.est_rempli());
}

#[test]
fn la_version_du_sdk_est_gardee_brute() {
    assert_eq!(tampon_exemple().infos.version_sdk, [1, 0x20, 3]);
}

#[test]
fn le_micrologiciel_s_affiche_comme_le_journal_du_sdk() {
    // Format "%u.%02u.%04u" du journal du SDK.
    assert_eq!(tampon_exemple().infos.micrologiciel(), "1.02.0003");
}

#[test]
fn le_numero_de_serie_et_le_code_produit_sont_lus_sans_zero_final() {
    let infos = tampon_exemple().infos;
    assert_eq!(infos.numero_de_serie(), "12345678");
    assert_eq!(infos.code_produit(), "9C1A");
}

#[test]
fn l_adresse_mac_est_lue_a_l_octet_0x24() {
    assert_eq!(tampon_exemple().infos.mac(), "00:20:6B:01:02:03");
}

#[test]
fn le_nom_du_produit_est_lu_a_l_octet_0x44() {
    assert_eq!(tampon_exemple().infos.produit(), "KONICA MINOLTA FD-9");
}

#[test]
fn le_rappel_d_evenements_tient_dans_un_pointeur_et_nul_desinscrit() {
    assert_eq!(size_of::<GestionnaireEvenements>(), size_of::<usize>());
    let aucun: GestionnaireEvenements = None;
    assert_eq!(
        unsafe { std::mem::transmute::<GestionnaireEvenements, usize>(aucun) },
        0
    );
}

#[test]
fn la_connexion_est_etablie_sur_0_et_sur_calibration_periodique_due() {
    assert!(connexion_etablie(0));
    // 1901 : connecté, mais l'instrument demande sa calibration périodique.
    assert!(connexion_etablie(1901));
    assert!(!connexion_etablie(1001));
    assert!(!connexion_etablie(1003));
    assert!(!connexion_etablie(1999));
}

#[test]
fn une_adresse_sans_zero_final_ne_deborde_pas_sur_l_identifiant() {
    let a = appareil(0, &[b'A'; 32], b"12345678");
    assert_eq!(a.adresse(), "A".repeat(32));
}
