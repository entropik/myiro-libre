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
    Ok(())
}

// ---- Séance de démonstration de la tâche Mesurer (ticket #8) ----

/// Magenta du bon à tirer, puis trois tirages qui s'en écartent de plus en
/// plus : creux d'absorption décalé ou moins profond.
const MAGENTA_TIRAGES: [Couleur; 3] = [
    Couleur {
        fond: 0.85,
        creux: &[(541.0, 45.0, 0.775)],
        azurant: 0.03,
    },
    Couleur {
        fond: 0.85,
        creux: &[(545.0, 46.0, 0.76)],
        azurant: 0.03,
    },
    Couleur {
        fond: 0.86,
        creux: &[(550.0, 48.0, 0.72)],
        azurant: 0.03,
    },
];

/// Mesure ponctuelle dont le pont ne fait que supposer les conditions de
/// mesure : l'écran avertit que l'écart est à prendre avec prudence.
fn ponctuelle_supposee(couleur: &Couleur, horodatage: &str, etalonnage: &str) -> Mesure {
    let mut provenance = provenance(horodatage, etalonnage, Geometrie::Ponctuelle {});
    if let Info::Confirmee(demande) = provenance.calcul.demande {
        provenance.calcul.demande = Info::Supposee(demande);
    }
    Mesure::new(vec![plage(couleur)], provenance).unwrap()
}

/// Séance de Mesurer garnie de mesures fictives rangées dans la bibliothèque
/// de démonstration (condition « Offset ») : le magenta du bon à tirer est la
/// couleur de référence, avec un seuil de 2,00 ; les trois tirages donnent
/// les trois verdicts, et une dernière mesure l'avertissement sur la
/// condition de mesure.
pub fn seance(ouverte: &crate::colonne::BibliothequeOuverte) -> crate::mesurer::Seance {
    use crate::mesurer::Seance;
    use crate::textes::Langue;
    let mut seance = Seance::default();
    let Some(offset) = ouverte
        .conditions()
        .ok()
        .and_then(|c| c.into_iter().find(|c| c.nom.starts_with("Offset")))
    else {
        return seance;
    };
    let etalonnage = "2026-10-09T08:40:00+02:00";
    let mut mesures = vec![(
        "Magenta du BAT",
        ponctuelle(&MAGENTA, "2026-10-09T08:45:12+02:00", etalonnage),
    )];
    for (i, (nom, couleur)) in ["Tirage 1", "Tirage 2", "Tirage 3"]
        .into_iter()
        .zip(&MAGENTA_TIRAGES)
        .enumerate()
    {
        let heure = format!("2026-10-09T09:{:02}:30+02:00", 10 + 5 * i);
        mesures.push((nom, ponctuelle(couleur, &heure, etalonnage)));
    }
    mesures.push((
        "Tirage 4, autre réglage",
        ponctuelle_supposee(&MAGENTA_TIRAGES[0], "2026-10-09T09:40:05+02:00", etalonnage),
    ));
    for (nom, mesure) in mesures {
        let numero = seance.ajouter(
            mesure,
            &offset,
            |m, n| ouverte.enregistrer_mesure_nommee(offset.id, m, n),
            Langue::Francais,
        );
        let _ = seance.renommer(numero, nom, |id, n| ouverte.renommer_mesure(id, n));
    }
    let _ = seance.designer_reference(1, ouverte);
    let _ = seance.regler_seuil("2", ouverte);
    seance
}

#[cfg(test)]
mod tests {
    use crate::mesurer::{Comparaison, VerdictEcart};
    use crate::textes::Langue;

    /// La démonstration montre les trois verdicts et l'avertissement, pour
    /// la validation de l'écran.
    #[test]
    fn la_seance_de_demonstration_montre_chaque_verdict() {
        let dossier = tempfile::tempdir().unwrap();
        let ouverte = crate::colonne::BibliothequeOuverte::demonstration(dossier.path());
        let seance = super::seance(&ouverte);

        let reference = seance.reference(Langue::Francais).unwrap();
        assert_eq!(
            (reference.numero, reference.seuil.as_deref()),
            (1, Some("2,00"))
        );
        // Spectre M1 (deuxième), de la plus récente à la plus ancienne.
        let ecarts: Vec<_> = seance
            .fiches(Langue::Francais)
            .into_iter()
            .filter_map(|f| f.spectres[1].ecart.clone())
            .map(|e| (e.verdict, e.comparaison, e.delta_e00.unwrap()))
            .collect();
        let verdicts: Vec<_> = ecarts.iter().map(|e| (e.0, e.1)).collect();
        assert_eq!(
            verdicts,
            [
                (VerdictEcart::Conforme, Comparaison::NonConfirmee),
                (VerdictEcart::HorsTolerance, Comparaison::MemeCondition),
                (VerdictEcart::ProcheDeLaLimite, Comparaison::MemeCondition),
                (VerdictEcart::Conforme, Comparaison::MemeCondition),
            ],
            "{ecarts:?}"
        );
    }
}
