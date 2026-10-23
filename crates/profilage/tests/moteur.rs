//! Moteur de profil vu par l'API publique de la crate : arguments de
//! `colprof`, lecture des sorties d'ArgyllCMS, recherche du dossier et moteur
//! simulé. Rien ici ne lance ArgyllCMS.

use std::fs;
use std::path::Path;

use profilage::{
    arguments_colprof, chercher_argyll, controler_version, lire_precision, lire_version,
    DemandeProfil, ErreurMoteur, Etape, MoteurProfil, MoteurSimule, Precision, Recette,
    VERSION_ARGYLL,
};

fn textes(recette: Recette, description: &str) -> Vec<String> {
    arguments_colprof(
        recette,
        description,
        Path::new("ref/sRGB.icm"),
        Path::new("p.icc"),
        Path::new("mesure"),
    )
    .iter()
    .map(|s| s.to_string_lossy().into_owned())
    .collect()
}

#[test]
fn la_recette_standard_demande_la_qualite_haute() {
    assert_eq!(
        textes(Recette::Standard, "Essai"),
        [
            "-v",
            "-qh",
            "-S",
            "ref/sRGB.icm",
            "-cmt",
            "-dpp",
            "-D",
            "Essai",
            "-O",
            "p.icc",
            "mesure"
        ]
    );
}

#[test]
fn rapide_et_approfondi_changent_seulement_la_qualite() {
    let standard = textes(Recette::Standard, "Essai");
    for (recette, qualite) in [(Recette::Rapide, "-qm"), (Recette::Approfondi, "-qu")] {
        let mut attendu = standard.clone();
        attendu[1] = qualite.to_string();
        assert_eq!(textes(recette, "Essai"), attendu);
    }
}

#[test]
fn une_description_accentuee_avec_guillemets_reste_un_seul_argument() {
    let description = "Hahnemühle \"Rag\" 308";
    let a = textes(Recette::Standard, description);
    assert_eq!(a.len(), 11);
    assert_eq!(a.iter().filter(|s| s.as_str() == description).count(), 1);
    let rang = a.iter().position(|s| s == "-D").unwrap();
    assert_eq!(a[rang + 1], description);
}

#[test]
fn la_recette_s_enregistre_en_json() {
    assert_eq!(
        serde_json::to_string(&Recette::Standard).unwrap(),
        "\"standard\""
    );
    let r: Recette = serde_json::from_str("\"approfondi\"").unwrap();
    assert_eq!(r, Recette::Approfondi);
}

#[test]
fn la_precision_se_lit_dans_la_sortie_de_profcheck() {
    let p = lire_precision(
        "Profile check complete, peak err = 2.512836, avg err = 0.448123, RMS = 0.6\n",
    )
    .unwrap();
    assert_eq!((p.max_de00, p.moyenne_de00), (2.512836, 0.448123));
}

#[test]
fn la_precision_se_lit_au_milieu_d_un_journal() {
    let sortie = "[1] 0.1 0.2 0.3\nProfile check complete, peak err = 1.5, avg err = 0.25, RMS = 0.3\r\nfin\n";
    assert_eq!(
        lire_precision(sortie),
        Some(Precision {
            moyenne_de00: 0.25,
            max_de00: 1.5
        })
    );
}

#[test]
fn une_sortie_sans_precision_donne_rien() {
    assert_eq!(lire_precision(""), None);
    assert_eq!(lire_precision("profcheck: Error - file not found\n"), None);
    assert_eq!(
        lire_precision("Profile check complete, peak err = abc, avg err = 0.4\n"),
        None
    );
}

fn poser_colprof(racine: &Path) {
    let bin = racine.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(
        bin.join(format!("colprof{}", std::env::consts::EXE_SUFFIX)),
        "",
    )
    .unwrap();
}

#[test]
fn argyll_est_cherche_a_cote_de_l_executable_puis_a_la_racine_du_depot() {
    let depot = tempfile::tempdir().unwrap();
    let exe = depot.path().join("target").join("debug");
    fs::create_dir_all(&exe).unwrap();
    assert_eq!(chercher_argyll(&exe), None);

    // Développement : <dépôt>/argyll, deux niveaux au-dessus de target/debug.
    poser_colprof(&depot.path().join("argyll"));
    let trouve = chercher_argyll(&exe).unwrap();
    assert!(trouve.join("bin").exists());
    assert_eq!(
        fs::canonicalize(&trouve).unwrap(),
        fs::canonicalize(depot.path().join("argyll")).unwrap()
    );

    // Installation : <exe>/argyll passe avant.
    poser_colprof(&exe.join("argyll"));
    assert_eq!(chercher_argyll(&exe), Some(exe.join("argyll")));
}

