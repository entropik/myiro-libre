//! Mire MYIRO-1 : demande contrôlée avant tout lancement, plages choisies
//! par `targen` hors du verrou de la bibliothèque, mise en page vérifiée,
//! pages dessinées et écrites en TIFF.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use bibliotheque::{IdCondition, NouvelleMire};
use mires::{
    contenu_qr, ecrire_tiff, lire_ti1, mettre_en_page, nombre_plages_myiro1, rendre_page, verifier,
    CodeTirage, ErreurMire, Etiquette, Format, Gabarit, TailleMire,
};
use profilage::MoteurProfil;

use super::{ecrire_fichier, nom_fichier_page, DemandeMire, MirePreparee, NOMBRE_LIBRE};
use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;

/// Demande MYIRO-1 contrôlée : gabarit et nombre de plages, avant tout
/// lancement.
pub(super) struct DemandeMyiro1 {
    pub(super) gabarit: Gabarit,
    pub(super) nombre: usize,
    /// Taille ou « libre », pour le nom et le texte de la page.
    taille: &'static str,
}

pub(super) fn controler_myiro1(demande: &DemandeMire) -> Result<DemandeMyiro1, String> {
    let format = demande.format.unwrap_or(Format::A4);
    let gabarit = match demande.cote_mm {
        Some(cote) => Gabarit::nouveau(format, cote).map_err(|e| e.cle().to_string())?,
        None => Gabarit::par_defaut(format),
    };
    let (nombre, taille) = match (demande.nombre, demande.taille) {
        (Some(n), _) if (NOMBRE_LIBRE.0..=NOMBRE_LIBRE.1).contains(&n) => (n, "libre"),
        (Some(_), _) => return Err("mire.erreur.nombre_libre".to_string()),
        (None, taille) => {
            let taille = taille.unwrap_or(TailleMire::Rapide);
            (nombre_plages_myiro1(taille), nom_taille(taille))
        }
    };
    Ok(DemandeMyiro1 {
        gabarit,
        nombre,
        taille,
    })
}

fn nom_taille(taille: TailleMire) -> &'static str {
    match taille {
        TailleMire::Rapide => "rapide",
        TailleMire::Standard => "standard",
        TailleMire::Approfondi => "approfondi",
    }
}

impl BibliothequeOuverte {
    pub(super) fn preparer_myiro1(
        &self,
        condition: IdCondition,
        demande: &DemandeMire,
        moteur: &dyn MoteurProfil,
        dossier: &Path,
        maintenant: SystemTime,
    ) -> Result<MirePreparee, String> {
        let d = controler_myiro1(demande)?;
        let detail = self.detail_reglage(condition)?;
        // Hors du verrou : la page et l'arbre restent utilisables.
        let plages = moteur.generer_plages(d.nombre).map_err(|e| {
            eprintln!("{e}");
            e.cle().to_string()
        })?;
        let refus = |e: ErreurMire| {
            eprintln!("{e}");
            e.cle().to_string()
        };
        let valeurs = lire_ti1(&plages.ti1, d.nombre).map_err(refus)?;
        let graine = maintenant
            .duration_since(UNIX_EPOCH)
            .map_or(0, |t| t.as_nanos() as u64);
        let mise = mettre_en_page(&valeurs, &d.gabarit, graine).map_err(refus)?;
        let defauts = verifier(&mise, &valeurs);
        if !defauts.is_empty() {
            eprintln!("mise en page en défaut : {defauts:?}");
            return Err(ErreurMire::Contraste {
                defauts: defauts.len(),
            }
            .cle()
            .to_string());
        }
        let json = |v: serde_json::Result<String>| {
            v.map_err(|e| refus(ErreurMire::Ecriture(e.to_string())))
        };
        let generation = json(serde_json::to_string(&serde_json::json!({
            "origine": "targen",
            "commande": plages.commande,
            "version": plages.version,
            "journal": plages.journal,
            "taille": d.taille,
        })))?;
        let valeurs_json = json(serde_json::to_string(&valeurs))?;
        let mise_json = json(serde_json::to_string(&mise))?;
        let empreinte = mise.empreinte();
        let horodatage = instant_utc(maintenant);
        let nom = format!("MYIRO-1 {}", d.taille);
        let pages = mise.pages.len();
        let nouvelle = NouvelleMire {
            nom: &nom,
            instrument: "myiro1",
            plages: d.nombre,
            generation: &generation,
            valeurs: &valeurs_json,
            mise_en_page: &mise_json,
            empreinte: &empreinte,
            horodatage: &horodatage,
        };
        let date = horodatage.get(..10).unwrap_or(&horodatage);
        let mire = if d.taille == "libre" {
            format!("{} plages", d.nombre)
        } else {
            d.taille.to_string()
        };
        // Toutes les pages sont écrites avant que le tirage ne soit rangé.
        let (tirage, fichiers) =
            self.ranger_et_deposer(condition, &nouvelle, pages, dossier, |tirage, travail| {
                let mut noms = Vec::with_capacity(pages);
                for page in &mise.pages {
                    let k = page.numero;
                    let etiquette = Etiquette {
                        lignes: vec![
                            format!(
                                "{} · {} · {}",
                                detail.imprimante, detail.papier, detail.resolution
                            ),
                            format!(
                                "Mire {mire} · {} plages · MYIRO-1 · page {k} sur {pages} · {date}",
                                d.nombre
                            ),
                            "Imprimer à 100 %, sans gestion des couleurs".to_string(),
                        ],
                        code: contenu_qr(&CodeTirage {
                            tirage: tirage.0,
                            page: k,
                            pages,
                            empreinte: empreinte.clone(),
                        }),
                    };
                    let nom = nom_fichier_page(
                        &detail.imprimante,
                        &detail.papier,
                        &detail.resolution,
                        date,
                        tirage.0,
                        k,
                        pages,
                    );
                    let image = rendre_page(&mise, &valeurs, k, &etiquette)?;
                    ecrire_fichier(&travail.join(&nom), |f| ecrire_tiff(&image, f))?;
                    noms.push(nom);
                }
                Ok(noms)
            })?;
        Ok(MirePreparee {
            tirage: tirage.0,
            fichiers,
            dossier: dossier.to_path_buf(),
            plages: d.nombre,
            pages,
            commande: plages.commande,
        })
    }
}
