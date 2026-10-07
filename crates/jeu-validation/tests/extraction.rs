//! Seam 1 : `extraire(sorties du pont archivées) -> JeuValidation`.
//! Données synthétiques versionnées dans `tests/donnees/` ; identifiants fictifs.

use jeu_validation::{extraire, Condition, SortieArchivee};

const CSV: &str = include_str!("donnees/mesures-synthetiques.csv");

#[test]
fn un_csv_du_pont_donne_une_paire_par_plage() {
    let jeu = extraire(&[SortieArchivee {
        nom: "essai",
        contenu: CSV,
    }])
    .unwrap();

    assert_eq!(jeu.paires.len(), 2);
    let a = &jeu.paires[0];
    assert_eq!(a.source, "essai");
    assert_eq!(a.plage, "A");
    assert_eq!(a.instrument, None);
    assert_eq!(a.brutes.len(), 152);
    assert_eq!(a.brutes[0], 0.5);
    assert_eq!(a.brutes[151], 2057.25);
    assert_eq!(a.spectre(Condition::M0), &[0.5; 36]);
    assert_eq!(a.spectre(Condition::M1)[35], 0.6);
    assert_eq!(a.spectre(Condition::M2)[0], 0.9);
    assert_eq!(a.spectre(Condition::M2)[1], 0.5);
    assert_eq!(jeu.paires[1].plage, "B");
}

const JSONL: &str = include_str!("donnees/sorties-pont-synthetiques.jsonl");

#[test]
fn les_lignes_json_du_pont_donnent_une_paire_par_plage_mesuree() {
    let jeu = extraire(&[SortieArchivee {
        nom: "seance",
        contenu: JSONL,
    }])
    .unwrap();

    // Trois réponses « mesure » : 1 + 2 + 1 plages ; les autres réponses sont ignorées.
    assert_eq!(jeu.paires.len(), 4);
    let p = &jeu.paires[1];
    assert_eq!(p.source, "seance");
    assert_eq!(p.plage, "mesure-2/plage-1");
    assert_eq!(p.brutes[0], 0.4);
    assert_eq!(p.spectre(Condition::M1)[0], 0.5);
    let origine = p.instrument.as_ref().unwrap();
    assert_eq!(origine.modele, "MYIRO-1");
    assert_eq!(origine.micrologiciel, "1.00");
    assert_eq!(origine.version_sdk, [1, 0, 1]);
    assert_eq!(origine.calcul, "D50, 2 degres");
}

#[test]
fn le_numero_de_serie_est_remplace_par_un_pseudonyme_stable() {
    let jeu = extraire(&[SortieArchivee {
        nom: "seance",
        contenu: JSONL,
    }])
    .unwrap();
    let pseudonymes: Vec<&str> = jeu
        .paires
        .iter()
        .map(|p| p.instrument.as_ref().unwrap().pseudonyme.as_str())
        .collect();
    assert_eq!(
        pseudonymes,
        ["instrument-1", "instrument-2", "instrument-2", "instrument-1"]
    );

    let texte = serde_json::to_string(&jeu).unwrap();
    for identifiant in ["12345678", "87654321", "00:00:5E:00:53:01", "FDX-0000"] {
        assert!(!texte.contains(identifiant), "{identifiant} reste dans le jeu");
    }
}

#[test]
fn une_ligne_json_illisible_est_refusee() {
    let abime = format!("{JSONL}{{\"rep\":\"mesure\"}}\n");
    assert!(extraire(&[SortieArchivee {
        nom: "seance",
        contenu: &abime,
    }])
    .is_err());
}

#[test]
fn une_plage_sans_ses_donnees_brutes_est_refusee() {
    let tronque: String = CSV
        .lines()
        .filter(|l| !l.starts_with("B;brutes"))
        .map(|l| format!("{l}\n"))
        .collect();
    let erreur = extraire(&[SortieArchivee {
        nom: "essai",
        contenu: &tronque,
    }])
    .unwrap_err();
    assert!(erreur.contains("B"), "{erreur}");
}

#[test]
fn des_donnees_brutes_de_mauvaise_longueur_sont_refusees() {
    // Retire la dernière valeur brute de chaque plage (fins de ligne LF ou CRLF).
    let abime: String = CSV
        .lines()
        .map(|l| format!("{}\n", l.strip_suffix(";2057.25").unwrap_or(l)))
        .collect();
    assert_ne!(abime.replace("\r\n", "\n"), CSV.replace("\r\n", "\n"));
    assert!(extraire(&[SortieArchivee {
        nom: "essai",
        contenu: &abime,
    }])
    .is_err());
}
