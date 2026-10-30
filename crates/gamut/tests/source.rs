//! Profil de l'image (ticket #80) : sRVB par défaut, matrice et courbes,
//! gris, table A2B0. Les profils sont construits ici : aucun profil d'un
//! autre logiciel n'entre dans le dépôt. La concordance avec le sRVB
//! d'ArgyllCMS se vérifie à l'atelier :
//!
//! `cargo test -p gamut --test source -- --ignored --nocapture`
//!
//! Racine d'ArgyllCMS : variable `ARGYLL`, sinon le premier dossier `argyll`
//! qui contient `ref/sRGB.icm` en remontant depuis cette crate.

use std::path::{Path, PathBuf};

use colorimetrie::{delta_e00, lab_vers_srgb, Lab};
use gamut::{profil_essai, profil_essai_xyz, ErreurIcc, ProfilEssai, ProfilIcc, ProfilSource};

fn lab(v: [f64; 3]) -> Lab {
    Lab {
        l: v[0],
        a: v[1],
        b: v[2],
    }
}

#[test]
fn le_blanc_srvb_est_le_blanc_d50() {
    let blanc = ProfilSource::srvb().demande([1.0, 1.0, 1.0]);
    assert!((blanc[0] - 100.0).abs() < 0.01, "{blanc:?}");
    assert!(blanc[1].abs() < 0.01 && blanc[2].abs() < 0.01, "{blanc:?}");
    assert_eq!(ProfilSource::srvb().nom(), "sRVB");
}

#[test]
fn srvb_concorde_avec_colorimetrie() {
    let srvb = ProfilSource::srvb();
    // 1 000 couleurs 8 bits réparties dans le cube : l'écran de
    // `colorimetrie` doit redonner le même octet (écart sous un demi-pas).
    for i in 0..1000u32 {
        let octets = [(i * 37) % 256, (i * 101 + 7) % 256, (i * 211 + 13) % 256];
        let rvb = octets.map(|o| f64::from(o) / 255.0);
        let ecran = lab_vers_srgb(lab(srvb.demande(rvb))).expect("couleur affichable");
        assert_eq!(
            [u32::from(ecran.r), u32::from(ecran.g), u32::from(ecran.b)],
            octets,
            "rvb {rvb:?}"
        );
    }
}

/// Octets d'un profil ICC v4 d'appareil au PCS XYZ.
fn profil(espace: &[u8; 4], balises: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    profil_pcs(espace, b"XYZ ", balises)
}

/// Octets d'un profil ICC v4 d'appareil : en-tête (espace de données, PCS,
/// illuminant D50) et balises dans l'ordre donné.
fn profil_pcs(espace: &[u8; 4], pcs: &[u8; 4], balises: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let debut = 132 + 12 * balises.len();
    let mut table = (balises.len() as u32).to_be_bytes().to_vec();
    let mut corps = Vec::new();
    for (sig, donnees) in balises {
        while !(debut + corps.len()).is_multiple_of(4) {
            corps.push(0);
        }
        table.extend_from_slice(sig);
        table.extend_from_slice(&((debut + corps.len()) as u32).to_be_bytes());
        table.extend_from_slice(&(donnees.len() as u32).to_be_bytes());
        corps.extend_from_slice(donnees);
    }
    let mut octets = vec![0u8; 128];
    octets[0..4].copy_from_slice(&((debut + corps.len()) as u32).to_be_bytes());
    octets[8] = 4;
    octets[12..16].copy_from_slice(b"mntr");
    octets[16..20].copy_from_slice(espace);
    octets[20..24].copy_from_slice(pcs);
    octets[36..40].copy_from_slice(b"acsp");
    for (k, v) in [0.9642, 1.0, 0.8249].iter().enumerate() {
        octets[68 + 4 * k..72 + 4 * k].copy_from_slice(&s15(*v));
    }
    octets.extend_from_slice(&table);
    octets.extend_from_slice(&corps);
    octets
}

fn s15(v: f64) -> [u8; 4] {
    ((v * 65536.0).round() as i32).to_be_bytes()
}

fn balise_xyz(v: [f64; 3]) -> Vec<u8> {
    let mut t = b"XYZ \0\0\0\0".to_vec();
    v.iter().for_each(|x| t.extend_from_slice(&s15(*x)));
    t
}

