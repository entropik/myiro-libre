//! Annuler une mesure en attente (ticket #26), contre un SDK simulé.
//!
//! L'annulation n'est prise en compte que tant que la mesure attend l'appui
//! sur le bouton (ou, en automatique, l'attente de mesure avant le
//! déclenchement). Une mesure partie (événement 2) ou terminée est gardée.
//! L'instrument est ensuite désarmé par la politique unique du pont, et le
//! résultat du retour au repos est rapporté.

mod commun;

use commun::{evenement, SdkSimule};
use pont_myiro1::{EtatInstrument, Evenement, Session};
use pont_protocole::{Declenchement, ErreurPont, Palier, RemiseAuRepos};

fn salve(codes: &[i32]) -> Vec<Evenement> {
    codes.iter().map(|&c| evenement(c)).collect()
}

/// Session étalonnée ; `a_l_armement` est émis au premier armement.
fn session(a_l_armement: &[i32]) -> Session<SdkSimule> {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend(salve(&[7, 8]));
    sdk.salves.push_back(salve(a_l_armement));
    let mut session = Session::new(sdk, Palier::MesurePonctuelle);
    session.version().unwrap();
    session.detecter().unwrap();
    session.connecter(0).unwrap();
    session.etalonner().unwrap();
    session
}

/// L'opérateur annulera la première fois que le pont attend sans événement.
fn annuler_pendant_l_attente(session: &mut Session<SdkSimule>) {
    let annulation = session.annulation();
    session.sdk_mut().annuler_pendant_attente = Some(annulation);
}

fn appels(session: &Session<SdkSimule>) -> Vec<&str> {
    session.sdk().appels.iter().map(String::as_str).collect()
}

fn a_lu(session: &Session<SdkSimule>) -> bool {
    appels(session).iter().any(|a| a.starts_with("lire"))
}

#[test]
fn une_mesure_qui_attend_le_bouton_s_annule_et_le_repos_est_verifie() {
    let mut s = session(&[1]);
    s.sdk_mut().repos_a_l_arret = true;
    annuler_pendant_l_attente(&mut s);

    assert_eq!(
        s.mesurer_ponctuelle(),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {}
        })
    );
    assert!(!a_lu(&s), "rien n'est lu : {:?}", appels(&s));
    assert_eq!(*appels(&s).last().unwrap(), "arreter");
    // L'annulation ne touche pas à l'étalonnage.
    assert!(matches!(s.etat(), EtatInstrument::Etalonne { .. }));
}

#[test]
fn sans_retour_au_repos_l_annulation_le_dit_et_bloque_la_mesure_suivante() {
    let mut s = session(&[1]);
    annuler_pendant_l_attente(&mut s);

    assert_eq!(
        s.mesurer_ponctuelle(),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::ReposNonSignale {}
        })
    );
    // Une nouvelle mesure n'est permise qu'une fois le repos rétabli.
    assert!(matches!(
        s.mesurer_ponctuelle(),
        Err(ErreurPont::ReposIncertain { .. })
    ));
}

#[test]
fn une_annulation_demandee_avant_l_armement_n_arme_pas() {
    let mut s = session(&[1, 2, 3]);
    s.annulation().annuler();

    assert_eq!(
        s.mesurer_ponctuelle(),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::ReposSuppose {}
        })
    );
    assert!(!appels(&s).iter().any(|a| a.starts_with("armer")));
}

/// Course entre la fin de la mesure et l'annulation : l'événement 2 est reçu,
/// la mesure est partie, elle est lue et gardée avec sa provenance.
#[test]
fn une_mesure_deja_partie_est_gardee_malgre_l_annulation() {
    let mut s = session(&[1, 2]);
    annuler_pendant_l_attente(&mut s);
    s.sdk_mut().evenements_apres_annulation = salve(&[3]);

    let mesure = s.mesurer_ponctuelle().expect("mesure gardée");
    assert_eq!(mesure.m0, vec![10.0; 36]);
}

#[test]
fn en_automatique_l_annulation_avant_l_attente_de_mesure_ne_declenche_jamais() {
    let mut s = session(&[]);
    s.sdk_mut().repos_a_l_arret = true;
    annuler_pendant_l_attente(&mut s);

    assert_eq!(
        s.mesurer_ponctuelle_avec(Declenchement::Automatique),
        Err(ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {}
        })
    );
    assert!(!appels(&s).contains(&"declencher"));
}

// La boucle du pont : les demandes arrivent pendant que la mesure attend.

use pont_myiro1::serveur::servir;
use pont_protocole::{lire_reponse, EffetAnnulation, Reponse};
use std::io::Write;
use std::time::{Duration, Instant};

const JUSQU_A_L_ETALONNAGE: [&str; 4] = [
    r#"{"cmd":"version"}"#,
    r#"{"cmd":"detecter"}"#,
    r#"{"cmd":"connecter","instrument":0}"#,
    r#"{"cmd":"etalonner"}"#,
];
const MESURER: &str = r#"{"cmd":"mesurer_ponctuelle"}"#;
const ANNULER: &str = r#"{"cmd":"annuler"}"#;

