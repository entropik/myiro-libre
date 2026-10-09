//! Bibliothèque locale (ADR 0001), vue par son API publique, sur une
//! bibliothèque temporaire. Toutes les mesures sont fictives (n° 12345678).

use bibliotheque::{
    Bibliotheque, Branche, ErreurBibliotheque, IdCondition, IdMesure, Instrument, ReferenceCouleur,
};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Empreinte,
    Geometrie, Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Plage,
    Provenance, Spectre,
};
use tempfile::TempDir;

/// Plage fictive : chaque valeur est distincte, pour voir la moindre perte.
fn plage(graine: f32) -> Plage {
    let spectre = |decalage: f32| {
        Spectre::new(
            (0..36)
                .map(|i| graine + decalage + i as f32 * 0.013_7)
                .collect(),
        )
        .unwrap()
    };
    Plage::new(
        [spectre(0.0), spectre(0.001), spectre(-0.002)],
        DonneesBrutes::new((0..152).map(|i| graine * 1000.0 + i as f32 * 3.5).collect()).unwrap(),
        [
            Lab::new([52.31, 1.07, -2.389]).unwrap(),
            Lab::new([52.4, 1.9, -6.01]).unwrap(),
            Lab::new([52.0, 1.1, -1.8]).unwrap(),
        ],
    )
    .unwrap()
}

fn provenance(numero_serie: u32, horodatage: &str, geometrie: Geometrie) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1D".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Confirmee(Empreinte::new("ab".repeat(32)).unwrap()),
        version_pont: "0.2.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new(horodatage).unwrap(),
        // Étalonnage et conditions observées : inconnus, ils doivent le rester.
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "spectres : M0/M1/M2 ; Lab : D50, 2°".into(),
            demande: Info::Confirmee(ConditionsCalcul {
                conditions_spectres: [
                    Info::Confirmee(ConditionMesure::M0),
                    Info::Supposee(ConditionMesure::M1),
                    Info::Inconnue,
                ],
                longueurs_onde: Info::Confirmee(Echantillonnage {
                    debut_nm: 380,
                    pas_nm: 10,
                }),
                illuminant_lab: Info::Confirmee(Illuminant::D50),
                observateur_lab: Info::Supposee(Observateur::DeuxDegres),
            }),
            observe: Info::Inconnue,
        },
    }
}

fn ponctuelle(numero_serie: u32, horodatage: &str) -> Mesure {
    Mesure::new(
        vec![plage(0.21)],
        provenance(numero_serie, horodatage, Geometrie::Ponctuelle {}),
    )
    .unwrap()
}

fn bande(horodatage: &str) -> Mesure {
    Mesure::new(
        vec![plage(0.1), plage(0.4), plage(0.85)],
        provenance(12345678, horodatage, Geometrie::Bande { sens: 2 }),
    )
    .unwrap()
}

fn bibliotheque_vide() -> (TempDir, Bibliotheque) {
    let dossier = tempfile::tempdir().unwrap();
    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    (dossier, biblio)
}

#[test]
fn on_cree_puis_liste_des_conditions_d_impression() {
    let (_dossier, biblio) = bibliotheque_vide();
    assert!(biblio.conditions().unwrap().is_empty());

    let jet = biblio
        .creer_condition("Jet d’encre, brillant 260 g")
        .unwrap();
    let offset = biblio.creer_condition("Offset, couché mat 150 g").unwrap();

    let noms: Vec<_> = biblio
        .conditions()
        .unwrap()
        .into_iter()
        .map(|c| (c.id, c.nom))
        .collect();
    assert_eq!(
        noms,
        vec![
            (jet.id, "Jet d’encre, brillant 260 g".to_string()),
            (offset.id, "Offset, couché mat 150 g".to_string()),
        ]
    );
}