fn balise_para(genre: u16, p: &[f64]) -> Vec<u8> {
    let mut t = b"para\0\0\0\0".to_vec();
    t.extend_from_slice(&genre.to_be_bytes());
    t.extend_from_slice(&[0, 0]);
    p.iter().for_each(|x| t.extend_from_slice(&s15(*x)));
    t
}

fn balise_desc(texte: &str) -> Vec<u8> {
    let mut t = b"desc\0\0\0\0".to_vec();
    t.extend_from_slice(&(texte.len() as u32 + 1).to_be_bytes());
    t.extend_from_slice(texte.as_bytes());
    t.extend_from_slice(&[0; 80]);
    t
}

fn chroma(lab: [f64; 3]) -> f64 {
    lab[1].hypot(lab[2])
}

#[test]
fn un_profil_display_p3_a_courbes_para_est_lu() {
    // Primaires Display P3 adaptées D50, courbe sRVB en `para` de type 3.
    let courbe = balise_para(3, &[2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045]);
    let octets = profil(
        b"RGB ",
        &[
            (*b"desc", balise_desc("Display P3 d'essai")),
            (*b"rXYZ", balise_xyz([0.5151, 0.2412, -0.0011])),
            (*b"gXYZ", balise_xyz([0.2920, 0.6922, 0.0419])),
            (*b"bXYZ", balise_xyz([0.1571, 0.0666, 0.7841])),
            (*b"rTRC", courbe.clone()),
            (*b"gTRC", courbe.clone()),
            (*b"bTRC", courbe),
        ],
    );
    let p3 = ProfilSource::lire(&octets).expect("profil P3 lisible");
    assert_eq!(p3.nom(), "Display P3 d'essai");
    let rouge_p3 = p3.demande([1.0, 0.0, 0.0]);
    let rouge_srvb = ProfilSource::srvb().demande([1.0, 0.0, 0.0]);
    assert!(
        chroma(rouge_p3) > chroma(rouge_srvb) + 10.0,
        "P3 {rouge_p3:?}, sRVB {rouge_srvb:?}"
    );
    // Même courbe que le sRVB : le blanc reste le blanc D50.
    let blanc = p3.demande([1.0; 3]);
    assert!(
        (blanc[0] - 100.0).abs() < 0.05 && chroma(blanc) < 0.05,
        "{blanc:?}"
    );
}

#[test]
fn un_profil_gris_ktrc_est_lu() {
    // Courbe `curv` d'un seul point : gamma 2,2 en u8Fixed8 (563 / 256).
    let mut courbe = b"curv\0\0\0\0".to_vec();
    courbe.extend_from_slice(&1u32.to_be_bytes());
    courbe.extend_from_slice(&563u16.to_be_bytes());
    let octets = profil(
        b"GRAY",
        &[
            (*b"wtpt", balise_xyz([0.9642, 1.0, 0.8249])),
            (*b"kTRC", courbe),
        ],
    );
    let gris = ProfilSource::lire(&octets).expect("profil gris lisible");
    let milieu = gris.demande([0.5; 3]);
    // Y = 0,5^2,2 ≈ 0,2176, soit L* ≈ 53,8.
    assert!((milieu[0] - 53.8).abs() < 0.2, "{milieu:?}");
    assert!(
        milieu[1].abs() < 0.01 && milieu[2].abs() < 0.01,
        "{milieu:?}"
    );
    assert_eq!(gris.nom(), "");
}

/// L = 20 + 78·(r+g+b)/3, a = 70·(r−g), b = 70·(g−b) : une table d'appareil.
fn appareil(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        20.0 + 78.0 * (r + g + b) / 3.0,
        70.0 * (r - g),
        70.0 * (g - b),
    ]
}

