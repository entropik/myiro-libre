//! Bibliothèque d'essai (ticket #86) : l'option `--bibliotheque <dossier>`
//! ouvre une autre bibliothèque que celle du poste, pour les essais d'écran.
//! Aucun test ne touche au dossier réel de l'utilisateur : chacun travaille
//! dans un dossier temporaire.

use std::path::PathBuf;

use app::colonne::BibliothequeOuverte;
use app::lancement::{bibliotheque_demandee, BibliothequeDemandee, RefusLancement};

fn args(liste: &[&str]) -> Vec<String> {
    std::iter::once("myiro-libre")
        .chain(liste.iter().copied())
        .map(String::from)
        .collect()
}

#[test]
fn sans_option_la_bibliotheque_du_poste_est_ouverte() {
    assert_eq!(
        bibliotheque_demandee(&args(&[])),
        Ok(BibliothequeDemandee::Poste)
    );
    assert_eq!(
        bibliotheque_demandee(&args(&["--langue", "en"])),
        Ok(BibliothequeDemandee::Poste)
    );
}

#[test]
fn avec_un_dossier_la_bibliotheque_d_essai_est_ouverte() {
    assert_eq!(
        bibliotheque_demandee(&args(&["--bibliotheque", r"C:\essais\copie"])),
        Ok(BibliothequeDemandee::Essai(PathBuf::from(
            r"C:\essais\copie"
        )))
    );
    assert_eq!(
        bibliotheque_demandee(&args(&["--langue", "en", "--bibliotheque", "copie"])),
        Ok(BibliothequeDemandee::Essai(PathBuf::from("copie")))
    );
}

#[test]
fn la_forme_avec_egal_est_comprise() {
    assert_eq!(
        bibliotheque_demandee(&args(&["--bibliotheque=copie"])),
        Ok(BibliothequeDemandee::Essai(PathBuf::from("copie")))
    );
}

/// Jamais de repli silencieux sur la vraie bibliothèque.
#[test]
fn l_option_sans_dossier_est_refusee() {
    for liste in [
        &["--bibliotheque"][..],
        &["--bibliotheque", ""],
        &["--bibliotheque", "   "],
        &["--bibliotheque="],
        &["--bibliotheque", "--langue", "en"],
    ] {
        assert_eq!(
            bibliotheque_demandee(&args(liste)),
            Err(RefusLancement::BibliothequeSansDossier),
            "{liste:?}"
        );
    }
}

#[test]
fn le_refus_dit_quoi_faire() {
    let message = RefusLancement::BibliothequeSansDossier.to_string();
    assert!(message.contains("--bibliotheque <dossier>"), "{message}");
}

#[test]
fn la_demonstration_l_emporte_sur_la_bibliotheque_d_essai() {
    assert_eq!(
        bibliotheque_demandee(&args(&["--demo"])),
        Ok(BibliothequeDemandee::Demonstration { essai_ecarte: None })
    );
    assert_eq!(
        bibliotheque_demandee(&args(&["--bibliotheque", "copie", "--demo"])),
        Ok(BibliothequeDemandee::Demonstration {
            essai_ecarte: Some(PathBuf::from("copie"))
        })
    );
}

#[test]
fn la_demonstration_ne_couvre_pas_une_option_sans_dossier() {
    assert_eq!(
        bibliotheque_demandee(&args(&["--demo", "--bibliotheque"])),
        Err(RefusLancement::BibliothequeSansDossier)
    );
}

/// Le dossier donné est la bibliothèque elle-même : créé s'il manque, puis
/// relu tel quel au lancement suivant.
#[test]
fn la_bibliotheque_d_essai_est_creee_puis_rouverte_dans_son_dossier() {
    let temporaire = tempfile::tempdir().unwrap();
    let dossier = temporaire.path().join("copie");
    let demande = bibliotheque_demandee(&args(&["--bibliotheque", dossier.to_str().unwrap()]));
    let Ok(BibliothequeDemandee::Essai(essai)) = demande else {
        panic!("bibliothèque d'essai attendue : {demande:?}");
    };
    assert!(essai.starts_with(temporaire.path()));

    let ouverte = BibliothequeOuverte::ouvrir(&essai);
    ouverte.creer_condition("Essai PRO-4100").unwrap();
    drop(ouverte);
    assert!(dossier.join(bibliotheque::FICHIER_BASE).is_file());

    let rouverte = BibliothequeOuverte::ouvrir(&essai);
    let noms: Vec<String> = rouverte
        .conditions()
        .unwrap()
        .into_iter()
        .map(|c| c.nom)
        .collect();
    assert_eq!(noms, ["Essai PRO-4100"]);
}
