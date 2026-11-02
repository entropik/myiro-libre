//! Calage d'une feuille au FD-9 : points visés et image réduite de la feuille,
//! rendus à côté de la lecture (fiche `docs/abi/FD9_GetMeasurePoint.md`,
//! contrat « Pour l'application »). Champ facultatif : une réponse sans lui
//! reste lisible. Valeurs et image synthétiques.

use pont_protocole::{
    ecrire_reponse, lire_reponse, CalageFeuille, Calcul, DonneesBrutes, ErreurPont, Geometrie,
    Horodatage, ImageFeuille, Info, InstrumentMesurant, Lab, LectureFeuille, Mesure, Plage,
    PointVise, Provenance, RemiseAuRepos, Reponse, Spectre, MODELE_FD9,
};

fn lecture() -> LectureFeuille {
    LectureFeuille {
        qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
        zones: 1,
        plages: 2_520,
        fiabilite: 0,
        ecart_max: 40,
    }
}

/// Image synthétique de 3 × 2 pixels gris : 0, 1, 2 puis 253, 254, 255.
fn image() -> ImageFeuille {
    ImageFeuille::new(3, 2, 60, vec![0, 1, 2, 253, 254, 255]).unwrap()
}

fn calage(image: Option<ImageFeuille>) -> CalageFeuille {
    CalageFeuille {
        decalage: [880, -110],
        ecart_max: 40,
        points: vec![PointVise {
            zone: 0,
            rangee: 0,
            colonne: 0,
            x: 1_649,
            y: 2_600,
        }],
        image,
    }
}

#[test]
fn la_prenumerisation_rend_son_calage_avec_l_image_en_base64() {
    let reponse = Reponse::Prenumerisee {
        lecture: lecture(),
        calage: Some(calage(Some(image()))),
    };
    let texte = ecrire_reponse(&reponse);
    assert!(
        texte.ends_with(
            r#","calage":{"decalage":[880,-110],"ecart_max":40,"points":[{"zone":0,"rangee":0,"colonne":0,"x":1649,"y":2600}],"image":{"largeur":3,"hauteur":2,"ppp":60,"gris":"AAEC/f7/"}}}"#
        ),
        "{texte}"
    );
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}

#[test]
fn une_prenumerisation_sans_calage_s_ecrit_et_se_relit_comme_avant() {
    let reponse = Reponse::Prenumerisee {
        lecture: lecture(),
        calage: None,
    };
    let texte = ecrire_reponse(&reponse);
    assert!(!texte.contains("calage"), "{texte}");
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}

#[test]
fn un_calage_sans_image_reste_lisible() {
    let reponse = Reponse::Prenumerisee {
        lecture: lecture(),
        calage: Some(calage(None)),
    };
    let texte = ecrire_reponse(&reponse);
    assert!(!texte.contains("image"), "{texte}");
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}

#[test]
fn une_image_dont_la_taille_ne_correspond_pas_est_refusee() {
    assert!(ImageFeuille::new(3, 2, 60, vec![0; 5]).is_err());
    assert!(ImageFeuille::new(0, 2, 60, Vec::new()).is_err());
    let texte = ecrire_reponse(&Reponse::Prenumerisee {
        lecture: lecture(),
        calage: Some(calage(Some(image()))),
    });
    for (bon, abime) in [
        (r#""gris":"AAEC/f7/""#, r#""gris":"AAEC""#),
        (r#""gris":"AAEC/f7/""#, r#""gris":"AAE*/f7/""#),
        (r#""hauteur":2"#, r#""hauteur":3"#),
        (r#""ppp":60"#, r#""ppp":0"#),
    ] {
        let ligne = texte.replace(bon, abime);
        assert_ne!(ligne, texte);
        assert!(lire_reponse(&ligne).is_err(), "{ligne}");
    }
}

#[test]
fn le_base64_suit_le_rembourrage_standard() {
    for (octets, texte) in [
        (vec![], ""),
        (vec![0x66], "Zg=="),
        (vec![0x66, 0x6f], "Zm8="),
        (vec![0x66, 0x6f, 0x6f], "Zm9v"),
        (vec![0x66, 0x6f, 0x6f, 0x62], "Zm9vYg=="),
    ] {
        let n = octets.len() as u32;
        if n == 0 {
            continue;
        }
        let image = ImageFeuille::new(n, 1, 30, octets).unwrap();
        let json = serde_json::to_string(&image).unwrap();
        assert!(json.contains(&format!(r#""gris":"{texte}""#)), "{json}");
        assert_eq!(serde_json::from_str::<ImageFeuille>(&json).unwrap(), image);
    }
}

fn mesure() -> Mesure {
    let plage = Plage::new(
        [
            Spectre::new(vec![0.3; 36]).unwrap(),
            Spectre::new(vec![0.4; 36]).unwrap(),
            Spectre::new(vec![0.5; 36]).unwrap(),
        ],
        DonneesBrutes::new(vec![1000.0; 152]).unwrap(),
        [Lab::new([50.0, 1.0, -2.0]).unwrap(); 3],
    )
    .unwrap();
    let provenance = Provenance {
        instrument: InstrumentMesurant {
            modele: MODELE_FD9.into(),
            numero_serie: 12345678,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1A".into(),
        },
        version_sdk: [1, 0x20, 3],
        empreinte_dll: Info::Inconnue,
        version_pont: "0.1.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new("2026-10-30T15:04:05+01:00").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie: Geometrie::Feuille {},
        calcul: Calcul {
            libelle: "M0/M1/M2 ; Lab D50, 2°".into(),
            demande: Info::Inconnue,
            observe: Info::Inconnue,
        },
        feuille: Some(lecture()),
    };
    Mesure::new(vec![plage], provenance).unwrap()
}

#[test]
fn la_mesure_d_une_feuille_rend_son_calage_hors_de_la_mesure_conservee() {
    let reponse = Reponse::Mesure {
        mesure: mesure(),
        remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
        calage: Some(calage(Some(image()))),
    };
    let texte = ecrire_reponse(&reponse);
    assert!(texte.ends_with(r#""gris":"AAEC/f7/"}}}"#), "{texte}");
    assert_eq!(lire_reponse(&texte), Ok(reponse));
    // Le format conservé de la mesure ne porte ni points ni image.
    assert!(!pont_protocole::ecrire_mesure(&mesure()).contains("calage"));
}

#[test]
fn une_mesure_sans_calage_s_ecrit_comme_avant() {
    let texte = ecrire_reponse(&Reponse::Mesure {
        mesure: mesure(),
        remise_au_repos: Info::Inconnue,
        calage: None,
    });
    assert!(!texte.contains("calage"), "{texte}");
    let Ok(Reponse::Mesure { calage, .. }) = lire_reponse(&texte) else {
        panic!("{texte}")
    };
    assert_eq!(calage, None);
}

#[test]
fn le_composant_qr_introuvable_s_ecrit_en_clair() {
    let reponse = Reponse::Erreur {
        erreur: ErreurPont::ComposantQrIntrouvable {
            chemin: "C:/myiro-libre/zxing.dll".into(),
        },
    };
    let texte = ecrire_reponse(&reponse);
    assert_eq!(
        texte,
        r#"{"rep":"erreur","erreur":{"type":"composant_qr_introuvable","chemin":"C:/myiro-libre/zxing.dll"}}"#
    );
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}
