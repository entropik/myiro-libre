//! Mire de vérification du FD-9 (plan 7, tâche 10) : créée une fois, rangée
//! dans la bibliothèque, et écrite en XML pour FD-S2w, qui l'ouvre et en
//! tire le TIFF à imprimer.

use std::path::Path;
use std::time::SystemTime;

use bibliotheque::MireVerificationRangee;
use verification::{ecrire_xml_fds2w, mire_verification_fd9};

use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;

impl BibliothequeOuverte {
    /// La mire de vérification FD-9 : rangée à la première demande, puis
    /// rendue telle quelle (même contenu, même empreinte).
    pub fn mire_verification_fd9(
        &self,
        maintenant: SystemTime,
    ) -> Result<MireVerificationRangee, String> {
        let mire = mire_verification_fd9();
        let xml = ecrire_xml_fds2w(&mire);
        let horodatage = instant_utc(maintenant);
        self.avec(|b| {
            let id =
                b.ranger_mire_verification(&mire.nom, "fd9", mire.version, &xml, &horodatage)?;
            b.mires_verification()?
                .into_iter()
                .find(|m| m.id == id)
                .ok_or_else(|| bibliotheque::ErreurBibliotheque::Base("mire introuvable".into()))
        })
    }

    /// Écrit la mire de vérification FD-9 pour FD-S2w ; rend le nom du
    /// fichier écrit.
    pub fn exporter_mire_fd9(
        &self,
        fichier: &Path,
        maintenant: SystemTime,
    ) -> Result<String, String> {
        let mire = self.mire_verification_fd9(maintenant)?;
        std::fs::write(fichier, mire.contenu).map_err(|e| {
            eprintln!("{} : {e}", fichier.display());
            "bibliotheque.erreur.export".to_string()
        })?;
        Ok(fichier
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default())
    }
}