#[test]
fn un_nom_vide_ou_deja_pris_est_refuse() {
    let (_dossier, biblio) = bibliotheque_vide();
    assert_eq!(
        biblio.creer_condition("   ").unwrap_err(),
        ErreurBibliotheque::NomVide
    );
    biblio.creer_condition("Offset").unwrap();
    assert_eq!(
        biblio.creer_condition(" Offset ").unwrap_err(),
        ErreurBibliotheque::NomDejaPris("Offset".into())
    );
    assert_eq!(biblio.conditions().unwrap().len(), 1);
}

#[test]
fn deux_noms_qui_ne_different_que_par_les_majuscules_ne_coexistent_pas() {
    let (_dossier, biblio) = bibliotheque_vide();
    let ecran = biblio.creer_condition("Écran, adhésif").unwrap();
    // Le refus cite le nom déjà en place.
    assert_eq!(
        biblio.creer_condition("écran, ADHÉSIF").unwrap_err(),
        ErreurBibliotheque::NomDejaPris("Écran, adhésif".into())
    );
    // Renommer une autre condition vers ce nom, en d'autres majuscules : refusé.
    let offset = biblio.creer_condition("Offset").unwrap();
    assert_eq!(
        biblio.renommer_condition(offset.id, "ÉCRAN, ADHÉSIF"),
        Err(ErreurBibliotheque::NomDejaPris("Écran, adhésif".into()))
    );
    // Changer les majuscules de son propre nom reste permis.
    biblio
        .renommer_condition(ecran.id, "écran, adhésif")
        .unwrap();
    assert_eq!(biblio.conditions().unwrap()[0].nom, "écran, adhésif");
}

#[test]
fn une_creation_interrompue_laisse_une_bibliotheque_qui_s_ouvre() {
    let dossier = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dossier.path()).unwrap();
    // Arrêt après la création des tables, avant le numéro d'organisation : la
    // base a ses tables mais reste au numéro 0.
    let base = rusqlite::Connection::open(dossier.path().join(bibliotheque::FICHIER_BASE)).unwrap();
    base.execute_batch(
        "CREATE TABLE conditions (id INTEGER PRIMARY KEY, nom TEXT NOT NULL UNIQUE);",
    )
    .unwrap();
    drop(base);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let offset = biblio.creer_condition("Offset").unwrap();
    biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(12345678, "2026-10-07T15:04:05+02:00"),
        )
        .unwrap();
    drop(biblio);
    assert_eq!(
        Bibliotheque::ouvrir(dossier.path())
            .unwrap()
            .arborescence("")
            .unwrap()[0]
            .mesures
            .len(),
        1
    );
}

#[test]
fn une_creation_annulee_en_cours_laisse_une_bibliotheque_qui_s_ouvre() {
    let dossier = tempfile::tempdir().unwrap();
    let mut base =
        rusqlite::Connection::open(dossier.path().join(bibliotheque::FICHIER_BASE)).unwrap();
    let transaction = base.transaction().unwrap();
    transaction
        .execute_batch("CREATE TABLE conditions (id INTEGER PRIMARY KEY, nom TEXT);")
        .unwrap();
    drop(transaction); // arrêt avant la fin : SQLite annule tout
    drop(base);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    biblio.creer_condition("Offset").unwrap();
}

#[test]
fn une_bibliotheque_d_une_version_plus_recente_est_refusee_sans_etre_touchee() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let base = rusqlite::Connection::open(&chemin).unwrap();
    base.pragma_update(None, "user_version", 99).unwrap();
    drop(base);

    assert_eq!(
        Bibliotheque::ouvrir(dossier.path()).err(),
        Some(ErreurBibliotheque::VersionBase(99))
    );
    let base = rusqlite::Connection::open(&chemin).unwrap();
    let tables: i64 = base
        .query_row("SELECT count(*) FROM sqlite_master", [], |l| l.get(0))
        .unwrap();
    assert_eq!(tables, 0);
}

