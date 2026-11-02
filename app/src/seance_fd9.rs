//! Séance de mesure d'un tirage au FD-9 (ticket #35) : plan figé au départ,
//! feuilles mesurées au FD-9 ou exports importés, reprise après fermeture,
//! compléments manuels, exclusions explicites, puis mesure moyennée.
//!
//! Méthodes de [`BibliothequeOuverte`], sans Tauri ; le calcul (plan,
//! association, moyennage, écarts) est celui de la crate `seance-mesure`.
//! Chaque feuille validée est rangée aussitôt : rien ne se perd à la
//! fermeture. Les erreurs rendues sont des clés du catalogue de textes.

use std::path::Path;
use std::time::SystemTime;

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, EtatSeance, FinSeance, IdCondition, IdMesure, IdSeance,
    IdTirage, NouvelleSeance, NouvelleSource, OrigineSource, PlaceEnSeance, RoleSource,
    SeanceRangee,
};
use cgats::MesureImportee;
use mires::{MireFd9, RGB2520_GEOMETRIE};
use pont_protocole::{Info, Palier};
use seance_mesure::{
    associer, moyenner, vue_plages, Association, Complement, CompositionPlage, ConditionEcartee,
    Ecart, IdentiteMire, Incompatible, Moyenne, Plan, RefusPlan, Role, Source, VuePlage,
};
use serde::{Deserialize, Serialize};

use crate::colonne::BibliothequeOuverte;
use crate::instrument::feuille::{definition_rgb2520, ErreurFeuille, EtapeFeuille};
use crate::instrument::Instrument;
use crate::mesurer_feuille::{garder_calage, garder_mesure, nom_feuille, EchecFeuille};
use crate::parc::instant_utc;
use crate::pont::{Panne, Pont};
use crate::verifier::FicheTirage;

/// Pourquoi une séance refuse : clé du catalogue et, pour un moyennage
/// impossible, la règle en cause (source nommée par son numéro).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefusSeance {
    pub cle: String,
    pub incompatible: Option<Incompatible>,
}

impl From<String> for RefusSeance {
    fn from(cle: String) -> Self {
        RefusSeance {
            cle,
            incompatible: None,
        }
    }
}

impl From<&str> for RefusSeance {
    fn from(cle: &str) -> Self {
        cle.to_string().into()
    }
}

impl From<Incompatible> for RefusSeance {
    /// `seance.refus.<règle>`, du nom de la règle dans le JSON.
    fn from(incompatible: Incompatible) -> Self {
        let regle = serde_json::to_value(&incompatible)
            .ok()
            .and_then(|v| v["refus"].as_str().map(String::from))
            .unwrap_or_default();
        RefusSeance {
            cle: format!("seance.refus.{regle}"),
            incompatible: Some(incompatible),
        }
    }
}

/// Une mesure source rangée, avec son association au plan.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceAjoutee {
    pub mesure: IdMesure,
    pub fichier: String,
    pub association: Association,
}

/// Ce que l'opérateur a choisi en fermant la séance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinDemandee {
    /// Fermer malgré des plages manquantes : la séance reste « incomplète »,
    /// et son résultat ne fait pas de profil.
    pub incomplet_assume: bool,
    /// Les écarts signalés entre exemplaires ont été montrés et acceptés.
    pub ecarts_vus: bool,
}

/// Une mesure source, telle que l'écran la montre.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VueSource {
    pub mesure: IdMesure,
    pub fichier: String,
    pub origine: OrigineSource,
    pub role: RoleSource,
    pub association: Association,
    pub retenue: bool,
    /// Le choix de l'opérateur (motif, heure), en JSON.
    pub selection: Option<String>,
    pub horodatage: String,
}

/// Ce que la moyenne donnerait maintenant, avant la fin de la séance.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApercuMoyenne {
    pub plages: usize,
    pub manquantes: Vec<String>,
    pub conditions_ecartees: Vec<ConditionEcartee>,
    pub ecarts: Vec<Ecart>,
    pub ecarts_signales: usize,
}

/// Une mesure source écartée à la main.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    pub source: i64,
    pub selection: Option<String>,
}

/// La composition d'une mesure moyennée, rangée en base avec la séance :
/// le CGATS du résultat ne la porte pas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompositionRangee {
    pub composition: Vec<CompositionPlage>,
    pub manquantes: Vec<String>,
    pub conditions_ecartees: Vec<ConditionEcartee>,
    pub ecarts: Vec<Ecart>,
    pub exclusions: Vec<Exclusion>,
    pub complements: Vec<Complement>,
    pub fin: FinDemandee,
}

