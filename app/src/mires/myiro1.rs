//! Mire MYIRO-1 : demande contrôlée avant tout lancement, plages choisies
//! par `targen` hors du verrou de la bibliothèque, mise en page vérifiée,
//! pages dessinées et écrites en TIFF. Une mire aux mêmes réglages, déjà
//! préparée et pas encore mesurée dans la résolution, est reprise sans
//! doublon.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use bibliotheque::{IdCondition, IdTirage, MireRangee, NouvelleMire, TiragePrepare};
use mires::{
    contenu_qr, ecrire_tiff, lire_ti1, mettre_en_page, nombre_plages_myiro1, rendre_page, verifier,
    CodeTirage, ErreurMire, Etiquette, Format, Gabarit, MiseEnPage, TailleMire,
};
use profilage::MoteurProfil;

use super::{
    completer, date_de, ecrire_fichier, nom_fichier_page, DemandeMire, MirePreparee, NOMBRE_LIBRE,
};
use crate::colonne::BibliothequeOuverte;
use crate::parc::{instant_utc, DetailReglage};

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

/// Ce qu'il faut pour dessiner les pages d'un tirage MYIRO-1 et les nommer.
struct Pages<'a> {
    detail: &'a DetailReglage,
    mise: &'a MiseEnPage,
    valeurs: &'a [[f64; 3]],
    empreinte: &'a str,
    /// Taille ou « libre ».
    taille: &'a str,
    nombre: usize,
    /// Date de la première préparation, pour le nom et le texte des pages.
    date: &'a str,
}

impl Pages<'_> {
    fn nombre_pages(&self) -> usize {
        self.mise.pages.len()
    }

    /// Nom du fichier de la page `k` (à partir de 1) du tirage.
    fn nom(&self, tirage: IdTirage, k: usize) -> String {
        let d = self.detail;
        let pages = self.nombre_pages();
        nom_fichier_page(
            &d.imprimante,
            &d.papier,
            &d.resolution,
            self.date,
            tirage.0,
            k,
            pages,
        )
    }

    /// Dessine la page `k` du tirage et l'écrit dans `travail` ; rend son nom.
    fn ecrire(&self, tirage: IdTirage, k: usize, travail: &Path) -> Result<String, ErreurMire> {
        let (d, pages, date) = (self.detail, self.nombre_pages(), self.date);
        let mire = if self.taille == "libre" {
            format!("{} plages", self.nombre)
        } else {
            self.taille.to_string()
        };
        let etiquette = Etiquette {
            lignes: vec![
                format!("{} · {} · {}", d.imprimante, d.papier, d.resolution),
                format!(
                    "Mire {mire} · {} plages · MYIRO-1 · page {k} sur {pages} · {date}",
                    self.nombre
                ),
                "Imprimer à 100 %, sans gestion des couleurs".to_string(),
            ],
            code: contenu_qr(&CodeTirage {
                tirage: tirage.0,
                page: k,
                pages,
                empreinte: self.empreinte.to_string(),
            }),
        };
        let nom = self.nom(tirage, k);
        let image = rendre_page(self.mise, self.valeurs, k, &etiquette)?;
        ecrire_fichier(&travail.join(&nom), |f| ecrire_tiff(&image, f))?;
        Ok(nom)
    }
}

/// La mire rangée a les mêmes réglages que la demande : même taille, même
/// nombre de plages, même gabarit (format et côté), et sa mise en page suit
/// le gabarit actuel, même version comprise (`VERSION_GABARIT`) : une mire
/// d'une version plus ancienne (page en paysage, avant le 28 octobre 2026)
/// n'est pas reprise. Son empreinte
/// ne sert pas : la graine de la mise en page vient de l'heure.
fn memes_reglages(mire: &MireRangee, d: &DemandeMyiro1) -> bool {
    let taille = serde_json::from_str::<serde_json::Value>(&mire.generation)
        .ok()
        .and_then(|g| g.get("taille")?.as_str().map(String::from));
    let gabarit = serde_json::from_str::<MiseEnPage>(&mire.mise_en_page)
        .ok()
        .filter(MiseEnPage::suit_le_gabarit_actuel)
        .map(|m| m.gabarit);
    mire.plages == d.nombre
        && taille.as_deref() == Some(d.taille)
        && gabarit.as_ref() == Some(&d.gabarit)
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
        // Avant targen : une mire aux mêmes réglages, pas encore mesurée,
        // est reprise sans rien générer.
        let deja = self
            .avec(|b| b.tirages_prepares(condition, "myiro1"))?
            .into_iter()
            .find(|t| memes_reglages(&t.mire, &d));
        if let Some(t) = deja {
            return reprendre(t, &d, &detail, dossier);
        }
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
        let pages = Pages {
            detail: &detail,
            mise: &mise,
            valeurs: &valeurs,
            empreinte: &empreinte,
            taille: d.taille,
            nombre: d.nombre,
            date: date_de(&horodatage),
        };
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
        // Toutes les pages sont écrites avant que le tirage ne soit rangé.
        let (tirage, fichiers) = self.ranger_et_deposer(
            condition,
            &nouvelle,
            pages.nombre_pages(),
            dossier,
            |tirage, travail| {
                (1..=pages.nombre_pages())
                    .map(|k| pages.ecrire(tirage, k, travail))
                    .collect()
            },
        )?;
        Ok(MirePreparee {
            tirage: tirage.0,
            fichiers,
            dossier: dossier.to_path_buf(),
            plages: d.nombre,
            pages: pages.nombre_pages(),
            commande: plages.commande,
            deja_preparee: None,
        })
    }
}

/// Reprend un tirage pas encore mesuré aux mêmes réglages : ses pages
/// effacées à la main sont redessinées à l'identique, sous le même numéro,
/// à partir des valeurs et de la mise en page rangées.
fn reprendre(
    t: TiragePrepare,
    d: &DemandeMyiro1,
    detail: &DetailReglage,
    dossier: &Path,
) -> Result<MirePreparee, String> {
    let illisible = |e: serde_json::Error| {
        eprintln!("mire n° {} rangée illisible : {e}", t.mire.id.0);
        ErreurMire::Ecriture(e.to_string()).cle().to_string()
    };
    let valeurs: Vec<[f64; 3]> = serde_json::from_str(&t.mire.valeurs).map_err(illisible)?;
    let mise: MiseEnPage = serde_json::from_str(&t.mire.mise_en_page).map_err(illisible)?;
    let commande = serde_json::from_str::<serde_json::Value>(&t.mire.generation)
        .ok()
        .and_then(|g| g.get("commande")?.as_str().map(String::from))
        .unwrap_or_default();
    let pages = Pages {
        detail,
        mise: &mise,
        valeurs: &valeurs,
        empreinte: &t.mire.empreinte,
        taille: d.taille,
        nombre: d.nombre,
        date: date_de(&t.horodatage),
    };
    let noms: Vec<String> = (1..=pages.nombre_pages())
        .map(|k| pages.nom(t.id, k))
        .collect();
    let fichiers = completer(dossier, t.id, &noms, |manquants, travail| {
        for &i in manquants {
            pages.ecrire(t.id, i + 1, travail)?;
        }
        Ok(())
    })?;
    Ok(MirePreparee {
        tirage: t.id.0,
        fichiers,
        dossier: dossier.to_path_buf(),
        plages: d.nombre,
        pages: pages.nombre_pages(),
        commande,
        deja_preparee: Some(t.horodatage.clone()),
    })
}
