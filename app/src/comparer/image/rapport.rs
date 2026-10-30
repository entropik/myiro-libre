//! Rapport client d'une image (ticket #89) : un `rapport::Document` construit
//! depuis l'image ouverte et sa dernière analyse, gardées en mémoire (#80).
//! La page n'envoie que les choix (client, objet, langue, seuil, profil de
//! la surimpression, points relevés, parties cochées) et les graphiques des
//! parties cochées. Le numéro de rapport n'est pris qu'une fois le PDF écrit.

use std::path::Path;
use std::sync::Arc;

use gamut::{AnalyseImage, ProfilIcc, ProfilSource, Rendu};
use rapport::{Bloc, Cartouche, Document, Figure, Libelles, PaireCouleurs, Section};
use serde::Deserialize;

use super::{EtatAnalyse, ImageOuverte, ImageResume, AUCUNE, SANS_ECARTS};
use crate::atelier::ReglageAtelier;
use crate::colonne::BibliothequeOuverte;
use crate::comparer::{FichiersOuverts, SourceComparaison};
use crate::pdf::{enregistrer_pdf, fabriquer_pdf};
use crate::textes::Langue;

pub mod commandes;
mod jour;
mod parties;
mod redaction;

pub use super::vignette::{vignette, vignette_surimpression};
pub use jour::Jour;
pub use parties::{GraphiquePartie, TableauPage, PARTIES};
use redaction::{code, colonne_chiffres, ecart, lignes_chiffres, lignes_point, t};
pub use redaction::{nombre, phrase_imprimable};

/// Points relevés à la pipette, au plus (décision 3 du mainteneur).
pub const POINTS_MAX: usize = 8;
/// Pastilles hors gamut reprises, au plus (comme la page).
pub const PASTILLES_MAX: usize = 12;
/// Plus grand côté d'une vignette, en pixels.
pub const COTE_VIGNETTE: u32 = 1_600;
const TROP_DE_POINTS: &str = "rapport.erreur.points";

/// Ce que la page demande.
#[derive(Clone, Debug, Deserialize)]
pub struct DemandeRapportImage {
    /// `fr` ou `en` ; le rapport est dans une seule langue.
    pub langue: String,
    pub client: String,
    pub objet: String,
    /// Numéro de l'analyse affichée : elle doit être la dernière gardée.
    pub analyse: u64,
    /// Rang du profil de la surimpression parmi les profils analysés.
    pub profil_surimpression: usize,
    pub seuil: f64,
    /// Pixels de l'image d'analyse, 8 au plus.
    pub points: Vec<(u32, u32)>,
    /// Clés des parties de l'accordéon cochées.
    pub parties: Vec<String>,
    pub graphiques: Vec<GraphiquePartie>,
}

/// Ce que l'image ouverte garde de sa dernière analyse, copié sous le verrou.
struct Gardee {
    resume: ImageResume,
    pixels: Arc<Vec<[u8; 4]>>,
    profil: Option<Arc<ProfilSource>>,
    ecarts: Vec<Option<Vec<u8>>>,
    etats: Vec<EtatAnalyse>,
    sources: Vec<SourceComparaison>,
    rendu: Rendu,
}

impl ImageOuverte {
    /// Nom du fichier de l'image ouverte.
    pub fn nom(&self) -> Result<String, String> {
        let etat = self.verrou();
        let image = etat.image.as_ref().ok_or_else(|| AUCUNE.to_string())?;
        Ok(image.resume.nom.clone())
    }

    fn gardee(&self, analyse: u64) -> Result<Gardee, String> {
        let etat = self.verrou();
        let image = etat.image.as_ref().ok_or_else(|| AUCUNE.to_string())?;
        let e = image
            .ecarts
            .as_ref()
            .filter(|e| e.analyse == analyse)
            .ok_or_else(|| SANS_ECARTS.to_string())?;
        Ok(Gardee {
            resume: image.resume.clone(),
            pixels: Arc::clone(&image.pixels),
            profil: image.profil.as_ref().map(Arc::clone),
            ecarts: e.par_profil.clone(),
            etats: e.etats.clone(),
            sources: e.sources.clone(),
            rendu: e.rendu,
        })
    }
}

fn calculee(e: &EtatAnalyse, langue: Langue) -> Result<&AnalyseImage, String> {
    match e {
        EtatAnalyse::Calculee { analyse } => Ok(analyse),
        EtatAnalyse::Inconnue { cle, detail } => {
            Err(t(langue, cle).replace("{detail}", detail.as_deref().unwrap_or("")))
        }
    }
}

/// Largeur d'une vignette : 85 mm, ou moins pour qu'elle ne dépasse pas
/// 110 mm de haut.
fn largeur_mm(r: &ImageResume) -> f32 {
    let rapport = r.largeur.max(1) as f32 / r.hauteur.max(1) as f32;
    (110.0 * rapport).min(85.0)
}

