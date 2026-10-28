//! Structures de la mesure d'une feuille (fiches `docs/abi/FD9_*`) : une
//! taille ou une position fausse corromprait la mémoire du pont. Ces tests
//! tournent en 64 bits (`cargo test`) et en 32 bits
//! (`cargo test --target i686-pc-windows-msvc`).

use fd9_sys::{
    CleOption, CompteZone, CoordonneesMire, DonneesQr, ErreurParametre, EtatSdk, GeometrieZone,
    ImageFd9, ModeReconnaissance, PointMesure, PositionPlage, ResultatQr, CODE_RECERTIFICATION_DUE,
    DONNEES_BRUTES, DONNEES_BRUTES_TYPE, LAB_TYPE, PLAGES_MAX, SPECTRE_TYPE, ZONES_MAX,
};
use pont_protocole::{DefinitionFeuille, ZoneFeuille};
use std::mem::{offset_of, size_of};

#[test]
fn coordonnees_mire_fait_0xd70_en_32_bits_et_0xd78_en_64() {
    #[cfg(target_pointer_width = "32")]
    {
        assert_eq!(size_of::<CoordonneesMire>(), 0xd70);
        assert_eq!(size_of::<ImageFd9>(), 16);
    }
    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(size_of::<CoordonneesMire>(), 0xd78);
        assert_eq!(size_of::<ImageFd9>(), 24);
    }
    assert_eq!(offset_of!(ImageFd9, bits), 8);
    assert_eq!(size_of::<CompteZone>(), 12);
    assert_eq!(size_of::<GeometrieZone>(), 24);
}

#[test]
fn les_geometries_commencent_a_0x268_et_le_modele_a_0xd60() {
    assert_eq!(offset_of!(CoordonneesMire, largeur), 0);
    assert_eq!(offset_of!(CoordonneesMire, hauteur), 4);
    assert_eq!(offset_of!(CoordonneesMire, zones), 8);
    assert_eq!(offset_of!(CoordonneesMire, plages), 0x0c);
    assert_eq!(offset_of!(CoordonneesMire, compte_zones), 0x10);
    assert_eq!(offset_of!(CoordonneesMire, geometries), 0x268);
    assert_eq!(offset_of!(CoordonneesMire, modele_present), 0x718);
    assert_eq!(offset_of!(CoordonneesMire, modele), 0xd60);
    assert_eq!(offset_of!(GeometrieZone, pas_x), 0x10);
    assert_eq!(offset_of!(GeometrieZone, pas_y), 0x14);
}

#[test]
fn donnees_qr_fait_1172_octets_et_le_texte_commence_a_0x294() {
    assert_eq!(size_of::<DonneesQr>(), 1172);
    assert_eq!(offset_of!(DonneesQr, appareil), 0);
    assert_eq!(offset_of!(DonneesQr, version), 4);
    assert_eq!(offset_of!(DonneesQr, texte), 0x294);
}

#[test]
fn le_qr_se_lit_sans_zero_final_et_vide_au_depart() {
    let mut qr = DonneesQr::vide();
    assert_eq!(qr.appareil(), "");
    assert_eq!(qr.texte(), "");
    // Comme le SDK le remplirait pour « FD-9:011M:RGB-2520 - 1P MY9*1+ ».
    qr.appareil.copy_from_slice(b"FD-9");
    qr.version.copy_from_slice(b"011M");
    qr.texte[..20].copy_from_slice(b"RGB-2520 - 1P MY9*1+");
    assert_eq!(qr.appareil(), "FD-9");
    assert_eq!(qr.texte(), "RGB-2520 - 1P MY9*1+");
}

#[test]
fn les_resultats_du_qr_sont_nommes() {
    assert_eq!(ResultatQr::depuis(0), ResultatQr::Trouve);
    assert_eq!(ResultatQr::depuis(1), ResultatQr::Absent);
    assert_eq!(ResultatQr::depuis(2), ResultatQr::Illisible);
    assert_eq!(ResultatQr::depuis(3), ResultatQr::Inconnu(3));
}

