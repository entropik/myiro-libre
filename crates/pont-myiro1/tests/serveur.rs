//! La boucle du pont : une ligne JSON par requête en entrée, une par réponse en sortie.

mod commun;

use commun::{evenement, SdkSimule};
use pont_myiro1::serveur::servir;
use pont_myiro1::Session;
use pont_protocole::{lire_reponse, ErreurPont, Palier, Reponse};

/// Envoie `requetes` au pont et rend ses réponses décodées.
fn dialoguer(sdk: SdkSimule, plafond: Palier, requetes: &[&str]) -> Vec<Reponse> {
    dialoguer_brut(sdk, plafond, requetes)
        .0
        .iter()
        .map(|ligne| lire_reponse(ligne).unwrap())
        .collect()
}

/// Idem, en rendant les lignes telles qu'écrites et la session pour examen.
fn dialoguer_brut(
    sdk: SdkSimule,
    plafond: Palier,
    requetes: &[&str],
) -> (Vec<String>, Session<SdkSimule>) {
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

const JUSQU_A_LA_CONNEXION: [&str; 3] = [
    r#"{"cmd":"version"}"#,
    r#"{"cmd":"detecter"}"#,
    r#"{"cmd":"connecter","instrument":0}"#,
];

fn sequence(suite: &[&'static str]) -> Vec<&'static str> {
    JUSQU_A_LA_CONNEXION.iter().chain(suite).copied().collect()
}

fn armements(session: &Session<SdkSimule>) -> usize {
    session
        .sdk()
        .appels
        .iter()
        .filter(|a| a.starts_with("armer"))
        .count()
}

#[test]
fn apres_un_etalonnage_echoue_le_pont_repond_etalonnage_requis_sans_armer() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8, 7, 9].map(evenement));
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
        ]),
    );
    assert_eq!(
        lignes[5],
        r#"{"rep":"erreur","erreur":{"type":"etalonnage_requis"}}"#
    );
    assert_eq!(armements(&session), 0);
}

#[test]
fn apres_une_perte_de_liaison_le_pont_repond_instrument_perdu_sans_armer() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 6].map(evenement).to_vec());
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
        ]),
    );
    let perdu = r#"{"rep":"erreur","erreur":{"type":"instrument_perdu"}}"#;
    assert_eq!(lignes[4], perdu);
    assert_eq!(lignes[5], perdu);
    assert_eq!(armements(&session), 1);
}

#[test]
fn une_identite_illisible_rend_la_session_inexploitable_pour_le_consommateur() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.code_infos = -1;
    sdk.evenements.extend([7, 8].map(evenement));
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[r#"{"cmd":"etalonner"}"#, r#"{"cmd":"mesurer_ponctuelle"}"#]),
    );
    let inexploitable = r#"{"rep":"erreur","erreur":{"type":"session_inexploitable"}}"#;
    assert_eq!(lignes[3], inexploitable);
    assert_eq!(lignes[4], inexploitable);
    assert!(!session
        .sdk()
        .appels
        .contains(&"etalonner blanc".to_string()));
    assert_eq!(armements(&session), 0);
}

#[test]
fn chaque_requete_recoit_sa_reponse() {
    let reponses = dialoguer(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    assert_eq!(reponses.len(), 4);
    assert_eq!(reponses[0], Reponse::Version { parties: [1, 1, 0] });
    let Reponse::Instruments { liste } = &reponses[1] else {
        panic!("{:?}", reponses[1])
    };
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].liaison, "reseau");
    let Reponse::Connecte { identite } = &reponses[2] else {
        panic!("{:?}", reponses[2])
    };
    assert_eq!(identite.numero_serie, 12345678);
    assert_eq!(identite.brute_hex.len(), 80, "40 octets en hexadécimal");
    assert_eq!(reponses[3], Reponse::Ferme {});
}

#[test]
fn une_ligne_illisible_ne_fait_rien_et_le_pont_continue() {
    let reponses = dialoguer(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &["n'importe quoi", r#"{"cmd":"version"}"#],
    );
    assert!(matches!(reponses[0], Reponse::RequeteInvalide { .. }));
    assert_eq!(reponses[1], Reponse::Version { parties: [1, 1, 0] });
}

#[test]
fn le_plafond_s_applique_aussi_par_le_protocole() {
    let reponses = dialoguer(
        SdkSimule::avec_un_myiro1(),
        Palier::Detection,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
        ],
    );
    assert_eq!(
        reponses[2],
        Reponse::Erreur {
            erreur: ErreurPont::PalierNonAutorise {
                demande: Palier::Connexion,
                plafond: Palier::Detection
            }
        }
    );
}

#[test]
fn la_fin_de_l_entree_termine_le_pont_sans_erreur() {
    let reponses = dialoguer(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &[r#"{"cmd":"version"}"#],
    );
    assert_eq!(reponses.len(), 1);
}

#[test]
fn rien_n_est_traite_apres_fermer() {
    let reponses = dialoguer(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &[r#"{"cmd":"fermer"}"#, r#"{"cmd":"version"}"#],
    );
    assert_eq!(reponses, [Reponse::Ferme {}]);
}

#[test]
fn une_bande_rend_une_plage_par_resultat() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    sdk.resultats_par_lecture = 12;
    let reponses = dialoguer(
        sdk,
        Palier::Bande,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_bande","plages_attendues":12}"#,
        ],
    );
    assert_eq!(reponses[3], Reponse::Etalonne {});
    let Reponse::Mesure { plages, .. } = &reponses[4] else {
        panic!("{:?}", reponses[4])
    };
    assert_eq!(plages.len(), 12);
    assert_eq!(plages[0].m1.len(), 36);
    assert_eq!(plages[0].brutes.len(), 152);
    assert_eq!(plages[0].lab_m1.len(), 3);
}
