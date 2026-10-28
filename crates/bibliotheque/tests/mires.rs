//! Mires et tirages rangés avec leur résolution (organisation 6), vus par
//! l'API publique de la bibliothèque. Valeurs et mise en page sont du JSON
//! opaque pour la bibliothèque.

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, IdCondition, IdMire, IdTirage, NouveauProfil, NouvelleMire,
};
use tempfile::TempDir;

/// Une bibliothèque avec une condition rangée sous Canon PRO-4100 →
/// Baryta → 600 ppp.
fn condition_rangee() -> (TempDir, Bibliotheque, IdCondition) {
    let dossier = tempfile::tempdir().unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
    let baryta = b.creer_papier(canon.id, "Baryta").unwrap();
    let condition = b.creer_reglage(baryta.id, "600 ppp").unwrap();
    (dossier, b, condition.id)
}

fn mire<'a>(empreinte: &'a str, horodatage: &'a str) -> NouvelleMire<'a> {
    NouvelleMire {
        nom: "Mire rapide",
        instrument: "myiro1",
        plages: 232,
        generation: r#"{"commande":"targen -v -d2 -G -f232 mire","version":"3.5.0"}"#,
        valeurs: "[[100.0,100.0,100.0],[0.0,0.0,0.0]]",
        mise_en_page: r#"{"graine":7}"#,
        empreinte,
        horodatage,
    }
}

#[test]
fn une_mire_et_son_tirage_se_rangent_et_se_relisent() {
    let (_d, b, condition) = condition_rangee();
    let nouvelle = mire("aa11", "2026-10-28T09:00:00Z");
    let (id_mire, id_tirage) = b
        .enregistrer_mire_et_tirage(condition, &nouvelle, 1)
        .unwrap();
    let tirages = b.tirages(condition).unwrap();
    assert_eq!(tirages.len(), 1);
    let t = &tirages[0];
    assert_eq!(
        (t.id, t.mire, t.nom.as_str(), t.instrument.as_str()),
        (id_tirage, id_mire, "Mire rapide", "myiro1")
    );
    assert_eq!((t.plages, t.pages), (232, 1));
    assert_eq!(t.horodatage, "2026-10-28T09:00:00Z");

    let (c, rangee) = b.mire_du_tirage(id_tirage).unwrap();
    assert_eq!(c, condition);
    assert_eq!(rangee.id, id_mire);
    assert_eq!(rangee.generation, nouvelle.generation);
    assert_eq!(rangee.valeurs, nouvelle.valeurs);
    assert_eq!(rangee.mise_en_page, nouvelle.mise_en_page);
    assert_eq!(rangee.empreinte, "aa11");
    assert_eq!(rangee.plages, 232);
}

#[test]
fn deux_tirages_de_la_meme_mire_partagent_une_seule_mire() {
    let (_d, b, condition) = condition_rangee();
    let fd9 = |horodatage| NouvelleMire {
        nom: "RGB-2520",
        instrument: "fd9",
        plages: 2520,
        ..mire("7550f4a3", horodatage)
    };
    let (m1, t1) = b
        .enregistrer_mire_et_tirage(condition, &fd9("2026-10-28T09:00:00Z"), 1)
        .unwrap();
    let (m2, t2) = b
        .enregistrer_mire_et_tirage(condition, &fd9("2026-10-28T10:00:00Z"), 1)
        .unwrap();
    assert_eq!(m1, m2);
    assert_ne!(t1, t2);
    assert_eq!(b.tirages(condition).unwrap().len(), 2);
}

#[test]
fn les_tirages_d_une_resolution_vont_du_plus_recent_au_plus_ancien() {
    let (_d, b, condition) = condition_rangee();
    let (_, ancien) = b
        .enregistrer_mire_et_tirage(condition, &mire("a", "2026-10-27T09:00:00Z"), 1)
        .unwrap();
    let (_, recent) = b
        .enregistrer_mire_et_tirage(condition, &mire("b", "2026-10-28T09:00:00+02:00"), 2)
        .unwrap();
    let (_, milieu) = b
        .enregistrer_mire_et_tirage(condition, &mire("c", "2026-10-27T20:00:00Z"), 4)
        .unwrap();
    let ordre: Vec<IdTirage> = b.tirages(condition).unwrap().iter().map(|t| t.id).collect();
    assert_eq!(ordre, [recent, milieu, ancien]);
}

