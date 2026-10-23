//! Calcul d'un profil RVB depuis une mesure importée, vu par l'application :
//! méthodes de `BibliothequeOuverte` (sans Tauri) et moteur simulé, jamais
//! ArgyllCMS. Exports FD-S2w fictifs ; les erreurs sont des clés du
//! catalogue de textes.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::profiler::Moteur;
use bibliotheque::{nom_profil_propose, EtatVerification, IdCondition, IdMesure};
use pont_protocole::ConditionMesure;
use profilage::{ErreurMoteur, Etape, MoteurSimule, Recette};
use tempfile::TempDir;

/// En-tête ICC RVB minimal de 132 octets : taille, `RGB `, `acsp`.
fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

/// Le 23 octobre 2026 à 9 h 15, UTC.
fn maintenant() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_746_900)
}

/// Export fictif à la manière de FD-S2w : `source` vaut `D50` (M1) ou `A`
/// (M0), `champs` et `lignes` donnent les plages.
fn export(source: &str, champs: &str, lignes: &[&str]) -> String {
    let donnees: String = lignes.iter().map(|l| format!("{l}\r\n")).collect();
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n\
         MEASUREMENT_SOURCE\t\"{source}\"\r\nBEGIN_DATA_FORMAT\r\n{champs}\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n{donnees}END_DATA\r\n",
        lignes.len()
    )
}

/// Trois plages, RVB sur 255, Lab.
fn export_rvb(source: &str) -> String {
    export(
        source,
        "SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B",
        &[
            "1\t255\t255\t255\t95\t0\t2",
            "2\t0\t117.61\t199.05\t42.09\t-28\t-60.57",
            "3\t0\t0\t0\t3.1\t0.2\t-0.4",
        ],
    )
}

struct Banc {
    _dossier: TempDir,
    ouverte: BibliothequeOuverte,
    condition: IdCondition,
}

/// Canon PRO-4100 → Hahnemühle Photo Rag 308 → 720 dpi.
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        _dossier: dossier,
        ouverte,
        condition,
    }
}

impl Banc {
    /// Importe un export dans une résolution, par l'aperçu, comme la page.
    fn importer(&self, condition: IdCondition, nom: &str, texte: &str) -> IdMesure {
        let chemin = self._dossier.path().join(nom);
        std::fs::write(&chemin, texte).unwrap();
        let apercu = self.ouverte.apercu_cgats(&chemin).unwrap();
        self.ouverte
            .ranger_apercu(apercu.numero, condition)
            .unwrap()
    }

    fn ranger_un_profil_existant(&self) -> bibliotheque::IdProfil {
        let chemin = self._dossier.path().join("ancien.icc");
        std::fs::write(&chemin, icc()).unwrap();
        self.ouverte
            .ranger_profil(self.condition, Path::new(&chemin), "2026-10-01")
            .unwrap()
    }
}

fn est_au_catalogue(cle: &str) -> bool {
    app::textes::cles().any(|c| c == cle)
}

#[test]
fn un_profil_calcule_devient_actif_et_non_verifie() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::reussit(icc());

    let resultat = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &moteur,
            maintenant(),
        )
        .unwrap();

    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    let actif = detail.actif.expect("profil actif");
    assert_eq!(actif.id, resultat.profil);
    assert_eq!(actif.etat, EtatVerification::NonVerifie);
    assert_eq!(detail.origine, Some("calcul"));
    assert_eq!(
        (resultat.precision_moyenne, resultat.precision_max),
        (0.5, 2.0)
    );

    // Le moteur a reçu les trois plages au format .ti3, RVB sur 100.
    let demandes = moteur.demandes();
    assert_eq!(demandes.len(), 1);
    assert_eq!(demandes[0].1, Recette::Standard);
    assert!(demandes[0].0.starts_with("CTI3"), "{}", demandes[0].0);
    assert!(demandes[0].0.contains("NUMBER_OF_SETS 3"));
}

