//! Écriture d'une mesure de myiro-libre en CGATS.17.

use std::fmt::Write as _;

use pont_protocole::{ConditionMesure, ConditionsCalcul, Echantillonnage, Info, Mesure, Spectre};

use crate::provenance::{self, PREFIXE};
use crate::{erreur, ErreurCgats, EMPLACEMENTS, FORMAT_CGATS};

/// Chaîne CGATS entre guillemets ; un guillemet s'y écrit doublé. Un texte
/// sur plusieurs lignes ne s'écrit pas en CGATS : il est refusé plutôt que
/// modifié.
fn chaine(texte: &str) -> Result<String, ErreurCgats> {
    if texte.chars().any(char::is_control) {
        return erreur(format!(
            "« {texte} » contient un retour à la ligne ou une tabulation, impossible en CGATS"
        ));
    }
    Ok(format!("\"{}\"", texte.replace('"', "\"\"")))
}

struct Sortie(String);

impl Sortie {
    fn ligne(&mut self, texte: &str) {
        self.0.push_str(texte);
        self.0.push_str("\r\n");
    }

    fn mot_cle(&mut self, nom: &str, valeur: &str) -> Result<(), ErreurCgats> {
        let valeur = chaine(valeur)?;
        self.ligne(&format!("{nom}\t{valeur}"));
        Ok(())
    }

    /// Mot-clé hors de la liste de CGATS.17, déclaré par `KEYWORD` avant usage.
    fn mot_cle_declare(&mut self, nom: &str, valeur: &str) -> Result<(), ErreurCgats> {
        self.ligne(&format!("KEYWORD\t\"{nom}\""));
        self.mot_cle(nom, valeur)
    }
}

/// Ce que le pont a demandé, s'il l'a confirmé : seule une donnée confirmée
/// entre dans un mot-clé standard, lu sans réserve par les autres logiciels.
fn confirmee<T: Clone>(
    demande: &Info<ConditionsCalcul>,
    champ: impl Fn(&ConditionsCalcul) -> &Info<T>,
) -> Option<T> {
    match demande {
        Info::Confirmee(c) => match champ(c) {
            Info::Confirmee(v) => Some(v.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Écrit une mesure en CGATS.17 : un tableau par emplacement de spectre (`m0`,
/// `m1`, `m2`), chacun avec le Lab et le spectre de chaque plage ; la
/// provenance complète est dans l'en-tête du premier tableau.
///
/// Refuse seulement une mesure dont un texte de provenance tient sur
/// plusieurs lignes, que CGATS ne sait pas écrire.
pub fn ecrire(mesure: &Mesure) -> Result<String, ErreurCgats> {
    let p = mesure.provenance();
    let demande = &p.calcul.demande;
    let mut s = Sortie(String::new());

    for (n, emplacement) in EMPLACEMENTS.iter().enumerate() {
        s.ligne("CGATS.17");
        if n == 0 {
            s.mot_cle("ORIGINATOR", "myiro-libre")?;
            s.mot_cle("FILE_DESCRIPTOR", "Mesure myiro-libre")?;
            s.mot_cle("CREATED", p.horodatage.texte())?;
            s.mot_cle("INSTRUMENTATION", &p.instrument.modele)?;
            s.mot_cle("SERIAL", &p.instrument.numero_serie.to_string())?;
            s.mot_cle_declare(&format!("{PREFIXE}FORMAT"), FORMAT_CGATS)?;
            for (cle, valeur) in provenance::ecrire(p) {
                s.mot_cle_declare(&format!("{PREFIXE}{cle}"), &valeur)?;
            }
        }
        s.mot_cle_declare(&format!("{PREFIXE}SPECTRE"), emplacement)?;
        if let Some(condition) = confirmee(demande, |c| &c.conditions_spectres[n]) {
            s.mot_cle_declare("MEASUREMENT_CONDITION", &format!("{condition:?}"))?;
            s.mot_cle("MEASUREMENT_SOURCE", source(condition))?;
        }
        if confirmee(demande, |c| &c.illuminant_lab).is_some() {
            s.mot_cle("WEIGHTING_FUNCTION", "ILLUMINANT,D50")?;
        }
        if confirmee(demande, |c| &c.observateur_lab).is_some() {
            s.mot_cle("WEIGHTING_FUNCTION", "OBSERVER,2 degree")?;
        }

        let spectres: Vec<&Spectre> = mesure
            .plages()
            .iter()
            .map(|plage| [plage.m0(), plage.m1(), plage.m2()][n])
            .collect();
        let longueur = spectres.first().map_or(0, |sp| sp.len());
        let echantillonnage = demande
            .valeur()
            .and_then(|c| c.longueurs_onde.valeur().copied());
        // Longueur d'onde de la valeur n° i, en 64 bits : pas de débordement.
        let nm =
            |e: Echantillonnage, i: usize| u64::from(e.debut_nm) + u64::from(e.pas_nm) * i as u64;
        // Bandes décrites comme ArgyllCMS les attend (`ti3_format`).
        if let Some(e) = echantillonnage.filter(|_| longueur > 0) {
            s.mot_cle_declare("SPECTRAL_BANDS", &longueur.to_string())?;
            s.mot_cle_declare("SPECTRAL_START_NM", &e.debut_nm.to_string())?;
            s.mot_cle_declare("SPECTRAL_END_NM", &nm(e, longueur - 1).to_string())?;
        }
        let mut champs = vec![
            "SAMPLE_ID".to_string(),
            "LAB_L".into(),
            "LAB_A".into(),
            "LAB_B".into(),
        ];
        champs.extend((0..longueur).map(|i| match echantillonnage {
            Some(e) => format!("SPEC_{}", nm(e, i)),
            None => format!("{PREFIXE}SPECTRE_{}", i + 1),
        }));

        s.ligne(&format!("NUMBER_OF_FIELDS\t{}", champs.len()));
        s.ligne("BEGIN_DATA_FORMAT");
        s.ligne(&champs.join("\t"));
        s.ligne("END_DATA_FORMAT");
        s.ligne(&format!("NUMBER_OF_SETS\t{}", spectres.len()));
        s.ligne("BEGIN_DATA");
        for (rang, (plage, spectre)) in mesure.plages().iter().zip(&spectres).enumerate() {
            let mut ligne = (rang + 1).to_string();
            // `{}` d'un nombre : la plus courte écriture décimale qui se relit
            // à l'identique, sans exposant.
            for v in plage.lab()[n].valeurs() {
                let _ = write!(ligne, "\t{v}");
            }
            // Spectre en pourcentage, comme ArgyllCMS : le f32 × 100 est exact
            // en f64, et se relit à l'identique en divisant par 100.
            for v in spectre.iter() {
                let _ = write!(ligne, "\t{}", f64::from(*v) * 100.0);
            }
            s.ligne(&ligne);
        }
        s.ligne("END_DATA");
    }
    Ok(s.0)
}

/// Source lumineuse de la condition, à la manière des exports CGATS du
/// fabricant (`A`, `D50`, `UVCUT`), pour les logiciels qui ne lisent qu'elle.
pub(crate) fn source(condition: ConditionMesure) -> &'static str {
    match condition {
        ConditionMesure::M0 => "A",
        ConditionMesure::M1 => "D50",
        ConditionMesure::M2 => "UVCUT",
    }
}
