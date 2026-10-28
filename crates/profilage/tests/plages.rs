//! Préparation des plages d'une mesure importée pour le calcul d'un profil,
//! vue par l'API publique de la crate. Toutes les mesures sont fictives.

use cgats::{MesureImportee, PlageImportee};
use pont_protocole::{ConditionMesure, Echantillonnage, Info, Lab, Spectre};
use profilage::{conditions_utilisables, preparer_plages, RefusPlages};

fn lab(l: f32, a: f32, b: f32) -> Info<Lab> {
    Info::Confirmee(Lab::new([l, a, b]).unwrap())
}

fn spectre(base: f32) -> Info<Spectre> {
    Info::Confirmee(Spectre::new((0..36).map(|i| base + i as f32 * 0.01).collect()).unwrap())
}

/// Plage fictive d'un export FD-S2w : RVB sur 255, Lab et spectre en M1 seul.
fn plage(identifiant: &str, rvb_255: [f32; 3], l: f32) -> PlageImportee {
    PlageImportee {
        identifiant: identifiant.to_string(),
        spectres: [Info::Inconnue, spectre(l / 100.0), Info::Inconnue],
        lab: [Info::Inconnue, lab(l, 1.5, -2.25), Info::Inconnue],
        rvb: Info::Supposee(rvb_255.map(|c| c / 255.0)),
    }
}

/// Mesure fictive d'un export FD-S2w : trois plages, condition M1 supposée
/// (source `D50` du fabricant), spectres de 380 à 730 nm.
fn mesure_fd_s2w() -> MesureImportee {
    MesureImportee {
        provenance: Info::Inconnue,
        instrument: Info::Confirmee("FD-9".to_string()),
        numero_serie: Info::Confirmee("12345678".to_string()),
        date: Info::Inconnue,
        conditions: [
            Info::Inconnue,
            Info::Supposee(ConditionMesure::M1),
            Info::Inconnue,
        ],
        longueurs_onde: Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10,
        }),
        plages: vec![
            plage("A1", [255.0, 255.0, 255.0], 95.0),
            plage("A2", [0.0, 117.61, 199.05], 42.09),
            plage("A3", [0.0, 0.0, 0.0], 3.5),
        ],
    }
}

fn proche(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

#[test]
fn une_mesure_fd_s2w_m1_se_prepare() {
    let p = preparer_plages(&mesure_fd_s2w(), ConditionMesure::M1).unwrap();
    assert_eq!(p.plages.len(), 3);
    assert_eq!(p.plages[0].rvb, [100.0, 100.0, 100.0]);
    assert!(
        proche(p.plages[1].rvb, [0.0, 46.121_57, 78.058_82]),
        "{:?}",
        p.plages[1].rvb
    );
    assert!(proche(p.plages[1].lab, [42.09, 1.5, -2.25]));
    assert_eq!(p.condition, ConditionMesure::M1);
    assert!(p.rvb_supposes && p.condition_supposee);
}

#[test]
fn une_condition_absente_est_refusee() {
    assert_eq!(
        preparer_plages(&mesure_fd_s2w(), ConditionMesure::M0),
        Err(RefusPlages::ConditionAbsente(ConditionMesure::M0))
    );
}

#[test]
fn un_lab_manquant_nomme_la_plage() {
    let mut m = mesure_fd_s2w();
    m.plages[1].identifiant = "17".to_string();
    m.plages[1].lab[1] = Info::Inconnue;
    assert_eq!(
        preparer_plages(&m, ConditionMesure::M1),
        Err(RefusPlages::LabInconnu {
            plage: "17".to_string()
        })
    );
}

#[test]
fn des_rvb_inconnus_sont_refuses() {
    let mut m = mesure_fd_s2w();
    for p in &mut m.plages {
        p.rvb = Info::Inconnue;
    }
    assert_eq!(
        preparer_plages(&m, ConditionMesure::M1),
        Err(RefusPlages::RvbInconnus)
    );
}

#[test]
fn un_rvb_au_dela_de_100_pour_cent_est_refuse() {
    let mut m = mesure_fd_s2w();
    m.plages[2].rvb = Info::Supposee([0.0, 255.01 / 255.0, 0.0]);
    assert_eq!(
        preparer_plages(&m, ConditionMesure::M1),
        Err(RefusPlages::RvbHorsEchelle {
            plage: "A3".to_string()
        })
    );
}

#[test]
fn un_rvb_negatif_est_refuse() {
    // Une valeur négative est gardée telle quelle à la lecture du CGATS.
    let mut m = mesure_fd_s2w();
    m.plages[0].rvb = Info::Supposee([1.0, -0.01 / 255.0, 1.0]);
    assert_eq!(
        preparer_plages(&m, ConditionMesure::M1),
        Err(RefusPlages::RvbHorsEchelle {
            plage: "A1".to_string()
        })
    );
}

#[test]
fn une_mesure_sans_plage_est_refusee() {
    let mut m = mesure_fd_s2w();
    m.plages.clear();
    assert_eq!(
        preparer_plages(&m, ConditionMesure::M1),
        Err(RefusPlages::Vide)
    );
}

#[test]
fn seules_les_conditions_completes_sont_utilisables() {
    let mut m = mesure_fd_s2w();
    assert_eq!(conditions_utilisables(&m), [ConditionMesure::M1]);

    // M0 et M2 confirmées, mais une plage sans Lab en M2.
    m.conditions[0] = Info::Confirmee(ConditionMesure::M0);
    m.conditions[2] = Info::Confirmee(ConditionMesure::M2);
    for p in &mut m.plages {
        p.lab[0] = lab(50.0, 0.0, 0.0);
        p.lab[2] = lab(50.0, 0.0, 0.0);
    }
    m.plages[1].lab[2] = Info::Inconnue;
    assert_eq!(
        conditions_utilisables(&m),
        [ConditionMesure::M0, ConditionMesure::M1]
    );

    for p in &mut m.plages {
        p.rvb = Info::Inconnue;
    }
    assert!(conditions_utilisables(&m).is_empty());
}

#[test]
fn avec_des_spectres_complets_les_longueurs_d_onde_sont_connues() {
    let p = preparer_plages(&mesure_fd_s2w(), ConditionMesure::M1).unwrap();
    assert_eq!(
        p.longueurs_onde,
        Some(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10
        })
    );
    // Spectre en pourcentage : 0,95 + 0,01 × i de réflectance.
    let s = p.plages[0].spectre.as_ref().unwrap();
    assert_eq!(s.len(), 36);
    assert!((s[0] - 95.0).abs() < 1e-4 && (s[35] - 130.0).abs() < 1e-4);
}

#[test]
fn sans_spectre_complet_les_longueurs_d_onde_sont_absentes() {
    let mut m = mesure_fd_s2w();
    m.plages[2].spectres[1] = Info::Inconnue;
    let p = preparer_plages(&m, ConditionMesure::M1).unwrap();
    assert_eq!(p.longueurs_onde, None);
    assert!(p.plages.iter().all(|plage| plage.spectre.is_none()));

    let mut m = mesure_fd_s2w();
    m.longueurs_onde = Info::Inconnue;
    let p = preparer_plages(&m, ConditionMesure::M1).unwrap();
    assert_eq!(p.longueurs_onde, None);
    assert!(p.plages.iter().all(|plage| plage.spectre.is_none()));
}
