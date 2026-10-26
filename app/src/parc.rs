//! Parc de la bibliothèque à l'écran : arbre imprimante → papier →
//! résolution, profils ICC rangés, exportés (page : `interface/parc.js`).
//!
//! Les méthodes de [`BibliothequeOuverte`] se testent sans Tauri ; les
//! commandes ne font qu'ouvrir les fenêtres de choix de fichier. Les erreurs
//! rendues à la page sont des clés du catalogue de textes.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bibliotheque::{
    nom_profil_propose, Bibliotheque, ConditionImpression, ErreurBibliotheque, EtatVerification,
    IdCondition, IdImprimante, IdPapier, IdProfil, Imprimante, NouveauProfil, Parc, ResumeImportee,
    ResumeMesure, ResumeProfil,
};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

use crate::colonne::BibliothequeOuverte;
use crate::installation::{
    self, est_installe, EtatInstallation, InstallationProfils, InstallationWindows,
};
use crate::profiler::{provenance_profil, sources_profil, ProvenanceProfil, SourceProfil};

/// Taille au-delà de laquelle un fichier n'est pas lu comme un profil :
/// 64 Mio, bien plus qu'un profil d'imprimante.
pub const TAILLE_MAX_PROFIL: u64 = 64 * 1024 * 1024;

/// Paramètres conservés avec un profil rangé tel quel, sans calcul.
const ORIGINE_PROFIL_EXISTANT: &str = r#"{"origine":"profil existant"}"#;

/// D'où vient un profil : `profil_existant` (rangé tel quel) ou `calcul`.
fn origine_profil(b: &Bibliotheque, profil: IdProfil) -> Result<&'static str, ErreurBibliotheque> {
    Ok(if b.parametres_profil(profil)? == ORIGINE_PROFIL_EXISTANT {
        "profil_existant"
    } else {
        "calcul"
    })
}

/// Clé du grand verdict d'une résolution. Un profil à vérifier dit d'où il
/// vient : « Profil calculé, à vérifier » ou « Profil importé, à
/// vérifier ». Seul le texte dépend de l'origine : l'état enregistré reste
/// `non_verifie`.
pub fn cle_verdict(etat: EtatVerification, origine: &str) -> &'static str {
    match etat {
        EtatVerification::NonVerifie if origine == "calcul" => "parc.verdict.calcule_a_verifier",
        EtatVerification::NonVerifie => "parc.verdict.importe_a_verifier",
        EtatVerification::Accepte => "parc.verdict.verifie",
        EtatVerification::HorsTolerance => "parc.verdict.hors",
        EtatVerification::Incomplet => "parc.verdict.incomplet",
    }
}

/// Clé de l'état d'un profil dans l'historique : le libellé du verdict tant
/// qu'il reste à vérifier, sinon son état court.
pub fn cle_etat(etat: EtatVerification, origine: &str) -> &'static str {
    match etat {
        EtatVerification::NonVerifie => cle_verdict(etat, origine),
        EtatVerification::Accepte => "parc.etat.accepte",
        EtatVerification::HorsTolerance => "parc.etat.hors_tolerance",
        EtatVerification::Incomplet => "parc.etat.incomplet",
    }
}

/// Un profil de l'historique, avec son origine et la clé de son libellé.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProfilHistorique {
    #[serde(flatten)]
    pub profil: ResumeProfil,
    /// `profil_existant` ou `calcul`.
    pub origine: &'static str,
    /// Clé du texte de son état ([`cle_etat`]).
    pub libelle: &'static str,
}

impl std::ops::Deref for ProfilHistorique {
    type Target = ResumeProfil;
    fn deref(&self) -> &ResumeProfil {
        &self.profil
    }
}

