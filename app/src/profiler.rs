//! Calcul d'un profil RVB à partir d'une mesure importée (ticket #40), rangé
//! dans la résolution comme profil actuel, « non vérifié ».
//!
//! Les méthodes de [`BibliothequeOuverte`] se testent sans Tauri, avec le
//! moteur simulé de la crate `profilage`. Le calcul se fait hors du verrou de
//! la bibliothèque : il peut durer plusieurs minutes. Les erreurs rendues à
//! la page sont des [`ErreurProfil`] : une clé du catalogue de textes et, si
//! ArgyllCMS en a donné une, sa cause.

use std::path::PathBuf;
use std::time::SystemTime;

use bibliotheque::{
    nom_profil_propose, Bibliotheque, ErreurBibliotheque, IdCondition, IdMesure, IdProfil,
    NouveauProfil, ResumeImportee, ResumeProfil,
};
use pont_protocole::ConditionMesure;
use profilage::{
    chercher_argyll, conditions_utilisables, ecrire_ti3, preparer_plages, ArgyllCms, DemandeProfil,
    ErreurMoteur, MoteurProfil, PlagesProfil, Recette, RefusPlages,
};
use serde::{Deserialize, Serialize};

use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;

/// Une mesure importée de la résolution, vue comme source d'un profil.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceProfil {
    pub mesure: IdMesure,
    /// Nom du fichier d'origine.
    pub fichier: String,
    /// Conditions de mesure que le calcul peut prendre : seulement celles
    /// que le fichier nomme, toutes plages mesurées.
    pub conditions: Vec<ConditionMesure>,
    /// Clé du texte qui dit pourquoi la mesure ne peut pas servir ; rien si
    /// elle peut servir.
    pub raison: Option<&'static str>,
}

/// Ce que la page reçoit après un calcul réussi.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ResultatProfil {
    pub profil: IdProfil,
    /// Écarts ΔE00 entre les mesures et ce que le profil prédit : une
    /// précision de calcul, pas une vérification.
    pub precision_moyenne: f64,
    pub precision_max: f64,
}

/// Pourquoi un profil n'a pas été calculé, pour la page.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ErreurProfil {
    /// Clé du texte : l'étape, la cause et l'action.
    pub cle: &'static str,
    /// Ce qu'ArgyllCMS a dit : la dernière ligne d'erreur d'un outil, ou la
    /// version trouvée. Montrée telle quelle, après le texte de la clé.
    pub cause: Option<String>,
}

impl ErreurProfil {
    fn sans_cause(cle: &'static str) -> Self {
        ErreurProfil { cle, cause: None }
    }

    fn du_moteur(erreur: &ErreurMoteur) -> Self {
        let cause = match erreur {
            ErreurMoteur::Echec { cause, .. } => Some(cause.clone()),
            ErreurMoteur::VersionNonValidee { trouvee } => Some(trouvee.clone()),
            ErreurMoteur::Introuvable | ErreurMoteur::Fichier(_) => None,
        };
        ErreurProfil {
            cle: erreur.cle(),
            cause,
        }
    }
}

impl From<String> for ErreurProfil {
    /// Une erreur de la bibliothèque, déjà rendue en clé du catalogue.
    fn from(cle: String) -> Self {
        let cle = crate::textes::cles()
            .find(|c| *c == cle)
            .unwrap_or("bibliotheque.erreur.autre");
        ErreurProfil::sans_cause(cle)
    }
}

/// D'où vient un profil calculé, lu dans ses paramètres conservés.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProvenanceProfil {
    pub mesure: IdMesure,
    /// Nom du fichier de la mesure source.
    pub fichier: String,
    pub condition: ConditionMesure,
    /// La condition est déduite d'une convention du fabricant.
    pub condition_supposee: bool,
    pub recette: Recette,
    /// Version d'ArgyllCMS qui a calculé le profil.
    pub version: String,
    pub precision_moyenne: f64,
    pub precision_max: f64,
    /// Commandes équivalentes au calcul, en lecture seule.
    pub commandes: Vec<String>,
    /// Journal d'ArgyllCMS pendant le calcul.
    pub journal: String,
}

/// Paramètres conservés d'un profil calculé, tels que [`BibliothequeOuverte::calculer_profil`]
/// les écrit (seuls les champs de la provenance).
#[derive(Deserialize)]
struct ParametresCalcul {
    origine: String,
    mesure: i64,
    fichier: String,
    condition: ConditionMesure,
    condition_supposee: bool,
    recette: Recette,
    version: String,
    precision: PrecisionLue,
    commandes: Vec<String>,
    journal: String,
}

