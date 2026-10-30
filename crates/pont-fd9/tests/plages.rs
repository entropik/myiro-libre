//! Palier Feuille du pont FD-9 contre le FD-9 simulé : mesure, lecture des
//! 152 données brutes de chaque plage, spectres et Lab M0, M1, M2 calculés
//! par la DLL, association à la mire vérifiée par la position rendue.
//! Fiches `docs/abi/FD9_StartMeasurement2.md`, `FD9_GetData.md`,
//! `FD9_SetOption.md` et `FD9_CalculateData.md`. Valeurs fictives.

mod commun;

use commun::Fd9Simule;
use fd9_sys::PositionPlage;
use pont_fd9::{Delais, Session};
use pont_protocole::{
    AdresseReseau, ConditionMesure, DefautPositions, DefinitionFeuille, Echantillonnage,
    ErreurPont, Geometrie, Illuminant, Info, Mesure, Observateur, Palier, ZoneFeuille, MODELE_FD9,
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

/// Une petite mire de 2 rangées × 3 colonnes, pour lire les appels un à un.
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

fn mesuree(mire: &DefinitionFeuille) -> (Mesure, Session<Fd9Simule>, usize) {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);
    let mesure = session.mesurer_feuille(mire).unwrap();
    (mesure, session, depuis)
}

#[test]
fn une_feuille_rgb2520_rend_2520_plages_dans_l_ordre() {
    let (mesure, session, _) = mesuree(&rgb2520());

    assert_eq!(mesure.plages().len(), 2_520);
    // Données brutes fictives du simulé : 1 000 + n° de la plage, puis + k.
    for (i, plage) in mesure.plages().iter().enumerate() {
        assert_eq!(plage.brutes()[0], 1_000.0 + i as f32, "plage {i}");
    }
    // Chaque plage demandée par son numéro, dans l'ordre.
    let lectures: Vec<&str> = session
        .sdk()
        .appels_dll()
        .into_iter()
        .filter(|a| a.starts_with("FD9_GetData"))
        .collect();
    assert_eq!(lectures.len(), 2_520);
    assert_eq!(lectures[0], "FD9_GetData 0");
    assert_eq!(lectures[2_519], "FD9_GetData 2519");
    // Serpentin sur 42 colonnes : les 30 rangées impaires sont à l'envers.
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("2520 plage(s) lue(s)") && l.contains("1260 hors de l'ordre")));
}

#[test]
fn chaque_plage_a_trois_spectres_et_ses_donnees_brutes() {
    let (mesure, _, _) = mesuree(&six_plages());

    let plage = &mesure.plages()[4];
    assert_eq!(plage.brutes().len(), 152);
    assert_eq!(plage.brutes()[151], 1_004.0 + 151.0);
    // Spectres fictifs : la condition posée (0, 2, 15), plus k / 4.
    assert_eq!(plage.m0().len(), 36);
    assert_eq!(plage.m0()[0], 0.0);
    assert_eq!(plage.m1()[0], 2.0);
    assert_eq!(plage.m2()[35], 15.0 + 35.0 / 4.0);
    // Lab fictifs : [clarté de la couleur imprimée, condition, illuminant ×
    // 10 + observateur] : D50 (2) et 2° (0) posés pour chacun des trois.
    let clarte = pont_fd9::calage::clarte_attendue(six_plages().couleurs.unwrap()[4]) as f32;
    let [m0, m1, m2] = plage.lab();
    assert_eq!(m0.valeurs(), [clarte, 0.0, 20.0]);
    assert_eq!(m1.valeurs(), [clarte, 2.0, 20.0]);
    assert_eq!(m2.valeurs(), [clarte, 15.0, 20.0]);
}

#[test]
fn les_options_sont_reposees_avant_chaque_lecture_et_chaque_calcul() {
    let (_, session, depuis) = mesuree(&six_plages());
    let appels = &session.sdk().appels_dll()[depuis..];
    let premiere = appels.iter().position(|a| *a == "FD9_GetData 0").unwrap();
    assert_eq!(appels[premiere - 1], "FD9_SetOption 3 12");
    // Après la lecture des six plages : la sortie, puis le calcul de la
    // première, M0 en tête.
    let sortie = appels
        .iter()
        .position(|a| *a == "FD9_ExitMeasurement 0 nul")
        .unwrap();
    assert_eq!(appels[sortie - 1], "FD9_GetData 5");
    assert_eq!(
        appels[sortie + 1..sortie + 9],
        [
            "FD9_SetOption 0 0",
            "FD9_SetOption 3 11",
            "FD9_CalculateData 36",
            "FD9_SetOption 3 0",
            "FD9_SetOption 1 2",
            "FD9_SetOption 2 0",
            "FD9_CalculateData 3",
            "FD9_SetOption 0 2",
        ]
    );
}

