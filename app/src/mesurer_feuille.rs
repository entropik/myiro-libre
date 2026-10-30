//! Mesurer la mire RGB-2520 au FD-9 et la ranger dans une résolution
//! (ticket #34). La mesure entre comme une mesure importée de FD-S2w : un
//! CGATS de myiro-libre avec les RVB de la mire et les spectres, que le
//! calcul de profil prend sans changement. La mesure complète (spectres,
//! données brutes, provenance), au format conservable `myiro-libre/mesure/1`,
//! est gardée à côté, dans `<bibliothèque>/mesures-fd9/<n°>.json`.
//!
//! Méthodes de [`BibliothequeOuverte`], sans Tauri ; l'instrument et son
//! pont sont ceux du module `instrument`. Les erreurs rendues sont des clés
//! du catalogue de textes.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use bibliotheque::{IdCondition, IdMesure};
use mires::{lire_mire_fd9, ErreurMire, MireFd9};
use pont_protocole::{ecrire_mesure, CalageFeuille, Mesure, Palier};
use serde::Serialize;

use crate::colonne::BibliothequeOuverte;
use crate::instrument::choix::Fermeture;
use crate::instrument::feuille::{definition_rgb2520, ErreurFeuille, EtapeFeuille, FeuilleMesuree};
use crate::instrument::Instrument;
use crate::pont::{Panne, Pont};

/// Dossier de la bibliothèque où la mesure complète est gardée.
pub const DOSSIER_MESURES_FD9: &str = "mesures-fd9";

/// Lit et vérifie la mire RGB-2520 trouvée (`chercher_mire_fd9`) ; sans
/// fichier, ou avec un autre, le refus est une clé `mire.erreur.*`.
pub fn lire_rgb2520(fichier: Option<&Path>) -> Result<MireFd9, String> {
    let fichier = fichier.ok_or_else(|| ErreurMire::MireFd9Introuvable.cle().to_string())?;
    let octets = fs::read(fichier).map_err(|e| {
        eprintln!("{} : {e}", fichier.display());
        ErreurMire::MireFd9Illisible(e.to_string())
            .cle()
            .to_string()
    })?;
    lire_mire_fd9(octets).map_err(|e| {
        eprintln!("{} : {e}", fichier.display());
        e.cle().to_string()
    })
}

/// Ce que la page reçoit d'une feuille mesurée et rangée.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FeuilleRangee {
    /// La mesure importée dans la résolution.
    pub mesure: IdMesure,
    /// Nom sous lequel elle est rangée.
    pub fichier: String,
    pub plages: usize,
    /// Mesure complète gardée (`myiro-libre/mesure/1`) ; rien si elle n'a
    /// pas pu s'écrire
    /// (la mesure reste rangée, la console dit pourquoi).
    pub json: Option<PathBuf>,
    /// Fermeture du pont de la feuille confirmée ; sinon, l'instrument est
    /// peut-être resté dans un état incertain (la mesure vaut).
    pub fermeture_confirmee: bool,
    /// Points visés et image de la feuille gardés à côté
    /// (`mesures-fd9/<n°>.calage.json`) ; rien si le pont n'en a pas rendu
    /// ou s'ils n'ont pas pu s'écrire.
    pub calage: Option<PathBuf>,
}

/// Pourquoi une feuille n'est pas rangée : clé du catalogue (`mire.erreur.*`,
/// `parc.erreur.*`, `bibliotheque.erreur.*`, `probleme.<code>` ou
/// `feuille_fd9.<type>`) et détail technique, montré replié.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EchecFeuille {
    pub cle: String,
    pub detail: Option<String>,
}

impl From<String> for EchecFeuille {
    fn from(cle: String) -> Self {
        EchecFeuille { cle, detail: None }
    }
}

/// Nom de rangement : « FD-9 RGB-2520 2026-10-30 1515.txt », d'après l'heure
/// de la mesure posée par le pont.
fn nom_feuille(mesure: &Mesure) -> String {
    let h = mesure.provenance().horodatage.texte();
    let jour = h.get(..10).unwrap_or("mesure");
    let heure: String = h.get(11..16).unwrap_or_default().replace(':', "");
    format!("FD-9 RGB-2520 {jour} {heure}.txt")
}

impl BibliothequeOuverte {
    /// Mesure la RGB-2520 au FD-9 détecté et la range dans la résolution
    /// `condition`. Sans mire, ou dans une résolution inconnue, rien n'est
    /// lancé. Sinon le pont est relancé au plafond Feuille pour cette
    /// mesure seulement, puis revient au plafond ordinaire (voir
    /// [`Instrument::mesurer_feuille`]). `dossier` : celui de la
    /// bibliothèque, où va la mesure complète.
    pub fn mesurer_feuille_fd9<P: Pont>(
        &self,
        condition: IdCondition,
        instrument: &mut Instrument<P>,
        mire: Result<MireFd9, String>,
        dossier: &Path,
        lancer: &mut impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        suivi: &mut impl FnMut(EtapeFeuille),
    ) -> Result<FeuilleRangee, EchecFeuille> {
        let mire = mire?;
        self.detail_reglage(condition)?;
        let echec = |e: ErreurFeuille| EchecFeuille {
            cle: e.cle(),
            detail: e.detail(),
        };
        let definition = definition_rgb2520(&mire).map_err(|e| {
            eprintln!("définition de la RGB-2520 : {e}");
            echec(ErreurFeuille::Definition(e))
        })?;
        let feuille = instrument
            .mesurer_feuille(definition, lancer, suivi)
            .map_err(echec)?;
        suivi(EtapeFeuille::Rangement);
        Ok(self.ranger_feuille_fd9(condition, &feuille, &mire, dossier)?)
    }

