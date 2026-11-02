//! Association des plages d'une mesure à celles du plan : par identifiant
//! et RVB, jamais devinée.

mod commun;

use commun::{lignes, plan, Export};
use seance_mesure::{associer, vue_plages, Doute, EtatPlage};

#[test]
fn un_export_conforme_au_plan_est_associe_avec_certitude() {
    let a = associer(&plan(), &Export::fd9(lignes(0.0)).mesure());
    assert!(a.certaine());
    assert!(a.complete());
    assert_eq!(a.rangs, [Some(0), Some(1), Some(2)]);
}

#[test]
fn l_ordre_du_fichier_n_importe_pas() {
    let mut l = lignes(0.0);
    l.reverse();
    let a = associer(&plan(), &Export::fd9(l).mesure());
    assert!(a.certaine());
    assert_eq!(a.rangs, [Some(2), Some(1), Some(0)]);
}

#[test]
fn un_import_incomplet_garde_ses_plages_et_nomme_les_manquantes() {
    let mut l = lignes(0.0);
    l.remove(2);
    let a = associer(&plan(), &Export::fd9(l).mesure());
    assert!(a.certaine());
    assert!(!a.complete());
    assert_eq!(a.manquantes, ["3"]);
    assert_eq!(a.rangs, [Some(0), Some(1), None]);
}

#[test]
fn des_rvb_differents_de_la_mire_rendent_l_association_incertaine() {
    let mut l = lignes(0.0);
    l[1].rvb[2] += 1.0;
    let a = associer(&plan(), &Export::fd9(l).mesure());
    assert!(!a.certaine());
    assert_eq!(
        a.doutes,
        [Doute::RvbDifferents {
            plages: vec!["2".into()]
        }]
    );
}

#[test]
fn un_demi_pas_d_ecart_sur_255_est_admis() {
    let mut l = lignes(0.0);
    l[1].rvb[2] += 0.4;
    assert!(associer(&plan(), &Export::fd9(l).mesure()).certaine());
}

#[test]
fn sans_rvb_l_association_est_incertaine() {
    let mut e = Export::fd9(lignes(0.0));
    e.rvb = false;
    let a = associer(&plan(), &e.mesure());
    assert!(!a.certaine());
    assert_eq!(
        a.doutes,
        [Doute::RvbInconnus {
            plages: vec!["1".into(), "2".into(), "3".into()]
        }]
    );
}

#[test]
fn une_plage_en_plus_du_plan_rend_l_association_incertaine() {
    let mut l = lignes(0.0);
    let mut en_plus = l[0].clone();
    en_plus.id = "4".into();
    l.push(en_plus);
    let a = associer(&plan(), &Export::fd9(l).mesure());
    assert!(!a.certaine());
    assert!(a.complete());
    assert_eq!(
        a.doutes,
        [Doute::IdentifiantsInconnus {
            plages: vec!["4".into()]
        }]
    );
}

#[test]
fn une_plage_en_double_rend_l_association_incertaine() {
    let mut l = lignes(0.0);
    l.push(l[1].clone());
    let a = associer(&plan(), &Export::fd9(l).mesure());
    assert!(!a.certaine());
    assert_eq!(
        a.doutes,
        [Doute::Doublons {
            plages: vec!["2".into()]
        }]
    );
    // Aucune des deux n'est choisie.
    assert_eq!(a.rangs[1], None);
}

#[test]
fn la_vue_montre_l_attendu_et_le_mesure_de_chaque_plage() {
    let mut l = lignes(0.0);
    l[1].rvb[0] = 10.0;
    l.remove(2);
    let mesure = Export::fd9(l).mesure();
    let a = associer(&plan(), &mesure);
    let vue = vue_plages(&plan(), &mesure, &a);
    assert_eq!(vue.len(), 3);
    assert_eq!(vue[0].identifiant, "1");
    assert_eq!(vue[0].rvb_attendu, [100.0, 100.0, 100.0]);
    assert_eq!(vue[0].etat, EtatPlage::Associee);
    assert_eq!(vue[0].lab[1], Some([90.0, 1.0, -2.0]));
    assert_eq!(vue[1].etat, EtatPlage::Douteuse);
    let rvb = vue[1].rvb_fichier.unwrap();
    assert!((rvb[0] - 10.0 / 2.55).abs() < 1e-4, "{rvb:?}");
    assert_eq!(vue[2].etat, EtatPlage::Manquante);
    assert_eq!(vue[2].rvb_fichier, None);
    assert_eq!(vue[2].lab, [None, None, None]);
}

#[test]
fn l_association_se_relit_telle_quelle() {
    let mut l = lignes(0.0);
    l.remove(0);
    let a = associer(&plan(), &Export::fd9(l).mesure());
    let texte = serde_json::to_string(&a).unwrap();
    assert_eq!(
        serde_json::from_str::<seance_mesure::Association>(&texte).unwrap(),
        a
    );
}
