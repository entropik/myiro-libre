//! Pastilles des couleurs hors gamut d'une image (ticket #80) : pour chaque
//! cellule du nuage (4 unités Lab de la couleur demandée), la couleur RVB la
//! plus fréquente, la couleur demandée et la couleur reproduite, et leur
//! écart. Profils d'essai synthétiques.

use gamut::{
    analyser_pixels, paire_couleur, profil_essai, AnalyseImage, ErreurIcc, ProfilEssai, ProfilIcc,
    ProfilSource, PASTILLES, SEUIL_PASTILLES,
};

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Inverse de `lineaire`, chroma d'abord ramené à 40 au plus : un gamut
/// étroit, où les couleurs sRVB saturées sont hors gamut.
fn inverse_chroma_40(lab: [f64; 3]) -> [f64; 3] {
    let c = lab[1].hypot(lab[2]);
    let k = if c > 40.0 { 40.0 / c } else { 1.0 };
    let [l, a, b] = [lab[0], lab[1] * k, lab[2] * k];
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

fn sortie(inverse: Option<fn([f64; 3]) -> [f64; 3]>) -> ProfilIcc {
    let p = ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible: lineaire,
        inverse,
        description: "Profil d'essai".into(),
        mesures: None,
    };
    ProfilIcc::lire(&profil_essai(&p)).expect("profil d'essai lisible")
}

fn analyser(pixels: &[[u8; 4]]) -> AnalyseImage {
    analyser_pixels(
        pixels,
        &ProfilSource::srvb(),
        &sortie(Some(inverse_chroma_40)),
        &|| false,
    )
    .expect("non annulée")
    .expect("table inverse lue")
}

/// Cellule du nuage d'une couleur RVB : 4 unités Lab de sa couleur demandée
/// (doc de `PointNuage`).
fn cellule(rvb: [u8; 3]) -> [i32; 3] {
    ProfilSource::srvb()
        .demande(rvb.map(|v| f64::from(v) / 255.0))
        .map(|v| (v / 4.0).floor() as i32)
}

fn hexa(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit())
}

#[test]
fn une_pastille_montre_la_couleur_la_plus_frequente_de_sa_cellule() {
    let rouge = [255, 0, 0];
    let voisin = [
        [254, 0, 0],
        [253, 0, 0],
        [255, 1, 0],
        [255, 0, 1],
        [252, 0, 0],
    ]
    .into_iter()
    .find(|&v| cellule(v) == cellule(rouge))
    .expect("un rouge voisin dans la même cellule");
    let mut pixels = vec![[rouge[0], rouge[1], rouge[2], 255]; 10];
    pixels.extend(vec![[voisin[0], voisin[1], voisin[2], 255]; 30]);
    let analyse = analyser(&pixels);
    assert_eq!(analyse.pastilles.len(), 1, "{:?}", analyse.pastilles);
    let p = &analyse.pastilles[0];
    assert_eq!(p.rvb, voisin, "le plus fréquent représente la cellule");
    assert_eq!(p.pixels, 40, "la pastille compte toute la cellule");
    // L'écart est celui des pixels du représentant, par la même règle.
    assert_eq!(p.paire.code, analyse.ecarts[10]);
    assert_eq!(p.paire.code, (p.paire.ecart * 10.0).floor() as u8);
    assert!(p.paire.ecart > 10.0, "écart {}", p.paire.ecart);
    assert!(hexa(&p.paire.couleur_demandee) && hexa(&p.paire.couleur_reproduite));
    assert_ne!(p.paire.couleur_demandee, p.paire.couleur_reproduite);
    // Couleur demandée sur le blanc du papier, comme le nuage.
    let point = &analyse.nuage[0];
    for k in 0..3 {
        assert!(
            (point.lab[k] - p.paire.demandee[k]).abs() < 1.0,
            "{point:?} {p:?}"
        );
    }
}

#[test]
fn seules_les_couleurs_au_dela_du_plus_petit_seuil_ont_leur_pastille() {
    // Des gris, dans le gamut, et un rouge saturé, hors gamut.
    let mut pixels: Vec<[u8; 4]> = (100..=160).map(|v| [v, v, v, 255]).collect();
    pixels.extend(vec![[255, 0, 0, 255]; 5]);
    let analyse = analyser(&pixels);
    assert_eq!(analyse.pastilles.len(), 1, "{:?}", analyse.pastilles);
    assert_eq!(analyse.pastilles[0].rvb, [255, 0, 0]);
    let minimum = (SEUIL_PASTILLES * 10.0).round() as u8;
    assert!(analyse.pastilles.iter().all(|p| p.paire.code >= minimum));

    let gris: Vec<[u8; 4]> = (100..=160).map(|v| [v, v, v, 255]).collect();
    assert!(analyser(&gris).pastilles.is_empty());
}

#[test]
fn les_pastilles_sont_plafonnees_et_rangees_par_pixels() {
    // Toutes les couleurs d'un pas de 8, puis un bleu très présent : sa
    // cellule vient en tête, et le bleu la représente.
    let mut pixels: Vec<[u8; 4]> = Vec::new();
    for r in (0..=255u16).step_by(8) {
        for g in (0..=255u16).step_by(8) {
            for b in (0..=255u16).step_by(8) {
                pixels.push([r as u8, g as u8, b as u8, 255]);
            }
        }
    }
    pixels.extend(std::iter::repeat_n([0, 0, 255, 255], 500));
    let analyse = analyser(&pixels);
    assert_eq!(analyse.pastilles.len(), PASTILLES);
    assert_eq!(PASTILLES, 48);
    assert_eq!(analyse.pastilles[0].rvb, [0, 0, 255]);
    assert!(analyse.pastilles[0].pixels >= 500);
    assert!(analyse
        .pastilles
        .windows(2)
        .all(|p| p[0].pixels >= p[1].pixels));
    let cellules: std::collections::HashSet<_> =
        analyse.pastilles.iter().map(|p| cellule(p.rvb)).collect();
    assert_eq!(cellules.len(), PASTILLES, "une pastille par cellule");
}

#[test]
fn la_paire_d_un_pixel_est_celle_de_sa_pastille() {
    let analyse = analyser(&[[255, 0, 0, 255]; 3]);
    let paire = paire_couleur(
        [255, 0, 0],
        &ProfilSource::srvb(),
        &sortie(Some(inverse_chroma_40)),
    )
    .expect("table inverse lue")
    .expect("écart calculable");
    assert_eq!(paire, analyse.pastilles[0].paire);
}

#[test]
fn sans_b2a1_la_paire_d_un_pixel_est_refusee_avec_sa_raison() {
    let resultat = paire_couleur([10, 20, 30], &ProfilSource::srvb(), &sortie(None));
    assert!(matches!(resultat, Err(ErreurIcc::SansTableInverse)));
}
