//! Mire de vérification du FD-9 (plan 7, tâche 10) : créée une fois, rangée
//! dans la bibliothèque, puis toujours relue telle qu'elle a été rangée, et
//! écrite en XML pour FD-S2w, qui l'ouvre et en tire le TIFF à imprimer.

use std::path::Path;
use std::time::SystemTime;

use bibliotheque::MireVerificationRangee;
use verification::{ecrire_xml_fds2w, mire_verification_fd9, MIRE_FD9_VERSION};

use crate::colonne::BibliothequeOuverte;
use crate::parc::instant_utc;

impl BibliothequeOuverte {
    /// La mire de vérification FD-9 de la version courante : rangée à la
    /// première demande, puis relue. Si le générateur ne redonne pas la mire
    /// rangée pour la même version, il a changé sans changer de version :
    /// refus (`verif.erreur.mire_changee`), rien n'est remplacé.
    pub fn mire_verification_fd9(
        &self,
        maintenant: SystemTime,
    ) -> Result<MireVerificationRangee, String> {
        let mire = mire_verification_fd9();
        let xml = ecrire_xml_fds2w(&mire);
        let horodatage = instant_utc(maintenant);
        let rangee = self.avec(|b| match b.mire_verification("fd9", MIRE_FD9_VERSION)? {
            Some(m) => Ok(m),
            None => b.ranger_mire_verification(&mire.nom, "fd9", mire.version, &xml, &horodatage),
        })?;
        if rangee.contenu != xml {
            eprintln!(
                "mire de vérification FD-9 v{MIRE_FD9_VERSION} : le générateur ne redonne pas la mire rangée ({})",
                rangee.empreinte
            );
            return Err("verif.erreur.mire_changee".to_string());
        }
        Ok(rangee)
    }

    /// Écrit la mire de vérification FD-9 pour FD-S2w, telle qu'elle est
    /// rangée ; rend le nom du fichier écrit.
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
