//! Tâche Mesurer (ticket #7) : les mesures ponctuelles de la séance, la plus
//! récente en haut, leurs valeurs calculées par la crate `colorimetrie` à
//! partir du spectre, et leur rangement dans la condition d'impression
//! choisie. Rien ici ne dépend de Tauri.
//!
//! Une mesure lue par l'instrument n'est jamais perdue : elle entre dans la
//! séance avant d'être rangée, et y reste si la bibliothèque la refuse.
//!
//! Une mesure rangée peut être désignée couleur de référence (ticket #8) :
//! les autres affichent leur écart à elle (ΔE00, ΔC, ΔH) et un verdict selon
//! le seuil choisi, conservé avec la référence dans la bibliothèque.

use bibliotheque::{Bibliotheque, ConditionImpression, IdMesure};
use colorimetrie::{Seuil, Verdict};
use pont_protocole::{ConditionMesure, Declenchement, Echantillonnage, Horodatage, Info, Mesure};
use serde::Serialize;

use crate::instrument::{Gestes, Instrument, MesureAcquise};
use crate::pont::Pont;
use crate::textes::{texte, Langue};

/// Clé du catalogue d'une mesure que la bibliothèque n'a pas pu ranger.
pub const ERREUR_RANGEMENT: &str = "mesurer.erreur.rangement";

/// Pourquoi une mesure ne part pas, avant même de demander le geste. Les
/// textes de l'écran sont `<code>.cause` et `<code>.action` du catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusMesure {
    /// Aucun instrument ouvert.
    SansInstrument,
    /// La condition d'impression choisie n'est plus dans la bibliothèque.
    ConditionAbsente,
    /// La bibliothèque ne s'est pas ouverte : rien ne pourrait être rangé.
    BibliothequeFermee,
}

impl RefusMesure {
    pub fn code(self) -> &'static str {
        match self {
            RefusMesure::SansInstrument => "mesurer.erreur.sans_instrument",
            RefusMesure::ConditionAbsente => "mesurer.erreur.condition_absente",
            RefusMesure::BibliothequeFermee => "mesurer.erreur.bibliotheque_fermee",
        }
    }
}

/// La condition d'impression `id` parmi celles de la bibliothèque.
pub fn choisir_condition(
    conditions: Result<Vec<ConditionImpression>, String>,
    id: i64,
) -> Result<ConditionImpression, RefusMesure> {
    conditions
        .map_err(|_| RefusMesure::BibliothequeFermee)?
        .into_iter()
        .find(|c| c.id.0 == id)
        .ok_or(RefusMesure::ConditionAbsente)
}

/// Une mesure de la séance.
struct MesureDeSeance {
    numero: usize,
    nom: String,
    mesure: Mesure,
    condition: ConditionImpression,
    rangee: Option<IdMesure>,
}

/// La couleur de référence de la séance et son seuil (aucun tant que
/// l'utilisateur n'en a pas fixé).
#[derive(Clone, Copy)]
struct ReferenceDeSeance {
    numero: usize,
    seuil: Option<Seuil>,
}

/// Les mesures faites depuis le lancement de l'application, dans l'ordre.
pub struct Seance {
    mesures: Vec<MesureDeSeance>,
    reference: Option<ReferenceDeSeance>,
    /// « Mesure : automatique / manuelle » ; automatique par défaut (ticket #51).
    declenchement: Declenchement,
}

impl Default for Seance {
    fn default() -> Self {
        Seance {
            mesures: Vec::new(),
            reference: None,
            declenchement: Declenchement::Automatique,
        }
    }
}

/// Choix « Mesure » tel qu'il est retenu d'une fois sur l'autre.
pub fn ecrire_declenchement(declenchement: Declenchement) -> &'static str {
    match declenchement {
        Declenchement::Automatique => "automatique",
        Declenchement::Manuel => "manuel",
    }
}

/// Retient le choix dans `fichier` (dossier créé au besoin). Un échec est
/// rendu : le choix vaut alors pour la séance seulement, et l'écran le dit.
pub fn retenir_declenchement(
    fichier: &std::path::Path,
    declenchement: Declenchement,
) -> std::io::Result<()> {
    if let Some(dossier) = fichier.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    std::fs::write(fichier, ecrire_declenchement(declenchement))
}

