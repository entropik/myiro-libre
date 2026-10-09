//! Colonne de gauche (bibliothèque par condition d'impression) et panneau de
//! détails : commandes appelées par la page (`interface/bibliotheque.js`).
//!
//! Les erreurs rendues à la page sont des clés du catalogue de textes, jamais
//! des phrases techniques.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bibliotheque::{
    Bibliotheque, Branche, ConditionImpression, ErreurBibliotheque, IdCondition, IdMesure,
    Instrument, MesureEnregistree,
};
use cgats::MesureImportee;
use pont_protocole::{ConditionMesure, Geometrie, Horodatage, Info};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// La bibliothèque ouverte au lancement, partagée par les commandes.
pub struct BibliothequeOuverte {
    bibliotheque: Result<Mutex<Bibliotheque>, ErreurBibliotheque>,
    demonstration: bool,
    /// Fichier CGATS lu, montré en aperçu, en attente de l'accord pour être rangé.
    apercu: Mutex<Option<Apercu>>,
    /// Numéro du dernier aperçu donné.
    numero_apercu: std::sync::atomic::AtomicU64,
}

/// Emplacement de la bibliothèque sur le poste : un seul, dans le dossier de
/// données de l'application de l'utilisateur (ADR 0001).
pub fn emplacement(donnees_application: &Path) -> PathBuf {
    donnees_application.join("bibliotheque")
}

