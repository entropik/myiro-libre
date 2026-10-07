//! Choix entre les deux ponts (ticket #13) : le MYIRO-1 d'abord, puis le FD-9.

use std::path::{Path, PathBuf};

use pont_protocole::Palier;

use super::{Etat, Instrument};
use crate::pont::{Architecture, Panne, Pont};

/// Où chercher la DLL d'un instrument, et ses ponts par architecture.
#[derive(Clone, Copy)]
pub struct Recherche<'a> {
    pub emplacements: &'a [PathBuf],
    pub ponts: &'a [(Architecture, PathBuf)],
}

/// Ouvre le MYIRO-1 ; s'il n'est pas détecté, cherche un FD-9. Si aucun des
/// deux ne l'est, le problème du MYIRO-1 reste celui qu'on montre.
pub fn ouvrir_l_un_ou_l_autre<P: Pont>(
    myiro1: Recherche,
    fd9: Recherche,
    mut lancer: impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
) -> Instrument<P> {
    let premier = Instrument::ouvrir(myiro1.emplacements, myiro1.ponts, &mut lancer);
    if premier.etat != Etat::NonDetecte {
        return premier;
    }
    let second = Instrument::ouvrir_fd9(fd9.emplacements, fd9.ponts, &mut lancer);
    if second.etat != Etat::NonDetecte {
        return second;
    }
    premier
}
