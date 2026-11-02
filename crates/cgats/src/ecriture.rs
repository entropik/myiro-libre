//! Écriture d'une mesure de myiro-libre en CGATS.17.

use std::fmt::Write as _;

use pont_protocole::{
    ConditionMesure, ConditionsCalcul, Echantillonnage, Info, Lab, Mesure, Spectre,
};

use crate::provenance::{self, PREFIXE};
use crate::{erreur, ErreurCgats, MesureImportee, EMPLACEMENTS, FORMAT_CGATS, FORMAT_MOYENNE};

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
    ecrire_tableaux(mesure, None)
}

/// Comme [`ecrire`], avec les valeurs RVB envoyées à l'imprimante pour
/// chaque plage, de 0 à 100, dans l'ordre des plages : colonnes `RGB_R`,
/// `RGB_G`, `RGB_B` dans chaque tableau, relues avec une échelle confirmée.
/// Refuse des valeurs RVB en nombre différent des plages.
pub fn ecrire_avec_rvb(mesure: &Mesure, rvb: &[[f64; 3]]) -> Result<String, ErreurCgats> {
    let plages = mesure.plages().len();
    if rvb.len() != plages {
        return erreur(format!(
            "{} valeurs RVB pour {plages} plages : chaque plage a les siennes",
            rvb.len()
        ));
    }
    ecrire_tableaux(mesure, Some(rvb))
}

