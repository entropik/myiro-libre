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
    assert_eq!(version, 4);
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

/// Base d'organisation 1 avec une condition et une bande.
fn base_organisation_1(chemin: &std::path::Path) -> Mesure {
    let mesure = bande("2026-10-07T15:04:05+02:00");
    let base = rusqlite::Connection::open(chemin).unwrap();
    base.execute_batch(ORGANISATION_1).unwrap();
    base.execute_batch(
        "INSERT INTO conditions (id, nom) VALUES (7, 'Offset');
         INSERT INTO instruments (id, modele, numero_serie) VALUES (3, 'MYIRO-1', 12345678);",
    )
    .unwrap();
    base.execute(
        "INSERT INTO mesures VALUES (5, 7, 3, '2026-10-07T15:04:05+02:00', ?1, 3, ?2)",
        rusqlite::params![
            serde_json::to_string(&Geometrie::Bande { sens: 2 }).unwrap(),
            pont_protocole::ecrire_mesure(&mesure)
        ],
    )
    .unwrap();
    mesure
}

#[test]
fn une_bibliotheque_d_organisation_1_se_relit_sans_perte_apres_mise_a_niveau() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let mesure = base_organisation_1(&chemin);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let relue = biblio.mesure(IdMesure(5)).unwrap();
    assert_eq!(relue.mesure, mesure);
    assert_eq!(relue.condition, IdCondition(7));
    assert_eq!(
        noms(biblio.arborescence("").unwrap()),
        vec![(
            "Offset".to_string(),
            vec!["2026-10-07T15:04:05+02:00".to_string()]
        )]
    );
    // La base mise à niveau accepte les mesures importées.
    biblio
        .importer_mesure(IdCondition(7), "lab.txt", &cgats_lab_seul())
        .unwrap();
    drop(biblio);
    let version: i32 = rusqlite::Connection::open(&chemin)
        .unwrap()
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 4);
}

/// Fichier CGATS fictif d'un autre logiciel : Lab seuls, en M1 par la
/// source lumineuse, pas de date.
fn cgats_lab_seul() -> String {
    "CGATS.17\nINSTRUMENTATION\t\"FD-9\"\nSERIAL\t\"12345678\"\nCREATED\t\"\"\n\
     MEASUREMENT_SOURCE\t\"D50\"\nNUMBER_OF_FIELDS\t4\nBEGIN_DATA_FORMAT\n\
     SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\nEND_DATA_FORMAT\nNUMBER_OF_SETS\t2\nBEGIN_DATA\n\
     1\t50.0\t1.0\t-2.0\n2\t80.5\t-3.0\t4.0\nEND_DATA\n"
        .to_string()
}

#[test]
fn une_mesure_importee_est_rangee_dans_sa_condition_et_marquee_importee() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let offset = biblio.conditions().unwrap()[1].clone();
    let id = biblio
        .importer_mesure(offset.id, r"C:\Exports\client\lab.txt", &cgats_lab_seul())
        .unwrap();

    let relue = biblio.mesure_importee(id).unwrap();
    assert_eq!(relue.condition, offset.id);
    // Le nom du fichier, jamais son chemin.
    assert_eq!(relue.fichier, "lab.txt");
    assert_eq!(relue.mesure, cgats::lire(&cgats_lab_seul()).unwrap());
    assert_eq!(relue.mesure.plages[0].spectres[1], Info::Inconnue);

    let branche = biblio
        .arborescence("")
        .unwrap()
        .into_iter()
        .find(|b| b.condition.id == offset.id)
        .unwrap();
    // Les mesures du pont restent à part : une importée ne s'y mêle pas.
    assert_eq!(branche.mesures.len(), 1);
    assert_eq!(branche.importees.len(), 1);
    let resume = &branche.importees[0];
    assert_eq!(resume.id, id);
    assert_eq!(resume.fichier, "lab.txt");
    assert_eq!(resume.plages, 2);
    assert_eq!(resume.instrument, Info::Confirmee("FD-9".into()));
    assert_eq!(resume.date, Info::Inconnue);
    // Une importée n'est pas une mesure attestée, ni un instrument connu.
    assert_eq!(
        biblio.mesure(id).unwrap_err(),
        ErreurBibliotheque::MesureInconnue(id)
    );
    assert_eq!(biblio.instruments().unwrap().len(), 2);

    // La recherche la trouve par son fichier ou son instrument déclaré.
    let trouvees = |mot: &str| -> usize {
        biblio
            .arborescence(mot)
            .unwrap()
            .iter()
            .map(|b| b.importees.len())
            .sum()
    };
    assert_eq!(trouvees("lab.txt"), 1);
    assert_eq!(trouvees("fd-9"), 1);
    assert_eq!(trouvees("introuvable"), 0);
}

