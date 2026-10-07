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

/// L'étalonnage réussi porte sa date, posée par le pont (ADR 0005 : la
/// provenance vient du pont). Sans elle, ou mal formée, la réponse est refusée.
#[test]
fn un_etalonnage_reussi_porte_la_date_du_pont() {
    use pont_protocole::{ecrire_reponse, lire_reponse, Horodatage, Reponse};
    let reponse = Reponse::Etalonne {
        date: Horodatage::new("2026-10-07T09:30:00+02:00").unwrap(),
    };
    let ligne = ecrire_reponse(&reponse);
    assert_eq!(
        ligne,
        r#"{"rep":"etalonne","date":"2026-10-07T09:30:00+02:00"}"#
    );
    assert_eq!(lire_reponse(&ligne), Ok(reponse));
    for refusee in [
        r#"{"rep":"etalonne"}"#,
        r#"{"rep":"etalonne","date":"2026-10-07T09:30:00"}"#,
        r#"{"rep":"etalonne","date":null}"#,
    ] {
        assert!(lire_reponse(refusee).is_err(), "{refusee}");
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

// --- FD-9 (ticket #13) : paramètre de connexion, version et détection propres.

#[test]
fn le_fd9_se_designe_par_son_adresse_reseau() {
    use pont_protocole::AdresseReseau;
    let requete = lire_requete(r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#);
    assert_eq!(
        requete,
        Ok(Requete::ConnecterAdresse {
            adresse: AdresseReseau::new("192.0.2.40").unwrap()
        })
    );
    let texte = serde_json::to_string(&requete.unwrap()).unwrap();
    assert_eq!(
        texte,
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#
    );
}

#[test]
fn une_adresse_que_la_dll_ne_recopierait_pas_en_entier_est_refusee() {
    // 23 caractères au plus (fiche docs/abi/FD9_Connect.md), ASCII, sans espace.
    let ligne = |adresse: &str| format!(r#"{{"cmd":"connecter_adresse","adresse":"{adresse}"}}"#);
    assert!(lire_requete(&ligne("fd9-atelier.exemple.lan")).is_ok());
    assert!(lire_requete(&ligne("fd9-atelier.exemple.lan2")).is_err());
    assert!(lire_requete(&ligne("")).is_err());
    assert!(lire_requete(&ligne("192.0.2.40 ")).is_err());
    assert!(lire_requete(&ligne("équipe")).is_err());
    assert!(
        lire_requete(r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40","port":49152}"#).is_err()
    );
    assert!(lire_requete(r#"{"cmd":"connecter_adresse"}"#).is_err());
}

#[test]
fn la_version_du_fd9_vient_du_fichier_de_la_dll() {
    use pont_protocole::{ecrire_reponse, lire_reponse, Empreinte, Info, Reponse};
    let reponse = Reponse::VersionDll {
        version_fichier: Info::Confirmee([1, 3, 2, 3]),
        empreinte: Info::Confirmee(Empreinte::new("5e".repeat(32)).unwrap()),
    };
    let ligne = ecrire_reponse(&reponse);
    assert_eq!(
        ligne,
        format!(
            r#"{{"rep":"version_dll","version_fichier":{{"statut":"confirmee","valeur":[1,3,2,3]}},"empreinte":{{"statut":"confirmee","valeur":"{}"}}}}"#,
            "5e".repeat(32)
        )
    );
    assert_eq!(lire_reponse(&ligne), Ok(reponse));
    let inconnue = r#"{"rep":"version_dll","version_fichier":{"statut":"inconnue"},"empreinte":{"statut":"inconnue"}}"#;
    assert_eq!(
        lire_reponse(inconnue),
        Ok(Reponse::VersionDll {
            version_fichier: Info::Inconnue,
            empreinte: Info::Inconnue,
        })
    );
    // Rien n'est deviné : une donnée absente est refusée, pas tenue pour inconnue.
    assert!(
        lire_reponse(r#"{"rep":"version_dll","version_fichier":{"statut":"inconnue"}}"#).is_err()
    );
}

#[test]
fn un_fd9_detecte_porte_une_liaison_et_un_identifiant_qualifies() {
    use pont_protocole::{ecrire_reponse, lire_reponse, Info, InstrumentFd9, LiaisonFd9, Reponse};
    let reponse = Reponse::InstrumentsFd9 {
        liste: vec![InstrumentFd9 {
            liaison: Info::Confirmee(LiaisonFd9::Reseau),
            adresse: "192.0.2.40".into(),
            identifiant: Info::Supposee("12345678".into()),
        }],
    };
    let ligne = ecrire_reponse(&reponse);
    assert_eq!(
        ligne,
        r#"{"rep":"instruments_fd9","liste":[{"liaison":{"statut":"confirmee","valeur":"reseau"},"adresse":"192.0.2.40","identifiant":{"statut":"supposee","valeur":"12345678"}}]}"#
    );
    assert_eq!(lire_reponse(&ligne), Ok(reponse));
    assert!(lire_reponse(
        r#"{"rep":"instruments_fd9","liste":[{"liaison":{"statut":"confirmee","valeur":"reseau"},"adresse":"192.0.2.40","identifiant":{"statut":"inconnue"},"port":49152}]}"#
    )
    .is_err());
}

#[test]
fn une_liaison_inconnue_est_dite_inconnue_jamais_en_texte_libre() {
    use pont_protocole::{lire_reponse, Info, Reponse};
    let ligne = r#"{"rep":"instruments_fd9","liste":[{"liaison":{"statut":"inconnue"},"adresse":"192.0.2.40","identifiant":{"statut":"inconnue"}}]}"#;
    match lire_reponse(ligne) {
        Ok(Reponse::InstrumentsFd9 { liste }) => {
            assert_eq!(liste[0].liaison, Info::Inconnue);
            assert_eq!(liste[0].identifiant, Info::Inconnue);
        }
        autre => panic!("{autre:?}"),
    }
    for nue in [
        r#"{"rep":"instruments_fd9","liste":[{"liaison":"reseau","adresse":"192.0.2.40","identifiant":{"statut":"inconnue"}}]}"#,
        r#"{"rep":"instruments_fd9","liste":[{"liaison":{"statut":"confirmee","valeur":"inconnue 7"},"adresse":"192.0.2.40","identifiant":{"statut":"inconnue"}}]}"#,
        r#"{"rep":"instruments_fd9","liste":[{"liaison":{"statut":"inconnue"},"adresse":"192.0.2.40","identifiant":"12345678"}]}"#,
    ] {
        assert!(lire_reponse(nue).is_err(), "{nue}");
    }
}

#[test]
fn les_lignes_du_myiro1_restent_lues_et_ecrites_comme_avant() {
    use pont_protocole::{ecrire_reponse, lire_reponse};
    for ligne in [
        r#"{"rep":"version","parties":[1,0,1]}"#,
        r#"{"rep":"instruments","liste":[{"liaison":"usb","port":"COM3","numero_serie":12345678}]}"#,
    ] {
        assert_eq!(ecrire_reponse(&lire_reponse(ligne).unwrap()), ligne);
    }
    assert_eq!(
        serde_json::to_string(&Requete::Connecter { instrument: 0 }).unwrap(),
        r#"{"cmd":"connecter","instrument":0}"#
    );
}
