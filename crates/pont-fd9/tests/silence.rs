//! Silence du rappel pendant la mesure (essai E5) : la DLL a envoyé 976
//! événements 11 puis plus rien. Après `Delais::silence` sans événement, le
//! pont relit l'état (`FD9_GetStatus`, sans question à l'instrument) au lieu
//! d'attendre à l'aveugle. FD-9 simulé, valeurs fictives.

mod commun;

use commun::{Fd9Simule, Programme};
use pont_fd9::{Delais, Session};
use pont_protocole::{AdresseReseau, DefinitionFeuille, ErreurPont, Palier, ZoneFeuille};
use std::time::Duration;

/// Une petite mire de 2 rangées × 3 colonnes, au pas de la RGB-2520.
fn six_plages() -> DefinitionFeuille {
    DefinitionFeuille {
        nom: "RGB-2520 - 1P MY9".into(),
        largeur: 27_940,
        hauteur: 41_660,
        zones: vec![ZoneFeuille {
            gauche: 1_349,
            haut: 2_300,
            largeur_plage: 601,
            hauteur_plage: 601,
            pas_x: 601,
            pas_y: 601,
            rangees: 2,
            colonnes: 3,
        }],
        couleurs: Some(commun::couleurs_fictives(6)),
    }
}

fn delais(silence: Duration) -> Delais {
    Delais {
        feuille: Duration::from_secs(5),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        silence,
        deconnexion: Duration::from_millis(50),
    }
}

/// Session connectée à un FD-9 simulé dont la mesure suit `programme`.
fn connectee(programme: Vec<Programme>, silence: Duration) -> Session<Fd9Simule> {
    let sdk = Fd9Simule {
        mesure_programmee: Some(programme),
        ..Fd9Simule::avec_une_rgb2520()
    };
    connectee_avec(sdk, silence)
}

fn connectee_avec(sdk: Fd9Simule, silence: Duration) -> Session<Fd9Simule> {
    let mut session = Session::new(sdk, Palier::Feuille).avec_delais(delais(silence));
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    session
}

/// Appels à la DLL après le lancement de la mesure.
fn apres_lancement(session: &Session<Fd9Simule>) -> Vec<String> {
    let appels = &session.sdk().appels;
    let lancement = appels
        .iter()
        .position(|a| a.starts_with("FD9_StartMeasurement2"))
        .expect("mesure lancée");
    appels[lancement + 1..].to_vec()
}

fn relectures(session: &Session<Fd9Simule>) -> usize {
    apres_lancement(session)
        .iter()
        .filter(|a| *a == "FD9_GetStatus")
        .count()
}

fn lignes_silence(session: &Session<Fd9Simule>) -> Vec<&String> {
    session
        .journal()
        .iter()
        .filter(|l| l.starts_with("silence de "))
        .collect()
}

/// Les avis de plages s'arrêtent, la DLL passe à l'état 9 sans le dire :
/// l'état relu suffit, les plages sont lues et la mesure rendue.
#[test]
fn une_mesure_finie_sans_evenement_9_est_lue_apres_un_silence() {
    let mut session = connectee(
        vec![
            Programme::avis(0, 2),
            Programme::avis(10, 4),
            Programme::muet(20, 9),
        ],
        Duration::from_millis(100),
    );

    let mesure = session.mesurer_feuille(&six_plages()).unwrap();

    assert_eq!(mesure.plages().len(), 6);
    assert_eq!(relectures(&session), 1);
    let silences = lignes_silence(&session);
    assert_eq!(silences.len(), 1);
    assert!(
        silences[0].contains("pendant la mesure : FD9_GetStatus code 0, état 9"),
        "{}",
        silences[0]
    );
    // La lecture des plages suit la relecture, puis la sortie.
    let apres = apres_lancement(&session);
    assert_eq!(apres[0], "FD9_GetStatus");
    assert!(apres.iter().any(|a| a == "FD9_GetData 5"));
    assert!(apres.iter().any(|a| a == "FD9_ExitMeasurement 0 nul"));
    assert_eq!(session.palier_atteint(), Some(Palier::Feuille));
}

