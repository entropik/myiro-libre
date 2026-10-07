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
