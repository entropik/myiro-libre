//! Table inverse B2A1 et aller-retour colorimétrique relatif (ticket #80), sur
//! des profils d'essai synthétiques. La concordance avec `xicclu` se vérifie
//! sur des profils calculés par ArgyllCMS, gardés à l'atelier :
//!
//! `PROFILS_COMPARAISON=<dossier> cargo test -p gamut --test inverse -- --ignored --nocapture`
//!
//! Le dossier contient `libre.icc` et `mt.icc` (ceux de `tests/reference.rs`).
//! Racine d'ArgyllCMS : variable `ARGYLL`, sinon le premier dossier `argyll`
//! qui contient `bin/xicclu` en remontant depuis cette crate.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use colorimetrie::{delta_e00, Lab};
use gamut::{profil_essai, profil_essai_xyz, ErreurIcc, ProfilEssai, ProfilIcc};

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Inverse exact de `lineaire`, borné au cube RVB.
fn lineaire_inverse(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

/// Comme `lineaire_inverse`, chroma d'abord ramené à 40 au plus.
fn inverse_chroma_40(lab: [f64; 3]) -> [f64; 3] {
    let c = lab[1].hypot(lab[2]);
    let k = if c > 40.0 { 40.0 / c } else { 1.0 };
    lineaire_inverse([lab[0], lab[1] * k, lab[2] * k])
}

fn essai(inverse: Option<fn([f64; 3]) -> [f64; 3]>) -> ProfilEssai {
    ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible: lineaire,
        inverse,
        description: "Profil d'essai inversible".into(),
        mesures: None,
    }
}

fn lire(p: &ProfilEssai) -> ProfilIcc {
    ProfilIcc::lire(&profil_essai(p)).expect("profil d'essai lisible")
}

fn ecart(a: [f64; 3], b: [f64; 3]) -> f64 {
    let lab = |v: [f64; 3]| Lab {
        l: v[0],
        a: v[1],
        b: v[2],
    };
    delta_e00(lab(a), lab(b)).expect("écart calculable")
}

/// Suite pseudo-aléatoire reproductible dans [0, 1).
fn tirages(n: usize) -> Vec<f64> {
    let mut etat: u64 = 0x2545_F491_4F6C_DD1D;
    (0..n)
        .map(|_| {
            etat ^= etat << 13;
            etat ^= etat >> 7;
            etat ^= etat << 17;
            (etat >> 11) as f64 / (1u64 << 53) as f64
        })
        .collect()
}

#[test]
fn l_aller_retour_d_une_couleur_du_gamut_rend_la_meme_couleur() {
    let profil = lire(&essai(Some(lineaire_inverse)));
    assert_eq!(profil.inverse(), Ok(()));
    let t = tirages(150);
    for rvb in t.as_chunks::<3>().0 {
        // Loin des bords du cube : chaque maille de B2A1 reste dans le gamut.
        let rvb = rvb.map(|x| 0.2 + 0.6 * x);
        let lab = profil.lab_relatif(rvb);
        let reproduite = profil.reproduire(lab).expect("table inverse lue");
        assert!(
            ecart(lab, reproduite) < 0.05,
            "{lab:?} reproduite en {reproduite:?}"
        );
    }
}

#[test]
fn une_couleur_hors_gamut_revient_sur_la_limite() {
    let profil = lire(&essai(Some(inverse_chroma_40)));
    let lab = [50.0, 70.0, 0.0];
    let reproduite = profil.reproduire(lab).expect("table inverse lue");
    for (k, attendu) in [50.0, 40.0, 0.0].into_iter().enumerate() {
        assert!(
            (reproduite[k] - attendu).abs() < 1.0,
            "reproduite {reproduite:?}, attendu ≈ [50, 40, 0]"
        );
    }
    // ΔE00 resserre les écarts de chroma : 30 unités de C* à C* ≈ 55 font
    // environ 8,6, bien au-dessus du seuil d'alerte de 2,0.
    assert!(
        ecart(lab, reproduite) > 5.0,
        "écart {}",
        ecart(lab, reproduite)
    );
}

