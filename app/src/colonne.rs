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
use pont_protocole::{ConditionMesure, Geometrie, Horodatage, Info};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// La bibliothèque ouverte au lancement, partagée par les commandes.
pub struct BibliothequeOuverte {
    bibliotheque: Result<Mutex<Bibliotheque>, ErreurBibliotheque>,
    demonstration: bool,
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

    /// Remplace toute la bibliothèque par une sauvegarde.
    pub fn restaurer(&self, fichier: &Path) -> Result<(), String> {
        let bibliotheque = self.bibliotheque.as_ref().map_err(|e| {
            eprintln!("{e}");
            "bibliotheque.erreur.ouverture".to_string()
        })?;
        let mut bibliotheque = bibliotheque.lock().unwrap_or_else(|e| e.into_inner());
        bibliotheque
            .restaurer(fichier)
            .map_err(|e| cle_erreur(&e).to_string())
    }
}

/// Ce que l'écran montre d'un fichier CGATS importé : ce qu'il contient, et
/// ce qui y reste inconnu.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResumeImport {
    /// Nom du fichier, sans le dossier.
    pub fichier: String,
    pub plages: usize,
    /// Écrit par myiro-libre (provenance complète dans l'en-tête).
    pub myiro_libre: bool,
    pub instrument: Info<String>,
    pub numero_serie: Info<String>,
    pub date: Info<String>,
    /// Condition de mesure de chaque emplacement de spectre.
    pub conditions: [Info<ConditionMesure>; 3],
    /// Le fichier donne-t-il le spectre, le Lab, de chaque emplacement ?
    pub spectres: [bool; 3],
    pub lab: [bool; 3],
}

/// Lit un fichier CGATS en mesure importée et le résume pour l'écran.
pub fn importer_cgats(fichier: &Path) -> Result<ResumeImport, String> {
    let illisible = |detail: String| {
        eprintln!("{} : {detail}", fichier.display());
        "bibliotheque.erreur.cgats_illisible".to_string()
    };
    let octets = std::fs::read(fichier).map_err(|e| illisible(e.to_string()))?;
    // Les exports d'autres logiciels ne sont pas toujours en UTF-8.
    let texte = String::from_utf8_lossy(&octets);
    let importee = cgats::lire(&texte).map_err(|e| illisible(e.to_string()))?;
    let present = |quoi: &dyn Fn(&cgats::PlageImportee, usize) -> bool| {
        [0, 1, 2].map(|e| importee.plages.iter().any(|p| quoi(p, e)))
    };
    Ok(ResumeImport {
        fichier: nom_de(fichier),
        plages: importee.plages.len(),
        myiro_libre: importee.provenance != Info::Inconnue,
        instrument: importee.instrument.clone(),
        numero_serie: importee.numero_serie.clone(),
        date: importee.date.clone(),
        conditions: importee.conditions.clone(),
        spectres: present(&|p, e| p.spectres[e] != Info::Inconnue),
        lab: present(&|p, e| p.lab[e] != Info::Inconnue),
    })
}

/// Clé du catalogue qui explique une erreur à l'utilisateur. Le détail
/// technique part dans la console, pas à l'écran.
pub fn cle_erreur(erreur: &ErreurBibliotheque) -> &'static str {
    match erreur {
        ErreurBibliotheque::NomVide => "bibliotheque.erreur.nom_vide",
        ErreurBibliotheque::NomDejaPris(_) => "bibliotheque.erreur.nom_pris",
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
) -> Result<Option<String>, String> {
    let id = IdMesure(id);
    let nom = ouverte.nom_export(id)?;
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .set_file_name(nom)
            .add_filter("CGATS", &["txt"])
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
) -> Result<Option<String>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .set_file_name("bibliotheque-myiro-libre.sqlite")
            .add_filter("SQLite", &["sqlite"])
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
) -> Result<Option<String>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .add_filter("SQLite", &["sqlite"])
            .blocking_pick_file(),
    ) else {
        return Ok(None);
    };
    ouverte.restaurer(&fichier)?;
    Ok(Some(nom_de(&fichier)))
}

/// Lit le fichier CGATS choisi dans le sélecteur et en rend le résumé.
#[tauri::command(async)]
pub fn bibliotheque_importer_cgats(app: tauri::AppHandle) -> Result<Option<ResumeImport>, String> {
    let Some(fichier) = choisi(
        app.dialog()
            .file()
            .add_filter("CGATS", &["txt", "cgats", "it8", "ti3"])
            .blocking_pick_file(),
    ) else {
        return Ok(None);
    };
    importer_cgats(&fichier).map(Some)
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

    #[test]
    fn un_fichier_cgats_importe_est_resume_sans_rien_deviner() {
        let (dossier, ouverte) = demonstration();
        let bande = ouverte.arborescence("").unwrap()[0].mesures[0].id;
        let fichier = dossier.path().join("export.txt");
        ouverte.exporter_cgats(bande, &fichier).unwrap();

        let resume = importer_cgats(&fichier).unwrap();
        assert_eq!(resume.fichier, "export.txt");
        assert_eq!(resume.plages, 4);
        assert!(resume.myiro_libre);
        assert_eq!(resume.instrument, Info::Confirmee("MYIRO-1".into()));
        assert_eq!(resume.spectres, [true, true, true]);
        assert_eq!(
            resume.conditions,
            [
                Info::Confirmee(ConditionMesure::M0),
                Info::Confirmee(ConditionMesure::M1),
                Info::Confirmee(ConditionMesure::M2),
            ]
        );

        // Un fichier sans spectre : les spectres restent inconnus.
        let sans_spectre = dossier.path().join("lab.txt");
        std::fs::write(
            &sans_spectre,
            "CGATS.17\nMEASUREMENT_SOURCE\t\"D50\"\nNUMBER_OF_FIELDS\t4\nBEGIN_DATA_FORMAT\n\
             SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\nEND_DATA_FORMAT\nNUMBER_OF_SETS\t1\nBEGIN_DATA\n\
             1\t50.0\t1.0\t-2.0\nEND_DATA\n",
        )
        .unwrap();
        let resume = importer_cgats(&sans_spectre).unwrap();
        assert!(!resume.myiro_libre);
        assert_eq!(resume.spectres, [false, false, false]);
        assert_eq!(resume.lab, [false, true, false]);
        assert_eq!(resume.instrument, Info::Inconnue);

        let illisible = dossier.path().join("illisible.txt");
        std::fs::write(&illisible, "rien de CGATS").unwrap();
        assert_eq!(
            importer_cgats(&illisible),
            Err("bibliotheque.erreur.cgats_illisible".to_string())
        );
    }

    #[test]
    fn les_textes_de_l_export_existent_au_catalogue() {
        for cle in [
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
