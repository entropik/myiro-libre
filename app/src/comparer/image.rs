//! Une photo de client ouverte pour la comparaison (ticket #80) : une seule
//! à la fois, en mémoire le temps de la séance, jamais rangée, copiée ni
//! écrite sur le disque. Elle est analysée face à chaque profil affiché,
//! hors de tout verrou. Une seule analyse vit à la fois : en lancer une
//! autre, ouvrir une autre image, fermer ou annuler arrête la précédente.

use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use gamut::{AnalyseImage, ErreurIcc, ProfilSource};
use serde::Serialize;

use super::{detail_refus, SourceComparaison};

mod analyse;
pub mod commandes;
mod pipette;

pub use pipette::{pixel_affiche, EtatPixel, PixelImage, PointAffiche};

/// Taille d'un fichier image lu au plus : au-delà, rien n'est lu.
pub const OCTETS_MAX_IMAGE: u64 = 1 << 30;

const AUCUNE: &str = "image.refus.aucune";
const PROFIL_REFUSE: &str = "image.refus.profil";
/// Profil présent dans le fichier, que le décodeur n'a pas pu en tirer
/// (`photo::Photo::profil_illisible`) : refusé comme un profil illisible.
const PROFIL_ILLISIBLE: &str = "comparer.image.refus.illisible";
const TROP_GRANDE: &str = "image.refus.trop_grande";
const ILLISIBLE: &str = "image.refus.illisible";
const SANS_ECARTS: &str = "image.refus.sans_ecarts";
const ANNULEE: &str = "image.annulee";
/// Le calcul d'un profil s'est arrêté sur une erreur de myiro-libre.
const CALCUL_INTERROMPU: &str = "image.erreur.calcul";

/// Clés des refus de l'image propres à l'application (ceux du décodage
/// viennent de `photo::RefusPhoto::cle`).
pub const CLES_IMAGE: [&str; 11] = [
    AUCUNE,
    PROFIL_REFUSE,
    PROFIL_ILLISIBLE,
    TROP_GRANDE,
    ILLISIBLE,
    SANS_ECARTS,
    ANNULEE,
    CALCUL_INTERROMPU,
    pipette::HORS_IMAGE,
    pipette::TRANSPARENT,
    pipette::PIXEL_INCONNU,
];

/// L'image ouverte pour la séance (une à la fois), en mémoire seulement, et
/// l'analyse qui la calcule.
#[derive(Default)]
pub struct ImageOuverte(Mutex<Etat>);

#[derive(Default)]
struct Etat {
    image: Option<ImageEnMemoire>,
    /// Dernier numéro d'image donné.
    images: u64,
    /// Dernier numéro d'analyse donné.
    analyses: u64,
    /// Jeton de l'analyse en cours : le poser l'arrête.
    en_cours: Option<Arc<AtomicBool>>,
}

impl Etat {
    /// Arrête l'analyse en cours, s'il y en a une.
    fn arreter(&mut self) {
        if let Some(jeton) = self.en_cours.take() {
            jeton.store(true, Ordering::Relaxed);
        }
    }
}

struct ImageEnMemoire {
    /// Change à chaque ouverture : l'analyse d'une image remplacée entre-temps
    /// ne se range pas avec la nouvelle.
    numero: u64,
    resume: ImageResume,
    pixels: Arc<Vec<[u8; 4]>>,
    /// Profil de l'image ; `None` quand celui qu'elle embarque est refusé
    /// (la raison est dans `resume.profil`).
    profil: Option<Arc<ProfilSource>>,
    /// Codes d'écart de la dernière analyse terminée.
    ecarts: Option<EcartsRanges>,
}

struct EcartsRanges {
    /// Numéro de l'analyse ([`Analyses::numero`]).
    analyse: u64,
    /// Un octet par pixel, profil par profil (`None` pour un profil inconnu).
    par_profil: Vec<Option<Vec<u8>>>,
}