#[test]
fn un_fichier_illisible_ou_une_condition_absente_n_importe_rien() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let jet = biblio.conditions().unwrap()[0].id;
    assert!(matches!(
        biblio.importer_mesure(jet, "x.txt", "rien de CGATS"),
        Err(ErreurBibliotheque::ImportIllisible(_))
    ));
    assert_eq!(
        biblio.importer_mesure(IdCondition(99), "x.txt", &cgats_lab_seul()),
        Err(ErreurBibliotheque::ConditionInconnue(IdCondition(99)))
    );
    assert!(biblio
        .arborescence("")
        .unwrap()
        .iter()
        .all(|b| b.importees.is_empty()));
}

#[test]
fn une_sauvegarde_garde_les_mesures_importees_et_une_ancienne_se_restaure() {
    let (dossier, biblio) = bibliotheque_garnie();
    let jet = biblio.conditions().unwrap()[0].id;
    let id = biblio
        .importer_mesure(jet, "lab.txt", &cgats_lab_seul())
        .unwrap();
    let fichier = dossier.path().join("sauvegarde.sqlite");
    biblio.sauvegarder(&fichier).unwrap();

    let (_ailleurs, mut autre) = bibliotheque_vide();
    autre.restaurer(&fichier).unwrap();
    assert_eq!(
        autre.mesure_importee(id).unwrap(),
        biblio.mesure_importee(id).unwrap()
    );

    // Une sauvegarde d'organisation 1 se restaure et se met à niveau.
    let ancienne = dossier.path().join("ancienne.sqlite");
    let mesure = base_organisation_1(&ancienne);
    autre.restaurer(&ancienne).unwrap();
    assert_eq!(autre.mesure(IdMesure(5)).unwrap().mesure, mesure);
    autre
        .importer_mesure(IdCondition(7), "lab.txt", &cgats_lab_seul())
        .unwrap();
}

/// Organisation 2 (ticket #7), telle qu'elle a mis à niveau les
/// bibliothèques réelles : figée ici, même si le code évolue.
const ORGANISATION_2: &str = "ALTER TABLE mesures ADD COLUMN nom TEXT; PRAGMA user_version = 2;";

/// Base d'organisation 2 : la bande de l'organisation 1, nommée, et une
/// ponctuelle sans nom.
fn base_organisation_2(chemin: &std::path::Path) -> (Mesure, Mesure) {
    let bande = base_organisation_1(chemin);
    let ponctuelle = ponctuelle(12345678, "2026-10-08T10:00:00+02:00");
    let base = rusqlite::Connection::open(chemin).unwrap();
    base.execute_batch(ORGANISATION_2).unwrap();
    base.execute("UPDATE mesures SET nom = 'Bande du matin' WHERE id = 5", [])
        .unwrap();
    base.execute(
        "INSERT INTO mesures (id, condition, instrument, horodatage, geometrie, plages, contenu)
         VALUES (6, 7, 3, '2026-10-08T10:00:00+02:00', '{\"lecture\":\"ponctuelle\"}', 1, ?1)",
        [pont_protocole::ecrire_mesure(&ponctuelle)],
    )
    .unwrap();
    (bande, ponctuelle)
}

/// Les noms et les mesures d'une bibliothèque d'organisation 2 sont relus.
fn verifier_organisation_2(biblio: &Bibliotheque, bande: &Mesure, ponctuelle: &Mesure) {
    let relue = biblio.mesure(IdMesure(5)).unwrap();
    assert_eq!(relue.mesure, *bande);
    assert_eq!(relue.nom.as_deref(), Some("Bande du matin"));
    let relue = biblio.mesure(IdMesure(6)).unwrap();
    assert_eq!(relue.mesure, *ponctuelle);
    assert_eq!(relue.nom, None);
    assert_eq!(biblio.arborescence("matin").unwrap()[0].mesures.len(), 1);
    // Mesure nommée et mesure importée vivent ensemble.
    biblio
        .importer_mesure(IdCondition(7), "lab.txt", &cgats_lab_seul())
        .unwrap();
    biblio.renommer_mesure(IdMesure(6), "Couleur 1").unwrap();
    let branche = &biblio.arborescence("").unwrap()[0];
    assert_eq!(branche.mesures.len(), 2);
    assert_eq!(branche.importees.len(), 1);
}

