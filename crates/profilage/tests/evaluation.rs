//! Interroger un profil par `xicclu` derrière le trait `MoteurVerification` :
//! arguments, lecture de la sortie, moteur simulé et ArgyllCMS incomplet.
//! Les essais avec le vrai ArgyllCMS sont dans `tests/argyll.rs`.

use std::path::Path;

use profilage::{
    arguments_xicclu, lire_sortie_xicclu, ArgyllCms, DemandeEvaluation, ErreurMoteur, Etape,
    Impression, MoteurVerification, MoteurVerificationSimule, VERSION_ARGYLL,
};

fn textes(a: Vec<std::ffi::OsString>) -> Vec<String> {
    a.iter().map(|s| s.to_string_lossy().into_owned()).collect()
}

#[test]
fn xicclu_est_appele_en_relatif_vers_lab() {
    let a = textes(arguments_xicclu("b", "r", Path::new("profil.icc")));
    assert_eq!(a, ["-v0", "-fb", "-ir", "-pl", "profil.icc"]);
    let a = textes(arguments_xicclu("f", "a", Path::new("sRGB.icm")));
    assert_eq!(a, ["-v0", "-ff", "-ia", "-pl", "sRGB.icm"]);
}

/// Forme relevée le 28 octobre 2026 avec `xicclu -v0` d'ArgyllCMS 3.5.0 :
/// trois nombres à six décimales séparés par une espace, fins de ligne CRLF.
#[test]
fn la_sortie_de_xicclu_se_lit_ligne_par_ligne() {
    let sortie = "100.000000 0.000000 0.000000\r\n54.290039 80.810432 69.895681\r\n\
                  53.389482 0.000000 -0.000000\r\n";
    assert_eq!(
        lire_sortie_xicclu(sortie, 3),
        Ok(vec![
            [100.0, 0.0, 0.0],
            [54.290039, 80.810432, 69.895681],
            [53.389482, 0.0, -0.0]
        ])
    );
}

#[test]
fn une_sortie_trop_courte_est_refusee() {
    let erreur = lire_sortie_xicclu("50 0 0\n", 2).unwrap_err();
    assert!(erreur.contains('1') && erreur.contains('2'), "{erreur}");
    assert!(lire_sortie_xicclu("50 0 0\n60 1\n", 2).is_err());
    assert!(lire_sortie_xicclu("50 x 0\n", 1).is_err());
    assert!(lire_sortie_xicclu("50 0 0\n51 0 0\n52 0 0\n", 2).is_err());
}

#[test]
fn sans_gestion_les_commandes_ne_demandent_pas_srgb() {
    let moteur = ArgyllCms::new("absent".into());
    let sans = moteur.commandes_verification(Impression::SansGestion);
    assert_eq!(
        sans,
        [
            "xicclu -v0 -ff -ir -pl profil.icc",
            "xicclu -v0 -ff -ia -pl profil.icc"
        ]
    );
    let avec = moteur.commandes_verification(Impression::ATraversLeProfil);
    assert_eq!(
        avec,
        [
            "xicclu -v0 -ff -ir -pl sRGB.icm",
            "xicclu -v0 -fb -ir -pl profil.icc",
            "xicclu -v0 -ff -ir -pl profil.icc",
            "xicclu -v0 -ff -ia -pl profil.icc"
        ]
    );
    assert_eq!(
        MoteurVerificationSimule::fidele().commandes_verification(Impression::SansGestion),
        sans
    );
}

