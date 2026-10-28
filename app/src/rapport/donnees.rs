//! Données du rapport, relues dans la bibliothèque, et export du rapport
//! seul ou à côté du profil.

use std::path::Path;

use bibliotheque::{IdVerification, ResumeProfil, VerificationRangee};
use verification::Criteres;

use super::rapport_html;
use crate::colonne::BibliothequeOuverte;
use crate::profiler::{provenance_depuis_parametres, ErreurProfil, ProvenanceProfil};
use crate::textes::Langue;
use crate::verifier::{lire_criteres, lire_json, CalculVerification, FicheTirage};

/// Tout ce que le rapport montre, relu dans la bibliothèque.
#[derive(Clone, Debug)]
pub struct DonneesRapport {
    pub imprimante: String,
    pub papier: String,
    pub resolution: String,
    pub profil: ResumeProfil,
    /// Recette et version d'ArgyllCMS d'un profil calculé par myiro-libre.
    pub provenance: Option<ProvenanceProfil>,
    pub verification: VerificationRangee,
    pub criteres: Criteres,
    pub calcul: CalculVerification,
    pub fiche: FicheTirage,
    /// Date de la vérification dont celle-ci est un recalcul.
    pub recalcul_de: Option<String>,
}

impl BibliothequeOuverte {
    /// Les données du rapport d'une vérification, relues dans la
    /// bibliothèque.
    pub fn donnees_rapport(
        &self,
        verification: IdVerification,
    ) -> Result<DonneesRapport, ErreurProfil> {
        let (rangee, profil, parametres, recalcul_de, condition) = self.avec(|b| {
            let rangee = b.verification(verification)?;
            let condition = b.condition_profil(rangee.profil)?;
            let profil = b
                .historique_profils(condition)?
                .into_iter()
                .find(|p| p.id == rangee.profil)
                .ok_or(bibliotheque::ErreurBibliotheque::ProfilInconnu(
                    rangee.profil,
                ))?;
            let parametres = b.parametres_profil(rangee.profil)?;
            let origine = match rangee.recalcul_de {
                Some(o) => Some(b.verification(o)?.horodatage),
                None => None,
            };
            Ok((rangee, profil, parametres, origine, condition))
        })?;
        let detail = self.detail_reglage(condition)?;
        Ok(DonneesRapport {
            imprimante: detail.imprimante,
            papier: detail.papier,
            resolution: detail.resolution,
            provenance: provenance_depuis_parametres(&parametres),
            criteres: lire_criteres(&rangee.tolerances)?,
            calcul: lire_json("calcul", &rangee.calcul)?,
            fiche: lire_json("fiche", &rangee.fiche)?,
            profil,
            recalcul_de,
            verification: rangee,
        })
    }

    /// Écrit le rapport d'une vérification, régénéré depuis les données.
    pub fn exporter_rapport(
        &self,
        verification: IdVerification,
        langue: Langue,
        fichier: &Path,
    ) -> Result<(), String> {
        let d = self
            .donnees_rapport(verification)
            .map_err(|e| e.cle.to_string())?;
        ecrire(fichier, rapport_html(&d, langue).as_bytes())
    }

    /// Écrit le profil vérifié puis, à côté, son rapport sous le même nom en
    /// `.html`.
    pub fn exporter_profil_et_rapport(
        &self,
        verification: IdVerification,
        langue: Langue,
        fichier_icc: &Path,
    ) -> Result<(), String> {
        let d = self
            .donnees_rapport(verification)
            .map_err(|e| e.cle.to_string())?;
        let icc = self.avec(|b| b.fichier_profil(d.profil.id))?;
        ecrire(fichier_icc, &icc)?;
        ecrire(
            &fichier_icc.with_extension("html"),
            rapport_html(&d, langue).as_bytes(),
        )
    }
}

fn ecrire(fichier: &Path, octets: &[u8]) -> Result<(), String> {
    std::fs::write(fichier, octets).map_err(|e| {
        eprintln!("{} : {e}", fichier.display());
        "bibliotheque.erreur.export".to_string()
    })
}
