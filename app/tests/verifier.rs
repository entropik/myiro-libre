//! Vérifier un profil sur une seconde mire importée, vu par l'application :
//! méthodes de `BibliothequeOuverte` (sans Tauri), moteurs simulés de la
//! crate `profilage`, jamais ArgyllCMS. Exports FD-S2w fictifs ; les erreurs
//! sont des clés du catalogue de textes.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::verifier::{DemandeVerification, FicheTirage};
use bibliotheque::{EtatVerification, IdCondition, IdMesure, IdProfil, IdTolerances};
use pont_protocole::ConditionMesure;
use profilage::{ErreurMoteur, Impression, MoteurSimule, MoteurVerificationSimule, Recette};
use tempfile::TempDir;
use verification::{Criteres, Seuils};

fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

/// Le 28 octobre 2026 à 9 h 00, UTC, plus `minutes`.
fn a(minutes: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_793_178_000 + 60 * minutes)
}

/// Une plage : RVB sur 255 et le Lab que le profil simulé fidèle prédit
/// (L = 20 + 75 (r + g + b) / 3, a = 40 (r − g), b = 40 (g − b)), L
/// décalé de `decalage`.
fn ligne(id: usize, rvb: [f64; 3], decalage: f64) -> String {
    let [r, g, b] = rvb.map(|v| v / 255.0);
    let l = 20.0 + 75.0 * (r + g + b) / 3.0 + decalage;
    let (a, bb) = (40.0 * (r - g), 40.0 * (g - b));
    format!(
        "{id}\t{}\t{}\t{}\t{l:.4}\t{a:.4}\t{bb:.4}",
        rvb[0], rvb[1], rvb[2]
    )
}

const MIRE_PROFIL: [[f64; 3]; 6] = [
    [255.0, 255.0, 255.0],
    [0.0, 0.0, 0.0],
    [255.0, 0.0, 0.0],
    [0.0, 255.0, 0.0],
    [0.0, 0.0, 255.0],
    [127.5, 127.5, 127.5],
];

const SECONDE_MIRE: [[f64; 3]; 6] = [
    [255.0, 255.0, 255.0],
    [51.0, 51.0, 51.0],
    [204.0, 102.0, 51.0],
    [51.0, 153.0, 102.0],
    [102.0, 51.0, 204.0],
    [178.5, 178.5, 178.5],
];

/// Export fictif à la manière de FD-S2w, en M1 (source D50).
fn export(rvb: &[[f64; 3]], decalage: f64) -> String {
    let donnees: String = rvb
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}\r\n", ligne(i + 1, *c, decalage)))
        .collect();
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n\
         INSTRUMENTATION\t\"FD-9\"\r\nMEASUREMENT_SOURCE\t\"D50\"\r\nBEGIN_DATA_FORMAT\r\n\
         SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n{donnees}END_DATA\r\n",
        rvb.len()
    )
}

fn atelier() -> Criteres {
    Criteres {
        fidelite: Seuils {
            moyenne: Some(2.0),
            p95: Some(4.0),
            max: Some(6.0),
        },
        prediction: Seuils {
            moyenne: Some(1.5),
            p95: Some(3.0),
            max: Some(5.0),
        },
        seuil_gamut: 1.0,
    }
}

fn fiche(impression: Impression) -> FicheTirage {
    FicheTirage {
        impression,
        logiciel: "Logiciel de l'atelier".into(),
        support_pilote: "Premium Semigloss".into(),
        qualite: "1440 dpi".into(),
        encres: "UltraChrome Pro".into(),
        remarques: String::new(),
        confirmee: true,
    }
}

struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    condition: IdCondition,
    mesure_profil: IdMesure,
    profil: IdProfil,
    atelier: IdTolerances,
}

/// Epson SC-P5300 → Semigloss → 1440 dpi, un profil calculé (moteur
/// simulé) depuis la mire du profil, et le jeu « Atelier ».
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let epson = ouverte.ajouter_imprimante("Epson SC-P5300").unwrap();
    let condition = ouverte
        .ajouter_papier(epson.id, "Semigloss 250", "1440 dpi")
        .unwrap()
        .id;
    let mut banc = Banc {
        dossier,
        ouverte,
        condition,
        mesure_profil: IdMesure(0),
        profil: IdProfil(0),
        atelier: IdTolerances(0),
    };
    banc.mesure_profil = banc.importer("mire-profil_M1.txt", &export(&MIRE_PROFIL, 0.0));
    banc.profil = banc.calculer(banc.mesure_profil, a(0));
    banc.atelier = banc
        .ouverte
        .enregistrer_tolerances("Atelier", &atelier(), a(1))
        .unwrap();
    banc
}