    /// Écrit le CGATS (RVB de la mire, spectres, provenance du pont),
    /// l'importe dans la résolution, puis garde la réponse du pont à côté.
    fn ranger_feuille_fd9(
        &self,
        condition: IdCondition,
        feuille: &FeuilleMesuree,
        mire: &MireFd9,
        dossier: &Path,
    ) -> Result<FeuilleRangee, String> {
        let texte = cgats::ecrire_avec_rvb(&feuille.mesure, &mire.valeurs).map_err(|e| {
            eprintln!("CGATS de la feuille : {e}");
            "bibliotheque.erreur.autre".to_string()
        })?;
        let fichier = nom_feuille(&feuille.mesure);
        let mesure = self.avec(|b| b.importer_mesure(condition, &fichier, &texte))?;
        let json = garder_mesure(dossier, mesure, &feuille.mesure);
        let calage = feuille
            .calage
            .as_ref()
            .and_then(|calage| garder_calage(dossier, mesure, calage));
        Ok(FeuilleRangee {
            mesure,
            fichier,
            plages: feuille.mesure.plages().len(),
            json,
            fermeture_confirmee: feuille.fermeture == Fermeture::Confirmee,
            calage,
        })
    }

    /// Calage de la dernière feuille FD-9 rangée dans la résolution
    /// `condition` (le plus grand n° de mesure qui en a un), relu depuis
    /// `<dossier>/mesures-fd9/`.
    pub fn dernier_calage_fd9(
        &self,
        condition: IdCondition,
        dossier: &Path,
    ) -> Option<(IdMesure, CalageFeuille)> {
        let detail = self.detail_reglage(condition).ok()?;
        let mut ids: Vec<IdMesure> = detail.importees.iter().map(|m| m.id).collect();
        ids.sort_by_key(|id| std::cmp::Reverse(id.0));
        ids.into_iter()
            .find_map(|id| lire_calage(dossier, id).map(|calage| (id, calage)))
    }
}

fn fichier_calage(dossier: &Path, id: IdMesure) -> PathBuf {
    dossier
        .join(DOSSIER_MESURES_FD9)
        .join(format!("{}.calage.json", id.0))
}

/// Écrit le calage d'une feuille (points visés, image réduite en base64)
/// dans `<dossier>/mesures-fd9/<n°>.calage.json`, sans remplacer un fichier
/// déjà là. L'image peut montrer l'écriture de l'opérateur : ce fichier
/// reste dans la bibliothèque du poste, jamais dans le dépôt.
fn garder_calage(dossier: &Path, id: IdMesure, calage: &CalageFeuille) -> Option<PathBuf> {
    let fichier = fichier_calage(dossier, id);
    let texte = serde_json::to_string(calage).expect("un calage se sérialise toujours");
    let ecrit = fs::create_dir_all(dossier.join(DOSSIER_MESURES_FD9)).and_then(|()| {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&fichier)?
            .write_all(texte.as_bytes())
    });
    match ecrit {
        Ok(()) => Some(fichier),
        Err(e) => {
            eprintln!("{} : {e}", fichier.display());
            None
        }
    }
}

/// Calage gardé avec la mesure `id`, s'il y en a un et qu'il se relit
/// (contrôlé comme une réponse du pont : image de la bonne taille).
pub fn lire_calage(dossier: &Path, id: IdMesure) -> Option<CalageFeuille> {
    let texte = fs::read_to_string(fichier_calage(dossier, id)).ok()?;
    serde_json::from_str(&texte)
        .map_err(|e| eprintln!("calage de la mesure {} : {e}", id.0))
        .ok()
}

/// Écrit la mesure au format conservable (`myiro-libre/mesure/1`,
/// `ecrire_mesure`) dans `<dossier>/mesures-fd9/<n°>.json`, sans jamais
/// remplacer un fichier déjà là (une bibliothèque restaurée peut réemployer
/// un numéro).
fn garder_mesure(dossier: &Path, id: IdMesure, mesure: &Mesure) -> Option<PathBuf> {
    let fichier = dossier
        .join(DOSSIER_MESURES_FD9)
        .join(format!("{}.json", id.0));
    let ecrit = fs::create_dir_all(dossier.join(DOSSIER_MESURES_FD9)).and_then(|()| {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&fichier)?
            .write_all(ecrire_mesure(mesure).as_bytes())
    });
    match ecrit {
        Ok(()) => Some(fichier),
        Err(e) => {
            eprintln!("{} : {e}", fichier.display());
            None
        }
    }
}
