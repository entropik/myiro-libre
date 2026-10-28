//! Ce que la vérification lit dans la bibliothèque avant de calculer, et
//! les refus qui arrivent avant tout calcul : fiche non confirmée, profil
//! CMJN, mesure du profil lui-même, mesure d'une autre résolution.

use bibliotheque::{EspaceProfil, IdProfil};
use cgats::MesureImportee;
use verification::Criteres;

use super::{lire_criteres, DemandeVerification};
use crate::colonne::BibliothequeOuverte;
use crate::profiler::{sources_profil, ErreurProfil, SourceProfil};

/// Les RVB des plages, arrondis au millième, triés : deux mires aux mêmes
/// couleurs dans un autre ordre restent les mêmes couleurs.
fn couleurs(mesure: &MesureImportee) -> Vec<[i64; 3]> {
    let mut c: Vec<[i64; 3]> = mesure
        .plages
        .iter()
        .filter_map(|p| {
            p.rvb
                .valeur()
                .map(|v| v.map(|x| (f64::from(x) * 1000.0).round() as i64))
        })
        .collect();
    c.sort_unstable();
    c
}

/// Ce qu'il faut pour interroger le profil, lu sous le verrou.
pub(super) struct Lecture {
    pub(super) icc: Vec<u8>,
    pub(super) criteres: Criteres,
    pub(super) importee: bibliotheque::MesureImporteeEnregistree,
    pub(super) memes_couleurs: bool,
}

impl BibliothequeOuverte {
    /// Mesures importées de la résolution du profil, sources possibles d'une
    /// vérification ; la mesure qui a servi à calculer le profil y reste,
    /// avec sa raison.
    pub fn sources_verification(
        &self,
        profil: IdProfil,
    ) -> Result<Vec<SourceProfil>, ErreurProfil> {
        Ok(self.avec(|b| {
            let condition = b.condition_profil(profil)?;
            let du_profil = b.mesures_profil(profil)?;
            let importees = b
                .arborescence("")?
                .into_iter()
                .find(|br| br.condition.id == condition)
                .map(|br| br.importees)
                .unwrap_or_default();
            let mut sources = sources_profil(b, &importees)?;
            for s in &mut sources {
                if du_profil.contains(&s.mesure) {
                    s.raison = Some("verif.refus.mesure_du_profil");
                }
            }
            Ok(sources)
        })?)
    }

    /// Refus avant tout calcul, puis lectures sous le verrou.
    pub(super) fn lire_pour_verifier(
        &self,
        d: &DemandeVerification,
    ) -> Result<Lecture, ErreurProfil> {
        if !d.fiche.confirmee {
            return Err(ErreurProfil::sans_cause("verif.refus.fiche"));
        }
        let lu = self.avec(|b| {
            let condition = b.condition_profil(d.profil)?;
            let espace = b
                .historique_profils(condition)?
                .into_iter()
                .find(|p| p.id == d.profil)
                .map(|p| p.espace);
            let du_profil = b.mesures_profil(d.profil)?;
            let importee = b.mesure_importee(d.mesure)?;
            let sources = du_profil
                .iter()
                .map(|m| b.mesure_importee(*m).map(|i| couleurs(&i.mesure)))
                .collect::<Result<Vec<_>, _>>()?;
            let jeu = b.jeu_tolerances(d.tolerances)?;
            let icc = b.fichier_profil(d.profil)?;
            Ok((condition, espace, du_profil, importee, sources, jeu, icc))
        })?;
        let (condition, espace, du_profil, importee, sources, jeu, icc) = lu;
        if espace == Some(EspaceProfil::Cmjn) {
            return Err(ErreurProfil::sans_cause("verif.refus.cmjn"));
        }
        if du_profil.contains(&d.mesure) {
            return Err(ErreurProfil::sans_cause("verif.refus.mesure_du_profil"));
        }
        if importee.condition != condition {
            eprintln!("mesure n° {} rangée dans une autre résolution", d.mesure.0);
            return Err(ErreurProfil::sans_cause("profil.erreur.mesure_ailleurs"));
        }
        let ici = couleurs(&importee.mesure);
        Ok(Lecture {
            icc,
            criteres: lire_criteres(&jeu)?,
            memes_couleurs: sources.contains(&ici),
            importee,
        })
    }
}
