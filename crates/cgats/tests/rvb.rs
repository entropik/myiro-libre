//! Colonnes RVB d'un CGATS écrit par myiro-libre (mesure d'une feuille au
//! FD-9) : écrites à côté de la mesure, relues avec une échelle confirmée.
//! Mesures fictives (n° 12345678).

use cgats::{ecrire_avec_rvb, lire};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Geometrie,
    Horodatage, Illuminant, Info, InstrumentMesurant, Lab, LectureFeuille, Mesure, Observateur,
    Plage, Provenance, Spectre,
};

fn plage(l: f32) -> Plage {
    let spectre = || Spectre::new(vec![l / 100.0; 36]).unwrap();
    Plage::new(
        [spectre(), spectre(), spectre()],
        DonneesBrutes::new(vec![1000.0; 152]).unwrap(),
        [
            Lab::new([l, 0.5, -1.0]).unwrap(),
            Lab::new([l, 0.4, -2.0]).unwrap(),
            Lab::new([l, 0.6, -0.5]).unwrap(),
        ],
    )
    .unwrap()
}

/// Une feuille de trois plages, conditions supposées, comme la rend le pont
/// FD-9.
fn feuille() -> Mesure {
    Mesure::new(
        vec![plage(95.0), plage(50.0), plage(3.0)],
        Provenance {
            instrument: InstrumentMesurant {
                modele: "FD-9".into(),
                numero_serie: 12345678,
                micrologiciel: "1.00".into(),
                code_produit: "fictif".into(),
            },
            version_sdk: [1, 3, 2],
            empreinte_dll: Info::Inconnue,
            version_pont: "0.1.0".into(),
            architecture: "x86".into(),
            horodatage: Horodatage::new("2026-10-30T15:15:00+01:00").unwrap(),
            etalonnage: Info::Inconnue,
            geometrie: Geometrie::Feuille {},
            calcul: Calcul {
                libelle: "fictif".into(),
                demande: Info::Confirmee(ConditionsCalcul {
                    conditions_spectres: [
                        Info::Supposee(ConditionMesure::M0),
                        Info::Supposee(ConditionMesure::M1),
                        Info::Supposee(ConditionMesure::M2),
                    ],
                    longueurs_onde: Info::Supposee(Echantillonnage {
                        debut_nm: 380,
                        pas_nm: 10,
                    }),
                    illuminant_lab: Info::Supposee(Illuminant::D50),
                    observateur_lab: Info::Supposee(Observateur::DeuxDegres),
                }),
                observe: Info::Inconnue,
            },
            feuille: Some(LectureFeuille {
                qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
                zones: 1,
                plages: 3,
                fiabilite: 0,
                ecart_max: 101,
            }),
        },
    )
    .unwrap()
}

#[test]
fn un_cgats_myiro_libre_avec_rvb_se_relit_avec_une_echelle_confirmee() {
    let rvb = [[100.0, 100.0, 100.0], [0.0, 50.0, 100.0], [0.0, 0.0, 0.0]];
    let texte = ecrire_avec_rvb(&feuille(), &rvb).unwrap();
    let relue = lire(&texte).unwrap();
    let lus: Vec<_> = relue.plages.iter().map(|p| p.rvb.clone()).collect();
    assert_eq!(
        lus,
        vec![
            Info::Confirmee([1.0, 1.0, 1.0]),
            Info::Confirmee([0.0, 0.5, 1.0]),
            Info::Confirmee([0.0, 0.0, 0.0]),
        ]
    );
    // Les trois tableaux (M0, M1, M2) portent les mêmes RVB.
    assert_eq!(texte.matches("\tRGB_R\tRGB_G\tRGB_B\t").count(), 3);
    // Le Lab et la provenance restent ceux de la mesure.
    assert_eq!(
        relue.plages[1].lab[1],
        Info::Confirmee(Lab::new([50.0, 0.4, -2.0]).unwrap())
    );
    assert_eq!(
        relue.provenance,
        Info::Confirmee(feuille().provenance().clone())
    );
}

#[test]
fn des_rvb_en_nombre_different_des_plages_sont_refuses() {
    let erreur = ecrire_avec_rvb(&feuille(), &[[0.0, 0.0, 0.0]; 2]).unwrap_err();
    assert!(erreur.0.contains("2 valeurs RVB pour 3 plages"), "{erreur}");
}

#[test]
fn les_conditions_supposees_par_le_pont_se_relisent_supposees() {
    // Sans condition confirmée, aucun MEASUREMENT_CONDITION n'est écrit :
    // la condition de chaque tableau vient alors de la provenance du
    // fichier, avec sa qualification. Rien n'est confirmé en route.
    let texte = ecrire_avec_rvb(&feuille(), &[[50.0, 50.0, 50.0]; 3]).unwrap();
    assert!(!texte.contains("MEASUREMENT_CONDITION"));
    let relue = lire(&texte).unwrap();
    assert_eq!(
        relue.conditions,
        [
            Info::Supposee(ConditionMesure::M0),
            Info::Supposee(ConditionMesure::M1),
            Info::Supposee(ConditionMesure::M2),
        ]
    );
}
