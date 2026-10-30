//! Mire MYIRO-1 : demande contrôlée avant tout lancement, plages choisies
//! par `targen` hors du verrou de la bibliothèque, mise en page vérifiée,
//! pages dessinées et écrites en TIFF. Une mire aux mêmes réglages, déjà
//! préparée et pas encore mesurée dans la résolution, est reprise sans
//! doublon.
//!
//! Le texte imprimé sur les pages suit la langue de l'application au
//! moment de la préparation (#79) ; cette langue est rangée avec le tirage
//! et une page redessinée la garde. La langue ne fait pas partie de
//! l'identité d'une mire : préparer de nouveau la même mire dans une autre
//! langue la reprend telle quelle, dans sa langue d'origine, sans doublon.

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
use crate::textes::{texte, Langue};

/// Demande MYIRO-1 contrôlée : gabarit et nombre de plages, avant tout
/// lancement.
pub(super) struct DemandeMyiro1 {
    pub(super) gabarit: Gabarit,
    pub(super) nombre: usize,
    /// Taille ; rien pour un nombre libre de plages.
    taille: Option<TailleMire>,
}

impl DemandeMyiro1 {
    /// Taille ou « libre », rangée avec la mire et dans son nom.
    fn nom_taille(&self) -> &'static str {
        match self.taille {
            Some(TailleMire::Rapide) => "rapide",
            Some(TailleMire::Standard) => "standard",
            Some(TailleMire::Approfondi) => "approfondi",
            None => "libre",
        }
    }
}

pub(super) fn controler_myiro1(demande: &DemandeMire) -> Result<DemandeMyiro1, String> {
    let format = demande.format.unwrap_or(Format::A4);
    let gabarit = match demande.cote_mm {
        Some(cote) => Gabarit::nouveau(format, cote).map_err(|e| e.cle().to_string())?,
        None => Gabarit::par_defaut(format),
    };
    let (nombre, taille) = match (demande.nombre, demande.taille) {
        (Some(n), _) if (NOMBRE_LIBRE.0..=NOMBRE_LIBRE.1).contains(&n) => (n, None),
        (Some(_), _) => return Err("mire.erreur.nombre_libre".to_string()),
        (None, taille) => {
            let taille = taille.unwrap_or(TailleMire::Rapide);
            (nombre_plages_myiro1(taille), Some(taille))
        }
    };
    Ok(DemandeMyiro1 {
        gabarit,
        nombre,
        taille,
    })
}

/// Clés du catalogue qui composent le texte imprimé sur les pages. Leurs
/// caractères doivent tous être dans la police des pages
/// ([`mires::dessinable`]).
pub const CLES_ETIQUETTE: &[&str] = &[
    "mire.etiquette.detail",
    "mire.etiquette.taille.rapide",
    "mire.etiquette.taille.standard",
    "mire.etiquette.taille.approfondi",
    "mire.etiquette.libre",
    "mire.etiquette.consigne",
    "mire.etiquette.date",
    "mire.etiquette.mois",
];

/// Ce que dit le texte imprimé d'une page MYIRO-1, hors langue.
#[derive(Clone, Debug)]
pub struct TexteEtiquette<'a> {
    pub imprimante: &'a str,
    pub papier: &'a str,
    pub resolution: &'a str,
    /// Taille ; rien pour un nombre libre de plages.
    pub taille: Option<TailleMire>,
    pub plages: usize,
    /// Numéro de la page, à partir de 1.
    pub page: usize,
    pub pages: usize,
    /// Première préparation, en RFC 3339 : la date imprimée.
    pub horodatage: &'a str,
}

/// Date d'un horodatage RFC 3339 dans la langue, de la même forme que les
/// dates de la page (« 23 oct. 2026 », « Oct 23, 2026 ») ; telle quelle si
/// elle n'a pas cette forme.
fn date_imprimee(horodatage: &str, langue: Langue) -> String {
    let nombre = |a: usize, b: usize| horodatage.get(a..b)?.parse::<usize>().ok();
    let nom_mois = |m: usize| {
        texte(langue, "mire.etiquette.mois")
            .split(' ')
            .nth(m.checked_sub(1)?)
    };
    match (nombre(0, 4), nombre(5, 7), nombre(8, 10)) {
        (Some(annee), Some(m), Some(jour)) => match nom_mois(m) {
            Some(mois) => texte(langue, "mire.etiquette.date")
                .replace("{jour}", &jour.to_string())
                .replace("{mois}", mois)
                .replace("{annee}", &annee.to_string()),
            None => horodatage.to_string(),
        },
        _ => horodatage.to_string(),
    }
}

