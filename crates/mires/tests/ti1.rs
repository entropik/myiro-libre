//! Lecture du `.ti1` écrit par `targen` : valeurs RVB de 0 à 100, dans
//! l'ordre des plages. Fichiers fictifs, de la forme relevée sur
//! ArgyllCMS 3.5.0 (28 octobre 2026) : un premier tableau des plages, puis
//! deux tableaux annexes (`DENSITY_EXTREME_VALUES`,
//! `DEVICE_COMBINATION_VALUES`) qui ne sont pas des plages.

use mires::{lire_ti1, ErreurMire};

fn ti1(colonnes: &str, lignes: &[&str]) -> String {
    let mut t = String::from(
        "CTI1   \n\nDESCRIPTOR \"Argyll Calibration Target chart information 1\"\n\
         ORIGINATOR \"Argyll targen\"\nCOLOR_REP \"iRGB\"\n\n",
    );
    t.push_str(&format!(
        "NUMBER_OF_FIELDS {}\nBEGIN_DATA_FORMAT\n{colonnes} \nEND_DATA_FORMAT\n\n",
        colonnes.split_whitespace().count()
    ));
    t.push_str(&format!("NUMBER_OF_SETS {}\nBEGIN_DATA\n", lignes.len()));
    for l in lignes {
        t.push_str(l);
        t.push_str(" \n");
    }
    t.push_str("END_DATA\n");
    t
}

const COLONNES: &str = "SAMPLE_ID RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z";

fn quatre() -> String {
    ti1(
        COLONNES,
        &[
            "1 100.0000 100.0000 100.0000 95.10649 100.0000 108.8440",
            "2 0.00000 0.00000 0.00000 1.000000 1.000000 1.000000",
            "3 100.0000 0.00000 0.00000 41.83017 22.05217 2.913230",
            "4 25.00000 50.00000 75.00000 20.0 21.0 40.0",
        ],
    )
}

#[test]
fn un_ti1_rvb_se_lit_dans_l_ordre() {
    let valeurs = lire_ti1(&quatre(), 4).unwrap();
    assert_eq!(
        valeurs,
        [
            [100.0, 100.0, 100.0],
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [25.0, 50.0, 75.0]
        ]
    );
}

#[test]
fn les_plages_suivent_l_ordre_des_sample_id() {
    let texte = ti1(COLONNES, &["2 0 0 0 1 1 1", "1 100 100 100 95 100 108"]);
    assert_eq!(
        lire_ti1(&texte, 2).unwrap(),
        [[100.0, 100.0, 100.0], [0.0, 0.0, 0.0]]
    );
}

#[test]
fn seul_le_premier_tableau_est_lu() {
    let annexe = ti1(
        "INDEX RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z",
        &["0 100.0000 0.00000 0.00000 41.8 22.0 2.9"],
    );
    let texte = format!("{}{annexe}", quatre());
    assert_eq!(lire_ti1(&texte, 4).unwrap().len(), 4);
}

#[test]
fn un_ti1_au_nombre_inattendu_est_refuse() {
    let texte = ti1(
        COLONNES,
        &["1 0 0 0 1 1 1", "2 50 50 50 20 21 22", "3 100 0 0 41 22 3"],
    );
    assert_eq!(
        lire_ti1(&texte, 4),
        Err(ErreurMire::NombreInattendu {
            attendu: 4,
            trouve: 3
        })
    );
}

#[test]
fn une_valeur_hors_echelle_est_refusee() {
    let texte = ti1(COLONNES, &["1 100.5 0 0 1 1 1"]);
    assert!(matches!(lire_ti1(&texte, 1), Err(ErreurMire::Ti1(_))));
    let texte = ti1(COLONNES, &["1 abc 0 0 1 1 1"]);
    assert!(matches!(lire_ti1(&texte, 1), Err(ErreurMire::Ti1(_))));
}

#[test]
fn un_ti1_sans_colonnes_rvb_est_refuse() {
    let texte = ti1("SAMPLE_ID CMYK_C CMYK_M CMYK_Y CMYK_K", &["1 0 0 0 0"]);
    assert!(matches!(lire_ti1(&texte, 1), Err(ErreurMire::Ti1(_))));
}

#[test]
fn un_fichier_qui_n_est_pas_un_ti1_est_refuse() {
    let texte = quatre().replacen("CTI1", "CTI3", 1);
    assert!(matches!(lire_ti1(&texte, 4), Err(ErreurMire::Ti1(_))));
    assert!(matches!(lire_ti1("", 4), Err(ErreurMire::Ti1(_))));
}