#[test]
fn un_profil_sans_b2a1_se_lit_mais_ne_reproduit_pas() {
    let profil = lire(&essai(None));
    assert_eq!(
        profil.reproduire([50.0, 0.0, 0.0]),
        Err(ErreurIcc::SansTableInverse)
    );
    assert!(profil.inverse().is_err());
    assert_eq!(
        ErreurIcc::SansTableInverse.cle(),
        "comparer.refus.sans_table_inverse"
    );
}

/// Position des données d'une balise.
fn position(octets: &[u8], sig: &[u8; 4]) -> usize {
    let mot =
        |p: usize| u32::from_be_bytes(octets[p..p + 4].try_into().expect("4 octets")) as usize;
    (0..mot(128))
        .map(|i| 132 + 12 * i)
        .find(|&e| &octets[e..e + 4] == sig)
        .map(|e| mot(e + 4))
        .expect("balise présente")
}

#[test]
fn un_b2a1_mab_est_dit_non_pris_en_charge() {
    let mut octets = profil_essai(&essai(Some(lineaire_inverse)));
    let p = position(&octets, b"B2A1");
    octets[p..p + 4].copy_from_slice(b"mAB ");
    let profil = ProfilIcc::lire(&octets).expect("A2B1 toujours lisible");
    let attendu = ErreurIcc::StructureNonPriseEnCharge {
        structure: "mAB".into(),
    };
    assert_eq!(profil.inverse(), Err(attendu.clone()));
    assert_eq!(profil.reproduire([50.0, 0.0, 0.0]), Err(attendu));
}

#[test]
fn un_b2a1_tronque_n_empeche_pas_la_lecture() {
    let octets = profil_essai(&essai(Some(lineaire_inverse)));
    let p = position(&octets, b"B2A1");
    // B2A1 est la dernière balise : le fichier s'arrête au milieu.
    let profil = ProfilIcc::lire(&octets[..p + 100]).expect("A2B1 toujours lisible");
    assert_eq!(
        profil.inverse(),
        Err(ErreurIcc::Tronque {
            balise: "B2A1".into()
        })
    );
}

#[test]
fn lab_relatif_et_absolu_valent_les_valeurs_calculees_a_la_main() {
    // Papier de blanc L* = 90, a* = b* = 0 : en XYZ, chaque composante est
    // multipliée par k = Y(90) / Y(100), et la racine cubique de Lab fait de
    // k^(1/3) = (90 + 16) / 116. D'où L*abs = (L* + 16)·106/116 − 16 et
    // a*abs = a*·106/116, b*abs = b*·106/116 (hors des très sombres).
    // RVB (0,3 ; 0,55 ; 0,8) par `lineaire` : L* = 25 + 75 × 1,65 / 3 = 66,25,
    // a* = 60 × (0,3 − 0,55) = −15, b* = 60 × (0,55 − 0,8) = −15.
    let rvb = [0.3, 0.55, 0.8];
    let relatif = [66.25, -15.0, -15.0];
    let k = 106.0 / 116.0;
    let absolu = [(66.25 + 16.0) * k - 16.0, -15.0 * k, -15.0 * k];
    let papier = ProfilEssai {
        blanc: [90.0, 0.0, 0.0],
        ..essai(None)
    };
    for (pcs, profil) in [
        ("Lab", ProfilIcc::lire(&profil_essai(&papier))),
        ("XYZ", ProfilIcc::lire(&profil_essai_xyz(&papier))),
    ] {
        let profil = profil.expect("profil d'essai lisible");
        proche(pcs, profil.lab_relatif(rvb), relatif, 0.02);
        proche(pcs, profil.relatif_vers_absolu(relatif), absolu, 0.02);
        proche(pcs, profil.lab_absolu(rvb), absolu, 0.03);
    }
}

fn proche(nom: &str, obtenu: [f64; 3], attendu: [f64; 3], tolerance: f64) {
    for k in 0..3 {
        assert!(
            (obtenu[k] - attendu[k]).abs() <= tolerance,
            "{nom} : obtenu {obtenu:?}, attendu {attendu:?} à {tolerance} près"
        );
    }
}

