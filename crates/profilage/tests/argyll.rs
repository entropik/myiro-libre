//! Essais avec le vrai ArgyllCMS, ignorés par défaut :
//! `cargo test -p profilage --test argyll -- --ignored --nocapture`.
//!
//! Racine d'ArgyllCMS : variable `ARGYLL`, sinon le premier dossier `argyll`
//! trouvé par `chercher_argyll` en remontant depuis cette crate (racine du
//! dépôt, ou dossier principal pour un worktree sous `.claude/worktrees/`).

use std::path::{Path, PathBuf};
use std::process::Command;

use bibliotheque::{espace_icc, EspaceProfil};
use pont_protocole::ConditionMesure;
use profilage::{
    chercher_argyll, ecrire_ti3, ArgyllCms, Avancement, DemandeEvaluation, DemandeProfil,
    EtapeCalcul, Impression, MoteurProfil, MoteurVerification, PlageProfil, PlagesProfil, Recette,
    VERSION_ARGYLL,
};

fn racine_argyll() -> PathBuf {
    if let Some(r) = std::env::var_os("ARGYLL") {
        return PathBuf::from(r);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(chercher_argyll)
        .expect("ArgyllCMS introuvable : variable ARGYLL ou dossier argyll/ du dépôt")
}

/// Grille RVB 5×5×5 (125 plages), Lab d'une fonction simple et régulière.
fn mire_synthetique() -> PlagesProfil {
    let niveaux = [0.0, 25.0, 50.0, 75.0, 100.0];
    let mut plages = Vec::new();
    for r in niveaux {
        for g in niveaux {
            for b in niveaux {
                plages.push(PlageProfil {
                    rvb: [r, g, b],
                    lab: [
                        20.0 + 0.75 * (r + g + b) / 3.0,
                        0.4 * (r - g),
                        0.4 * (g - b),
                    ],
                    spectre: None,
                });
            }
        }
    }
    PlagesProfil {
        plages,
        condition: ConditionMesure::M1,
        condition_supposee: false,
        rvb_supposes: false,
        longueurs_onde: None,
    }
}

fn calculer(description: &str, recette: Recette) -> profilage::ProfilCalcule {
    calculer_avec_avancement(description, recette).0
}

/// Calcule et rend aussi tout ce que le moteur a annoncé en route.
fn calculer_avec_avancement(
    description: &str,
    recette: Recette,
) -> (profilage::ProfilCalcule, Vec<Avancement>) {
    let mut annonces = Vec::new();
    let ti3 = ecrire_ti3(&mire_synthetique(), description, "2026-10-23T09:00:00Z");
    let moteur = ArgyllCms::new(racine_argyll());
    let profil = moteur
        .calculer(
            &DemandeProfil {
                ti3: &ti3,
                description,
                recette,
            },
            &mut |a| annonces.push(a),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    println!("avancement : {} annonces", annonces.len());
    println!("{}", profil.journal);
    println!("commandes : {:#?}", profil.commandes);
    println!("précision : {:?}", profil.precision);
    (profil, annonces)
}

/// Vingt RVB (de 0 à 1) d'une seconde mire : blanc, noir, primaires,
/// secondaires, gris et quelques couleurs entre deux.
fn seconde_mire() -> Vec<[f64; 3]> {
    let mut rvb = vec![
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
    ];
    for i in 1..=6 {
        let g = f64::from(i) / 7.0;
        rvb.push([g, g, g]);
    }
    rvb.extend([
        [0.8, 0.6, 0.5],
        [0.3, 0.5, 0.2],
        [0.15, 0.3, 0.6],
        [0.9, 0.4, 0.1],
        [0.55, 0.2, 0.45],
        [0.33, 0.66, 0.9],
    ]);
    rvb
}

/// Le profil de la mire synthétique, interrogé par xicclu comme pour une
/// seconde mire imprimée à travers lui (sRVB, colorimétrique relatif).
#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn xicclu_evalue_un_profil_calcule() {
    let profil = calculer("Essai verification 2026-10-28", Recette::Rapide);
    let rvb = seconde_mire();
    let moteur = ArgyllCms::new(racine_argyll());
    for impression in [Impression::ATraversLeProfil, Impression::SansGestion] {
        let e = moteur
            .evaluer(&DemandeEvaluation {
                icc: &profil.icc,
                rvb: &rvb,
                impression,
            })
            .unwrap_or_else(|err| panic!("{err}"));
        println!("{impression:?}\n{}", e.journal);
        println!("demande : {:?}", e.demande.as_ref().map(|d| &d[..3]));
        println!("appareil : {:?}", &e.appareil[..3]);
        println!("prédiction relative : {:?}", &e.prediction_rel[..3]);
        println!("prédiction absolue : {:?}", &e.prediction_abs[..3]);
        println!("blanc du support : {:?}", e.blanc_support);
        assert_eq!(e.version, VERSION_ARGYLL);
        assert_eq!(e.appareil.len(), rvb.len());
        assert_eq!(e.prediction_rel.len(), rvb.len());
        assert_eq!(e.prediction_abs.len(), rvb.len());
        assert_eq!(
            e.demande.is_some(),
            impression == Impression::ATraversLeProfil
        );
        assert!(e.blanc_support[0] > 90.0, "{:?}", e.blanc_support);
        assert!(e
            .appareil
            .iter()
            .flatten()
            .all(|v| (-1e-6..=1.0 + 1e-6).contains(v)));
        assert_eq!(e.commandes, moteur.commandes_verification(impression));
    }
}

/// Notre passage d'une prédiction absolue en relatif (règle ICC, rapport au
/// blanc du support en XYZ) concorde avec celui d'ArgyllCMS (`-ir`).
#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn notre_passage_en_relatif_concorde_avec_argyll() {
    let profil = calculer("Essai relatif 2026-10-28", Recette::Rapide);
    let rvb = seconde_mire();
    let e = ArgyllCms::new(racine_argyll())
        .evaluer(&DemandeEvaluation {
            icc: &profil.icc,
            rvb: &rvb,
            impression: Impression::SansGestion,
        })
        .unwrap_or_else(|err| panic!("{err}"));
    let lab = |[l, a, b]: [f64; 3]| colorimetrie::Lab { l, a, b };
    let blanc = lab(e.blanc_support);
    let mut pire = 0.0f64;
    for (abs, rel) in e.prediction_abs.iter().zip(&e.prediction_rel) {
        let notre = verification::vers_relatif(lab(*abs), blanc).unwrap();
        let ecart = colorimetrie::delta_e00(notre, lab(*rel)).unwrap();
        pire = pire.max(ecart);
        assert!(
            ecart <= 0.2,
            "{abs:?} → {notre:?}, ArgyllCMS {rel:?} : {ecart}"
        );
    }
    println!("plus grand écart entre les deux passages en relatif : {pire:.4} ΔE00");
}

