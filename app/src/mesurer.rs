//! Tâche Mesurer (ticket #7) : les mesures ponctuelles de la séance, la plus
//! récente en haut, leurs valeurs calculées par la crate `colorimetrie` à
//! partir du spectre, et leur rangement dans la condition d'impression
//! choisie. Rien ici ne dépend de Tauri.
//!
//! Une mesure lue par l'instrument n'est jamais perdue : elle entre dans la
//! séance avant d'être rangée, et y reste si la bibliothèque la refuse.

use bibliotheque::{ConditionImpression, IdMesure};
use pont_protocole::{ConditionMesure, Echantillonnage, Horodatage, Info, Mesure};
use serde::Serialize;

use crate::instrument::{Gestes, Instrument, MesureAcquise};
use crate::pont::Pont;
use crate::textes::{texte, Langue};

/// Clé du catalogue d'une mesure que la bibliothèque n'a pas pu ranger.
pub const ERREUR_RANGEMENT: &str = "mesurer.erreur.rangement";

/// Une mesure de la séance.
struct MesureDeSeance {
    numero: usize,
    nom: String,
    acquise: MesureAcquise,
    condition: ConditionImpression,
    rangee: Option<IdMesure>,
}

/// Les mesures faites depuis le lancement de l'application, dans l'ordre.
#[derive(Default)]
pub struct Seance {
    mesures: Vec<MesureDeSeance>,
}

/// Ce que la feuille Mesurer et les détails montrent d'une mesure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FicheMesure {
    /// Numéro de la mesure dans la séance (1 pour la première).
    pub numero: usize,
    pub nom: String,
    /// Fin de la mesure, telle que le pont l'a datée.
    pub horodatage: String,
    pub condition_impression: String,
    /// Clé du catalogue si la mesure n'est pas rangée dans la bibliothèque.
    pub erreur_rangement: Option<&'static str>,
    pub modele: String,
    pub numero_serie: u32,
    pub micrologiciel: String,
    pub etalonnage: Info<Horodatage>,
    /// Les trois spectres de la plage, dans l'ordre où le pont les a rendus.
    pub spectres: Vec<FicheSpectre>,
}

/// Un spectre de la mesure : sa condition de mesure, telle que le pont l'a
/// dite, et ses valeurs calculées par notre colorimétrie (D50, 2°).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FicheSpectre {
    pub condition: Info<ConditionMesure>,
    /// Inconnues si les longueurs d'onde du spectre ne sont pas établies ou
    /// si le calcul est impossible.
    pub valeurs: Option<Valeurs>,
}

/// Valeurs d'un spectre, écrites à deux décimales dans la langue de l'écran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Valeurs {
    pub l: String,
    pub a: String,
    pub b: String,
    pub c: String,
    /// Teinte en degrés ; inconnue pour un gris parfait.
    pub h: Option<String>,
    pub x: String,
    pub y: String,
    pub z: String,
}

/// Longueurs d'onde sur lesquelles la crate `colorimetrie` calcule : 380 à
/// 730 nm par 10 nm.
const ECHANTILLONNAGE: Echantillonnage = Echantillonnage {
    debut_nm: 380,
    pas_nm: 10,
};

/// Nombre à deux décimales, avec la virgule en français. Un zéro négatif
/// s'écrit comme un zéro.
pub fn decimal(valeur: f64, langue: Langue) -> String {
    let mut texte = format!("{valeur:.2}");
    if texte.trim_start_matches('-').bytes().all(|o| o == b'0' || o == b'.') {
        texte = texte.trim_start_matches('-').to_string();
    }
    match langue {
        Langue::Francais => texte.replace('.', ","),
        Langue::Anglais => texte,
    }
}

/// Lab, LCH et XYZ d'un spectre (D50, 2°), ou `None` si l'un est inconnu.
fn valeurs(spectre: &[f32], langue: Langue) -> Option<Valeurs> {
    let spectre: Vec<f64> = spectre.iter().map(|&v| f64::from(v)).collect();
    let xyz = colorimetrie::spectre_vers_xyz(&spectre).ok()?;
    let lab = colorimetrie::xyz_vers_lab(xyz, colorimetrie::blanc_d50()).ok()?;
    let lch = colorimetrie::lab_vers_lch(lab).ok()?;
    let d = |v| decimal(v, langue);
    Some(Valeurs {
        l: d(lab.l),
        a: d(lab.a),
        b: d(lab.b),
        c: d(lch.c),
        h: lch.h.ok().map(d),
        x: d(xyz.x),
        y: d(xyz.y),
        z: d(xyz.z),
    })
}