#[test]
fn un_profil_a2b0_mft2_est_lu() {
    let octets = profil_essai(&ProfilEssai {
        grille: 5,
        blanc: [96.0, 0.0, 0.0],
        cible: appareil,
        inverse: None,
        description: "Table d'essai".into(),
        mesures: None,
    });
    let source = ProfilSource::lire(&octets).expect("profil à table lisible");
    let icc = ProfilIcc::lire(&octets).expect("profil d'essai lisible");
    assert_eq!(source.nom(), "Table d'essai");
    // Sur les nœuds de la grille, la table donne la cible.
    for rvb in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.25], [0.5, 0.75, 1.0]] {
        let d = source.demande(rvb);
        let c = appareil(rvb);
        assert!(
            (0..3).all(|k| (d[k] - c[k]).abs() < 0.01),
            "{rvb:?} : {d:?} ≠ {c:?}"
        );
    }
    // Entre les nœuds, la même couleur que la lecture d'un profil de sortie.
    for rvb in [[0.1, 0.2, 0.3], [0.9, 0.35, 0.6]] {
        let d = source.demande(rvb);
        let r = icc.lab_relatif(rvb);
        assert!((0..3).all(|k| (d[k] - r[k]).abs() < 1e-6), "{d:?} ≠ {r:?}");
    }
}

#[test]
fn un_profil_a_table_au_pcs_xyz_est_lu() {
    let octets = profil_essai_xyz(&ProfilEssai {
        grille: 5,
        blanc: [96.0, 0.0, 0.0],
        cible: appareil,
        inverse: None,
        description: "Table XYZ d'essai".into(),
        mesures: None,
    });
    let source = ProfilSource::lire(&octets).expect("profil XYZ lisible");
    for rvb in [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.25],
        [0.5, 0.75, 1.0],
        [1.0; 3],
    ] {
        let d = source.demande(rvb);
        let c = appareil(rvb);
        assert!(
            (0..3).all(|k| (d[k] - c[k]).abs() < 0.1),
            "{rvb:?} : {d:?} ≠ {c:?}"
        );
    }
}

#[test]
fn un_profil_cmjn_embarque_est_refuse() {
    let octets = profil(b"CMYK", &[(*b"desc", balise_desc("CMJN d'essai"))]);
    assert_eq!(
        ProfilSource::lire(&octets).err(),
        Some(ErreurIcc::PasRvb {
            espace: "CMYK".into()
        })
    );
    // Un profil d'un autre espace de données (Lab) : en-tête refusé.
    let lab = profil(b"Lab ", &[]);
    assert!(matches!(
        ProfilSource::lire(&lab),
        Err(ErreurIcc::EnTete(_))
    ));
}

fn racine_argyll() -> PathBuf {
    if let Some(r) = std::env::var_os("ARGYLL") {
        return PathBuf::from(r);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .flat_map(|d| [d.join("argyll"), d.join("../argyll")])
        .find(|a| a.join("ref/sRGB.icm").is_file())
        .expect("ArgyllCMS introuvable : variable ARGYLL ou dossier argyll/ du dépôt")
}

#[test]
#[ignore = "lit ref/sRGB.icm d'ArgyllCMS, absent du dépôt"]
fn srvb_concorde_avec_le_srgb_d_argyll() {
    let octets = std::fs::read(racine_argyll().join("ref/sRGB.icm")).expect("sRGB.icm lisible");
    let argyll = ProfilSource::lire(&octets).expect("sRGB d'ArgyllCMS lisible");
    let srvb = ProfilSource::srvb();
    let mut pire: f64 = 0.0;
    for i in 0..1000u32 {
        let rvb = [(i * 37) % 256, (i * 101 + 7) % 256, (i * 211 + 13) % 256]
            .map(|o| f64::from(o) / 255.0);
        let e = delta_e00(lab(argyll.demande(rvb)), lab(srvb.demande(rvb))).expect("écart");
        pire = pire.max(e);
    }
    println!(
        "{} : écart maximal {pire:.4} ΔE00 avec srvb()",
        argyll.nom()
    );
    assert!(pire <= 0.05, "écart maximal {pire} ΔE00");
}

#[test]
#[ignore = "lit ref/DisplayP3.icm d'ArgyllCMS, absent du dépôt"]
fn le_display_p3_d_argyll_est_lu_comme_tel() {
    let octets =
        std::fs::read(racine_argyll().join("ref/DisplayP3.icm")).expect("DisplayP3.icm lisible");
    let p3 = ProfilSource::lire(&octets).expect("Display P3 d'ArgyllCMS lisible");
    let rouge_p3 = p3.demande([1.0, 0.0, 0.0]);
    let rouge_srvb = ProfilSource::srvb().demande([1.0, 0.0, 0.0]);
    println!("{} : rouge {rouge_p3:?}, sRVB {rouge_srvb:?}", p3.nom());
    assert!(chroma(rouge_p3) > chroma(rouge_srvb) + 10.0);
}

/// Données d'une balise d'un profil, lues dans sa table des balises.
fn balise_de(octets: &[u8], sig: &[u8; 4]) -> Vec<u8> {
    let mot = |i: usize| {
        u32::from_be_bytes([octets[i], octets[i + 1], octets[i + 2], octets[i + 3]]) as usize
    };
    (0..mot(128))
        .map(|k| 132 + 12 * k)
        .find(|&e| &octets[e..e + 4] == sig)
        .map(|e| octets[mot(e + 4)..mot(e + 4) + mot(e + 8)].to_vec())
        .expect("balise présente")
}

/// Table A2B1 `mft2` (PCS Lab) d'un profil d'essai dont la cible est `appareil`.
fn table_appareil() -> Vec<u8> {
    balise_de(
        &profil_essai(&ProfilEssai {
            grille: 5,
            blanc: [96.0, 0.0, 0.0],
            cible: appareil,
            inverse: None,
            description: "Table d'essai".into(),
            mesures: None,
        }),
        b"A2B1",
    )
}

fn balises_p3() -> Vec<([u8; 4], Vec<u8>)> {
    let courbe = balise_para(3, &[2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045]);
    vec![
        (*b"rXYZ", balise_xyz([0.5151, 0.2412, -0.0011])),
        (*b"gXYZ", balise_xyz([0.2920, 0.6922, 0.0419])),
        (*b"bXYZ", balise_xyz([0.1571, 0.0666, 0.7841])),
        (*b"rTRC", courbe.clone()),
        (*b"gTRC", courbe.clone()),
        (*b"bTRC", courbe),
    ]
}

fn proche(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < tolerance)
}

