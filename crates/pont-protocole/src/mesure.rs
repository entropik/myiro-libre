//! Contrat des mesures : valeurs, plages, provenance et format conservable.
//!
//! Chaque type vérifie ses invariants à la construction comme à la lecture
//! (`serde` passe par la même vérification) : un consommateur ne peut pas
//! obtenir une mesure mal formée. Format décrit dans `docs/formats/mesure.md`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

/// Défaut d'une mesure, expliqué en clair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErreurMesure(pub String);

impl fmt::Display for ErreurMesure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErreurMesure {}

fn erreur<T>(detail: impl Into<String>) -> Result<T, ErreurMesure> {
    Err(ErreurMesure(detail.into()))
}

fn verifier_finies(quoi: &str, valeurs: &[f32]) -> Result<(), ErreurMesure> {
    match valeurs.iter().position(|v| !v.is_finite()) {
        Some(i) => erreur(format!("{quoi} : valeur n° {} non finie", i + 1)),
        None => Ok(()),
    }
}

/// Spectre de réflexion : au moins une valeur, toutes finies. Une valeur
/// supérieure à 1 (papier azuré, fluorescence) est une mesure, pas une erreur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<f32>", into = "Vec<f32>")]
pub struct Spectre(Vec<f32>);

impl Spectre {
    pub fn new(valeurs: Vec<f32>) -> Result<Self, ErreurMesure> {
        if valeurs.is_empty() {
            return erreur("spectre vide");
        }
        verifier_finies("spectre", &valeurs)?;
        Ok(Spectre(valeurs))
    }
}

impl TryFrom<Vec<f32>> for Spectre {
    type Error = ErreurMesure;
    fn try_from(valeurs: Vec<f32>) -> Result<Self, ErreurMesure> {
        Spectre::new(valeurs)
    }
}

impl From<Spectre> for Vec<f32> {
    fn from(spectre: Spectre) -> Self {
        spectre.0
    }
}

impl Deref for Spectre {
    type Target = [f32];
    fn deref(&self) -> &[f32] {
        &self.0
    }
}

/// Données brutes de l'instrument, toutes finies (152 par plage pour le MYIRO-1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<f32>", into = "Vec<f32>")]
pub struct DonneesBrutes(Vec<f32>);

impl DonneesBrutes {
    pub fn new(valeurs: Vec<f32>) -> Result<Self, ErreurMesure> {
        verifier_finies("données brutes", &valeurs)?;
        Ok(DonneesBrutes(valeurs))
    }
}

impl TryFrom<Vec<f32>> for DonneesBrutes {
    type Error = ErreurMesure;
    fn try_from(valeurs: Vec<f32>) -> Result<Self, ErreurMesure> {
        DonneesBrutes::new(valeurs)
    }
}

impl From<DonneesBrutes> for Vec<f32> {
    fn from(brutes: DonneesBrutes) -> Self {
        brutes.0
    }
}

impl Deref for DonneesBrutes {
    type Target = [f32];
    fn deref(&self) -> &[f32] {
        &self.0
    }
}

/// L*, a*, b* : trois nombres finis.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<f32>", into = "Vec<f32>")]
pub struct Lab([f32; 3]);

impl Lab {
    pub fn new(valeurs: [f32; 3]) -> Result<Self, ErreurMesure> {
        verifier_finies("Lab", &valeurs)?;
        Ok(Lab(valeurs))
    }

    pub fn valeurs(&self) -> [f32; 3] {
        self.0
    }
}

impl TryFrom<Vec<f32>> for Lab {
    type Error = ErreurMesure;
    fn try_from(valeurs: Vec<f32>) -> Result<Self, ErreurMesure> {
        match <[f32; 3]>::try_from(valeurs) {
            Ok(trois) => Lab::new(trois),
            Err(valeurs) => erreur(format!("Lab : {} valeurs au lieu de 3", valeurs.len())),
        }
    }
}

impl From<Lab> for Vec<f32> {
    fn from(lab: Lab) -> Self {
        lab.0.to_vec()
    }
}

/// Une plage mesurée : spectres M0, M1, M2 de même longueur, données brutes
/// et Lab calculés par la DLL pour chacun des trois spectres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PlageLue")]
pub struct Plage {
    m0: Spectre,
    m1: Spectre,
    m2: Spectre,
    brutes: DonneesBrutes,
    lab_m0: Lab,
    lab_m1: Lab,
    lab_m2: Lab,
}

