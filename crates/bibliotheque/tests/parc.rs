//! Parc des imprimantes, papiers et résolutions (organisation 5), vu par
//! l'API publique de la bibliothèque. Données fictives (n° 12345678).

use bibliotheque::Bibliotheque;
use tempfile::TempDir;

mod commun;
use commun::ponctuelle;

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
