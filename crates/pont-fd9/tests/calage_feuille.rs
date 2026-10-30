//! Calage rendu à l'application (ticket #34) : les points que le FD-9 va
//! viser et l'image de la feuille qu'il a prise (`FD9_GetChartImage`),
//! réduite par le pont à environ 1 000 pixels de haut. Contrat : fiches
//! `docs/abi/FD9_GetMeasurePoint.md` et `FD9_GetChartImage.md`. Images et
//! points synthétiques : jamais l'image d'une vraie feuille.

mod commun;

use commun::Fd9Simule;
use fd9_sys::PointMesure;
use pont_fd9::{Delais, Session};
use pont_protocole::{
    AdresseReseau, DefinitionFeuille, ErreurPont, Palier, PointVise, ZoneFeuille,
};
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

/// Mire de 2 rangées × 3 colonnes, pour lire les appels un à un.
fn six_plages() -> DefinitionFeuille {
    let mut mire = rgb2520();
    mire.zones[0].rangees = 2;
    mire.zones[0].colonnes = 3;
    mire.couleurs = Some(commun::couleurs_fictives(6));
    mire
}

const COURTS: Delais = Delais {
    feuille: Duration::from_millis(50),
    etape: Duration::from_millis(50),
    mesure: Duration::from_millis(50),
    deconnexion: Duration::from_millis(50),
};

fn connectee(sdk: Fd9Simule, plafond: Palier) -> (Session<Fd9Simule>, usize) {
    let mut session = Session::new(sdk, plafond).avec_delais(COURTS);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let appels = session.sdk().appels.len();
    (session, appels)
}

#[test]
fn la_prenumerisation_rend_les_points_vises_et_l_image_de_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.decalage_points = (880, -110);
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&six_plages()).unwrap();
    let calage = session.prendre_calage().unwrap();

    assert_eq!(calage.decalage, [880, -110]);
    assert_eq!(calage.ecart_max, 0);
    // Un point par plage, dans l'ordre de la définition : centre théorique
    // (1 349 + 300 ; 2 300 + 300) décalé comme la feuille.
    assert_eq!(calage.points.len(), 6);
    assert_eq!(
        calage.points[0],
        PointVise {
            zone: 0,
            rangee: 0,
            colonne: 0,
            x: 1_649 + 880,
            y: 2_600 - 110,
        }
    );
    assert_eq!((calage.points[5].rangee, calage.points[5].colonne), (1, 2));
    assert_eq!(calage.points[5].x, 1_649 + 2 * 601 + 880);
    // Petite image (40 × 25 à 300 ppp) : rendue telle quelle.
    let image = calage.image.unwrap();
    assert_eq!(
        (image.largeur(), image.hauteur(), image.ppp()),
        (40, 25, 300)
    );
    assert_eq!(image.gris()[..3], [0, 1, 2]);
    assert_eq!(image.gris()[40], 1);
    // Lue à l'état 7, après les points, avant l'arrêt ; tampon de 3 638
    // pixels de large, assez haut pour l'image prise (1 000 octets).
    let appels = &session.sdk().appels_dll()[depuis..];
    let points = appels
        .iter()
        .position(|a| a.starts_with("FD9_GetMeasurePoint"));
    let image = appels
        .iter()
        .position(|a| *a == "FD9_GetChartImage 3638 × 1, 8 bits");
    let arret = appels
        .iter()
        .position(|a| a.starts_with("FD9_StopMeasurement"));
    assert!(points < image && image < arret, "{appels:?}");
    // Rendu une fois : la réponse suivante n'en a plus.
    assert_eq!(session.prendre_calage(), None);
}

#[test]
fn une_grande_image_est_reduite_a_environ_mille_pixels_de_haut() {
    // A3 à 300 ppp : 1 203 × 4 920. Réduite par 5 (un diviseur de 300) :
    // 241 × 984 à 60 ppp, chaque pixel la moyenne d'un carré de 5 × 5.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.taille_image = 1_203 * 4_920;
    sdk.image_feuille = Some((1_203, 4_920));
    // Damier de carrés de 5 × 5 pixels : noir, puis blanc.
    sdk.motif_image = Some(|x, y| if (x / 5 + y / 5) % 2 == 0 { 0 } else { 255 });
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&six_plages()).unwrap();
    let image = session.prendre_calage().unwrap().image.unwrap();

    assert_eq!(
        (image.largeur(), image.hauteur(), image.ppp()),
        (241, 984, 60)
    );
    assert_eq!(image.gris()[..3], [0, 255, 0]);
    assert_eq!(image.gris()[241], 255);
    // Dernière colonne : 3 pixels de large seulement, moyennés aussi.
    assert_eq!(image.gris()[240], 0);
}

