//! Préparer deux fois la même mire MYIRO-1 dans une résolution ne fait pas
//! de doublon : tant que le tirage n'est pas mesuré, il est repris avec ses
//! fichiers, sans relancer `targen`. Un réglage différent (taille, côté)
//! donne une autre mire, donc un nouveau tirage. Moteur simulé, jamais
//! ArgyllCMS.

mod commun_mires;

use std::time::Duration;

use app::mires::DemandeMire;
use commun_mires::*;
use mires::TailleMire;

/// Une heure après la première préparation.
fn plus_tard() -> std::time::SystemTime {
    maintenant() + Duration::from_secs(3600)
}

#[test]
fn deux_preparations_myiro1_aux_memes_reglages_donnent_un_seul_tirage() {
    let b = banc();
    let m = moteur(232);
    let premiere = b.preparer(&myiro1(TailleMire::Rapide), &m, None).unwrap();
    assert_eq!(premiere.deja_preparee, None);
    let seconde = b
        .ouverte
        .preparer_mire(
            b.condition,
            &myiro1(TailleMire::Rapide),
            &m,
            None,
            &b.mires(),
            plus_tard(),
        )
        .unwrap();
    assert_eq!(seconde.tirage, premiere.tirage);
    assert_eq!(seconde.fichiers, premiere.fichiers);
    assert_eq!((seconde.plages, seconde.pages), (232, 1));
    assert_eq!(seconde.commande, premiere.commande);
    assert_eq!(
        seconde.deja_preparee.as_deref(),
        Some("2026-10-23T09:15:00Z")
    );
    // Rien n'est généré pour un tirage repris.
    assert_eq!(m.plages_demandees(), [232]);
    assert_eq!(b.tirages(), 1);
    assert_eq!(b.fichiers().len(), 1);
}

#[test]
fn un_reglage_change_donne_un_nouveau_tirage() {
    let b = banc();
    let rapide = b
        .preparer(&myiro1(TailleMire::Rapide), &moteur(232), None)
        .unwrap();
    let standard = b
        .preparer(&myiro1(TailleMire::Standard), &moteur(464), None)
        .unwrap();
    let cote_9 = DemandeMire {
        cote_mm: Some(9.0),
        ..myiro1(TailleMire::Rapide)
    };
    let autre_cote = b.preparer(&cote_9, &moteur(232), None).unwrap();
    for preparee in [&standard, &autre_cote] {
        assert_eq!(preparee.deja_preparee, None);
        assert_ne!(preparee.tirage, rapide.tirage);
    }
    assert_ne!(standard.tirage, autre_cote.tirage);
    assert_eq!(b.tirages(), 3);
    // Le réglage d'origine reprend toujours son tirage.
    let encore = b
        .preparer(&myiro1(TailleMire::Rapide), &moteur(232), None)
        .unwrap();
    assert_eq!(encore.tirage, rapide.tirage);
    assert_eq!(b.tirages(), 3);
}

/// Une page effacée à la main est redessinée, à l'identique, sous le même
/// numéro de tirage, à partir des valeurs et de la mise en page rangées ;
/// la page restée en place n'est pas touchée.
#[test]
fn une_page_myiro1_effacee_est_redessinee_sous_le_meme_tirage() {
    let b = banc();
    let m = moteur(928);
    let premiere = b
        .preparer(&myiro1(TailleMire::Approfondi), &m, None)
        .unwrap();
    assert_eq!(premiere.fichiers.len(), 2);
    let page_2 = std::fs::read(&premiere.fichiers[1]).unwrap();
    std::fs::remove_file(&premiere.fichiers[1]).unwrap();
    let seconde = b
        .ouverte
        .preparer_mire(
            b.condition,
            &myiro1(TailleMire::Approfondi),
            &m,
            None,
            &b.mires(),
            plus_tard(),
        )
        .unwrap();
    assert_eq!(seconde.tirage, premiere.tirage);
    assert_eq!(seconde.fichiers, premiere.fichiers);
    assert_eq!(std::fs::read(&seconde.fichiers[1]).unwrap(), page_2);
    assert_eq!(m.plages_demandees(), [928]);
    assert_eq!(b.tirages(), 1);
    assert_eq!(b.fichiers().len(), 2);
}

#[test]
fn deux_resolutions_donnent_deux_tirages_myiro1() {
    let b = banc();
    let epson = b.ouverte.ajouter_imprimante("Epson SC-P5300").unwrap();
    let autre = b
        .ouverte
        .ajouter_papier(epson.id, "Baryta", "1440 dpi")
        .unwrap()
        .id;
    let m = moteur(232);
    let premiere = b.preparer(&myiro1(TailleMire::Rapide), &m, None).unwrap();
    let seconde = b
        .ouverte
        .preparer_mire(
            autre,
            &myiro1(TailleMire::Rapide),
            &m,
            None,
            &b.mires(),
            maintenant(),
        )
        .unwrap();
    assert_ne!(seconde.tirage, premiere.tirage);
    assert_eq!(seconde.deja_preparee, None);
    assert_eq!(m.plages_demandees(), [232, 232]);
    assert_eq!(b.tirages(), 1);
    assert_eq!(b.ouverte.detail_reglage(autre).unwrap().tirages.len(), 1);
}