#[test]
fn un_dossier_argyll_sans_colprof_est_ignore() {
    let exe = tempfile::tempdir().unwrap();
    fs::create_dir_all(exe.path().join("argyll").join("bin")).unwrap();
    assert_eq!(chercher_argyll(exe.path()), None);
}

#[test]
fn une_autre_version_d_argyll_est_refusee() {
    let trouvee =
        lire_version("Create ICC profile, Version 3.4.1\n usage: colprof [-options] inoutfile\n");
    assert_eq!(trouvee.as_deref(), Some("3.4.1"));
    assert_eq!(
        controler_version("3.4.1"),
        Err(ErreurMoteur::VersionNonValidee {
            trouvee: "3.4.1".into()
        })
    );
    assert_eq!(controler_version(VERSION_ARGYLL), Ok(()));
    assert_eq!(lire_version("usage: colprof [-options]\n"), None);
}

#[test]
fn chaque_erreur_a_sa_cle_de_texte() {
    let cas = [
        (
            ErreurMoteur::Introuvable,
            "profil.erreur.argyll_introuvable",
        ),
        (
            ErreurMoteur::VersionNonValidee {
                trouvee: "3.4.1".into(),
            },
            "profil.erreur.argyll_version",
        ),
        (
            ErreurMoteur::Echec {
                etape: Etape::Version,
                cause: "x".into(),
            },
            "profil.erreur.argyll_version",
        ),
        (
            ErreurMoteur::Echec {
                etape: Etape::Profil,
                cause: "x".into(),
            },
            "profil.erreur.calcul",
        ),
        (
            ErreurMoteur::Echec {
                etape: Etape::Controle,
                cause: "x".into(),
            },
            "profil.erreur.controle",
        ),
        (ErreurMoteur::Fichier("x".into()), "profil.erreur.fichier"),
    ];
    for (erreur, cle) in cas {
        assert_eq!(erreur.cle(), cle, "{erreur}");
    }
}

#[test]
fn le_moteur_simule_rend_son_resultat_et_garde_les_demandes() {
    let moteur = MoteurSimule::reussit(vec![1, 2, 3]);
    assert!(moteur.demandes().is_empty());
    let demande = DemandeProfil {
        ti3: "CTI3\r\n",
        description: "Essai",
        recette: Recette::Rapide,
    };

    // Montrer les commandes ne lance rien.
    let commandes = moteur.commandes(&demande);
    assert!(moteur.demandes().is_empty());
    assert_eq!(commandes.len(), 2);
    assert!(
        commandes[0].starts_with("colprof -v -qm "),
        "{}",
        commandes[0]
    );
    assert!(
        commandes[1].starts_with("profcheck -k "),
        "{}",
        commandes[1]
    );

    let profil = moteur.calculer(&demande).unwrap();
    assert_eq!(profil.icc, [1, 2, 3]);
    assert_eq!(profil.version, VERSION_ARGYLL);
    assert_eq!(profil.commandes, commandes);
    assert_eq!(
        moteur.demandes(),
        [("CTI3\r\n".to_string(), Recette::Rapide)]
    );
}

#[test]
fn le_moteur_simule_peut_echouer() {
    let erreur = ErreurMoteur::Echec {
        etape: Etape::Profil,
        cause: "colprof: Error - too few patches".into(),
    };
    let moteur = MoteurSimule::echoue(erreur.clone());
    let demande = DemandeProfil {
        ti3: "CTI3\r\n",
        description: "Essai",
        recette: Recette::Standard,
    };
    assert_eq!(moteur.calculer(&demande), Err(erreur));
    assert_eq!(moteur.demandes().len(), 1);
}

#[test]
fn les_commandes_montrees_citent_une_description_avec_espaces() {
    let moteur = MoteurSimule::reussit(Vec::new());
    let commandes = moteur.commandes(&DemandeProfil {
        ti3: "",
        description: "PRO-4100 Rag",
        recette: Recette::Standard,
    });
    assert_eq!(
        commandes,
        [
            "colprof -v -qh -S sRGB.icm -cmt -dpp -D \"PRO-4100 Rag\" -O profil.icc mesure",
            "profcheck -k mesure.ti3 profil.icc"
        ]
    );
}