#[test]
fn une_bibliotheque_d_organisation_2_garde_ses_noms_apres_mise_a_niveau() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let (bande, ponctuelle) = base_organisation_2(&chemin);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    verifier_organisation_2(&biblio, &bande, &ponctuelle);
    drop(biblio);
    let version: i32 = rusqlite::Connection::open(&chemin)
        .unwrap()
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 4);
}

#[test]
fn une_sauvegarde_d_organisation_2_se_restaure_puis_se_met_a_niveau() {
    let (dossier, mut biblio) = bibliotheque_garnie();
    let ancienne = dossier.path().join("organisation-2.sqlite");
    let (bande, ponctuelle) = base_organisation_2(&ancienne);

    biblio.restaurer(&ancienne).unwrap();
    verifier_organisation_2(&biblio, &bande, &ponctuelle);
}

#[test]
fn une_restauration_qui_echoue_en_route_remet_la_bibliotheque_d_avant() {
    let (dossier, mut biblio) = bibliotheque_garnie();
    let attendu = contenu(&biblio);
    // Sauvegarde d'organisation 1 qui passe l'examen, mais dont la mise à
    // niveau échoue : une table du même nom que celle qu'elle doit créer.
    let piegee = dossier.path().join("piegee.sqlite");
    base_organisation_1(&piegee);
    rusqlite::Connection::open(&piegee)
        .unwrap()
        .execute_batch("CREATE TABLE mesures_3 (x)")
        .unwrap();

    assert!(biblio.restaurer(&piegee).is_err());
    assert_eq!(contenu(&biblio), attendu);
    // La copie de secours reste dans le dossier de la bibliothèque.
    assert_eq!(copies_de_secours(dossier.path()).len(), 1);
    biblio.creer_condition("Après l'échec").unwrap();
}

/// Copies de secours laissées par les restaurations dans le dossier.
fn copies_de_secours(dossier: &std::path::Path) -> Vec<String> {
    let mut noms: Vec<String> = std::fs::read_dir(dossier)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("bibliotheque.sqlite.avant-restauration-"))
        .collect();
    noms.sort();
    noms
}

#[test]
fn chaque_restauration_garde_sa_propre_copie_de_secours_datee() {
    let (dossier, mut biblio) = bibliotheque_garnie();
    let fichier = dossier.path().join("sauvegarde.sqlite");
    biblio.sauvegarder(&fichier).unwrap();
    biblio.restaurer(&fichier).unwrap();
    biblio.restaurer(&fichier).unwrap();

    let copies = copies_de_secours(dossier.path());
    assert_eq!(copies.len(), 2, "{copies:?}");
    // bibliotheque.sqlite.avant-restauration-AAAA-MM-JJ-HHMMSS…
    let date = &copies[0]["bibliotheque.sqlite.avant-restauration-".len()..];
    let o = date.as_bytes();
    assert!(
        date.len() >= 17
            && o[4] == b'-'
            && o[7] == b'-'
            && o[10] == b'-'
            && o[..17]
                .iter()
                .enumerate()
                .all(|(i, c)| [4, 7, 10].contains(&i) || c.is_ascii_digit()),
        "{date}"
    );
}

#[test]
fn une_mesure_importee_ne_se_renomme_pas() {
    let (_dossier, biblio) = bibliotheque_garnie();
    let jet = biblio.conditions().unwrap()[0].id;
    let id = biblio
        .importer_mesure(jet, "lab.txt", &cgats_lab_seul())
        .unwrap();
    assert_eq!(
        biblio.renommer_mesure(id, "Autre nom"),
        Err(ErreurBibliotheque::MesureInconnue(id))
    );
    assert_eq!(biblio.mesure_importee(id).unwrap().fichier, "lab.txt");
}

/// Tout ce que la bibliothèque montre : arborescence, instruments, mesures.
fn contenu(biblio: &Bibliotheque) -> (Vec<Branche>, Vec<Instrument>, Vec<Mesure>) {
    let branches = biblio.arborescence("").unwrap();
    let mesures = branches
        .iter()
        .flat_map(|b| &b.mesures)
        .map(|m| biblio.mesure(m.id).unwrap().mesure)
        .collect();
    (branches, biblio.instruments().unwrap(), mesures)
}

