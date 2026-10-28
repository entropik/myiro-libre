//! Lecture d'un profil ICC RVB et colorimétrie absolue, sur des profils
//! d'essai synthétiques (aucun profil d'un autre logiciel dans le dépôt).

use gamut::{profil_essai, ErreurIcc, ProfilEssai, ProfilIcc};

/// Profil d'essai lineaire : L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

const D50_LAB: [f64; 3] = [100.0, 0.0, 0.0];

fn essai(blanc: [f64; 3], mesures: Option<&str>) -> ProfilEssai {
    ProfilEssai {
        grille: 9,
        blanc,
        cible: lineaire,
        description: "Profil d'essai lineaire".into(),
        mesures: mesures.map(String::from),
    }
}

#[test]
fn un_profil_d_essai_se_relit() {
    let octets = profil_essai(&essai(D50_LAB, None));
    let profil = ProfilIcc::lire(&octets).expect("profil d'essai lisible");
    // `profil_essai` écrit un profil ICC v2.4.
    assert_eq!(profil.version(), "2.4");
    assert_eq!(profil.grille(), 9);
    assert_eq!(
        profil.description().as_deref(),
        Some("Profil d'essai lineaire")
    );
    assert_eq!(
        bibliotheque::espace_icc(&octets),
        Ok(bibliotheque::EspaceProfil::Rvb)
    );
}

fn proche(obtenu: [f64; 3], attendu: [f64; 3], tolerance: f64) {
    for k in 0..3 {
        assert!(
            (obtenu[k] - attendu[k]).abs() <= tolerance,
            "obtenu {obtenu:?}, attendu {attendu:?} à {tolerance} près"
        );
    }
}

#[test]
fn aux_noeuds_la_table_est_rendue_exactement() {
    let profil = ProfilIcc::lire(&profil_essai(&essai(D50_LAB, None))).expect("lisible");
    for r in 0..9 {
        for g in 0..9 {
            for b in 0..9 {
                let rvb = [f64::from(r) / 8.0, f64::from(g) / 8.0, f64::from(b) / 8.0];
                proche(profil.lab_absolu(rvb), lineaire(rvb), 0.01);
            }
        }
    }
}

#[test]
fn entre_les_noeuds_une_fonction_lineaire_est_reproduite() {
    let profil = ProfilIcc::lire(&profil_essai(&essai(D50_LAB, None))).expect("lisible");
    let rvb = [0.3, 0.55, 0.8];
    proche(profil.lab_absolu(rvb), lineaire(rvb), 0.01);
}

fn blanc_parfait(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    let moyenne = (r + g + b) / 3.0;
    [100.0 * moyenne, 40.0 * (r - g), 40.0 * (g - b)]
}

#[test]
fn le_blanc_absolu_est_le_blanc_du_papier() {
    let papier = [95.0, -0.5, 2.5];
    let octets = profil_essai(&ProfilEssai {
        cible: blanc_parfait,
        ..essai(papier, None)
    });
    let profil = ProfilIcc::lire(&octets).expect("lisible");
    proche(profil.lab_absolu([1.0, 1.0, 1.0]), papier, 0.02);
    proche(profil.blanc_support(), papier, 0.02);
}

/// Position et taille d'une balise dans la table des balises du profil.
fn balise(octets: &[u8], sig: &[u8; 4]) -> (usize, usize) {
    let mot =
        |p: usize| u32::from_be_bytes(octets[p..p + 4].try_into().expect("4 octets")) as usize;
    (0..mot(128))
        .map(|i| 132 + 12 * i)
        .find(|&e| &octets[e..e + 4] == sig)
        .map(|e| (mot(e + 4), mot(e + 8)))
        .expect("balise présente")
}

/// Position de l'entrée d'une balise dans la table des balises.
fn entree(octets: &[u8], sig: &[u8; 4]) -> usize {
    let n = u32::from_be_bytes(octets[128..132].try_into().expect("4 octets")) as usize;
    (0..n)
        .map(|i| 132 + 12 * i)
        .find(|&e| &octets[e..e + 4] == sig)
        .expect("balise présente")
}