#[test]
fn un_tirage_sur_une_condition_inconnue_est_refuse_sans_ranger_la_mire() {
    let (_d, b, condition) = condition_rangee();
    let inconnue = IdCondition(999);
    assert_eq!(
        b.enregistrer_mire_et_tirage(inconnue, &mire("aa", "2026-10-28T09:00:00Z"), 1),
        Err(ErreurBibliotheque::ConditionInconnue(inconnue))
    );
    // La mire n'a pas été rangée : la même empreinte donne la mire n° 1.
    let (id, _) = b
        .enregistrer_mire_et_tirage(condition, &mire("aa", "2026-10-28T09:00:00Z"), 1)
        .unwrap();
    assert_eq!(id, IdMire(1));
}

#[test]
fn un_instrument_inconnu_est_refuse() {
    let (_d, b, condition) = condition_rangee();
    let autre = NouvelleMire {
        instrument: "i1pro",
        ..mire("aa", "2026-10-28T09:00:00Z")
    };
    assert!(matches!(
        b.enregistrer_mire_et_tirage(condition, &autre, 1),
        Err(ErreurBibliotheque::MireInvalide(_))
    ));
    assert!(matches!(
        b.enregistrer_mire_et_tirage(condition, &mire("aa", "2026-10-28T09:00:00Z"), 0),
        Err(ErreurBibliotheque::MireInvalide(_))
    ));
    assert!(matches!(
        b.enregistrer_mire_et_tirage(condition, &mire("aa", "hier"), 1),
        Err(ErreurBibliotheque::HorodatageInvalide(_))
    ));
    assert!(b.tirages(condition).unwrap().is_empty());
}

#[test]
fn un_tirage_inconnu_est_dit_inconnu() {
    let (_d, b, _) = condition_rangee();
    assert_eq!(
        b.mire_du_tirage(IdTirage(5)),
        Err(ErreurBibliotheque::TirageInconnu(IdTirage(5)))
    );
}

#[test]
fn une_base_en_organisation_5_passe_en_6_sans_rien_perdre() {
    let (dossier, b, condition) = condition_rangee();
    let mut icc = vec![0u8; 132];
    icc[0..4].copy_from_slice(&132u32.to_be_bytes());
    icc[16..20].copy_from_slice(b"RGB ");
    icc[36..40].copy_from_slice(b"acsp");
    let profil = b
        .enregistrer_profil(&NouveauProfil {
            condition,
            nom: "Profil",
            icc: &icc,
            horodatage: "2026-10-20T09:00:00Z",
            parametres: "{}",
            mesures: &[],
        })
        .unwrap();
    let parc_avant = b.parc("", std::time::SystemTime::UNIX_EPOCH).unwrap();
    drop(b);

    // Retour à l'organisation 5 : sans les tables des mires.
    let chemin = std::fs::read_dir(dossier.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "sqlite"))
        .unwrap();
    let base = rusqlite::Connection::open(&chemin).unwrap();
    base.execute_batch("DROP TABLE tirages; DROP TABLE mires; PRAGMA user_version = 5;")
        .unwrap();
    drop(base);

    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    assert_eq!(
        b.parc("", std::time::SystemTime::UNIX_EPOCH).unwrap(),
        parc_avant
    );
    assert_eq!(b.fichier_profil(profil).unwrap(), icc);
    assert!(b.tirages(condition).unwrap().is_empty());
    b.enregistrer_mire_et_tirage(condition, &mire("aa", "2026-10-28T09:00:00Z"), 1)
        .unwrap();
    let base = rusqlite::Connection::open(&chemin).unwrap();
    let version: i32 = base
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 6);
}