impl Banc {
    fn importer(&self, nom: &str, texte: &str) -> IdMesure {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, texte).unwrap();
        let apercu = self.ouverte.apercu_cgats(&chemin).unwrap();
        self.ouverte
            .ranger_apercu(apercu.numero, self.condition)
            .unwrap()
    }

    fn calculer(&self, mesure: IdMesure, quand: SystemTime) -> IdProfil {
        self.ouverte
            .calculer_profil(
                self.condition,
                mesure,
                ConditionMesure::M1,
                Recette::Standard,
                &MoteurSimule::reussit(icc()),
                quand,
                &mut |_| {},
            )
            .unwrap()
            .profil
    }

    fn demande(&self, mesure: IdMesure, impression: Impression) -> DemandeVerification {
        DemandeVerification {
            profil: self.profil,
            mesure,
            mesure_condition: ConditionMesure::M1,
            tolerances: self.atelier,
            fiche: fiche(impression),
        }
    }

    fn etat(&self, profil: IdProfil) -> EtatVerification {
        self.ouverte
            .detail_reglage(self.condition)
            .unwrap()
            .historique
            .iter()
            .find(|p| p.id == profil)
            .unwrap()
            .etat
    }
}

#[test]
fn un_profil_fidele_est_accepte_selon_la_version_du_jeu() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.etat, EtatVerification::Accepte);
    assert_eq!(resultat.calcul.resultats.plages.len(), 6);
    assert_eq!(resultat.calcul.resultats.plages[1].id, "2");
    assert_eq!(resultat.calcul.instrument.as_deref(), Some("FD-9"));
    assert_eq!(resultat.calcul.fichier, "seconde_M1.txt");
    assert!(!resultat.calcul.memes_couleurs_que_le_profil);
    assert_eq!(resultat.calcul.controles.len(), 6);
    let historique = b.ouverte.historique_verifications(b.profil).unwrap();
    assert_eq!(historique.len(), 1);
    assert_eq!(historique[0].tolerances, "Atelier v1");
    assert_eq!(historique[0].etat, EtatVerification::Accepte);
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.actif.unwrap().etat, EtatVerification::Accepte);
    assert_eq!(
        detail.verification.map(|v| v.verification),
        Some(resultat.verification)
    );
}

#[test]
fn un_profil_hors_tolerance_reste_actif_et_exportable() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, -8.0));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.etat, EtatVerification::HorsTolerance);
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    let actif = detail.actif.unwrap();
    assert_eq!(
        (actif.id, actif.etat),
        (b.profil, EtatVerification::HorsTolerance)
    );
    let fichier = b.dossier.path().join("exporte.icc");
    b.ouverte.exporter_profil(b.profil, &fichier).unwrap();
    assert_eq!(std::fs::read(&fichier).unwrap(), icc());
}

#[test]
fn la_mesure_du_profil_ne_peut_pas_le_verifier() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    let erreur = b
        .ouverte
        .verifier_profil(
            &b.demande(b.mesure_profil, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::echoue(ErreurMoteur::Introuvable),
            a(10),
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "verif.refus.mesure_du_profil");
    let sources = b.ouverte.sources_verification(b.profil).unwrap();
    let raison = |m: IdMesure| sources.iter().find(|s| s.mesure == m).unwrap().raison;
    assert_eq!(
        raison(b.mesure_profil),
        Some("verif.refus.mesure_du_profil")
    );
    assert_eq!(raison(seconde), None);
    assert!(b
        .ouverte
        .historique_verifications(b.profil)
        .unwrap()
        .is_empty());
}

#[test]
fn une_fiche_non_confirmee_est_refusee() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    let mut demande = b.demande(seconde, Impression::ATraversLeProfil);
    demande.fiche.confirmee = false;
    let erreur = b
        .ouverte
        .verifier_profil(&demande, &MoteurVerificationSimule::fidele(), a(10))
        .unwrap_err();
    assert_eq!(erreur.cle, "verif.refus.fiche");
    assert_eq!(b.etat(b.profil), EtatVerification::NonVerifie);
}

#[test]
fn sans_gestion_avec_un_critere_de_fidelite_la_verification_est_incomplete() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::SansGestion),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.etat, EtatVerification::Incomplet);
    assert_eq!(resultat.calcul.resultats.fidelite, None);
    assert!(resultat.calcul.resultats.prediction.is_some());
    assert_eq!(b.etat(b.profil), EtatVerification::Incomplet);
}