#[test]
fn les_mesures_se_trient_sur_l_instant_reel_quel_que_soit_le_fuseau() {
    let (_dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    // 15:00 à Paris (13:00 UTC) est plus ancien que 14:30 UTC.
    biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(12345678, "2026-10-07T15:00:00+02:00"),
        )
        .unwrap();
    biblio
        .enregistrer_mesure(offset.id, &ponctuelle(12345678, "2026-10-07T14:30:00Z"))
        .unwrap();
    biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(12345678, "2026-10-07T13:59:59.5+00:00"),
        )
        .unwrap();

    assert_eq!(
        noms(biblio.arborescence("").unwrap()),
        vec![(
            "Offset".to_string(),
            vec![
                "2026-10-07T14:30:00Z".to_string(),
                "2026-10-07T13:59:59.5+00:00".to_string(),
                "2026-10-07T15:00:00+02:00".to_string(),
            ]
        )]
    );
}

#[test]
fn on_renomme_une_condition_d_impression() {
    let (_dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    let jet = biblio.creer_condition("Jet d’encre").unwrap();

    biblio
        .renommer_condition(offset.id, " Offset, couché mat ")
        .unwrap();
    // Garder son propre nom n'est pas un conflit.
    biblio.renommer_condition(jet.id, "Jet d’encre").unwrap();

    assert_eq!(
        biblio.renommer_condition(jet.id, "Offset, couché mat"),
        Err(ErreurBibliotheque::NomDejaPris("Offset, couché mat".into()))
    );
    assert_eq!(
        biblio.renommer_condition(jet.id, ""),
        Err(ErreurBibliotheque::NomVide)
    );
    let inconnue = IdCondition(999);
    assert_eq!(
        biblio.renommer_condition(inconnue, "Autre"),
        Err(ErreurBibliotheque::ConditionInconnue(inconnue))
    );
    let noms: Vec<_> = biblio
        .conditions()
        .unwrap()
        .into_iter()
        .map(|c| c.nom)
        .collect();
    assert_eq!(noms, ["Offset, couché mat", "Jet d’encre"]);
}

#[test]
fn la_bibliotheque_se_retrouve_a_la_reouverture() {
    let dossier = tempfile::tempdir().unwrap();
    let emplacement = dossier.path().join("myiro-libre");
    Bibliotheque::ouvrir(&emplacement)
        .unwrap()
        .creer_condition("Offset")
        .unwrap();

    let rouverte = Bibliotheque::ouvrir(&emplacement).unwrap();
    let noms: Vec<_> = rouverte
        .conditions()
        .unwrap()
        .into_iter()
        .map(|c| c.nom)
        .collect();
    assert_eq!(noms, ["Offset"]);
}

#[test]
fn une_mesure_se_relit_sans_perte_rattachee_a_sa_condition_et_son_instrument() {
    let (_dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    let originale = bande("2026-10-07T15:04:05.250+02:00");

    let id = biblio.enregistrer_mesure(offset.id, &originale).unwrap();
    let relue = biblio.mesure(id).unwrap();

    assert_eq!(relue.id, id);
    assert_eq!(relue.condition, offset.id);
    assert_eq!(
        relue.instrument,
        Instrument {
            modele: "MYIRO-1".into(),
            numero_serie: 12345678,
        }
    );
    // Spectres M0, M1, M2, données brutes, Lab et provenance : tout, à l'identique.
    assert_eq!(relue.mesure, originale);
    let p = relue.mesure.provenance();
    assert_eq!(p.etalonnage, Info::Inconnue);
    assert_eq!(p.calcul.observe, Info::Inconnue);
    assert_eq!(p.geometrie, Geometrie::Bande { sens: 2 });
}

#[test]
fn une_mesure_survit_a_la_reouverture() {
    let dossier = tempfile::tempdir().unwrap();
    let originale = ponctuelle(12345678, "2026-10-07T15:04:05+02:00");
    let id = {
        let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
        let jet = biblio.creer_condition("Jet d’encre").unwrap();
        biblio.enregistrer_mesure(jet.id, &originale).unwrap()
    };
    let rouverte = Bibliotheque::ouvrir(dossier.path()).unwrap();
    assert_eq!(rouverte.mesure(id).unwrap().mesure, originale);
}

#[test]
fn une_mesure_sans_condition_existante_est_refusee() {
    let (_dossier, biblio) = bibliotheque_vide();
    let inconnue = IdCondition(42);
    assert_eq!(
        biblio.enregistrer_mesure(inconnue, &ponctuelle(12345678, "2026-10-07T15:04:05+02:00")),
        Err(ErreurBibliotheque::ConditionInconnue(inconnue))
    );
    assert_eq!(
        biblio.mesure(IdMesure(1)).unwrap_err(),
        ErreurBibliotheque::MesureInconnue(IdMesure(1))
    );
    assert!(biblio.instruments().unwrap().is_empty());
}

/// Arborescence réduite à des noms : condition → horodatages de ses mesures.
fn noms(branches: Vec<Branche>) -> Vec<(String, Vec<String>)> {
    branches
        .into_iter()
        .map(|b| {
            (
                b.condition.nom,
                b.mesures.into_iter().map(|m| m.horodatage).collect(),
            )
        })
        .collect()
}

/// Deux conditions, trois mesures, deux instruments fictifs, une condition vide.
fn bibliotheque_garnie() -> (TempDir, Bibliotheque) {
    let (dossier, biblio) = bibliotheque_vide();
    let jet = biblio
        .creer_condition("Jet d’encre, brillant 260 g")
        .unwrap();
    let offset = biblio.creer_condition("Offset, couché mat").unwrap();
    biblio.creer_condition("Sérigraphie").unwrap();
    biblio
        .enregistrer_mesure(jet.id, &ponctuelle(12345678, "2026-10-06T09:30:00+02:00"))
        .unwrap();
    biblio
        .enregistrer_mesure(jet.id, &bande("2026-10-07T15:04:05+02:00"))
        .unwrap();
    biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(87654321, "2026-10-07T11:00:00+02:00"),
        )
        .unwrap();
    (dossier, biblio)
}

#[test]
fn l_arborescence_range_les_mesures_sous_leur_condition_les_plus_recentes_d_abord() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let branches = biblio.arborescence("").unwrap();

    let bande = &branches[0].mesures[0];
    assert_eq!(
        bande.instrument,
        Instrument {
            modele: "MYIRO-1".into(),
            numero_serie: 12345678,
        }
    );
    assert_eq!(bande.plages, 3);
    assert_eq!(bande.geometrie, Geometrie::Bande { sens: 2 });
    assert_eq!(
        biblio.mesure(bande.id).unwrap().mesure,
        self::bande("2026-10-07T15:04:05+02:00")
    );

    assert_eq!(
        noms(branches),
        vec![
            (
                "Jet d’encre, brillant 260 g".to_string(),
                vec![
                    "2026-10-07T15:04:05+02:00".to_string(),
                    "2026-10-06T09:30:00+02:00".to_string(),
                ]
            ),
            (
                "Offset, couché mat".to_string(),
                vec!["2026-10-07T11:00:00+02:00".to_string()]
            ),
            ("Sérigraphie".to_string(), vec![]),
        ]
    );
}

