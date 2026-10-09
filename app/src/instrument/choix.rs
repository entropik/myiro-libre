//! Choix entre les deux ponts (ticket #13) : le MYIRO-1 d'abord, puis le FD-9.

use std::path::{Path, PathBuf};

use pont_protocole::Palier;

use super::parefeu::{AutorisationPareFeu, PareFeu};
use super::{Etat, Instrument, Probleme};
use crate::pont::{Architecture, Panne, Pont};

/// Où chercher la DLL d'un instrument, et ses ponts par architecture.
#[derive(Clone, Copy)]
pub struct Recherche<'a> {
    pub emplacements: &'a [PathBuf],
    pub ponts: &'a [(Architecture, PathBuf)],
}

/// Ouvre le MYIRO-1 ; s'il n'est pas détecté, cherche un FD-9. Si aucun des
/// deux ne l'est, on montre le problème du MYIRO-1, sauf quand son logiciel
/// est absent alors que celui du FD-9 a été trouvé : le vrai problème est
/// alors celui du FD-9 (pont manquant, DLL refusée, détection en erreur…).
/// `pare_feu` sert au FD-9 seul (ticket #47).
pub fn ouvrir_l_un_ou_l_autre<P: Pont, F: PareFeu>(
    myiro1: Recherche,
    fd9: Recherche,
    mut lancer: impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
    pare_feu: &mut AutorisationPareFeu<F>,
) -> Instrument<P> {
    let premier = Instrument::ouvrir(myiro1.emplacements, myiro1.ponts, &mut lancer);
    if premier.etat != Etat::NonDetecte {
        return premier;
    }
    let second = Instrument::ouvrir_fd9(fd9.emplacements, fd9.ponts, &mut lancer, pare_feu);
    let logiciel_absent =
        |i: &Instrument<P>| matches!(i.probleme(), Some(Probleme::LogicielAbsent { .. }));
    if second.etat != Etat::NonDetecte || (logiciel_absent(&premier) && !logiciel_absent(&second)) {
        return second;
    }
    premier
}
