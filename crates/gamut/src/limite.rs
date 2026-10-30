//! Lecture de la limite du gamut calculée par iccgamut (fichier `.gam`).

use crate::surface::Maillage;

/// Une table du fichier : noms des champs et lignes de valeurs.
struct Table<'a> {
    champs: Vec<&'a str>,
    lignes: Vec<Vec<&'a str>>,
}

/// Lit un fichier `.gam` d'iccgamut : la première table donne les sommets
/// (`VERTEX_NO LAB_L LAB_A LAB_B`), la seconde les triangles
/// (`VERTEX_0 VERTEX_1 VERTEX_2`). Un indice hors des sommets est une
/// erreur.
pub fn lire_gam(texte: &str) -> Result<Maillage, String> {
    let tables = tables(texte)?;
    let [sommets, triangles] = tables.as_slice() else {
        return Err(format!(
            "deux tables attendues (sommets, triangles), {} trouvée(s)",
            tables.len()
        ));
    };
    attendre_champs(sommets, &["VERTEX_NO", "LAB_L", "LAB_A", "LAB_B"])?;
    attendre_champs(triangles, &["VERTEX_0", "VERTEX_1", "VERTEX_2"])?;
    let mut points = Vec::with_capacity(sommets.lignes.len());
    for (rang, ligne) in sommets.lignes.iter().enumerate() {
        let numero: usize = nombre(ligne[0])?;
        if numero != rang {
            return Err(format!("sommet {numero} à la place du sommet {rang}"));
        }
        points.push([nombre(ligne[1])?, nombre(ligne[2])?, nombre(ligne[3])?]);
    }
    let mut faces = Vec::with_capacity(triangles.lignes.len());
    for ligne in &triangles.lignes {
        let mut face = [0u32; 3];
        for (k, valeur) in ligne.iter().enumerate() {
            let indice: u32 = nombre(valeur)?;
            if indice as usize >= points.len() {
                return Err(format!(
                    "triangle vers le sommet {indice}, hors des {} sommets",
                    points.len()
                ));
            }
            face[k] = indice;
        }
        faces.push(face);
    }
    Ok(Maillage {
        sommets: points,
        triangles: faces,
    })
}

/// Découpe le texte en tables : champs entre `BEGIN_DATA_FORMAT` et
/// `END_DATA_FORMAT`, lignes entre `BEGIN_DATA` et `END_DATA`.
fn tables(texte: &str) -> Result<Vec<Table<'_>>, String> {
    let mut tables = Vec::new();
    let mut lignes = texte.lines().map(str::trim);
    while let Some(ligne) = lignes.next() {
        if ligne != "BEGIN_DATA_FORMAT" {
            continue;
        }
        let champs: Vec<&str> = lignes
            .by_ref()
            .take_while(|l| *l != "END_DATA_FORMAT")
            .flat_map(str::split_whitespace)
            .collect();
        if !lignes.by_ref().any(|l| l == "BEGIN_DATA") {
            return Err("table sans BEGIN_DATA".into());
        }
        let mut valeurs = Vec::new();
        let mut fermee = false;
        for l in lignes.by_ref() {
            if l == "END_DATA" {
                fermee = true;
                break;
            }
            if l.is_empty() {
                continue;
            }
            let ligne: Vec<&str> = l.split_whitespace().collect();
            if ligne.len() != champs.len() {
                return Err(format!(
                    "ligne de {} valeurs pour {} champs : « {l} »",
                    ligne.len(),
                    champs.len()
                ));
            }
            valeurs.push(ligne);
        }
        if !fermee {
            return Err("table sans END_DATA".into());
        }
        tables.push(Table {
            champs,
            lignes: valeurs,
        });
    }
    Ok(tables)
}

fn attendre_champs(table: &Table, attendus: &[&str]) -> Result<(), String> {
    if table.champs == attendus {
        Ok(())
    } else {
        Err(format!(
            "champs {:?} au lieu de {:?}",
            table.champs, attendus
        ))
    }
}

fn nombre<T: std::str::FromStr>(valeur: &str) -> Result<T, String> {
    valeur
        .parse()
        .map_err(|_| format!("« {valeur} » n'est pas un nombre attendu"))
}