impl BibliothequeOuverte {
    /// Ouvre la bibliothèque du dossier.
    pub fn ouvrir(dossier: &Path) -> BibliothequeOuverte {
        BibliothequeOuverte {
            bibliotheque: Bibliotheque::ouvrir(dossier).map(Mutex::new),
            demonstration: false,
            apercu: Mutex::new(None),
            numero_apercu: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Bibliothèque de démonstration : vidée puis garnie de mesures fictives.
    pub fn demonstration(dossier: &Path) -> BibliothequeOuverte {
        let base = dossier.join(bibliotheque::FICHIER_BASE);
        let ouverte = match std::fs::remove_file(&base) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(ErreurBibliotheque::Base(
                format!("{} : {e}", base.display()),
            )),
            _ => Bibliotheque::ouvrir(dossier),
        }
        .and_then(|b| crate::demonstration::remplir(&b).map(|()| b));
        BibliothequeOuverte {
            bibliotheque: ouverte.map(Mutex::new),
            demonstration: true,
            apercu: Mutex::new(None),
            numero_apercu: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn avec<T>(
        &self,
        f: impl FnOnce(&Bibliotheque) -> Result<T, ErreurBibliotheque>,
    ) -> Result<T, String> {
        let bibliotheque = self.bibliotheque.as_ref().map_err(|e| {
            eprintln!("{e}");
            "bibliotheque.erreur.ouverture".to_string()
        })?;
        let bibliotheque = bibliotheque.lock().unwrap_or_else(|e| e.into_inner());
        f(&bibliotheque).map_err(|e| cle_erreur(&e).to_string())
    }

    pub fn arborescence(&self, recherche: &str) -> Result<Vec<Branche>, String> {
        self.avec(|b| b.arborescence(recherche))
    }

    pub fn detail_mesure(&self, id: IdMesure) -> Result<DetailMesure, String> {
        self.avec(|b| {
            let mesure = b.mesure(id)?;
            let nom = b
                .conditions()?
                .into_iter()
                .find(|c| c.id == mesure.condition)
                .map(|c| c.nom)
                .unwrap_or_default();
            Ok(DetailMesure::de(&mesure, nom))
        })
    }

    pub fn creer_condition(&self, nom: &str) -> Result<ConditionImpression, String> {
        self.avec(|b| b.creer_condition(nom))
    }

    pub fn renommer_condition(&self, id: IdCondition, nom: &str) -> Result<(), String> {
        self.avec(|b| b.renommer_condition(id, nom))
    }

    /// Conditions d'impression, pour la feuille Mesurer.
    pub fn conditions(&self) -> Result<Vec<ConditionImpression>, String> {
        self.avec(|b| b.conditions())
    }

    /// Range une mesure et son nom dans une condition d'impression (tâche
    /// Mesurer).
    pub fn enregistrer_mesure_nommee(
        &self,
        condition: IdCondition,
        mesure: &pont_protocole::Mesure,
        nom: &str,
    ) -> Result<IdMesure, String> {
        self.avec(|b| b.enregistrer_mesure_nommee(condition, mesure, nom))
    }

    /// Renomme une mesure rangée (tâche Mesurer).
    pub fn renommer_mesure(&self, id: IdMesure, nom: &str) -> Result<(), String> {
        self.avec(|b| b.renommer_mesure(id, nom))
    }
    /// Nom de fichier proposé pour l'export d'une mesure : sa date, telle que
    /// le pont l'a écrite (`mesure-2026-10-06-0921.txt`).
    pub fn nom_export(&self, id: IdMesure) -> Result<String, String> {
        self.avec(|b| {
            let mesure = b.mesure(id)?;
            let h = mesure.mesure.provenance().horodatage.texte();
            let jour = h.get(..10).unwrap_or("mesure");
            let heure: String = h.get(11..16).unwrap_or_default().replace(':', "");
            Ok(format!("mesure-{jour}-{heure}.txt"))
        })
    }

    /// Écrit une mesure en CGATS.17 dans le fichier.
    pub fn exporter_cgats(&self, id: IdMesure, fichier: &Path) -> Result<(), String> {
        let mesure = self.avec(|b| b.mesure(id))?.mesure;
        let texte = cgats::ecrire(&mesure).map_err(|e| {
            eprintln!("{e}");
            "bibliotheque.erreur.export".to_string()
        })?;
        std::fs::write(fichier, texte).map_err(|e| {
            eprintln!("{} : {e}", fichier.display());
            "bibliotheque.erreur.export".to_string()
        })
    }

    /// Sauvegarde toute la bibliothèque dans un seul fichier.
    pub fn sauvegarder(&self, fichier: &Path) -> Result<(), String> {
        self.avec(|b| b.sauvegarder(fichier)).map_err(|cle| {
            if cle == "bibliotheque.erreur.autre" {
                "bibliotheque.erreur.export".to_string()
            } else {
                cle
            }
        })
    }

    /// Remplace toute la bibliothèque par une sauvegarde. L'aperçu d'import en
    /// cours est abandonné d'abord : il visait l'ancienne bibliothèque, et ne
    /// doit jamais être rangé dans la restaurée.
    pub fn restaurer(&self, fichier: &Path) -> Result<(), String> {
        // Pris et rendu avant le verrou de la bibliothèque, comme partout.
        self.annuler_apercu();
        let bibliotheque = self.bibliotheque.as_ref().map_err(|e| {
            eprintln!("{e}");
            "bibliotheque.erreur.ouverture".to_string()
        })?;
        let mut bibliotheque = bibliotheque.lock().unwrap_or_else(|e| e.into_inner());
        bibliotheque
            .restaurer(fichier)
            .map_err(|e| cle_erreur(&e).to_string())
    }

    /// Lit un fichier CGATS et le garde en aperçu, sans rien ranger : la
    /// mesure n'entre dans la bibliothèque qu'avec [`Self::ranger_apercu`].
    /// Un fichier de plus de [`TAILLE_MAX_CGATS`] est refusé avant d'être lu.
    pub fn apercu_cgats(&self, fichier: &Path) -> Result<ApercuImport, String> {
        let illisible = |e: std::io::Error| {
            eprintln!("{} : {e}", fichier.display());
            "bibliotheque.erreur.cgats_illisible".to_string()
        };
        let taille = std::fs::metadata(fichier).map_err(illisible)?.len();
        if taille > TAILLE_MAX_CGATS {
            eprintln!("{} : {taille} octets", fichier.display());
            return Err("bibliotheque.erreur.cgats_trop_gros".to_string());
        }
        // Lecture bornée aussi : le fichier a pu grossir depuis sa taille lue.
        let mut octets = Vec::new();
        std::io::Read::read_to_end(
            &mut std::io::Read::take(
                std::fs::File::open(fichier).map_err(illisible)?,
                TAILLE_MAX_CGATS + 1,
            ),
            &mut octets,
        )
        .map_err(illisible)?;
        if octets.len() as u64 > TAILLE_MAX_CGATS {
            return Err("bibliotheque.erreur.cgats_trop_gros".to_string());
        }
        // UTF-8, ou Latin-1 de Windows sans abîmer les accents.
        let texte = cgats::decoder(&octets);
        let mesure = cgats::lire(&texte).map_err(|e| {
            eprintln!("{} : {e}", fichier.display());
            "bibliotheque.erreur.cgats_illisible".to_string()
        })?;
        let nom = nom_de(fichier);
        let contenu = ContenuImport::de(mesure, nom.clone());
        // Chaque aperçu a son numéro : « Ranger » doit le citer, un aperçu
        // remplacé ou abandonné ne range jamais rien.
        let mut apercu = self.apercu.lock().unwrap_or_else(|e| e.into_inner());
        let numero = self
            .numero_apercu
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        *apercu = Some(Apercu {
            numero,
            fichier: nom,
            texte,
        });
        Ok(ApercuImport { numero, contenu })
    }

    /// Range dans la condition d'impression la mesure de l'aperçu `numero`.
    /// Un aperçu remplacé, abandonné ou d'avant une restauration est refusé.
    /// Un refus de la bibliothèque garde l'aperçu, pour réessayer avec une
    /// autre condition ; une fois rangée, l'aperçu est consommé.
    pub fn ranger_apercu(&self, numero: u64, condition: IdCondition) -> Result<IdMesure, String> {
        let mut apercu = self.apercu.lock().unwrap_or_else(|e| e.into_inner());
        let Some(a) = apercu.as_ref().filter(|a| a.numero == numero) else {
            return Err("bibliotheque.erreur.apercu_absent".to_string());
        };
        let id = self.avec(|b| b.importer_mesure(condition, &a.fichier, &a.texte))?;
        *apercu = None;
        Ok(id)
    }

    /// Abandonne la mesure en aperçu : rien n'est rangé.
    pub fn annuler_apercu(&self) {
        *self.apercu.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn detail_importee(&self, id: IdMesure) -> Result<DetailImportee, String> {
        self.avec(|b| {
            let enregistree = b.mesure_importee(id)?;
            let nom = b
                .conditions()?
                .into_iter()
                .find(|c| c.id == enregistree.condition)
                .map(|c| c.nom)
                .unwrap_or_default();
            Ok(DetailImportee {
                id: enregistree.id,
                condition: enregistree.condition,
                nom_condition: nom,
                contenu: ContenuImport::de(enregistree.mesure, enregistree.fichier),
            })
        })
    }
}

/// Fichier CGATS lu et montré en aperçu, pas encore rangé.
struct Apercu {
    numero: u64,
    fichier: String,
    texte: String,
}

/// Taille au-delà de laquelle un fichier CGATS n'est pas lu : 16 Mio, bien
/// plus qu'une mire de plusieurs milliers de plages avec leurs spectres.
pub const TAILLE_MAX_CGATS: u64 = 16 * 1024 * 1024;

/// Ce que l'écran montre d'une mesure importée, en aperçu ou rangée : ce que
/// le fichier contient, et ce qui y reste inconnu.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ContenuImport {
    /// Nom du fichier d'origine, sans le dossier.
    pub fichier: String,
    pub plages: usize,
    /// Écrit par myiro-libre (provenance complète dans l'en-tête).
    pub myiro_libre: bool,
    pub instrument: Info<String>,
    pub numero_serie: Info<String>,
    pub date: Info<String>,
    /// Condition de mesure de chaque emplacement de spectre.
    pub conditions: [Info<ConditionMesure>; 3],
    /// Le fichier donne-t-il un spectre pour chaque emplacement ?
    pub spectres: [bool; 3],
    /// Lab de chaque plage, par emplacement ; inconnu s'il manque au fichier.
    pub lab: Vec<[Info<[f32; 3]>; 3]>,
}

impl ContenuImport {
    fn de(m: MesureImportee, fichier: String) -> ContenuImport {
        let spectres = [0, 1, 2].map(|e| m.plages.iter().any(|p| p.spectres[e] != Info::Inconnue));
        let lab = m
            .plages
            .iter()
            .map(|p| {
                p.lab.clone().map(|l| match l {
                    Info::Confirmee(l) => Info::Confirmee(l.valeurs()),
                    Info::Supposee(l) => Info::Supposee(l.valeurs()),
                    Info::Inconnue => Info::Inconnue,
                })
            })
            .collect();
        ContenuImport {
            fichier,
            plages: m.plages.len(),
            myiro_libre: m.provenance != Info::Inconnue,
            instrument: m.instrument,
            numero_serie: m.numero_serie,
            date: m.date,
            conditions: m.conditions,
            spectres,
            lab,
        }
    }
}

/// Aperçu d'un fichier CGATS lu, pas encore rangé, avec son numéro.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApercuImport {
    pub numero: u64,
    #[serde(flatten)]
    pub contenu: ContenuImport,
}

/// Une mesure importée rangée dans la bibliothèque.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DetailImportee {
    pub id: IdMesure,
    pub condition: IdCondition,
    pub nom_condition: String,
    #[serde(flatten)]
    pub contenu: ContenuImport,
}

/// Clé du catalogue qui explique une erreur à l'utilisateur. Le détail
/// technique part dans la console, pas à l'écran.
pub fn cle_erreur(erreur: &ErreurBibliotheque) -> &'static str {
    match erreur {
        ErreurBibliotheque::NomVide => "bibliotheque.erreur.nom_vide",
        ErreurBibliotheque::NomDejaPris(_) => "bibliotheque.erreur.nom_pris",
        ErreurBibliotheque::ImportIllisible(detail) => {
            eprintln!("{detail}");
            "bibliotheque.erreur.cgats_illisible"
        }
        ErreurBibliotheque::SauvegardeInvalide(detail) => {
            eprintln!("{detail}");
            "bibliotheque.erreur.sauvegarde_invalide"
        }
        autre => {
            eprintln!("{autre}");
            "bibliotheque.erreur.autre"
        }
    }
}

