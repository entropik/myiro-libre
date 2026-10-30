//! Valeurs de référence du prototype, sur les profils de l'essai du
//! 26 octobre 2026. Ces profils sont à l'atelier et restent hors du dépôt :
//! la variable `PROFILS_COMPARAISON` désigne le dossier local qui contient
//! `mt.icc`, `libre.icc` et `libre.gam`.
//!
//! `PROFILS_COMPARAISON=<dossier> cargo test -p gamut --test reference -- --ignored`

use std::path::PathBuf;

use gamut::{analyser, comparer, lire_gam, volume, ProfilIcc};

fn dossier() -> PathBuf {
    PathBuf::from(
        std::env::var_os("PROFILS_COMPARAISON")
            .expect("PROFILS_COMPARAISON : dossier des profils de l'essai"),
    )
}

fn profil(nom: &str) -> ProfilIcc {
    let chemin = dossier().join(nom);
    let octets = std::fs::read(&chemin).unwrap_or_else(|e| panic!("{chemin:?} : {e}"));
    ProfilIcc::lire(&octets).unwrap_or_else(|e| panic!("{nom} : {e:?}"))
}

fn proche(nom: &str, obtenu: f64, attendu: f64, tolerance: f64) {
    assert!(
        (obtenu - attendu).abs() <= tolerance,
        "{nom} : obtenu {obtenu}, attendu {attendu} à {tolerance} près"
    );
}

fn proche3(nom: &str, obtenu: [f64; 3], attendu: [f64; 3], tolerance: f64) {
    for k in 0..3 {
        proche(nom, obtenu[k], attendu[k], tolerance);
    }
}

#[test]
#[ignore = "profils de l'atelier, variable PROFILS_COMPARAISON"]
fn libre_icc_retrouve_les_valeurs_du_prototype() {
    let a = analyser(&profil("libre.icc"));
    proche("volume", a.volume_surface, 467_693.0, 2.0);
    proche3("rouge", a.primaires[2].lab, [55.35, 71.20, 53.12], 0.01);
    proche3("blanc", a.blanc, [97.91, -0.33, 2.18], 0.01);
    proche("densité du noir", a.dmax_approche, 1.59, 0.01);
}

#[test]
#[ignore = "profils de l'atelier, variable PROFILS_COMPARAISON"]
fn mt_icc_retrouve_les_valeurs_du_prototype() {
    let a = analyser(&profil("mt.icc"));
    proche("volume", a.volume_surface, 468_785.0, 2.0);
    proche3("rouge", a.primaires[2].lab, [55.38, 71.48, 53.28], 0.01);
}

#[test]
#[ignore = "profils de l'atelier, variable PROFILS_COMPARAISON"]
fn les_ecarts_entre_les_deux_profils_retrouvent_le_prototype() {
    let e = comparer(&profil("mt.icc"), &profil("libre.icc"));
    proche("moyenne", e.grille.moyenne, 0.31, 0.01);
    proche("p95", e.grille.p95, 0.65, 0.01);
    proche("max", e.grille.max, 5.70, 0.01);
    proche("médiane", e.grille.mediane, 0.26, 0.01);
    proche("part > 1", e.grille.part_sup_1, 1.6, 0.1);
    proche("part > 2", e.grille.part_sup_2, 0.3, 0.1);
}

#[test]
#[ignore = "profils de l'atelier, variable PROFILS_COMPARAISON"]
fn la_limite_d_iccgamut_se_relit() {
    let chemin = dossier().join("libre.gam");
    let texte = std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{chemin:?} : {e}"));
    let m = lire_gam(&texte).expect("libre.gam lisible");
    assert_eq!(m.sommets.len(), 722);
    assert_eq!(m.triangles.len(), 1_440);
    // Méthode différente de celle d'iccgamut : 1 % près.
    proche("volume", volume(&m), 467_316.0, 4_673.0);
}
