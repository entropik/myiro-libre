//! Garde d'une feuille (essai E5) : si la DLL ne rend pas la main pendant
//! une feuille (mesure, arrêt), le pont répond quand même `feuille_bloquee`
//! dans un délai borné, puis l'exécutable s'arrête de force. Ici, la garde
//! ne termine pas le processus : elle libère la DLL simulée, pour vérifier
//! que la boucle n'écrit rien d'autre pour cette feuille. FD-9 simulé.

mod commun;

use commun::Fd9Simule;
use pont_fd9::serveur::{servir, Garde};
use pont_fd9::{Delais, Session};
use pont_protocole::{lire_reponse, ErreurPont, Palier, Reponse};
use std::sync::mpsc;
use std::time::Duration;

fn mire() -> String {
    let couleurs: Vec<String> = commun::couleurs_fictives(6)
        .iter()
        .map(|[r, v, b]| format!("[{r},{v},{b}]"))
        .collect();
    format!(
        r#"{{"nom":"RGB-2520 - 1P MY9","largeur":27940,"hauteur":41660,"zones":[{{"gauche":1349,"haut":2300,"largeur_plage":601,"hauteur_plage":601,"pas_x":601,"pas_y":601,"rangees":2,"colonnes":3}}],"couleurs":[{}]}}"#,
        couleurs.join(",")
    )
}

/// Version, connexion, une feuille, puis fin de l'entrée. `liberer` rend
/// la main à la DLL simulée retenue, quand la garde agit. Rend les réponses
/// finales écrites par la boucle (sans l'avancement), ce que la garde a
/// écrit, et la session.
fn feuille_gardee(
    sdk: Fd9Simule,
    liberer: Option<mpsc::Sender<()>>,
    delai: Duration,
) -> (Vec<Reponse>, Vec<Reponse>, Session<Fd9Simule>) {
    let mut session = Session::new(sdk, Palier::Feuille).avec_delais(Delais {
        feuille: Duration::from_secs(5),
        etape: Duration::from_secs(5),
        mesure: Duration::from_secs(5),
        silence: Duration::from_secs(60),
        deconnexion: Duration::from_millis(50),
    });
    let (envoi, recu) = mpsc::channel();
    let liberer = std::sync::Mutex::new(liberer);
    let garde = Garde::new(Duration::from_secs(60), |_| {}).avec_feuille(delai, move |reponse| {
        envoi.send(reponse).unwrap();
        // En vrai, le processus s'arrête ici.
        if let Some(liberer) = liberer.lock().unwrap().take() {
            let _ = liberer.send(());
        }
    });
    let requetes = [
        r#"{"cmd":"version"}"#.to_string(),
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#.to_string(),
        format!(r#"{{"cmd":"mesurer_feuille","mire":{}}}"#, mire()),
    ];
    let mut sortie = Vec::new();
    servir(
        &mut session,
        std::io::Cursor::new(requetes.join("\n").into_bytes()),
        &mut sortie,
        garde,
    )
    .unwrap();
    let finales = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .map(|l| lire_reponse(l).unwrap())
        .filter(|r| !matches!(r, Reponse::Avancement { .. }))
        .collect();
    (finales, recu.try_iter().collect(), session)
}

const BLOQUEE: Reponse = Reponse::Erreur {
    erreur: ErreurPont::FeuilleBloquee {},
};

/// `FD9_StartMeasurement2` ne rend pas la main : la garde répond
/// `feuille_bloquee`, une seule fois ; la mesure rendue plus tard n'est pas
/// écrite.
#[test]
fn une_mesure_qui_ne_rend_pas_la_main_recoit_feuille_bloquee() {
    let (liberer, retenue) = mpsc::channel();
    let sdk = Fd9Simule {
        mesure_retenue: Some(retenue),
        ..Fd9Simule::avec_une_rgb2520()
    };

    let (finales, garde, session) = feuille_gardee(sdk, Some(liberer), Duration::from_millis(100));

    assert_eq!(garde, [BLOQUEE]);
    // Version et connexion seulement : rien d'autre pour la feuille.
    assert_eq!(finales.len(), 2, "{finales:?}");
    assert!(session
        .journal()
        .iter()
        .any(|l| l.starts_with("feuille rendue après le délai de garde : Some(Mesure")));
}

/// L'arrêt d'une feuille (autre mire) ne rend pas la main : même issue.
#[test]
fn un_arret_qui_ne_rend_pas_la_main_recoit_feuille_bloquee() {
    let (liberer, retenu) = mpsc::channel();
    let sdk = Fd9Simule {
        arret_retenu: Some(retenu),
        qr_texte: "RGB-1617 - 1P MY9".into(),
        ..Fd9Simule::avec_une_rgb2520()
    };

    let (finales, garde, session) = feuille_gardee(sdk, Some(liberer), Duration::from_millis(100));

    assert_eq!(garde, [BLOQUEE]);
    assert_eq!(finales.len(), 2, "{finales:?}");
    assert!(session
        .sdk()
        .appels_dll()
        .iter()
        .any(|a| a.starts_with("FD9_StopMeasurement")));
}

/// Une feuille qui rend la main à temps désarme la garde : sa réponse est
/// celle de la boucle, la garde n'agit pas.
#[test]
fn une_feuille_rendue_a_temps_desarme_la_garde() {
    let (finales, garde, _) =
        feuille_gardee(Fd9Simule::avec_une_rgb2520(), None, Duration::from_secs(5));

    assert!(garde.is_empty(), "{garde:?}");
    assert_eq!(finales.len(), 3);
    assert!(
        matches!(finales[2], Reponse::Mesure { .. }),
        "{:?}",
        finales[2]
    );
}

/// Par défaut, la garde d'une feuille attend 28 min : toutes ses attentes,
/// plus 4 min, et moins que les 30 min que l'application lui laisse.
#[test]
fn la_garde_d_une_feuille_repond_avant_l_application() {
    let delais = Delais::default();
    assert_eq!(delais.garde_feuille(), Duration::from_secs(28 * 60));
    assert!(delais.garde_feuille() > delais.feuille + delais.etape * 4 + delais.mesure);
    assert!(delais.garde_feuille() < Duration::from_secs(30 * 60));
}
