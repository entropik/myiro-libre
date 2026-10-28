//! Le dialogue JSON du pont FD-9 : même protocole que le pont MYIRO-1, une
//! ligne par requête et une par réponse, avec ses propres paliers.

mod commun;

use commun::{Fd9Simule, EMPREINTE_SIMULEE};
use pont_fd9::serveur::servir;
use pont_fd9::{Session, PALIER_MAX};
use pont_protocole::{
    lire_reponse, Empreinte, ErreurPont, Info, InstrumentFd9, LiaisonFd9, Palier, RemiseAuRepos,
    Reponse,
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
fn la_connexion_est_refusee_au_plafond_detection_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        ],
    );
    let refus = Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande: Palier::Connexion,
            plafond: Palier::Detection,
        },
    };
    assert_eq!(reponses(&lignes)[2..], [refus.clone(), refus]);
    assert_eq!(session.sdk().appels.len(), 2, "{:?}", session.sdk().appels);
}

#[test]
fn etalonnage_et_mesures_du_myiro1_sont_refuses_par_les_paliers_du_fd9() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Bande,
        &[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"mesurer_bande"}"#,
        ],
    );
    let refus = |demande| Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande,
            plafond: PALIER_MAX,
        },
    };
    assert_eq!(
        reponses(&lignes),
        [
            refus(Palier::Etalonnage),
            refus(Palier::MesurePonctuelle),
            refus(Palier::Bande),
        ]
    );
    assert!(
        session.sdk().appels.is_empty(),
        "{:?}",
        session.sdk().appels
    );
}

#[test]
fn connecter_puis_fermer_rendent_l_identite_puis_ferme() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    let reponses = reponses(&lignes);
    let Reponse::Connecte { identite } = &reponses[1] else {
        panic!("{lignes:?}")
    };
    assert_eq!(identite.numero_serie, 12345678);
    assert_eq!(reponses[2], Reponse::Ferme {});
    assert_eq!(
        session.sdk().appels_dll().last(),
        Some(&"FD9_RegisterDeviceEventHandler nul")
    );
}

#[test]
fn une_deconnexion_refusee_rend_fermeture_incertaine_et_arrete_le_pont() {
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_un_fd9().rendre("FD9_Disconnect", 1104),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"version"}"#,
        ],
    );
    assert_eq!(lignes.len(), 3);
    assert_eq!(
        lire_reponse(&lignes[2]),
        Ok(Reponse::FermetureIncertaine {
            remise_au_repos: RemiseAuRepos::ArretRefuse { code: 1104 }
        })
    );
}

#[test]
fn la_fin_de_l_entree_deconnecte() {
    let (_, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        ],
    );
    assert!(!session.sdk().connecte && !session.sdk().inscrit);
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

/// Le FD-9 ne mesure pas : `annuler` n'a jamais rien à interrompre (#26).
#[test]
fn annuler_est_sans_effet_et_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"annuler"}"#],
    );
    assert_eq!(
        reponses(&lignes),
        vec![Reponse::Annulation {
            effet: pont_protocole::EffetAnnulation::SansEffet
        }]
    );
    assert!(session.sdk().appels.is_empty());
}

/// Définition de la RGB-2520, telle que l'application l'enverra.
const MIRE: &str = r#"{"nom":"RGB-2520 - 1P MY9","largeur":27940,"hauteur":41660,"zones":[{"gauche":1349,"haut":2300,"largeur_plage":601,"hauteur_plage":601,"pas_x":601,"pas_y":601,"rangees":60,"colonnes":42}]}"#;

#[test]
fn la_feuille_est_refusee_par_les_paliers_du_fd9_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Feuille,
        &[
            &format!(r#"{{"cmd":"prenumeriser","mire":{MIRE}}}"#),
            &format!(r#"{{"cmd":"mesurer_feuille","mire":{MIRE}}}"#),
        ],
    );
    let refus = |demande| Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande,
            plafond: PALIER_MAX,
        },
    };
    assert_eq!(
        reponses(&lignes),
        [refus(Palier::Prenumerisation), refus(Palier::Feuille)]
    );
    assert!(
        session.sdk().appels.is_empty(),
        "{:?}",
        session.sdk().appels
    );
}
