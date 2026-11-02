//! Ce que le pont écrit sur sa sortie d'erreur (journal recopié, trace)
//! part dans un fichier local de l'application : adresse, identifiant,
//! n° de série et adresse MAC de l'instrument y sont masqués. Valeurs
//! fictives (adresse 192.0.2.40, n° 12345678).

mod commun;

use commun::{fd9_reseau, Fd9Simule};
use pont_fd9::trace::SdkTrace;
use pont_fd9::{Masque, Session};
use pont_protocole::{AdresseReseau, Palier};
use std::sync::{Arc, Mutex};

const SECRETS: [&str; 3] = ["192.0.2.40", "12345678", "02:00:00:00:00:01"];

fn capteur() -> (Arc<Mutex<Vec<String>>>, impl FnMut(&str) + 'static) {
    let lignes = Arc::new(Mutex::new(Vec::new()));
    let copie = lignes.clone();
    (lignes, move |l: &str| {
        copie.lock().unwrap().push(l.to_string())
    })
}

#[test]
fn un_identifiant_retenu_est_remplace_par_son_nom() {
    let masque = Masque::default();
    masque.retenir("192.0.2.40", "adresse");
    masque.retenir("12345678\0\0", "n° de série");
    // Trop court pour être distingué d'un nombre ordinaire : ignoré.
    masque.retenir("COM", "adresse");

    assert_eq!(
        masque.appliquer("FD9_Connect 192.0.2.40, n° 12345678, port COM4"),
        "FD9_Connect <adresse>, n° <n° de série>, port COM4"
    );
}

/// Une connexion par l'adresse retient l'adresse, le n° de série et la
/// MAC ; toute ligne du journal recopiée ensuite les masque, même si elle
/// les cite. Le journal gardé en mémoire n'est pas touché.
#[test]
fn le_journal_recopie_ne_contient_aucun_identifiant() {
    let (lignes, echo) = capteur();
    let mut session =
        Session::new(Fd9Simule::avec_une_rgb2520(), Palier::Connexion).avec_echo_journal(echo);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let masque = session.masque();
    let cite = masque.appliquer(&format!("ligne qui cite {}", SECRETS.join(" et ")));

    assert_eq!(cite, "ligne qui cite <adresse> et <n° de série> et <MAC>");
    let lignes = lignes.lock().unwrap();
    assert!(!lignes.is_empty());
    for ligne in lignes.iter() {
        for secret in SECRETS {
            assert!(!ligne.contains(secret), "{ligne}");
        }
    }
}

/// Les instruments détectés sont retenus dès la détection.
#[test]
fn une_detection_retient_les_instruments_vus() {
    let mut session = Session::new(Fd9Simule::avec_une_rgb2520(), Palier::Detection);
    session.version().unwrap();
    session.detecter().unwrap();

    let adresse = fd9_reseau().adresse();
    assert_eq!(
        session.masque().appliquer(&format!("vu à {adresse}")),
        "vu à <adresse>"
    );
}

/// La trace, masquée comme le fait l'exécutable, ne cite aucun identifiant.
#[test]
fn la_trace_masquee_ne_contient_aucun_identifiant() {
    let masque = Masque::default();
    let (lignes, sortie) = capteur();
    let mut sortie = sortie;
    let m = masque.clone();
    let sdk = SdkTrace::new(Fd9Simule::avec_une_rgb2520(), move |l| {
        sortie(&m.appliquer(l))
    });
    let mut session = Session::new(sdk, Palier::Connexion).avec_masque(masque.clone());
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    session.fermer();

    let lignes = lignes.lock().unwrap();
    assert!(lignes.iter().any(|l| l.contains("FD9_Connect")));
    for ligne in lignes.iter() {
        for secret in SECRETS {
            assert!(!ligne.contains(secret), "{ligne}");
        }
    }
}
