//! Vérifier un profil ICC sur une seconde mire importée (ticket #41).
//!
//! Les méthodes de [`BibliothequeOuverte`] se testent sans Tauri, avec les
//! moteurs simulés de la crate `profilage`. Le profil est interrogé hors du
//! verrou de la bibliothèque. Tout refus arrive avant le calcul ; un échec
//! ne range rien, et un profil hors tolérance reste actif et exportable.
//! Les commandes de la page sont dans [`commandes`].

mod association;
pub mod commandes;
mod lecture;
mod mire;

use std::time::SystemTime;

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, EtatVerification, IdCondition, IdMesure, IdProfil,
    IdTolerances, IdVerification, JeuTolerances, NouvelleVerification, VerificationRangee,
};
use pont_protocole::ConditionMesure;
use profilage::{preparer_plages, DemandeEvaluation, Evaluation, Impression, MoteurVerification};
use serde::{Deserialize, Serialize};
use verification::{
    comparer, conclure, reclasser, valider_criteres, Conclusion, Controle, Criteres,
    PlageAVerifier, Resultats, ValeursProfil,
};

use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;
use crate::profiler::{refuser_incoherence, ErreurProfil};
use crate::textes::{texte, Langue};

/// Fiche d'impression de la seconde mire, confirmée par l'imprimeur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FicheTirage {
    pub impression: Impression,
    pub logiciel: String,
    pub support_pilote: String,
    pub qualite: String,
    pub encres: String,
    pub remarques: String,
    pub confirmee: bool,
}

/// Ce que la page demande : le profil, la mesure de la seconde mire, sa
/// condition de mesure, le jeu de tolérances et la fiche.
#[derive(Clone, Debug, Deserialize)]
pub struct DemandeVerification {
    pub profil: IdProfil,
    pub mesure: IdMesure,
    pub mesure_condition: ConditionMesure,
    pub tolerances: IdTolerances,
    pub fiche: FicheTirage,
}

/// Le calcul d'une vérification, rangé tel quel (JSON) : de quoi refaire le
/// rapport et recalculer sans remesure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalculVerification {
    pub fichier: String,
    /// Modèle de l'instrument que le fichier déclare (« FD-9 »), jamais son
    /// numéro de série ; rien si le fichier ne nomme pas un modèle connu.
    pub instrument: Option<String>,
    pub condition: ConditionMesure,
    pub condition_supposee: bool,
    /// L'échelle des RVB du fichier (0 à 255 pour FD-S2w) est supposée.
    #[serde(default)]
    pub rvb_supposes: bool,
    /// La mire de vérification mesurée ; rien quand c'est la mire du profil.
    #[serde(default)]
    pub mire: Option<String>,
    pub memes_couleurs_que_le_profil: bool,
    pub evaluation: Evaluation,
    pub resultats: Resultats,
    pub controles: Vec<Controle>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResultatVerification {
    pub verification: IdVerification,
    pub etat: EtatVerification,
    pub calcul: CalculVerification,
}

/// Une vérification dans l'historique d'un profil.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResumeVerification {
    pub verification: IdVerification,
    pub horodatage: String,
    pub etat: EtatVerification,
    /// Nom et version du jeu de tolérances, montrés ensemble
    /// (`verif.tolerances.nom_version` : « Atelier v2 »).
    pub tolerances_nom: String,
    pub tolerances_version: u32,
}

/// Nom et version d'un jeu dans la langue : « Atelier v2 ».
pub fn nom_version(jeu: &JeuTolerances, langue: Langue) -> String {
    texte(langue, "verif.tolerances.nom_version")
        .replace("{nom}", &jeu.nom)
        .replace("{version}", &jeu.version.to_string())
}

pub(crate) fn resume(v: &VerificationRangee) -> ResumeVerification {
    ResumeVerification {
        verification: v.id,
        horodatage: v.horodatage.clone(),
        etat: v.etat,
        tolerances_nom: v.tolerances.nom.clone(),
        tolerances_version: v.tolerances.version,
    }
}

/// La plus récente vérification d'un profil, s'il en a une.
pub(crate) fn derniere_verification(
    b: &Bibliotheque,
    profil: IdProfil,
) -> Result<Option<ResumeVerification>, ErreurBibliotheque> {
    Ok(b.verifications(profil)?.first().map(resume))
}