#[test]
fn la_recherche_par_nom_garde_toute_la_condition_sans_tenir_compte_des_accents() {
    let (_dossier, biblio) = bibliotheque_garnie();
    assert_eq!(
        noms(biblio.arborescence("COUCHE").unwrap()),
        vec![(
            "Offset, couché mat".to_string(),
            vec!["2026-10-07T11:00:00+02:00".to_string()]
        )]
    );
    assert_eq!(
        noms(biblio.arborescence("serig").unwrap()),
        vec![("Sérigraphie".to_string(), vec![])]
    );
}

#[test]
fn la_recherche_trouve_les_mesures_par_instrument_ou_par_date() {
    let (_dossier, biblio) = bibliotheque_garnie();
    assert_eq!(
        noms(biblio.arborescence("87654321").unwrap()),
        vec![(
            "Offset, couché mat".to_string(),
            vec!["2026-10-07T11:00:00+02:00".to_string()]
        )]
    );
    // Tous les mots doivent se trouver : le nom de la condition et la date.
    assert_eq!(
        noms(biblio.arborescence("jet 2026-10-06").unwrap()),
        vec![(
            "Jet d’encre, brillant 260 g".to_string(),
            vec!["2026-10-06T09:30:00+02:00".to_string()]
        )]
    );
    assert!(biblio.arborescence("introuvable").unwrap().is_empty());
}

