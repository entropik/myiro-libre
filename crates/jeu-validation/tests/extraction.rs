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
    assert_eq!(a.source, "sortie-1");
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
    assert_eq!(p.source, "sortie-1");
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
        [
            "instrument-1",
            "instrument-2",
            "instrument-2",
            "instrument-1"
        ]
    );

    let texte = serde_json::to_string(&jeu).unwrap();
    for identifiant in ["12345678", "87654321", "02:00:00:00:00:01", "FDX-0000"] {
        assert!(
            !texte.contains(identifiant),
            "{identifiant} reste dans le jeu"
        );
    }
}

#[test]
fn le_format_courant_du_pont_donne_les_memes_paires_que_le_format_initial() {
    use pont_protocole::{ecrire_reponse, lire_reponse};
    // Les lignes de `JSONL` sont au format initial (pont 0.1.0) ; relues puis
    // réécrites, elles passent au format courant.
    let courant: String = JSONL
        .lines()
        .map(|ligne| format!("{}\n", ecrire_reponse(&lire_reponse(ligne).unwrap())))
        .collect();
    assert!(courant.contains("myiro-libre/mesure/1"));
    let lire = |contenu: &str| {
        extraire(&[SortieArchivee {
            nom: "seance",
            contenu,
        }])
        .unwrap()
    };
    assert_eq!(lire(&courant), lire(JSONL));
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

#[test]
fn le_nom_du_fichier_source_n_entre_pas_dans_le_jeu() {
    // Un nom de fichier peut contenir un numéro de série : seul un numéro d'ordre est gardé.
    let jeu = extraire(&[
        SortieArchivee {
            nom: "mesures-12345678.csv",
            contenu: CSV,
        },
        SortieArchivee {
            nom: "seance-12345678.jsonl",
            contenu: JSONL,
        },
    ])
    .unwrap();
    assert_eq!(jeu.paires[0].source, "sortie-1");
    assert_eq!(jeu.paires[2].source, "sortie-2");
    let texte = serde_json::to_string(&jeu).unwrap();
    assert!(!texte.contains("12345678"));
}

fn csv_avec_premier_spectre(valeur: &str) -> String {
    CSV.lines()
        .map(|l| match l.strip_prefix("A;M0;50;0;0;0.5;") {
            Some(reste) => format!(
                "A;M0;50;0;0;{valeur};{reste}
"
            ),
            None => format!(
                "{l}
"
            ),
        })
        .collect()
}

#[test]
fn une_ligne_sans_aucune_valeur_est_dite_trop_courte() {
    // Cinq champs : plage, données, L, a, b, et aucune valeur après.
    let abime = format!("{}\nC;M0;50;0;0\n", CSV.trim_end());
    let erreur = extraire(&[SortieArchivee {
        nom: "essai",
        contenu: &abime,
    }])
    .unwrap_err();
    assert!(erreur.contains("trop courte"), "{erreur}");
}

#[test]
fn une_valeur_non_finie_est_refusee_a_l_extraction() {
    for valeur in ["NaN", "inf", "-inf"] {
        let abime = csv_avec_premier_spectre(valeur);
        assert!(abime.contains(&format!("A;M0;50;0;0;{valeur};")));
        let resultat = extraire(&[SortieArchivee {
            nom: "essai",
            contenu: &abime,
        }]);
        assert!(resultat.is_err(), "{valeur} accepté");
    }
}

#[test]
fn une_plage_qui_revient_plus_loin_est_refusee() {
    // Les quatre lignes de A, recopiées après celles de B.
    let lignes_a: String = CSV
        .lines()
        .filter(|l| l.starts_with("A;"))
        .map(|l| {
            format!(
                "{l}
"
            )
        })
        .collect();
    let double = format!(
        "{}
{lignes_a}",
        CSV.trim_end()
    );
    let erreur = extraire(&[SortieArchivee {
        nom: "essai",
        contenu: &double,
    }])
    .unwrap_err();
    assert!(erreur.contains("A"), "{erreur}");
}