/// Relit le choix retenu ; absent ou illisible, c'est l'automatique,
/// demandé par le mainteneur le 9 octobre 2026.
pub fn lire_declenchement(texte: Option<&str>) -> Declenchement {
    match texte.map(str::trim) {
        Some("manuel") => Declenchement::Manuel,
        _ => Declenchement::Automatique,
    }
}

/// Là où la couleur de référence et son seuil sont conservés : la
/// bibliothèque. Les refus sont des phrases techniques, pour la console.
pub trait ReferencesConservees {
    /// Désigne la mesure, ou change son seuil.
    fn designer(&self, id: IdMesure, seuil: Option<f64>) -> Result<(), String>;
    /// Remplace une référence par une autre, sans seuil, d'un bloc.
    fn remplacer(&self, ancienne: IdMesure, nouvelle: IdMesure) -> Result<(), String>;
    fn retirer(&self, id: IdMesure) -> Result<(), String>;
    /// La couleur de référence conservée au plus grand numéro de mesure,
    /// pour la reprendre au lancement ou après une restauration.
    fn reference_conservee(&self) -> Result<Option<ReferenceReprise>, String>;
}

/// Une couleur de référence conservée par la bibliothèque, telle qu'elle
/// revient dans une nouvelle séance.
#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceReprise {
    pub id: IdMesure,
    pub mesure: Mesure,
    pub nom: Option<String>,
    pub condition: ConditionImpression,
    pub seuil: Option<f64>,
}

impl ReferencesConservees for Bibliotheque {
    fn designer(&self, id: IdMesure, seuil: Option<f64>) -> Result<(), String> {
        self.designer_reference(id, seuil)
            .map_err(|e| e.to_string())
    }

    fn remplacer(&self, ancienne: IdMesure, nouvelle: IdMesure) -> Result<(), String> {
        self.remplacer_reference(ancienne, nouvelle)
            .map_err(|e| e.to_string())
    }

    fn retirer(&self, id: IdMesure) -> Result<(), String> {
        self.retirer_reference(id).map_err(|e| e.to_string())
    }

    fn reference_conservee(&self) -> Result<Option<ReferenceReprise>, String> {
        reference_conservee(self).map_err(|e| e.to_string())
    }
}

/// La couleur de référence de la bibliothèque au plus grand numéro de mesure.
pub fn reference_conservee(
    biblio: &Bibliotheque,
) -> Result<Option<ReferenceReprise>, bibliotheque::ErreurBibliotheque> {
    let Some(reference) = biblio.references()?.into_iter().max_by_key(|r| r.mesure) else {
        return Ok(None);
    };
    let enregistree = biblio.mesure(reference.mesure)?;
    let condition = biblio
        .conditions()?
        .into_iter()
        .find(|c| c.id == enregistree.condition)
        .ok_or(bibliotheque::ErreurBibliotheque::ConditionInconnue(
            enregistree.condition,
        ))?;
    Ok(Some(ReferenceReprise {
        id: reference.mesure,
        mesure: enregistree.mesure,
        nom: enregistree.nom,
        condition,
        seuil: reference.seuil,
    }))
}

/// La couleur de référence, telle que la feuille Mesurer la montre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FicheReference {
    pub numero: usize,
    pub nom: String,
    /// Seuil ΔE00 écrit dans la langue de l'écran ; aucun s'il n'est pas fixé.
    pub seuil: Option<String>,
}

/// Verdict d'un écart à la référence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictEcart {
    Conforme,
    ProcheDeLaLimite,
    HorsTolerance,
    /// Aucun seuil fixé : pas de verdict.
    SeuilNonFixe,
    /// Conditions de mesure différentes : ni écart ni verdict.
    NonComparable,
    /// Écart inconnu (valeurs d'une des deux mesures inconnues).
    Inconnu,
}

/// Les deux spectres comparés ont-ils la même condition de mesure ?
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparaison {
    /// Même condition, confirmée pour les deux mesures.
    MemeCondition,
    /// Conditions connues et différentes : l'écart ne veut pas dire grand-chose.
    ConditionDifferente,
    /// La condition d'au moins une des deux mesures n'est pas confirmée.
    NonConfirmee,
}