#[test]
fn les_parametres_gardent_commande_version_recette_et_precision() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::reussit(icc());
    let resultat = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Rapide,
            &moteur,
            maintenant(),
        )
        .unwrap();

    let texte = b.ouverte.parametres_profil(resultat.profil).unwrap();
    let p: serde_json::Value = serde_json::from_str(&texte).unwrap();
    assert_eq!(p["origine"], "calcul");
    assert_eq!(p["moteur"], "ArgyllCMS");
    assert_eq!(p["version"], profilage::VERSION_ARGYLL);
    assert_eq!(p["recette"], "rapide");
    assert_eq!(p["condition"], "M1");
    assert_eq!(p["condition_supposee"], true);
    assert_eq!(p["rvb_supposes"], true);
    assert_eq!(p["mesure"], mesure.0);
    assert_eq!(p["fichier"], "mesure00001_M1.txt");
    let commandes: Vec<&str> = p["commandes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c.as_str())
        .collect();
    assert_eq!(commandes.len(), 2, "{commandes:?}");
    assert!(commandes[0].starts_with("colprof") && commandes[0].contains("-qm"));
    assert!(commandes[1].starts_with("profcheck"));
    assert_eq!(p["precision"]["moyenne_de00"], 0.5);
    assert_eq!(p["precision"]["max_de00"], 2.0);
    assert!(p["ti3"].as_str().unwrap().starts_with("CTI3"));
    assert_eq!(p["journal"], "moteur simulé\n");
}

#[test]
fn le_nom_du_profil_est_celui_propose() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let resultat = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
        )
        .unwrap();
    let profil = b.ouverte.profil(resultat.profil).unwrap();
    assert_eq!(
        profil.nom,
        nom_profil_propose(
            "Canon PRO-4100",
            "Hahnemühle Photo Rag 308",
            "720 dpi",
            "2026-10-23"
        )
    );
    assert_eq!(profil.horodatage, "2026-10-23T09:15:00Z");
}

#[test]
fn un_echec_du_moteur_laisse_le_profil_actif_inchange() {
    let b = banc();
    let ancien = b.ranger_un_profil_existant();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::echoue(ErreurMoteur::Echec {
        etape: Etape::Profil,
        cause: "Not enough patches".to_string(),
    });

    let erreur = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &moteur,
            maintenant(),
        )
        .unwrap_err();

    assert_eq!(erreur, "profil.erreur.calcul");
    assert!(est_au_catalogue(&erreur));
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.actif.map(|p| p.id), Some(ancien));
    assert_eq!(detail.historique.len(), 1);
}

#[test]
fn argyll_introuvable_ne_range_rien() {
    // Au lancement, ArgyllCMS n'a pas été trouvé : le moteur manque.
    let erreur = Moteur(None).moteur().err();
    assert_eq!(erreur.as_deref(), Some("profil.erreur.argyll_introuvable"));
    assert!(est_au_catalogue("profil.erreur.argyll_introuvable"));

    // Trouvé puis disparu : le moteur le dit au calcul.
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let erreur = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::echoue(ErreurMoteur::Introuvable),
            maintenant(),
        )
        .unwrap_err();
    assert_eq!(erreur, "profil.erreur.argyll_introuvable");
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert!(detail.actif.is_none());
    assert!(detail.historique.is_empty());
}

#[test]
fn une_mesure_sans_rvb_ne_peut_pas_servir_de_source() {
    let b = banc();
    let lab_seul = export(
        "D50",
        "SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B",
        &["1\t95\t0\t2", "2\t42.09\t-28\t-60.57"],
    );
    let mesure = b.importer(b.condition, "lab_seul.txt", &lab_seul);

    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.sources.len(), 1);
    let source = &detail.sources[0];
    assert_eq!(source.mesure, mesure);
    assert_eq!(source.fichier, "lab_seul.txt");
    assert!(source.conditions.is_empty());
    assert_eq!(source.raison, Some("profil.refus.rvb_inconnus"));
    assert!(est_au_catalogue("profil.refus.rvb_inconnus"));

    // Le calcul la refuse aussi, sans appeler le moteur.
    let moteur = MoteurSimule::reussit(icc());
    let erreur = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &moteur,
            maintenant(),
        )
        .unwrap_err();
    assert_eq!(erreur, "profil.refus.rvb_inconnus");
    assert!(moteur.demandes().is_empty());
}