/// Forme lue avant vérification de la cohérence de la plage.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlageLue {
    m0: Spectre,
    m1: Spectre,
    m2: Spectre,
    brutes: DonneesBrutes,
    lab_m0: Lab,
    lab_m1: Lab,
    lab_m2: Lab,
}

impl TryFrom<PlageLue> for Plage {
    type Error = ErreurMesure;
    fn try_from(p: PlageLue) -> Result<Self, ErreurMesure> {
        Plage::new([p.m0, p.m1, p.m2], p.brutes, [p.lab_m0, p.lab_m1, p.lab_m2])
    }
}

impl Plage {
    /// Spectres et Lab dans l'ordre M0, M1, M2.
    pub fn new(
        spectres: [Spectre; 3],
        brutes: DonneesBrutes,
        lab: [Lab; 3],
    ) -> Result<Self, ErreurMesure> {
        let [m0, m1, m2] = spectres;
        if m1.len() != m0.len() || m2.len() != m0.len() {
            return erreur(format!(
                "spectres de longueurs différentes : {}, {}, {}",
                m0.len(),
                m1.len(),
                m2.len()
            ));
        }
        let [lab_m0, lab_m1, lab_m2] = lab;
        Ok(Plage {
            m0,
            m1,
            m2,
            brutes,
            lab_m0,
            lab_m1,
            lab_m2,
        })
    }

    pub fn m0(&self) -> &Spectre {
        &self.m0
    }

    pub fn m1(&self) -> &Spectre {
        &self.m1
    }

    pub fn m2(&self) -> &Spectre {
        &self.m2
    }

    pub fn brutes(&self) -> &DonneesBrutes {
        &self.brutes
    }

    /// Lab de M0, M1, M2, dans cet ordre.
    pub fn lab(&self) -> [Lab; 3] {
        [self.lab_m0, self.lab_m1, self.lab_m2]
    }
}

/// Niveau de connaissance d'une donnée (GLOSSARY : Inconnu). Écrit
/// `{"statut": "confirmee", "valeur": …}` : une valeur nue est refusée à la
/// lecture, et une donnée inconnue n'a jamais de valeur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "statut",
    content = "valeur",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Info<T> {
    /// Établie par une fiche `docs/abi/` confirmée ou observée par le pont.
    Confirmee(T),
    /// Plausible, pas encore vérifiée sur l'instrument.
    Supposee(T),
    Inconnue,
}

impl<T> Info<T> {
    /// La valeur, confirmée ou supposée.
    pub fn valeur(&self) -> Option<&T> {
        match self {
            Info::Confirmee(v) | Info::Supposee(v) => Some(v),
            Info::Inconnue => None,
        }
    }
}

/// Date et heure RFC 3339 avec fuseau : `2026-10-07T15:04:05+02:00` (ou `Z`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Horodatage(String);

impl Horodatage {
    pub fn new(texte: impl Into<String>) -> Result<Self, ErreurMesure> {
        let texte = texte.into();
        if horodatage_valable(&texte) {
            Ok(Horodatage(texte))
        } else {
            erreur(format!(
                "horodatage « {texte} » : attendu AAAA-MM-JJTHH:MM:SS avec fuseau"
            ))
        }
    }

    pub fn texte(&self) -> &str {
        &self.0
    }
}

fn horodatage_valable(texte: &str) -> bool {
    let o = texte.as_bytes();
    let nombre = |debut: usize, fin: usize| -> Option<u32> {
        let morceau = o.get(debut..fin)?;
        if !morceau.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(morceau).ok()?.parse().ok()
    };
    let date_heure = (|| {
        let (mois, jour) = (nombre(5, 7)?, nombre(8, 10)?);
        let (h, m, s) = (nombre(11, 13)?, nombre(14, 16)?, nombre(17, 19)?);
        nombre(0, 4)?;
        Some(
            o[4] == b'-'
                && o[7] == b'-'
                && o[10] == b'T'
                && o[13] == b':'
                && o[16] == b':'
                && (1..=12).contains(&mois)
                && (1..=31).contains(&jour)
                && h < 24
                && m < 60
                && s <= 60,
        )
    })();
    if date_heure != Some(true) {
        return false;
    }
    // Fraction de seconde facultative, puis fuseau obligatoire.
    let mut i = 19;
    if o.get(i) == Some(&b'.') {
        i += 1;
        let debut = i;
        while o.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == debut {
            return false;
        }
    }
    match &o[i..] {
        [b'Z'] => true,
        [signe, ..] if (*signe == b'+' || *signe == b'-') && o.len() == i + 6 => {
            o[i + 3] == b':'
                && nombre(i + 1, i + 3).is_some_and(|h| h < 24)
                && nombre(i + 4, i + 6).is_some_and(|m| m < 60)
        }
        _ => false,
    }
}

