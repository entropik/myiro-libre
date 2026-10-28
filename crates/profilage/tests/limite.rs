//! Limite du gamut d'un profil par `iccgamut`, derrière le trait
//! `MoteurGamut`. Le test réel est ignoré par défaut :
//! `cargo test -p profilage --test limite -- --ignored --nocapture`
//! (variable `ARGYLL`, sinon le dossier `argyll/` du dépôt, comme
//! `tests/argyll.rs`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use profilage::{
    arguments_iccgamut, chercher_argyll, controler_version, lire_volume_iccgamut, ArgyllCms,
    ErreurMoteur, Etape, MoteurGamut, MoteurGamutSimule, VERSION_ARGYLL,
};

#[test]
fn le_volume_se_lit_dans_la_sortie_d_iccgamut() {
    let sortie = "Doing input space\n\
                  Total volume of gamut is 467316.128099 cubic colorspace units\n\
                  Output Total surface area = 1234\n";
    assert_eq!(lire_volume_iccgamut(sortie), Some(467316.128099));
    assert_eq!(lire_volume_iccgamut("iccgamut: Error - x\n"), None);
    assert_eq!(
        lire_volume_iccgamut("Total volume of gamut is ??? cubic colorspace units"),
        None
    );
}

#[test]
fn les_arguments_d_iccgamut_sont_fixes() {
    let attendus: Vec<OsString> = ["-v", "-ia", "-pl", "profil.icc"]
        .map(OsString::from)
        .into();
    assert_eq!(arguments_iccgamut(), attendus);
    let moteur = ArgyllCms::new(PathBuf::from("absent"));
    assert_eq!(moteur.commande_limite(), "iccgamut -v -ia -pl profil.icc");
    assert_eq!(
        MoteurGamutSimule::reussit(String::new()).commande_limite(),
        "iccgamut -v -ia -pl profil.icc"
    );
}

#[test]
fn le_moteur_simule_rend_sa_limite() {
    let moteur = MoteurGamutSimule::reussit("GAMUT\n".to_string());
    let limite = moteur.limite(b"profil").unwrap();
    assert_eq!(limite.gam, "GAMUT\n");
    assert_eq!(limite.volume, None);
    assert_eq!(limite.version, VERSION_ARGYLL);
}

#[test]
fn le_moteur_simule_echoue_comme_demande() {
    let erreur = ErreurMoteur::Echec {
        etape: Etape::Gamut,
        cause: "profil illisible".into(),
    };
    let moteur = MoteurGamutSimule::echoue(erreur.clone());
    assert_eq!(moteur.limite(b"profil"), Err(erreur.clone()));
    assert_eq!(erreur.cle(), "comparer.erreur.limite");
    assert!(erreur.to_string().contains("limite du gamut"), "{erreur}");
}

/// ArgyllCMS ne se simule pas par un faux `colprof.exe` : comme au plan 3,
/// le refus d'une autre version passe par `controler_version`, que
/// `limite` appelle avant tout calcul.
#[test]
fn une_autre_version_refuse_la_limite() {
    assert_eq!(
        controler_version("3.4.0"),
        Err(ErreurMoteur::VersionNonValidee {
            trouvee: "3.4.0".into()
        })
    );
}

#[test]
fn sans_argyll_la_limite_est_introuvable() {
    let vide = tempfile::tempdir().unwrap();
    let moteur = ArgyllCms::new(vide.path().join("argyll"));
    assert_eq!(moteur.limite(b"profil"), Err(ErreurMoteur::Introuvable));
}

fn racine_argyll() -> PathBuf {
    if let Some(r) = std::env::var_os("ARGYLL") {
        return PathBuf::from(r);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(chercher_argyll)
        .expect("ArgyllCMS introuvable : variable ARGYLL ou dossier argyll/ du dépôt")
}

/// Profil d'essai linéaire : son gamut est l'image du cube RVB par une
/// transformation affine de déterminant 270 000.
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

#[test]
#[ignore]
fn limite_reelle_d_un_profil_d_essai() {
    let icc = gamut::profil_essai(&gamut::ProfilEssai {
        grille: 9,
        blanc: [100.0, 0.0, 0.0],
        cible: lineaire,
        description: "Essai lineaire".into(),
        mesures: None,
    });
    let moteur = ArgyllCms::new(racine_argyll());
    let limite = moteur.limite(&icc).unwrap_or_else(|e| panic!("{e}"));
    println!("volume iccgamut : {:?}", limite.volume);
    assert_eq!(limite.version, VERSION_ARGYLL);
    let maillage = gamut::lire_gam(&limite.gam).unwrap();
    let volume = gamut::volume(&maillage);
    println!(
        "{} sommets, {} triangles, volume du .gam {volume}",
        maillage.sommets.len(),
        maillage.triangles.len()
    );
    assert!((volume - 270_000.0).abs() <= 0.02 * 270_000.0, "{volume}");
    let annonce = limite.volume.expect("volume lu dans la sortie d'iccgamut");
    assert!((annonce - 270_000.0).abs() <= 0.02 * 270_000.0, "{annonce}");
}
