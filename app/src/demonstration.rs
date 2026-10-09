//! Bibliothèque de démonstration : quelques mesures fictives pour voir la
//! colonne de gauche et le panneau de détails sans instrument
//! (`cargo run -p app -- --demo`). Rien ici ne vient d'un instrument réel : le
//! numéro de série est fictif (12345678) et les spectres sont des courbes
//! dessinées, dont les Lab sont calculés par la crate `colorimetrie`.

use bibliotheque::{Bibliotheque, ErreurBibliotheque};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Geometrie,
    Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Plage, Provenance,
    Spectre,
};

/// Dossier de la bibliothèque de démonstration, recréé à chaque lancement,
/// séparé de la vraie bibliothèque.
pub fn dossier() -> std::path::PathBuf {
    std::env::temp_dir().join("myiro-libre-demonstration")
}

/// Couleur fictive : réflectance de fond, et creux d'absorption (centre en
/// nm, largeur, profondeur) ; `azurant` ajoute un peu de bleu sous UV.
struct Couleur {
    fond: f64,
    creux: &'static [(f64, f64, f64)],
    azurant: f64,
}

const PAPIER: Couleur = Couleur {
    fond: 0.9,
    creux: &[],
    azurant: 0.06,
};
const CYAN: Couleur = Couleur {
    fond: 0.82,
    creux: &[(640.0, 70.0, 0.75)],
    azurant: 0.03,
};
const MAGENTA: Couleur = Couleur {
    fond: 0.85,
    creux: &[(540.0, 45.0, 0.78)],
    azurant: 0.03,
};
const JAUNE: Couleur = Couleur {
    fond: 0.88,
    creux: &[(440.0, 40.0, 0.82)],
    azurant: 0.0,
};
const NOIR: Couleur = Couleur {
    fond: 0.05,
    creux: &[],
    azurant: 0.0,
};

/// Spectre de 380 à 730 nm par 10 nm. `uv` règle la part d'azurant vue :
/// 1 pour M1, 0,5 pour M0, 0 pour M2 (sans UV).
fn spectre(couleur: &Couleur, uv: f64) -> Vec<f64> {
    (0..36)
        .map(|i| {
            let nm = 380.0 + 10.0 * i as f64;
            let mut r = couleur.fond;
            for (centre, largeur, profondeur) in couleur.creux {
                let x = (nm - centre) / largeur;
                r *= 1.0 - profondeur * (-x * x).exp();
            }
            let x = (nm - 440.0) / 25.0;
            r + uv * couleur.azurant * (-x * x).exp()
        })
        .collect()
}

fn plage(couleur: &Couleur) -> Plage {
    let spectres = [0.5, 1.0, 0.0].map(|uv| spectre(couleur, uv));
    let lab = spectres.clone().map(|s| {
        let lab = colorimetrie::spectre_vers_lab(&s).expect("spectre de démonstration calculable");
        Lab::new([lab.l as f32, lab.a as f32, lab.b as f32]).expect("Lab fini")
    });
    let spectres =
        spectres.map(|s| Spectre::new(s.into_iter().map(|v| v as f32).collect()).unwrap());
    let brutes = DonneesBrutes::new(
        (0..152)
            .map(|i| (couleur.fond * 40_000.0) as f32 + i as f32)
            .collect(),
    )
    .unwrap();
    Plage::new(spectres, brutes, lab).unwrap()
}