/// Ce que la feuille du centre et le cartouche montrent d'une mesure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DetailMesure {
    pub id: IdMesure,
    pub condition: IdCondition,
    /// Nom de la condition d'impression, même si la recherche la masque à gauche.
    pub nom_condition: String,
    pub horodatage: String,
    pub instrument: Instrument,
    pub etalonnage: Info<Horodatage>,
    pub geometrie: Geometrie,
    /// Condition de mesure de chacun des trois spectres d'une plage, telle
    /// que le pont l'a demandée : inconnue ou supposée si le pont l'a dit
    /// ainsi. C'est la seule source des libellés M0, M1, M2 à l'écran.
    pub conditions_mesure: Info<[Info<ConditionMesure>; 3]>,
    /// Lab de chaque plage : trois valeurs, une par spectre, dans l'ordre des
    /// spectres de la plage (leur condition est dans `conditions_mesure`).
    pub lab: Vec<[[f32; 3]; 3]>,
}

impl DetailMesure {
    pub fn de(enregistree: &MesureEnregistree, nom_condition: String) -> DetailMesure {
        let p = enregistree.mesure.provenance();
        let conditions_mesure = match &p.calcul.demande {
            Info::Confirmee(c) => Info::Confirmee(c.conditions_spectres.clone()),
            Info::Supposee(c) => Info::Supposee(c.conditions_spectres.clone()),
            Info::Inconnue => Info::Inconnue,
        };
        DetailMesure {
            id: enregistree.id,
            condition: enregistree.condition,
            nom_condition,
            horodatage: p.horodatage.texte().to_string(),
            instrument: enregistree.instrument.clone(),
            etalonnage: p.etalonnage.clone(),
            geometrie: p.geometrie.clone(),
            conditions_mesure,
            lab: enregistree
                .mesure
                .plages()
                .iter()
                .map(|plage| plage.lab().map(|l| l.valeurs()))
                .collect(),
        }
    }
}