#[test]
fn chaque_instrument_est_connu_une_seule_fois() {
    let (_dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    let jet = biblio.creer_condition("Jet d’encre").unwrap();
    biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(12345678, "2026-10-07T15:04:05+02:00"),
        )
        .unwrap();
    biblio
        .enregistrer_mesure(jet.id, &ponctuelle(12345678, "2026-10-07T16:00:00+02:00"))
        .unwrap();
    biblio
        .enregistrer_mesure(jet.id, &ponctuelle(87654321, "2026-10-07T16:05:00+02:00"))
        .unwrap();

    assert_eq!(
        biblio.instruments().unwrap(),
        vec![
            Instrument {
                modele: "MYIRO-1".into(),
                numero_serie: 12345678,
            },
            Instrument {
                modele: "MYIRO-1".into(),
                numero_serie: 87654321,
            },
        ]
    );
}

// ---- Nom d'une mesure (ticket #7) ----

#[test]
fn une_mesure_nommee_garde_son_nom_et_se_renomme() {
    let (dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    let mesure = ponctuelle(12345678, "2026-10-07T15:04:05+02:00");

    let id = biblio
        .enregistrer_mesure_nommee(offset.id, &mesure, "  Couleur 1 ")
        .unwrap();
    assert_eq!(biblio.mesure(id).unwrap().nom.as_deref(), Some("Couleur 1"));
    assert_eq!(
        biblio.arborescence("").unwrap()[0].mesures[0]
            .nom
            .as_deref(),
        Some("Couleur 1")
    );

    biblio.renommer_mesure(id, "Magenta du logo").unwrap();
    drop(biblio);
    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let relue = biblio.mesure(id).unwrap();
    assert_eq!(relue.nom.as_deref(), Some("Magenta du logo"));
    assert_eq!(relue.mesure, mesure, "renommer ne touche pas à la mesure");

    assert_eq!(
        biblio.renommer_mesure(id, "   "),
        Err(ErreurBibliotheque::NomVide)
    );
    assert_eq!(
        biblio.renommer_mesure(IdMesure(999), "Absente"),
        Err(ErreurBibliotheque::MesureInconnue(IdMesure(999)))
    );
    assert_eq!(
        biblio.mesure(id).unwrap().nom.as_deref(),
        Some("Magenta du logo")
    );
}

#[test]
fn une_mesure_au_nom_vide_n_est_pas_enregistree() {
    let (_dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();

    assert_eq!(
        biblio.enregistrer_mesure_nommee(
            offset.id,
            &ponctuelle(12345678, "2026-10-07T15:04:05+02:00"),
            " "
        ),
        Err(ErreurBibliotheque::NomVide)
    );
    assert!(biblio.arborescence("").unwrap()[0].mesures.is_empty());
}

/// Une mesure enregistrée sans nom (bande, import) n'en reçoit pas d'office.
#[test]
fn une_mesure_sans_nom_reste_sans_nom() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let branches = biblio.arborescence("").unwrap();
    assert!(branches
        .iter()
        .flat_map(|b| &b.mesures)
        .all(|m| m.nom.is_none()));
    assert_eq!(biblio.mesure(branches[0].mesures[0].id).unwrap().nom, None);
}

#[test]
fn la_recherche_trouve_une_mesure_par_son_nom() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let offset = biblio.conditions().unwrap()[1].id;
    biblio
        .enregistrer_mesure_nommee(
            offset,
            &ponctuelle(12345678, "2026-10-08T10:00:00+02:00"),
            "Rouge du logo",
        )
        .unwrap();

    assert_eq!(
        noms(biblio.arborescence("rouge LOGO").unwrap()),
        vec![(
            "Offset, couché mat".to_string(),
            vec!["2026-10-08T10:00:00+02:00".to_string()]
        )]
    );
}