#[derive(Deserialize)]
struct PrecisionLue {
    moyenne_de00: f64,
    max_de00: f64,
}

/// Provenance du profil `actif`, s'il a été calculé par myiro-libre ; rien
/// pour un profil rangé tel quel ou des paramètres illisibles.
pub(crate) fn provenance_profil(
    b: &Bibliotheque,
    actif: Option<&ResumeProfil>,
) -> Result<Option<ProvenanceProfil>, ErreurBibliotheque> {
    let Some(actif) = actif else {
        return Ok(None);
    };
    let texte = b.parametres_profil(actif.id)?;
    Ok(serde_json::from_str::<ParametresCalcul>(&texte)
        .ok()
        .filter(|p| p.origine == "calcul")
        .map(|p| ProvenanceProfil {
            mesure: IdMesure(p.mesure),
            fichier: p.fichier,
            condition: p.condition,
            condition_supposee: p.condition_supposee,
            recette: p.recette,
            version: p.version,
            precision_moyenne: p.precision.moyenne_de00,
            precision_max: p.precision.max_de00,
            commandes: p.commandes,
            journal: p.journal,
        }))
}

/// Le moteur de calcul de l'application : ArgyllCMS, cherché une fois au
/// lancement ; rien s'il est introuvable.
pub struct Moteur(pub Option<ArgyllCms>);

impl Moteur {
    /// Cherche ArgyllCMS à côté de l'exécutable, puis à la racine du dépôt.
    pub fn au_lancement() -> Moteur {
        let dossier_exe = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(PathBuf::from))
            .unwrap_or_default();
        let racine = chercher_argyll(&dossier_exe);
        if racine.is_none() {
            eprintln!("ArgyllCMS introuvable à côté de {}", dossier_exe.display());
        }
        Moteur(racine.map(ArgyllCms::new))
    }

    /// Le moteur, ou la clé `profil.erreur.argyll_introuvable`.
    pub fn moteur(&self) -> Result<&dyn MoteurProfil, ErreurProfil> {
        self.0
            .as_ref()
            .map(|a| a as &dyn MoteurProfil)
            .ok_or_else(|| ErreurProfil::du_moteur(&ErreurMoteur::Introuvable))
    }
}

/// Sources possibles d'un profil : une par mesure importée, dans l'ordre
/// donné. Une mesure qui ne peut pas servir dit pourquoi : RVB inconnus, ou
/// aucune condition de mesure complète.
pub(crate) fn sources_profil(
    b: &Bibliotheque,
    importees: &[ResumeImportee],
) -> Result<Vec<SourceProfil>, ErreurBibliotheque> {
    importees
        .iter()
        .map(|i| {
            let mesure = b.mesure_importee(i.id)?.mesure;
            let conditions = conditions_utilisables(&mesure);
            let raison = if !conditions.is_empty() {
                None
            } else if mesure.plages.is_empty() {
                Some(RefusPlages::Vide.cle())
            } else if mesure.plages.iter().any(|p| p.rvb.valeur().is_none()) {
                Some(RefusPlages::RvbInconnus.cle())
            } else {
                Some(RefusPlages::AucuneCondition.cle())
            };
            Ok(SourceProfil {
                mesure: i.id,
                fichier: i.fichier.clone(),
                conditions,
                raison,
            })
        })
        .collect()
}

/// Tout ce qu'il faut pour lancer le calcul, lu dans la bibliothèque.
struct Preparation {
    nom: String,
    description: String,
    horodatage: String,
    fichier: String,
    plages: PlagesProfil,
    ti3: String,
}

