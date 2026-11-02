//! Les textes du rapport d'une image, écrits comme la page les écrit
//! (`verdict-image.js`, `comparer-image.js`, `pastilles-image.js`) : mêmes
//! clés du catalogue, mêmes règles d'arrondi, dans la langue du rapport.

use gamut::{AnalyseImage, Paire, FAMILLES_TEINTE};

use crate::textes::{texte, Langue};

/// Classes d'écart du mainteneur, en codes d'écart : début inclus, fin
/// exclue (comme `CLASSES` de `comparer-image.js`).
const CLASSES: [(&str, usize, usize); 3] =
    [("leger", 20, 50), ("moyen", 50, 100), ("fort", 100, 255)];

pub(super) fn t(langue: Langue, cle: &str) -> String {
    texte(langue, cle).to_string()
}

/// Code d'écart d'un seuil : `round(10 × seuil)`.
pub(super) fn code(seuil: f64) -> usize {
    (seuil * 10.0).round().clamp(0.0, 255.0) as usize
}

/// Un nombre à `decimales` décimales, comme `Intl.NumberFormat` : en
/// français, virgule décimale et espace fine insécable entre les milliers ;
/// en anglais, point et virgule. Une valeur non finie est « inconnue ».
pub fn nombre(valeur: f64, decimales: usize, langue: Langue) -> String {
    if !valeur.is_finite() {
        return t(langue, "comparer.inconnu");
    }
    let absolu = format!("{:.*}", decimales, valeur.abs());
    let (entier, fraction) = absolu.split_once('.').unwrap_or((&absolu, ""));
    let (milliers, virgule) = match langue {
        Langue::Francais => ('\u{202f}', ','),
        Langue::Anglais => (',', '.'),
    };
    let mut s = String::new();
    for (i, c) in entier.chars().enumerate() {
        if i > 0 && (entier.len() - i) % 3 == 0 {
            s.push(milliers);
        }
        s.push(c);
    }
    if !fraction.is_empty() {
        s.push(virgule);
        s.push_str(fraction);
    }
    // « 0,0 » et non « -0,0 » pour une valeur à peine négative.
    if valeur < 0.0 && absolu.chars().any(|c| c.is_ascii_digit() && c != '0') {
        s.insert(0, '-');
    }
    s
}

/// Part de `n` sur `total`, en % à 0,1 près : « plus de 99,9 » s'il en
/// manque au moins un, « moins de 0,1 » s'il y en a au moins un.
fn pourcent(n: u64, total: u64, langue: Langue) -> String {
    let p = n as f64 / total as f64 * 100.0;
    if n < total && p >= 99.95 {
        return t(langue, "image.verdict.plus_de").replace("{p}", &nombre(99.9, 1, langue));
    }
    if n > 0 && p < 0.05 {
        return t(langue, "image.verdict.moins_de").replace("{p}", &nombre(0.1, 1, langue));
    }
    nombre(p, 1, langue)
}

/// Pixels à l'écart connu, et ceux d'entre eux dont le code va de `de`
/// (inclus) à `a` (exclu), dans l'histogramme `h`.
fn compte(a: &AnalyseImage, h: &[u64], de: usize, fin: usize) -> (u64, u64) {
    let connus = a.pixels_comptes.saturating_sub(a.pixels_inconnus);
    let dans = h.iter().skip(de).take(fin.saturating_sub(de)).sum();
    (dans, connus)
}

/// La phrase du verdict : « 97,2 % de l'image est imprimable sur ce papier
/// avec ce profil », suivie de la part des pixels à l'écart inconnu. Même
/// règle que `verdict-image.js` : les deux doivent rester identiques.
pub fn phrase_imprimable(a: Option<&AnalyseImage>, seuil: f64, langue: Langue) -> String {
    let parts = a.map(|a| compte(a, &a.histogramme, code(seuil), 255));
    let mut phrase = match parts {
        Some((au_dela, connus)) if connus > 0 => t(langue, "image.verdict").replace(
            "{p}",
            &pourcent(connus - au_dela.min(connus), connus, langue),
        ),
        _ => t(langue, "image.verdict.inconnu"),
    };
    if let Some(a) = a.filter(|a| a.pixels_inconnus > 0) {
        phrase += &t(langue, "image.verdict.inconnus").replace(
            "{p}",
            &pourcent(a.pixels_inconnus, a.pixels_comptes, langue),
        );
    }
    phrase
}

/// Part de l'image, en %, des pixels de code `de` à `fin` ; « inconnu » sans
/// pixel connu.
fn part(a: &AnalyseImage, h: &[u64], de: usize, fin: usize, langue: Langue) -> String {
    match compte(a, h, de, fin) {
        (_, 0) => t(langue, "comparer.inconnu"),
        (n, connus) => t(langue, "image.valeur.pourcent")
            .replace("{p}", &nombre(n as f64 / connus as f64 * 100.0, 1, langue)),
    }
}

