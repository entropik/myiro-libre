//! Parc de la bibliothèque vu par l'application : imprimantes, papiers,
//! résolutions et profils ICC rangés, par les méthodes de
//! `BibliothequeOuverte` (sans Tauri). Les erreurs sont des clés du
//! catalogue de textes. Profils fictifs : en-têtes ICC sans étiquette.

use std::path::{Path, PathBuf};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::parc::si_choisi;
use bibliotheque::{EspaceProfil, IdCondition};
use tempfile::TempDir;

/// En-tête ICC minimal de 132 octets : taille, espace de données, `acsp`.
fn icc(espace: &[u8; 4]) -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(espace);
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

fn ouverte() -> (TempDir, BibliothequeOuverte) {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    (dossier, ouverte)
}

fn fichier(dossier: &Path, nom: &str, octets: &[u8]) -> PathBuf {
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, octets).unwrap();
    chemin
}

/// Canon PRO-4100 → HMPhoto Luster 290 → 1200 ppp.
fn reglage(ouverte: &BibliothequeOuverte) -> IdCondition {
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    ouverte
        .ajouter_papier(canon.id, "HMPhoto Luster 290", "1200 ppp")
        .unwrap()
        .id
}

fn est_au_catalogue(cle: &str) -> bool {
    app::textes::cles().any(|c| c == cle)
}

#[test]
fn un_parc_vide_propose_d_ajouter_une_imprimante() {
    let (_dossier, ouverte) = ouverte();
    let parc = ouverte.parc("").unwrap();
    assert!(parc.imprimantes.is_empty());
    assert!(parc.non_classees.is_empty());
    for cle in ["parc.vide", "parc.ajouter_imprimante"] {
        assert!(est_au_catalogue(cle), "{cle}");
    }
}

#[test]
fn ajouter_un_papier_cree_sa_premiere_resolution() {
    let (_dossier, ouverte) = ouverte();
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "HMPhoto Luster 290", "1200 ppp")
        .unwrap();

    let parc = ouverte.parc("").unwrap();
    assert_eq!(parc.imprimantes.len(), 1);
    let papiers = &parc.imprimantes[0].papiers;
    assert_eq!(papiers.len(), 1);
    assert_eq!(papiers[0].papier.nom, "HMPhoto Luster 290");
    assert_eq!(papiers[0].reglages.len(), 1);
    assert_eq!(papiers[0].reglages[0].resolution, "1200 ppp");
    assert_eq!(papiers[0].reglages[0].condition.id, condition.id);
    assert!(papiers[0].reglages[0].profil.is_none());

    // Une seconde résolution sur le même papier.
    ouverte
        .ajouter_resolution(papiers[0].papier.id, "600 ppp")
        .unwrap();
    let parc = ouverte.parc("").unwrap();
    let resolutions: Vec<_> = parc.imprimantes[0].papiers[0]
        .reglages
        .iter()
        .map(|r| r.resolution.as_str())
        .collect();
    assert_eq!(resolutions, ["1200 ppp", "600 ppp"]);
}

#[test]
fn ranger_un_profil_existant_le_rend_actif_avec_le_nom_propose() {
    let (dossier, ouverte) = ouverte();
    let condition = reglage(&ouverte);
    let profil = fichier(dossier.path(), "atelier.icc", &icc(b"RGB "));

    let id = ouverte
        .ranger_profil(condition, &profil, "2026-10-22")
        .unwrap();

    let detail = ouverte.detail_reglage(condition).unwrap();
    assert_eq!(detail.imprimante, "Canon PRO-4100");
    assert_eq!(detail.papier, "HMPhoto Luster 290");
    assert_eq!(detail.resolution, "1200 ppp");
    let actif = detail.actif.expect("profil actif");
    assert_eq!(actif.id, id);
    assert_eq!(
        actif.nom,
        "Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-22.icc"
    );
    assert_eq!(actif.espace, EspaceProfil::Rvb);
    assert_eq!(detail.historique.len(), 1);
    assert!(!detail.ancien);
    // Provenance : un profil rangé tel quel, sans calcul ni mesure.
    assert_eq!(detail.origine, Some("profil_existant"));
    // Verdict et historique : « Profil importé, à vérifier ».
    assert_eq!(detail.verdict, "parc.verdict.importe_a_verifier");
    assert_eq!(detail.historique[0].origine, "profil_existant");
    assert_eq!(
        detail.historique[0].libelle,
        "parc.verdict.importe_a_verifier"
    );
    // Rangé tel quel : pas de recette.
    assert_eq!(detail.historique[0].recette, None);
    assert!(detail.mesures.is_empty() && detail.importees.is_empty());
}

