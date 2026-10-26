//! Calcul d'un profil RVB à partir d'une mesure importée (ticket #40), rangé
//! dans la résolution comme profil actuel, « à vérifier ».
//!
//! Les méthodes de [`BibliothequeOuverte`] se testent sans Tauri, avec le
//! moteur simulé de la crate `profilage`. Le calcul se fait hors du verrou de
//! la bibliothèque : il peut durer plusieurs minutes. Les erreurs rendues à
//! la page sont des [`ErreurProfil`] : une clé du catalogue de textes et, si
//! ArgyllCMS en a donné une, sa cause.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bibliotheque::{
    nom_profil_propose, Bibliotheque, ErreurBibliotheque, IdCondition, IdMesure, IdProfil,
    NouveauProfil, ResumeImportee, ResumeProfil,
};
use pont_protocole::ConditionMesure;
use profilage::{
    chercher_argyll, conditions_utilisables, ecrire_ti3, lignes_commandes, preparer_plages,
    ArgyllCms, Avancement, DemandeProfil, ErreurMoteur, EtapeCalcul, EtatAvancement, MoteurProfil,
    PlagesProfil, Recette, RefusPlages, SuiviAvancement,
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
    /// Nombre de plages calculées : rapporte la durée du calcul à la mesure
    /// suivante ([`HistoriqueDurees`]).
    pub plages: usize,
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
    /// Ce qu'ArgyllCMS ou le système a dit : la dernière ligne d'erreur d'un
    /// outil, ou la cause d'un fichier de travail. Montrée telle quelle,
    /// après le texte de la clé.
    pub cause: Option<String>,
    /// Version d'ArgyllCMS trouvée mais refusée : elle prend la place de
    /// `{version}` dans le texte de la clé.
    pub version: Option<String>,
}

impl ErreurProfil {
    fn sans_cause(cle: &'static str) -> Self {
        ErreurProfil {
            cle,
            cause: None,
            version: None,
        }
    }

    fn du_moteur(erreur: &ErreurMoteur) -> Self {
        let mut e = ErreurProfil::sans_cause(erreur.cle());
        match erreur {
            ErreurMoteur::Echec { cause, .. } | ErreurMoteur::Fichier(cause) => {
                e.cause = Some(cause.clone());
            }
            ErreurMoteur::VersionNonValidee { trouvee } => e.version = Some(trouvee.clone()),
            ErreurMoteur::Introuvable => {}
        }
        e
    }
}

/// Ce que la page reçoit pour la section « Avancé » : la commande
/// équivalente, et la raison qui empêchera le calcul si ArgyllCMS manque.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandesProfil {
    pub commandes: Vec<String>,
    pub moteur: Option<ErreurProfil>,
    /// Durée estimée du calcul choisi, en secondes ; rien sans historique
    /// pour sa recette ([`HistoriqueDurees`]).
    pub estimation: Option<f64>,
}

/// Fichier où les durées des derniers calculs sont enregistrées, dans le
/// dossier de configuration de l'application.
pub const FICHIER_DUREES: &str = "durees-calcul.json";

/// Durée et nombre de plages d'un calcul réussi.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DureeCalcul {
    pub secondes: f64,
    pub plages: usize,
}

/// Durée du dernier calcul réussi de chaque recette, enregistrée sur le
/// poste. Elle sert à estimer la durée du calcul suivant, rapportée au
/// nombre de plages : une estimation, dite comme telle à l'écran.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoriqueDurees {
    #[serde(default)]
    rapide: Option<DureeCalcul>,
    #[serde(default)]
    standard: Option<DureeCalcul>,
    #[serde(default)]
    approfondi: Option<DureeCalcul>,
}

impl HistoriqueDurees {
    fn place(&mut self, recette: Recette) -> &mut Option<DureeCalcul> {
        match recette {
            Recette::Rapide => &mut self.rapide,
            Recette::Standard => &mut self.standard,
            Recette::Approfondi => &mut self.approfondi,
        }
    }