/// Une séance, telle que l'écran la montre.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EtatSeanceVue {
    pub id: IdSeance,
    pub condition: IdCondition,
    pub tirage: IdTirage,
    pub plan: Plan,
    pub etat: EtatSeance,
    pub horodatage: String,
    pub fin: Option<String>,
    pub sources: Vec<VueSource>,
    pub complements: Vec<Complement>,
    /// En cours : ce que la moyenne donnerait ; rien si elle est refusée
    /// ([`EtatSeanceVue::refus`]) ou si la séance est fermée.
    pub apercu: Option<ApercuMoyenne>,
    pub refus: Option<RefusSeance>,
    /// Fermée : la mesure moyennée et sa composition.
    pub resultat: Option<IdMesure>,
    pub composition: Option<CompositionRangee>,
}

/// Une valeur JSON rangée par l'application qui ne se relit plus.
fn illisible<T>(quoi: &str, e: serde_json::Error) -> Result<T, String> {
    eprintln!("{quoi} illisible : {e}");
    Err("bibliotheque.erreur.autre".into())
}

fn plan_de(seance: &SeanceRangee) -> Result<Plan, String> {
    serde_json::from_str(&seance.plan).or_else(|e| illisible("plan de la séance", e))
}

fn complements_de(seance: &SeanceRangee) -> Result<Vec<Complement>, String> {
    serde_json::from_str(&seance.complements).or_else(|e| illisible("compléments", e))
}

fn json<T: Serialize>(valeur: &T) -> String {
    serde_json::to_string(valeur).expect("une valeur de séance se sérialise toujours")
}

/// La définition envoyée au FD-9, reconstruite depuis le plan figé : la
/// mire n'est jamais relue sur le disque pendant la séance.
fn definition(plan: &Plan) -> Result<pont_protocole::DefinitionFeuille, ErreurFeuille> {
    let mire = MireFd9 {
        octets: Vec::new(),
        empreinte: plan.mire.empreinte.clone(),
        geometrie: RGB2520_GEOMETRIE,
        valeurs: plan.plages.iter().map(|p| p.rvb).collect(),
    };
    definition_rgb2520(&mire).map_err(ErreurFeuille::Definition)
}

fn role(role: RoleSource) -> Role {
    match role {
        RoleSource::Exemplaire => Role::Exemplaire,
        RoleSource::Complement => Role::Complement,
    }
}

/// Pourquoi une mesure rangée dans une séance ne fait pas de profil, ni ne
/// sert de seconde mire : association incertaine, mesure incomplète,
/// séance en cours, ou résultat d'une séance fermée incomplète. Rien pour
/// une mesure hors séance, ou utilisable.
pub(crate) fn refus_seance(
    b: &Bibliotheque,
    mesure: IdMesure,
) -> Result<Option<&'static str>, ErreurBibliotheque> {
    let Some(situation) = b.situation_en_seance(mesure)? else {
        return Ok(None);
    };
    Ok(match situation.place {
        PlaceEnSeance::Source { association } => {
            match serde_json::from_str::<Association>(&association) {
                Ok(a) if !a.certaine() => Some("profil.refus.association_incertaine"),
                Err(_) => Some("profil.refus.association_incertaine"),
                Ok(a) if !a.complete() => Some("profil.refus.mesure_incomplete"),
                Ok(_) if situation.etat == EtatSeance::EnCours => {
                    Some("profil.refus.seance_en_cours")
                }
                Ok(_) => None,
            }
        }
        PlaceEnSeance::Resultat if situation.etat == EtatSeance::Incomplete => {
            Some("profil.refus.seance_incomplete")
        }
        PlaceEnSeance::Resultat => None,
    })
}

impl BibliothequeOuverte {
    /// Commence la séance d'un tirage FD-9 pas encore mesuré de la
    /// résolution : le plan (mire, plages, fiche confirmée) est figé. Une
    /// séance déjà en cours sur ce tirage est reprise.
    pub fn commencer_seance_fd9(
        &self,
        condition: IdCondition,
        tirage: IdTirage,
        fiche: &FicheTirage,
        maintenant: SystemTime,
    ) -> Result<IdSeance, RefusSeance> {
        self.avec(|b| {
            for id in b.seances_en_cours(condition)? {
                if b.seance(id)?.tirage == tirage {
                    return Ok(Some(id));
                }
            }
            Ok(None)
        })?
        .map_or_else(
            || self.nouvelle_seance(condition, tirage, fiche, maintenant),
            Ok,
        )
    }