#[test]
fn position_plage_fait_12_octets_et_point_mesure_20() {
    assert_eq!(size_of::<PositionPlage>(), 12);
    assert_eq!(offset_of!(PositionPlage, rangee), 4);
    assert_eq!(offset_of!(PositionPlage, colonne), 8);
    assert_eq!(size_of::<PointMesure>(), 20);
    // (index, -1, -1) désigne la plage n° index (fiche FD9_GetData).
    assert_eq!(
        PositionPlage::numero(41),
        PositionPlage {
            zone_ou_index: 41,
            rangee: -1,
            colonne: -1
        }
    );
}

#[test]
fn les_options_et_les_modes_ont_les_valeurs_du_sdk() {
    assert_eq!(ModeReconnaissance::Combine as u32, 4);
    assert_eq!(ModeReconnaissance::Coordonnees as u32, 2);
    assert_eq!(CleOption::Condition as u32, 0);
    assert_eq!(CleOption::Illuminant as u32, 1);
    assert_eq!(CleOption::Observateur as u32, 2);
    assert_eq!(CleOption::TypeDonnees as u32, 3);
    assert_eq!(
        (DONNEES_BRUTES_TYPE, SPECTRE_TYPE, LAB_TYPE),
        (0x0c, 0x0b, 0)
    );
    assert_eq!(DONNEES_BRUTES, 152);
    assert_eq!((ZONES_MAX, PLAGES_MAX), (50, 15_000));
    assert_eq!(CODE_RECERTIFICATION_DUE, 1901);
}

/// La RGB-2520 (nombres de l'étude statique, 1/100 mm).
fn rgb2520() -> DefinitionFeuille {
    DefinitionFeuille {
        nom: "RGB-2520 - 1P MY9".into(),
        largeur: 27_940,
        hauteur: 41_660,
        zones: vec![ZoneFeuille {
            gauche: 1_349,
            haut: 2_300,
            largeur_plage: 601,
            hauteur_plage: 601,
            pas_x: 601,
            pas_y: 601,
            rangees: 60,
            colonnes: 42,
        }],
    }
}

#[test]
fn la_rgb2520_tient_dans_la_feuille() {
    // 1 349 + 601 × 42 = 26 591 ≤ 27 940 ; 2 300 + 601 × 60 = 38 360 ≤ 41 660.
    let c = CoordonneesMire::depuis_definition(&rgb2520()).unwrap();
    assert_eq!(
        (c.largeur, c.hauteur, c.zones, c.plages),
        (27_940, 41_660, 1, 2_520)
    );
    assert_eq!(
        c.compte_zones[0],
        CompteZone {
            rangees: 60,
            colonnes: 42,
            plages: 2_520
        }
    );
    assert_eq!(
        c.geometries[0],
        GeometrieZone {
            gauche: 1_349,
            haut: 2_300,
            largeur: 601,
            hauteur: 601,
            pas_x: 601,
            pas_y: 601
        }
    );
    // Tout le reste à zéro : ni modèle, ni zone fantôme.
    assert_eq!(c.compte_zones[1], CompteZone::default());
    assert_eq!(c.geometries[49], GeometrieZone::default());
    assert_eq!(c.modele_present, 0);
    assert!(c.reserve.iter().all(|&o| o == 0));
    assert!(c.modele.donnees.is_null());
    assert_eq!(
        (c.modele.largeur, c.modele.hauteur, c.modele.bits),
        (0, 0, 0)
    );
}

#[test]
fn une_zone_qui_deborde_est_refusee() {
    let mut mire = rgb2520();
    mire.largeur = 26_590;
    assert_eq!(
        CoordonneesMire::depuis_definition(&mire).unwrap_err(),
        ErreurParametre::ZoneHorsFeuille { zone: 1 }
    );
    let mut mire = rgb2520();
    mire.hauteur = 38_359;
    assert!(CoordonneesMire::depuis_definition(&mire).is_err());
}