/// Écart d'un spectre à celui de la couleur de référence, écrit à deux
/// décimales dans la langue de l'écran. ΔE00 (CIEDE2000) ; ΔC et ΔH
/// (CIELAB), signés : positifs quand la mesure est plus saturée, ou que sa
/// teinte tourne dans le sens direct. Inconnus si un des Lab l'est.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FicheEcart {
    pub delta_e00: Option<String>,
    pub delta_c: Option<String>,
    pub delta_h: Option<String>,
    pub verdict: VerdictEcart,
    pub comparaison: Comparaison,
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
    /// Modèle et micrologiciel ; `None` si le pont les a rendus vides : le
    /// cartouche écrit alors « inconnu », jamais une case vide.
    pub modele: Option<String>,
    pub numero_serie: u32,
    pub micrologiciel: Option<String>,
    pub etalonnage: Info<Horodatage>,
    /// Les trois spectres de la plage, dans l'ordre où le pont les a rendus.
    pub spectres: Vec<FicheSpectre>,
    /// La mesure est la couleur de référence de la séance.
    pub reference: bool,
}

/// Un spectre de la mesure : sa condition de mesure, telle que le pont l'a
/// dite, et ses valeurs calculées par notre colorimétrie (D50, 2°).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FicheSpectre {
    pub condition: Info<ConditionMesure>,
    /// Inconnues si les longueurs d'onde du spectre ne sont pas établies ou
    /// si le calcul est impossible.
    pub valeurs: Option<Valeurs>,
    /// Écart à la couleur de référence ; aucun sans référence, ou pour la
    /// référence elle-même.
    pub ecart: Option<FicheEcart>,
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
    /// Couleur à l'écran, `#rrggbb` (sRGB, crate `colorimetrie`).
    pub ecran: String,
    /// Couleur hors du gamut sRGB, ramenée dedans : l'écran n'en montre
    /// qu'une approximation.
    pub approchee: bool,
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
    let mut texte = format!("{:.2}", deux_decimales(valeur));
    if texte
        .trim_start_matches('-')
        .bytes()
        .all(|o| o == b'0' || o == b'.')
    {
        texte = texte.trim_start_matches('-').to_string();
    }
    match langue {
        Langue::Francais => texte.replace('.', ","),
        Langue::Anglais => texte,
    }
}

/// Valeur arrondie à deux décimales, telle que [`decimal`] l'écrit : le
/// verdict et le seuil passent par là, pour juger exactement ce qui est
/// affiché.
pub fn deux_decimales(valeur: f64) -> f64 {
    format!("{valeur:.2}").parse().unwrap_or(valeur)
}

/// Lab d'un spectre (D50, 2°), ou `None` s'il est inconnu.
fn lab(spectre: &[f32]) -> Option<colorimetrie::Lab> {
    let spectre: Vec<f64> = spectre.iter().map(|&v| f64::from(v)).collect();
    colorimetrie::spectre_vers_lab(&spectre).ok()
}

/// Lab, LCH et XYZ d'un spectre (D50, 2°), ou `None` si l'un est inconnu.
fn valeurs(spectre: &[f32], langue: Langue) -> Option<Valeurs> {
    let spectre: Vec<f64> = spectre.iter().map(|&v| f64::from(v)).collect();
    let xyz = colorimetrie::spectre_vers_xyz(&spectre).ok()?;
    let lab = colorimetrie::xyz_vers_lab(xyz, colorimetrie::blanc_d50()).ok()?;
    let lch = colorimetrie::lab_vers_lch(lab).ok()?;
    let ecran = colorimetrie::lab_vers_srgb(lab).ok()?;
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
        ecran: ecran.hexadecimal(),
        approchee: ecran.ramenee,
    })
}

impl Seance {
    pub fn declenchement(&self) -> Declenchement {
        self.declenchement
    }

    /// En automatique, « Mesurer » fait partir la mesure ; en manuel, c'est
    /// le bouton de l'instrument.
    pub fn choisir_declenchement(&mut self, declenchement: Declenchement) {
        self.declenchement = declenchement;
    }