#[test]
fn une_table_mab_est_refusee_en_la_nommant() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    let (position, _) = balise(&octets, b"A2B1");
    octets[position..position + 4].copy_from_slice(b"mAB ");
    let erreur = ProfilIcc::lire(&octets).expect_err("structure refusée");
    assert_eq!(
        erreur,
        ErreurIcc::StructureNonPriseEnCharge {
            structure: "mAB".into()
        }
    );
    assert_eq!(erreur.cle(), "comparer.refus.structure");
}

#[test]
fn un_profil_sans_blanc_du_support_est_refuse() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    let e = entree(&octets, b"wtpt");
    octets[e..e + 4].copy_from_slice(b"xxxx");
    let erreur = ProfilIcc::lire(&octets).expect_err("sans blanc");
    assert_eq!(erreur, ErreurIcc::SansBlanc);
    assert_eq!(erreur.cle(), "comparer.refus.sans_blanc");
}

#[test]
fn un_blanc_du_support_d_un_autre_type_est_refuse() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    let (position, _) = balise(&octets, b"wtpt");
    octets[position..position + 4].copy_from_slice(b"text");
    let erreur = ProfilIcc::lire(&octets).expect_err("blanc illisible");
    assert_eq!(erreur, ErreurIcc::SansBlanc);
    assert_eq!(erreur.cle(), "comparer.refus.sans_blanc");
}

#[test]
fn un_profil_cmjn_est_refuse() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    octets[16..20].copy_from_slice(b"CMYK");
    let erreur = ProfilIcc::lire(&octets).expect_err("CMJN refusé");
    assert_eq!(
        erreur,
        ErreurIcc::PasRvb {
            espace: "CMYK".into()
        }
    );
    assert_eq!(erreur.cle(), "comparer.refus.pas_rvb");
}

#[test]
fn une_balise_qui_sort_du_fichier_est_refusee() {
    let octets = profil_essai(&essai(D50_LAB, None));
    let (position, taille) = balise(&octets, b"A2B1");
    let erreur = ProfilIcc::lire(&octets[..position + taille / 2]).expect_err("tronqué");
    assert_eq!(
        erreur,
        ErreurIcc::Tronque {
            balise: "A2B1".into()
        }
    );
    assert_eq!(erreur.cle(), "comparer.refus.tronque");
}

#[test]
fn un_profil_sans_table_a2b1_est_refuse() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    let e = entree(&octets, b"A2B1");
    octets[e..e + 4].copy_from_slice(b"xxxx");
    let erreur = ProfilIcc::lire(&octets).expect_err("sans table");
    assert_eq!(erreur, ErreurIcc::SansTable);
    assert_eq!(erreur.cle(), "comparer.refus.sans_table");
}

#[test]
fn un_en_tete_sans_signature_est_refuse() {
    let mut octets = profil_essai(&essai(D50_LAB, None));
    octets[36..40].copy_from_slice(b"xxxx");
    let erreur = ProfilIcc::lire(&octets).expect_err("en-tête refusé");
    assert!(matches!(erreur, ErreurIcc::EnTete(_)));
    assert_eq!(erreur.cle(), "comparer.refus.en_tete");
}

#[test]
fn les_mesures_embarquees_sont_rendues_telles_quelles() {
    let texte = "CTI3\n\nDESCRIPTOR \"Mesures d'essai\"\nNUMBER_OF_FIELDS 4\n";
    let profil = ProfilIcc::lire(&profil_essai(&essai(D50_LAB, Some(texte)))).expect("lisible");
    assert_eq!(profil.mesures_embarquees(), Some(texte.as_bytes()));
    let sans = ProfilIcc::lire(&profil_essai(&essai(D50_LAB, None))).expect("lisible");
    assert_eq!(sans.mesures_embarquees(), None);
}