#[test]
fn verifier_un_ancien_profil_ne_touche_pas_le_profil_actif() {
    let b = banc();
    let ancien = b.profil;
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, -8.0));
    let recent = b.calculer(b.mesure_profil, a(5));
    b.ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    let actif = detail.actif.unwrap();
    assert_eq!(
        (actif.id, actif.etat),
        (recent, EtatVerification::NonVerifie)
    );
    assert_eq!(detail.verification, None);
    assert_eq!(b.etat(ancien), EtatVerification::HorsTolerance);
}

#[test]
fn sans_xicclu_rien_n_est_range() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    let moteur = MoteurVerificationSimule::echoue(ErreurMoteur::OutilManquant {
        outil: "xicclu".into(),
    });
    let erreur = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &moteur,
            a(10),
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "profil.erreur.outil_manquant");
    assert_eq!(erreur.cause.as_deref(), Some("xicclu"));
    assert!(b
        .ouverte
        .historique_verifications(b.profil)
        .unwrap()
        .is_empty());
    assert_eq!(b.etat(b.profil), EtatVerification::NonVerifie);
    // Le calcul de profil reste possible.
    b.calculer(seconde, a(20));
}

#[test]
fn un_recalcul_avec_une_nouvelle_version_ajoute_une_verification() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, -4.0));
    let premiere = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(premiere.etat, EtatVerification::HorsTolerance);
    let large = Criteres {
        prediction: Seuils {
            moyenne: Some(8.0),
            p95: Some(9.0),
            max: Some(10.0),
        },
        fidelite: Seuils {
            moyenne: Some(8.0),
            p95: None,
            max: None,
        },
        seuil_gamut: 1.0,
    };
    let v2 = b
        .ouverte
        .enregistrer_tolerances("Atelier", &large, a(20))
        .unwrap();
    let recalcul = b
        .ouverte
        .recalculer_verification(premiere.verification, v2, a(21))
        .unwrap();
    assert_eq!(recalcul.etat, EtatVerification::Accepte);
    assert_eq!(
        recalcul.calcul.resultats.plages,
        premiere.calcul.resultats.plages
    );
    let historique = b.ouverte.historique_verifications(b.profil).unwrap();
    let resume: Vec<_> = historique
        .iter()
        .map(|v| (v.verification, v.tolerances.as_str(), v.etat))
        .collect();
    assert_eq!(
        resume,
        [
            (
                recalcul.verification,
                "Atelier v2",
                EtatVerification::Accepte
            ),
            (
                premiere.verification,
                "Atelier v1",
                EtatVerification::HorsTolerance
            )
        ]
    );
    assert_eq!(b.etat(b.profil), EtatVerification::Accepte);
}

#[test]
fn la_derniere_fiche_est_reprise_non_confirmee() {
    let b = banc();
    assert_eq!(b.ouverte.derniere_fiche(b.condition).unwrap(), None);
    let seconde = b.importer("seconde_M1.txt", &export(&SECONDE_MIRE, 0.0));
    b.ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    let reprise = b.ouverte.derniere_fiche(b.condition).unwrap().unwrap();
    assert!(!reprise.confirmee);
    assert_eq!(
        FicheTirage {
            confirmee: true,
            ..reprise
        },
        fiche(Impression::ATraversLeProfil)
    );
}

#[test]
fn les_memes_couleurs_que_la_mire_du_profil_sont_signalees() {
    let b = banc();
    let meme = b.importer("reimpression_M1.txt", &export(&MIRE_PROFIL, 0.2));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(meme, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert!(resultat.calcul.memes_couleurs_que_le_profil);
}

#[test]
fn un_jeu_sans_seuil_est_refuse_avant_d_etre_range() {
    let b = banc();
    let vide = Criteres {
        fidelite: Seuils::default(),
        prediction: Seuils::default(),
        seuil_gamut: 1.0,
    };
    let erreur = b
        .ouverte
        .enregistrer_tolerances("Vide", &vide, a(2))
        .unwrap_err();
    assert_eq!(erreur.cle, "verif.tolerances.aucun_seuil");
    let jeux = b.ouverte.jeux_tolerances().unwrap();
    assert_eq!(jeux.len(), 1);
    assert_eq!((jeux[0].nom.as_str(), jeux[0].version), ("Atelier", 1));
}

#[test]
fn les_textes_de_la_verification_existent() {
    for cle in [
        "verif.refus.fiche",
        "verif.refus.mesure_du_profil",
        "verif.refus.cmjn",
        "verif.refus.vide",
        "verif.refus.tailles",
        "verif.refus.valeur",
        "verif.tolerances.aucun_seuil",
        "verif.tolerances.seuil_invalide",
        "verif.erreur.calcul",
        "profil.erreur.outil_manquant",
    ] {
        assert!(app::textes::cles().any(|c| c == cle), "{cle}");
    }
}
