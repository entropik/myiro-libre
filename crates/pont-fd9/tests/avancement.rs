//! Avancement d'une feuille (ticket #34) : le pont annonce la feuille
//! engagée, la lecture, le calage prêt (avant la mesure) et la mesure en
//! cours, avec le compte de plages que rend le rappel (état 11, fiche
//! `docs/abi/FD9_RegisterDeviceEventHandler.md`). FD-9 simulé seulement.

mod commun;

use commun::Fd9Simule;
use pont_fd9::{Delais, Session};
use pont_protocole::{AdresseReseau, AvancementFeuille, DefinitionFeuille, Palier, ZoneFeuille};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Mire de 2 rangées × 3 colonnes : six plages.
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

const COURTS: Delais = Delais {
    feuille: Duration::from_millis(50),
    etape: Duration::from_millis(50),
    mesure: Duration::from_millis(50),
    deconnexion: Duration::from_millis(50),
};

/// Session connectée au plafond `plafond`, dont l'avancement est suivi.
fn suivie(sdk: Fd9Simule, plafond: Palier) -> (Session<Fd9Simule>, Receiver<AvancementFeuille>) {
    let mut session = Session::new(sdk, plafond).avec_delais(COURTS);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    let (envoi, recus) = mpsc::channel();
    session.suivre(envoi);
    (session, recus)
}

fn etapes(recus: &Receiver<AvancementFeuille>) -> Vec<AvancementFeuille> {
    recus.try_iter().collect()
}

fn mesure(mesurees: u32) -> AvancementFeuille {
    AvancementFeuille::Mesure {
        mesurees,
        plages: 6,
    }
}

#[test]
fn une_feuille_mesuree_annonce_chaque_etape_avant_sa_reponse() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.comptes_mesure = Some(vec![0, 2, 2, 5, 6]);
    let (mut session, recus) = suivie(sdk, Palier::Feuille);

    session.mesurer_feuille(&six_plages()).unwrap();

    // Le calage est annoncé avant la mesure ; la mesure lancée l'est sans
    // compte ; chaque compte vient d'un événement 11, un compte répété
    // n'est annoncé qu'une fois.
    let etapes = etapes(&recus);
    assert!(matches!(etapes[2], AvancementFeuille::CalagePret { .. }));
    assert_eq!(
        etapes,
        vec![
            AvancementFeuille::FeuilleEngagee {},
            AvancementFeuille::Lecture {},
            etapes[2].clone(),
            AvancementFeuille::MesureLancee {},
            mesure(0),
            mesure(2),
            mesure(5),
            mesure(6),
        ]
    );
}

/// Sans événement 11, aucun compte n'est inventé : « mesure lancée »
/// seulement.
#[test]
fn sans_compte_du_fd9_aucun_compte_n_est_annonce() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.comptes_mesure = Some(vec![]);
    let (mut session, recus) = suivie(sdk, Palier::Feuille);

    session.mesurer_feuille(&six_plages()).unwrap();

    let etapes = etapes(&recus);
    assert_eq!(etapes.last(), Some(&AvancementFeuille::MesureLancee {}));
    assert!(!etapes
        .iter()
        .any(|e| matches!(e, AvancementFeuille::Mesure { .. })));
}

/// Le calage annoncé est rendu : la réponse finale ne le renvoie pas. Sans
/// suivi, la réponse le porte comme avant.
#[test]
fn le_calage_n_est_rendu_qu_une_fois() {
    let (mut session, recus) = suivie(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);
    session.mesurer_feuille(&six_plages()).unwrap();
    assert!(etapes(&recus)
        .iter()
        .any(|e| matches!(e, AvancementFeuille::CalagePret { .. })));
    assert_eq!(session.prendre_calage(), None);

    session.ne_plus_suivre();
    // Une deuxième feuille glissée.
    session.sdk_mut().a_venir = [2, 3].into();
    session.mesurer_feuille(&six_plages()).unwrap();
    assert!(session.prendre_calage().is_some());
}

#[test]
fn le_calage_pret_part_avant_le_lancement_de_la_mesure() {
    let (mut session, recus) = suivie(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);
    session.mesurer_feuille(&six_plages()).unwrap();
    // L'image est lue avant le lancement de la mesure, et le calage annoncé
    // avant la première annonce de mesure.
    let appels = session.sdk().appels_dll();
    let image = appels
        .iter()
        .position(|a| a.starts_with("FD9_GetChartImage"))
        .unwrap();
    let mesure = appels
        .iter()
        .position(|a| a.starts_with("FD9_StartMeasurement2"))
        .unwrap();
    assert!(image < mesure, "{appels:?}");
    let etapes = etapes(&recus);
    let calage = etapes
        .iter()
        .position(|e| matches!(e, AvancementFeuille::CalagePret { .. }))
        .unwrap();
    let debut_mesure = etapes
        .iter()
        .position(|e| matches!(e, AvancementFeuille::Mesure { .. }))
        .unwrap();
    assert!(calage < debut_mesure, "{etapes:?}");
}

#[test]
fn une_feuille_deja_engagee_est_annoncee_tout_de_suite() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, recus) = suivie(sdk, Palier::Prenumerisation);
    // L'opérateur a glissé la feuille avant la requête : état 3.
    session.sdk_mut().etat = 3;

    session.prenumeriser(&six_plages()).unwrap();

    let etapes = etapes(&recus);
    assert_eq!(
        etapes[..2],
        [
            AvancementFeuille::FeuilleEngagee {},
            AvancementFeuille::Lecture {}
        ]
    );
    // Prénumérisation : le calage, puis l'arrêt ; jamais de mesure.
    assert!(matches!(etapes[2], AvancementFeuille::CalagePret { .. }));
    assert_eq!(etapes.len(), 3, "{etapes:?}");
}

#[test]
fn une_autre_mire_s_arrete_apres_la_lecture_sans_calage() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.qr_texte = "CMYK-2520 - 1P MY9*1+".into();
    let (mut session, recus) = suivie(sdk, Palier::Feuille);

    assert!(session.mesurer_feuille(&six_plages()).is_err());

    assert_eq!(
        etapes(&recus),
        vec![
            AvancementFeuille::FeuilleEngagee {},
            AvancementFeuille::Lecture {}
        ]
    );
}

#[test]
fn sans_feuille_glissee_rien_n_est_annonce() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let (mut session, recus) = suivie(sdk, Palier::Feuille);

    assert!(session.mesurer_feuille(&six_plages()).is_err());

    assert_eq!(etapes(&recus), vec![]);
}

#[test]
fn sans_suivi_la_feuille_se_mesure_comme_avant() {
    let mut session =
        Session::new(Fd9Simule::avec_une_rgb2520(), Palier::Feuille).avec_delais(COURTS);
    session.version().unwrap();
    session
        .connecter_adresse(&AdresseReseau::new("192.0.2.40").unwrap())
        .unwrap();
    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
}

#[test]
fn un_suivi_arrete_n_est_plus_servi() {
    let (mut session, recus) = suivie(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);
    session.ne_plus_suivre();
    session.mesurer_feuille(&six_plages()).unwrap();
    assert_eq!(etapes(&recus), vec![]);
}
