//! Profils ICC d'une condition d'impression, vus par l'API publique de la
//! bibliothèque. Les profils sont des en-têtes ICC fictifs, sans étiquette.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bibliotheque::{
    espace_icc, nom_profil_propose, profil_ancien, Bibliotheque, ErreurBibliotheque, EspaceProfil,
    EtatVerification, IdCondition, IdProfil, NouveauProfil,
};
use tempfile::TempDir;

/// En-tête ICC minimal de 132 octets : taille, espace de données, signature
/// `acsp`, aucune étiquette.
fn icc(espace: &[u8; 4]) -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(espace);
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

/// 2026-10-20T00:00:00Z.
fn le_20_octobre_2026() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_454_400)
}

/// Une bibliothèque avec une condition rangée sous Canon PRO-4100 →
/// HMPhoto Luster 290 → 1200 ppp.
fn condition_rangee() -> (TempDir, Bibliotheque, IdCondition) {
    let dossier = tempfile::tempdir().unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
    let luster = b.creer_papier(canon.id, "HMPhoto Luster 290").unwrap();
    let condition = b.creer_reglage(luster.id, "1200 ppp").unwrap();
    (dossier, b, condition.id)
}

fn nouveau<'a>(
    condition: IdCondition,
    nom: &'a str,
    icc: &'a [u8],
    horodatage: &'a str,
) -> NouveauProfil<'a> {
    NouveauProfil {
        condition,
        nom,
        icc,
        horodatage,
        parametres: "colprof -v -qm -aX profil",
        mesures: &[],
    }
}