impl Seance {
    /// Mesure ponctuelle, puis rangement dans la condition d'impression
    /// choisie par `ranger`. Rend le numéro de la nouvelle mesure, ou `None`
    /// si rien n'a été mesuré (le module `instrument` dit pourquoi).
    pub fn mesurer<P: Pont>(
        &mut self,
        instrument: &mut Instrument<P>,
        gestes: &mut impl Gestes,
        condition: &ConditionImpression,
        ranger: impl FnOnce(&Mesure) -> Result<IdMesure, String>,
        langue: Langue,
    ) -> Option<usize> {
        let acquise = instrument.mesurer_ponctuelle(gestes)?;
        let numero = self.mesures.len() + 1;
        // La mesure entre dans la séance avant d'être rangée.
        self.mesures.push(MesureDeSeance {
            numero,
            nom: texte(langue, "mesurer.nom_defaut").replace("{n}", &numero.to_string()),
            acquise,
            condition: condition.clone(),
            rangee: None,
        });
        let derniere = self.mesures.last_mut().expect("mesure ajoutée");
        match ranger(&derniere.acquise.mesure) {
            Ok(id) => derniere.rangee = Some(id),
            Err(detail) => eprintln!("rangement de la mesure {numero} : {detail}"),
        }
        Some(numero)
    }

    /// La mesure n° `numero` de la séance, telle que le pont l'a rendue.
    pub fn mesure(&self, numero: usize) -> Option<&Mesure> {
        self.trouver(numero).map(|m| &m.acquise.mesure)
    }

    /// Renomme une mesure de la séance ; le nom est débarrassé de ses espaces
    /// de début et de fin. Les refus sont des clés du catalogue.
    pub fn renommer(&mut self, numero: usize, nom: &str) -> Result<(), &'static str> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err("bibliotheque.erreur.nom_vide");
        }
        let mesure = self
            .mesures
            .iter_mut()
            .find(|m| m.numero == numero)
            .ok_or("bibliotheque.erreur.autre")?;
        mesure.nom = nom.to_string();
        Ok(())
    }

    fn trouver(&self, numero: usize) -> Option<&MesureDeSeance> {
        self.mesures.iter().find(|m| m.numero == numero)
    }

    /// Les mesures de la séance, la plus récente d'abord.
    pub fn fiches(&self, langue: Langue) -> Vec<FicheMesure> {
        self.mesures.iter().rev().map(|m| fiche(m, langue)).collect()
    }
}

fn fiche(m: &MesureDeSeance, langue: Langue) -> FicheMesure {
    let p = m.acquise.mesure.provenance();
    // Ce que le pont a demandé ; une demande supposée rend chaque condition
    // supposée au mieux. Jamais devinée d'après la place du spectre.
    let (conditions, echantillonnage) = match &p.calcul.demande {
        Info::Confirmee(c) => (c.conditions_spectres.clone(), c.longueurs_onde.valeur()),
        Info::Supposee(c) => (
            c.conditions_spectres.clone().map(|i| match i {
                Info::Confirmee(v) | Info::Supposee(v) => Info::Supposee(v),
                Info::Inconnue => Info::Inconnue,
            }),
            c.longueurs_onde.valeur(),
        ),
        Info::Inconnue => ([Info::Inconnue, Info::Inconnue, Info::Inconnue], None),
    };
    let calculable = echantillonnage == Some(&ECHANTILLONNAGE);
    // Une mesure ponctuelle a exactement une plage (format `myiro-libre/mesure/1`).
    let plage = &m.acquise.mesure.plages()[0];
    let spectres = [plage.m0(), plage.m1(), plage.m2()]
        .into_iter()
        .zip(conditions)
        .map(|(spectre, condition)| FicheSpectre {
            condition,
            valeurs: calculable.then(|| valeurs(spectre, langue)).flatten(),
        })
        .collect();
    FicheMesure {
        numero: m.numero,
        nom: m.nom.clone(),
        horodatage: p.horodatage.texte().to_string(),
        condition_impression: m.condition.nom.clone(),
        erreur_rangement: m.rangee.is_none().then_some(ERREUR_RANGEMENT),
        modele: p.instrument.modele.clone(),
        numero_serie: p.instrument.numero_serie,
        micrologiciel: p.instrument.micrologiciel.clone(),
        etalonnage: p.etalonnage.clone(),
        spectres,
    }
}