/// Version de l'application, pour le pied du cartouche.
#[tauri::command]
pub fn version_application() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn bibliotheque_demonstration(ouverte: tauri::State<'_, BibliothequeOuverte>) -> bool {
    ouverte.demonstration
}

#[tauri::command]
pub fn bibliotheque_arborescence(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    recherche: &str,
) -> Result<Vec<Branche>, String> {
    ouverte.arborescence(recherche)
}

#[tauri::command]
pub fn bibliotheque_detail_mesure(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    id: i64,
) -> Result<DetailMesure, String> {
    ouverte.detail_mesure(IdMesure(id))
}

#[tauri::command]
pub fn bibliotheque_creer_condition(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    nom: &str,
) -> Result<ConditionImpression, String> {
    ouverte.creer_condition(nom)
}

#[tauri::command]
pub fn bibliotheque_renommer_condition(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    id: i64,
    nom: &str,
) -> Result<(), String> {
    ouverte.renommer_condition(IdCondition(id), nom)
}

/// Fichier choisi dans le sélecteur de Windows ; `None` si l'opérateur a annulé.
/// Chaque commande reçoit de la page le nom du type de fichier (`filtre`),
/// en mots simples et dans la langue de l'écran.
fn choisi(fichier: Option<tauri_plugin_dialog::FilePath>) -> Option<PathBuf> {
    fichier?.into_path().ok()
}

/// Exporte la mesure en CGATS : le sélecteur de Windows demande où. Rend le
/// nom du fichier écrit, ou `None` si l'opérateur a annulé.
#[tauri::command(async)]
pub fn bibliotheque_exporter_cgats(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    id: i64,
    filtre: String,
) -> Result<Option<String>, String> {
    let id = IdMesure(id);
    let nom = ouverte.nom_export(id)?;
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .set_file_name(nom)
            .add_filter(filtre, &["txt"])
            .blocking_save_file(),
    ) else {
        return Ok(None);
    };
    ouverte.exporter_cgats(id, &fichier)?;
    Ok(Some(nom_de(&fichier)))
}

/// Sauvegarde toute la bibliothèque, à l'endroit choisi dans le sélecteur.
#[tauri::command(async)]
pub fn bibliotheque_sauvegarder(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    filtre: String,
) -> Result<Option<String>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .set_file_name("bibliotheque-myiro-libre.sqlite")
            .add_filter(&filtre, &["sqlite"])
            .blocking_save_file(),
    ) else {
        return Ok(None);
    };
    ouverte.sauvegarder(&fichier)?;
    Ok(Some(nom_de(&fichier)))
}

/// Remplace la bibliothèque par la sauvegarde choisie dans le sélecteur. La
/// page a demandé l'accord de l'opérateur avant.
#[tauri::command(async)]
pub fn bibliotheque_restaurer(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    filtre: String,
) -> Result<Option<String>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .add_filter(&filtre, &["sqlite"])
            .blocking_pick_file(),
    ) else {
        return Ok(None);
    };
    ouverte.restaurer(&fichier)?;
    Ok(Some(nom_de(&fichier)))
}