#[test]
fn ranger_un_fichier_qui_n_est_pas_un_profil_donne_une_cle_de_texte() {
    let (dossier, ouverte) = ouverte();
    let condition = reglage(&ouverte);
    let pdf = fichier(dossier.path(), "devis.pdf", b"%PDF-1.7 fictif");
    let mut gris = icc(b"GRAY");
    gris[0..4].copy_from_slice(&132u32.to_be_bytes());
    let gris = fichier(dossier.path(), "gris.icc", &gris);

    for chemin in [pdf, gris] {
        let cle = ouverte
            .ranger_profil(condition, &chemin, "2026-10-22")
            .unwrap_err();
        assert_eq!(cle, "parc.erreur.profil_illisible");
        assert!(est_au_catalogue(&cle));
    }
    let detail = ouverte.detail_reglage(condition).unwrap();
    assert_eq!((detail.actif, detail.origine), (None, None));
    assert_eq!(detail.verdict, "parc.verdict.aucun");

    // Un fichier introuvable aussi, sans rien changer.
    let absent = dossier.path().join("absent.icc");
    let cle = ouverte
        .ranger_profil(condition, &absent, "2026-10-22")
        .unwrap_err();
    assert!(est_au_catalogue(&cle), "{cle}");
}

#[test]
fn exporter_un_profil_ecrit_le_fichier_a_l_identique() {
    let (dossier, ouverte) = ouverte();
    let condition = reglage(&ouverte);
    let octets = icc(b"CMYK");
    let source = fichier(dossier.path(), "source.icm", &octets);
    let id = ouverte
        .ranger_profil(condition, &source, "2026-10-22")
        .unwrap();

    let cible = dossier.path().join("copie.icc");
    ouverte.exporter_profil(id, &cible).unwrap();
    assert_eq!(std::fs::read(&cible).unwrap(), octets);

    let cle = ouverte
        .exporter_profil(bibliotheque::IdProfil(999), &cible)
        .unwrap_err();
    assert_eq!(cle, "parc.erreur.profil_inconnu");
    assert!(est_au_catalogue(&cle));
}

#[test]
fn annuler_le_choix_du_fichier_ne_change_rien() {
    let (dossier, ouverte) = ouverte();
    let condition = reglage(&ouverte);
    let avant = ouverte.parc("").unwrap();

    let range = si_choisi(None, |f| ouverte.ranger_profil(condition, f, "2026-10-22"));
    assert_eq!(range, Ok(None));
    assert_eq!(ouverte.parc("").unwrap(), avant);
    assert_eq!(ouverte.detail_reglage(condition).unwrap().actif, None);

    // Avec un fichier choisi, la même fonction range.
    let profil = fichier(dossier.path(), "atelier.icc", &icc(b"RGB "));
    let range = si_choisi(Some(profil), |f| {
        ouverte.ranger_profil(condition, f, "2026-10-22")
    });
    assert!(matches!(range, Ok(Some(_))));
}

#[test]
fn un_nom_de_papier_en_double_donne_une_cle_de_texte() {
    let (_dossier, ouverte) = ouverte();
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    ouverte
        .ajouter_papier(canon.id, "Baryta II", "1200 ppp")
        .unwrap();

    let cle = ouverte
        .ajouter_papier(canon.id, "baryta ii", "600 ppp")
        .unwrap_err();
    assert_eq!(cle, "bibliotheque.erreur.nom_pris");
    // Rien n'est créé à moitié : le papier en double n'a laissé aucune résolution.
    assert_eq!(ouverte.parc("").unwrap().imprimantes[0].papiers.len(), 1);

    for (cle, attendue) in [
        (
            ouverte.ajouter_imprimante("  ").unwrap_err(),
            "bibliotheque.erreur.nom_vide",
        ),
        (
            ouverte
                .ajouter_papier(bibliotheque::IdImprimante(999), "Luster", "1200 ppp")
                .unwrap_err(),
            "parc.erreur.imprimante_inconnue",
        ),
        (
            ouverte
                .ajouter_resolution(bibliotheque::IdPapier(999), "600 ppp")
                .unwrap_err(),
            "parc.erreur.papier_inconnu",
        ),
        (
            ouverte.detail_reglage(IdCondition(999)).unwrap_err(),
            "parc.erreur.reglage_inconnu",
        ),
    ] {
        assert_eq!(cle, attendue);
        assert!(est_au_catalogue(&cle), "{cle}");
    }
}