    fn nouvelle_seance(
        &self,
        condition: IdCondition,
        tirage: IdTirage,
        fiche: &FicheTirage,
        maintenant: SystemTime,
    ) -> Result<IdSeance, RefusSeance> {
        let prepare = self
            .avec(|b| b.tirages_prepares(condition, "fd9"))?
            .into_iter()
            .find(|t| t.id == tirage)
            .ok_or("seance.erreur.tirage_indisponible")?;
        let valeurs: Vec<[f64; 3]> = serde_json::from_str(&prepare.mire.valeurs)
            .or_else(|e| illisible("valeurs de la mire", e))?;
        let plan = Plan::figer(
            IdentiteMire {
                id: prepare.mire.id.0,
                nom: prepare.mire.nom.clone(),
                empreinte: prepare.mire.empreinte.clone(),
            },
            &valeurs,
            &json(fiche),
            fiche.confirmee,
        )
        .map_err(|refus| match refus {
            RefusPlan::FicheNonConfirmee => "seance.erreur.fiche_non_confirmee",
            RefusPlan::MireVide | RefusPlan::ValeurHorsEchelle { .. } => {
                "seance.erreur.mire_illisible"
            }
        })?;
        Ok(self.avec(|b| {
            b.creer_seance(&NouvelleSeance {
                condition,
                tirage,
                plan: &json(&plan),
                fiche: &plan.fiche,
                horodatage: &instant_utc(maintenant),
            })
        })?)
    }

    /// Les séances en cours de la résolution, à reprendre.
    pub fn seances_en_cours(&self, condition: IdCondition) -> Result<Vec<IdSeance>, String> {
        self.avec(|b| b.seances_en_cours(condition))
    }

    /// La séance, en cours ; refusée si elle est fermée.
    fn seance_en_cours(&self, seance: IdSeance) -> Result<(SeanceRangee, Plan), String> {
        let rangee = self.avec(|b| b.seance(seance))?;
        if rangee.etat != EtatSeance::EnCours {
            return Err("seance.erreur.fermee".into());
        }
        let plan = plan_de(&rangee)?;
        Ok((rangee, plan))
    }

    /// Range une mesure source dans la séance, avec son association au plan.
    #[allow(clippy::too_many_arguments)]
    fn ranger_source(
        &self,
        seance: IdSeance,
        plan: &Plan,
        fichier: &str,
        texte: &str,
        importee: &MesureImportee,
        origine: OrigineSource,
        role: RoleSource,
        maintenant: SystemTime,
    ) -> Result<SourceAjoutee, String> {
        let association = associer(plan, importee);
        let mesure = self.avec(|b| {
            b.ajouter_source(
                seance,
                &NouvelleSource {
                    fichier,
                    texte,
                    origine,
                    role,
                    association: &json(&association),
                    horodatage: &instant_utc(maintenant),
                },
            )
        })?;
        Ok(SourceAjoutee {
            mesure,
            fichier: fichier.rsplit(['/', '\\']).next().unwrap_or(fichier).into(),
            association,
        })
    }

    /// Mesure une feuille de la séance au FD-9 détecté et la range aussitôt
    /// comme mesure source (un exemplaire). La mire envoyée est celle du
    /// plan figé. Une feuille refusée par le pont (mire non reconnue,
    /// plages mal placées…) ne range rien : la séance reste en cours, et la
    /// feuille se remesure. La mesure complète et le calage sont gardés à
    /// côté, dans `<dossier>/mesures-fd9/`, comme au ticket #34.
    pub fn mesurer_exemplaire_fd9<P: Pont>(
        &self,
        seance: IdSeance,
        instrument: &mut Instrument<P>,
        dossier: &Path,
        lancer: &mut impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        suivi: &mut impl FnMut(EtapeFeuille),
    ) -> Result<SourceAjoutee, EchecFeuille> {
        let echec = |e: ErreurFeuille| EchecFeuille {
            cle: e.cle(),
            detail: e.detail(),
        };
        let (_, plan) = self.seance_en_cours(seance)?;
        let definition = definition(&plan).map_err(echec)?;
        let feuille = instrument
            .mesurer_feuille(definition, lancer, suivi)
            .map_err(echec)?;
        suivi(EtapeFeuille::Rangement);
        let valeurs: Vec<[f64; 3]> = plan.plages.iter().map(|p| p.rvb).collect();
        let texte = cgats::ecrire_avec_rvb(&feuille.mesure, &valeurs).map_err(|e| {
            eprintln!("CGATS de la feuille : {e}");
            "bibliotheque.erreur.autre".to_string()
        })?;
        let importee = cgats::lire(&texte).map_err(|e| {
            eprintln!("CGATS de la feuille : {e}");
            "bibliotheque.erreur.autre".to_string()
        })?;
        let source = self.ranger_source(
            seance,
            &plan,
            &nom_feuille(&feuille.mesure),
            &texte,
            &importee,
            OrigineSource::Acquisition,
            RoleSource::Exemplaire,
            SystemTime::now(),
        )?;
        garder_mesure(dossier, source.mesure, &feuille.mesure);
        if let Some(calage) = &feuille.calage {
            garder_calage(dossier, source.mesure, calage);
        }
        Ok(source)
    }

