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
}

/// Clé du catalogue qui explique une erreur à l'utilisateur. Le détail
/// technique part dans la console, pas à l'écran.
pub fn cle_erreur(erreur: &ErreurBibliotheque) -> &'static str {
    match erreur {
        ErreurBibliotheque::NomVide => "bibliotheque.erreur.nom_vide",
        ErreurBibliotheque::NomDejaPris(_) => "bibliotheque.erreur.nom_pris",
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