/// Une conversion du profil de l'image écartée parce qu'illisible.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TableEcartee {
    pub table: String,
    pub cle: &'static str,
    pub detail: Option<String>,
}

/// Profil de l'image : celui qu'elle embarque, sRVB sans profil (ou par choix
/// de l'opérateur, alors « supposé »), ou refusé avec sa raison.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatProfilImage {
    /// `nom` est vide quand le profil n'en donne pas (texte
    /// `image.profil.sans_nom`).
    Embarque {
        nom: String,
        ecartees: Vec<TableEcartee>,
    },
    Srvb {
        suppose: bool,
    },
    Refuse {
        cle: &'static str,
        detail: Option<String>,
    },
}

/// Ce que la page reçoit de l'image ouverte.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ImageResume {
    pub nom: String,
    /// Image d'analyse, orientation appliquée.
    pub largeur: u32,
    pub hauteur: u32,
    /// Fichier, avant orientation.
    pub largeur_origine: u32,
    pub hauteur_origine: u32,
    /// Un pixel sur `pas` dans chaque sens (texte `image.approche`).
    pub pas: u32,
    pub seize_bits: bool,
    pub profil: EtatProfilImage,
}

/// Écarts de l'image à un profil, ou pourquoi ils sont inconnus.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum EtatAnalyse {
    /// Les codes d'écart par pixel ne sont pas dans `analyse` : ils se
    /// demandent à part, en binaire ([`ImageOuverte::ecarts`]).
    Calculee { analyse: AnalyseImage },
    Inconnue {
        cle: &'static str,
        detail: Option<String>,
    },
}

/// L'analyse de l'image face à un profil affiché.
#[derive(Clone, Debug, Serialize)]
pub struct AnalyseProfil {
    pub source: SourceComparaison,
    pub etat: EtatAnalyse,
}

/// Une analyse menée à terme : ses codes d'écart se demandent par `numero`.
#[derive(Clone, Debug, Serialize)]
pub struct Analyses {
    pub numero: u64,
    /// Dans l'ordre des profils demandés.
    pub profils: Vec<AnalyseProfil>,
}

fn etat_profil(icc: Option<&[u8]>, profil: &Result<ProfilSource, ErreurIcc>) -> EtatProfilImage {
    match (icc, profil) {
        (_, Err(e)) => EtatProfilImage::Refuse {
            cle: ProfilSource::cle_refus(e),
            detail: detail_refus(e),
        },
        (None, Ok(_)) => EtatProfilImage::Srvb { suppose: false },
        (Some(_), Ok(p)) => EtatProfilImage::Embarque {
            nom: p.nom(),
            ecartees: p
                .ecartees()
                .iter()
                .map(|(table, e)| TableEcartee {
                    table: table.clone(),
                    cle: ProfilSource::cle_refus(e),
                    detail: detail_refus(e),
                })
                .collect(),
        },
    }
}

impl ImageOuverte {
    fn verrou(&self) -> MutexGuard<'_, Etat> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lit le fichier ([`OCTETS_MAX_IMAGE`] au plus) et l'ouvre à la place
    /// de l'image précédente. Rien n'est écrit.
    pub fn ouvrir(&self, chemin: &Path) -> Result<ImageResume, String> {
        let fichier = std::fs::File::open(chemin).map_err(|_| ILLISIBLE.to_string())?;
        let taille = fichier.metadata().map(|m| m.len()).unwrap_or(0);
        if taille > OCTETS_MAX_IMAGE {
            return Err(TROP_GRANDE.to_string());
        }
        let mut octets = Vec::new();
        fichier
            .take(OCTETS_MAX_IMAGE + 1)
            .read_to_end(&mut octets)
            .map_err(|_| ILLISIBLE.to_string())?;
        if octets.len() as u64 > OCTETS_MAX_IMAGE {
            return Err(TROP_GRANDE.to_string());
        }
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.ouvrir_octets(&nom, &octets)
    }