    /// Mesure ponctuelle, puis rangement dans la condition d'impression
    /// choisie par `ranger`, qui reçoit la mesure et son nom. Rend le numéro
    /// de la nouvelle mesure, ou `None` si rien n'a été mesuré (le module
    /// `instrument` dit pourquoi).
    pub fn mesurer<P: Pont>(
        &mut self,
        instrument: &mut Instrument<P>,
        gestes: &mut impl Gestes,
        condition: &ConditionImpression,
        ranger: impl FnOnce(&Mesure, &str) -> Result<IdMesure, String>,
        langue: Langue,
    ) -> Option<usize> {
        let acquise: MesureAcquise =
            instrument.mesurer_ponctuelle_avec(gestes, self.declenchement)?;
        Some(self.ajouter(acquise.mesure, condition, ranger, langue))
    }

    /// Ajoute à la séance une mesure ponctuelle déjà lue, puis la range
    /// comme [`Seance::mesurer`]. Rend son numéro.
    pub fn ajouter(
        &mut self,
        mesure: Mesure,
        condition: &ConditionImpression,
        ranger: impl FnOnce(&Mesure, &str) -> Result<IdMesure, String>,
        langue: Langue,
    ) -> usize {
        let numero = self.mesures.len() + 1;
        // La mesure entre dans la séance avant d'être rangée.
        self.mesures.push(MesureDeSeance {
            numero,
            nom: texte(langue, "mesurer.nom_defaut").replace("{n}", &numero.to_string()),
            mesure,
            condition: condition.clone(),
            rangee: None,
        });
        let derniere = self.mesures.last_mut().expect("mesure ajoutée");
        match ranger(&derniere.mesure, &derniere.nom) {
            Ok(id) => derniere.rangee = Some(id),
            Err(detail) => eprintln!("rangement de la mesure {numero} : {detail}"),
        }
        numero
    }

    /// La mesure n° `numero` de la séance, telle que le pont l'a rendue.
    pub fn mesure(&self, numero: usize) -> Option<&Mesure> {
        self.trouver(numero).map(|m| &m.mesure)
    }