/// Organisation 1, telle que la première bibliothèque l'écrivait : figée ici
/// pour vérifier la migration, même si le code évolue.
const ORGANISATION_1: &str = "
CREATE TABLE conditions (id INTEGER PRIMARY KEY, nom TEXT NOT NULL UNIQUE);
CREATE TABLE instruments (
    id INTEGER PRIMARY KEY, modele TEXT NOT NULL, numero_serie INTEGER NOT NULL,
    UNIQUE (modele, numero_serie)
);
CREATE TABLE mesures (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    instrument INTEGER NOT NULL REFERENCES instruments (id),
    horodatage TEXT NOT NULL, geometrie TEXT NOT NULL, plages INTEGER NOT NULL,
    contenu TEXT NOT NULL
);
CREATE INDEX mesures_par_condition ON mesures (condition);
PRAGMA user_version = 1;
";

/// Une bibliothèque écrite avant les noms de mesure se relit sans perte :
/// ses mesures restent sans nom et peuvent ensuite en recevoir un.
#[test]
fn une_bibliotheque_d_avant_les_noms_se_relit_sans_perte() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let mesure = ponctuelle(12345678, "2026-10-07T15:04:05+02:00");
    let base = rusqlite::Connection::open(&chemin).unwrap();
    base.execute_batch(ORGANISATION_1).unwrap();
    base.execute_batch(&format!(
        "INSERT INTO conditions (id, nom) VALUES (1, 'Offset');
         INSERT INTO instruments (id, modele, numero_serie) VALUES (1, 'MYIRO-1', 12345678);
         INSERT INTO mesures (id, condition, instrument, horodatage, geometrie, plages, contenu)
         VALUES (1, 1, 1, '2026-10-07T15:04:05+02:00', '{{\"lecture\":\"ponctuelle\"}}', 1, '{}');",
        pont_protocole::ecrire_mesure(&mesure).replace('\'', "''")
    ))
    .unwrap();
    drop(base);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();

    let relue = biblio.mesure(IdMesure(1)).unwrap();
    assert_eq!(relue.mesure, mesure);
    assert_eq!(relue.condition, IdCondition(1));
    assert_eq!(relue.nom, None);
    biblio.renommer_mesure(IdMesure(1), "Couleur 1").unwrap();
    assert_eq!(
        biblio.arborescence("").unwrap()[0].mesures[0]
            .nom
            .as_deref(),
        Some("Couleur 1")
    );
    drop(biblio);
    let version: i32 = rusqlite::Connection::open(&chemin)
        .unwrap()
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 3);
}

// ---- Couleur de référence (ticket #8) ----

/// Une bibliothèque avec une condition et deux mesures ponctuelles.
fn deux_mesures() -> (TempDir, Bibliotheque, IdMesure, IdMesure) {
    let (dossier, biblio) = bibliotheque_vide();
    let offset = biblio.creer_condition("Offset").unwrap();
    let a = biblio
        .enregistrer_mesure_nommee(
            offset.id,
            &ponctuelle(12345678, "2026-10-09T10:00:00+02:00"),
            "Magenta",
        )
        .unwrap();
    let b = biblio
        .enregistrer_mesure(
            offset.id,
            &ponctuelle(12345678, "2026-10-09T10:01:00+02:00"),
        )
        .unwrap();
    (dossier, biblio, a, b)
}

