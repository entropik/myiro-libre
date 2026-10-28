//! Lecture du `.ti1` écrit par `targen` (ArgyllCMS). Seul le premier tableau
//! compte : il porte les plages. `targen` 3.5.0 en ajoute deux autres
//! (`DENSITY_EXTREME_VALUES`, `DEVICE_COMBINATION_VALUES`), qui ne sont pas
//! des plages de la mire.

use crate::ErreurMire;

/// Valeurs RVB du `.ti1`, de 0 à 100, dans l'ordre des `SAMPLE_ID` (ordre du
/// fichier sans cette colonne). Les autres colonnes (`XYZ_*`) sont ignorées.
pub fn lire_ti1(texte: &str, attendu: usize) -> Result<Vec<[f64; 3]>, ErreurMire> {
    let erreur = |cause: &str| ErreurMire::Ti1(cause.to_string());
    let mut lignes = texte.lines().map(str::trim);
    if lignes.next() != Some("CTI1") {
        return Err(erreur("en-tête CTI1 absent"));
    }
    let colonnes: Vec<&str> = lignes
        .by_ref()
        .skip_while(|l| *l != "BEGIN_DATA_FORMAT")
        .nth(1)
        .ok_or_else(|| erreur("colonnes absentes"))?
        .split_whitespace()
        .collect();
    let rang = |nom: &str| colonnes.iter().position(|c| *c == nom);
    let (Some(r), Some(v), Some(b)) = (rang("RGB_R"), rang("RGB_G"), rang("RGB_B")) else {
        return Err(erreur("colonnes RGB_R, RGB_G et RGB_B absentes"));
    };
    let identifiant = rang("SAMPLE_ID");

    let mut plages = Vec::new();
    let mut dans_donnees = false;
    for ligne in lignes {
        match ligne {
            "BEGIN_DATA" => dans_donnees = true,
            "END_DATA" if dans_donnees => break,
            "" => {}
            _ if dans_donnees => {
                plages.push(lire_ligne(ligne, colonnes.len(), [r, v, b], identifiant)?)
            }
            _ => {}
        }
    }
    if !dans_donnees {
        return Err(erreur("données absentes"));
    }
    if plages.len() != attendu {
        return Err(ErreurMire::NombreInattendu {
            attendu,
            trouve: plages.len(),
        });
    }
    plages.sort_by_key(|(id, _)| *id);
    Ok(plages.into_iter().map(|(_, rvb)| rvb).collect())
}

/// Une ligne de données : son rang de tri et ses valeurs RVB.
fn lire_ligne(
    ligne: &str,
    nombre: usize,
    rvb: [usize; 3],
    identifiant: Option<usize>,
) -> Result<(u64, [f64; 3]), ErreurMire> {
    let champs: Vec<&str> = ligne.split_whitespace().collect();
    if champs.len() != nombre {
        return Err(ErreurMire::Ti1(format!(
            "ligne de {} valeurs au lieu de {nombre} : {ligne}",
            champs.len()
        )));
    }
    let mut valeurs = [0.0; 3];
    for (valeur, colonne) in valeurs.iter_mut().zip(rvb) {
        *valeur = champs[colonne]
            .parse::<f64>()
            .ok()
            .filter(|v| (0.0..=100.0).contains(v))
            .ok_or_else(|| {
                ErreurMire::Ti1(format!("valeur RVB hors de 0 à 100 : {}", champs[colonne]))
            })?;
    }
    let id = match identifiant {
        Some(i) => champs[i]
            .parse::<u64>()
            .map_err(|_| ErreurMire::Ti1(format!("SAMPLE_ID illisible : {}", champs[i])))?,
        None => 0,
    };
    Ok((id, valeurs))
}
