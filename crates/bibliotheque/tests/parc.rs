//! Parc des imprimantes, papiers et résolutions (organisation 5), vu par
//! l'API publique de la bibliothèque. Données fictives (n° 12345678).

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, IdImprimante, IdMesure, IdPapier, Parc, ReferenceCouleur,
};
use tempfile::TempDir;

mod commun;
use commun::ponctuelle;

fn maintenant() -> std::time::SystemTime {
    std::time::SystemTime::now()
}

/// Un papier et ses résolutions, par leurs noms.
type NomsPapier = (String, Vec<String>);

/// Noms de l'arbre : imprimante → papier → résolutions.
fn arbre(parc: &Parc) -> Vec<(String, Vec<NomsPapier>)> {
    parc.imprimantes
        .iter()
        .map(|i| {
            let papiers = i
                .papiers
                .iter()
                .map(|p| {
                    let reglages = p.reglages.iter().map(|r| r.resolution.clone()).collect();
                    (p.papier.nom.clone(), reglages)
                })
                .collect();
            (i.imprimante.nom.clone(), papiers)
        })
        .collect()
}

/// Base d'organisation 4 figée, produite une fois par
/// [`produire_fixture_organisation_4`].
const FIXTURE_ORGANISATION_4: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/donnees/organisation-4.sqlite"
);

fn bibliotheque_temporaire() -> (TempDir, Bibliotheque) {
    let dossier = tempfile::tempdir().unwrap();
    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    (dossier, biblio)
}

/// A servi une seule fois, quand la bibliothèque était encore en
/// organisation 4, à produire `tests/donnees/organisation-4.sqlite`. Ne plus
/// le relancer : il écrirait une base de l'organisation d'aujourd'hui.
#[test]
#[ignore]
fn produire_fixture_organisation_4() {
    let (dossier, biblio) = bibliotheque_temporaire();
    let essai = biblio.creer_condition("Essai").unwrap();
    let mesure = biblio
        .enregistrer_mesure(essai.id, &ponctuelle(12345678, "2026-10-07T09:15:00Z"))
        .unwrap();
    biblio.designer_reference(mesure, Some(1.5)).unwrap();
    drop(biblio);
    let cible = std::path::Path::new(FIXTURE_ORGANISATION_4);
    std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
    std::fs::copy(dossier.path().join(bibliotheque::FICHIER_BASE), cible).unwrap();
}

#[test]
fn une_base_neuve_est_en_organisation_5() {
    let (dossier, biblio) = bibliotheque_temporaire();
    drop(biblio);
    let base = rusqlite::Connection::open(dossier.path().join(bibliotheque::FICHIER_BASE)).unwrap();
    let version: i32 = base
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 5);
    base.prepare("SELECT papier, resolution FROM conditions")
        .unwrap();
    base.prepare("SELECT id, nom FROM imprimantes").unwrap();
    base.prepare("SELECT id, imprimante, nom FROM papiers")
        .unwrap();
    base.prepare(
        "SELECT id, condition, nom, horodatage, espace, etat, actif, empreinte, parametres,
                mesures, icc FROM profils",
    )
    .unwrap();
}

#[test]
fn l_arbre_range_imprimante_papier_resolution() {
    let (_d, b) = bibliotheque_temporaire();
    let canon = b.creer_imprimante(" Canon PRO-4100 ").unwrap();
    assert_eq!(canon.nom, "Canon PRO-4100");
    let luster = b.creer_papier(canon.id, "HMPhoto Luster 290").unwrap();
    assert_eq!(luster.imprimante, canon.id);
    b.creer_reglage(luster.id, "1200 ppp").unwrap();
    let c600 = b.creer_reglage(luster.id, "600 ppp").unwrap();
    assert_eq!(c600.nom, "Canon PRO-4100 · HMPhoto Luster 290 · 600 ppp");
    let parc = b.parc("", maintenant()).unwrap();
    assert_eq!(parc.imprimantes.len(), 1);
    let reglages: Vec<_> = parc.imprimantes[0].papiers[0]
        .reglages
        .iter()
        .map(|r| r.resolution.as_str())
        .collect();
    assert_eq!(reglages, ["1200 ppp", "600 ppp"]);
    let reglage = &parc.imprimantes[0].papiers[0].reglages[1];
    assert_eq!(reglage.condition, c600);
    assert_eq!(reglage.profil, None);
    assert!(!reglage.profil_ancien);
    assert!(parc.non_classees.is_empty());
    assert_eq!(b.imprimantes().unwrap(), vec![canon]);
}

#[test]
fn chaque_niveau_se_trie_par_ordre_alphabetique() {
    let (_d, b) = bibliotheque_temporaire();
    let epson = b.creer_imprimante("epson SC-P5300").unwrap();
    let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
    b.creer_papier(epson.id, "Luster").unwrap();
    let rag = b.creer_papier(canon.id, "Photo Rag").unwrap();
    b.creer_papier(canon.id, "Baryta").unwrap();
    b.creer_reglage(rag.id, "720 ppp").unwrap();
    b.creer_reglage(rag.id, "1440 ppp").unwrap();
    let parc = b.parc("", maintenant()).unwrap();
    let s = |t: &str| t.to_string();
    assert_eq!(
        arbre(&parc),
        vec![
            (
                s("Canon PRO-4100"),
                vec![
                    (s("Baryta"), vec![]),
                    (s("Photo Rag"), vec![s("1440 ppp"), s("720 ppp")])
                ]
            ),
            (s("epson SC-P5300"), vec![(s("Luster"), vec![])]),
        ]
    );
}