/// Un papier sans nom de résolution n'est pas créé seul.
#[test]
fn un_papier_sans_resolution_n_est_pas_cree() {
    let (_dossier, ouverte) = ouverte();
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    assert_eq!(
        ouverte.ajouter_papier(canon.id, "Luster", " ").unwrap_err(),
        "bibliotheque.erreur.nom_vide"
    );
    assert!(ouverte.parc("").unwrap().imprimantes[0].papiers.is_empty());
}

/// Les conditions d'avant le parc restent visibles, à part, avec leurs
/// mesures.
#[test]
fn les_anciennes_conditions_sont_non_classees() {
    let (_dossier, ouverte) = ouverte();
    ouverte.creer_condition("Offset, couché mat").unwrap();
    reglage(&ouverte);
    let parc = ouverte.parc("").unwrap();
    let noms: Vec<_> = parc.non_classees.iter().map(|c| c.nom.as_str()).collect();
    assert_eq!(noms, ["Offset, couché mat"]);
}

/// Le libellé d'un profil suit son état et, tant qu'il reste à vérifier, son
/// origine : calculé par myiro-libre ou importé (rangé tel quel). Le statut
/// enregistré, lui, ne change pas (`non_verifie`).
#[test]
fn le_libelle_d_un_profil_a_verifier_dit_son_origine() {
    use app::parc::{cle_etat, cle_verdict};
    use bibliotheque::EtatVerification as E;
    assert_eq!(
        cle_verdict(E::NonVerifie, "calcul"),
        "parc.verdict.calcule_a_verifier"
    );
    assert_eq!(
        cle_verdict(E::NonVerifie, "profil_existant"),
        "parc.verdict.importe_a_verifier"
    );
    assert_eq!(cle_verdict(E::Accepte, "calcul"), "parc.verdict.verifie");
    assert_eq!(cle_verdict(E::HorsTolerance, "calcul"), "parc.verdict.hors");
    assert_eq!(
        cle_verdict(E::Incomplet, "calcul"),
        "parc.verdict.incomplet"
    );
    // L'historique dit la même chose qu'un verdict tant que le profil reste
    // à vérifier ; sinon son état court.
    assert_eq!(
        cle_etat(E::NonVerifie, "calcul"),
        "parc.verdict.calcule_a_verifier"
    );
    assert_eq!(
        cle_etat(E::NonVerifie, "profil_existant"),
        "parc.verdict.importe_a_verifier"
    );
    assert_eq!(cle_etat(E::Accepte, "calcul"), "parc.etat.accepte");
    assert_eq!(
        cle_etat(E::HorsTolerance, "profil_existant"),
        "parc.etat.hors_tolerance"
    );
    assert_eq!(cle_etat(E::Incomplet, "calcul"), "parc.etat.incomplet");
}

/// Les libellés « à vérifier », en français et en anglais ; plus aucun texte
/// ne dit « non vérifié », jugé trop inquiétant.
#[test]
fn les_libelles_a_verifier_existent_en_deux_langues() {
    use app::textes::{cles, texte, Langue};
    let attendus = [
        (
            "parc.verdict.calcule_a_verifier",
            "Profil calculé, à vérifier",
            "Profile computed, to be verified",
        ),
        (
            "parc.verdict.importe_a_verifier",
            "Profil importé, à vérifier",
            "Profile imported, to be verified",
        ),
    ];
    for (cle, fr, en) in attendus {
        assert_eq!(texte(Langue::Francais, cle), fr);
        assert_eq!(texte(Langue::Anglais, cle), en);
    }
    for cle in cles() {
        let fr = texte(Langue::Francais, cle).to_lowercase();
        let en = texte(Langue::Anglais, cle).to_lowercase();
        assert!(!fr.contains("non vérifié"), "{cle} : {fr}");
        assert!(!fr.contains("pas encore vérifié"), "{cle} : {fr}");
        assert!(!en.contains("not verified"), "{cle} : {en}");
    }
    assert!(texte(Langue::Francais, "profil.fait").contains("à vérifier"));
    assert!(texte(Langue::Anglais, "profil.fait").contains("to be verified"));
}
