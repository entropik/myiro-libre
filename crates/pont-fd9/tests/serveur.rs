//! Le dialogue JSON du pont FD-9 : même protocole que le pont MYIRO-1, une
//! ligne par requête et une par réponse, avec ses propres paliers.

mod commun;

use commun::{Fd9Simule, EMPREINTE_SIMULEE};
use pont_fd9::serveur::servir;
use pont_fd9::Session;
use pont_protocole::{
    lire_reponse, Empreinte, ErreurPont, Info, InstrumentFd9, LiaisonFd9, Palier, Reponse,
};

/// Envoie `requetes` au pont ; rend les lignes écrites et la session.
fn dialoguer(
    sdk: Fd9Simule,
    plafond: Palier,
    requetes: &[&str],
) -> (Vec<String>, Session<Fd9Simule>) {
    let mut session = Session::new(sdk, plafond);
    let entree = requetes.join("\n");
    let mut sortie = Vec::new();
    servir(&mut session, entree.as_bytes(), &mut sortie).unwrap();
    let lignes = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    (lignes, session)
}

fn reponses(lignes: &[String]) -> Vec<Reponse> {
    lignes.iter().map(|l| lire_reponse(l).unwrap()).collect()
}

#[test]
fn version_puis_detection_rendent_le_fd9_du_reseau() {
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"version"}"#, r#"{"cmd":"detecter"}"#],
    );
    assert_eq!(
        reponses(&lignes),
        vec![
            Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Confirmee(Empreinte::new(EMPREINTE_SIMULEE.repeat(32)).unwrap()),
            },
            Reponse::InstrumentsFd9 {
                liste: vec![InstrumentFd9 {
                    liaison: Info::Confirmee(LiaisonFd9::Reseau),
                    adresse: "192.0.2.40".into(),
                    identifiant: Info::Supposee("12345678".into()),
                }],
            },
        ]
    );
}

#[test]
fn une_liaison_hors_fiche_et_un_identifiant_vide_sont_inconnus() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.appareils[0].code_liaison = 7;
    sdk.appareils[0].identifiant = [0; 8];
    let (lignes, _) = dialoguer(
        sdk,
        Palier::Detection,
        &[r#"{"cmd":"version"}"#, r#"{"cmd":"detecter"}"#],
    );
    assert_eq!(
        reponses(&lignes)[1],
        Reponse::InstrumentsFd9 {
            liste: vec![InstrumentFd9 {
                liaison: Info::Inconnue,
                adresse: "192.0.2.40".into(),
                identifiant: Info::Inconnue,
            }],
        }
    );
}

#[test]
fn une_version_illisible_est_dite_inconnue() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.version_fichier = None;
    sdk.empreinte = Some("pas une empreinte".into());
    let (lignes, _) = dialoguer(sdk, Palier::Version, &[r#"{"cmd":"version"}"#]);
    assert_eq!(
        reponses(&lignes),
        vec![Reponse::VersionDll {
            version_fichier: Info::Inconnue,
            empreinte: Info::Inconnue,
        }]
    );
}

#[test]
fn la_connexion_est_refusee_par_les_paliers_du_fd9_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Bande,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"mesurer_bande"}"#,
        ],
    );
    let refus = |demande| Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande,
            plafond: Palier::Detection,
        },
    };
    assert_eq!(
        reponses(&lignes)[2..],
        [
            refus(Palier::Connexion),
            refus(Palier::Connexion),
            refus(Palier::Etalonnage),
            refus(Palier::MesurePonctuelle),
            refus(Palier::Bande),
        ]
    );
    assert_eq!(session.sdk().appels.len(), 2, "{:?}", session.sdk().appels);
}

#[test]
fn une_adresse_trop_longue_est_une_requete_invalide() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"connecter_adresse","adresse":"fd9-atelier.exemple.lan2"}"#],
    );
    assert!(matches!(
        reponses(&lignes)[0],
        Reponse::RequeteInvalide { .. }
    ));
    assert!(session.sdk().appels.is_empty());
}

#[test]
fn fermer_est_confirme_et_arrete_le_pont() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"detecter"}"#,
        ],
    );
    assert_eq!(lignes.len(), 2);
    assert_eq!(lire_reponse(&lignes[1]), Ok(Reponse::Ferme {}));
    assert_eq!(session.sdk().appels.len(), 1);
}

#[test]
fn une_ligne_illisible_ne_declenche_rien() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &["version", r#"{"cmd":"jig_get_sdk_version"}"#],
    );
    assert!(reponses(&lignes)
        .iter()
        .all(|r| matches!(r, Reponse::RequeteInvalide { .. })));
    assert!(session.sdk().appels.is_empty());
}