#[test]
fn au_pcs_xyz_l_aller_retour_passe_par_la_matrice_puis_les_courbes() {
    // B2A1 du profil d'essai XYZ : matrice diagonale (0,75 ; 0,875 ; 0,625),
    // puis courbes d'entrée non linéaires. Sans la matrice, ou les courbes
    // avant la matrice, l'aller-retour s'écarte de plusieurs ΔE00.
    let profil =
        ProfilIcc::lire(&profil_essai_xyz(&essai(Some(lineaire_inverse)))).expect("lisible");
    assert_eq!(profil.inverse(), Ok(()));
    let t = tirages(150);
    for rvb in t.as_chunks::<3>().0 {
        let rvb = rvb.map(|x| 0.3 + 0.4 * x);
        proche("A2B1", profil.lab_relatif(rvb), lineaire(rvb), 0.02);
        let lab = profil.lab_relatif(rvb);
        let reproduite = profil.reproduire(lab).expect("table inverse lue");
        assert!(
            ecart(lab, reproduite) < 0.05,
            "{lab:?} reproduite en {reproduite:?}"
        );
    }
}

fn racine_argyll() -> PathBuf {
    if let Some(r) = std::env::var_os("ARGYLL") {
        return PathBuf::from(r);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .flat_map(|d| [d.join("argyll"), d.join("../argyll")])
        .find(|a| a.join("bin/xicclu.exe").is_file() || a.join("bin/xicclu").is_file())
        .expect("ArgyllCMS introuvable : variable ARGYLL ou dossier argyll/ du dépôt")
}

/// Passe des lignes de trois nombres par `xicclu` et relit ses sorties.
fn xicclu(profil: &Path, fonction: &str, entrees: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut enfant = Command::new(racine_argyll().join("bin/xicclu"))
        .args(["-v0", fonction, "-ir", "-pl"])
        .arg(profil)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("xicclu se lance");
    let mut texte = String::new();
    for v in entrees {
        texte.push_str(&format!("{} {} {}\n", v[0], v[1], v[2]));
    }
    enfant
        .stdin
        .take()
        .expect("entrée de xicclu")
        .write_all(texte.as_bytes())
        .expect("écriture vers xicclu");
    let sortie = enfant.wait_with_output().expect("xicclu se termine");
    String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let n: Vec<f64> = l
                .split_whitespace()
                .take(3)
                .map(|x| x.parse().expect("nombre"))
                .collect();
            [n[0], n[1], n[2]]
        })
        .collect()
}

#[test]
#[ignore = "profils de l'atelier et ArgyllCMS, variable PROFILS_COMPARAISON"]
fn l_aller_retour_concorde_avec_xicclu() {
    let dossier = PathBuf::from(
        std::env::var_os("PROFILS_COMPARAISON")
            .expect("PROFILS_COMPARAISON : dossier des profils de l'essai"),
    );
    let t = tirages(300);
    let couleurs: Vec<[f64; 3]> = t
        .as_chunks::<3>()
        .0
        .iter()
        .map(|x| {
            [
                5.0 + 90.0 * x[0],
                -90.0 + 180.0 * x[1],
                -90.0 + 180.0 * x[2],
            ]
        })
        .collect();
    for nom in ["libre.icc", "mt.icc"] {
        let chemin = dossier.join(nom);
        let octets = std::fs::read(&chemin).unwrap_or_else(|e| panic!("{chemin:?} : {e}"));
        let profil = ProfilIcc::lire(&octets).unwrap_or_else(|e| panic!("{nom} : {e:?}"));
        let rvb = xicclu(&chemin, "-fb", &couleurs);
        let attendues = xicclu(&chemin, "-ff", &rvb);
        assert_eq!(attendues.len(), couleurs.len(), "{nom} : sorties de xicclu");
        let mut maximum = 0f64;
        for (lab, attendu) in couleurs.iter().zip(&attendues) {
            let obtenu = profil.reproduire(*lab).expect("table inverse lue");
            maximum = maximum.max(ecart(obtenu, *attendu));
        }
        println!("{nom} : écart maximal à xicclu {maximum:.4} ΔE00 sur 100 couleurs");
        assert!(maximum <= 0.1, "{nom} : écart {maximum} ΔE00");
    }
}