#[test]
fn un_nouveau_profil_devient_actif_et_l_ancien_passe_a_l_historique() {
    let (_d, b, c) = condition_rangee();
    let rvb = icc(b"RGB ");
    let p1 = b
        .enregistrer_profil(&nouveau(c, "v1", &rvb, "2026-03-03T10:00:00Z"))
        .unwrap();
    let p2 = b
        .enregistrer_profil(&nouveau(c, "v2", &rvb, "2026-10-20T09:00:00Z"))
        .unwrap();

    let actif = b.profil_actif(c).unwrap().unwrap();
    assert_eq!(actif.id, p2);
    assert_eq!(actif.nom, "v2");
    assert!(actif.actif);
    assert_eq!(actif.espace, EspaceProfil::Rvb);
    assert_eq!(actif.etat, EtatVerification::NonVerifie);
    assert_eq!(actif.horodatage, "2026-10-20T09:00:00Z");

    let historique = b.historique_profils(c).unwrap();
    let ids: Vec<_> = historique.iter().map(|p| p.id).collect();
    assert_eq!(ids, [p2, p1]);
    assert!(!historique[1].actif);

    assert_eq!(b.fichier_profil(p1).unwrap(), rvb);
    let empreinte = &historique[1].empreinte;
    assert_eq!(empreinte.len(), 64);
    assert!(empreinte
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    assert_eq!(
        b.fichier_profil(IdProfil(999)),
        Err(ErreurBibliotheque::ProfilInconnu(IdProfil(999)))
    );
}

#[test]
fn un_profil_se_retrouve_a_la_reouverture() {
    let dossier = tempfile::tempdir().unwrap();
    let c = {
        let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
        let canon = b.creer_imprimante("Canon PRO-4100").unwrap();
        let luster = b.creer_papier(canon.id, "Luster").unwrap();
        let c = b.creer_reglage(luster.id, "600 ppp").unwrap().id;
        b.enregistrer_profil(&nouveau(c, "v1", &icc(b"CMYK"), "2026-10-20T09:00:00Z"))
            .unwrap();
        c
    };
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    assert_eq!(b.profil_actif(c).unwrap().unwrap().nom, "v1");
}

#[test]
fn un_fichier_qui_n_est_pas_un_profil_rvb_ou_cmjn_est_refuse_sans_rien_changer() {
    let (_d, b, c) = condition_rangee();
    let p1 = b
        .enregistrer_profil(&nouveau(c, "v1", &icc(b"RGB "), "2026-03-03T10:00:00Z"))
        .unwrap();

    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.resize(400, b' ');
    let mut tronque = icc(b"RGB ");
    tronque[0..4].copy_from_slice(&2000u32.to_be_bytes());
    let mut sans_signature = icc(b"RGB ");
    sans_signature[36..40].copy_from_slice(b"xxxx");
    for (fichier, attendu) in [
        (pdf, "signature acsp absente"),
        (tronque, "taille annoncée 2000, fichier de 132 octets"),
        (sans_signature, "signature acsp absente"),
        (icc(b"Lab "), "espace de données Lab non pris en charge"),
        (icc(b"GRAY"), "espace de données GRAY non pris en charge"),
        (vec![0u8; 20], "moins de 132 octets"),
    ] {
        match b.enregistrer_profil(&nouveau(c, "mauvais", &fichier, "2026-10-20T09:00:00Z")) {
            Err(ErreurBibliotheque::ProfilIllisible(detail)) => {
                assert!(detail.contains(attendu), "{detail} / {attendu}")
            }
            autre => panic!("{autre:?} pour {attendu}"),
        }
    }
    assert_eq!(b.profil_actif(c).unwrap().unwrap().id, p1);
    assert_eq!(b.historique_profils(c).unwrap().len(), 1);
}

#[test]
fn un_profil_sans_nom_ou_sans_condition_est_refuse() {
    let (_d, b, c) = condition_rangee();
    let rvb = icc(b"RGB ");
    assert_eq!(
        b.enregistrer_profil(&nouveau(c, "  ", &rvb, "2026-10-20T09:00:00Z")),
        Err(ErreurBibliotheque::NomVide)
    );
    assert_eq!(
        b.enregistrer_profil(&nouveau(
            IdCondition(999),
            "v1",
            &rvb,
            "2026-10-20T09:00:00Z"
        )),
        Err(ErreurBibliotheque::ConditionInconnue(IdCondition(999)))
    );
    assert!(b.historique_profils(c).unwrap().is_empty());
    assert_eq!(b.profil_actif(c).unwrap(), None);
}

#[test]
fn un_profil_a_la_date_illisible_est_refuse_sans_rien_ecrire() {
    let (_d, b, c) = condition_rangee();
    let p1 = b
        .enregistrer_profil(&nouveau(c, "v1", &icc(b"RGB "), "2026-03-03T10:00:00Z"))
        .unwrap();
    for date in ["hier", "2026-13-45T00:00:00Z", "", "2026-10-20"] {
        match b.enregistrer_profil(&nouveau(c, "v2", &icc(b"RGB "), date)) {
            Err(ErreurBibliotheque::HorodatageInvalide(texte)) => {
                assert_eq!(texte, date);
                let message = ErreurBibliotheque::HorodatageInvalide(texte).to_string();
                assert!(message.contains("date du profil"), "{message}");
            }
            autre => panic!("{autre:?} pour « {date} »"),
        }
    }
    assert_eq!(b.profil_actif(c).unwrap().unwrap().id, p1);
    assert_eq!(b.historique_profils(c).unwrap().len(), 1);
}

#[test]
fn le_cmjn_est_reconnu() {
    assert_eq!(espace_icc(&icc(b"CMYK")), Ok(EspaceProfil::Cmjn));
    assert_eq!(espace_icc(&icc(b"RGB ")), Ok(EspaceProfil::Rvb));
    let (_d, b, c) = condition_rangee();
    b.enregistrer_profil(&nouveau(c, "cmjn", &icc(b"CMYK"), "2026-10-20T09:00:00Z"))
        .unwrap();
    assert_eq!(
        b.profil_actif(c).unwrap().unwrap().espace,
        EspaceProfil::Cmjn
    );
}

#[test]
fn la_verification_change_l_etat() {
    let (_d, b, c) = condition_rangee();
    let p = b
        .enregistrer_profil(&nouveau(c, "v1", &icc(b"RGB "), "2026-10-20T09:00:00Z"))
        .unwrap();
    for etat in [
        EtatVerification::Accepte,
        EtatVerification::HorsTolerance,
        EtatVerification::Incomplet,
    ] {
        b.marquer_verification(p, etat).unwrap();
        assert_eq!(b.profil_actif(c).unwrap().unwrap().etat, etat);
    }
    assert_eq!(
        b.marquer_verification(IdProfil(999), EtatVerification::Accepte),
        Err(ErreurBibliotheque::ProfilInconnu(IdProfil(999)))
    );
}

#[test]
fn le_nom_propose_est_un_nom_de_fichier_windows_valide() {
    assert_eq!(
        nom_profil_propose(
            "Canon PRO-4100",
            "HMPhoto Luster 290",
            "1200 ppp",
            "2026-10-20"
        ),
        "Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-20.icc"
    );
    assert_eq!(
        nom_profil_propose(
            "Epson SC-P5300",
            "Canson « Baryta » Mat/Satin",
            "720 ppp",
            "2026-10-20"
        ),
        "Epson-SC-P5300_Canson-Baryta-MatSatin_720-ppp_2026-10-20.icc"
    );
    let n = nom_profil_propose(
        "Canon iPF5100",
        "Hahnemühle Photo Rag 308",
        "600 ppp",
        "2026-10-20",
    );
    assert!(n.contains("Hahnemühle"));
    assert!(!n
        .chars()
        .any(|c| "<>:\"/\\|?*".contains(c) || c.is_control()));
    let n = nom_profil_propose("L'imprimante \"A\"", " Mat\t: ?* ", "a|b<c>", "2026-10-20");
    assert_eq!(n, "L'imprimante-A_Mat_abc_2026-10-20.icc");
}

#[test]
fn un_profil_de_plus_d_un_an_est_ancien() {
    let maintenant = le_20_octobre_2026();
    assert!(profil_ancien("2025-10-19T23:59:59Z", maintenant));
    assert!(!profil_ancien("2025-10-21T00:00:00Z", maintenant));
    assert!(!profil_ancien("2026-10-20T00:00:00Z", maintenant));

    let (_d, b, c) = condition_rangee();
    let reglage = |b: &Bibliotheque| {
        b.parc("", maintenant).unwrap().imprimantes[0].papiers[0].reglages[0].clone()
    };
    assert_eq!(reglage(&b).profil, None);
    assert!(!reglage(&b).profil_ancien);

    b.enregistrer_profil(&nouveau(c, "vieux", &icc(b"RGB "), "2025-06-01T08:00:00Z"))
        .unwrap();
    let r = reglage(&b);
    assert_eq!(r.profil.map(|p| p.nom), Some("vieux".to_string()));
    assert!(r.profil_ancien);

    b.enregistrer_profil(&nouveau(c, "neuf", &icc(b"RGB "), "2026-10-01T08:00:00Z"))
        .unwrap();
    let r = reglage(&b);
    assert_eq!(r.profil.map(|p| p.nom), Some("neuf".to_string()));
    assert!(!r.profil_ancien);
}
