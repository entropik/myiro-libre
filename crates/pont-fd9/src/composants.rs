//! Composants que la DLL du fabricant charge d'elle-même pendant une feuille,
//! cherchés à côté du pont et non à côté d'elle (ticket #87).
//!
//! À la lecture du QR, `FD9SDK.dll` passe par `FD9BarcodeManager.dll`
//! (assembly .NET mixte), qui charge l'assembly `zxing` 0.14.0.0 (ZXing.Net,
//! Apache-2.0). Le CLR la cherche dans le dossier de l'exécutable qui a
//! chargé la DLL : celui du pont. Sans elle, une exception .NET non gérée
//! arrête le pont, feuille engagée (essai E3, 30 octobre 2026).

use std::path::{Path, PathBuf};

/// Assembly de lecture du QR, à poser à côté de `pont-fd9`.
pub const COMPOSANT_QR: &str = "zxing.dll";

/// Chemin du composant de lecture du QR dans `dossier` s'il y est (un
/// fichier), sinon le chemin où il manque. Ne charge rien.
pub fn composant_qr(dossier: &Path) -> Result<PathBuf, PathBuf> {
    let chemin = dossier.join(COMPOSANT_QR);
    if chemin.is_file() {
        Ok(chemin)
    } else {
        Err(chemin)
    }
}