    /// Désigne la mesure n° `numero` couleur de référence, sans seuil. Elle
    /// doit être rangée : la référence est conservée dans la bibliothèque.
    /// L'ancienne référence, s'il y en a une, est retirée. Si la
    /// bibliothèque refuse, rien ne change. Les refus sont des clés du
    /// catalogue.
    pub fn designer_reference(
        &mut self,
        numero: usize,
        conserver: &impl ReferencesConservees,
    ) -> Result<(), &'static str> {
        if self.reference.is_some_and(|r| r.numero == numero) {
            return Ok(()); // déjà la référence : son seuil reste
        }
        let id = self
            .trouver(numero)
            .ok_or("bibliotheque.erreur.autre")?
            .rangee
            .ok_or("mesurer.reference.non_rangee")?;
        // L'ancienne est remplacée d'un bloc : jamais deux références, et
        // rien ne change si la bibliothèque refuse.
        let ancienne = self
            .reference
            .and_then(|a| self.trouver(a.numero))
            .and_then(|m| m.rangee);
        match ancienne {
            Some(ancienne) => conserver.remplacer(ancienne, id),
            None => conserver.designer(id, None),
        }
        .map_err(|detail| {
            eprintln!("référence {numero} : {detail}");
            "bibliotheque.erreur.autre"
        })?;
        self.reference = Some(ReferenceDeSeance {
            numero,
            seuil: None,
        });
        Ok(())
    }

    /// La séance n'a plus de couleur de référence. Si la bibliothèque
    /// refuse, rien ne change.
    pub fn retirer_reference(
        &mut self,
        conserver: &impl ReferencesConservees,
    ) -> Result<(), &'static str> {
        let Some(reference) = self.reference else {
            return Ok(());
        };
        if let Some(id) = self.trouver(reference.numero).and_then(|m| m.rangee) {
            conserver.retirer(id).map_err(|detail| {
                eprintln!("retrait de la référence {} : {detail}", reference.numero);
                "bibliotheque.erreur.autre"
            })?;
        }
        self.reference = None;
        Ok(())
    }

    /// Règle le seuil ΔE00 de la référence, tel que l'utilisateur l'a écrit
    /// (virgule ou point) ; un texte vide retire le seuil. Un seuil qui n'est
    /// pas un nombre strictement positif est refusé et rien ne change.
    pub fn regler_seuil(
        &mut self,
        texte: &str,
        conserver: &impl ReferencesConservees,
    ) -> Result<(), &'static str> {
        let reference = self.reference.ok_or("mesurer.reference.aucune")?;
        let texte = texte.trim();
        let seuil = if texte.is_empty() {
            None
        } else {
            let valeur: f64 = texte
                .replace(',', ".")
                .parse()
                .map_err(|_| "mesurer.reference.seuil_invalide")?;
            // Arrondi comme les écarts affichés : écrit, conservé et jugé à l'identique.
            Some(Seuil::new(deux_decimales(valeur)).ok_or("mesurer.reference.seuil_invalide")?)
        };
        let id = self
            .trouver(reference.numero)
            .and_then(|m| m.rangee)
            .ok_or("bibliotheque.erreur.autre")?;
        conserver
            .designer(id, seuil.map(Seuil::valeur))
            .map_err(|detail| {
                eprintln!("seuil de la référence {} : {detail}", reference.numero);
                "bibliotheque.erreur.autre"
            })?;
        self.reference = Some(ReferenceDeSeance { seuil, ..reference });
        Ok(())
    }

    /// Au lancement : reprend dans la séance, encore vide, la couleur de
    /// référence conservée par la bibliothèque, avec son nom et son seuil.
    /// Elle n'est pas rangée une seconde fois. Sans référence conservée,
    /// rien ne change.
    pub fn reprendre_reference(
        &mut self,
        conserver: &impl ReferencesConservees,
        langue: Langue,
    ) -> Result<(), &'static str> {
        if !self.mesures.is_empty() {
            return Ok(());
        }
        self.charger_reference(conserver, langue)
    }

    /// Après une restauration de la bibliothèque : la séance oublie sa
    /// référence, qui désignait une mesure de l'ancienne base, et reprend
    /// celle de la base restaurée s'il y en a une. Une mesure de la séance
    /// identique à la référence restaurée, au même numéro, est reprise telle
    /// quelle plutôt qu'ajoutée une seconde fois.
    pub fn suivre_restauration(
        &mut self,
        conserver: &impl ReferencesConservees,
        langue: Langue,
    ) -> Result<(), &'static str> {
        self.reference = None;
        self.charger_reference(conserver, langue)
    }

    fn charger_reference(
        &mut self,
        conserver: &impl ReferencesConservees,
        langue: Langue,
    ) -> Result<(), &'static str> {
        let reprise = conserver.reference_conservee().map_err(|detail| {
            eprintln!("reprise de la référence : {detail}");
            "bibliotheque.erreur.autre"
        })?;
        let Some(reprise) = reprise else {
            return Ok(());
        };
        let deja = self
            .mesures
            .iter()
            .find(|m| m.rangee == Some(reprise.id) && m.mesure == reprise.mesure)
            .map(|m| m.numero);
        let numero = match deja {
            Some(numero) => numero,
            None => {
                let numero = self.ajouter(
                    reprise.mesure,
                    &reprise.condition,
                    |_, _| Ok(reprise.id),
                    langue,
                );
                if let Some(nom) = reprise.nom {
                    let _ = self.renommer(numero, &nom, |_, _| Ok(()));
                }
                numero
            }
        };
        self.reference = Some(ReferenceDeSeance {
            numero,
            seuil: reprise.seuil.and_then(Seuil::new),
        });
        Ok(())
    }

    /// La couleur de référence de la séance, s'il y en a une.
    pub fn reference(&self, langue: Langue) -> Option<FicheReference> {
        let reference = self.reference?;
        let mesure = self.trouver(reference.numero)?;
        Some(FicheReference {
            numero: reference.numero,
            nom: mesure.nom.clone(),
            seuil: reference.seuil.map(|s| decimal(s.valeur(), langue)),
        })
    }

    /// Renomme une mesure de la séance ; le nom est débarrassé de ses espaces
    /// de début et de fin. Une mesure rangée est d'abord renommée dans la
    /// bibliothèque par `renommer_rangee` : si celle-ci refuse, rien ne
    /// change. Les refus sont des clés du catalogue.
    pub fn renommer(
        &mut self,
        numero: usize,
        nom: &str,
        renommer_rangee: impl FnOnce(IdMesure, &str) -> Result<(), String>,
    ) -> Result<(), &'static str> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err("bibliotheque.erreur.nom_vide");
        }
        let mesure = self
            .mesures
            .iter_mut()
            .find(|m| m.numero == numero)
            .ok_or("bibliotheque.erreur.autre")?;
        if let Some(id) = mesure.rangee {
            renommer_rangee(id, nom).map_err(|detail| {
                eprintln!("renommage de la mesure {numero} : {detail}");
                "bibliotheque.erreur.autre"
            })?;
        }
        mesure.nom = nom.to_string();
        Ok(())
    }

    /// Range à nouveau, avec leur nom et dans leur condition d'impression,
    /// les mesures que la bibliothèque avait refusées ; les autres ne sont
    /// pas touchées. Rend le nombre de mesures toujours pas rangées.
    pub fn ranger_a_nouveau(
        &mut self,
        mut ranger: impl FnMut(&ConditionImpression, &Mesure, &str) -> Result<IdMesure, String>,
    ) -> usize {
        let mut restantes = 0;
        for m in self.mesures.iter_mut().filter(|m| m.rangee.is_none()) {
            match ranger(&m.condition, &m.mesure, &m.nom) {
                Ok(id) => m.rangee = Some(id),
                Err(detail) => {
                    eprintln!("rangement de la mesure {} : {detail}", m.numero);
                    restantes += 1;
                }
            }
        }
        restantes
    }

    fn trouver(&self, numero: usize) -> Option<&MesureDeSeance> {
        self.mesures.iter().find(|m| m.numero == numero)
    }

    /// Les mesures de la séance, la plus récente d'abord.
    pub fn fiches(&self, langue: Langue) -> Vec<FicheMesure> {
        let reference = self
            .reference
            .and_then(|r| Some((self.trouver(r.numero)?, r.seuil)));
        self.mesures
            .iter()
            .rev()
            .map(|m| fiche(m, reference, langue))
            .collect()
    }
}

