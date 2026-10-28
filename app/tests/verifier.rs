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

/// Mesures décalées d'un rang (ticket #1) : la plage i porte la mesure de la
/// plage i + 1. Refusées avant tout calcul, avec la clé du contrôle et les
/// plages en cause ; rien n'est rangé.
#[test]
fn des_mesures_incoherentes_bloquent_la_verification() {
    let b = banc();
    let n = SECONDE_MIRE.len();
    let donnees: String = (0..n)
        .map(|i| {
            let mesure = ligne(i + 1, SECONDE_MIRE[(i + 1) % n], 0.0);
            let lab: Vec<&str> = mesure.split('\t').skip(4).collect();
            let [r, g, bb] = SECONDE_MIRE[i];
            format!("{}\t{r}\t{g}\t{bb}\t{}\r\n", i + 1, lab.join("\t"))
        })
        .collect();
    let texte = export(&SECONDE_MIRE, 0.0);
    let debut = texte.find("BEGIN_DATA\r\n").unwrap() + "BEGIN_DATA\r\n".len();
    let fin = texte.find("END_DATA\r\n").unwrap();
    let decale = format!("{}{donnees}{}", &texte[..debut], &texte[fin..]);
    let seconde = b.importer("decalee_M1.txt", &decale);
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
