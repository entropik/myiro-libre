//! Ce que la vérification lit dans la bibliothèque avant de calculer, et
//! les refus qui arrivent avant tout calcul : fiche non confirmée, profil
//! CMJN, mesure du profil lui-même, mesure d'une autre résolution, mesure
//! qu'aucune mire connue ne couvre.

use bibliotheque::{EspaceProfil, IdProfil, MesureImporteeEnregistree};
use verification::{lire_xml_fds2w, Criteres};

use super::association::{associer, Association, MireConnue};
use super::{lire_criteres, DemandeVerification};
use crate::colonne::BibliothequeOuverte;
use crate::profiler::{sources_profil, ErreurProfil, SourceProfil};

/// Ce qu'il faut pour interroger le profil, lu sous le verrou.
pub(super) struct Lecture {
    pub(super) icc: Vec<u8>,
    pub(super) criteres: Criteres,
    pub(super) importee: MesureImporteeEnregistree,
    pub(super) association: Association,
}

/// La mire du profil : les RVB de sa mesure source, par identifiant.
fn mire_du_profil(source: &MesureImporteeEnregistree) -> MireConnue {
    MireConnue {
        nom: None,
        couleurs: source
            .mesure
            .plages
            .iter()
            .filter_map(|p| {
                let v = p.rvb.valeur()?;
                Some((p.identifiant.clone(), v.map(|x| f64::from(x) * 255.0)))
            })
            .collect(),
    }
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
            let mut mires: Vec<MireConnue> = Vec::new();
            for m in b.mires_verification()? {
                match lire_xml_fds2w(&m.contenu) {
                    Ok(mire) => mires.push(MireConnue {
                        nom: Some(m.nom),
                        couleurs: mire
                            .plages
                            .iter()
                            .map(|p| (p.id.to_string(), p.rvb.map(f64::from)))
                            .collect(),
                    }),
                    Err(e) => eprintln!("mire de vérification n° {} illisible : {e}", m.id.0),
                }
            }
            for source in &du_profil {
                mires.push(mire_du_profil(&b.mesure_importee(*source)?));
            }
            let jeu = b.jeu_tolerances(d.tolerances)?;
            let icc = b.fichier_profil(d.profil)?;
            Ok((condition, espace, du_profil, importee, mires, jeu, icc))
        })?;
        let (condition, espace, du_profil, importee, mires, jeu, icc) = lu;
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
        let association = associer(&importee.mesure.plages, &mires)?;
        Ok(Lecture {
            icc,
            criteres: lire_criteres(&jeu)?,
            importee,
            association,
        })
    }
}