impl TryFrom<String> for Horodatage {
    type Error = ErreurMesure;
    fn try_from(texte: String) -> Result<Self, ErreurMesure> {
        Horodatage::new(texte)
    }
}

impl From<Horodatage> for String {
    fn from(h: Horodatage) -> Self {
        h.0
    }
}

/// SHA-256 en hexadécimal minuscule (64 caractères).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Empreinte(String);

impl Empreinte {
    pub fn new(texte: impl Into<String>) -> Result<Self, ErreurMesure> {
        let texte = texte.into();
        let hexa = |c: u8| c.is_ascii_digit() || (b'a'..=b'f').contains(&c);
        if texte.len() == 64 && texte.bytes().all(hexa) {
            Ok(Empreinte(texte))
        } else {
            erreur(format!(
                "empreinte « {texte} » : attendu 64 chiffres hexadécimaux minuscules"
            ))
        }
    }

    pub fn texte(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Empreinte {
    type Error = ErreurMesure;
    fn try_from(texte: String) -> Result<Self, ErreurMesure> {
        Empreinte::new(texte)
    }
}

impl From<Empreinte> for String {
    fn from(e: Empreinte) -> Self {
        e.0
    }
}

/// Géométrie de lecture (GLOSSARY : Bande, Feuille, Mesure ponctuelle).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "lecture", rename_all = "snake_case", deny_unknown_fields)]
pub enum Geometrie {
    // Accolades vides : sans elles, serde accepterait des champs en trop.
    Ponctuelle {},
    /// `sens` : sens de passage rendu par la DLL, brut (0, 1 ou 2 ; 2 inverse
    /// l'ordre des plages, fiche `docs/abi/FDX_GetMeasureData.md`).
    Bande {
        sens: u32,
    },
    Feuille {},
}

/// Conditions de calcul : ce que le pont a demandé à la DLL, et ce qui a été
/// relu sur l'instrument. Les deux sont distincts : une condition n'est
/// « observée » qu'après un appel vérifié qui la rend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calcul {
    /// Description en clair, telle que le pont l'a écrite.
    pub libelle: String,
    pub demande: Info<ConditionsCalcul>,
    pub observe: Info<ConditionsCalcul>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionsCalcul {
    /// Condition de mesure de chacun des trois spectres d'une plage, dans
    /// l'ordre `m0`, `m1`, `m2`.
    pub conditions_spectres: [Info<ConditionMesure>; 3],
    /// Longueur d'onde de la première valeur d'un spectre et pas.
    pub longueurs_onde: Info<Echantillonnage>,
    /// Illuminant et observateur des Lab.
    pub illuminant_lab: Info<Illuminant>,
    pub observateur_lab: Info<Observateur>,
}

/// Condition de mesure ISO 13655 (GLOSSARY : Condition de mesure).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionMesure {
    M0,
    M1,
    M2,
}

/// Valeur n° i d'un spectre : `debut_nm + i × pas_nm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Echantillonnage {
    pub debut_nm: u32,
    pub pas_nm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Illuminant {
    D50,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observateur {
    #[serde(rename = "2_degres")]
    DeuxDegres,
}

/// Ce que le pont atteste sur une mesure (ADR 0005, GLOSSARY : Provenance).
/// Posée par le pont, jamais reconstituée par l'application : une fois dans
/// une [`Mesure`], elle ne se lit plus qu'en lecture seule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub instrument: InstrumentMesurant,
    /// Les trois nombres de `FDX_GetSDKVersion`, tels quels.
    pub version_sdk: [u32; 3],
    /// SHA-256 de la DLL chargée.
    pub empreinte_dll: Info<Empreinte>,
    pub version_pont: String,
    /// « x86 » ou « x86_64 ».
    pub architecture: String,
    /// Fin de la mesure, heure de l'ordinateur.
    pub horodatage: Horodatage,
    /// Dernier étalonnage réussi de la session.
    pub etalonnage: Info<Horodatage>,
    pub geometrie: Geometrie,
    pub calcul: Calcul,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentMesurant {
    pub modele: String,
    pub numero_serie: u32,
    pub micrologiciel: String,
    pub code_produit: String,
}

/// Version du format conservable des mesures ; tout autre nom est refusé.
pub const FORMAT_MESURE: &str = "myiro-libre/mesure/1";

/// Modèle du MYIRO-1 dans la provenance.
pub const MODELE_MYIRO1: &str = "MYIRO-1";

/// Valeurs par spectre et données brutes par plage exigées du MYIRO-1 ; les
/// autres instruments ne sont tenus qu'à des plages cohérentes entre elles.
const DIMENSIONS_MYIRO1: (usize, usize) = (36, 152);

/// Une mesure : ses plages et sa provenance, vérifiées ensemble.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Mesure {
    format: &'static str,
    plages: Vec<Plage>,
    // En boîte : garde petite la réponse `mesure` du protocole.
    provenance: Box<Provenance>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MesureLue {
    #[allow(dead_code)]
    format: String,
    plages: Vec<Plage>,
    provenance: Provenance,
}

impl<'de> Deserialize<'de> for Mesure {
    /// Le numéro de format est lu en premier : une version non prise en
    /// charge est refusée comme telle, avant tout examen de son contenu.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let valeur = serde_json::Value::deserialize(d)?;
        verifier_format(&valeur).map_err(D::Error::custom)?;
        let lue: MesureLue = serde_json::from_value(valeur).map_err(D::Error::custom)?;
        Mesure::new(lue.plages, lue.provenance).map_err(D::Error::custom)
    }
}

fn verifier_format(valeur: &serde_json::Value) -> Result<(), ErreurMesure> {
    match valeur.get("format") {
        Some(serde_json::Value::String(f)) if f == FORMAT_MESURE => Ok(()),
        Some(serde_json::Value::String(f)) => erreur(format!(
            "format de mesure « {f} » non pris en charge (attendu {FORMAT_MESURE})"
        )),
        _ => erreur(format!("mesure sans format (attendu {FORMAT_MESURE})")),
    }
}

impl Mesure {
    pub fn new(plages: Vec<Plage>, provenance: Provenance) -> Result<Self, ErreurMesure> {
        verifier_provenance(&provenance)?;
        let Some(premiere) = plages.first() else {
            return erreur("mesure sans plage");
        };
        if matches!(provenance.geometrie, Geometrie::Ponctuelle {}) && plages.len() != 1 {
            return erreur(format!(
                "{} plages pour une mesure ponctuelle",
                plages.len()
            ));
        }
        let (spectre, brutes) = (premiere.m0.len(), premiere.brutes.len());
        if let Some((n, p)) = plages
            .iter()
            .enumerate()
            .find(|(_, p)| p.m0.len() != spectre || p.brutes.len() != brutes)
        {
            return erreur(format!(
                "plage n° {} : {} valeurs par spectre et {} données brutes, au lieu de {spectre} et {brutes}",
                n + 1,
                p.m0.len(),
                p.brutes.len()
            ));
        }
        if provenance.instrument.modele == MODELE_MYIRO1 {
            let (attendu_spectre, attendu_brutes) = DIMENSIONS_MYIRO1;
            if spectre != attendu_spectre {
                return erreur(format!(
                    "MYIRO-1 : {spectre} valeurs par spectre au lieu de {attendu_spectre}"
                ));
            }
            if brutes != attendu_brutes {
                return erreur(format!(
                    "MYIRO-1 : {brutes} données brutes au lieu de {attendu_brutes}"
                ));
            }
        }
        Ok(Mesure {
            format: FORMAT_MESURE,
            plages,
            provenance: Box::new(provenance),
        })
    }

