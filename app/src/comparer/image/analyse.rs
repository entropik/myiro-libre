//! L'analyse de l'image ouverte face aux profils affichés : chacune reçoit
//! son jeton d'annulation et son numéro, et ne range ses écarts que si elle
//! est encore la dernière sur la même image.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gamut::{ErreurIcc, ProfilIcc, ProfilSource};

use super::{
    detail_refus, AnalyseProfil, Analyses, EcartsRanges, EtatAnalyse, ImageOuverte, ANNULEE,
    AUCUNE, CALCUL_INTERROMPU, PROFIL_REFUSE,
};
use crate::colonne::BibliothequeOuverte;
use crate::comparer::{FichiersOuverts, SourceComparaison};

/// Ce qu'une analyse emporte, copié sous le verrou.
struct Depart {
    image: u64,
    analyse: u64,
    pixels: Arc<Vec<[u8; 4]>>,
    profil: Arc<ProfilSource>,
    jeton: Arc<AtomicBool>,
}

impl ImageOuverte {
    /// Arrête l'analyse précédente et donne son numéro et son jeton à la
    /// nouvelle.
    fn demarrer(&self) -> Result<Depart, String> {
        let mut etat = self.verrou();
        let image = etat.image.as_ref().ok_or_else(|| AUCUNE.to_string())?;
        let profil = image
            .profil
            .as_ref()
            .map(Arc::clone)
            .ok_or_else(|| PROFIL_REFUSE.to_string())?;
        let (numero, pixels) = (image.numero, Arc::clone(&image.pixels));
        etat.arreter();
        etat.analyses += 1;
        let jeton = Arc::new(AtomicBool::new(false));
        etat.en_cours = Some(Arc::clone(&jeton));
        Ok(Depart {
            image: numero,
            analyse: etat.analyses,
            pixels,
            profil,
            jeton,
        })
    }

    /// Range les écarts d'une analyse terminée, si elle est encore la
    /// dernière sur la même image ; sinon (ou sans écarts : analyse
    /// arrêtée), elle est caduque.
    fn terminer(&self, depart: &Depart, ecarts: Option<Vec<Option<Vec<u8>>>>) -> bool {
        let mut etat = self.verrou();
        if etat
            .en_cours
            .as_ref()
            .is_some_and(|j| Arc::ptr_eq(j, &depart.jeton))
        {
            etat.en_cours = None;
        }
        let derniere = etat.analyses == depart.analyse;
        match (ecarts, etat.image.as_mut()) {
            (Some(par_profil), Some(image)) if derniere && image.numero == depart.image => {
                image.ecarts = Some(EcartsRanges {
                    analyse: depart.analyse,
                    par_profil,
                });
                true
            }
            _ => false,
        }
    }
}

/// Résultat d'une analyse face à un profil : `Err(())` si elle a été annulée.
type Resultat = Result<(EtatAnalyse, Option<Vec<u8>>), ()>;

fn analyser_un(
    icc: &[u8],
    pixels: &[[u8; 4]],
    source: &ProfilSource,
    jeton: &AtomicBool,
) -> Resultat {
    let inconnue = |e: &ErreurIcc| EtatAnalyse::Inconnue {
        cle: e.cle(),
        detail: detail_refus(e),
    };
    let sortie = match ProfilIcc::lire(icc) {
        Ok(p) => p,
        Err(e) => return Ok((inconnue(&e), None)),
    };
    match gamut::analyser_pixels(pixels, source, &sortie, &|| jeton.load(Ordering::Relaxed)) {
        Err(gamut::Annulee) => Err(()),
        Ok(Err(e)) => Ok((inconnue(&e), None)),
        Ok(Ok(mut analyse)) => {
            let ecarts = std::mem::take(&mut analyse.ecarts);
            Ok((EtatAnalyse::Calculee { analyse }, Some(ecarts)))
        }
    }
}

impl BibliothequeOuverte {
    /// Écarts de l'image ouverte à chaque profil de `sources`, dans leur
    /// ordre. Pixels et octets des profils sont copiés sous les verrous,
    /// puis chaque profil est calculé à part, en parallèle. Un profil
    /// illisible ou sans table inverse est « inconnu » sans empêcher les
    /// autres. Une analyse arrêtée, ou devenue caduque (autre analyse, autre
    /// image, image fermée), rend `image.annulee` et ne range rien.
    pub fn analyser_image(
        &self,
        image: &ImageOuverte,
        sources: &[SourceComparaison],
        fichiers: &FichiersOuverts,
    ) -> Result<Analyses, String> {
        let depart = image.demarrer()?;
        let lus = match self.lire_sources(sources, fichiers) {
            Ok(lus) => lus,
            Err(e) => {
                image.terminer(&depart, None);
                return Err(e);
            }
        };
        // Hors de tout verrou à partir d'ici.
        let resultats: Vec<Resultat> = std::thread::scope(|s| {
            let fils: Vec<_> = lus
                .iter()
                .map(|l| {
                    s.spawn(|| analyser_un(&l.icc, &depart.pixels, &depart.profil, &depart.jeton))
                })
                .collect();
            fils.into_iter()
                .map(|f| {
                    f.join().unwrap_or(Ok((
                        EtatAnalyse::Inconnue {
                            cle: CALCUL_INTERROMPU,
                            detail: None,
                        },
                        None,
                    )))
                })
                .collect()
        });
        let Ok(resultats) = resultats.into_iter().collect::<Result<Vec<_>, ()>>() else {
            image.terminer(&depart, None);
            return Err(ANNULEE.to_string());
        };
        let (etats, ecarts): (Vec<_>, Vec<_>) = resultats.into_iter().unzip();
        if !image.terminer(&depart, Some(ecarts)) {
            return Err(ANNULEE.to_string());
        }
        Ok(Analyses {
            numero: depart.analyse,
            profils: lus
                .iter()
                .zip(etats)
                .map(|(l, etat)| AnalyseProfil {
                    source: l.source,
                    etat,
                })
                .collect(),
        })
    }
}