/// Ce que le centre et la droite montrent d'une résolution d'impression.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DetailReglage {
    pub condition: ConditionImpression,
    pub imprimante: String,
    pub papier: String,
    pub resolution: String,
    /// Le profil actif, s'il y en a un.
    pub actif: Option<ResumeProfil>,
    /// Tous les profils, le plus récent d'abord, actif compris.
    pub historique: Vec<ProfilHistorique>,
    /// Clé du grand verdict ([`cle_verdict`]) ; « Aucun profil » sans profil.
    pub verdict: &'static str,
    /// Le profil actif a plus d'un an.
    pub ancien: bool,
    /// D'où vient le profil actif : `profil_existant` (rangé tel quel) ou
    /// `calcul` ; rien sans profil.
    pub origine: Option<&'static str>,
    /// Mesure, condition, recette, version et précision d'un profil calculé.
    pub provenance: Option<ProvenanceProfil>,
    pub mesures: Vec<ResumeMesure>,
    pub importees: Vec<ResumeImportee>,
    /// Les mesures importées vues comme sources d'un profil, dans le même
    /// ordre que `importees`.
    pub sources: Vec<SourceProfil>,
    /// Le profil actif est installé dans Windows, à l'identique. Faux tant
    /// que le système n'a pas été regardé ([`BibliothequeOuverte::detail_reglage_installe`]).
    pub installe: bool,
}

/// Après une fenêtre de choix de fichier : rien si l'opérateur a annulé
/// (`None`), sinon `faire` avec le fichier choisi.
pub fn si_choisi<T>(
    fichier: Option<PathBuf>,
    faire: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<Option<T>, String> {
    fichier.map(|f| faire(&f)).transpose()
}

/// Instant en RFC 3339 UTC à la seconde (`2026-10-22T09:15:00Z`).
pub fn instant_utc(instant: SystemTime) -> String {
    let secondes = instant
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (jours, reste) = (secondes / 86_400, secondes % 86_400);
    // Jours depuis 1970 vers année, mois, jour (algorithme des ères de 400 ans).
    let z = i64::try_from(jours).unwrap_or(0) + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z - ere * 146_097;
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let m = (5 * jour_annee + 2) / 153;
    let jour = jour_annee - (153 * m + 2) / 5 + 1;
    let mois = if m < 10 { m + 3 } else { m - 9 };
    let annee = annee_ere + ere * 400 + i64::from(mois <= 2);
    format!(
        "{annee:04}-{mois:02}-{jour:02}T{:02}:{:02}:{:02}Z",
        reste / 3600,
        reste % 3600 / 60,
        reste % 60
    )
}

/// Lit un fichier choisi comme profil, borné à [`TAILLE_MAX_PROFIL`].
fn lire_profil(fichier: &Path) -> Result<Vec<u8>, String> {
    let illisible = |e: std::io::Error| {
        eprintln!("{} : {e}", fichier.display());
        "parc.erreur.fichier_illisible".to_string()
    };
    let mut octets = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(
            std::fs::File::open(fichier).map_err(illisible)?,
            TAILLE_MAX_PROFIL + 1,
        ),
        &mut octets,
    )
    .map_err(illisible)?;
    if octets.len() as u64 > TAILLE_MAX_PROFIL {
        eprintln!("{} : plus de {TAILLE_MAX_PROFIL} octets", fichier.display());
        return Err("parc.erreur.profil_illisible".to_string());
    }
    Ok(octets)
}

impl BibliothequeOuverte {
    /// Le parc, réduit par une recherche sans accents ni majuscules.
    pub fn parc(&self, recherche: &str) -> Result<Parc, String> {
        self.avec(|b| b.parc(recherche, SystemTime::now()))
    }

    pub fn ajouter_imprimante(&self, nom: &str) -> Result<Imprimante, String> {
        self.avec(|b| b.creer_imprimante(nom))
    }

    /// Crée le papier et sa première résolution en un geste. Une résolution
    /// vide est refusée avant que le papier ne soit créé.
    pub fn ajouter_papier(
        &self,
        imprimante: IdImprimante,
        nom: &str,
        resolution: &str,
    ) -> Result<ConditionImpression, String> {
        if resolution.trim().is_empty() {
            return Err("bibliotheque.erreur.nom_vide".to_string());
        }
        self.avec(|b| {
            let papier = b.creer_papier(imprimante, nom)?;
            b.creer_reglage(papier.id, resolution)
        })
    }

    pub fn ajouter_resolution(
        &self,
        papier: IdPapier,
        resolution: &str,
    ) -> Result<ConditionImpression, String> {
        self.avec(|b| b.creer_reglage(papier, resolution))
    }

