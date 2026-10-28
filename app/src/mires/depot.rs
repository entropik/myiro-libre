//! Dépôt des fichiers d'un tirage : écrits dans un dossier de travail, mis à
//! leur place sans jamais écraser un fichier, et le tirage rangé seulement
//! ensuite. Un tirage n'apparaît jamais sans ses fichiers.

use std::fs;
use std::path::{Path, PathBuf};

use bibliotheque::{IdCondition, IdTirage, NouvelleMire};
use mires::ErreurMire;

use crate::colonne::BibliothequeOuverte;

/// Écrit un fichier, avec la cause en cas d'échec.
pub(super) fn ecrire_fichier(
    fichier: &Path,
    ecrire: impl FnOnce(&mut fs::File) -> Result<(), ErreurMire>,
) -> Result<(), ErreurMire> {
    fs::File::create(fichier)
        .map_err(|e| ErreurMire::Ecriture(format!("{} : {e}", fichier.display())))
        .and_then(|mut f| ecrire(&mut f))
}

/// Écrit les fichiers d'un tirage dans un dossier de travail placé dans
/// `dossier` (`ecrire` y écrit et rend leurs noms), puis les met à leur
/// place ; les fichiers déposés s'ajoutent à `deposes`. Aucun fichier
/// existant n'est écrasé : si une place est prise, rien n'est déposé. En cas
/// d'échec, les fichiers déjà déposés sont retirés. Le dossier de travail est
/// toujours effacé ([`nettoyer`]).
fn deposer(
    dossier: &Path,
    tirage: IdTirage,
    deposes: &mut Vec<PathBuf>,
    ecrire: impl FnOnce(IdTirage, &Path) -> Result<Vec<String>, ErreurMire>,
) -> Result<(), ErreurMire> {
    let travail = dossier.join(format!(".preparation-T{}", tirage.0));
    let ecriture = |chemin: &Path, e: std::io::Error| {
        ErreurMire::Ecriture(format!("{} : {e}", chemin.display()))
    };
    let faire = || {
        fs::create_dir_all(&travail).map_err(|e| ecriture(&travail, e))?;
        let noms = ecrire(tirage, &travail)?;
        // Toutes les places sont libres avant le premier déplacement.
        if let Some(prise) = noms.iter().map(|n| dossier.join(n)).find(|p| p.exists()) {
            return Err(ErreurMire::FichierExistant(prise.display().to_string()));
        }
        for nom in noms {
            let place = dossier.join(&nom);
            // `rename` écraserait sans prévenir un fichier arrivé entre-temps.
            if place.exists() {
                return Err(ErreurMire::FichierExistant(place.display().to_string()));
            }
            fs::rename(travail.join(&nom), &place).map_err(|e| ecriture(&place, e))?;
            deposes.push(place);
        }
        Ok(())
    };
    let resultat = nettoyer(&travail, faire(), |d| {
        if d.exists() {
            fs::remove_dir_all(d)
        } else {
            Ok(())
        }
    });
    if resultat.is_err() {
        retirer(deposes);
    }
    resultat
}

/// Efface le dossier de travail par `effacer`. Un échec n'est jamais tu :
/// après une réussite, il est écrit dans le journal ; après un échec, il
/// s'ajoute à l'erreur rendue.
fn nettoyer(
    travail: &Path,
    resultat: Result<(), ErreurMire>,
    effacer: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), ErreurMire> {
    let Err(e) = effacer(travail) else {
        return resultat;
    };
    let reste = format!(
        "dossier de travail {} non effacé, à effacer à la main : {e}",
        travail.display()
    );
    match resultat {
        Ok(()) => {
            eprintln!("{reste}");
            Ok(())
        }
        Err(erreur) => Err(ErreurMire::Ecriture(format!("{erreur} ; {reste}"))),
    }
}

fn retirer(fichiers: &mut Vec<PathBuf>) {
    for f in fichiers.drain(..) {
        let _ = fs::remove_file(f);
    }
}