#[test]
fn une_image_moyenne_garde_l_arrondi_au_plus_proche() {
    // Deux rangées de 1 001 pixels de haut au total : réduite par 2.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.taille_image = 4 * 1_001;
    sdk.image_feuille = Some((2, 1_001));
    // Un seul pixel blanc par carré de 2 × 2 : moyenne 63,75, arrondie à 64.
    sdk.motif_image = Some(|x, y| if x == 0 && y % 2 == 1 { 255 } else { 0 });
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&six_plages()).unwrap();
    let image = session.prendre_calage().unwrap().image.unwrap();

    assert_eq!(
        (image.largeur(), image.hauteur(), image.ppp()),
        (1, 501, 150)
    );
    assert_eq!(image.gris()[0], 64);
    // Dernière rangée : un seul pixel de haut (la 1 001ᵉ, impaire).
    assert_eq!(image.gris()[500], 0);
}

#[test]
fn sans_image_le_calage_est_rendu_quand_meme() {
    // La DLL refuse (1005 : image plus grande que le tampon) : la feuille
    // n'échoue pas, le calage est rendu sans image et le journal le dit.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.image_feuille = Some((4_000, 4_000));
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&six_plages()).unwrap();
    let calage = session.prendre_calage().unwrap();

    assert_eq!(calage.points.len(), 6);
    assert_eq!(calage.image, None);
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("FD9_GetChartImage") && l.contains("1005")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn une_taille_rendue_plus_grande_que_le_tampon_est_ignoree() {
    // Code 0 mais dimensions au-delà du tampon : rien n'est lu au-delà.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.dimensions_forcees = Some((3_638, 2));
    let (mut session, _) = connectee(sdk, Palier::Prenumerisation);

    session.prenumeriser(&six_plages()).unwrap();

    assert_eq!(session.prendre_calage().unwrap().image, None);
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("FD9_GetChartImage") && l.contains("3638 × 2")));
}

#[test]
fn une_feuille_refusee_ne_rend_ni_calage_ni_image() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.points_forces.insert(
        1,
        PointMesure {
            x: 1_649,
            y: 2_600,
            zone: 0,
            rangee: 0,
            colonne: 0,
        },
    );
    let (mut session, depuis) = connectee(sdk, Palier::Prenumerisation);

    assert!(matches!(
        session.prenumeriser(&six_plages()),
        Err(ErreurPont::PositionsPlages { .. })
    ));
    assert_eq!(session.prendre_calage(), None);
    assert!(!session.sdk().appels_dll()[depuis..]
        .iter()
        .any(|a| a.starts_with("FD9_GetChartImage")));
}

#[test]
fn le_calage_d_une_feuille_precedente_ne_reste_pas() {
    let (mut session, _) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);
    session.prenumeriser(&six_plages()).unwrap();
    // Deuxième feuille refusée avant tout appel : plus de calage.
    let mut sans_couleurs = six_plages();
    sans_couleurs.couleurs = None;
    assert!(session.prenumeriser(&sans_couleurs).is_err());
    assert_eq!(session.prendre_calage(), None);
}

#[test]
fn la_mesure_d_une_feuille_rend_aussi_son_calage() {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);

    let mesure = session.mesurer_feuille(&six_plages()).unwrap();
    let calage = session.prendre_calage().unwrap();

    assert_eq!(mesure.plages().len(), 6);
    assert_eq!(calage.points.len(), 6);
    assert!(calage.image.is_some());
    // L'image est lue avant le lancement de la mesure (état 7).
    let appels = &session.sdk().appels_dll()[depuis..];
    let image = appels
        .iter()
        .position(|a| a.starts_with("FD9_GetChartImage"));
    let mesure = appels
        .iter()
        .position(|a| a.starts_with("FD9_StartMeasurement2"));
    assert!(image.is_some() && image < mesure, "{appels:?}");
}