#[test]
fn la_fin_lit_les_plages_puis_sort_sans_arreter() {
    let (_, session, depuis) = mesuree(&six_plages());
    let appels = &session.sdk().appels_dll()[depuis..];

    // Ni StopMeasurement(vrai), refusé à l'état 9, ni arrêt d'aucune sorte.
    assert!(!appels.iter().any(|a| a.starts_with("FD9_StopMeasurement")));
    let debut: Vec<&str> = appels
        .iter()
        .copied()
        .filter(|a| !a.starts_with("FD9_SetOption") && !a.starts_with("FD9_CalculateData"))
        .collect();
    assert_eq!(
        debut,
        [
            "FD9_GetStatus",
            "FD9_StartCapture",
            "FD9_GetCaptureResult image nulle",
            "FD9_GetCaptureResult image 1000",
            "FD9_StartPatchRecognition mode 4, couleurs RVB 6, rangées 0, colonnes 0",
            "FD9_GetPatchRecognitionResult2",
            "FD9_GetMeasurePoint 6",
            "FD9_StartMeasurement2 1 1",
            "FD9_GetData 0",
            "FD9_GetData 1",
            "FD9_GetData 2",
            "FD9_GetData 3",
            "FD9_GetData 4",
            "FD9_GetData 5",
            "FD9_ExitMeasurement 0 nul",
        ]
    );
}

#[test]
fn le_nombre_de_plages_mesurees_est_journalise() {
    let (_, session, _) = mesuree(&six_plages());
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("état 11") && l.contains("compte 6")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn les_plages_lues_en_serpentin_sont_rangees_dans_l_ordre_de_la_mire() {
    // Essai E4 : le FD-9 rend la rangée 1 de droite à gauche. La plage lue
    // n° 4 est en (rangée 1, colonne 2) : elle est rangée 6ᵉ. Seule la
    // colonne du milieu de la rangée 1 tombe à sa place.
    let (mesure, session, _) = mesuree(&six_plages());

    let premieres: Vec<f32> = mesure.plages().iter().map(|p| p.brutes()[0]).collect();
    assert_eq!(
        premieres,
        [1_000.0, 1_001.0, 1_002.0, 1_003.0, 1_004.0, 1_005.0]
    );
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("6 plage(s) lue(s)") && l.contains("2 hors de l'ordre de la mire")));
}

#[test]
fn un_ordre_quelconque_est_range_par_la_position_rendue() {
    // Le pont ne suppose pas le serpentin : seule la position rendue compte.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.ordre = Some(vec![5, 0, 3, 1, 4, 2]);
    let (mut session, _) = connectee(sdk, Palier::Feuille);

    let mesure = session.mesurer_feuille(&six_plages()).unwrap();

    let premieres: Vec<f32> = mesure.plages().iter().map(|p| p.brutes()[0]).collect();
    assert_eq!(
        premieres,
        [1_000.0, 1_001.0, 1_002.0, 1_003.0, 1_004.0, 1_005.0]
    );
}

#[test]
fn deux_zones_sont_rangees_zone_par_zone() {
    // Zone 0 : 2 × 3 ; zone 1 : 1 × 2, plus bas sur la feuille.
    let mut mire = six_plages();
    let mut seconde = mire.zones[0].clone();
    seconde.haut = 20_000;
    seconde.rangees = 1;
    seconde.colonnes = 2;
    mire.zones.push(seconde);
    mire.couleurs = Some(commun::couleurs_fictives(8));
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.ordre = Some(vec![7, 6, 0, 1, 2, 5, 4, 3]);
    let (mut session, _) = connectee(sdk, Palier::Feuille);

    let mesure = session.mesurer_feuille(&mire).unwrap();

    let premieres: Vec<f32> = mesure.plages().iter().map(|p| p.brutes()[0]).collect();
    assert_eq!(
        premieres,
        (0..8).map(|n| 1_000.0 + n as f32).collect::<Vec<_>>()
    );
}

