//! Préparer une mire MYIRO-1, vu par l'application : méthodes de
//! `BibliothequeOuverte` (sans Tauri) et moteur simulé, jamais ArgyllCMS.
//! Aussi : nom des fichiers, aperçu, dépôt des pages et clés de texte. Les
//! erreurs sont des clés du catalogue de textes.

mod commun_mires;

use app::mires::{nom_fichier_page, DemandeMire};
use app::textes::{texte, Langue};
use commun_mires::*;
use mires::{ErreurMire, Format, TailleMire};
use profilage::{ErreurMoteur, MoteurSimule};

#[test]
fn une_mire_preparee_ecrit_ses_pages_et_range_son_tirage() {
    let b = banc();
    let m = moteur(232);
    let preparee = b.preparer(&myiro1(TailleMire::Rapide), &m, None).unwrap();
    assert_eq!((preparee.plages, preparee.pages), (232, 1));
    assert_eq!(preparee.commande, "targen -v -d2 -G -f232 mire");
    assert_eq!(preparee.dossier, b.mires());
    assert_eq!(preparee.fichiers.len(), 1);
    assert!(preparee.fichiers[0].is_file());
    assert_eq!(
        b.fichiers(),
        [format!(
            "Canon PRO-4100 - Hahnemühle Photo Rag 308 - 720 dpi - 2026-10-23 - T{} - page 1 sur 1.tif",
            preparee.tirage
        )]
    );
    assert_eq!(m.plages_demandees(), [232]);
    assert_eq!(b.tirages(), 1);
    // Une page TIFF : en-tête petit-boutiste.
    let octets = std::fs::read(&preparee.fichiers[0]).unwrap();
    assert_eq!(&octets[..4], b"II*\0");
}

#[test]
fn une_mire_de_deux_pages_ecrit_deux_fichiers_nommes() {
    let b = banc();
    let preparee = b
        .preparer(&myiro1(TailleMire::Approfondi), &moteur(928), None)
        .unwrap();
    assert_eq!(preparee.pages, 2);
    let fichiers = b.fichiers();
    assert_eq!(fichiers.len(), 2);
    assert!(fichiers[0].ends_with("page 1 sur 2.tif"), "{fichiers:?}");
    assert!(fichiers[1].ends_with("page 2 sur 2.tif"), "{fichiers:?}");
    assert!(!fichiers.iter().any(|f| f.ends_with(".part")));
}

#[test]
fn le_nom_de_fichier_est_nettoye() {
    assert_eq!(
        nom_fichier_page(
            "Canon PRO-4100",
            "Canson « Baryta » / 2",
            "600 dpi",
            "2026-10-23",
            42,
            1,
            2
        ),
        "Canon PRO-4100 - Canson Baryta 2 - 600 dpi - 2026-10-23 - T42 - page 1 sur 2.tif"
    );
    let nom = nom_fichier_page(
        "HP: <Latex>",
        "a\\b|c?d*e\"f",
        "\t1200",
        "2026-10-23",
        1,
        1,
        1,
    );
    assert!(
        !nom.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\t']),
        "{nom}"
    );
}

#[test]
fn l_apercu_dit_plages_pages_et_commande_sans_rien_lancer() {
    let b = banc();
    let m = moteur(928);
    let apercu = b
        .ouverte
        .apercu_mire(&myiro1(TailleMire::Approfondi), &m)
        .unwrap();
    assert_eq!((apercu.plages, apercu.pages), (928, 2));
    assert_eq!(apercu.cote_mm, 8.0);
    assert_eq!(apercu.limites_cote, (7.0, None));
    assert_eq!(apercu.commande, "targen -v -d2 -G -f928 mire");
    let libre = DemandeMire {
        taille: None,
        nombre: Some(500),
        format: Some(Format::A3),
        cote_mm: Some(10.0),
        ..myiro1(TailleMire::Rapide)
    };
    let apercu = b.ouverte.apercu_mire(&libre, &m).unwrap();
    assert_eq!((apercu.plages, apercu.cote_mm), (500, 10.0));
    let apercu = b.ouverte.apercu_mire(&fd9(), &m).unwrap();
    assert_eq!((apercu.plages, apercu.pages), (2520, 1));
    assert_eq!(apercu.commande, "");
    assert!(m.plages_demandees().is_empty());
}

