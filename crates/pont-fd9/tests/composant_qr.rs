//! Composant de lecture du QR (ticket #87) : à la lecture du QR, la DLL du
//! fabricant charge l'assembly .NET `zxing` depuis le dossier du pont. Sans
//! elle, le pont s'arrêtait net, feuille engagée (essai E3). Le pont vérifie
//! sa présence avant de lancer une feuille et refuse en le disant.

mod commun;

use commun::Fd9Simule;
use pont_fd9::composants::{composant_qr, dossier_de_l_executable, COMPOSANT_QR};
use pont_fd9::{Delais, Session};
use pont_protocole::{AdresseReseau, DefinitionFeuille, ErreurPont, Palier, ZoneFeuille};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Dossier vide propre à un test, sous le dossier temporaire du système,
/// effacé à la fin du test.
struct DossierTemporaire(PathBuf);

impl std::ops::Deref for DossierTemporaire {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for DossierTemporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dossier(nom: &str) -> DossierTemporaire {
    let dossier =
        std::env::temp_dir().join(format!("pont-fd9-composant-{}-{nom}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    DossierTemporaire(dossier)
}

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

const COURTS: Delais = Delais {
    feuille: Duration::from_millis(50),
    etape: Duration::from_millis(50),
    mesure: Duration::from_millis(50),
    deconnexion: Duration::from_millis(50),
};

fn connectee(plafond: Palier, dossier_du_pont: PathBuf) -> (Session<Fd9Simule>, usize) {
    let mut session = Session::new(Fd9Simule::avec_une_rgb2520(), plafond)
        .avec_delais(COURTS)
        .avec_dossier_du_pont(dossier_du_pont);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let appels = session.sdk().appels.len();
    (session, appels)
}

#[test]
fn le_composant_est_cherche_a_cote_du_pont() {
    assert_eq!(COMPOSANT_QR, "zxing.dll");
    let vide = dossier("cherche-vide");
    assert_eq!(composant_qr(&vide), Err(vide.join("zxing.dll")));
    let plein = dossier("cherche-plein");
    std::fs::write(plein.join("zxing.dll"), b"assembly factice").unwrap();
    assert_eq!(composant_qr(&plein), Ok(plein.join("zxing.dll")));
    // Un dossier du même nom n'est pas le composant.
    let leurre = dossier("cherche-leurre");
    std::fs::create_dir(leurre.join("zxing.dll")).unwrap();
    assert!(composant_qr(&leurre).is_err());
}

#[test]
fn sans_zxing_la_prenumerisation_est_refusee_sans_appel_a_la_dll() {
    let vide = dossier("prenumerisation");
    let (mut session, depuis) = connectee(Palier::Prenumerisation, vide.to_path_buf());

    let erreur = session.prenumeriser(&rgb2520()).unwrap_err();

    assert_eq!(
        erreur,
        ErreurPont::ComposantQrIntrouvable {
            chemin: vide.join("zxing.dll").display().to_string()
        }
    );
    // Rien n'est lancé : ni lecture de l'état, ni capture.
    assert_eq!(session.sdk().appels.len(), depuis);
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("zxing.dll") && l.contains("QR")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn sans_zxing_la_mesure_d_une_feuille_est_refusee_sans_appel_a_la_dll() {
    let vide = dossier("feuille");
    let (mut session, depuis) = connectee(Palier::Feuille, vide.to_path_buf());

    let erreur = session.mesurer_feuille(&rgb2520()).unwrap_err();

    assert!(
        matches!(erreur, ErreurPont::ComposantQrIntrouvable { .. }),
        "{erreur:?}"
    );
    assert_eq!(session.sdk().appels.len(), depuis);
}

#[test]
fn avec_zxing_la_feuille_part() {
    let plein = dossier("present");
    std::fs::write(plein.join("zxing.dll"), b"assembly factice").unwrap();
    let (mut session, depuis) = connectee(Palier::Prenumerisation, plein.to_path_buf());

    session.prenumeriser(&rgb2520()).unwrap();

    assert!(session.sdk().appels_dll()[depuis..].contains(&"FD9_StartCapture"));
}

#[test]
fn le_plafond_passe_avant_le_composant() {
    // Palier refusé : la réponse est celle du plafond, comme avant.
    let vide = dossier("plafond");
    let (mut session, _) = connectee(Palier::Connexion, vide.to_path_buf());

    assert!(matches!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::PalierNonAutorise { .. })
    ));
}

#[test]
fn le_dossier_du_pont_est_celui_de_son_executable() {
    assert_eq!(
        dossier_de_l_executable(Ok(PathBuf::from("C:/myiro-libre/pont-fd9-x86.exe"))),
        Some(PathBuf::from("C:/myiro-libre"))
    );
    // Chemin illisible ou sans dossier : aucun dossier, jamais le dossier
    // courant à sa place.
    assert_eq!(
        dossier_de_l_executable(Err(std::io::Error::other("illisible"))),
        None
    );
    assert_eq!(dossier_de_l_executable(Ok(PathBuf::new())), None);
}

#[test]
fn sans_dossier_du_pont_lisible_la_feuille_est_refusee_sans_appel_a_la_dll() {
    let mut session = Session::new(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation)
        .avec_delais(COURTS)
        .avec_dossier_du_pont_illisible();
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let depuis = session.sdk().appels.len();

    assert_eq!(
        session.prenumeriser(&rgb2520()),
        Err(ErreurPont::ComposantQrIntrouvable {
            chemin: "zxing.dll".into()
        })
    );
    assert_eq!(session.sdk().appels.len(), depuis);
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("dossier du pont illisible")));
}