    /// Importe un fichier CGATS (export FD-S2w par exemple) dans la séance,
    /// tel quel. Il est rangé même incomplet ou d'association incertaine :
    /// il s'inspecte ([`Self::plages_source`]), mais ne fait pas de profil.
    /// Un fichier illisible ne range rien.
    pub fn importer_dans_seance(
        &self,
        seance: IdSeance,
        fichier: &str,
        octets: &[u8],
        role: Role,
        maintenant: SystemTime,
    ) -> Result<SourceAjoutee, RefusSeance> {
        let (_, plan) = self.seance_en_cours(seance)?;
        let texte = cgats::decoder(octets);
        let importee = cgats::lire(&texte).map_err(|e| {
            eprintln!("{fichier} : {e}");
            "bibliotheque.erreur.cgats_illisible"
        })?;
        let role = match role {
            Role::Exemplaire => RoleSource::Exemplaire,
            Role::Complement => RoleSource::Complement,
        };
        Ok(self.ranger_source(
            seance,
            &plan,
            fichier,
            &texte,
            &importee,
            OrigineSource::Import,
            role,
            maintenant,
        )?)
    }

    /// Le texte d'une mesure source, tel qu'il a été rangé.
    pub fn original_source(&self, mesure: IdMesure) -> Result<String, String> {
        self.avec(|b| b.texte_importee(mesure))
    }

    /// Chaque plage du plan, l'attendu à côté de ce que la mesure source en
    /// donne.
    pub fn plages_source(
        &self,
        seance: IdSeance,
        mesure: IdMesure,
    ) -> Result<Vec<VuePlage>, RefusSeance> {
        let rangee = self.avec(|b| b.seance(seance))?;
        if !rangee.sources.iter().any(|s| s.mesure == mesure) {
            return Err("bibliotheque.erreur.autre".into());
        }
        let plan = plan_de(&rangee)?;
        let importee = self.avec(|b| b.mesure_importee(mesure))?.mesure;
        let association = associer(&plan, &importee);
        Ok(vue_plages(&plan, &importee, &association))
    }

    /// Remplace les compléments manuels de la séance. Ils sont contrôlés à
    /// l'aperçu et à la fin, par le moyennage.
    pub fn completer_seance(
        &self,
        seance: IdSeance,
        complements: Vec<Complement>,
    ) -> Result<(), RefusSeance> {
        Ok(self.avec(|b| b.changer_complements(seance, &json(&complements)))?)
    }

    /// Retient ou écarte une mesure source, par un choix explicite de
    /// l'opérateur : son motif et l'heure sont gardés avec elle.
    pub fn retenir_source(
        &self,
        seance: IdSeance,
        mesure: IdMesure,
        retenue: bool,
        motif: &str,
        maintenant: SystemTime,
    ) -> Result<(), RefusSeance> {
        let selection = json(&serde_json::json!({
            "retenue": retenue,
            "motif": motif.trim(),
            "horodatage": instant_utc(maintenant),
        }));
        Ok(self.avec(|b| b.retenir_source(seance, mesure, retenue, Some(&selection)))?)
    }