fn teintes(a: &AnalyseImage, k: usize, langue: Langue) -> String {
    let mut touchees: Vec<(&str, f64)> = FAMILLES_TEINTE
        .iter()
        .enumerate()
        .filter_map(|(s, f)| {
            let h = a.par_teinte.get(s)?;
            let (n, connus) = compte(a, h, k, 255);
            (n > 0 && connus > 0).then(|| (*f, n as f64 / connus as f64 * 100.0))
        })
        .collect();
    touchees.sort_by(|x, y| y.1.total_cmp(&x.1));
    if touchees.is_empty() {
        return t(langue, "image.teintes.aucune");
    }
    touchees
        .iter()
        .take(3)
        .map(|(f, p)| {
            t(langue, "image.teinte.valeur")
                .replace("{nom}", &t(langue, &format!("image.teinte.{f}")))
                .replace("{p}", &nombre(*p, 1, langue))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn inconnus(a: &AnalyseImage, langue: Langue) -> String {
    if a.pixels_inconnus == 0 {
        return t(langue, "image.inconnus.aucun");
    }
    t(langue, "image.inconnus.valeur")
        .replace("{n}", &nombre(a.pixels_inconnus as f64, 0, langue))
        .replace(
            "{p}",
            &nombre(
                a.pixels_inconnus as f64 / a.pixels_comptes as f64 * 100.0,
                1,
                langue,
            ),
        )
}

/// Les libellés des lignes des chiffres de l'image, au seuil `seuil`.
pub(super) fn lignes_chiffres(seuil: f64, langue: Langue) -> Vec<String> {
    let mut cles = vec!["image.chiffres.part".to_string()];
    cles.extend(
        CLASSES
            .iter()
            .map(|(nom, _, _)| format!("image.classe.{nom}")),
    );
    cles.extend(
        [
            "image.chiffres.teintes",
            "image.chiffres.inconnus",
            "image.chiffres.couleurs",
        ]
        .map(String::from),
    );
    cles.iter()
        .map(|c| t(langue, c).replace("{s}", &nombre(seuil, 1, langue)))
        .collect()
}

/// La colonne des chiffres d'un profil (dans l'ordre de [`lignes_chiffres`]),
/// ou sa raison répétée s'il est inconnu.
pub(super) fn colonne_chiffres(
    a: Result<&AnalyseImage, String>,
    seuil: f64,
    langue: Langue,
) -> Vec<String> {
    let a = match a {
        Ok(a) => a,
        Err(raison) => return vec![raison; 7],
    };
    let k = code(seuil);
    let mut c = vec![part(a, &a.histogramme, k, 255, langue)];
    c.extend(
        CLASSES
            .iter()
            .map(|(_, de, fin)| part(a, &a.histogramme, *de, *fin, langue)),
    );
    c.push(teintes(a, k, langue));
    c.push(inconnus(a, langue));
    c.push(nombre(f64::from(a.couleurs_distinctes), 0, langue));
    c
}

/// « L* 52  a* 10  b* -4 ».
pub(super) fn lab(v: [f64; 3], langue: Langue) -> String {
    t(langue, "pastilles.lab")
        .replace("{l}", &nombre(v[0], 0, langue))
        .replace("{a}", &nombre(v[1], 0, langue))
        .replace("{b}", &nombre(v[2], 0, langue))
}

pub(super) fn ecart(p: &Paire, langue: Langue) -> String {
    t(langue, "pastilles.ecart").replace("{e}", &nombre(p.ecart, 1, langue))
}

/// Les lignes d'un point relevé : Image, Imprimé, écart, seuil, et la couleur
/// visée avec la compensation du point noir.
pub(super) fn lignes_point(p: &Paire, seuil: f64, langue: Langue) -> Vec<String> {
    let cle = if usize::from(p.code) >= code(seuil) {
        "pastilles.pipette.au_dela"
    } else {
        "pastilles.pipette.en_deca"
    };
    let mut lignes = vec![
        format!(
            "{} {}",
            t(langue, "pastilles.image"),
            lab(p.demandee, langue)
        ),
        format!(
            "{} {}",
            t(langue, "pastilles.imprimee"),
            lab(p.reproduite, langue)
        ),
        ecart(p, langue),
        t(langue, cle).replace("{s}", &nombre(seuil, 1, langue)),
    ];
    if let (Some(visee), Some(e)) = (p.visee, p.ecart_visee.filter(|e| *e >= 0.1)) {
        lignes.push(
            t(langue, "pastilles.pipette.visee")
                .replace("{lab}", &lab(visee, langue))
                .replace("{e}", &nombre(e, 1, langue)),
        );
    }
    lignes
}