fn etat(conclusion: Conclusion) -> EtatVerification {
    match conclusion {
        Conclusion::Accepte => EtatVerification::Accepte,
        Conclusion::HorsTolerance => EtatVerification::HorsTolerance,
        Conclusion::Incomplet => EtatVerification::Incomplet,
    }
}

fn illisible(quoi: &str, e: serde_json::Error) -> ErreurProfil {
    eprintln!("{quoi} illisible : {e}");
    ErreurProfil::sans_cause("bibliotheque.erreur.autre")
}

pub(crate) fn lire_criteres(jeu: &JeuTolerances) -> Result<Criteres, ErreurProfil> {
    lire_json("jeu de tolérances", &jeu.criteres)
}

/// Relit un JSON rangé (fiche, calcul) ; illisible : erreur de la
/// bibliothèque, jamais une valeur inventée.
pub(crate) fn lire_json<T: serde::de::DeserializeOwned>(
    quoi: &str,
    texte: &str,
) -> Result<T, ErreurProfil> {
    serde_json::from_str(texte).map_err(|e| illisible(quoi, e))
}

fn lab([l, a, b]: [f64; 3]) -> colorimetrie::Lab {
    colorimetrie::Lab { l, a, b }
}

impl BibliothequeOuverte {
    /// La fiche de la dernière vérification faite dans la résolution, à
    /// confirmer de nouveau ; rien avant la première.
    pub fn derniere_fiche(
        &self,
        condition: IdCondition,
    ) -> Result<Option<FicheTirage>, ErreurProfil> {
        let derniere = self.avec(|b| {
            let mut toutes = Vec::new();
            for p in b.historique_profils(condition)? {
                toutes.extend(b.verifications(p.id)?);
            }
            Ok(toutes
                .into_iter()
                .max_by(|x, y| (x.horodatage.as_str(), x.id).cmp(&(y.horodatage.as_str(), y.id))))
        })?;
        let Some(v) = derniere else { return Ok(None) };
        let mut fiche: FicheTirage =
            serde_json::from_str(&v.fiche).map_err(|e| illisible("fiche", e))?;
        fiche.confirmee = false;
        Ok(Some(fiche))
    }

    /// Range une nouvelle version du jeu `nom`, après contrôle des seuils.
    pub fn enregistrer_tolerances(
        &self,
        nom: &str,
        criteres: &Criteres,
        maintenant: SystemTime,
    ) -> Result<IdTolerances, ErreurProfil> {
        valider_criteres(criteres).map_err(|e| ErreurProfil::sans_cause(e.cle()))?;
        let texte = serde_json::to_string(criteres).map_err(|e| illisible("critères", e))?;
        Ok(self.avec(|b| b.enregistrer_tolerances(nom, &texte, &instant_utc(maintenant)))?)
    }

    pub fn jeux_tolerances(&self) -> Result<Vec<JeuTolerances>, ErreurProfil> {
        Ok(self.avec(|b| b.jeux_tolerances())?)
    }