/// Conditions de mesure des trois spectres, telles que le pont les a
/// demandées (une demande supposée rend chaque condition supposée au mieux),
/// et si nos tables couvrent ses longueurs d'onde.
fn conditions_spectres(mesure: &Mesure) -> ([Info<ConditionMesure>; 3], bool) {
    let p = mesure.provenance();
    // Jamais devinées d'après la place du spectre.
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
    (conditions, echantillonnage == Some(&ECHANTILLONNAGE))
}

/// Spectres de la plage unique d'une mesure ponctuelle.
fn spectres(mesure: &Mesure) -> [&[f32]; 3] {
    // Une mesure ponctuelle a exactement une plage (format `myiro-libre/mesure/1`).
    let plage = &mesure.plages()[0];
    [plage.m0(), plage.m1(), plage.m2()]
}

/// Écart du spectre n° `i` de `mesure` au spectre de même condition de
/// mesure de la référence ; à défaut, au spectre de même place, avec
/// l'avertissement qui convient.
fn ecart(
    mesure: &Mesure,
    i: usize,
    reference: &Mesure,
    seuil: Option<Seuil>,
    langue: Langue,
) -> FicheEcart {
    let (conditions, calculable) = conditions_spectres(mesure);
    let (conditions_ref, calculable_ref) = conditions_spectres(reference);
    let (j, comparaison) = match &conditions[i] {
        Info::Confirmee(c) => match conditions_ref
            .iter()
            .position(|r| *r == Info::Confirmee(*c))
        {
            Some(j) => (j, Comparaison::MemeCondition),
            None => (i, comparer(&conditions[i], &conditions_ref[i])),
        },
        autre => (i, comparer(autre, &conditions_ref[i])),
    };
    if comparaison == Comparaison::ConditionDifferente {
        // M0 contre M1 n'est pas un écart d'impression : ni chiffre ni verdict.
        return FicheEcart {
            delta_e00: None,
            delta_c: None,
            delta_h: None,
            verdict: VerdictEcart::NonComparable,
            comparaison,
        };
    }
    let labs = (calculable && calculable_ref)
        .then(|| Some((lab(spectres(reference)[j])?, lab(spectres(mesure)[i])?)))
        .flatten();
    let Some((lab_ref, lab_mes)) = labs else {
        return FicheEcart {
            delta_e00: None,
            delta_c: None,
            delta_h: None,
            verdict: VerdictEcart::Inconnu,
            comparaison,
        };
    };
    let d = |v: Result<f64, colorimetrie::Inconnu>| v.ok().map(|v| decimal(v, langue));
    let de00 = colorimetrie::delta_e00(lab_ref, lab_mes).ok();
    // Le verdict porte sur l'écart tel qu'il est écrit, à deux décimales.
    let verdict = match (de00, seuil) {
        (None, _) => VerdictEcart::Inconnu,
        (Some(_), None) => VerdictEcart::SeuilNonFixe,
        (Some(e), Some(s)) => match s.verdict(deux_decimales(e)) {
            Ok(Verdict::Conforme) => VerdictEcart::Conforme,
            Ok(Verdict::ProcheDeLaLimite) => VerdictEcart::ProcheDeLaLimite,
            Ok(Verdict::HorsTolerance) => VerdictEcart::HorsTolerance,
            Err(_) => VerdictEcart::Inconnu,
        },
    };
    FicheEcart {
        delta_e00: de00.map(|v| decimal(v, langue)),
        delta_c: d(colorimetrie::delta_c(lab_ref, lab_mes)),
        delta_h: d(colorimetrie::delta_h(lab_ref, lab_mes)),
        verdict,
        comparaison,
    }
}