    /// Une résolution rangée dans le parc : noms, profils et mesures.
    pub fn detail_reglage(&self, condition: IdCondition) -> Result<DetailReglage, String> {
        let detail = self.avec(|b| {
            let parc = b.parc("", SystemTime::now())?;
            let trouve = parc.imprimantes.iter().find_map(|i| {
                i.papiers.iter().find_map(|p| {
                    p.reglages
                        .iter()
                        .find(|r| r.condition.id == condition)
                        .map(|r| (i, p, r))
                })
            });
            let Some((i, p, r)) = trouve else {
                return Ok(None);
            };
            let branche = b
                .arborescence("")?
                .into_iter()
                .find(|br| br.condition.id == condition);
            let (mesures, importees) =
                branche.map_or_else(Default::default, |br| (br.mesures, br.importees));
            let origine = match &r.profil {
                Some(p) => Some(origine_profil(b, p.id)?),
                None => None,
            };
            let verdict = match (&r.profil, origine) {
                (Some(p), Some(o)) => cle_verdict(p.etat, o),
                _ => "parc.verdict.aucun",
            };
            let historique = b
                .historique_profils(condition)?
                .into_iter()
                .map(|profil| {
                    let origine = origine_profil(b, profil.id)?;
                    Ok(ProfilHistorique {
                        libelle: cle_etat(profil.etat, origine),
                        origine,
                        profil,
                    })
                })
                .collect::<Result<Vec<_>, ErreurBibliotheque>>()?;
            let sources = sources_profil(b, &importees)?;
            let provenance = provenance_profil(b, r.profil.as_ref())?;
            Ok(Some(DetailReglage {
                origine,
                provenance,
                condition: r.condition.clone(),
                imprimante: i.imprimante.nom.clone(),
                papier: p.papier.nom.clone(),
                resolution: r.resolution.clone(),
                actif: r.profil.clone(),
                historique,
                verdict,
                ancien: r.profil_ancien,
                mesures,
                importees,
                sources,
                installe: false,
            }))
        })?;
        detail.ok_or_else(|| "parc.erreur.reglage_inconnu".to_string())
    }

    /// Comme [`Self::detail_reglage`], en regardant aussi si le profil actif
    /// est installé dans `systeme` à l'identique.
    pub fn detail_reglage_installe(
        &self,
        condition: IdCondition,
        systeme: &dyn InstallationProfils,
    ) -> Result<DetailReglage, String> {
        let mut detail = self.detail_reglage(condition)?;
        if let Some(actif) = &detail.actif {
            let icc = self.avec(|b| b.fichier_profil(actif.id))?;
            detail.installe = est_installe(systeme, &actif.nom, &icc);
        }
        Ok(detail)
    }

    /// Installe un profil dans `systeme`, sous son nom, sans jamais écraser
    /// un autre profil du même nom. `temporaire` : dossier où le fichier est
    /// écrit le temps de l'installation.
    pub fn installer_profil(
        &self,
        profil: IdProfil,
        systeme: &dyn InstallationProfils,
        temporaire: &Path,
    ) -> Result<EtatInstallation, String> {
        let nom = self.profil(profil)?.nom;
        let icc = self.avec(|b| b.fichier_profil(profil))?;
        installation::installer_profil(systeme, &nom, &icc, temporaire)
            .map_err(|e| installation::cle_erreur(&e).to_string())
    }

    /// Range un profil ICC existant dans une résolution : il devient le
    /// profil actif, sous le nom proposé. `aujourd_hui` est la date
    /// (`AAAA-MM-JJ`) ou l'instant RFC 3339 du rangement. Un fichier qui
    /// n'est pas un profil RVB ou CMJN est refusé et rien ne change.
    pub fn ranger_profil(
        &self,
        condition: IdCondition,
        fichier: &Path,
        aujourd_hui: &str,
    ) -> Result<IdProfil, String> {
        let icc = lire_profil(fichier)?;
        let detail = self.detail_reglage(condition)?;
        let date = aujourd_hui.get(..10).unwrap_or(aujourd_hui);
        let horodatage = if aujourd_hui.len() > 10 {
            aujourd_hui.to_string()
        } else {
            format!("{date}T00:00:00Z")
        };
        let nom = nom_profil_propose(&detail.imprimante, &detail.papier, &detail.resolution, date);
        self.avec(|b| {
            b.enregistrer_profil(&NouveauProfil {
                condition,
                nom: &nom,
                icc: &icc,
                horodatage: &horodatage,
                parametres: ORIGINE_PROFIL_EXISTANT,
                mesures: &[],
            })
        })
    }