#[test]
fn une_mesure_devient_couleur_de_reference_avec_ou_sans_seuil() {
    let (dossier, biblio, a, b) = deux_mesures();
    assert_eq!(biblio.reference(a).unwrap(), None);

    // Sans seuil : aucune valeur à la place, le seuil est absent.
    biblio.designer_reference(a, None).unwrap();
    assert_eq!(
        biblio.reference(a).unwrap(),
        Some(ReferenceCouleur {
            mesure: a,
            seuil: None
        })
    );
    biblio.designer_reference(a, Some(2.5)).unwrap();
    biblio.designer_reference(b, Some(1.0)).unwrap();
    drop(biblio);

    // Le seuil est conservé avec la référence, à la réouverture.
    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    assert_eq!(
        biblio.references().unwrap(),
        vec![
            ReferenceCouleur {
                mesure: a,
                seuil: Some(2.5)
            },
            ReferenceCouleur {
                mesure: b,
                seuil: Some(1.0)
            },
        ]
    );
    biblio.retirer_reference(a).unwrap();
    assert_eq!(biblio.reference(a).unwrap(), None);
    // La mesure elle-même reste, avec son nom.
    assert_eq!(biblio.mesure(a).unwrap().nom.as_deref(), Some("Magenta"));
    // Retirer une référence absente ne change rien.
    biblio.retirer_reference(a).unwrap();
}

#[test]
fn un_seuil_impossible_ou_une_mesure_inconnue_sont_refuses() {
    let (_dossier, biblio, a, _) = deux_mesures();
    for seuil in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            biblio.designer_reference(a, Some(seuil)),
            Err(ErreurBibliotheque::SeuilInvalide),
            "{seuil}"
        );
    }
    assert_eq!(biblio.reference(a).unwrap(), None, "rien n'a été écrit");
    assert_eq!(
        biblio.designer_reference(IdMesure(99), None),
        Err(ErreurBibliotheque::MesureInconnue(IdMesure(99)))
    );
}

/// Une bibliothèque de l'organisation 2 (avec les noms) se relit sans
/// perte : aucune mesure n'y est couleur de référence.
#[test]
fn une_bibliotheque_d_avant_les_references_se_relit_sans_perte() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let mesure = ponctuelle(12345678, "2026-10-07T15:04:05+02:00");
    let base = rusqlite::Connection::open(&chemin).unwrap();
    base.execute_batch(ORGANISATION_1).unwrap();
    base.execute_batch(&format!(
        "ALTER TABLE mesures ADD COLUMN nom TEXT;
         PRAGMA user_version = 2;
         INSERT INTO conditions (id, nom) VALUES (1, 'Offset');
         INSERT INTO instruments (id, modele, numero_serie) VALUES (1, 'MYIRO-1', 12345678);
         INSERT INTO mesures (id, condition, instrument, horodatage, geometrie, plages, contenu, nom)
         VALUES (1, 1, 1, '2026-10-07T15:04:05+02:00', '{{\"lecture\":\"ponctuelle\"}}', 1, '{}', 'Cyan');",
        pont_protocole::ecrire_mesure(&mesure).replace('\'', "''")
    ))
    .unwrap();
    drop(base);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();

    let relue = biblio.mesure(IdMesure(1)).unwrap();
    assert_eq!(relue.mesure, mesure);
    assert_eq!(relue.nom.as_deref(), Some("Cyan"));
    assert_eq!(biblio.references().unwrap(), vec![]);
    biblio.designer_reference(IdMesure(1), Some(2.0)).unwrap();
    assert_eq!(
        biblio.reference(IdMesure(1)).unwrap(),
        Some(ReferenceCouleur {
            mesure: IdMesure(1),
            seuil: Some(2.0)
        })
    );
}

/// Remplacer une référence par une autre se fait d'un bloc : jamais deux
/// références, et rien ne change si la nouvelle mesure est inconnue.
#[test]
fn une_reference_se_remplace_d_un_bloc() {
    let (_dossier, biblio, a, b) = deux_mesures();
    biblio.designer_reference(a, Some(2.0)).unwrap();

    assert_eq!(
        biblio.remplacer_reference(a, IdMesure(99)),
        Err(ErreurBibliotheque::MesureInconnue(IdMesure(99)))
    );
    assert_eq!(
        biblio.references().unwrap(),
        vec![ReferenceCouleur {
            mesure: a,
            seuil: Some(2.0)
        }]
    );

    biblio.remplacer_reference(a, b).unwrap();
    assert_eq!(
        biblio.references().unwrap(),
        vec![ReferenceCouleur {
            mesure: b,
            seuil: None
        }]
    );
}