    /// Vérifie un profil sur la mesure d'une seconde mire et range le
    /// résultat : le profil prend l'état de la vérification. En cas de
    /// refus ou d'échec, rien n'est rangé.
    pub fn verifier_profil(
        &self,
        demande: &DemandeVerification,
        moteur: &dyn MoteurVerification,
        maintenant: SystemTime,
    ) -> Result<ResultatVerification, ErreurProfil> {
        let lu = self.lire_pour_verifier(demande)?;
        let fichier = &lu.importee.fichier;
        let plages =
            preparer_plages(&lu.importee.mesure, demande.mesure_condition).map_err(|e| {
                eprintln!("{fichier} : {e}");
                ErreurProfil::sans_cause(e.cle())
            })?;
        // Mêmes contrôles et mêmes seuils que pour le calcul d'un profil.
        refuser_incoherence(&lu.importee, &plages)?;
        // Les couleurs demandées sont celles de la mire retrouvée, par ID.
        let demandes = &lu.association.demandes;
        let rvb: Vec<[f64; 3]> = plages
            .plages
            .iter()
            .zip(&lu.importee.mesure.plages)
            .map(|(p, origine)| {
                demandes
                    .get(&origine.identifiant)
                    .copied()
                    .unwrap_or(p.rvb.map(|v| v / 100.0))
            })
            .collect();
        // Hors du verrou : la page et l'arbre restent utilisables.
        let evaluation = moteur
            .evaluer(&DemandeEvaluation {
                icc: &lu.icc,
                rvb: &rvb,
                impression: demande.fiche.impression,
            })
            .map_err(|e| {
                eprintln!("{e}");
                ErreurProfil::du_moteur(&e)
            })?;
        let a_verifier: Vec<PlageAVerifier> = plages
            .plages
            .iter()
            .zip(&lu.importee.mesure.plages)
            .zip(&rvb)
            .map(|((p, origine), rvb)| PlageAVerifier {
                id: origine.identifiant.clone(),
                rvb: *rvb,
                mesure: lab(p.lab),
            })
            .collect();
        let valeurs = ValeursProfil {
            demande: evaluation
                .demande
                .as_ref()
                .map(|d| d.iter().copied().map(lab).collect()),
            appareil: evaluation.appareil.clone(),
            prediction_rel: evaluation.prediction_rel.iter().copied().map(lab).collect(),
            prediction_abs: evaluation.prediction_abs.iter().copied().map(lab).collect(),
            blanc_support: lab(evaluation.blanc_support),
        };
        let resultats = comparer(&a_verifier, &valeurs, lu.criteres.seuil_gamut).map_err(|e| {
            eprintln!("{fichier} : {e}");
            ErreurProfil::sans_cause(e.cle())
        })?;
        let (conclusion, controles) = conclure(&resultats, &lu.criteres);
        let calcul = CalculVerification {
            fichier: fichier.clone(),
            instrument: lu
                .importee
                .mesure
                .instrument
                .valeur()
                .and_then(|i| cgats::modele_instrument(i)),
            condition: plages.condition,
            condition_supposee: plages.condition_supposee,
            rvb_supposes: plages.rvb_supposes,
            memes_couleurs_que_le_profil: lu.association.mire.is_none(),
            mire: lu.association.mire.clone(),
            evaluation,
            resultats,
            controles,
        };
        let fiche = serde_json::to_string(&demande.fiche).map_err(|e| illisible("fiche", e))?;
        self.ranger(
            demande.profil,
            demande.mesure,
            demande.tolerances,
            &fiche,
            calcul,
            etat(conclusion),
            None,
            maintenant,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn ranger(
        &self,
        profil: IdProfil,
        mesure: IdMesure,
        tolerances: IdTolerances,
        fiche: &str,
        calcul: CalculVerification,
        etat: EtatVerification,
        recalcul_de: Option<IdVerification>,
        maintenant: SystemTime,
    ) -> Result<ResultatVerification, ErreurProfil> {
        let texte = serde_json::to_string(&calcul).map_err(|e| illisible("calcul", e))?;
        let horodatage = instant_utc(maintenant);
        let verification = self.avec(|b| {
            b.enregistrer_verification(&NouvelleVerification {
                profil,
                mesure,
                tolerances,
                horodatage: &horodatage,
                fiche,
                calcul: &texte,
                etat,
                recalcul_de,
            })
        })?;
        Ok(ResultatVerification {
            verification,
            etat,
            calcul,
        })
    }

    /// Recalcule une vérification avec un autre jeu (ou une autre version),
    /// à partir des écarts conservés plage par plage : ni ArgyllCMS ni
    /// nouvelle mesure. Une vérification s'ajoute ; l'originale ne change pas.
    pub fn recalculer_verification(
        &self,
        verification: IdVerification,
        tolerances: IdTolerances,
        maintenant: SystemTime,
    ) -> Result<ResultatVerification, ErreurProfil> {
        let (origine, jeu) =
            self.avec(|b| Ok((b.verification(verification)?, b.jeu_tolerances(tolerances)?)))?;
        let criteres = lire_criteres(&jeu)?;
        let mut calcul: CalculVerification =
            serde_json::from_str(&origine.calcul).map_err(|e| illisible("calcul", e))?;
        calcul.resultats = reclasser(&calcul.resultats, criteres.seuil_gamut);
        let (conclusion, controles) = conclure(&calcul.resultats, &criteres);
        calcul.controles = controles;
        self.ranger(
            origine.profil,
            origine.mesure,
            tolerances,
            &origine.fiche,
            calcul,
            etat(conclusion),
            Some(verification),
            maintenant,
        )
    }

    /// Les vérifications d'un profil, la plus récente d'abord.
    pub fn historique_verifications(
        &self,
        profil: IdProfil,
    ) -> Result<Vec<ResumeVerification>, ErreurProfil> {
        Ok(self.avec(|b| Ok(b.verifications(profil)?.iter().map(resume).collect()))?)
    }
}
