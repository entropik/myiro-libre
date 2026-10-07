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
fn une_erreur_sans_detail_s_ecrit_par_son_seul_type() {
    use pont_protocole::{ecrire_reponse, lire_reponse, ErreurPont, Reponse};
    for (erreur, texte) in [
        (ErreurPont::Delai {}, "delai"),
        (ErreurPont::InstrumentPerdu {}, "instrument_perdu"),
        (ErreurPont::InstrumentInconnu {}, "instrument_inconnu"),
        (ErreurPont::ParametreRefuse {}, "parametre_refuse"),
        (ErreurPont::EtatIncompatible {}, "etat_incompatible"),
        (ErreurPont::NonEtalonne {}, "non_etalonne"),
        (ErreurPont::EtalonnageRequis {}, "etalonnage_requis"),
        (ErreurPont::SessionInexploitable {}, "session_inexploitable"),
        (ErreurPont::SessionFermee {}, "session_fermee"),
    ] {
        let reponse = Reponse::Erreur { erreur };
        let ligne = format!(r#"{{"rep":"erreur","erreur":{{"type":"{texte}"}}}}"#);
        assert_eq!(ecrire_reponse(&reponse), ligne);
        assert_eq!(lire_reponse(&ligne), Ok(reponse));
    }
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