/// Deux conditions de mesure qui ne sont pas toutes deux confirmées et
/// égales : différentes si elles sont connues (même supposées) et
/// différentes, non confirmées sinon.
fn comparer(a: &Info<ConditionMesure>, b: &Info<ConditionMesure>) -> Comparaison {
    match (a, b) {
        (Info::Confirmee(x), Info::Confirmee(y)) if x == y => Comparaison::MemeCondition,
        (Info::Confirmee(x) | Info::Supposee(x), Info::Confirmee(y) | Info::Supposee(y))
            if x != y =>
        {
            Comparaison::ConditionDifferente
        }
        _ => Comparaison::NonConfirmee,
    }
}

/// Texte rendu par le pont, ou `None` s'il est vide.
fn renseigne(texte: &str) -> Option<String> {
    let texte = texte.trim();
    (!texte.is_empty()).then(|| texte.to_string())
}

fn fiche(
    m: &MesureDeSeance,
    reference: Option<(&MesureDeSeance, Option<Seuil>)>,
    langue: Langue,
) -> FicheMesure {
    let p = m.mesure.provenance();
    let (conditions, calculable) = conditions_spectres(&m.mesure);
    let est_reference = reference.is_some_and(|(r, _)| r.numero == m.numero);
    let spectres = spectres(&m.mesure)
        .into_iter()
        .zip(conditions)
        .enumerate()
        .map(|(i, (spectre, condition))| FicheSpectre {
            condition,
            valeurs: calculable.then(|| valeurs(spectre, langue)).flatten(),
            ecart: reference
                .filter(|_| !est_reference)
                .map(|(r, seuil)| ecart(&m.mesure, i, &r.mesure, seuil, langue)),
        })
        .collect();
    FicheMesure {
        numero: m.numero,
        nom: m.nom.clone(),
        horodatage: p.horodatage.texte().to_string(),
        condition_impression: m.condition.nom.clone(),
        erreur_rangement: m.rangee.is_none().then_some(ERREUR_RANGEMENT),
        modele: renseigne(&p.instrument.modele),
        numero_serie: p.instrument.numero_serie,
        micrologiciel: renseigne(&p.instrument.micrologiciel),
        etalonnage: p.etalonnage.clone(),
        spectres,
        reference: est_reference,
    }
}