/// Silence, puis l'état relu est une erreur : incident, comme s'il était
/// arrivé par le rappel.
#[test]
fn un_etat_d_erreur_relu_apres_un_silence_est_un_incident() {
    let mut session = connectee(
        vec![Programme::avis(0, 2), Programme::muet(10, 202)],
        Duration::from_millis(100),
    );

    let erreur = session.mesurer_feuille(&six_plages()).unwrap_err();

    assert_eq!(
        erreur,
        ErreurPont::IncidentFeuille {
            etat: 202,
            premesure: None
        }
    );
    let apres = apres_lancement(&session);
    assert_eq!(apres[0], "FD9_GetStatus");
    // La feuille sort sans arrêt et sans lecture de plages.
    assert!(apres.iter().any(|a| a == "FD9_ExitMeasurement 0 nul"));
    assert!(!apres.iter().any(|a| a.starts_with("FD9_GetData")));
    assert!(!apres.iter().any(|a| a.starts_with("FD9_StopMeasurement")));
    assert!(lignes_silence(&session)[0].contains("état 202"));
}

/// Silence, l'état relu dit la mesure en cours (8) : l'attente continue,
/// relue à chaque nouveau silence, jusqu'à l'événement 9.
#[test]
fn une_mesure_encore_en_cours_continue_d_etre_attendue() {
    let mut session = connectee(
        vec![
            Programme::avis(0, 2),
            Programme::avis(10, 4),
            Programme::evenement(350, 9),
        ],
        Duration::from_millis(100),
    );

    let mesure = session.mesurer_feuille(&six_plages()).unwrap();

    assert_eq!(mesure.plages().len(), 6);
    let silences = lignes_silence(&session);
    assert!(!silences.is_empty());
    assert_eq!(silences.len(), relectures(&session));
    assert!(
        silences.iter().all(|l| l.contains("état 8")),
        "{silences:?}"
    );
    // La relecture précède l'événement 9, qui conclut la mesure.
    let journal = session.journal();
    let relue = journal
        .iter()
        .position(|l| l.starts_with("silence de "))
        .unwrap();
    let fin = journal
        .iter()
        .position(|l| l.starts_with("événement : état 9 "))
        .unwrap();
    assert!(relue < fin);
}

/// Tant que les avis arrivent, l'état n'est jamais relu.
#[test]
fn aucune_relecture_tant_que_les_evenements_arrivent() {
    let mut programme: Vec<Programme> = (1..=25).map(|k| Programme::avis(k * 20, 1)).collect();
    programme.push(Programme::evenement(520, 9));
    let mut session = connectee(programme, Duration::from_millis(300));

    session.mesurer_feuille(&six_plages()).unwrap();

    assert_eq!(relectures(&session), 0);
    assert!(lignes_silence(&session).is_empty());
}

/// Une lecture de l'état refusée est notée, et l'attente continue jusqu'à
/// la relecture suivante.
#[test]
fn une_relecture_refusee_est_notee_et_l_attente_continue() {
    let mut session = connectee(
        vec![Programme::avis(0, 6), Programme::muet(10, 9)],
        Duration::from_millis(100),
    );
    // Le premier `FD9_GetStatus` après la connexion est celui du début de
    // la feuille.
    session.sdk_mut().codes_statut = [0, 1999].into();

    let mesure = session.mesurer_feuille(&six_plages()).unwrap();

    assert_eq!(mesure.plages().len(), 6);
    let silences = lignes_silence(&session);
    assert_eq!(silences.len(), 2, "{silences:?}");
    assert!(silences[0].contains("FD9_GetStatus code 1999 ; l'attente continue"));
    assert!(silences[1].contains("code 0, état 9"));
}

/// Par défaut, 60 s de silence avant de relire, et 20 min pour la mesure.
#[test]
fn delais_par_defaut_de_la_mesure() {
    let delais = Delais::default();
    assert_eq!(delais.silence, Duration::from_secs(60));
    assert_eq!(delais.mesure, Duration::from_secs(20 * 60));
}
