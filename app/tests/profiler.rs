//! Calcul d'un profil RVB depuis une mesure importée, vu par l'application :
//! méthodes de `BibliothequeOuverte` (sans Tauri) et moteur simulé, jamais
//! ArgyllCMS. Exports FD-S2w fictifs ; les erreurs sont des clés du
//! catalogue de textes.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::profiler::{
    arrondir_duree, journal_court, provenance_depuis_parametres, ErreurProfil, HistoriqueDurees,
    Moteur, ProvenanceProfil, SuiviCalcul,
};
use bibliotheque::{nom_profil_propose, EtatVerification, IdCondition, IdMesure};
use pont_protocole::ConditionMesure;
use profilage::{Avancement, ErreurMoteur, Etape, EtapeCalcul, MoteurSimule, Recette};
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
            &mut |_| {},
        )
        .unwrap();

    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    let actif = detail.actif.expect("profil actif");
    assert_eq!(actif.id, resultat.profil);
    assert_eq!(actif.etat, EtatVerification::NonVerifie);
    assert_eq!(detail.origine, Some("calcul"));
    // Verdict et historique : « Profil calculé, à vérifier ».
    assert_eq!(detail.verdict, "parc.verdict.calcule_a_verifier");
    assert_eq!(detail.historique[0].origine, "calcul");
    assert_eq!(
        detail.historique[0].libelle,
        "parc.verdict.calcule_a_verifier"
    );
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
            &mut |_| {},
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
            &mut |_| {},
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
            &mut |_| {},
        )
        .unwrap_err();

    // La page reçoit la clé et la dernière ligne d'erreur d'ArgyllCMS.
    assert_eq!(
        erreur,
        ErreurProfil {
            cle: "profil.erreur.calcul",
            cause: Some("Not enough patches".to_string()),
            version: None,
        }
    );
    assert!(est_au_catalogue(erreur.cle));
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.actif.map(|p| p.id), Some(ancien));
    assert_eq!(detail.historique.len(), 1);
}

#[test]
fn argyll_introuvable_ne_range_rien() {
    // Au lancement, ArgyllCMS n'a pas été trouvé : le moteur manque.
    let erreur = Moteur(None).moteur().err();
    assert_eq!(
        erreur,
        Some(ErreurProfil {
            cle: "profil.erreur.argyll_introuvable",
            cause: None,
            version: None,
        })
    );
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
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "profil.erreur.argyll_introuvable");
    assert_eq!(erreur.cause, None);
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
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "profil.refus.rvb_inconnus");
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
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "profil.refus.condition_absente");
    assert!(est_au_catalogue(erreur.cle));
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
                &mut |_| {},
            )
            .unwrap_err(),
        b.ouverte
            .commandes_profil(b.condition, mesure, ConditionMesure::M1, Recette::Standard)
            .unwrap_err(),
    ] {
        assert_eq!(
            erreur,
            ErreurProfil {
                cle: "profil.erreur.mesure_ailleurs",
                cause: None,
                version: None,
            }
        );
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

    // Aucun moteur n'est demandé : la commande se construit sans ArgyllCMS.
    let commandes = b
        .ouverte
        .commandes_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Approfondi,
        )
        .unwrap();

    assert_eq!(commandes.len(), 2, "{commandes:?}");
    assert!(commandes[0].starts_with("colprof") && commandes[0].contains("-qu"));
    // La description passée à colprof est sans accents (ArgyllCMS n'écrit
    // que de l'ASCII) ; la bibliothèque garde le nom d'origine.
    assert!(
        commandes[0].contains("-D \"Canon PRO-4100, Hahnemuhle Photo Rag 308, 720 dpi\""),
        "{}",
        commandes[0]
    );
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
        "profil.erreur.argyll_illisible",
        "profil.erreur.calcul",
        "profil.erreur.controle",
        "profil.erreur.fichier",
        "profil.erreur.mesure_ailleurs",
        "profil.erreur.cause",
        "profil.rvb",
        "profil.rvb.supposee",
    ] {
        assert!(est_au_catalogue(cle), "{cle}");
    }
}

