//! Fichier `.ti3` écrit pour ArgyllCMS, vu par l'API publique de la crate.
//! Toutes les plages sont fictives.

use pont_protocole::{ConditionMesure, Echantillonnage};
use profilage::{ecrire_ti3, PlageProfil, PlagesProfil};

fn plages_fictives() -> PlagesProfil {
    PlagesProfil {
        plages: vec![
            PlageProfil {
                rvb: [100.0, 100.0, 100.0],
                lab: [95.12, -0.375, 2.5],
                spectre: None,
            },
            PlageProfil {
                rvb: [0.0, 46.121_568_627_450_98, 78.058_823_529_411_77],
                lab: [42.09, -28.0, -60.57],
                spectre: None,
            },
            PlageProfil {
                rvb: [0.0, 0.0, 0.0],
                lab: [3.5, 0.125, -0.0625],
                spectre: None,
            },
        ],
        condition: ConditionMesure::M1,
        condition_supposee: true,
        rvb_supposes: true,
        longueurs_onde: None,
    }
}

fn avec_spectres() -> PlagesProfil {
    let mut p = plages_fictives();
    for (n, plage) in p.plages.iter_mut().enumerate() {
        plage.spectre = Some((0..36).map(|i| n as f64 * 10.0 + i as f64 * 0.5).collect());
    }
    p.longueurs_onde = Some(Echantillonnage {
        debut_nm: 380,
        pas_nm: 10,
    });
    p
}

fn ti3(plages: &PlagesProfil) -> String {
    ecrire_ti3(plages, "PRO-4100 Rag 2026-10-23", "2026-10-23T09:00:00Z")
}

/// Lignes de données (entre `BEGIN_DATA` et `END_DATA`), découpées en champs.
fn donnees(texte: &str) -> Vec<Vec<String>> {
    texte
        .lines()
        .skip_while(|l| *l != "BEGIN_DATA")
        .skip(1)
        .take_while(|l| *l != "END_DATA")
        .map(|l| l.split_whitespace().map(str::to_string).collect())
        .collect()
}

/// Liste des champs, entre `BEGIN_DATA_FORMAT` et `END_DATA_FORMAT`.
fn champs(texte: &str) -> Vec<String> {
    let ligne = texte
        .lines()
        .skip_while(|l| *l != "BEGIN_DATA_FORMAT")
        .nth(1)
        .unwrap();
    ligne.split_whitespace().map(str::to_string).collect()
}

#[test]
fn le_ti3_a_l_en_tete_attendu_par_argyll() {
    let t = ti3(&plages_fictives());
    assert!(t.starts_with("CTI3\r\n"), "{t}");
    for l in [
        "DESCRIPTOR \"PRO-4100 Rag 2026-10-23\"",
        "ORIGINATOR \"myiro-libre\"",
        "CREATED \"2026-10-23T09:00:00Z\"",
        "KEYWORD \"DEVICE_CLASS\"",
        "DEVICE_CLASS \"OUTPUT\"",
        "KEYWORD \"COLOR_REP\"",
        "COLOR_REP \"iRGB_LAB\"",
        "INSTRUMENT_TYPE_SPECTRAL \"NO\"",
        "NUMBER_OF_FIELDS 7",
        "NUMBER_OF_SETS 3",
        "SAMPLE_ID RGB_R RGB_G RGB_B LAB_L LAB_A LAB_B",
    ] {
        assert!(t.contains(&format!("{l}\r\n")), "{l} absent de :\n{t}");
    }
    assert!(!t.contains("SPECTRAL_BANDS"));
    assert!(t.ends_with("END_DATA\r\n"));
    assert!(
        !t.replace("\r\n", "").contains('\n'),
        "fin de ligne \\n seule"
    );
}

#[test]
fn les_identifiants_du_ti3_sont_les_rangs() {
    let lignes = donnees(&ti3(&plages_fictives()));
    let ids: Vec<&str> = lignes.iter().map(|l| l[0].as_str()).collect();
    assert_eq!(ids, ["1", "2", "3"]);
}

#[test]
fn avec_spectres_les_bandes_sont_declarees() {
    let t = ti3(&avec_spectres());
    for l in [
        "INSTRUMENT_TYPE_SPECTRAL \"YES\"",
        "KEYWORD \"SPECTRAL_BANDS\"",
        "SPECTRAL_BANDS \"36\"",
        "SPECTRAL_START_NM \"380\"",
        "SPECTRAL_END_NM \"730\"",
        "NUMBER_OF_FIELDS 43",
    ] {
        assert!(t.contains(&format!("{l}\r\n")), "{l} absent de :\n{t}");
    }
    let c = champs(&t);
    assert_eq!(c.len(), 43);
    assert_eq!(c[7], "SPEC_380");
    assert_eq!(c[8], "SPEC_390");
    assert_eq!(c[42], "SPEC_730");
    // Spectre en pourcentage, tel que préparé : 20 + 0,5 × 35 pour la 3e plage.
    let lignes = donnees(&t);
    assert_eq!(lignes[2][42], "37.5");
    assert_eq!(lignes[2].len(), 43);
}

#[test]
fn une_description_avec_guillemets_est_doublee() {
    let t = ecrire_ti3(
        &plages_fictives(),
        "Hahnemühle Canson \"Baryta\" 308",
        "2026-10-23T09:00:00Z",
    );
    assert!(
        t.contains("DESCRIPTOR \"Hahnemühle Canson \"\"Baryta\"\" 308\"\r\n"),
        "{t}"
    );
}

#[test]
fn une_description_sur_plusieurs_lignes_reste_sur_une_seule() {
    let t = ecrire_ti3(
        &plages_fictives(),
        "Rag\r\n308\tmat",
        "2026-10-23T09:00:00Z",
    );
    assert!(t.contains("DESCRIPTOR \"Rag  308 mat\"\r\n"), "{t}");
}

#[test]
fn les_nombres_se_relisent_a_l_identique() {
    for plages in [plages_fictives(), avec_spectres()] {
        let lignes = donnees(&ti3(&plages));
        assert_eq!(lignes.len(), plages.plages.len());
        for (ligne, plage) in lignes.iter().zip(&plages.plages) {
            let relu: Vec<f64> = ligne[1..].iter().map(|v| v.parse().unwrap()).collect();
            let mut attendu: Vec<f64> = plage.rvb.iter().chain(&plage.lab).copied().collect();
            attendu.extend(plage.spectre.iter().flatten());
            assert_eq!(relu, attendu);
        }
    }
}
