//! « Annuler » pendant la mesure des plages : l'annulation est lue pendant
//! l'attente de la fin de mesure comme pendant celle de la feuille ; la
//! mesure est arrêtée (`FD9_StopMeasurement`, événement 10, sortie) et la
//! feuille rendue annulée, perdue. FD-9 simulé, valeurs fictives.

mod commun;

use commun::{Fd9Simule, Programme};
use pont_fd9::{Delais, Session};
use pont_protocole::{
    AdresseReseau, DefinitionFeuille, ErreurPont, Palier, RemiseAuRepos, ZoneFeuille,
};
use std::time::Duration;

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

#[test]
fn annulee_pendant_la_mesure_la_mesure_est_arretee_et_la_feuille_annulee() {
    // La mesure dure : un avis, puis la fin bien après l'annulation.
    let sdk = Fd9Simule {
        mesure_programmee: Some(vec![Programme::avis(0, 2), Programme::evenement(3_000, 9)]),
        ..Fd9Simule::avec_une_rgb2520()
    };
    let mut session = Session::new(sdk, Palier::Feuille).avec_delais(Delais {
        feuille: Duration::from_secs(5),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        silence: Duration::from_secs(60),
        deconnexion: Duration::from_millis(50),
    });
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    // L'opérateur annule dès que le pont attend sans événement : ici, pendant
    // la mesure des plages (la feuille et ses étapes arrivent sans attente).
    session.sdk_mut().annuler_pendant_l_attente = Some(session.annulation());

    let erreur = session.mesurer_feuille(&six_plages()).unwrap_err();

    assert_eq!(
        erreur,
        ErreurPont::MesureAnnulee {
            remise_au_repos: RemiseAuRepos::AuRepos {},
        }
    );
    let appels = session.sdk().appels_dll();
    let lancement = appels
        .iter()
        .position(|a| *a == "FD9_StartMeasurement2 1 1")
        .unwrap();
    let apres: Vec<&str> = appels[lancement + 1..].to_vec();
    assert_eq!(
        apres,
        ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"],
        "{appels:?}"
    );
    assert!(session.journal().iter().any(|l| l == "annulation reçue"));
}