/// Sans calcul précédent avec la même recette, aucune estimation : rien
/// n'est inventé.
#[test]
fn sans_historique_aucune_duree_n_est_estimee() {
    let h = HistoriqueDurees::default();
    assert_eq!(h.estimer(Recette::Standard, 2520), None);
}

/// L'estimation reprend la durée du dernier calcul de la même recette,
/// rapportée au nombre de plages ; une autre recette n'en a pas.
#[test]
fn la_duree_s_estime_d_apres_le_dernier_calcul_de_la_recette() {
    let mut h = HistoriqueDurees::default();
    h.noter(Recette::Standard, 120.0, 1260);
    assert_eq!(h.estimer(Recette::Standard, 2520), Some(240.0));
    assert_eq!(h.estimer(Recette::Standard, 630), Some(60.0));
    assert_eq!(h.estimer(Recette::Rapide, 2520), None);
    // Le dernier calcul remplace le précédent.
    h.noter(Recette::Standard, 300.0, 2520);
    assert_eq!(h.estimer(Recette::Standard, 2520), Some(300.0));
    // Rien à rapporter : pas d'estimation.
    h.noter(Recette::Rapide, 10.0, 0);
    assert_eq!(h.estimer(Recette::Rapide, 100), None);
}

/// L'historique est enregistré sur le poste et relu au calcul suivant ; un
/// fichier absent ou illisible donne un historique vide.
#[test]
fn l_historique_des_durees_se_relit() {
    let dossier = TempDir::new().unwrap();
    let fichier = dossier.path().join("config").join("durees-calcul.json");
    assert_eq!(
        HistoriqueDurees::lire(&fichier),
        HistoriqueDurees::default()
    );
    let mut h = HistoriqueDurees::default();
    h.noter(Recette::Approfondi, 900.0, 2520);
    h.ecrire(&fichier).unwrap();
    assert_eq!(HistoriqueDurees::lire(&fichier), h);
    std::fs::write(&fichier, "pas du json").unwrap();
    assert_eq!(
        HistoriqueDurees::lire(&fichier),
        HistoriqueDurees::default()
    );
}

/// Un calcul réussi dit combien de plages il a pris : c'est ce qui rapporte
/// sa durée à la mesure suivante.
#[test]
fn un_calcul_reussi_dit_son_nombre_de_plages() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let r = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(r.plages, 3);
}

/// Avant le clic, la page reçoit la durée estimée du calcul choisi, s'il y
/// a un historique pour sa recette.
#[test]
fn l_estimation_suit_la_mesure_et_la_recette_choisies() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let mut h = HistoriqueDurees::default();
    h.noter(Recette::Standard, 60.0, 6);
    let estimer = |recette| {
        b.ouverte
            .estimation_profil(b.condition, mesure, ConditionMesure::M1, recette, &h)
            .unwrap()
    };
    assert_eq!(estimer(Recette::Standard), Some(30.0));
    assert_eq!(estimer(Recette::Rapide), None);
}

/// Dans « Avancé », chaque suite d'astérisques de colprof tient sur une
/// ligne courte avec leur nombre ; le reste du journal ne change pas.
#[test]
fn les_asterisques_du_journal_se_regroupent_a_l_affichage() {
    let journal = "$ colprof -v\r\nCreate final clut from scattered data\r\n*********\r\n****\r\nWarning\r\nfin\n";
    assert_eq!(
        journal_court(journal),
        "$ colprof -v\r\nCreate final clut from scattered data\r\n****** (×13)\r\nWarning\r\nfin\n"
    );
    assert_eq!(
        journal_court("a\n**\nb\n***"),
        "a\n****** (×2)\nb\n****** (×3)"
    );
    assert_eq!(journal_court("sans étoiles\n"), "sans étoiles\n");
    assert_eq!(journal_court("* note *\n"), "* note *\n");
}