#[test]
fn une_mesure_sans_condition_complete_le_dit_autrement() {
    let b = banc();
    // RVB connus, mais une plage sans Lab : aucune condition complète.
    let incomplet = export(
        "D50",
        "SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tXYZ_X\tXYZ_Y\tXYZ_Z",
        &["1\t255\t255\t255\t96\t100\t82"],
    );
    b.importer(b.condition, "sans_lab.txt", &incomplet);
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    let source = &detail.sources[0];
    assert!(source.conditions.is_empty());
    assert_eq!(source.raison, Some("profil.refus.aucune_condition"));
    assert!(est_au_catalogue("profil.refus.aucune_condition"));
}

#[test]
fn seules_les_conditions_presentes_sont_proposees() {
    let b = banc();
    b.importer(b.condition, "mesure00001_M0.txt", &export_rvb("A"));
    b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));

    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    // Les dernières importées d'abord.
    let par_fichier: Vec<_> = detail
        .sources
        .iter()
        .map(|s| (s.fichier.as_str(), s.conditions.clone(), s.raison))
        .collect();
    assert_eq!(
        par_fichier,
        [
            ("mesure00001_M1.txt", vec![ConditionMesure::M1], None),
            ("mesure00001_M0.txt", vec![ConditionMesure::M0], None),
        ]
    );
}

#[test]
fn une_condition_absente_de_la_mesure_est_refusee() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M0.txt", &export_rvb("A"));
    let erreur = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
        )
        .unwrap_err();
    assert_eq!(erreur, "profil.refus.condition_absente");
    assert!(est_au_catalogue(&erreur));
}

#[test]
fn une_mesure_d_une_autre_resolution_est_refusee() {
    let b = banc();
    let papier = b
        .ouverte
        .parc("")
        .unwrap()
        .imprimantes
        .remove(0)
        .papiers
        .remove(0)
        .papier
        .id;
    let autre = b.ouverte.ajouter_resolution(papier, "1440 dpi").unwrap().id;
    let mesure = b.importer(autre, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::reussit(icc());

    for erreur in [
        b.ouverte
            .calculer_profil(
                b.condition,
                mesure,
                ConditionMesure::M1,
                Recette::Standard,
                &moteur,
                maintenant(),
            )
            .unwrap_err(),
        b.ouverte
            .commandes_profil(
                b.condition,
                mesure,
                ConditionMesure::M1,
                Recette::Standard,
                &moteur,
            )
            .unwrap_err(),
    ] {
        assert_eq!(erreur, "profil.erreur.mesure_ailleurs");
    }
    assert!(est_au_catalogue("profil.erreur.mesure_ailleurs"));
    assert!(moteur.demandes().is_empty());
    assert!(b
        .ouverte
        .detail_reglage(b.condition)
        .unwrap()
        .actif
        .is_none());
}

#[test]
fn les_commandes_se_montrent_sans_rien_calculer() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::reussit(icc());

    let commandes = b
        .ouverte
        .commandes_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Approfondi,
            &moteur,
        )
        .unwrap();

    assert_eq!(commandes.len(), 2, "{commandes:?}");
    assert!(commandes[0].starts_with("colprof") && commandes[0].contains("-qu"));
    // La description du profil garde les accents.
    assert!(
        commandes[0].contains("Hahnemühle Photo Rag 308"),
        "{}",
        commandes[0]
    );
    assert!(moteur.demandes().is_empty());
    assert!(b
        .ouverte
        .detail_reglage(b.condition)
        .unwrap()
        .actif
        .is_none());
}

#[test]
fn les_textes_du_calcul_existent() {
    for cle in [
        "profil.titre",
        "profil.source",
        "profil.condition",
        "profil.recette",
        "profil.recette.rapide",
        "profil.recette.standard",
        "profil.recette.approfondi",
        "profil.avance",
        "profil.calculer",
        "profil.en_cours",
        "profil.fait",
        "profil.supposee",
        "profil.refus.condition_absente",
        "profil.refus.rvb_inconnus",
        "profil.refus.lab_inconnu",
        "profil.refus.rvb_hors_echelle",
        "profil.refus.vide",
        "profil.refus.aucune_condition",
        "profil.erreur.argyll_introuvable",
        "profil.erreur.argyll_version",
        "profil.erreur.calcul",
        "profil.erreur.controle",
        "profil.erreur.fichier",
        "profil.erreur.mesure_ailleurs",
    ] {
        assert!(est_au_catalogue(cle), "{cle}");
    }
}