#[test]
fn plus_de_50_zones_sont_refusees() {
    let mut mire = rgb2520();
    mire.zones[0].rangees = 1;
    mire.zones[0].colonnes = 1;
    mire.zones = vec![mire.zones[0].clone(); 51];
    assert_eq!(
        CoordonneesMire::depuis_definition(&mire).unwrap_err(),
        ErreurParametre::TropDeZones { maximum: 50 }
    );
    mire.zones.pop();
    let c = CoordonneesMire::depuis_definition(&mire).unwrap();
    assert_eq!((c.zones, c.plages), (50, 50));
}

#[test]
fn une_definition_sans_zone_est_refusee() {
    let mut mire = rgb2520();
    mire.zones.clear();
    assert_eq!(
        CoordonneesMire::depuis_definition(&mire).unwrap_err(),
        ErreurParametre::Vide
    );
}

#[test]
fn des_rangees_ou_colonnes_hors_bornes_sont_refusees() {
    // Bornes du SDK : 1 à 250 rangées, 1 à 60 colonnes par zone.
    for (rangees, colonnes) in [(251, 1), (1, 61), (0, 1), (1, 0)] {
        let mut mire = rgb2520();
        mire.largeur = 100_000;
        mire.hauteur = 200_000;
        mire.zones[0].rangees = rangees;
        mire.zones[0].colonnes = colonnes;
        assert_eq!(
            CoordonneesMire::depuis_definition(&mire).unwrap_err(),
            ErreurParametre::ZoneHorsBornes { zone: 1 },
            "{rangees} × {colonnes}"
        );
    }
}

#[test]
fn plus_de_15000_plages_sont_refusees() {
    // 2 zones de 250 × 31 = 15 500 plages.
    let mut mire = rgb2520();
    mire.largeur = 100_000;
    mire.hauteur = 400_000;
    let zone = ZoneFeuille {
        rangees: 250,
        colonnes: 31,
        ..mire.zones[0].clone()
    };
    mire.zones = vec![zone; 2];
    assert_eq!(
        CoordonneesMire::depuis_definition(&mire).unwrap_err(),
        ErreurParametre::TropDePlages { maximum: 15_000 }
    );
}

#[test]
fn les_etats_inconnus_restent_inconnus() {
    assert_eq!(EtatSdk::depuis(42), EtatSdk::Inconnu(42));
    assert_eq!(EtatSdk::depuis(42).code(), 42);
    assert!(!EtatSdk::depuis(42).est_erreur());
}

#[test]
fn les_etats_du_rappel_sont_nommes_et_relus() {
    let attendus = [
        (0, EtatSdk::AttenteSansMain),
        (1, EtatSdk::Attente),
        (2, EtatSdk::Alimentation),
        (3, EtatSdk::AlimentationFinie),
        (4, EtatSdk::Capture),
        (5, EtatSdk::CaptureFinie),
        (6, EtatSdk::Reconnaissance),
        (7, EtatSdk::ReconnaissanceFinie),
        (8, EtatSdk::Mesure),
        (9, EtatSdk::MesureFinie),
        (10, EtatSdk::AnnulationFinie),
        (11, EtatSdk::AvisPlages),
        (101, EtatSdk::CouvercleOuvert),
        (102, EtatSdk::Bourrage),
        (103, EtatSdk::Communication),
        (201, EtatSdk::Premesure),
        (202, EtatSdk::ErreurMesure),
        (203, EtatSdk::Capteur),
        (204, EtatSdk::Calcul),
        (205, EtatSdk::FeuilleTropLongue),
        (206, EtatSdk::Memoire),
        (301, EtatSdk::Fatale),
    ];
    for (code, etat) in attendus {
        assert_eq!(EtatSdk::depuis(code), etat);
        assert_eq!(etat.code(), code);
        assert_eq!(etat.est_erreur(), code >= 101, "{code}");
    }
    for trou in [12, 100, 104, 200, 207, 300, 302] {
        assert_eq!(EtatSdk::depuis(trou), EtatSdk::Inconnu(trou));
    }
}