/// Pendant le calcul, l'écran apprend l'étape en cours : préparation des
/// mesures, puis calcul du profil, puis contrôle de précision.
#[test]
fn le_calcul_annonce_ses_etapes_dans_l_ordre() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let mut annonces = Vec::new();
    b.ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
            &mut |a| annonces.push(a),
        )
        .unwrap();
    assert_eq!(
        annonces,
        [
            Avancement::Etape(EtapeCalcul::Preparation),
            Avancement::Etape(EtapeCalcul::Profil),
            Avancement::Etape(EtapeCalcul::Controle),
        ]
    );
}

/// Une mesure refusée avant le moteur n'annonce que la préparation.
#[test]
fn une_mesure_refusee_n_annonce_que_la_preparation() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let mut annonces = Vec::new();
    let moteur = MoteurSimule::reussit(icc());
    b.ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M0,
            Recette::Standard,
            &moteur,
            maintenant(),
            &mut |a| annonces.push(a),
        )
        .unwrap_err();
    assert_eq!(annonces, [Avancement::Etape(EtapeCalcul::Preparation)]);
    assert!(moteur.demandes().is_empty());
}

/// Les exports RVB de FD-S2w et de MYIROtools sont tous deux acceptés : les
/// textes qui invitent à importer les nomment tous les deux.
#[test]
fn les_textes_d_import_nomment_fd_s2w_et_myirotools() {
    use app::textes::{texte, Langue};
    for langue in [Langue::Francais, Langue::Anglais] {
        for cle in [
            "profil.refus.rvb_inconnus",
            "profil.refus.aucune_condition",
            "profil.sans_mesure",
        ] {
            let t = texte(langue, cle);
            assert!(t.contains("FD-S2w") && t.contains("MYIROtools"), "{t}");
        }
    }
}

#[test]
fn une_autre_version_d_argyll_donne_la_version_trouvee() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::echoue(ErreurMoteur::VersionNonValidee {
        trouvee: "3.4.1".to_string(),
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
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurProfil {
            cle: "profil.erreur.argyll_version",
            cause: None,
            version: Some("3.4.1".to_string()),
        }
    );
    use app::textes::{texte, Langue};
    for langue in [Langue::Francais, Langue::Anglais] {
        let t = texte(langue, "profil.erreur.argyll_version");
        assert!(t.contains("{version}") && t.contains("3.5.0"), "{t}");
    }
}

/// C4 : une version qu'on ne sait pas lire a son texte à elle, avec la
/// sortie d'ArgyllCMS en cause.
#[test]
fn une_version_illisible_a_sa_propre_cle() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::echoue(ErreurMoteur::Echec {
        etape: Etape::Version,
        cause: "colprof: Error - usage".to_string(),
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
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurProfil {
            cle: "profil.erreur.argyll_illisible",
            cause: Some("colprof: Error - usage".to_string()),
            version: None,
        }
    );
    assert!(est_au_catalogue(erreur.cle));
}

/// C6 : un fichier de travail illisible garde sa cause ; le texte ne
/// suppose pas un disque plein.
#[test]
fn un_fichier_de_travail_garde_sa_cause() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let moteur = MoteurSimule::echoue(ErreurMoteur::Fichier("accès refusé".to_string()));
    let erreur = b
        .ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &moteur,
            maintenant(),
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurProfil {
            cle: "profil.erreur.fichier",
            cause: Some("accès refusé".to_string()),
            version: None,
        }
    );
    use app::textes::{texte, Langue};
    let fr = texte(Langue::Francais, "profil.erreur.fichier");
    assert!(fr.contains("lu ou écrit") && !fr.contains("disque"), "{fr}");
    let en = texte(Langue::Anglais, "profil.erreur.fichier");
    assert!(
        en.contains("read or written") && !en.contains("disk"),
        "{en}"
    );
}