    /// Ouvre une image depuis ses octets, à la place de la précédente. Le
    /// décodage se fait hors du verrou.
    pub fn ouvrir_octets(&self, nom: &str, octets: &[u8]) -> Result<ImageResume, String> {
        let photo = photo::ouvrir(octets).map_err(|refus| {
            eprintln!("{refus:?}");
            refus.cle().to_string()
        })?;
        let (profil, etat) = match &photo.profil_illisible {
            // Jamais pris pour une absence de profil, donc jamais pour sRVB.
            Some(raison) => (
                None,
                EtatProfilImage::Refuse {
                    cle: PROFIL_ILLISIBLE,
                    detail: Some(raison.clone()),
                },
            ),
            None => {
                let icc = photo.icc.as_deref();
                let lu = icc.map_or_else(|| Ok(ProfilSource::srvb()), ProfilSource::lire);
                let etat = etat_profil(icc, &lu);
                (lu.ok().map(Arc::new), etat)
            }
        };
        let resume = ImageResume {
            nom: nom.to_string(),
            largeur: photo.largeur,
            hauteur: photo.hauteur,
            largeur_origine: photo.largeur_origine,
            hauteur_origine: photo.hauteur_origine,
            pas: photo.pas,
            seize_bits: photo.seize_bits,
            profil: etat,
        };
        let mut etat = self.verrou();
        etat.arreter();
        etat.images += 1;
        etat.image = Some(ImageEnMemoire {
            numero: etat.images,
            resume: resume.clone(),
            pixels: Arc::new(photo.rvba),
            profil,
            ecarts: None,
        });
        Ok(resume)
    }

    /// Le profil embarqué refusé est remplacé par sRVB, à la demande de
    /// l'opérateur : la page l'affiche alors « supposé ». Un profil lisible
    /// n'est jamais remplacé.
    pub fn traiter_comme_srvb(&self) -> Result<ImageResume, String> {
        let mut etat = self.verrou();
        let image = etat.image.as_mut().ok_or_else(|| AUCUNE.to_string())?;
        if image.profil.is_none() {
            image.profil = Some(Arc::new(ProfilSource::srvb()));
            image.resume.profil = EtatProfilImage::Srvb { suppose: true };
            image.ecarts = None;
        }
        Ok(image.resume.clone())
    }

    /// Oublie l'image et ses écarts, et arrête l'analyse en cours.
    pub fn fermer(&self) {
        let mut etat = self.verrou();
        etat.arreter();
        etat.image = None;
    }

    /// Arrête l'analyse en cours : elle rend `image.annulee`. Sans analyse
    /// en cours, rien ne change.
    pub fn annuler_analyse(&self) {
        self.verrou().arreter();
    }

    /// Pixels RVBA de l'image d'analyse, ligne par ligne :
    /// `largeur` × `hauteur` × 4 octets.
    pub fn apercu(&self) -> Result<Vec<u8>, String> {
        let pixels = self
            .verrou()
            .image
            .as_ref()
            .map(|i| Arc::clone(&i.pixels))
            .ok_or_else(|| AUCUNE.to_string())?;
        Ok(pixels.iter().flatten().copied().collect())
    }

    /// Codes d'écart (un octet par pixel de l'image d'analyse, ligne par
    /// ligne) du profil `index` de l'analyse `analyse`, si c'est la dernière
    /// terminée sur cette image.
    pub fn ecarts(&self, analyse: u64, index: usize) -> Result<Vec<u8>, String> {
        let etat = self.verrou();
        let image = etat.image.as_ref().ok_or_else(|| AUCUNE.to_string())?;
        image
            .ecarts
            .as_ref()
            .filter(|e| e.analyse == analyse)
            .and_then(|e| e.par_profil.get(index))
            .and_then(Clone::clone)
            .ok_or_else(|| SANS_ECARTS.to_string())
    }
}