#[test]
fn une_table_a2b_passe_avant_la_matrice() {
    let mut balises = balises_p3();
    balises.push((*b"A2B1", table_appareil()));
    let source = ProfilSource::lire(&profil_pcs(b"RGB ", b"Lab ", &balises)).expect("lisible");
    let rouge = source.demande([1.0, 0.0, 0.0]);
    assert!(proche(rouge, appareil([1.0, 0.0, 0.0]), 0.01), "{rouge:?}");
    assert!(source.ecartees().is_empty());
}

#[test]
fn une_a2b1_illisible_fait_passer_a_l_a2b0_en_le_disant() {
    let mab = b"mAB       ".to_vec();
    let mut balises = balises_p3();
    balises.push((*b"A2B1", mab.clone()));
    balises.push((*b"A2B0", table_appareil()));
    let source = ProfilSource::lire(&profil_pcs(b"RGB ", b"Lab ", &balises)).expect("lisible");
    let vert = source.demande([0.0, 1.0, 0.0]);
    assert!(proche(vert, appareil([0.0, 1.0, 0.0]), 0.01), "{vert:?}");
    assert_eq!(
        source.ecartees(),
        &[(
            "A2B1".to_string(),
            ErreurIcc::StructureNonPriseEnCharge {
                structure: "mAB".into()
            }
        )]
    );
    // Sans autre conversion, la raison de la table illisible est rendue.
    let seule = profil_pcs(b"RGB ", b"Lab ", &[(*b"A2B1", mab)]);
    assert_eq!(
        ProfilSource::lire(&seule).err(),
        Some(ErreurIcc::StructureNonPriseEnCharge {
            structure: "mAB".into()
        })
    );
}

#[test]
fn un_profil_gris_au_pcs_lab_donne_l_par_sa_courbe() {
    // ICC.1 (annexe F.2) : au PCS Lab, la courbe grayTRC donne L* (0–1
    // pour 0–100), a* = b* = 0 ; au PCS XYZ, elle donne Y.
    let identite = balise_para(0, &[1.0]);
    let lab = ProfilSource::lire(&profil_pcs(
        b"GRAY",
        b"Lab ",
        &[(*b"kTRC", identite.clone())],
    ))
    .expect("gris Lab lisible");
    assert!(proche(lab.demande([0.5; 3]), [50.0, 0.0, 0.0], 0.01));
    let xyz = ProfilSource::lire(&profil(b"GRAY", &[(*b"kTRC", identite)])).expect("gris XYZ");
    assert!(proche(xyz.demande([0.5; 3]), [l_de_y(0.5), 0.0, 0.0], 0.01));
}