impl BibliothequeOuverte {
    /// Lit la mesure et les noms de la résolution, contrôle les plages et
    /// écrit le `.ti3`. Le verrou n'est tenu que pendant les lectures.
    fn preparer_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        maintenant: SystemTime,
    ) -> Result<Preparation, ErreurProfil> {
        let detail = self.detail_reglage(condition)?;
        let importee = self.avec(|b| b.mesure_importee(mesure))?;
        if importee.condition != condition {
            eprintln!("mesure n° {} rangée dans une autre résolution", mesure.0);
            return Err(ErreurProfil::sans_cause("profil.erreur.mesure_ailleurs"));
        }
        let plages = preparer_plages(&importee.mesure, mesure_condition).map_err(|e| {
            eprintln!("{} : {e}", importee.fichier);
            ErreurProfil::sans_cause(e.cle())
        })?;
        let horodatage = instant_utc(maintenant);
        let date = horodatage.get(..10).unwrap_or(&horodatage);
        let nom = nom_profil_propose(&detail.imprimante, &detail.papier, &detail.resolution, date);
        let description = format!(
            "{}, {}, {}",
            detail.imprimante, detail.papier, detail.resolution
        );
        let ti3 = ecrire_ti3(&plages, &description, &horodatage);
        Ok(Preparation {
            nom,
            description,
            horodatage,
            fichier: importee.fichier,
            plages,
            ti3,
        })
    }

    /// Commandes équivalentes au calcul, sans rien lancer (section
    /// « Avancé » de la page).
    pub fn commandes_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        recette: Recette,
        moteur: &dyn MoteurProfil,
    ) -> Result<Vec<String>, ErreurProfil> {
        let p = self.preparer_profil(condition, mesure, mesure_condition, SystemTime::now())?;
        Ok(moteur.commandes(&DemandeProfil {
            ti3: &p.ti3,
            description: &p.description,
            recette,
        }))
    }

    /// Calcule un profil RVB à partir d'une mesure importée de la résolution
    /// et le range : il devient le profil actuel, « non vérifié », sous le
    /// nom proposé. La commande, la version du moteur, le journal et la
    /// précision sont conservés avec lui. En cas d'erreur, rien n'est rangé
    /// et le profil actuel reste le même.
    pub fn calculer_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        recette: Recette,
        moteur: &dyn MoteurProfil,
        maintenant: SystemTime,
    ) -> Result<ResultatProfil, ErreurProfil> {
        let p = self.preparer_profil(condition, mesure, mesure_condition, maintenant)?;
        // Hors du verrou : la page et l'arbre restent utilisables.
        let calcule = moteur
            .calculer(&DemandeProfil {
                ti3: &p.ti3,
                description: &p.description,
                recette,
            })
            .map_err(|e| {
                eprintln!("{e}");
                ErreurProfil::du_moteur(&e)
            })?;
        let parametres = serde_json::json!({
            "origine": "calcul",
            "moteur": "ArgyllCMS",
            "version": calcule.version,
            "recette": recette,
            "condition": p.plages.condition,
            "condition_supposee": p.plages.condition_supposee,
            "rvb_supposes": p.plages.rvb_supposes,
            "mesure": mesure.0,
            "fichier": p.fichier,
            "commandes": calcule.commandes,
            "precision": calcule.precision,
            "ti3": p.ti3,
            "journal": calcule.journal,
        })
        .to_string();
        let profil = self.avec(|b| {
            b.enregistrer_profil(&NouveauProfil {
                condition,
                nom: &p.nom,
                icc: &calcule.icc,
                horodatage: &p.horodatage,
                parametres: &parametres,
                mesures: &[mesure],
            })
        })?;
        Ok(ResultatProfil {
            profil,
            precision_moyenne: calcule.precision.moyenne_de00,
            precision_max: calcule.precision.max_de00,
        })
    }

    /// Commande et réglages conservés avec un profil, tels quels (JSON).
    pub fn parametres_profil(&self, profil: IdProfil) -> Result<String, String> {
        self.avec(|b| b.parametres_profil(profil))
    }
}

// ---- Commandes de la page ----

/// Commandes équivalentes au calcul, en lecture seule.
#[tauri::command(async)]
pub fn profiler_commandes(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    condition: i64,
    mesure: i64,
    mesure_condition: ConditionMesure,
    recette: Recette,
) -> Result<Vec<String>, ErreurProfil> {
    ouverte.commandes_profil(
        IdCondition(condition),
        IdMesure(mesure),
        mesure_condition,
        recette,
        moteur.moteur()?,
    )
}

/// Calcule et range le profil. Hors du fil de la fenêtre : le calcul peut
/// durer plusieurs minutes.
#[tauri::command(async)]
pub fn profiler_calculer(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    condition: i64,
    mesure: i64,
    mesure_condition: ConditionMesure,
    recette: Recette,
) -> Result<ResultatProfil, ErreurProfil> {
    ouverte.calculer_profil(
        IdCondition(condition),
        IdMesure(mesure),
        mesure_condition,
        recette,
        moteur.moteur()?,
        SystemTime::now(),
    )
}
