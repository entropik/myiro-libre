//! Fichier `.ti3` d'ArgyllCMS : les plages d'une mesure, telles que `colprof`
//! les lit (description du format `ti3_format` d'ArgyllCMS).

use std::fmt::Write as _;

use crate::PlagesProfil;

/// Chaîne CGATS entre guillemets ; un guillemet s'y écrit doublé. Un retour
/// à la ligne ou une tabulation devient une espace : une chaîne CGATS tient
/// sur une ligne.
fn chaine(texte: &str) -> String {
    let texte: String = texte
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    format!("\"{}\"", texte.replace('"', "\"\""))
}

struct Sortie(String);

impl Sortie {
    fn ligne(&mut self, texte: &str) {
        self.0.push_str(texte);
        self.0.push_str("\r\n");
    }

    fn mot_cle(&mut self, nom: &str, valeur: &str) {
        self.ligne(&format!("{nom} {}", chaine(valeur)));
    }

    /// Mot-clé hors de la liste de CGATS.17, déclaré par `KEYWORD` avant usage.
    fn mot_cle_declare(&mut self, nom: &str, valeur: &str) {
        self.ligne(&format!("KEYWORD \"{nom}\""));
        self.mot_cle(nom, valeur);
    }
}

/// Écrit les plages en `.ti3` pour une imprimante RVB : `description` devient
/// le `DESCRIPTOR`, `cree` la date `CREATED`. Les plages sont numérotées de 1
/// à n ; les spectres, en pourcentage, sont écrits s'ils sont connus pour
/// toutes les plages.
pub fn ecrire_ti3(plages: &PlagesProfil, description: &str, cree: &str) -> String {
    let mut s = Sortie(String::new());
    s.ligne("CTI3");
    s.mot_cle("DESCRIPTOR", description);
    s.mot_cle("ORIGINATOR", "myiro-libre");
    s.mot_cle("CREATED", cree);
    s.mot_cle_declare("DEVICE_CLASS", "OUTPUT");
    // `targen -d2` écrit "iRGB" dans le `.ti1` (valeurs RVB seules) ; le
    // `.ti3` y ajoute les mesures Lab : "iRGB_LAB", que `colprof` 3.5.0 lit
    // (confirmé le 26 octobre 2026).
    s.mot_cle_declare("COLOR_REP", "iRGB_LAB");

    let longueur = plages
        .plages
        .first()
        .and_then(|p| p.spectre.as_ref())
        .map_or(0, Vec::len);
    // Spectres écrits seulement s'ils sont tous là et de même longueur.
    let complets = plages
        .plages
        .iter()
        .all(|p| p.spectre.as_ref().is_some_and(|sp| sp.len() == longueur));
    let bandes = plages.longueurs_onde.filter(|_| longueur > 0 && complets);
    // Longueur d'onde de la valeur n° i, en 64 bits : pas de débordement.
    let nm = |i: usize| {
        bandes.map_or(0, |e| {
            u64::from(e.debut_nm) + u64::from(e.pas_nm) * i as u64
        })
    };
    s.mot_cle_declare(
        "INSTRUMENT_TYPE_SPECTRAL",
        if bandes.is_some() { "YES" } else { "NO" },
    );
    if let Some(e) = bandes {
        s.mot_cle_declare("SPECTRAL_BANDS", &longueur.to_string());
        s.mot_cle_declare("SPECTRAL_START_NM", &e.debut_nm.to_string());
        s.mot_cle_declare("SPECTRAL_END_NM", &nm(longueur - 1).to_string());
    }

    let mut champs: Vec<String> = [
        "SAMPLE_ID",
        "RGB_R",
        "RGB_G",
        "RGB_B",
        "LAB_L",
        "LAB_A",
        "LAB_B",
    ]
    .map(String::from)
    .into();
    if bandes.is_some() {
        champs.extend((0..longueur).map(|i| format!("SPEC_{}", nm(i))));
    }
    s.ligne(&format!("NUMBER_OF_FIELDS {}", champs.len()));
    s.ligne("BEGIN_DATA_FORMAT");
    s.ligne(&champs.join(" "));
    s.ligne("END_DATA_FORMAT");
    s.ligne(&format!("NUMBER_OF_SETS {}", plages.plages.len()));
    s.ligne("BEGIN_DATA");
    for (rang, plage) in plages.plages.iter().enumerate() {
        let mut ligne = (rang + 1).to_string();
        // `{}` d'un nombre : la plus courte écriture décimale qui se relit
        // à l'identique, sans exposant.
        for v in plage.rvb.iter().chain(&plage.lab) {
            let _ = write!(ligne, " {v}");
        }
        if bandes.is_some() {
            for v in plage.spectre.iter().flatten() {
                let _ = write!(ligne, " {v}");
            }
        }
        s.ligne(&ligne);
    }
    s.ligne("END_DATA");
    s.0
}