fn provenance(horodatage: &str, etalonnage: &str, geometrie: Geometrie) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie: 12345678,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1D".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Inconnue,
        version_pont: "démonstration".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new(horodatage).unwrap(),
        etalonnage: Info::Confirmee(Horodatage::new(etalonnage).unwrap()),
        geometrie,
        calcul: Calcul {
            libelle: "données fictives de démonstration".into(),
            demande: Info::Confirmee(ConditionsCalcul {
                conditions_spectres: [
                    Info::Confirmee(ConditionMesure::M0),
                    Info::Confirmee(ConditionMesure::M1),
                    Info::Confirmee(ConditionMesure::M2),
                ],
                longueurs_onde: Info::Confirmee(Echantillonnage {
                    debut_nm: 380,
                    pas_nm: 10,
                }),
                illuminant_lab: Info::Confirmee(Illuminant::D50),
                observateur_lab: Info::Supposee(Observateur::DeuxDegres),
            }),
            observe: Info::Inconnue,
        },
    }
}

fn ponctuelle(couleur: &Couleur, horodatage: &str, etalonnage: &str) -> Mesure {
    Mesure::new(
        vec![plage(couleur)],
        provenance(horodatage, etalonnage, Geometrie::Ponctuelle {}),
    )
    .unwrap()
}

/// Garnit une bibliothèque vide de deux conditions d'impression et de
/// quelques mesures fictives ; une troisième condition reste vide.
pub fn remplir(biblio: &Bibliotheque) -> Result<(), ErreurBibliotheque> {
    let jet = biblio.creer_condition("Jet d’encre, brillant 260\u{202f}g")?;
    let offset = biblio.creer_condition("Offset, couché mat 150\u{202f}g")?;
    biblio.creer_condition("Sérigraphie, adhésif blanc")?;

    let etalonnage = "2026-10-06T09:12:00+02:00";
    biblio.enregistrer_mesure(
        jet.id,
        &ponctuelle(&PAPIER, "2026-10-06T09:15:42+02:00", etalonnage),
    )?;
    biblio.enregistrer_mesure(
        jet.id,
        &Mesure::new(
            vec![plage(&CYAN), plage(&MAGENTA), plage(&JAUNE), plage(&NOIR)],
            provenance(
                "2026-10-06T09:21:07+02:00",
                etalonnage,
                Geometrie::Bande { sens: 0 },
            ),
        )
        .unwrap(),
    )?;
    let etalonnage = "2026-10-07T14:02:00+02:00";
    biblio.enregistrer_mesure(
        offset.id,
        &ponctuelle(&MAGENTA, "2026-10-07T14:05:31+02:00", etalonnage),
    )?;
    biblio.enregistrer_mesure(
        offset.id,
        &ponctuelle(&CYAN, "2026-10-07T14:06:10+02:00", etalonnage),
    )?;
    // Mesure importée d'un fichier CGATS fictif de 36 plages, comme une mire
    // de comparaison : Lab seuls, en M1 d'après la source lumineuse, sans
    // date ; ses spectres restent inconnus.
    biblio.importer_mesure(jet.id, "releve-lab.txt", &cgats_lab_seul())?;
    Ok(())
}

/// Fichier CGATS fictif : 36 plages réparties sur un cercle de teintes, à
/// trois clartés. Aucune valeur ne vient d'une mesure réelle.
fn cgats_lab_seul() -> String {
    let mut texte = String::from(
        "CGATS.17\r\nORIGINATOR\t\"Logiciel fictif\"\r\nCREATED\t\"\"\r\n\
         INSTRUMENTATION\t\"FD-9\"\r\nSERIAL\t\"12345678\"\r\n\
         MEASUREMENT_SOURCE\t\"D50\"\r\nNUMBER_OF_FIELDS\t4\r\n\
         BEGIN_DATA_FORMAT\r\nSAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t36\r\nBEGIN_DATA\r\n",
    );
    for i in 0..36 {
        let teinte = f64::from(i) * 30.0_f64.to_radians();
        let clarte = [80.0, 60.0, 40.0][(i / 12) as usize];
        texte.push_str(&format!(
            "{}\t{clarte:.2}\t{:.2}\t{:.2}\r\n",
            i + 1,
            45.0 * teinte.cos(),
            45.0 * teinte.sin()
        ));
    }
    texte.push_str("END_DATA\r\n");
    texte
}