    /// Écrit le fichier ICC d'un profil, octet pour octet.
    pub fn exporter_profil(&self, profil: IdProfil, fichier: &Path) -> Result<(), String> {
        let icc = self.avec(|b| b.fichier_profil(profil))?;
        std::fs::write(fichier, icc).map_err(|e| {
            eprintln!("{} : {e}", fichier.display());
            "bibliotheque.erreur.export".to_string()
        })
    }

    /// Le résumé d'un profil rangé, d'où qu'il soit.
    pub fn profil(&self, profil: IdProfil) -> Result<ResumeProfil, String> {
        self.avec(|b| {
            let parc = b.parc("", SystemTime::now())?;
            let conditions = parc
                .imprimantes
                .iter()
                .flat_map(|i| &i.papiers)
                .flat_map(|p| &p.reglages)
                .map(|r| r.condition.id)
                .chain(parc.non_classees.iter().map(|c| c.id));
            for condition in conditions {
                if let Some(trouve) = b
                    .historique_profils(condition)?
                    .into_iter()
                    .find(|p| p.id == profil)
                {
                    return Ok(Some(trouve));
                }
            }
            Ok(None)
        })?
        .ok_or_else(|| "parc.erreur.profil_inconnu".to_string())
    }
}

// ---- Commandes de la page ----

#[tauri::command]
pub fn parc(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    recherche: &str,
) -> Result<Parc, String> {
    ouverte.parc(recherche)
}

#[tauri::command]
pub fn parc_ajouter_imprimante(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    nom: &str,
) -> Result<Imprimante, String> {
    ouverte.ajouter_imprimante(nom)
}

#[tauri::command]
pub fn parc_ajouter_papier(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    imprimante: i64,
    nom: &str,
    resolution: &str,
) -> Result<ConditionImpression, String> {
    ouverte.ajouter_papier(IdImprimante(imprimante), nom, resolution)
}

#[tauri::command]
pub fn parc_ajouter_resolution(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    papier: i64,
    resolution: &str,
) -> Result<ConditionImpression, String> {
    ouverte.ajouter_resolution(IdPapier(papier), resolution)
}

#[tauri::command]
pub fn parc_detail_reglage(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    condition: i64,
) -> Result<DetailReglage, String> {
    ouverte.detail_reglage_installe(IdCondition(condition), &InstallationWindows)
}

/// Installe le profil dans le dossier des couleurs de Windows.
#[tauri::command(async)]
pub fn parc_installer_profil(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    profil: i64,
) -> Result<EtatInstallation, String> {
    let temporaire = std::env::temp_dir().join("myiro-libre-profils");
    std::fs::create_dir_all(&temporaire).map_err(|e| {
        eprintln!("{} : {e}", temporaire.display());
        "installation.echec".to_string()
    })?;
    ouverte.installer_profil(IdProfil(profil), &InstallationWindows, &temporaire)
}

/// Range le profil choisi dans la fenêtre de Windows (`.icc`, `.icm`).
/// `None` si l'opérateur a annulé : rien ne change.
#[tauri::command(async)]
pub fn parc_ranger_profil(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    condition: i64,
    filtre: String,
) -> Result<Option<IdProfil>, String> {
    let fichier = app
        .dialog()
        .file()
        .add_filter(filtre, &["icc", "icm"])
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok());
    let maintenant = instant_utc(SystemTime::now());
    si_choisi(fichier, |f| {
        ouverte.ranger_profil(IdCondition(condition), f, &maintenant)
    })
}

/// Exporte le profil là où l'opérateur le dit, sous son nom par défaut.
/// Rend le nom du fichier écrit, ou `None` si l'opérateur a annulé.
#[tauri::command(async)]
pub fn parc_exporter_profil(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    profil: i64,
    filtre: String,
) -> Result<Option<String>, String> {
    let profil = IdProfil(profil);
    let nom = ouverte.profil(profil)?.nom;
    let fichier = app
        .dialog()
        .file()
        .set_file_name(nom)
        .add_filter(filtre, &["icc", "icm"])
        .blocking_save_file()
        .and_then(|f| f.into_path().ok());
    si_choisi(fichier, |f| {
        ouverte.exporter_profil(profil, f)?;
        Ok(f.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    })
}