/// Feuille de six plages dont la plage lue d'index `index` rend `position`.
fn position_forcee(index: u32, position: (u32, i32, i32)) -> Session<Fd9Simule> {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.positions_forcees.insert(
        index,
        PositionPlage {
            zone_ou_index: position.0,
            rangee: position.1,
            colonne: position.2,
        },
    );
    connectee(sdk, Palier::Feuille).0
}

/// Toutes les plages sont lues, puis la feuille est close, sans calcul.
fn lue_puis_close_sans_calcul(session: &Session<Fd9Simule>) {
    let appels = session.sdk().appels_dll();
    assert_eq!(
        appels
            .iter()
            .filter(|a| a.starts_with("FD9_GetData"))
            .count(),
        6
    );
    assert_eq!(appels.last(), Some(&"FD9_ExitMeasurement 0 nul"));
    assert!(!appels.iter().any(|a| a.starts_with("FD9_CalculateData")));
}

#[test]
fn une_position_rendue_deux_fois_refuse_la_mesure() {
    // La plage lue n° 3 rend la position de la première.
    let mut session = position_forcee(2, (0, 0, 0));

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::PositionsPlages {
            defaut: DefautPositions::Doublon {
                lue: 3,
                zone: 0,
                rangee: 0,
                colonne: 0
            }
        })
    );
    lue_puis_close_sans_calcul(&session);
}

#[test]
fn une_position_hors_de_la_mire_refuse_la_mesure() {
    for (position, lue) in [
        ((0, 2, 0), 5),
        ((0, 0, 3), 5),
        ((1, 0, 0), 5),
        ((0, -1, 0), 5),
    ] {
        let mut session = position_forcee(4, position);

        assert_eq!(
            session.mesurer_feuille(&six_plages()),
            Err(ErreurPont::PositionsPlages {
                defaut: DefautPositions::HorsMire {
                    lue,
                    zone: position.0,
                    rangee: position.1,
                    colonne: position.2
                }
            }),
            "{position:?}"
        );
        lue_puis_close_sans_calcul(&session);
    }
}

#[test]
fn une_mesure_a_cote_des_plages_est_refusee() {
    // Essai E4 bis : chaque plage mesurée sur sa voisine. Les L* ne suivent
    // plus la clarté des couleurs imprimées : rien n'est rendu.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.lab_decale = true;
    let (mut session, _) = connectee(sdk, Palier::Feuille);

    let Err(ErreurPont::MesureInvraisemblable {
        correlation_milliemes,
    }) = session.mesurer_feuille(&rgb2520())
    else {
        panic!("{:?}", session.journal())
    };
    assert!(correlation_milliemes < 500, "{correlation_milliemes}");
    assert!(session
        .journal()
        .iter()
        .any(|l| l.starts_with("vraisemblance : corrélation")));
}

#[test]
fn une_mesure_juste_passe_le_controle_de_vraisemblance() {
    let (_, session, _) = mesuree(&rgb2520());
    assert!(session
        .journal()
        .iter()
        .any(|l| l.contains("vraisemblance : corrélation des L* M1 à la clarté imprimée 1000")));
}

#[test]
fn un_spectre_de_longueur_inattendue_refuse_la_mesure() {
    // La DLL demande plus de 36 valeurs : 1005, sortie trop courte.
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.spectre_longueur = 40;
    let (mut session, _) = connectee(sdk, Palier::Feuille);

    let Err(ErreurPont::ReponseInattendue { detail }) = session.mesurer_feuille(&six_plages())
    else {
        panic!("{:?}", session.journal())
    };
    assert!(detail.contains("36") && detail.contains("1005"), "{detail}");
}

#[test]
fn une_lecture_refusee_sort_et_le_dit() {
    let sdk = Fd9Simule::avec_une_rgb2520().rendre("FD9_GetData", 1005);
    let (mut session, depuis) = connectee(sdk, Palier::Feuille);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::Sdk { code: 1005 })
    );
    let appels = &session.sdk().appels_dll()[depuis..];
    assert_eq!(appels.last(), Some(&"FD9_ExitMeasurement 0 nul"));
}

