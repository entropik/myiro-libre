//! La boucle du pont : une ligne JSON par requête en entrée, une par réponse en sortie.

mod commun;

use commun::{evenement, SdkSimule};
use pont_myiro1::serveur::servir;
use pont_myiro1::Session;
use pont_protocole::{lire_reponse, ErreurPont, Info, Palier, RemiseAuRepos, Reponse};

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
fn la_connexion_par_adresse_du_fd9_est_refusee_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer_brut(
        SdkSimule::avec_un_myiro1(),
        Palier::Bande,
        &[r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#],
    );
    assert_eq!(lignes.len(), 1);
    assert!(matches!(
        lire_reponse(&lignes[0]).unwrap(),
        Reponse::RequeteInvalide { .. }
    ));
    // Rien n'a été connecté : la fin de l'entrée ne déconnecte rien non plus.
    assert!(
        session.sdk().appels.is_empty(),
        "{:?}",
        session.sdk().appels
    );
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
    assert_eq!(
        reponses[3],
        Reponse::FermetureIncertaine {
            remise_au_repos: RemiseAuRepos::ReposSuppose {}
        }
    );
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

fn appels(session: &Session<SdkSimule>, nom: &str) -> usize {
    session.sdk().appels.iter().filter(|a| *a == nom).count()
}

#[test]
fn ferme_n_est_repondu_qu_apres_la_deconnexion_et_un_repos_prouve() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 2, 3, 0].map(evenement).to_vec());
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"fermer"}"#,
        ]),
    );
    assert_eq!(lignes[5], r#"{"rep":"ferme"}"#);
    assert_eq!(session.sdk().appels.last().unwrap(), "deconnecter");
}

#[test]
fn fermer_juste_apres_connecter_n_annonce_qu_un_repos_suppose() {
    let (lignes, session) = dialoguer_brut(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &sequence(&[r#"{"cmd":"fermer"}"#]),
    );
    assert_eq!(
        lignes[3],
        r#"{"rep":"fermeture_incertaine","remise_au_repos":{"etat":"repos_suppose"}}"#
    );
    assert_eq!(session.sdk().appels.last().unwrap(), "deconnecter");
}

#[test]
fn une_deconnexion_echouee_n_est_pas_annoncee_fermee_et_le_pont_attend() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.code_deconnexion = -9987;
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::Connexion,
        &sequence(&[
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
        ]),
    );
    assert_eq!(
        lignes[3],
        r#"{"rep":"erreur","erreur":{"type":"deconnexion_echouee","code":-9987,"remise_au_repos":{"etat":"repos_suppose"}}}"#
    );
    let fermee = r#"{"rep":"erreur","erreur":{"type":"session_fermee"}}"#;
    assert_eq!(lignes[4], fermee);
    assert_eq!(lignes[5], fermee);
    assert_eq!(appels(&session, "version"), 1);
    assert_eq!(appels(&session, "connecter 10"), 1);
}

#[test]
fn une_fermeture_reprise_apres_echec_finit_par_la_deconnexion() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.code_deconnexion = -9987;
    let mut session = Session::new(sdk, Palier::Connexion);
    let mut sortie = Vec::new();
    servir(
        &mut session,
        sequence(&[r#"{"cmd":"fermer"}"#]).join("\n").as_bytes(),
        &mut sortie,
    )
    .unwrap();
    session.sdk_mut().code_deconnexion = 0;
    let mut sortie = Vec::new();
    servir(
        &mut session,
        &b"{\"cmd\":\"fermer\"}\n{\"cmd\":\"version\"}"[..],
        &mut sortie,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(sortie).unwrap(),
        "{\"rep\":\"fermeture_incertaine\",\"remise_au_repos\":{\"etat\":\"repos_suppose\"}}\n"
    );
    assert_eq!(appels(&session, "arreter"), 1, "désarmement fait une fois");
}

#[test]
fn une_fermeture_sans_repos_prouve_est_rapportee_incertaine() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"version"}"#,
        ]),
    );
    assert_eq!(lignes.len(), 6, "rien n'est traité après la fermeture");
    assert_eq!(
        lignes[5],
        r#"{"rep":"fermeture_incertaine","remise_au_repos":{"etat":"arret_refuse","code":-9986}}"#
    );
    assert_eq!(session.sdk().appels.last().unwrap(), "deconnecter");
}

#[test]
fn une_mesure_acquise_est_rendue_avec_un_repos_incertain_et_la_suivante_refusee() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    sdk.salves.push_back([1, 2, 3, 0].map(evenement).to_vec());
    let (lignes, session) = dialoguer_brut(
        sdk,
        Palier::MesurePonctuelle,
        &sequence(&[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
        ]),
    );
    let Reponse::Mesure {
        mesure,
        remise_au_repos,
    } = lire_reponse(&lignes[4]).unwrap()
    else {
        panic!("{}", lignes[4])
    };
    assert_eq!(mesure.plages()[0].m1().len(), 36);
    assert_eq!(
        remise_au_repos,
        Info::Confirmee(RemiseAuRepos::ReposNonSignale {})
    );
    assert!(lignes[4].ends_with(
        r#","remise_au_repos":{"statut":"confirmee","valeur":{"etat":"repos_non_signale"}}}"#
    ));
    assert_eq!(
        lignes[5],
        r#"{"rep":"erreur","erreur":{"type":"repos_incertain","remise_au_repos":{"etat":"arret_refuse","code":-9986}}}"#
    );
    assert_eq!(armements(&session), 1);
}

#[test]
fn la_fin_de_l_entree_ferme_la_session_comme_fermer() {
    let (lignes, session) = dialoguer_brut(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &JUSQU_A_LA_CONNEXION,
    );
    assert_eq!(lignes.len(), 3, "aucune réponse sans requête");
    let journal = session.journal().join("\n");
    assert!(
        journal.contains("résultat de la fermeture : Ok(Incertaine"),
        "{journal}"
    );
    assert_eq!(session.sdk().appels.last().unwrap(), "deconnecter");
    assert_eq!(appels(&session, "deconnecter"), 1);
}

#[test]
fn apres_ferme_la_fin_de_l_entree_ne_rappelle_pas_la_dll() {
    let (_, session) = dialoguer_brut(
        SdkSimule::avec_un_myiro1(),
        Palier::Connexion,
        &sequence(&[r#"{"cmd":"fermer"}"#]),
    );
    assert_eq!(appels(&session, "deconnecter"), 1);
    assert_eq!(appels(&session, "arreter"), 1);
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
    let Reponse::Etalonne { date } = &reponses[3] else {
        panic!("{:?}", reponses[3])
    };
    let Reponse::Mesure { mesure, .. } = &reponses[4] else {
        panic!("{:?}", reponses[4])
    };
    // La date rendue à l'étalonnage est celle de la provenance des mesures.
    assert_eq!(
        mesure.provenance().etalonnage,
        pont_protocole::Info::Confirmee(date.clone())
    );
    let Reponse::Mesure { mesure, .. } = &reponses[4] else {
        panic!("{:?}", reponses[4])
    };
    let plages = mesure.plages();
    assert_eq!(plages.len(), 12);
    assert_eq!(plages[0].m1().len(), 36);
    assert_eq!(plages[0].brutes().len(), 152);
}