/// Ouvre le fichier CGATS choisi dans le sélecteur et le montre en aperçu,
/// sans rien ranger. `None` si l'opérateur a annulé.
#[tauri::command(async)]
pub fn bibliotheque_apercu_cgats(
    app: tauri::AppHandle,
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    filtre: String,
) -> Result<Option<ApercuImport>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .add_filter(filtre, &["txt", "cgats", "it8", "ti3"])
            .blocking_pick_file(),
    ) else {
        return Ok(None);
    };
    ouverte.apercu_cgats(&fichier).map(Some)
}

/// Range la mesure en aperçu dans la condition d'impression, à la demande
/// de l'opérateur (bouton « Ranger dans … »).
#[tauri::command]
pub fn bibliotheque_ranger_import(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    numero: u64,
    condition: i64,
) -> Result<IdMesure, String> {
    ouverte.ranger_apercu(numero, IdCondition(condition))
}

/// Abandonne l'aperçu (bouton « Annuler ») : rien n'est rangé.
#[tauri::command]
pub fn bibliotheque_annuler_import(ouverte: tauri::State<'_, BibliothequeOuverte>) {
    ouverte.annuler_apercu();
}

#[tauri::command]
pub fn bibliotheque_detail_importee(
    ouverte: tauri::State<'_, BibliothequeOuverte>,
    id: i64,
) -> Result<DetailImportee, String> {
    ouverte.detail_importee(IdMesure(id))
}

