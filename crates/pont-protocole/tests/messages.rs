//! Le dialogue application ↔ pont : une ligne JSON par message, rien de deviné.

use pont_protocole::{lire_requete, Requete};

#[test]
fn une_requete_connue_est_lue() {
    assert_eq!(
        lire_requete(r#"{"cmd":"version"}"#),
        Ok(Requete::Version {})
    );
    assert_eq!(
        lire_requete(r#"{"cmd":"detecter"}"#),
        Ok(Requete::Detecter {})
    );
    assert_eq!(
        lire_requete(r#"{"cmd":"connecter","instrument":0}"#),
        Ok(Requete::Connecter { instrument: 0 })
    );
}

#[test]
fn une_commande_inconnue_est_rejetee() {
    assert!(lire_requete(r#"{"cmd":"jig_update_program"}"#).is_err());
    assert!(lire_requete(r#"{"cmd":"set_network_info"}"#).is_err());
}

#[test]
fn un_champ_inattendu_est_rejete() {
    assert!(lire_requete(r#"{"cmd":"version","force":true}"#).is_err());
}

#[test]
fn un_message_mal_forme_est_rejete() {
    assert!(lire_requete("version").is_err());
    assert!(lire_requete(r#"{"cmd":"connecter"}"#).is_err());
    assert!(lire_requete(r#"{"cmd":"connecter","instrument":-1}"#).is_err());
}

#[test]
fn chaque_requete_relue_redonne_la_meme() {
    for requete in [
        Requete::Version {},
        Requete::Detecter {},
        Requete::Connecter { instrument: 3 },
    ] {
        let texte = serde_json::to_string(&requete).unwrap();
        assert_eq!(lire_requete(&texte), Ok(requete));
    }
}

#[test]
fn les_requetes_de_mesure_sont_lues() {
    assert_eq!(
        lire_requete(r#"{"cmd":"etalonner"}"#),
        Ok(Requete::Etalonner {})
    );
    assert_eq!(
        lire_requete(r#"{"cmd":"mesurer_ponctuelle"}"#),
        Ok(Requete::MesurerPonctuelle {})
    );
    assert_eq!(
        lire_requete(r#"{"cmd":"mesurer_bande","plages_attendues":12}"#),
        Ok(Requete::MesurerBande {
            plages_attendues: Some(12)
        })
    );
    assert_eq!(
        lire_requete(r#"{"cmd":"mesurer_bande"}"#),
        Ok(Requete::MesurerBande {
            plages_attendues: None
        })
    );
    assert_eq!(lire_requete(r#"{"cmd":"fermer"}"#), Ok(Requete::Fermer {}));
}

#[test]
fn une_erreur_s_ecrit_en_clair_pour_l_application() {
    use pont_protocole::{ecrire_reponse, ErreurPont, Palier, Reponse};
    let reponse = Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande: Palier::Connexion,
            plafond: Palier::Detection,
        },
    };
    assert_eq!(
        ecrire_reponse(&reponse),
        r#"{"rep":"erreur","erreur":{"type":"palier_non_autorise","demande":"connexion","plafond":"detection"}}"#
    );
}

#[test]
fn une_reponse_relue_par_l_application_redonne_la_meme() {
    use pont_protocole::{ecrire_reponse, lire_reponse, InstrumentDetecte, Reponse};
    let reponse = Reponse::Instruments {
        liste: vec![InstrumentDetecte {
            liaison: "usb".into(),
            port: "COM3".into(),
            numero_serie: 12345678,
        }],
    };
    assert_eq!(lire_reponse(&ecrire_reponse(&reponse)), Ok(reponse));
}

#[test]
fn une_mesure_porte_sa_provenance() {
    use pont_protocole::{ecrire_reponse, lire_reponse, InstrumentMesurant, Provenance, Reponse};
    let reponse = Reponse::Mesure {
        plages: vec![],
        sens: 0,
        provenance: Provenance {
            instrument: InstrumentMesurant {
                modele: "MYIRO-1".into(),
                numero_serie: 12345678,
                micrologiciel: "1.02.0005".into(),
                code_produit: "9C1D".into(),
            },
            version_sdk: [1, 1, 0],
            empreinte_dll: Some("ab".repeat(32)),
            version_pont: "0.1.0".into(),
            architecture: "x86_64".into(),
            horodatage: "2026-10-07T15:04:05+02:00".into(),
            etalonnage: Some("2026-10-07T15:00:00+02:00".into()),
            geometrie: "ponctuelle".into(),
            calcul: "M0/M1/M2 : Illuminant 0/1/2 ; Lab D50, 2°".into(),
        },
    };
    let texte = ecrire_reponse(&reponse);
    assert!(texte.contains(r#""horodatage":"2026-10-07T15:04:05+02:00""#));
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}