/// L* d'une luminance relative Y (CIE 1976).
fn l_de_y(y: f64) -> f64 {
    if y > 216.0 / 24389.0 {
        116.0 * y.cbrt() - 16.0
    } else {
        24389.0 / 27.0 * y
    }
}

fn balise_curv(points: &[u16]) -> Vec<u8> {
    let mut t = b"curv    ".to_vec();
    t.extend_from_slice(&(points.len() as u32).to_be_bytes());
    points
        .iter()
        .for_each(|p| t.extend_from_slice(&p.to_be_bytes()));
    t
}

#[test]
fn les_courbes_curv_et_para_suivent_la_norme() {
    // (courbe, entrée, Y attendu), valeurs posées à la main depuis ICC.1.
    let cas: Vec<(Vec<u8>, f64, f64)> = vec![
        (balise_curv(&[]), 0.5, 0.5),
        (balise_curv(&[0, 16384, 65535]), 0.5, 16384.0 / 65535.0),
        (
            balise_curv(&[0, 16384, 65535]),
            0.75,
            (16384.0 / 65535.0 + 1.0) / 2.0,
        ),
        (balise_para(0, &[2.0]), 0.5, 0.25),
        (balise_para(1, &[1.0, 2.0, -1.0]), 0.75, 0.5),
        (balise_para(1, &[1.0, 2.0, -1.0]), 0.25, 0.0),
        (balise_para(2, &[1.0, 2.0, -1.0, 0.1]), 0.25, 0.1),
        (balise_para(2, &[1.0, 2.0, -1.0, 0.1]), 0.75, 0.6),
        (
            balise_para(4, &[1.0, 1.0, 0.0, 0.5, 0.5, 0.1, 0.05]),
            0.25,
            0.175,
        ),
        (
            balise_para(4, &[1.0, 1.0, 0.0, 0.5, 0.5, 0.1, 0.05]),
            0.75,
            0.85,
        ),
    ];
    for (i, (courbe, x, y)) in cas.into_iter().enumerate() {
        let gris = ProfilSource::lire(&profil(b"GRAY", &[(*b"kTRC", courbe)])).expect("lisible");
        let l = gris.demande([x; 3])[0];
        assert!(
            (l - l_de_y(y)).abs() < 0.01,
            "cas {i} : L* {l}, attendu {}",
            l_de_y(y)
        );
    }
}

#[test]
fn un_profil_tronque_est_refuse_en_le_disant() {
    let entier = profil(b"GRAY", &[(*b"kTRC", balise_curv(&[0, 16384, 65535]))]);
    let coupe = &entier[..entier.len() - 2];
    assert_eq!(
        ProfilSource::lire(coupe).err(),
        Some(ErreurIcc::Tronque {
            balise: "kTRC".into()
        })
    );
    // Une courbe qui annonce plus de points qu'elle n'en a.
    let mut menteuse = balise_curv(&[0, 65535]);
    menteuse[8..12].copy_from_slice(&100u32.to_be_bytes());
    assert_eq!(
        ProfilSource::lire(&profil(b"GRAY", &[(*b"kTRC", menteuse)])).err(),
        Some(ErreurIcc::Tronque {
            balise: "kTRC".into()
        })
    );
    assert!(matches!(
        ProfilSource::lire(&entier[..100]),
        Err(ErreurIcc::EnTete(_))
    ));
}

#[test]
fn les_refus_d_un_profil_d_image_ont_leurs_propres_raisons() {
    let refus = [
        ErreurIcc::EnTete("signature acsp absente".into()),
        ErreurIcc::PasRvb {
            espace: "CMYK".into(),
        },
        ErreurIcc::SansTable,
        ErreurIcc::StructureNonPriseEnCharge {
            structure: "mAB".into(),
        },
        ErreurIcc::Tronque {
            balise: "kTRC".into(),
        },
    ];
    let cles: Vec<_> = refus.iter().map(ProfilSource::cle_refus).collect();
    assert_eq!(
        cles,
        [
            "comparer.image.refus.en_tete",
            "comparer.image.refus.pas_rvb",
            "comparer.image.refus.sans_conversion",
            "comparer.image.refus.structure",
            "comparer.image.refus.tronque",
        ]
    );
}