#[test]
fn les_doublons_sont_refuses_a_chaque_niveau() {
    let (_d, b) = bibliotheque_temporaire();
    let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
    assert!(matches!(
        b.creer_imprimante(" canon pro-4100 "),
        Err(ErreurBibliotheque::NomDejaPris(_))
    ));
    let epson = b.creer_imprimante("Epson SC-P5300").unwrap();
    let luster = b.creer_papier(canon.id, "Luster").unwrap();
    assert!(matches!(
        b.creer_papier(canon.id, "LUSTER"),
        Err(ErreurBibliotheque::NomDejaPris(_))
    ));
    b.creer_papier(epson.id, "Luster").unwrap();
    b.creer_reglage(luster.id, "600 ppp").unwrap();
    assert!(matches!(
        b.creer_reglage(luster.id, " 600 PPP"),
        Err(ErreurBibliotheque::NomDejaPris(_))
    ));
    assert_eq!(b.creer_imprimante("  "), Err(ErreurBibliotheque::NomVide));
    assert_eq!(
        b.creer_papier(canon.id, ""),
        Err(ErreurBibliotheque::NomVide)
    );
    assert_eq!(
        b.creer_reglage(luster.id, " "),
        Err(ErreurBibliotheque::NomVide)
    );
    // Le message sert à tous les niveaux : il ne parle pas de condition.
    let message = b.creer_papier(canon.id, "").unwrap_err().to_string();
    assert!(!message.contains("condition"), "{message}");
    assert!(message.contains("vide"), "{message}");
    assert_eq!(
        b.creer_papier(IdImprimante(999), "Luster"),
        Err(ErreurBibliotheque::ImprimanteInconnue(IdImprimante(999)))
    );
    assert_eq!(
        b.creer_reglage(IdPapier(999), "600 ppp"),
        Err(ErreurBibliotheque::PapierInconnu(IdPapier(999)))
    );
}

#[test]
fn la_recherche_ignore_accents_et_majuscules() {
    let (_d, b) = bibliotheque_temporaire();
    let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
    let rag = b
        .creer_papier(canon.id, "Hahnemühle Photo Rag 308")
        .unwrap();
    b.creer_papier(canon.id, "Baryta").unwrap();
    b.creer_reglage(rag.id, "600 ppp").unwrap();
    let epson = b.creer_imprimante("Epson SC-P5300").unwrap();
    let luster = b.creer_papier(epson.id, "Luster").unwrap();
    b.creer_reglage(luster.id, "1440 ppp").unwrap();
    let s = |t: &str| t.to_string();
    let epson_seule = vec![(
        s("Epson SC-P5300"),
        vec![(s("Luster"), vec![s("1440 ppp")])],
    )];

    assert_eq!(
        arbre(&b.parc("hahnemuhle", maintenant()).unwrap()),
        vec![(
            s("Canon PRO-4100"),
            vec![(s("Hahnemühle Photo Rag 308"), vec![s("600 ppp")])]
        )]
    );
    assert_eq!(arbre(&b.parc("LUSTER", maintenant()).unwrap()), epson_seule);
    // Une résolution trouvée garde son papier et son imprimante.
    assert_eq!(arbre(&b.parc("1440", maintenant()).unwrap()), epson_seule);
    // Une imprimante trouvée garde tous ses papiers.
    let canon_seule = arbre(&b.parc("canon", maintenant()).unwrap());
    assert_eq!(canon_seule.len(), 1);
    assert_eq!(canon_seule[0].1.len(), 2);
    assert!(b.parc("zzz", maintenant()).unwrap().imprimantes.is_empty());
}

#[test]
fn une_base_d_organisation_4_garde_tout_et_reste_non_classee() {
    let dossier = tempfile::tempdir().unwrap();
    std::fs::copy(
        FIXTURE_ORGANISATION_4,
        dossier.path().join(bibliotheque::FICHIER_BASE),
    )
    .unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();

    let conditions = b.conditions().unwrap();
    let noms: Vec<_> = conditions.iter().map(|c| c.nom.as_str()).collect();
    assert_eq!(noms, ["Essai"]);
    let relue = b.mesure(IdMesure(1)).unwrap();
    assert_eq!(relue.condition, conditions[0].id);
    assert_eq!(relue.mesure, ponctuelle(12345678, "2026-10-07T09:15:00Z"));
    assert_eq!(
        b.reference(IdMesure(1)).unwrap(),
        Some(ReferenceCouleur {
            mesure: IdMesure(1),
            seuil: Some(1.5)
        })
    );
    let parc = b.parc("", maintenant()).unwrap();
    assert_eq!(parc.non_classees, conditions);
    assert!(parc.imprimantes.is_empty());
    // Une recherche qui la nomme la garde ; une autre l'écarte.
    assert_eq!(
        b.parc("essai", maintenant()).unwrap().non_classees,
        conditions
    );
    assert!(b.parc("zzz", maintenant()).unwrap().non_classees.is_empty());
}