/// Reprend les fichiers d'un tirage déjà rangé, nommés `noms` dans
/// `dossier` : ceux qui manquent (effacés à la main) sont réécrits sous le
/// même numéro de tirage, par [`deposer`] ; `ecrire` reçoit leurs rangs dans
/// `noms` et le dossier de travail où les écrire. Aucun fichier présent
/// n'est touché. Rend les chemins de tous les fichiers du tirage.
pub(super) fn completer(
    dossier: &Path,
    tirage: IdTirage,
    noms: &[String],
    ecrire: impl FnOnce(&[usize], &Path) -> Result<(), ErreurMire>,
) -> Result<Vec<PathBuf>, String> {
    let manquants: Vec<usize> = (0..noms.len())
        .filter(|&i| !dossier.join(&noms[i]).exists())
        .collect();
    if !manquants.is_empty() {
        let mut deposes = Vec::new();
        deposer(dossier, tirage, &mut deposes, |_, travail| {
            ecrire(&manquants, travail)?;
            Ok(manquants.iter().map(|&i| noms[i].clone()).collect())
        })
        .map_err(|e| {
            eprintln!("{e}");
            e.cle().to_string()
        })?;
    }
    Ok(noms.iter().map(|n| dossier.join(n)).collect())
}

impl BibliothequeOuverte {
    /// Range la mire et son tirage, et dépose ses fichiers dans `dossier`
    /// ([`deposer`]) : la transaction de la bibliothèque n'est validée
    /// qu'après le dépôt de tous les fichiers. Un tirage n'apparaît donc
    /// jamais sans ses fichiers : en cas d'échec, rien n'est rangé, aucun
    /// fichier ne reste, et la clé de l'erreur est rendue. Le verrou de la
    /// bibliothèque est tenu pendant l'écriture des pages.
    pub(super) fn ranger_et_deposer(
        &self,
        condition: IdCondition,
        mire: &NouvelleMire,
        pages: usize,
        dossier: &Path,
        ecrire: impl FnOnce(IdTirage, &Path) -> Result<Vec<String>, ErreurMire>,
    ) -> Result<(IdTirage, Vec<PathBuf>), String> {
        let mut deposes = Vec::new();
        let rangee = self.avec(|b| {
            b.enregistrer_mire_et_tirage_si(condition, mire, pages, |tirage| {
                deposer(dossier, tirage, &mut deposes, ecrire)
            })
        });
        match rangee {
            Ok(Ok((_, tirage))) => Ok((tirage, deposes)),
            Ok(Err(e)) => {
                eprintln!("{e}");
                Err(e.cle().to_string())
            }
            // Validation refusée par la base après le dépôt : les fichiers partent.
            Err(cle) => {
                retirer(&mut deposes);
                Err(cle)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refus(_: &Path) -> std::io::Result<()> {
        Err(std::io::Error::other("fichier ouvert ailleurs"))
    }

    /// Un dossier de travail qui ne s'efface pas est dit dans l'erreur rendue.
    #[test]
    fn un_dossier_de_travail_non_efface_est_dit_dans_l_erreur() {
        let erreur = nettoyer(
            Path::new("Mires/.preparation-T3"),
            Err(ErreurMire::Ecriture("disque plein".into())),
            refus,
        )
        .unwrap_err();
        let texte = erreur.to_string();
        assert!(texte.contains("disque plein"), "{texte}");
        assert!(
            texte.contains(".preparation-T3") && texte.contains("non effacé"),
            "{texte}"
        );
    }

    /// Après une réussite, il ne fait pas échouer la préparation : il est
    /// écrit dans le journal.
    #[test]
    fn apres_une_reussite_le_dossier_non_efface_ne_fait_pas_echouer() {
        assert_eq!(nettoyer(Path::new("x"), Ok(()), refus), Ok(()));
        assert_eq!(nettoyer(Path::new("x"), Ok(()), |_| Ok(())), Ok(()));
    }
}