impl BibliothequeOuverte {
    /// Le document du rapport de l'image ouverte, pour sa dernière analyse.
    pub fn document_rapport_image(
        &self,
        image: &ImageOuverte,
        fichiers: &FichiersOuverts,
        d: &DemandeRapportImage,
        atelier: rapport::Atelier,
        numero: &str,
        jour: Jour,
    ) -> Result<Document, String> {
        let langue = Langue::depuis_code(&d.langue).unwrap_or(Langue::Francais);
        let tt = |cle: &str| t(langue, cle);
        if d.points.len() > POINTS_MAX {
            return Err(TROP_DE_POINTS.into());
        }
        let g = image.gardee(d.analyse)?;
        let r = &g.resume;
        if d.points
            .iter()
            .any(|&(x, y)| x >= r.largeur || y >= r.hauteur)
        {
            return Err(super::pipette::HORS_IMAGE.into());
        }
        let lus = self.lire_sources(&g.sources, fichiers)?;
        let rang = d.profil_surimpression.min(lus.len() - 1);
        let surimpression = &lus[rang];
        let seuil = if d.seuil.is_finite() {
            d.seuil.clamp(0.5, 25.0)
        } else {
            2.0
        };
        let s = nombre(seuil, 1, langue);
        let analyse = g.etats.get(rang).map(|e| calculee(e, langue));
        let a = analyse.as_ref().and_then(|a| a.as_ref().ok().copied());

        let mut sections = Vec::new();
        let legende = if g.rendu == Rendu::PointNoir {
            "image.legende.point_noir"
        } else {
            "image.legende"
        };
        sections.push(Section {
            titre: tt("rapport.section.verdict"),
            blocs: vec![Bloc::Verdict {
                texte: phrase_imprimable(a, seuil, langue),
                detail: tt(legende)
                    .replace("{s}", &s)
                    .replace("{profil}", &surimpression.nom),
            }],
        });

        let largeur = largeur_mm(r);
        let mut figures = vec![Figure {
            png: vignette(&g.pixels, r.largeur, COTE_VIGNETTE),
            legende: tt("rapport.image.photo"),
            largeur_mm: largeur,
        }];
        if let Some(Some(ecarts)) = g.ecarts.get(rang) {
            figures.push(Figure {
                png: vignette_surimpression(&g.pixels, ecarts, r.largeur, seuil, COTE_VIGNETTE),
                legende: tt("rapport.image.surimpression").replace("{s}", &s),
                largeur_mm: largeur,
            });
        }
        sections.push(Section {
            titre: tt("image.titre"),
            blocs: vec![Bloc::Images(figures)],
        });

        let mut pastilles = vec![Bloc::Paragraphe(tt("pastilles.lecture"))];
        match &analyse {
            Some(Ok(a)) => {
                let connus = a.pixels_comptes.saturating_sub(a.pixels_inconnus).max(1) as f64;
                let choisies: Vec<PaireCouleurs> = a
                    .pastilles
                    .iter()
                    .filter(|p| usize::from(p.paire.code) >= code(seuil))
                    .take(PASTILLES_MAX)
                    .map(|p| PaireCouleurs {
                        image: p.paire.couleur_demandee.clone(),
                        imprime: p.paire.couleur_reproduite.clone(),
                        titre: ecart(&p.paire, langue),
                        lignes: vec![tt("pastilles.part").replace(
                            "{p}",
                            &nombre(f64::from(p.pixels) / connus * 100.0, 1, langue),
                        )],
                    })
                    .collect();
                if choisies.is_empty() {
                    pastilles.push(Bloc::Paragraphe(
                        tt("pastilles.aucune")
                            .replace("{s}", &s)
                            .replace("{profil}", &surimpression.nom),
                    ));
                } else {
                    pastilles.push(Bloc::Paires(choisies));
                }
            }
            Some(Err(raison)) => pastilles.push(Bloc::Paragraphe(
                tt("image.surimpression.inconnue").replace("{raison}", raison),
            )),
            None => {}
        }
        pastilles.push(Bloc::Paragraphe(tt("rapport.precaution")));
        sections.push(Section {
            titre: tt("pastilles.titre"),
            blocs: pastilles,
        });

        if !d.points.is_empty() {
            sections.push(self.points(&g, d, &surimpression.icc, seuil, langue)?);
        }

        let mut entetes = vec![String::new()];
        entetes.extend(lus.iter().map(|l| l.nom.clone()));
        let colonnes: Vec<Vec<String>> = g
            .etats
            .iter()
            .map(|e| colonne_chiffres(calculee(e, langue), seuil, langue))
            .collect();
        let lignes = lignes_chiffres(seuil, langue)
            .into_iter()
            .enumerate()
            .map(|(i, libelle)| {
                let mut ligne = vec![libelle];
                ligne.extend(colonnes.iter().map(|c| c[i].clone()));
                ligne
            })
            .collect();
        let mut nombres = vec![true; entetes.len()];
        nombres[0] = false;
        sections.push(Section {
            titre: tt("image.chiffres"),
            blocs: vec![Bloc::Tableau {
                entetes,
                lignes,
                nombres,
            }],
        });
        sections.extend(parties::sections(d, langue));

        Ok(Document {
            langue: langue.code().into(),
            cartouche: Cartouche {
                atelier,
                client: d.client.clone(),
                objet: d.objet.clone(),
                lignes: vec![
                    (tt("rapport.cartouche.profil"), surimpression.nom.clone()),
                    (
                        tt("rapport.cartouche.papier"),
                        surimpression.origine.clone(),
                    ),
                    (
                        tt("rapport.cartouche.image"),
                        tt("rapport.cartouche.image.valeur")
                            .replace("{nom}", &r.nom)
                            .replace("{l}", &r.largeur_origine.to_string())
                            .replace("{h}", &r.hauteur_origine.to_string()),
                    ),
                    (
                        tt("rapport.cartouche.seuil"),
                        tt("pastilles.ecart").replace("{e}", &s),
                    ),
                    (tt("rapport.cartouche.date"), jour.texte(langue)),
                ],
                numero: numero.into(),
                titre: tt("rapport.image.titre"),
            },
            sections,
            libelles: Libelles {
                page: tt("rapport.page"),
                atelier: tt("rapport.atelier"),
                client: tt("rapport.client"),
                objet: tt("rapport.objet"),
                numero: tt("rapport.numero"),
            },
        })
    }