/// Envoie au pont les groupes de lignes, `pause` entre deux groupes, puis
/// ferme son entrée. Rend les réponses (sans celles jusqu'à l'étalonnage), la
/// session et la durée du dialogue.
fn dialoguer_au_fil_du_temps(
    sdk: SdkSimule,
    groupes: Vec<Vec<&'static str>>,
    pause: Duration,
) -> (Vec<Reponse>, Session<SdkSimule>, Duration) {
    let (lecture, mut ecriture) = std::io::pipe().unwrap();
    let ecrivain = std::thread::spawn(move || {
        for (n, groupe) in groupes.into_iter().enumerate() {
            if n > 0 {
                std::thread::sleep(pause);
            }
            for ligne in groupe {
                writeln!(ecriture, "{ligne}").unwrap();
            }
        }
    });
    let mut session = Session::new(sdk, Palier::MesurePonctuelle);
    let mut sortie = Vec::new();
    let debut = Instant::now();
    servir(&mut session, std::io::BufReader::new(lecture), &mut sortie).unwrap();
    let duree = debut.elapsed();
    ecrivain.join().unwrap();
    let reponses = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .skip(JUSQU_A_L_ETALONNAGE.len())
        .map(|l| lire_reponse(l).unwrap())
        .collect();
    (reponses, session, duree)
}

/// Instrument armé qui attend vraiment un appui qui ne vient pas.
fn sdk_qui_attend_le_bouton() -> SdkSimule {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend(salve(&[7, 8]));
    sdk.salves.push_back(salve(&[1]));
    sdk.repos_a_l_arret = true;
    sdk.attente_reelle = true;
    sdk
}

fn avec_etalonnage(suite: &[&'static str]) -> Vec<&'static str> {
    JUSQU_A_L_ETALONNAGE.iter().chain(suite).copied().collect()
}

#[test]
fn le_pont_lit_annuler_pendant_que_la_mesure_attend_et_repond_dans_l_ordre() {
    let (reponses, session, duree) = dialoguer_au_fil_du_temps(
        sdk_qui_attend_le_bouton(),
        vec![avec_etalonnage(&[MESURER]), vec![ANNULER, MESURER], vec![]],
        Duration::from_millis(150),
    );

    assert_eq!(reponses.len(), 3, "{reponses:?}");
    assert_eq!(
        reponses[..2],
        [
            Reponse::Erreur {
                erreur: ErreurPont::MesureAnnulee {
                    remise_au_repos: RemiseAuRepos::AuRepos {}
                }
            },
            Reponse::Annulation {
                effet: EffetAnnulation::Appliquee
            },
        ]
    );
    // L'annulation ne visait que la première mesure : la suivante part, et
    // attend à son tour jusqu'à la fin de l'entrée, qui l'annule.
    assert!(matches!(
        reponses[2],
        Reponse::Erreur {
            erreur: ErreurPont::MesureAnnulee { .. }
        }
    ));
    assert_eq!(
        session
            .sdk()
            .appels
            .iter()
            .filter(|a| *a == "armer ponctuelle")
            .count(),
        2
    );
    assert!(duree < Duration::from_secs(5), "{duree:?}");
}

/// Résultat tardif : la mesure s'est terminée avant que `annuler` arrive. Elle
/// reste la réponse de sa propre demande ; `annuler` est sans effet, et la
/// mesure suivante reçoit sa propre réponse.
#[test]
fn un_resultat_arrive_avant_l_annulation_reste_celui_de_sa_mesure() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.evenements.extend(salve(&[7, 8]));
    sdk.salves.push_back(salve(&[1, 2, 3, 0]));
    sdk.salves.push_back(salve(&[1, 2, 3, 0]));
    // L'entrée reste ouverte le temps de la seconde mesure.
    let (reponses, _, _) = dialoguer_au_fil_du_temps(
        sdk,
        vec![avec_etalonnage(&[MESURER]), vec![ANNULER, MESURER], vec![]],
        Duration::from_millis(150),
    );

    assert_eq!(reponses.len(), 3, "{reponses:?}");
    assert!(matches!(reponses[0], Reponse::Mesure { .. }));
    assert_eq!(
        reponses[1],
        Reponse::Annulation {
            effet: EffetAnnulation::SansEffet
        }
    );
    assert!(matches!(reponses[2], Reponse::Mesure { .. }));
}

#[test]
fn annuler_sans_mesure_en_cours_est_sans_effet() {
    let (reponses, session, _) = dialoguer_au_fil_du_temps(
        sdk_qui_attend_le_bouton(),
        vec![avec_etalonnage(&[ANNULER])],
        Duration::ZERO,
    );
    assert_eq!(
        reponses,
        [Reponse::Annulation {
            effet: EffetAnnulation::SansEffet
        }]
    );
    assert!(!appels(&session).iter().any(|a| a.starts_with("armer")));
}

/// L'application disparaît (fin de l'entrée) pendant que la mesure attend :
/// le pont l'annule, désarme, puis ferme la session sans attendre le délai
/// de l'appui.
#[test]
fn la_fin_de_l_entree_annule_la_mesure_en_attente_puis_ferme() {
    let (reponses, session, duree) = dialoguer_au_fil_du_temps(
        sdk_qui_attend_le_bouton(),
        vec![avec_etalonnage(&[MESURER])],
        Duration::ZERO,
    );
    assert!(matches!(
        reponses[..],
        [Reponse::Erreur {
            erreur: ErreurPont::MesureAnnulee { .. }
        }]
    ));
    assert_eq!(*appels(&session).last().unwrap(), "deconnecter");
    assert!(duree < Duration::from_secs(5), "{duree:?}");
}

#[test]
fn apres_une_annulation_verifiee_une_nouvelle_mesure_part() {
    let mut s = session(&[1]);
    s.sdk_mut().repos_a_l_arret = true;
    annuler_pendant_l_attente(&mut s);
    assert!(s.mesurer_ponctuelle().is_err());

    s.sdk_mut().salves.push_back(salve(&[1, 2, 3]));
    let suivante = s.mesurer_ponctuelle();
    assert!(suivante.is_ok(), "{suivante:?}");
}