#[test]
fn le_moteur_simule_fidele_rend_l_appareil_egal_aux_rvb() {
    let rvb = [[1.0, 1.0, 1.0], [0.25, 0.5, 0.75], [0.0, 0.0, 0.0]];
    let moteur = MoteurVerificationSimule::fidele();
    let e = moteur
        .evaluer(&DemandeEvaluation {
            icc: b"profil",
            rvb: &rvb,
            impression: Impression::ATraversLeProfil,
        })
        .unwrap();
    assert_eq!(e.appareil, rvb);
    let demande = e.demande.expect("demande");
    assert_eq!(demande.len(), 3);
    assert_eq!(e.prediction_rel, demande);
    assert_eq!(e.prediction_abs, demande);
    // Fonction simple et régulière des RVB (mire synthétique du plan 3).
    assert_eq!(demande[1], [20.0 + 37.5, -10.0, -10.0]);
    assert_eq!(e.version, VERSION_ARGYLL);
    assert_eq!(e.commandes.len(), 4);

    let sans = moteur
        .evaluer(&DemandeEvaluation {
            icc: b"profil",
            rvb: &rvb,
            impression: Impression::SansGestion,
        })
        .unwrap();
    assert_eq!(sans.demande, None);
    assert_eq!(sans.appareil, rvb);
}

#[test]
fn le_moteur_simule_rend_ou_echoue_comme_demande() {
    let erreur = ErreurMoteur::Echec {
        etape: Etape::Verification,
        cause: "xicclu: Error - profil illisible".into(),
    };
    assert_eq!(erreur.cle(), "verif.erreur.calcul");
    assert!(erreur.to_string().contains("vérification"), "{erreur}");
    let demande = DemandeEvaluation {
        icc: b"profil",
        rvb: &[[0.5; 3]],
        impression: Impression::SansGestion,
    };
    let moteur = MoteurVerificationSimule::echoue(erreur.clone());
    assert_eq!(moteur.evaluer(&demande), Err(erreur));
    let rendu = MoteurVerificationSimule::fidele()
        .evaluer(&demande)
        .unwrap();
    let moteur = MoteurVerificationSimule::rend(rendu.clone());
    assert_eq!(moteur.evaluer(&demande), Ok(rendu));
}

/// Un ArgyllCMS du plan 3 copié à moitié : `colprof`, `profcheck` et
/// `sRGB.icm`, mais pas `xicclu`. L'outil est dit manquant, rien n'est
/// lancé.
#[test]
fn sans_xicclu_l_outil_est_dit_manquant() {
    let dossier = tempfile::tempdir().unwrap();
    let bin = dossier.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(dossier.path().join("ref")).unwrap();
    for outil in ["colprof", "profcheck"] {
        std::fs::write(
            bin.join(format!("{outil}{}", std::env::consts::EXE_SUFFIX)),
            b"",
        )
        .unwrap();
    }
    std::fs::write(dossier.path().join("ref").join("sRGB.icm"), b"").unwrap();
    let moteur = ArgyllCms::new(dossier.path().to_path_buf());
    let erreur = moteur
        .evaluer(&DemandeEvaluation {
            icc: b"profil",
            rvb: &[[0.5; 3]],
            impression: Impression::ATraversLeProfil,
        })
        .unwrap_err();
    assert_eq!(
        erreur,
        ErreurMoteur::OutilManquant {
            outil: "xicclu".into()
        }
    );
    assert_eq!(erreur.cle(), "profil.erreur.outil_manquant");
    assert!(erreur.to_string().contains("xicclu"), "{erreur}");
}

#[test]
fn sans_argyll_l_evaluation_est_introuvable() {
    let vide = tempfile::tempdir().unwrap();
    let moteur = ArgyllCms::new(vide.path().join("argyll"));
    let erreur = moteur
        .evaluer(&DemandeEvaluation {
            icc: b"profil",
            rvb: &[[0.5; 3]],
            impression: Impression::SansGestion,
        })
        .unwrap_err();
    assert_eq!(erreur, ErreurMoteur::Introuvable);
}

#[test]
fn une_evaluation_se_relit_a_l_identique_en_json() {
    let e = MoteurVerificationSimule::fidele()
        .evaluer(&DemandeEvaluation {
            icc: b"profil",
            rvb: &[[0.1, 0.2, 0.3]],
            impression: Impression::ATraversLeProfil,
        })
        .unwrap();
    let texte = serde_json::to_string(&e).unwrap();
    assert_eq!(
        serde_json::from_str::<profilage::Evaluation>(&texte).unwrap(),
        e
    );
    assert_eq!(
        serde_json::to_string(&Impression::ATraversLeProfil).unwrap(),
        "\"a_travers_le_profil\""
    );
}