    /// Les points relevés, chacun avec sa couleur imprimée par le profil de
    /// la surimpression, calculée comme à la pipette.
    fn points(
        &self,
        g: &Gardee,
        d: &DemandeRapportImage,
        icc: &[u8],
        seuil: f64,
        langue: Langue,
    ) -> Result<Section, String> {
        let source = g
            .profil
            .as_ref()
            .ok_or_else(|| super::PROFIL_REFUSE.to_string())?;
        let sortie = ProfilIcc::lire(icc).map_err(|e| e.cle().to_string())?;
        let mut paires = Vec::new();
        for (i, &(x, y)) in d.points.iter().enumerate() {
            let p = g.pixels[y as usize * g.resume.largeur as usize + x as usize];
            let titre = t(langue, "rapport.point")
                .replace("{n}", &(i + 1).to_string())
                .replace("{x}", &x.to_string())
                .replace("{y}", &y.to_string());
            let paire = gamut::paire_couleur([p[0], p[1], p[2]], source, &sortie, g.rendu)
                .ok()
                .flatten()
                .filter(|_| p[3] != 0);
            paires.push(match paire {
                Some(paire) => PaireCouleurs {
                    image: paire.couleur_demandee.clone(),
                    imprime: paire.couleur_reproduite.clone(),
                    titre,
                    lignes: lignes_point(&paire, seuil, langue),
                },
                None => PaireCouleurs {
                    image: String::new(),
                    imprime: String::new(),
                    titre,
                    lignes: vec![t(langue, "image.pixel.inconnu")],
                },
            });
        }
        Ok(Section {
            titre: t(langue, "rapport.points"),
            blocs: vec![Bloc::Paires(paires)],
        })
    }

    /// Construit le rapport, l'écrit en PDF à `sortie`, puis prend le numéro
    /// de rapport : un échec ne consomme aucun numéro. Rend le numéro pris.
    pub fn exporter_rapport_image(
        &self,
        image: &ImageOuverte,
        fichiers: &FichiersOuverts,
        d: &DemandeRapportImage,
        reglage: &ReglageAtelier,
        sortie: &Path,
        jour: Jour,
    ) -> Result<String, String> {
        let cle = |e: crate::atelier::ErreurAtelier| e.cle().to_string();
        let atelier = reglage.lire().map_err(cle)?;
        let numero = reglage.prochain_numero(jour.annee).map_err(cle)?;
        let document = self.document_rapport_image(image, fichiers, d, atelier, &numero, jour)?;
        let octets = fabriquer_pdf(&document).map_err(|e| {
            eprintln!("rapport : {e:?}");
            e.cle().to_string()
        })?;
        enregistrer_pdf(&octets, sortie).map_err(|e| {
            eprintln!("rapport : {e:?}");
            e.cle().to_string()
        })?;
        reglage.prendre_numero(jour.annee).map_err(cle)
    }
}
