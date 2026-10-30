//! Avancement d'une feuille au FD-9 (ticket #34) : lignes écrites par le
//! pont avant sa réponse finale, pour que l'écran ne reste pas sept minutes
//! sur « glissez la feuille ». Une réponse sans avancement reste lisible.
//! Valeurs et image synthétiques.

use pont_protocole::{
    ecrire_reponse, lire_reponse, AvancementFeuille, CalageFeuille, ErreurPont, ImageFeuille,
    PointVise, Reponse,
};

fn calage() -> CalageFeuille {
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
        image: Some(ImageFeuille::new(3, 2, 60, vec![0, 1, 2, 253, 254, 255]).unwrap()),
    }
}

#[test]
fn chaque_etape_s_ecrit_sur_une_ligne_et_se_relit() {
    for (avancement, attendu) in [
        (
            AvancementFeuille::FeuilleEngagee {},
            r#"{"rep":"avancement","avancement":{"etape":"feuille_engagee"}}"#,
        ),
        (
            AvancementFeuille::Lecture {},
            r#"{"rep":"avancement","avancement":{"etape":"lecture"}}"#,
        ),
        (
            AvancementFeuille::MesureLancee {},
            r#"{"rep":"avancement","avancement":{"etape":"mesure_lancee"}}"#,
        ),
        (
            AvancementFeuille::Mesure {
                mesurees: 12,
                plages: 2_520,
            },
            r#"{"rep":"avancement","avancement":{"etape":"mesure","mesurees":12,"plages":2520}}"#,
        ),
    ] {
        let reponse = Reponse::Avancement { avancement };
        let texte = ecrire_reponse(&reponse);
        assert_eq!(texte, attendu);
        assert!(!texte.contains('\n'));
        assert_eq!(lire_reponse(&texte), Ok(reponse));
    }
}

#[test]
fn le_calage_pret_porte_les_points_et_l_image() {
    let reponse = Reponse::Avancement {
        avancement: AvancementFeuille::CalagePret { calage: calage() },
    };
    let texte = ecrire_reponse(&reponse);
    assert!(
        texte.starts_with(r#"{"rep":"avancement","avancement":{"etape":"calage_pret","calage":{"decalage":[880,-110]"#),
        "{texte}"
    );
    assert!(texte.contains(r#""gris":"AAEC/f7/""#), "{texte}");
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}

#[test]
fn une_etape_inconnue_ou_un_champ_en_trop_est_refuse() {
    for texte in [
        r#"{"rep":"avancement","avancement":{"etape":"sechage"}}"#,
        r#"{"rep":"avancement","avancement":{"etape":"lecture","en_trop":1}}"#,
        r#"{"rep":"avancement","avancement":{"etape":"mesure","mesurees":1}}"#,
        r#"{"rep":"avancement"}"#,
    ] {
        assert!(lire_reponse(texte).is_err(), "{texte}");
    }
}

/// Un pont d'avant l'avancement ne rend que sa réponse finale : elle se lit
/// comme avant.
#[test]
fn une_reponse_finale_sans_avancement_reste_lisible() {
    let texte = r#"{"rep":"erreur","erreur":{"type":"feuille_absente"}}"#;
    assert_eq!(
        lire_reponse(texte),
        Ok(Reponse::Erreur {
            erreur: ErreurPont::FeuilleAbsente {}
        })
    );
}