#[test]
fn une_sauvegarde_restauree_rend_toute_la_bibliotheque_a_l_identique() {
    let (dossier, biblio) = bibliotheque_garnie();
    let attendu = contenu(&biblio);
    let fichier = dossier.path().join("sauvegarde.sqlite");
    biblio.sauvegarder(&fichier).unwrap();
    // Une deuxième sauvegarde au même endroit remplace la première.
    biblio.sauvegarder(&fichier).unwrap();

    // Autre poste : une bibliothèque qui a déjà son propre contenu.
    let (_ailleurs, mut autre) = bibliotheque_vide();
    let locale = autre.creer_condition("Condition locale").unwrap();
    autre
        .enregistrer_mesure(
            locale.id,
            &ponctuelle(12345678, "2026-10-08T08:00:00+02:00"),
        )
        .unwrap();

    autre.restaurer(&fichier).unwrap();
    assert_eq!(contenu(&autre), attendu);
    // La bibliothèque restaurée continue de servir.
    autre.creer_condition("Après restauration").unwrap();
}

#[test]
fn un_fichier_qui_n_est_pas_une_sauvegarde_est_refuse_sans_rien_toucher() {
    let (dossier, mut biblio) = bibliotheque_garnie();
    let attendu = contenu(&biblio);

    let texte = dossier.path().join("texte.sqlite");
    std::fs::write(&texte, "pas une base").unwrap();
    let autre_base = dossier.path().join("autre.sqlite");
    rusqlite::Connection::open(&autre_base)
        .unwrap()
        .execute_batch("CREATE TABLE autre (x)")
        .unwrap();
    let recente = dossier.path().join("recente.sqlite");
    biblio.sauvegarder(&recente).unwrap();
    rusqlite::Connection::open(&recente)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();
    let absent = dossier.path().join("absent.sqlite");

    for fichier in [&texte, &autre_base, &recente, &absent] {
        let refus = biblio.restaurer(fichier).unwrap_err();
        assert!(
            matches!(refus, ErreurBibliotheque::SauvegardeInvalide(_)),
            "{} : {refus:?}",
            fichier.display()
        );
        assert_eq!(contenu(&biblio), attendu);
    }
    assert!(!absent.exists(), "la restauration ne crée pas de fichier");
}

#[test]
fn une_sauvegarde_dont_une_mesure_ne_se_relit_plus_est_refusee() {
    let (dossier, mut biblio) = bibliotheque_garnie();
    let attendu = contenu(&biblio);
    let abimee = dossier.path().join("abimee.sqlite");
    biblio.sauvegarder(&abimee).unwrap();
    rusqlite::Connection::open(&abimee)
        .unwrap()
        .execute("UPDATE mesures SET contenu = '{}' WHERE id = 1", [])
        .unwrap();

    assert!(matches!(
        biblio.restaurer(&abimee),
        Err(ErreurBibliotheque::SauvegardeInvalide(_))
    ));
    assert_eq!(contenu(&biblio), attendu);
}

// ---- Organisation 4 : couleurs de référence par-dessus les mesures importées ----

/// Organisation 3 (ticket #9), telle qu'elle met à niveau les bibliothèques
/// réelles : figée ici, même si le code évolue.
const ORGANISATION_3: &str = "
CREATE TABLE mesures_3 (
    id INTEGER PRIMARY KEY,
    condition INTEGER NOT NULL REFERENCES conditions (id),
    origine TEXT NOT NULL DEFAULT 'pont' CHECK (origine IN ('pont', 'importee')),
    instrument INTEGER REFERENCES instruments (id),
    horodatage TEXT,
    geometrie TEXT,
    plages INTEGER NOT NULL,
    fichier TEXT,
    contenu TEXT NOT NULL,
    nom TEXT,
    CHECK (origine = 'importee' OR (instrument IS NOT NULL AND horodatage IS NOT NULL
                                    AND geometrie IS NOT NULL)),
    CHECK (origine = 'pont' OR fichier IS NOT NULL)
);
INSERT INTO mesures_3 (id, condition, origine, instrument, horodatage, geometrie, plages, contenu, nom)
    SELECT id, condition, 'pont', instrument, horodatage, geometrie, plages, contenu, nom FROM mesures;