/// Lignes du texte imprimé sur une page MYIRO-1, dans la langue donnée :
/// imprimante, papier et résolution ; mire, plages, page et date ; rappel
/// d'impression.
pub fn lignes_etiquette(t: &TexteEtiquette, langue: Langue) -> Vec<String> {
    let n = t.plages.to_string();
    let taille = match t.taille {
        Some(TailleMire::Rapide) => texte(langue, "mire.etiquette.taille.rapide").to_string(),
        Some(TailleMire::Standard) => texte(langue, "mire.etiquette.taille.standard").to_string(),
        Some(TailleMire::Approfondi) => {
            texte(langue, "mire.etiquette.taille.approfondi").to_string()
        }
        None => texte(langue, "mire.etiquette.libre").replace("{n}", &n),
    };
    vec![
        format!("{} · {} · {}", t.imprimante, t.papier, t.resolution),
        texte(langue, "mire.etiquette.detail")
            .replace("{taille}", &taille)
            .replace("{n}", &n)
            .replace("{pages}", &t.pages.to_string())
            .replace("{page}", &t.page.to_string())
            .replace("{date}", &date_imprimee(t.horodatage, langue)),
        texte(langue, "mire.etiquette.consigne").to_string(),
    ]
}

/// Ce qu'il faut pour dessiner les pages d'un tirage MYIRO-1 et les nommer.
struct Pages<'a> {
    detail: &'a DetailReglage,
    mise: &'a MiseEnPage,
    valeurs: &'a [[f64; 3]],
    empreinte: &'a str,
    /// Taille ; rien pour un nombre libre de plages.
    taille: Option<TailleMire>,
    nombre: usize,
    /// Première préparation, en RFC 3339 : date du nom et du texte des pages.
    horodatage: &'a str,
    /// Langue du texte imprimé, rangée avec le tirage.
    langue: Langue,
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
            date_de(self.horodatage),
            tirage.0,
            k,
            pages,
        )
    }

    /// Dessine la page `k` du tirage et l'écrit dans `travail` ; rend son nom.
    fn ecrire(&self, tirage: IdTirage, k: usize, travail: &Path) -> Result<String, ErreurMire> {
        let (d, pages) = (self.detail, self.nombre_pages());
        let texte = TexteEtiquette {
            imprimante: &d.imprimante,
            papier: &d.papier,
            resolution: &d.resolution,
            taille: self.taille,
            plages: self.nombre,
            page: k,
            pages,
            horodatage: self.horodatage,
        };
        let etiquette = Etiquette {
            lignes: lignes_etiquette(&texte, self.langue),
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
/// ne sert pas : la graine de la mise en page vient de l'heure. La langue du
/// texte imprimé ne compte pas non plus : la mire est reprise dans sa langue
/// d'origine, sans doublon pour une autre langue (#79).
fn memes_reglages(mire: &MireRangee, d: &DemandeMyiro1) -> bool {
    let taille = serde_json::from_str::<serde_json::Value>(&mire.generation)
        .ok()
        .and_then(|g| g.get("taille")?.as_str().map(String::from));
    let gabarit = serde_json::from_str::<MiseEnPage>(&mire.mise_en_page)
        .ok()
        .filter(MiseEnPage::suit_le_gabarit_actuel)
        .map(|m| m.gabarit);
    mire.plages == d.nombre
        && taille.as_deref() == Some(d.nom_taille())
        && gabarit.as_ref() == Some(&d.gabarit)
}

/// Langue du texte imprimé, rangée dans le JSON `generation` du tirage ; une
/// mire rangée avant le ticket #79, sans langue, est en français.
fn langue_rangee(generation: &str) -> Langue {
    serde_json::from_str::<serde_json::Value>(generation)
        .ok()
        .and_then(|g| Langue::depuis_code(g.get("langue")?.as_str()?))
        .unwrap_or(Langue::Francais)
}

impl BibliothequeOuverte {
    /// Prépare une mire MYIRO-1 dont le texte imprimé est dans `langue`, ou
    /// reprend celle déjà préparée aux mêmes réglages, dans sa langue
    /// d'origine.
    pub(super) fn preparer_myiro1(
        &self,
        condition: IdCondition,
        demande: &DemandeMire,
        moteur: &dyn MoteurProfil,
        dossier: &Path,
        maintenant: SystemTime,
        langue: Langue,
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
            "taille": d.nom_taille(),
            "langue": langue.code(),
        })))?;
        let valeurs_json = json(serde_json::to_string(&valeurs))?;
        let mise_json = json(serde_json::to_string(&mise))?;
        let empreinte = mise.empreinte();
        let horodatage = instant_utc(maintenant);
        let nom = format!("MYIRO-1 {}", d.nom_taille());
        let pages = Pages {
            detail: &detail,
            mise: &mise,
            valeurs: &valeurs,
            empreinte: &empreinte,
            taille: d.taille,
            nombre: d.nombre,
            horodatage: &horodatage,
            langue,
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
/// à partir des valeurs, de la mise en page et de la langue rangées : le
/// texte imprimé reste celui d'origine, même si l'application a changé de
/// langue entre-temps.
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
        horodatage: &t.horodatage,
        langue: langue_rangee(&t.mire.generation),
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