/// La recherche d'ArgyllCMS ne se fait qu'au lancement : le message le dit.
#[test]
fn argyll_introuvable_demande_de_relancer() {
    use app::textes::{texte, Langue};
    assert!(texte(Langue::Francais, "profil.erreur.argyll_introuvable").contains("relancez"));
    assert!(texte(Langue::Anglais, "profil.erreur.argyll_introuvable").contains("restart"));
}

/// Seules les durées mesurées sont données (poste de l'imprimeur, 26 octobre
/// 2026, 2 520 plages) : Rapide 58 s, Standard 3 min 03 s. Approfondi ne
/// promet aucune durée.
#[test]
fn seule_la_duree_mesuree_est_donnee() {
    use app::textes::{texte, Langue};
    for (recette, fr, en) in [
        (
            "rapide",
            "environ 1\u{a0}min pour 2\u{202f}520 plages",
            "about 1 min for 2,520 patches",
        ),
        (
            "standard",
            "environ 3\u{a0}min pour 2\u{202f}520 plages",
            "about 3 min for 2,520 patches",
        ),
    ] {
        let cle = format!("profil.recette.{recette}");
        assert!(texte(Langue::Francais, &cle).contains(fr), "{cle}");
        assert!(texte(Langue::Anglais, &cle).contains(en), "{cle}");
    }
    let fr = texte(Langue::Francais, "profil.recette.approfondi");
    let en = texte(Langue::Anglais, "profil.recette.approfondi");
    assert!(fr.contains("durée à mesurer"), "{fr}");
    assert!(en.contains("duration to be measured"), "{en}");
    assert!(!fr.contains("min") && !en.contains("min"), "{fr} / {en}");
}

#[test]
fn la_cause_s_ecrit_dans_les_deux_langues() {
    use app::textes::{texte, Langue};
    for langue in [Langue::Francais, Langue::Anglais] {
        let t = texte(langue, "profil.erreur.cause");
        assert!(t.contains("{cause}") && t.contains("ArgyllCMS"), "{t}");
    }
}

#[test]
fn la_provenance_d_un_profil_calcule_est_donnee_a_la_page() {
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
            &mut |_| {},
        )
        .unwrap();
    let texte = b.ouverte.parametres_profil(resultat.profil).unwrap();
    let parametres: serde_json::Value = serde_json::from_str(&texte).unwrap();
    let commandes: Vec<String> = parametres["commandes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c.as_str().map(String::from))
        .collect();
    assert_eq!(commandes.len(), 2, "{commandes:?}");

    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(
        detail.provenance,
        Some(ProvenanceProfil {
            // La commande et le journal restent consultables après le calcul.
            commandes,
            journal: "moteur simulé\n".to_string(),
            mesure,
            fichier: "mesure00001_M1.txt".to_string(),
            condition: ConditionMesure::M1,
            condition_supposee: true,
            // Export FD-S2w : échelle 0 à 255 supposée.
            rvb_supposes: true,
            recette: Recette::Standard,
            version: "3.5.0".to_string(),
            precision_moyenne: 0.5,
            precision_max: 2.0,
            // La durée mesurée du calcul, la même que celle retenue pour
            // estimer le suivant.
            duree_s: Some(resultat.duree_s),
        })
    );
    // Durée rangée au dixième de seconde : elle se relit à l'identique.
    assert!(resultat.duree_s >= 0.0 && resultat.duree_s.is_finite());
    assert_eq!(
        resultat.duree_s,
        (resultat.duree_s * 10.0).round() / 10.0,
        "durée non arrondie au dixième"
    );
    assert_eq!(parametres["duree_s"].as_f64(), Some(resultat.duree_s));
}

/// La durée est arrondie au dixième de seconde avant d'être rangée.
#[test]
fn la_duree_est_arrondie_au_dixieme() {
    assert_eq!(arrondir_duree(182.96), 183.0);
    assert_eq!(arrondir_duree(58.04), 58.0);
    assert_eq!(arrondir_duree(0.0004), 0.0);
    assert_eq!(arrondir_duree(3.25), 3.3);
}

