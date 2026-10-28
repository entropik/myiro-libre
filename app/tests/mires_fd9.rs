//! Préparer la mire FD-9 RGB-2520, vu par l'application : copie telle
//! quelle, refus en clair, recherche du fichier. La mire est fictive, de même
//! forme que RGB-2520 ; les erreurs sont des clés du catalogue de textes.

mod commun_mires;

use std::time::Duration;

use app::mires::{chercher_mire_fd9, DemandeMire};
use commun_mires::*;
use mires::{Format, TailleMire, RGB2520_FICHIER};

#[test]
fn la_mire_fd9_est_copiee_telle_quelle() {
    let b = banc();
    let (octets, mire) = lire_fictive();
    let preparee = b
        .ouverte
        .preparer_mire_fd9(b.condition, mire, &b.mires(), maintenant())
        .unwrap();
    assert_eq!((preparee.plages, preparee.pages), (24, 1));
    assert_eq!(preparee.commande, "");
    assert_eq!(preparee.fichiers.len(), 1);
    assert_eq!(std::fs::read(&preparee.fichiers[0]).unwrap(), octets);
    assert_eq!(b.tirages(), 1);
}

#[test]
fn une_mire_fd9_absente_ne_prepare_rien() {
    let b = banc();
    let m = moteur(232);
    assert_eq!(
        b.preparer(&fd9(), &m, None).map(|_| ()),
        Err("mire.erreur.fd9_introuvable".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
    assert!(m.plages_demandees().is_empty());
}

#[test]
fn une_mire_fd9_d_une_autre_empreinte_est_refusee() {
    let b = banc();
    let (octets, _) = mire_fd9_fictive();
    let fichier = b.dossier.path().join(RGB2520_FICHIER);
    std::fs::write(&fichier, &octets).unwrap();
    assert_eq!(
        b.preparer(&fd9(), &moteur(232), Some(&fichier)).map(|_| ()),
        Err("mire.erreur.fd9_autre".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn une_mire_fd9_n_accepte_ni_taille_ni_format() {
    let b = banc();
    let m = moteur(232);
    for demande in [
        DemandeMire {
            taille: Some(TailleMire::Rapide),
            ..fd9()
        },
        DemandeMire {
            format: Some(Format::A3),
            ..fd9()
        },
        DemandeMire {
            nombre: Some(100),
            ..fd9()
        },
        DemandeMire {
            cote_mm: Some(10.0),
            ..fd9()
        },
    ] {
        assert_eq!(
            b.ouverte.apercu_mire(&demande, &m).map(|_| ()),
            Err("mire.erreur.fd9_fixe".to_string())
        );
        assert_eq!(
            b.preparer(&demande, &m, None).map(|_| ()),
            Err("mire.erreur.fd9_fixe".to_string())
        );
    }
    assert_eq!(b.tirages(), 0);
}

#[test]
fn deux_preparations_fd9_donnent_deux_tirages_d_une_meme_mire() {
    let b = banc();
    let premiere = b
        .ouverte
        .preparer_mire_fd9(b.condition, lire_fictive().1, &b.mires(), maintenant())
        .unwrap();
    let seconde = b
        .ouverte
        .preparer_mire_fd9(
            b.condition,
            lire_fictive().1,
            &b.mires(),
            maintenant() + Duration::from_secs(60),
        )
        .unwrap();
    assert_ne!(premiere.tirage, seconde.tirage);
    let tirages = b.ouverte.detail_reglage(b.condition).unwrap().tirages;
    assert_eq!(tirages.len(), 2);
    assert_eq!(tirages[0].mire, tirages[1].mire);
    assert_eq!(tirages[0].instrument, "fd9");
    assert_eq!(b.fichiers().len(), 2);
}

/// La mire FD-9 suit la même règle : copie impossible, rien n'est rangé.
#[test]
fn une_mire_fd9_qui_ne_se_copie_pas_ne_range_rien() {
    let b = banc();
    std::fs::write(b.mires(), b"occupe").unwrap();
    assert_eq!(
        b.ouverte
            .preparer_mire_fd9(b.condition, lire_fictive().1, &b.mires(), maintenant())
            .map(|_| ()),
        Err("mire.erreur.ecriture".to_string())
    );
    assert_eq!(b.tirages(), 0);
}

#[test]
fn la_mire_fd9_se_cherche_a_cote_de_l_application_puis_a_la_racine_du_depot() {
    let racine = tempfile::tempdir().unwrap();
    let exe = racine.path().join("target").join("debug");
    std::fs::create_dir_all(&exe).unwrap();
    assert_eq!(chercher_mire_fd9(&exe), None);
    let depot = racine.path().join("mire-fd9");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::write(depot.join(RGB2520_FICHIER), b"x").unwrap();
    assert_eq!(
        chercher_mire_fd9(&exe).map(|c| c.canonicalize().unwrap()),
        Some(depot.join(RGB2520_FICHIER).canonicalize().unwrap())
    );
    let installe = exe.join("mire-fd9");
    std::fs::create_dir_all(&installe).unwrap();
    std::fs::write(installe.join(RGB2520_FICHIER), b"x").unwrap();
    assert_eq!(
        chercher_mire_fd9(&exe),
        Some(installe.join(RGB2520_FICHIER))
    );
}