    /// Les mesures sources retenues, relues, et les compléments.
    fn moyenne(&self, rangee: &SeanceRangee, plan: &Plan) -> Result<Moyenne, RefusSeance> {
        let complements = complements_de(rangee)?;
        let sources = rangee
            .sources
            .iter()
            .filter(|s| s.retenue)
            .map(|s| {
                Ok(Source {
                    id: s.mesure.0,
                    mire: plan.mire.empreinte.clone(),
                    role: role(s.role),
                    mesure: self.avec(|b| b.mesure_importee(s.mesure))?.mesure,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(moyenner(plan, &sources, &complements)?)
    }

    /// La séance, ses mesures sources et, en cours, ce que la moyenne
    /// donnerait ; fermée, son résultat et sa composition.
    pub fn etat_seance(&self, seance: IdSeance) -> Result<EtatSeanceVue, RefusSeance> {
        let rangee = self.avec(|b| b.seance(seance))?;
        let plan = plan_de(&rangee)?;
        let sources = rangee
            .sources
            .iter()
            .map(|s| {
                Ok(VueSource {
                    mesure: s.mesure,
                    fichier: s.fichier.clone(),
                    origine: s.origine,
                    role: s.role,
                    association: serde_json::from_str(&s.association)
                        .or_else(|e| illisible("association", e))?,
                    retenue: s.retenue,
                    selection: s.selection.clone(),
                    horodatage: s.horodatage.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let (apercu, refus) = if rangee.etat == EtatSeance::EnCours && !sources.is_empty() {
            match self.moyenne(&rangee, &plan) {
                Ok(m) => (
                    Some(ApercuMoyenne {
                        plages: m.mesure.plages.len(),
                        ecarts_signales: m.ecarts_signales().count(),
                        manquantes: m.manquantes,
                        conditions_ecartees: m.conditions_ecartees,
                        ecarts: m.ecarts,
                    }),
                    None,
                ),
                Err(refus) => (None, Some(refus)),
            }
        } else {
            (None, None)
        };
        let composition = match &rangee.composition {
            Some(texte) => {
                Some(serde_json::from_str(texte).or_else(|e| illisible("composition", e))?)
            }
            None => None,
        };
        Ok(EtatSeanceVue {
            id: rangee.id,
            condition: rangee.condition,
            tirage: rangee.tirage,
            complements: complements_de(&rangee)?,
            plan,
            etat: rangee.etat,
            horodatage: rangee.horodatage,
            fin: rangee.fin,
            sources,
            apercu,
            refus,
            resultat: rangee.resultat,
            composition,
        })
    }

    /// Ferme la séance et range la mesure moyennée dans la résolution, avec
    /// sa composition. Refusée tant que la moyenne l'est (mauvaise
    /// association, mesures incompatibles), que des plages manquent sans
    /// le choix explicite d'une fin incomplète, ou que des écarts signalés
    /// n'ont pas été vus. Une fin incomplète ne fait pas de profil.
    pub fn terminer_seance(
        &self,
        seance: IdSeance,
        fin: FinDemandee,
        maintenant: SystemTime,
    ) -> Result<IdMesure, RefusSeance> {
        let (rangee, plan) = self.seance_en_cours(seance)?;
        let mut moyenne = self.moyenne(&rangee, &plan)?;
        if !moyenne.complete() && !fin.incomplet_assume {
            return Err("seance.fin.plages_manquantes".into());
        }
        if moyenne.ecarts_signales().next().is_some() && !fin.ecarts_vus {
            return Err("seance.fin.ecarts_a_voir".into());
        }
        let horodatage = instant_utc(maintenant);
        moyenne.mesure.date = Info::Confirmee(horodatage.clone());
        let retenues: Vec<String> = rangee
            .sources
            .iter()
            .filter(|s| s.retenue)
            .map(|s| s.mesure.0.to_string())
            .collect();
        let retenues: Vec<&str> = retenues.iter().map(String::as_str).collect();
        let texte = cgats::ecrire_moyenne(&moyenne.mesure, &plan.mire.empreinte, &retenues)
            .map_err(|e| {
                eprintln!("CGATS de la moyenne : {e}");
                "bibliotheque.erreur.autre".to_string()
            })?;
        let composition = CompositionRangee {
            composition: moyenne.composition,
            manquantes: moyenne.manquantes,
            conditions_ecartees: moyenne.conditions_ecartees,
            ecarts: moyenne.ecarts,
            exclusions: rangee
                .sources
                .iter()
                .filter(|s| !s.retenue)
                .map(|s| Exclusion {
                    source: s.mesure.0,
                    selection: s.selection.clone(),
                })
                .collect(),
            complements: complements_de(&rangee)?,
            fin,
        };
        let etat = if composition.manquantes.is_empty() {
            EtatSeance::Terminee
        } else {
            EtatSeance::Incomplete
        };
        let jour = horodatage.get(..10).unwrap_or(&horodatage);
        let fichier = format!("{} séance {} moyenne {jour}.txt", plan.mire.nom, seance.0);
        let resultat = self.avec(|b| {
            b.terminer_seance(
                seance,
                &FinSeance {
                    etat,
                    horodatage: &horodatage,
                    resultat: Some((&fichier, &texte)),
                    composition: &json(&composition),
                },
            )
        })?;
        resultat.ok_or_else(|| "bibliotheque.erreur.autre".into())
    }
}