fn nom_de(fichier: &Path) -> String {
    fichier
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demonstration() -> (tempfile::TempDir, BibliothequeOuverte) {
        let dossier = tempfile::tempdir().unwrap();
        let ouverte = BibliothequeOuverte::demonstration(dossier.path());
        (dossier, ouverte)
    }

    /// Le détail ne transforme jamais la position d'un spectre en condition de
    /// mesure : ce que le pont n'a pas dit reste inconnu, ce qu'il suppose
    /// reste supposé (ADR 0005).
    #[test]
    fn le_detail_garde_les_conditions_de_mesure_inconnues_ou_supposees() {
        use pont_protocole::{ConditionsCalcul, Mesure};
        let (dossier, ouverte) = demonstration();
        let id = ouverte.arborescence("").unwrap()[0].mesures[1].id;
        drop(ouverte);
        let enregistree = Bibliotheque::ouvrir(dossier.path())
            .unwrap()
            .mesure(id)
            .unwrap();
        let avec_demande = |demande: Info<ConditionsCalcul>| {
            let mut provenance = enregistree.mesure.provenance().clone();
            provenance.calcul.demande = demande;
            MesureEnregistree {
                mesure: Mesure::new(enregistree.mesure.plages().to_vec(), provenance).unwrap(),
                ..enregistree.clone()
            }
        };

        let inconnue = DetailMesure::de(&avec_demande(Info::Inconnue), String::new());
        assert_eq!(inconnue.conditions_mesure, Info::Inconnue);
        assert_eq!(inconnue.lab.len(), 1);

        let Some(mut conditions) = enregistree
            .mesure
            .provenance()
            .calcul
            .demande
            .valeur()
            .cloned()
        else {
            panic!("la démonstration dit ses conditions");
        };
        conditions.conditions_spectres = [
            Info::Inconnue,
            Info::Supposee(ConditionMesure::M1),
            Info::Confirmee(ConditionMesure::M2),
        ];
        let melangee = DetailMesure::de(&avec_demande(Info::Supposee(conditions)), String::new());
        assert_eq!(
            melangee.conditions_mesure,
            Info::Supposee([
                Info::Inconnue,
                Info::Supposee(ConditionMesure::M1),
                Info::Confirmee(ConditionMesure::M2),
            ])
        );
    }

    /// Le cartouche nomme la condition d'impression de la mesure même quand
    /// la recherche l'a retirée de la colonne de gauche.
    #[test]
    fn le_detail_nomme_la_condition_de_la_mesure() {
        let (_dossier, ouverte) = demonstration();
        let mesure = ouverte.arborescence("offset").unwrap()[0].mesures[0].id;
        assert!(ouverte.arborescence("jet").unwrap()[0]
            .mesures
            .iter()
            .all(|m| m.id != mesure));
        assert_eq!(
            ouverte.detail_mesure(mesure).unwrap().nom_condition,
            "Offset, couché mat 150\u{202f}g"
        );
    }

    #[test]
    fn la_demonstration_montre_trois_conditions_dont_une_vide() {
        let (_dossier, ouverte) = demonstration();
        let branches = ouverte.arborescence("").unwrap();
        let resume: Vec<_> = branches
            .iter()
            .map(|b| (b.condition.nom.as_str(), b.mesures.len()))
            .collect();
        assert_eq!(
            resume,
            [
                ("Jet d’encre, brillant 260\u{202f}g", 2),
                ("Offset, couché mat 150\u{202f}g", 2),
                ("Sérigraphie, adhésif blanc", 0),
            ]
        );
        assert!(branches
            .iter()
            .flat_map(|b| &b.mesures)
            .all(|m| m.instrument.numero_serie == 12345678));
        // Une mesure importée, à part, marquée par son fichier.
        let fichiers: Vec<_> = branches
            .iter()
            .map(|b| {
                b.importees
                    .iter()
                    .map(|m| m.fichier.as_str())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(fichiers, [vec!["releve-lab.txt"], vec![], vec![]]);
        // Assez de plages pour voir le tableau défiler seul (ticket #9).
        assert_eq!(branches[0].importees[0].plages, 36);
    }

    #[test]
    fn la_demonstration_repart_de_zero_a_chaque_lancement() {
        let dossier = tempfile::tempdir().unwrap();
        drop(BibliothequeOuverte::demonstration(dossier.path()));
        let ouverte = BibliothequeOuverte::demonstration(dossier.path());
        assert_eq!(ouverte.arborescence("").unwrap().len(), 3);
    }

    #[test]
    fn le_detail_d_une_bande_donne_le_lab_de_chaque_plage_en_m0_m1_m2() {
        let (_dossier, ouverte) = demonstration();
        let bande = ouverte.arborescence("").unwrap()[0].mesures[0].clone();
        assert_eq!(bande.geometrie, Geometrie::Bande { sens: 0 });

        let detail = ouverte.detail_mesure(bande.id).unwrap();
        assert_eq!(detail.lab.len(), 4);
        // Le noir (dernière plage) est sombre, le jaune est jaune.
        assert!(detail.lab[3][0][0] < 30.0);
        assert!(detail.lab[2][1][2] > 40.0);
        assert_eq!(
            detail.conditions_mesure,
            Info::Confirmee([
                Info::Confirmee(ConditionMesure::M0),
                Info::Confirmee(ConditionMesure::M1),
                Info::Confirmee(ConditionMesure::M2),
            ])
        );
        assert_eq!(
            detail.etalonnage,
            Info::Confirmee(Horodatage::new("2026-10-06T09:12:00+02:00").unwrap())
        );
    }

    #[test]
    fn les_refus_arrivent_a_la_page_comme_des_cles_du_catalogue() {
        let (_dossier, ouverte) = demonstration();
        assert_eq!(
            ouverte.creer_condition("  "),
            Err("bibliotheque.erreur.nom_vide".to_string())
        );
        assert_eq!(
            ouverte.creer_condition("Offset, couché mat 150\u{202f}g"),
            Err("bibliotheque.erreur.nom_pris".to_string())
        );
        assert_eq!(
            ouverte.detail_mesure(IdMesure(999)),
            Err("bibliotheque.erreur.autre".to_string())
        );
        for cle in [
            "bibliotheque.erreur.nom_vide",
            "bibliotheque.erreur.nom_pris",
            "bibliotheque.erreur.autre",
            "bibliotheque.erreur.ouverture",
        ] {
            assert!(
                crate::textes::cles().any(|c| c == cle),
                "{cle} manque au catalogue"
            );
        }
    }

    #[test]
    fn une_mesure_exportee_en_cgats_se_relit_avec_ses_lab() {
        let (dossier, ouverte) = demonstration();
        let bande = ouverte.arborescence("").unwrap()[0].mesures[0].clone();
        let fichier = dossier.path().join("export.txt");
        ouverte.exporter_cgats(bande.id, &fichier).unwrap();

        let relue = cgats::lire(&std::fs::read_to_string(&fichier).unwrap()).unwrap();
        let detail = ouverte.detail_mesure(bande.id).unwrap();
        assert_eq!(relue.plages.len(), detail.lab.len());
        assert_eq!(
            relue.plages[3].lab[1],
            Info::Confirmee(pont_protocole::Lab::new(detail.lab[3][1]).unwrap())
        );
        assert_eq!(
            ouverte.nom_export(bande.id).unwrap(),
            "mesure-2026-10-06-0921.txt"
        );
    }

    #[test]
    fn une_sauvegarde_se_restaure_et_un_mauvais_fichier_est_refuse_en_clair() {
        let (dossier, ouverte) = demonstration();
        let avant = ouverte.arborescence("").unwrap();
        let fichier = dossier.path().join("sauvegarde.sqlite");
        ouverte.sauvegarder(&fichier).unwrap();
        ouverte
            .creer_condition("Ajoutée après la sauvegarde")
            .unwrap();

        ouverte.restaurer(&fichier).unwrap();
        assert_eq!(ouverte.arborescence("").unwrap(), avant);

        let texte = dossier.path().join("texte.sqlite");
        std::fs::write(&texte, "pas une sauvegarde").unwrap();
        assert_eq!(
            ouverte.restaurer(&texte),
            Err("bibliotheque.erreur.sauvegarde_invalide".to_string())
        );
        assert_eq!(ouverte.arborescence("").unwrap(), avant);
    }

    /// Import en deux temps, comme à l'écran : aperçu, puis rangement.
    fn importer(ouverte: &BibliothequeOuverte, condition: IdCondition, fichier: &Path) -> IdMesure {
        let apercu = ouverte.apercu_cgats(fichier).unwrap();
        ouverte.ranger_apercu(apercu.numero, condition).unwrap()
    }

    fn fichier_lab(dossier: &Path) -> PathBuf {
        let fichier = dossier.join("lab.txt");
        std::fs::write(
            &fichier,
            "CGATS.17\nMEASUREMENT_SOURCE\t\"D50\"\nBEGIN_DATA_FORMAT\n\
             SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\nEND_DATA_FORMAT\nBEGIN_DATA\n\
             1\t50.0\t1.0\t-2.0\nEND_DATA\n",
        )
        .unwrap();
        fichier
    }

    #[test]
    fn un_apercu_ne_range_rien_avant_l_accord() {
        let (dossier, ouverte) = demonstration();
        let offset = ouverte.arborescence("").unwrap()[1].condition.id;
        let fichier = fichier_lab(dossier.path());
        let importees = |o: &BibliothequeOuverte| o.arborescence("").unwrap()[1].importees.len();

        let apercu = ouverte.apercu_cgats(&fichier).unwrap();
        assert_eq!(apercu.contenu.fichier, "lab.txt");
        assert_eq!(apercu.contenu.plages, 1);
        assert_eq!(importees(&ouverte), 0, "un aperçu ne range rien");

        ouverte.annuler_apercu();
        let absent = Err("bibliotheque.erreur.apercu_absent".to_string());
        assert_eq!(ouverte.ranger_apercu(apercu.numero, offset), absent);
        assert_eq!(importees(&ouverte), 0);

        let apercu = ouverte.apercu_cgats(&fichier).unwrap();
        // Un refus garde l'aperçu : l'opérateur peut choisir une autre condition.
        assert!(ouverte
            .ranger_apercu(apercu.numero, IdCondition(999))
            .is_err());
        let id = ouverte.ranger_apercu(apercu.numero, offset).unwrap();
        assert_eq!(ouverte.arborescence("").unwrap()[1].importees[0].id, id);
        // Rangé une fois : l'aperçu est consommé.
        assert_eq!(ouverte.ranger_apercu(apercu.numero, offset), absent);
        assert_eq!(importees(&ouverte), 1);
    }

    #[test]
    fn un_apercu_perime_ou_d_avant_une_restauration_est_refuse() {
        let (dossier, ouverte) = demonstration();
        let offset = ouverte.arborescence("").unwrap()[1].condition.id;
        let fichier = fichier_lab(dossier.path());
        let absent = Err("bibliotheque.erreur.apercu_absent".to_string());

        // Un nouvel aperçu remplace l'ancien : l'ancien numéro ne range rien.
        let ancien = ouverte.apercu_cgats(&fichier).unwrap();
        let nouveau = ouverte.apercu_cgats(&fichier).unwrap();
        assert_ne!(ancien.numero, nouveau.numero);
        assert_eq!(ouverte.ranger_apercu(ancien.numero, offset), absent);

        // Une restauration abandonne l'aperçu en cours.
        let sauvegarde = dossier.path().join("sauvegarde.sqlite");
        ouverte.sauvegarder(&sauvegarde).unwrap();
        ouverte.restaurer(&sauvegarde).unwrap();
        assert_eq!(ouverte.ranger_apercu(nouveau.numero, offset), absent);
        assert!(ouverte.arborescence("").unwrap()[1].importees.is_empty());
    }

    #[test]
    fn un_fichier_cgats_importe_est_range_dans_sa_condition_sans_rien_deviner() {
        let (dossier, ouverte) = demonstration();
        let branches = ouverte.arborescence("").unwrap();
        let bande = branches[0].mesures[0].id;
        let offset = branches[1].condition.id;
        let fichier = dossier.path().join("export.txt");
        ouverte.exporter_cgats(bande, &fichier).unwrap();

        let id = importer(&ouverte, offset, &fichier);
        let rangee = &ouverte.arborescence("").unwrap()[1];
        assert_eq!(rangee.importees.len(), 1);
        assert_eq!(rangee.importees[0].id, id);
        assert_eq!(rangee.importees[0].fichier, "export.txt");

        let detail = ouverte.detail_importee(id).unwrap();
        assert_eq!(detail.contenu.fichier, "export.txt");
        assert_eq!(detail.nom_condition, "Offset, couché mat 150\u{202f}g");
        assert_eq!(detail.contenu.plages, 4);
        assert!(detail.contenu.myiro_libre);
        assert_eq!(detail.contenu.instrument, Info::Confirmee("MYIRO-1".into()));
        assert_eq!(detail.contenu.spectres, [true, true, true]);
        assert_eq!(
            detail.contenu.conditions,
            [
                Info::Confirmee(ConditionMesure::M0),
                Info::Confirmee(ConditionMesure::M1),
                Info::Confirmee(ConditionMesure::M2),
            ]
        );
        let origine = ouverte.detail_mesure(bande).unwrap();
        assert_eq!(detail.contenu.lab[3][1], Info::Confirmee(origine.lab[3][1]));

        // Un fichier sans spectre : les spectres restent inconnus.
        let sans_spectre = dossier.path().join("lab.txt");
        std::fs::write(
            &sans_spectre,
            "CGATS.17\nMEASUREMENT_SOURCE\t\"D50\"\nNUMBER_OF_FIELDS\t4\nBEGIN_DATA_FORMAT\n\
             SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\nEND_DATA_FORMAT\nNUMBER_OF_SETS\t1\nBEGIN_DATA\n\
             1\t50.0\t1.0\t-2.0\nEND_DATA\n",
        )
        .unwrap();
        let id = importer(&ouverte, offset, &sans_spectre);
        let detail = ouverte.detail_importee(id).unwrap();
        assert!(!detail.contenu.myiro_libre);
        assert_eq!(detail.contenu.spectres, [false, false, false]);
        assert_eq!(
            detail.contenu.lab[0],
            [
                Info::Inconnue,
                Info::Confirmee([50.0, 1.0, -2.0]),
                Info::Inconnue
            ]
        );
        assert_eq!(detail.contenu.instrument, Info::Inconnue);
        assert_eq!(detail.contenu.date, Info::Inconnue);

        let illisible = dossier.path().join("illisible.txt");
        std::fs::write(&illisible, "rien de CGATS").unwrap();
        assert_eq!(
            ouverte.apercu_cgats(&illisible).map(|_| ()),
            Err("bibliotheque.erreur.cgats_illisible".to_string())
        );
        assert_eq!(ouverte.arborescence("").unwrap()[1].importees.len(), 2);
    }

    #[test]
    fn un_fichier_trop_gros_est_refuse_avant_lecture_et_le_latin_1_garde_ses_accents() {
        let (dossier, ouverte) = demonstration();
        let offset = ouverte.arborescence("").unwrap()[1].condition.id;
        let gros = dossier.path().join("gros.txt");
        std::fs::File::create(&gros)
            .unwrap()
            .set_len(16 * 1024 * 1024 + 1)
            .unwrap();
        assert_eq!(
            ouverte.apercu_cgats(&gros).map(|_| ()),
            Err("bibliotheque.erreur.cgats_trop_gros".to_string())
        );

        let latin = dossier.path().join("latin.txt");
        std::fs::write(
            &latin,
            b"CGATS.17\nINSTRUMENTATION\t\"Relev\xe9 fictif\"\nMEASUREMENT_SOURCE\t\"D50\"\n\
              BEGIN_DATA_FORMAT\nSAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\nEND_DATA_FORMAT\n\
              BEGIN_DATA\n1\t50.0\t1.0\t-2.0\nEND_DATA\n",
        )
        .unwrap();
        let id = importer(&ouverte, offset, &latin);
        assert_eq!(
            ouverte.detail_importee(id).unwrap().contenu.instrument,
            Info::Confirmee("Relevé fictif".into())
        );
    }

    #[test]
    fn les_textes_de_l_export_existent_au_catalogue() {
        for cle in [
            "bibliotheque.erreur.cgats_trop_gros",
            "bibliotheque.erreur.apercu_absent",
            "import.abandonne",
            "bibliotheque.erreur.sauvegarde_invalide",
            "bibliotheque.erreur.cgats_illisible",
            "bibliotheque.erreur.export",
        ] {
            assert!(
                crate::textes::cles().any(|c| c == cle),
                "{cle} manque au catalogue"
            );
        }
    }

    #[test]
    fn une_condition_creee_puis_renommee_apparait_sous_son_nouveau_nom() {
        let dossier = tempfile::tempdir().unwrap();
        let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
        let creee = ouverte.creer_condition("Offset").unwrap();
        ouverte
            .renommer_condition(creee.id, "Offset, couché mat")
            .unwrap();
        let noms: Vec<_> = ouverte
            .arborescence("")
            .unwrap()
            .into_iter()
            .map(|b| b.condition.nom)
            .collect();
        assert_eq!(noms, ["Offset, couché mat"]);
    }
}