/// Un profil calculé avant que la durée soit gardée n'en a pas : rien
/// n'est inventé, le reste de sa provenance se lit comme avant.
#[test]
fn un_profil_calcule_sans_duree_gardee_n_en_montre_pas() {
    let ancien = r#"{"origine":"calcul","moteur":"ArgyllCMS","version":"3.5.0","recette":"standard","condition":"M1","condition_supposee":false,"rvb_supposes":true,"mesure":3,"fichier":"mesure.txt","commandes":["colprof"],"precision":{"moyenne_de00":0.22,"max_de00":3.53},"ti3":"CTI3","journal":"j"}"#;
    let p = provenance_depuis_parametres(ancien).expect("provenance lue");
    assert_eq!(p.duree_s, None);
    assert_eq!(p.recette, Recette::Standard);
    let nouveau = ancien.replace(r#""journal":"j""#, r#""journal":"j","duree_s":183.4"#);
    assert_eq!(
        provenance_depuis_parametres(&nouveau).unwrap().duree_s,
        Some(183.4)
    );
    // Un profil rangé tel quel n'a pas de provenance de calcul.
    assert_eq!(
        provenance_depuis_parametres(r#"{"origine":"profil existant"}"#),
        None
    );
}

#[test]
fn la_duree_du_calcul_se_dit_dans_les_deux_langues() {
    use app::textes::{texte, Langue};
    assert_eq!(texte(Langue::Francais, "profil.duree"), "Durée du calcul");
    assert_eq!(texte(Langue::Anglais, "profil.duree"), "Calculation time");
}

#[test]
fn un_profil_seulement_range_n_a_pas_de_provenance_de_calcul() {
    let b = banc();
    b.ranger_un_profil_existant();
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert!(detail.actif.is_some());
    assert_eq!(detail.provenance, None);
    // Sans profil non plus.
    let vide = banc();
    assert_eq!(
        vide.ouverte
            .detail_reglage(vide.condition)
            .unwrap()
            .provenance,
        None
    );
}

/// C3 : sans ArgyllCMS, la page reçoit quand même la commande, avec la
/// raison qui empêchera le calcul.
#[test]
fn sans_argyll_la_page_sait_pourquoi_avant_le_clic() {
    assert_eq!(
        Moteur(None).indisponible(),
        Some(ErreurProfil {
            cle: "profil.erreur.argyll_introuvable",
            cause: None,
            version: None,
        })
    );
    assert_eq!(
        Moteur(Some(profilage::ArgyllCms::new("argyll".into()))).indisponible(),
        None
    );
}

/// L'événement d'avancement porte le numéro du calcul que la page a lancé :
/// elle ignore ceux d'un autre calcul. Il porte aussi ce que la barre montre
/// (étape, étapes faites, position…), tiré des annonces du moteur.
#[test]
fn l_evenement_d_avancement_porte_le_numero_du_calcul() {
    let b = banc();
    let mesure = b.importer(b.condition, "mesure00001_M1.txt", &export_rvb("D50"));
    let mut suivi = SuiviCalcul::new(42);
    let mut evenements = Vec::new();
    b.ouverte
        .calculer_profil(
            b.condition,
            mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
            &mut |a| evenements.push(serde_json::to_value(suivi.recevoir(&a)).unwrap()),
        )
        .unwrap();
    assert_eq!(evenements.len(), 3);
    assert!(evenements.iter().all(|e| e["calcul"] == 42));
    let etapes: Vec<_> = evenements.iter().map(|e| e["etape"].clone()).collect();
    assert_eq!(etapes, ["preparation", "profil", "controle"]);
    assert_eq!(evenements[2]["faites"], 6);
    assert_eq!(evenements[2]["total"], 7);
}