DROP TABLE mesures;
ALTER TABLE mesures_3 RENAME TO mesures;
CREATE INDEX mesures_par_condition ON mesures (condition);
PRAGMA user_version = 3;
";

/// Base d'organisation 3 : celle de l'organisation 2, plus une mesure
/// importée (n° 7).
fn base_organisation_3(chemin: &std::path::Path) -> (Mesure, Mesure) {
    let mesures = base_organisation_2(chemin);
    let base = rusqlite::Connection::open(chemin).unwrap();
    base.execute_batch(ORGANISATION_3).unwrap();
    base.execute(
        "INSERT INTO mesures (id, condition, origine, plages, fichier, contenu)
         VALUES (7, 7, 'importee', 1, 'lab.txt', ?1)",
        [cgats_lab_seul()],
    )
    .unwrap();
    mesures
}

fn organisation(chemin: &std::path::Path) -> i32 {
    rusqlite::Connection::open(chemin)
        .unwrap()
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap()
}

/// La table des références pointe vers la table des mesures refaite par
/// l'organisation 3 : aucun lien rompu, et une référence vers une mesure
/// absente est refusée par la base elle-même.
fn verifier_cle_etrangere(chemin: &std::path::Path) {
    let base = rusqlite::Connection::open(chemin).unwrap();
    base.pragma_update(None, "foreign_keys", true).unwrap();
    let rompus = base
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap();
    assert!(!rompus, "liens rompus");
    let cible: String = base
        .query_row(
            "SELECT \"table\" FROM pragma_foreign_key_list('references_couleur')",
            [],
            |l| l.get(0),
        )
        .unwrap();
    assert_eq!(cible, "mesures");
    assert!(base
        .execute(
            "INSERT INTO references_couleur (mesure, seuil) VALUES (999, NULL)",
            []
        )
        .is_err());
}

#[test]
fn une_bibliotheque_d_organisation_3_passe_a_l_organisation_4_sans_perte() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let (bande, ponctuelle) = base_organisation_3(&chemin);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();

    assert_eq!(biblio.mesure(IdMesure(5)).unwrap().mesure, bande);
    assert_eq!(biblio.mesure(IdMesure(6)).unwrap().mesure, ponctuelle);
    assert_eq!(
        biblio.mesure_importee(IdMesure(7)).unwrap().fichier,
        "lab.txt"
    );
    assert_eq!(biblio.references().unwrap(), vec![]);
    biblio.designer_reference(IdMesure(6), Some(2.0)).unwrap();
    // Une mesure importée n'est pas une mesure d'un pont : pas de référence.
    assert_eq!(
        biblio.designer_reference(IdMesure(7), None),
        Err(ErreurBibliotheque::MesureInconnue(IdMesure(7)))
    );
    drop(biblio);
    assert_eq!(organisation(&chemin), 4);
    verifier_cle_etrangere(&chemin);
}

#[test]
fn une_bibliotheque_d_organisation_2_passe_a_l_organisation_4_pas_a_pas() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let (bande, _) = base_organisation_2(&chemin);

    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();

    let relue = biblio.mesure(IdMesure(5)).unwrap();
    assert_eq!(
        (relue.mesure, relue.nom.as_deref()),
        (bande, Some("Bande du matin"))
    );
    biblio.designer_reference(IdMesure(5), None).unwrap();
    drop(biblio);
    assert_eq!(organisation(&chemin), 4);
    verifier_cle_etrangere(&chemin);
}

/// Une bibliothèque passée à l'organisation 4 porte ce numéro : une version
/// d'avant les références, qui attend l'organisation 3 au plus, la refuse
/// sans la toucher au lieu de perdre les références.
#[test]
fn une_bibliotheque_d_organisation_4_est_refusee_par_l_ancienne_regle() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    drop(Bibliotheque::ouvrir(dossier.path()).unwrap());

    let organisation = organisation(&chemin);
    let ancienne_regle = |v: i32| v <= 3;
    assert!(!ancienne_regle(organisation), "organisation {organisation}");
    // La règle d'aujourd'hui refuse de même une organisation plus récente.
    rusqlite::Connection::open(&chemin)
        .unwrap()
        .pragma_update(None, "user_version", 5)
        .unwrap();
    let refus = Bibliotheque::ouvrir(dossier.path()).err().unwrap();
    assert_eq!(refus, ErreurBibliotheque::VersionBase(5));
    assert!(refus.to_string().contains("attendu 4 au plus"), "{refus}");
}