    /// Historique enregistré ; vide si le fichier manque ou est illisible.
    pub fn lire(fichier: &Path) -> Self {
        std::fs::read_to_string(fichier)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn ecrire(&self, fichier: &Path) -> std::io::Result<()> {
        if let Some(dossier) = fichier.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        let texte = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(fichier, texte)
    }

    /// Retient la durée d'un calcul réussi : elle remplace la précédente de
    /// la même recette.
    pub fn noter(&mut self, recette: Recette, secondes: f64, plages: usize) {
        *self.place(recette) = Some(DureeCalcul { secondes, plages });
    }

    /// Durée estimée d'un calcul de `plages` plages : celle du dernier calcul
    /// de la même recette, rapportée au nombre de plages. Rien sans
    /// historique.
    pub fn estimer(&self, recette: Recette, plages: usize) -> Option<f64> {
        let mut copie = self.clone();
        let d = (*copie.place(recette))?;
        (d.plages > 0 && d.secondes.is_finite() && d.secondes > 0.0)
            .then(|| d.secondes * plages as f64 / d.plages as f64)
    }
}

/// Journal tel qu'affiché dans « Avancé » : chaque suite de lignes faites
/// seulement d'astérisques (table principale de colprof) devient une seule
/// ligne « ****** (×N) », N le nombre d'astérisques. Le journal enregistré,
/// lui, reste complet.
pub fn journal_court(journal: &str) -> String {
    let mut sortie = String::with_capacity(journal.len());
    let mut asterisques = 0usize;
    // Fin de ligne de la suite en cours, reprise après « (×N) ».
    let mut fin_suite = "";
    for morceau in journal.split_inclusive('\n') {
        let contenu = morceau.trim_end_matches(['\r', '\n']);
        let fin = &morceau[contenu.len()..];
        if !contenu.is_empty() && contenu.chars().all(|c| c == '*') {
            asterisques += contenu.len();
            fin_suite = fin;
            continue;
        }
        if asterisques > 0 {
            sortie.push_str(&format!("****** (×{asterisques}){fin_suite}"));
            asterisques = 0;
        }
        sortie.push_str(morceau);
    }
    if asterisques > 0 {
        sortie.push_str(&format!("****** (×{asterisques}){fin_suite}"));
    }
    sortie
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
    /// L'échelle des valeurs RVB (0 à 255 pour FD-S2w et MYIROtools) est supposée.
    pub rvb_supposes: bool,
    pub recette: Recette,
    /// Version d'ArgyllCMS qui a calculé le profil.
    pub version: String,
    pub precision_moyenne: f64,
    pub precision_max: f64,
    /// Commandes équivalentes au calcul, en lecture seule.
    pub commandes: Vec<String>,
    /// Journal d'ArgyllCMS pendant le calcul, astérisques regroupés pour
    /// l'affichage ([`journal_court`]) ; le journal enregistré reste complet.
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
    rvb_supposes: bool,
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
            rvb_supposes: p.rvb_supposes,
            recette: p.recette,
            version: p.version,
            precision_moyenne: p.precision.moyenne_de00,
            precision_max: p.precision.max_de00,
            commandes: p.commandes,
            journal: journal_court(&p.journal),
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

    /// Pourquoi le calcul sera refusé avant même d'être lancé ; rien si
    /// ArgyllCMS a été trouvé au lancement.
    pub fn indisponible(&self) -> Option<ErreurProfil> {
        self.moteur().err()
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
    /// « Avancé » de la page) ; ArgyllCMS n'est pas nécessaire.
    pub fn commandes_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        recette: Recette,
    ) -> Result<Vec<String>, ErreurProfil> {
        let p = self.preparer_profil(condition, mesure, mesure_condition, SystemTime::now())?;
        Ok(lignes_commandes(recette, &p.description))
    }

    /// Durée estimée du calcul choisi, d'après l'historique ; rien sans
    /// historique pour la recette.
    pub fn estimation_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        recette: Recette,
        historique: &HistoriqueDurees,
    ) -> Result<Option<f64>, ErreurProfil> {
        let p = self.preparer_profil(condition, mesure, mesure_condition, SystemTime::now())?;
        Ok(historique.estimer(recette, p.plages.plages.len()))
    }