#[test]
fn la_provenance_dit_feuille_et_qr_lu() {
    let (mesure, _, _) = mesuree(&six_plages());
    let p = mesure.provenance();

    assert_eq!(p.instrument.modele, MODELE_FD9);
    assert_eq!(p.instrument.numero_serie, 12345678);
    assert_eq!(p.instrument.micrologiciel, "1.02.0005");
    assert_eq!(p.version_sdk, [1, 0x20, 3]);
    assert_eq!(
        p.empreinte_dll.valeur().map(|e| e.texte()),
        Some(&*"5e".repeat(32))
    );
    assert_eq!(p.geometrie, Geometrie::Feuille {});
    let lecture = p.feuille.as_ref().unwrap();
    assert_eq!(lecture.qr, Info::Confirmee("RGB-2520 - 1P MY9*1+".into()));
    assert_eq!((lecture.zones, lecture.plages), (1, 6));
    // Pas d'étalonnage par l'utilisateur : l'instrument s'étalonne seul.
    assert_eq!(p.etalonnage, Info::Inconnue);
    assert!(p.calcul.libelle.contains("étalonnage automatique"));
}

#[test]
fn les_conditions_restent_supposees_tant_que_non_verifiees() {
    let (mesure, _, _) = mesuree(&six_plages());
    let calcul = &mesure.provenance().calcul;
    let Info::Confirmee(demande) = &calcul.demande else {
        panic!("{calcul:?}")
    };
    assert_eq!(
        demande.conditions_spectres,
        [
            Info::Supposee(ConditionMesure::M0),
            Info::Supposee(ConditionMesure::M1),
            Info::Supposee(ConditionMesure::M2),
        ]
    );
    assert_eq!(
        demande.longueurs_onde,
        Info::Supposee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10
        })
    );
    // Valeurs 2 et 0 tirées d'une table de FD-S2w, sans relecture possible
    // dans la DLL : supposées (fiche FD9_SetOption).
    assert_eq!(demande.illuminant_lab, Info::Supposee(Illuminant::D50));
    assert_eq!(
        demande.observateur_lab,
        Info::Supposee(Observateur::DeuxDegres)
    );
    assert_eq!(calcul.observe, Info::Inconnue);
}

#[test]
fn mesurer_sans_prenumerisation_prealable_est_permis() {
    let (mut session, _) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Feuille);
    assert_eq!(session.palier_atteint(), Some(Palier::Connexion));

    assert_eq!(
        session
            .mesurer_feuille(&six_plages())
            .unwrap()
            .plages()
            .len(),
        6
    );
    assert_eq!(session.palier_atteint(), Some(Palier::Feuille));
}

#[test]
fn la_feuille_est_refusee_au_plafond_prenumerisation_sans_appel() {
    let (mut session, depuis) = connectee(Fd9Simule::avec_une_rgb2520(), Palier::Prenumerisation);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::PalierNonAutorise {
            demande: Palier::Feuille,
            plafond: Palier::Prenumerisation,
        })
    );
    assert_eq!(session.sdk().appels.len(), depuis);
}

#[test]
fn sans_fin_de_mesure_le_delai_arrete_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    // Mesure lancée (état 8), jamais finie.
    sdk.reactions.insert("FD9_StartMeasurement2", vec![8]);
    let (mut session, depuis) = connectee(sdk, Palier::Feuille);

    assert_eq!(
        session.mesurer_feuille(&six_plages()),
        Err(ErreurPont::Delai {})
    );
    let appels = &session.sdk().appels_dll()[depuis..];
    assert_eq!(
        appels[appels.len() - 2..],
        ["FD9_StopMeasurement faux", "FD9_ExitMeasurement 0 nul"]
    );
    assert!(!appels.iter().any(|a| a.starts_with("FD9_GetData")));
}

#[test]
fn une_position_non_ecrite_refuse_la_mesure() {
    // Une seule plage : sa position attendue est (0, 0, 0), celle d'une
    // structure à zéro. La DLL rend 0 sans rien écrire : la position de
    // départ du pont, impossible, reste.
    let mut mire = six_plages();
    mire.zones[0].rangees = 1;
    mire.zones[0].colonnes = 1;
    mire.couleurs = Some(commun::couleurs_fictives(1));
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.position_non_ecrite = true;
    let (mut session, _) = connectee(sdk, Palier::Feuille);

    let Err(ErreurPont::PositionsPlages {
        defaut: DefautPositions::HorsMire { lue: 1, .. },
    }) = session.mesurer_feuille(&mire)
    else {
        panic!("{:?}", session.journal())
    };
}