/// Description d'un profil ICC v2 (`desc`, textDescriptionType) : partie
/// ASCII et partie Unicode.
fn description_icc(icc: &[u8]) -> (String, String) {
    let u32_a =
        |i: usize| u32::from_be_bytes([icc[i], icc[i + 1], icc[i + 2], icc[i + 3]]) as usize;
    let nombre = u32_a(128);
    let entree = (0..nombre)
        .map(|n| 132 + 12 * n)
        .find(|&e| &icc[e..e + 4] == b"desc")
        .expect("pas de balise desc");
    let debut = u32_a(entree + 4);
    assert_eq!(&icc[debut..debut + 4], b"desc", "type de la balise desc");
    let n_ascii = u32_a(debut + 8);
    let ascii = &icc[debut + 12..debut + 12 + n_ascii];
    let ascii = String::from_utf8_lossy(ascii)
        .trim_end_matches('\0')
        .to_string();
    let suite = debut + 12 + n_ascii;
    let n_unicode = u32_a(suite + 4);
    let unicode: Vec<u16> = (0..n_unicode)
        .map(|i| u16::from_be_bytes([icc[suite + 8 + 2 * i], icc[suite + 9 + 2 * i]]))
        .collect();
    let unicode = String::from_utf16_lossy(&unicode)
        .trim_end_matches('\0')
        .to_string();
    (ascii, unicode)
}

