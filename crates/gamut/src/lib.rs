//! Gamut des profils ICC RVB, pour comparer des profils (ticket #71).
//!
//! La crate lit la table **A2B1** (colorimétrique relative) d'un profil de
//! sortie RVB, de type `mft2` (lut16), et la passe en **colorimétrie
//! absolue** avec le blanc du support (`wtpt`) : le blanc du papier est
//! compris dans les valeurs. Un profil d'une autre structure, sans table ou
//! sans blanc du support est refusé en le disant (`ErreurIcc`).
//!
//! Elle lit aussi, si elle existe, la table inverse **B2A1** du même type :
//! l'aller-retour A2B1(B2A1(Lab)) en colorimétrique relatif donne la couleur
//! que l'imprimante rend d'une couleur demandée (`ProfilIcc::reproduire`,
//! ticket #80). Un profil sans B2A1 lisible se lit quand même ; seul
//! l'aller-retour est alors refusé.
//!
//! De ces valeurs, elle tire la surface du cube RVB et son volume, les
//! couleurs de base, la densité approchée du noir, la table de chroma par
//! clarté et teinte (approchée) et les chemins du cube RVB (`analyser`,
//! `chemin`).
//!
//! Elle compare deux profils par ΔE00 sur la grille RVB, par zone et sur les
//! couleurs de base (`comparer`), mesure la fidélité d'un profil à ses
//! mesures et la répétabilité des plages répétées (`fidelite`,
//! `repetabilite`), reconnaît deux profils faits des mêmes mesures
//! (`memes_mesures`) et relit la limite du gamut calculée par iccgamut
//! (`lire_gam`).
//!
//! Elle ne dépend que de `colorimetrie` et de `serde` : ni Windows, ni
//! l'interface, ni les ponts. Son interface publique est un contrat
//! (ADR 0004).

mod analyse;
mod comparaison;
mod essai;
mod icc;
mod limite;
mod mesures;
mod surface;
mod table;

pub use analyse::{analyser, chemin, Analyse, Primaire};
pub use comparaison::{
    comparer, statistiques, EcartZone, Ecarts, Famille, PointEcart, Statistiques, Zone,
};
pub use essai::{profil_essai, ProfilEssai};
pub use icc::{ErreurIcc, ProfilIcc};
pub use limite::lire_gam;
pub use mesures::{fidelite, memes_mesures, points_du_chemin, repetabilite, Mesure, Repetabilite};
pub use surface::{maillage_affiche, surface_cube, volume, Maillage, MaillageAffiche};
