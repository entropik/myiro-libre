//! Vérifier un profil sur une seconde mire importée, vu par l'application :
//! méthodes de `BibliothequeOuverte` (sans Tauri), moteurs simulés de la
//! crate `profilage`, jamais ArgyllCMS. Exports FD-S2w fictifs ; les erreurs
//! sont des clés du catalogue de textes.

mod banc_verification;

use app::verifier::FicheTirage;
use banc_verification::*;
use bibliotheque::{EtatVerification, IdMesure};
use profilage::{ErreurMoteur, Impression, MoteurVerificationSimule};
use verification::{Criteres, Seuils};

#[test]
fn un_profil_fidele_est_accepte_selon_la_version_du_jeu() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.etat, EtatVerification::Accepte);
    assert_eq!(resultat.calcul.resultats.plages.len(), b.mire.plages.len());
    assert_eq!(
        resultat.calcul.resultats.plages[1].id,
        b.mire.plages[1].id.to_string()
    );
    assert_eq!(resultat.calcul.instrument.as_deref(), Some("FD-9"));
    assert_eq!(resultat.calcul.fichier, "seconde_M1.txt");
    assert!(!resultat.calcul.memes_couleurs_que_le_profil);
    assert_eq!(resultat.calcul.controles.len(), 6);
    let historique = b.ouverte.historique_verifications(b.profil).unwrap();
    assert_eq!(historique.len(), 1);
    assert_eq!(
        (
            historique[0].tolerances_nom.as_str(),
            historique[0].tolerances_version
        ),
        ("Atelier", 1)
    );
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(-8.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(-8.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
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
    let seconde = b.importer("seconde_M1.txt", &b.seconde(-4.0));
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
        .map(|v| (v.verification, v.tolerances_version, v.etat))
        .collect();
    assert_eq!(
        resume,
        [
            (recalcul.verification, 2, EtatVerification::Accepte),
            (premiere.verification, 1, EtatVerification::HorsTolerance)
        ]
    );
    assert!(historique.iter().all(|v| v.tolerances_nom == "Atelier"));
    assert_eq!(b.etat(b.profil), EtatVerification::Accepte);
}

#[test]
fn la_derniere_fiche_est_reprise_non_confirmee() {
    let b = banc();
    assert_eq!(b.ouverte.derniere_fiche(b.condition).unwrap(), None);
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
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

/// Mesures décalées d'un rang (ticket #1) : la plage i porte la mesure de la
/// plage i + 1. Refusées avant tout calcul, avec la clé du contrôle et les
/// plages en cause ; rien n'est rangé.
#[test]
fn des_mesures_incoherentes_bloquent_la_verification() {
    let b = banc();
    let plages = &b.mire.plages;
    let n = plages.len();
    let lignes: Vec<String> = (0..n)
        .map(|i| {
            let rvb = plages[i].rvb.map(f64::from);
            let suivante = plages[(i + 1) % n].rvb.map(f64::from);
            ligne_lab(&plages[i].id.to_string(), rvb, lab_srvb(suivante))
        })
        .collect();
    let seconde = b.importer("decalee_M1.txt", &export_lignes(&lignes, "FD-9"));
    // Un moteur qui échouerait autrement : s'il était appelé, l'erreur serait la sienne.
    let moteur = MoteurVerificationSimule::echoue(ErreurMoteur::Introuvable);
    let erreur = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &moteur,
            a(10),
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "profil.refus.incoherent");
    let coherence = erreur.coherence.expect("plages en cause");
    assert!(!coherence.plages.is_empty());
    assert!(b
        .ouverte
        .historique_verifications(b.profil)
        .unwrap()
        .is_empty());
    assert_eq!(b.etat(b.profil), EtatVerification::NonVerifie);
}

/// Lab D50 d'un RVB sur 255 lu comme du sRVB (matrice de Bradford D65 vers
/// D50, Lindbloom) : des mesures vraisemblables d'une imprimante.
fn lab_srvb(rvb: [f64; 3]) -> [f64; 3] {
    let lin = rvb.map(|v| {
        let v = v / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    let m = [
        [0.436_074_7, 0.385_064_9, 0.143_080_4],
        [0.222_504_5, 0.716_878_6, 0.060_616_9],
        [0.013_932_2, 0.097_104_5, 0.714_173_3],
    ];
    let [x, y, z] = m.map(|l| (l[0] * lin[0] + l[1] * lin[1] + l[2] * lin[2]) * 100.0);
    let lab = colorimetrie::xyz_vers_lab(
        colorimetrie::Xyz { x, y, z },
        colorimetrie::Xyz {
            x: 96.422,
            y: 100.0,
            z: 82.521,
        },
    )
    .unwrap();
    // Le papier n'est pas un blanc parfait, l'encre n'est pas un noir parfait.
    [8.0 + lab.l * 0.87, lab.a * 0.85, lab.b * 0.85]
}

/// La mire de vérification FD-9 (210 couleurs dispersées), mesurée avec des
/// Lab vraisemblables, passe le contrôle de cohérence (#1) sans refus à tort.
#[test]
fn la_mire_de_verification_fd9_passe_le_controle_de_coherence() {
    let b = banc();
    let mesure = export_lignes(&b.lignes_mire(lab_srvb), "FD-9");
    let seconde = b.importer("mire-verification_M1.txt", &mesure);
    let resultat = b.ouverte.verifier_profil(
        &b.demande(seconde, Impression::ATraversLeProfil),
        &MoteurVerificationSimule::fidele(),
        a(10),
    );
    let erreur = resultat.as_ref().err();
    assert!(
        erreur.is_none_or(|e| e.cle != "profil.refus.incoherent"),
        "{erreur:?}"
    );
    assert!(resultat.is_ok(), "{erreur:?}");
}

/// Le numéro de série de la ligne `INSTRUMENTATION` n'est jamais gardé :
/// seul le modèle l'est.
#[test]
fn seul_le_modele_de_l_instrument_est_garde() {
    let b = banc();
    let mesure = export_lignes(
        &b.lignes_mire(|c| lab_fidele(c, 0.0)),
        INSTRUMENT_AVEC_SERIE,
    );
    let seconde = b.importer("seconde_M1.txt", &mesure);
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.calcul.instrument.as_deref(), Some("FD-9"));
    let range = serde_json::to_string(&resultat.calcul).unwrap();
    assert!(!range.contains("12345678") && !range.contains("Serial"));
}

/// Les couleurs demandées viennent de la mire rangée, retrouvée par les ID
/// de l'export : des RVB qui ne correspondent pas sont refusés, une mire
/// inconnue aussi (association incertaine, #1).
#[test]
fn une_mesure_qui_ne_correspond_a_aucune_mire_rangee_est_refusee() {
    let b = banc();
    // Une plage dont les RVB ne sont pas ceux de la mire pour son ID.
    let mut lignes = b.lignes_mire(|c| lab_fidele(c, 0.0));
    let premiere = &b.mire.plages[0];
    let autre = [1.0, 2.0, 3.0];
    lignes[0] = ligne_lab(&premiere.id.to_string(), autre, lab_fidele(autre, 0.0));
    let desaccord = b.importer("desaccord_M1.txt", &export_lignes(&lignes, "FD-9"));
    let erreur = b
        .ouverte
        .verifier_profil(
            &b.demande(desaccord, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "verif.refus.mire_desaccord");
    assert_eq!(
        erreur.cause.as_deref(),
        Some(premiere.id.to_string().as_str())
    );

    // Une mire qui n'est ni la mire de vérification ni celle du profil.
    let inconnue = b.importer(
        "inconnue_M1.txt",
        &export(
            &[
                [10.0, 20.0, 30.0],
                [200.0, 100.0, 50.0],
                [255.0, 255.0, 255.0],
            ],
            0.0,
        ),
    );
    let erreur = b
        .ouverte
        .verifier_profil(
            &b.demande(inconnue, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(11),
        )
        .unwrap_err();
    assert_eq!(erreur.cle, "verif.refus.mire_inconnue");
    assert!(b
        .ouverte
        .historique_verifications(b.profil)
        .unwrap()
        .is_empty());
}

#[test]
fn la_mire_et_l_echelle_supposee_des_rvb_sont_gardees() {
    let b = banc();
    let seconde = b.importer("seconde_M1.txt", &b.seconde(0.0));
    let resultat = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap();
    assert_eq!(resultat.calcul.mire.as_deref(), Some(b.mire.nom.as_str()));
    // FD-S2w écrit ses RVB de 0 à 255 : l'échelle est supposée.
    assert!(resultat.calcul.rvb_supposes);
}

/// La mire rangée est relue, jamais régénérée en silence : si le générateur
/// donne autre chose pour la même version, la mire est refusée.
#[test]
fn une_mire_regeneree_autrement_pour_la_meme_version_est_refusee() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = app::colonne::emplacement(dossier.path());
    let b = bibliotheque::Bibliotheque::ouvrir(&chemin).unwrap();
    b.ranger_mire_verification(&bibliotheque::NouvelleMireVerification {
        nom: "myiro-libre verification FD-9 v1",
        instrument: "fd9",
        version: verification::MIRE_FD9_VERSION,
        plages: 1,
        valeurs: "[[0.0,0.0,0.0]]",
        contenu: "<Data/>",
        horodatage: "2026-10-27T09:00:00Z",
    })
    .unwrap();
    drop(b);
    let ouverte = app::colonne::BibliothequeOuverte::ouvrir(&chemin);
    assert_eq!(
        ouverte.mire_verification_fd9(a(0)),
        Err("verif.erreur.mire_changee".to_string())
    );
}

/// La mire de vérification FD-9 est créée une fois : la demander de nouveau
/// rend la même mire (même empreinte), sans doublon, et le fichier écrit
/// pour FD-S2w est son XML, à l'octet.
#[test]
fn la_mire_de_verification_est_creee_une_seule_fois() {
    let b = banc();
    let premiere = b.ouverte.mire_verification_fd9(a(1)).unwrap();
    let encore = b.ouverte.mire_verification_fd9(a(2)).unwrap();
    assert_eq!(premiere, encore);
    assert_eq!(premiere.instrument, "fd9");
    let xml = verification::ecrire_xml_fds2w(&verification::mire_verification_fd9());
    assert_eq!(premiere.contenu, xml);
    let fichier = b.dossier.path().join("mire.xml");
    let nom = b.ouverte.exporter_mire_fd9(&fichier, a(3)).unwrap();
    assert_eq!(nom, "mire.xml");
    assert_eq!(std::fs::read_to_string(&fichier).unwrap(), xml);
    assert_eq!(b.ouverte.mire_verification_fd9(a(4)).unwrap(), premiere);
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
