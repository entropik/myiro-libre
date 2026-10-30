//! Trace des appels à la DLL (`--trace`) : chaque appel est écrit avant
//! d'entrer dans la DLL, puis son retour. Un plantage dans la DLL laisse donc
//! sa dernière entrée sans retour (essai E3).

mod commun;

use commun::Fd9Simule;
use fd9_sys::{CoordonneesMire, ModeReconnaissance};
use pont_fd9::trace::SdkTrace;
use pont_fd9::{Delais, SdkFd9, Session};
use pont_protocole::{AdresseReseau, DefinitionFeuille, Palier, ZoneFeuille};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn rgb2520() -> DefinitionFeuille {
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
            rangees: 60,
            colonnes: 42,
        }],
        couleurs: Some(commun::couleurs_fictives(2_520)),
    }
}

/// Lignes de trace partagées avec le test.
fn capteur() -> (Arc<Mutex<Vec<String>>>, impl FnMut(&str) + 'static) {
    let lignes = Arc::new(Mutex::new(Vec::new()));
    let copie = lignes.clone();
    (lignes, move |l: &str| {
        copie.lock().unwrap().push(l.to_string())
    })
}

#[test]
fn chaque_appel_est_trace_a_l_entree_puis_au_retour() {
    let (lignes, sortie) = capteur();
    let sdk = SdkTrace::new(Fd9Simule::avec_une_rgb2520(), sortie);
    let mut session = Session::new(sdk, Palier::Prenumerisation).avec_delais(Delais {
        feuille: Duration::from_millis(50),
        etape: Duration::from_millis(50),
        mesure: Duration::from_millis(50),
        deconnexion: Duration::from_millis(50),
    });
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();

    session.prenumeriser(&rgb2520()).unwrap();

    let lignes = lignes.lock().unwrap();
    let entree = lignes
        .iter()
        .position(|l| l.contains("→ FD9_StartPatchRecognition(mode 4"))
        .unwrap_or_else(|| panic!("{lignes:#?}"));
    assert!(
        lignes[entree].contains("zones 1, plages 2520"),
        "{}",
        lignes[entree]
    );
    assert!(lignes[entree + 1].contains("← FD9_StartPatchRecognition : 0"));
    // Autant de retours que d'entrées, et l'adresse de l'instrument n'est
    // jamais écrite.
    let entrees = lignes.iter().filter(|l| l.contains("→ ")).count();
    let retours = lignes.iter().filter(|l| l.contains("← ")).count();
    assert_eq!(entrees, retours, "{lignes:#?}");
    assert!(lignes.iter().all(|l| !l.contains("192.0.2.40")));
    // La trace ne change rien à ce que la DLL reçoit.
    assert!(session
        .sdk()
        .interieur()
        .appels
        .iter()
        .any(|a| a.starts_with("FD9_StartPatchRecognition mode 4")));
}

#[test]
fn un_plantage_dans_la_dll_laisse_l_entree_sans_retour() {
    // Le FD-9 simulé s'arrête comme la DLL de l'essai E3 sur COMBINE sans
    // couleurs : la trace a déjà écrit l'appel.
    let (lignes, sortie) = capteur();
    let mut sdk = SdkTrace::new(Fd9Simule::avec_une_rgb2520(), sortie);
    let coordonnees = CoordonneesMire::depuis_definition(&rgb2520()).unwrap();

    let plantage = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sdk.reconnaitre(ModeReconnaissance::Combine, &coordonnees, None)
    }));

    assert!(plantage.is_err());
    let lignes = lignes.lock().unwrap();
    assert!(lignes
        .last()
        .unwrap()
        .contains("→ FD9_StartPatchRecognition(mode 4"));
}