#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn argyll_du_depot_est_en_version_3_5_0() {
    let version = ArgyllCms::new(racine_argyll()).version().unwrap();
    assert_eq!(version, VERSION_ARGYLL);
}

#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn un_profil_se_calcule_sur_une_mire_synthetique() {
    let (profil, annonces) =
        calculer_avec_avancement("Essai synthétique 2026-10-23", Recette::Standard);
    // colprof annonce ses étapes, ses lignes et des pourcentages à lui ;
    // les parties de colprof sont repérées dans ses lignes, dans l'ordre.
    let etapes: Vec<_> = annonces
        .iter()
        .filter(|a| matches!(a, Avancement::Etape(_)))
        .collect();
    assert_eq!(
        etapes,
        [
            &Avancement::Etape(EtapeCalcul::Profil),
            &Avancement::Etape(EtapeCalcul::Table),
            &Avancement::Etape(EtapeCalcul::Gamut),
            &Avancement::Etape(EtapeCalcul::Inverses),
            &Avancement::Etape(EtapeCalcul::Limite),
            &Avancement::Etape(EtapeCalcul::Controle)
        ]
    );
    assert!(annonces.contains(&Avancement::Ligne("Creating B to A tables".into())));
    assert!(annonces.contains(&Avancement::Pourcentage(50)));
    assert_eq!(espace_icc(&profil.icc), Ok(EspaceProfil::Rvb));
    assert!(
        profil.precision.moyenne_de00 < 1.0,
        "{:?}",
        profil.precision
    );
    assert!(profil.commandes[0].starts_with("colprof -v -qh "));
}

/// Guillemets doublés dans le `DESCRIPTOR` du `.ti3` et accents.
/// `colprof` 3.5.0 n'écrit que la partie ASCII de la description (un accent
/// y deviendrait « ? », constaté le 26 octobre 2026) : `-D` reçoit le nom
/// sans accents, les guillemets passent tels quels.
#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn une_description_accentuee_avec_guillemets_s_ecrit_sans_accents_dans_le_profil() {
    let description = "Hahnemühle Photo Rag 308 Canson \"Baryta\"";
    let profil = calculer(description, Recette::Rapide);
    let (ascii, unicode) = description_icc(&profil.icc);
    println!("desc ASCII : {ascii:?}\ndesc Unicode : {unicode:?}");
    assert_eq!(ascii, "Hahnemuhle Photo Rag 308 Canson \"Baryta\"");
}

/// Valeur de `COLOR_REP` qu'ArgyllCMS écrit pour une imprimante RVB
/// (`targen -d2`) : "iRGB" dans le `.ti1`, sans mesure ; notre `.ti3`, qui
/// porte aussi le Lab, écrit "iRGB_LAB" (confirmé avec 3.5.0).
#[test]
#[ignore = "demande ArgyllCMS 3.5.0"]
fn targen_ecrit_irgb_pour_une_imprimante_rvb() {
    let dossier = tempfile::tempdir().unwrap();
    let targen = racine_argyll()
        .join("bin")
        .join(format!("targen{}", std::env::consts::EXE_SUFFIX));
    let sortie = Command::new(targen)
        .args(["-d2", "-f20", "essai"])
        .current_dir(dossier.path())
        .output()
        .unwrap();
    assert!(sortie.status.success(), "{sortie:?}");
    let ti1 = std::fs::read_to_string(dossier.path().join("essai.ti1")).unwrap();
    let ligne = ti1
        .lines()
        .find(|l| l.starts_with("COLOR_REP"))
        .expect("COLOR_REP absent");
    println!("{ligne}");
    assert_eq!(ligne.trim(), "COLOR_REP \"iRGB\"");
}