fn ecrire_tableaux(mesure: &Mesure, rvb: Option<&[[f64; 3]]>) -> Result<String, ErreurCgats> {
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
        let mut champs = vec!["SAMPLE_ID".to_string()];
        if rvb.is_some() {
            champs.extend(["RGB_R".into(), "RGB_G".into(), "RGB_B".into()]);
        }
        champs.extend(["LAB_L".into(), "LAB_A".into(), "LAB_B".into()]);
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
            for v in rvb.and_then(|r| r.get(rang)).into_iter().flatten() {
                let _ = write!(ligne, "	{v}");
            }
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

/// Écrit une **mesure moyennée** (ticket #35) : plusieurs mesures d'une même
/// mire, composées par myiro-libre, rangées comme une mesure importée. Le
/// fichier porte `MYIRO_LIBRE_FORMAT "myiro-libre/moyenne/1"`, l'empreinte
/// de la mire (`MYIRO_LIBRE_MIRE`) et les numéros des mesures sources dans
/// la bibliothèque (`MYIRO_LIBRE_SOURCES`, séparés par une espace). Aucune
/// provenance de pont : une moyenne n'est pas une mesure attestée par un
/// instrument. Aucun numéro de série n'est écrit.
///
/// Les RVB sont ceux de la mire : confirmés pour chaque plage, écrits de 0
/// à 100 et relus confirmés. Une condition confirmée est écrite
/// (`MEASUREMENT_CONDITION`) ; une condition supposée ne l'est que par sa
/// source lumineuse, et se relit supposée ; une condition inconnue reste
/// inconnue. Seuls les emplacements qui ont un Lab ou un spectre sont
/// écrits. Relue par [`crate::lire`], la mesure est la même.
///
/// Refusé en clair : RVB inconnus ou supposés, Lab ou spectre connu sur une
/// partie des plages seulement d'un emplacement, spectres de longueurs
/// différentes, condition supposée hors de son emplacement, aucune donnée.
pub fn ecrire_moyenne(
    mesure: &MesureImportee,
    mire: &str,
    sources: &[&str],
) -> Result<String, ErreurCgats> {
    let mut rvb = Vec::with_capacity(mesure.plages.len());
    for p in &mesure.plages {
        match &p.rvb {
            Info::Confirmee(v) => rvb.push(v.map(|c| f64::from(c) * 100.0)),
            _ => {
                return erreur(format!(
                    "plage {} : RVB de la mire inconnus ou supposés",
                    p.identifiant
                ))
            }
        }
    }
    let mut s = Sortie(String::new());
    let mut premier = true;
    for (n, emplacement) in EMPLACEMENTS.iter().enumerate() {
        let labs: Vec<Option<&Lab>> = mesure.plages.iter().map(|p| p.lab[n].valeur()).collect();
        let spectres: Vec<Option<&Spectre>> = mesure
            .plages
            .iter()
            .map(|p| p.spectres[n].valeur())
            .collect();
        let avec_lab = uniforme(&labs, "Lab", emplacement)?;
        let avec_spectre = uniforme(&spectres, "spectre", emplacement)?;
        if !avec_lab && !avec_spectre {
            continue;
        }
        let longueur = spectres.iter().flatten().map(|sp| sp.len()).max();
        if spectres
            .iter()
            .flatten()
            .any(|sp| Some(sp.len()) != longueur)
        {
            return erreur(format!("{emplacement} : spectres de longueurs différentes"));
        }
        let longueur = longueur.unwrap_or(0);

        s.ligne("CGATS.17");
        if std::mem::take(&mut premier) {
            s.mot_cle("ORIGINATOR", "myiro-libre")?;
            s.mot_cle("FILE_DESCRIPTOR", "Mesure moyennée myiro-libre")?;
            if let Some(date) = mesure.date.valeur() {
                s.mot_cle("CREATED", date)?;
            }
            if let Some(instrument) = mesure.instrument.valeur() {
                s.mot_cle("INSTRUMENTATION", instrument)?;
            }
            s.mot_cle_declare(&format!("{PREFIXE}FORMAT"), FORMAT_MOYENNE)?;
            s.mot_cle_declare(&format!("{PREFIXE}MIRE"), mire)?;
            s.mot_cle_declare(&format!("{PREFIXE}SOURCES"), &sources.join(" "))?;
        }
        match &mesure.conditions[n] {
            Info::Confirmee(c) => {
                s.mot_cle_declare(&format!("{PREFIXE}SPECTRE"), emplacement)?;
                s.mot_cle_declare("MEASUREMENT_CONDITION", &format!("{c:?}"))?;
                s.mot_cle("MEASUREMENT_SOURCE", source(*c))?;
            }
            // Relue supposée par sa seule source lumineuse, à son emplacement.
            Info::Supposee(c) if *c as usize == n => {
                s.mot_cle("MEASUREMENT_SOURCE", source(*c))?;
            }
            Info::Supposee(c) => {
                return erreur(format!(
                    "{emplacement} : condition supposée {c:?} hors de son emplacement"
                ))
            }
            Info::Inconnue => s.mot_cle_declare(&format!("{PREFIXE}SPECTRE"), emplacement)?,
        }
        let echantillonnage = mesure.longueurs_onde.valeur().copied();
        let nm =
            |e: Echantillonnage, i: usize| u64::from(e.debut_nm) + u64::from(e.pas_nm) * i as u64;
        let mut champs: Vec<String> = ["SAMPLE_ID", "RGB_R", "RGB_G", "RGB_B"]
            .map(String::from)
            .into();
        if avec_lab {
            champs.extend(["LAB_L".into(), "LAB_A".into(), "LAB_B".into()]);
        }
        champs.extend((0..longueur).map(|i| match echantillonnage {
            Some(e) => format!("SPEC_{}", nm(e, i)),
            None => format!("{PREFIXE}SPECTRE_{}", i + 1),
        }));
        s.ligne(&format!("NUMBER_OF_FIELDS\t{}", champs.len()));
        s.ligne("BEGIN_DATA_FORMAT");
        s.ligne(&champs.join("\t"));
        s.ligne("END_DATA_FORMAT");
        s.ligne(&format!("NUMBER_OF_SETS\t{}", mesure.plages.len()));
        s.ligne("BEGIN_DATA");
        for (rang, p) in mesure.plages.iter().enumerate() {
            let mut ligne = identifiant(&p.identifiant)?;
            for v in rvb[rang] {
                let _ = write!(ligne, "\t{v}");
            }
            for v in labs[rang].into_iter().flat_map(|l| l.valeurs()) {
                let _ = write!(ligne, "\t{v}");
            }
            for v in spectres[rang].into_iter().flat_map(|sp| sp.iter()) {
                let _ = write!(ligne, "\t{}", f64::from(*v) * 100.0);
            }
            s.ligne(&ligne);
        }
        s.ligne("END_DATA");
    }
    if premier {
        return erreur("aucun Lab ni spectre à écrire");
    }
    Ok(s.0)
}

/// Une donnée connue sur toutes les plages d'un emplacement (`true`), sur
/// aucune (`false`) ; sur une partie seulement, refusée.
fn uniforme<T>(valeurs: &[Option<T>], quoi: &str, emplacement: &str) -> Result<bool, ErreurCgats> {
    let connues = valeurs.iter().filter(|v| v.is_some()).count();
    if connues != 0 && connues != valeurs.len() {
        return erreur(format!(
            "{emplacement} : {quoi} connu sur {connues} plages sur {}",
            valeurs.len()
        ));
    }
    Ok(connues != 0)
}

/// Identifiant de plage écrit sans guillemets : il ne doit contenir ni
/// espace, ni tabulation, ni guillemet.
fn identifiant(texte: &str) -> Result<String, ErreurCgats> {
    if texte.is_empty() || texte.chars().any(|c| c.is_whitespace() || c == '"') {
        return erreur(format!(
            "identifiant de plage « {texte} » impossible à écrire"
        ));
    }
    Ok(texte.to_string())
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
