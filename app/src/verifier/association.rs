//! Quelle mire a été mesurée (plan 7, tâche 10) : la mesure de la seconde
//! mire est rapprochée, par les identifiants de ses plages, d'une mire
//! connue — une mire de vérification rangée, ou la mire qui a servi à
//! calculer le profil. Les couleurs demandées sont celles de la mire, et les
//! RVB de l'export doivent leur correspondre. Une mesure qu'aucune mire ne
//! couvre, ou dont des RVB diffèrent, est refusée : l'association serait
//! incertaine (#1), et les écarts calculés n'auraient pas de sens.

use std::collections::HashMap;

use cgats::PlageImportee;

use crate::profiler::ErreurProfil;

/// Écart toléré entre un RVB de l'export et celui de la mire, sur 255 : un
/// demi-pas de 8 bits.
const ECART_RVB: f64 = 0.5;

/// Plages en cause nommées dans un refus ; les suivantes sont comptées.
const NOMMEES: usize = 10;

/// Une mire connue : son nom (rien pour la mire du profil) et ses RVB sur
/// 255, par identifiant de plage.
pub(super) struct MireConnue {
    pub nom: Option<String>,
    pub couleurs: HashMap<String, [f64; 3]>,
}

/// La mire retrouvée et les couleurs demandées (de 0 à 1) par identifiant.
pub(super) struct Association {
    /// Nom de la mire de vérification ; rien pour la mire du profil.
    pub mire: Option<String>,
    pub demandes: HashMap<String, [f64; 3]>,
}

/// Rapproche les plages mesurées d'une mire connue : mêmes identifiants,
/// même nombre de plages, mêmes RVB (à un demi-pas près).
pub(super) fn associer(
    plages: &[PlageImportee],
    mires: &[MireConnue],
) -> Result<Association, ErreurProfil> {
    let mut rvb = Vec::with_capacity(plages.len());
    for p in plages {
        let Some(v) = p.rvb.valeur() else {
            return Err(ErreurProfil::sans_cause("profil.refus.rvb_inconnus"));
        };
        rvb.push((p.identifiant.as_str(), v.map(|x| f64::from(x) * 255.0)));
    }
    let mut desaccord: Option<Vec<&str>> = None;
    for mire in mires {
        if mire.couleurs.len() != rvb.len()
            || rvb.iter().any(|(id, _)| !mire.couleurs.contains_key(*id))
        {
            continue;
        }
        let differentes: Vec<&str> = rvb
            .iter()
            .filter(|(id, v)| {
                let attendu = mire.couleurs[*id];
                (0..3).any(|i| (v[i] - attendu[i]).abs() > ECART_RVB)
            })
            .map(|(id, _)| *id)
            .collect();
        if differentes.is_empty() {
            return Ok(Association {
                mire: mire.nom.clone(),
                demandes: mire
                    .couleurs
                    .iter()
                    .map(|(id, c)| (id.clone(), c.map(|x| x / 255.0)))
                    .collect(),
            });
        }
        desaccord.get_or_insert(differentes);
    }
    Err(match desaccord {
        Some(ids) => {
            eprintln!("RVB différents de la mire pour les plages {ids:?}");
            let mut e = ErreurProfil::sans_cause("verif.refus.mire_desaccord");
            let mut cause = ids[..ids.len().min(NOMMEES)].join(", ");
            if ids.len() > NOMMEES {
                cause.push_str(&format!(" (+{})", ids.len() - NOMMEES));
            }
            e.cause = Some(cause);
            e
        }
        None => ErreurProfil::sans_cause("verif.refus.mire_inconnue"),
    })
}