    /// Calcule un profil RVB à partir d'une mesure importée de la résolution
    /// et le range : il devient le profil actuel, « à vérifier », sous le
    /// nom proposé. La commande, la version du moteur, le journal et la
    /// précision sont conservés avec lui. En cas d'erreur, rien n'est rangé
    /// et le profil actuel reste le même. L'avancement (étapes, lignes et
    /// pourcentages d'ArgyllCMS) est annoncé à `avancement` en route.
    #[allow(clippy::too_many_arguments)]
    pub fn calculer_profil(
        &self,
        condition: IdCondition,
        mesure: IdMesure,
        mesure_condition: ConditionMesure,
        recette: Recette,
        moteur: &dyn MoteurProfil,
        maintenant: SystemTime,
        avancement: &mut dyn FnMut(Avancement),
    ) -> Result<ResultatProfil, ErreurProfil> {
        avancement(Avancement::Etape(EtapeCalcul::Preparation));
        let p = self.preparer_profil(condition, mesure, mesure_condition, maintenant)?;
        // Hors du verrou : la page et l'arbre restent utilisables.
        let calcule = moteur
            .calculer(
                &DemandeProfil {
                    ti3: &p.ti3,
                    description: &p.description,
                    recette,
                },
                avancement,
            )
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
            plages: p.plages.plages.len(),
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

/// Fichier de l'historique des durées de calcul ; rien si le dossier de
/// configuration est introuvable.
fn fichier_durees(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri::Manager;
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(FICHIER_DUREES))
}

/// Commandes équivalentes au calcul, en lecture seule, même sans
/// ArgyllCMS ; la page apprend aussi s'il manque, avant le clic, et la
/// durée estimée d'après le dernier calcul de la même recette.
#[tauri::command(async)]
pub fn profiler_commandes(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    condition: i64,
    mesure: i64,
    mesure_condition: ConditionMesure,
    recette: Recette,
) -> Result<CommandesProfil, ErreurProfil> {
    let (condition, mesure) = (IdCondition(condition), IdMesure(mesure));
    let historique = fichier_durees(&app)
        .map(|f| HistoriqueDurees::lire(&f))
        .unwrap_or_default();
    Ok(CommandesProfil {
        commandes: ouverte.commandes_profil(condition, mesure, mesure_condition, recette)?,
        moteur: moteur.indisponible(),
        estimation: ouverte.estimation_profil(
            condition,
            mesure,
            mesure_condition,
            recette,
            &historique,
        )?,
    })
}

/// Événement qui porte l'avancement d'un calcul vers la page.
pub const EVENEMENT_AVANCEMENT: &str = "profil-avancement";

/// Ce que l'événement [`EVENEMENT_AVANCEMENT`] porte : le numéro du calcul,
/// donné par la page, et ce que la barre montre.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EvenementAvancement<'a> {
    pub calcul: u64,
    #[serde(flatten)]
    pub etat: &'a EtatAvancement,
}

/// Suit un calcul lancé par la page : chaque annonce du moteur devient un
/// événement à son numéro. La page ignore les événements d'un autre calcul.
pub struct SuiviCalcul {
    calcul: u64,
    suivi: SuiviAvancement,
}

impl SuiviCalcul {
    pub fn new(calcul: u64) -> Self {
        SuiviCalcul {
            calcul,
            suivi: SuiviAvancement::new(),
        }
    }

    pub fn recevoir(&mut self, annonce: &Avancement) -> EvenementAvancement<'_> {
        EvenementAvancement {
            calcul: self.calcul,
            etat: self.suivi.recevoir(annonce),
        }
    }
}

/// Calcule et range le profil. Hors du fil de la fenêtre : le calcul peut
/// durer plusieurs minutes. Son avancement part vers la page par
/// l'événement [`EVENEMENT_AVANCEMENT`], au numéro `calcul` donné par la
/// page ; la durée d'un calcul réussi est
/// retenue pour estimer le suivant.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn profiler_calculer(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    moteur: tauri::State<'_, Moteur>,
    condition: i64,
    mesure: i64,
    mesure_condition: ConditionMesure,
    recette: Recette,
    calcul: u64,
) -> Result<ResultatProfil, ErreurProfil> {
    use tauri::Emitter;
    let debut = std::time::Instant::now();
    let mut suivi = SuiviCalcul::new(calcul);
    let resultat = ouverte.calculer_profil(
        IdCondition(condition),
        IdMesure(mesure),
        mesure_condition,
        recette,
        moteur.moteur()?,
        SystemTime::now(),
        // Une page fermée ne reçoit rien : le calcul continue.
        &mut |a| {
            let _ = app.emit(EVENEMENT_AVANCEMENT, suivi.recevoir(&a));
        },
    )?;
    if let Some(fichier) = fichier_durees(&app) {
        let mut historique = HistoriqueDurees::lire(&fichier);
        historique.noter(recette, debut.elapsed().as_secs_f64(), resultat.plages);
        // Une estimation perdue n'empêche pas le profil d'être rangé.
        if let Err(e) = historique.ecrire(&fichier) {
            eprintln!("{} : {e}", fichier.display());
        }
    }
    Ok(resultat)
}