    pub fn plages(&self) -> &[Plage] {
        &self.plages
    }

    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

fn verifier_provenance(p: &Provenance) -> Result<(), ErreurMesure> {
    for (quoi, texte) in [
        ("modèle", &p.instrument.modele),
        ("version du pont", &p.version_pont),
        ("architecture", &p.architecture),
        ("libellé du calcul", &p.calcul.libelle),
    ] {
        if texte.trim().is_empty() {
            return erreur(format!("provenance : {quoi} vide"));
        }
    }
    for conditions in [&p.calcul.demande, &p.calcul.observe] {
        if let Some(ConditionsCalcul { longueurs_onde, .. }) = conditions.valeur() {
            if longueurs_onde.valeur().is_some_and(|e| e.pas_nm == 0) {
                return erreur("provenance : pas de longueur d'onde nul");
            }
        }
    }
    Ok(())
}

/// Écrit une mesure au format conservable (une ligne JSON).
pub fn ecrire_mesure(mesure: &Mesure) -> String {
    serde_json::to_string(mesure).expect("une mesure se sérialise toujours")
}

/// Relit une mesure conservée : format [`FORMAT_MESURE`], ou réponse `mesure`
/// du format initial (voir `docs/formats/mesure.md`). Toute autre version est
/// refusée en clair ; rien n'est deviné.
pub fn lire_mesure(texte: &str) -> Result<Mesure, ErreurMesure> {
    let valeur: serde_json::Value =
        serde_json::from_str(texte).map_err(|e| ErreurMesure(e.to_string()))?;
    if est_mesure_initiale(&valeur) {
        return mesure_initiale(valeur);
    }
    verifier_format(&valeur)?;
    serde_json::from_value(valeur).map_err(|e| ErreurMesure(e.to_string()))
}

/// Réponse `mesure` du format initial : pas de numéro de format, plages et
/// provenance à plat dans la réponse.
pub(crate) fn est_mesure_initiale(valeur: &serde_json::Value) -> bool {
    valeur.get("rep").and_then(|r| r.as_str()) == Some("mesure")
        && valeur.get("mesure").is_none()
        && valeur.get("format").is_none()
}

/// Réponse `mesure` du pont 0.1.0, champ pour champ.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MesureInitiale {
    #[allow(dead_code)]
    rep: String,
    plages: Vec<Plage>,
    sens: u32,
    provenance: ProvenanceInitiale,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvenanceInitiale {
    instrument: InstrumentMesurant,
    version_sdk: [u32; 3],
    empreinte_dll: Option<String>,
    version_pont: String,
    architecture: String,
    horodatage: Horodatage,
    etalonnage: Option<Horodatage>,
    geometrie: String,
    calcul: String,
}

/// Reprend une mesure du format initial. Ce que le pont 0.1.0 avait observé
/// lui-même (empreinte calculée, date d'étalonnage) est repris tel quel ; le
/// texte libre du calcul est gardé comme libellé, sans rien en déduire.
pub(crate) fn mesure_initiale(valeur: serde_json::Value) -> Result<Mesure, ErreurMesure> {
    let lue: MesureInitiale = serde_json::from_value(valeur)
        .map_err(|e| ErreurMesure(format!("mesure du format initial : {e}")))?;
    let p = lue.provenance;
    let geometrie = match p.geometrie.as_str() {
        // Le pont 0.1.0 écrivait un sens 0 qu'il n'avait pas lu en ponctuelle.
        "ponctuelle" => Geometrie::Ponctuelle {},
        "bande" => Geometrie::Bande { sens: lue.sens },
        autre => {
            return erreur(format!(
                "mesure du format initial : géométrie « {autre} » inconnue"
            ))
        }
    };
    let empreinte_dll = match p.empreinte_dll {
        Some(texte) => Info::Confirmee(Empreinte::new(texte)?),
        None => Info::Inconnue,
    };
    let provenance = Provenance {
        instrument: p.instrument,
        version_sdk: p.version_sdk,
        empreinte_dll,
        version_pont: p.version_pont,
        architecture: p.architecture,
        horodatage: p.horodatage,
        etalonnage: p.etalonnage.map_or(Info::Inconnue, Info::Confirmee),
        geometrie,
        calcul: Calcul {
            libelle: p.calcul,
            demande: Info::Inconnue,
            observe: Info::Inconnue,
        },
    };
    Mesure::new(lue.plages, provenance)
        .map_err(|e| ErreurMesure(format!("mesure du format initial : {e}")))
}