#[test]
fn un_cote_hors_limites_ne_lance_pas_targen() {
    let b = banc();
    let m = moteur(232);
    let petit = DemandeMire {
        cote_mm: Some(6.0),
        ..myiro1(TailleMire::Rapide)
    };
    assert_eq!(
        b.preparer(&petit, &m, None).map(|_| ()),
        Err("mire.erreur.cote".to_string())
    );
    assert_eq!(
        b.ouverte.apercu_mire(&petit, &m).map(|_| ()),
        Err("mire.erreur.cote".to_string())
    );
    for nombre in [23, 3001] {
        let libre = DemandeMire {
            taille: None,
            nombre: Some(nombre),
            ..myiro1(TailleMire::Rapide)
        };
        assert_eq!(
            b.preparer(&libre, &m, None).map(|_| ()),
            Err("mire.erreur.nombre_libre".to_string())
        );
    }
    assert!(m.plages_demandees().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn argyll_introuvable_ne_prepare_rien() {
    let b = banc();
    let m = MoteurSimule::echoue(ErreurMoteur::Introuvable).avec_plages(ti1(232));
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &m, None)
            .map(|_| ()),
        Err("profil.erreur.argyll_introuvable".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn un_ti1_au_nombre_inattendu_ne_range_rien() {
    let b = banc();
    let m = MoteurSimule::reussit(Vec::new()).avec_plages(ti1(200));
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &m, None)
            .map(|_| ()),
        Err("mire.erreur.nombre".to_string())
    );
    assert_eq!(b.tirages(), 0);
}

#[test]
fn un_refus_de_mise_en_page_ne_range_rien() {
    let b = banc();
    let m = MoteurSimule::reussit(Vec::new()).avec_plages(ti1_clair(30));
    let libre = DemandeMire {
        taille: None,
        nombre: Some(30),
        ..myiro1(TailleMire::Rapide)
    };
    assert_eq!(
        b.preparer(&libre, &m, None).map(|_| ()),
        Err("mire.erreur.contraste".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn une_ecriture_impossible_ne_laisse_aucun_fichier() {
    let b = banc();
    // Le dossier des mires est occupé par un fichier : rien ne peut s'y écrire.
    std::fs::write(b.mires(), b"occupe").unwrap();
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &moteur(232), None)
            .map(|_| ()),
        Err("mire.erreur.ecriture".to_string())
    );
    assert_eq!(b.tirages(), 0, "un tirage sans fichiers");
}

/// Un tirage n'apparaît jamais sans ses fichiers : si la place de la
/// deuxième page est prise, aucune page n'est déposée, aucun dossier de
/// travail ne reste et rien n'est rangé.
#[test]
fn une_page_qui_ne_se_depose_pas_ne_laisse_ni_tirage_ni_fichier() {
    let b = banc();
    // La place de la page 2 du premier tirage est prise par un dossier.
    let page_2 =
        "Canon PRO-4100 - Hahnemühle Photo Rag 308 - 720 dpi - 2026-10-23 - T1 - page 2 sur 2.tif";
    std::fs::create_dir_all(b.mires().join(page_2).join("occupe")).unwrap();
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Approfondi), &moteur(928), None)
            .map(|_| ()),
        Err("mire.erreur.fichier_existant".to_string())
    );
    assert_eq!(b.tirages(), 0);
    assert_eq!(b.fichiers(), [page_2]);
    // Une préparation suivante réussit, avec ses deux pages.
    std::fs::remove_dir_all(b.mires().join(page_2)).unwrap();
    let preparee = b
        .preparer(&myiro1(TailleMire::Approfondi), &moteur(928), None)
        .unwrap();
    assert_eq!(preparee.fichiers.len(), 2);
    assert_eq!(b.tirages(), 1);
    assert_eq!(b.fichiers().len(), 2);
}

/// Un fichier de même nom (numéros de tirage repartis après la restauration
/// d'une ancienne sauvegarde) n'est jamais écrasé : rien n'est rangé et le
/// message le dit.
#[test]
fn une_page_deja_presente_n_est_pas_ecrasee() {
    let b = banc();
    let page_1 =
        "Canon PRO-4100 - Hahnemühle Photo Rag 308 - 720 dpi - 2026-10-23 - T1 - page 1 sur 1.tif";
    std::fs::create_dir_all(b.mires()).unwrap();
    std::fs::write(b.mires().join(page_1), b"ancienne page").unwrap();
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &moteur(232), None)
            .map(|_| ()),
        Err("mire.erreur.fichier_existant".to_string())
    );
    assert_eq!(
        std::fs::read(b.mires().join(page_1)).unwrap(),
        b"ancienne page"
    );
    assert_eq!(b.fichiers(), [page_1]);
    assert_eq!(b.tirages(), 0);
}

#[test]
fn toutes_les_cles_mire_existent_en_deux_langues() {
    let mut cles: Vec<&str> = [
        ErreurMire::CoteHorsLimites {
            min: 7.0,
            max: None,
        },
        ErreurMire::Ti1(String::new()),
        ErreurMire::NombreInattendu {
            attendu: 1,
            trouve: 0,
        },
        ErreurMire::Contraste { defauts: 1 },
        ErreurMire::Ecriture(String::new()),
        ErreurMire::MireFd9Introuvable,
        ErreurMire::MireFd9Autre {
            empreinte: String::new(),
        },
        ErreurMire::MireFd9Illisible(String::new()),
        ErreurMire::FichierExistant(String::new()),
    ]
    .iter()
    .map(ErreurMire::cle)
    .collect();
    cles.extend([
        "mire.erreur.generation",
        "mire.erreur.fd9_fixe",
        "mire.erreur.nombre_libre",
    ]);
    for cle in cles {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} absente en {langue:?}");
        }
    }
}
