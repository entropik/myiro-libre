//! Compensation du point noir (ticket #80) : le noir de l'image tombe sur le
//! noir du profil de sortie, le blanc reste le blanc. Profils d'essai
//! synthétiques : leur noir (RVB 0, 0, 0) est à L* 25.

use gamut::{
    analyser_pixels, paire_couleur, profil_essai, PointNoir, ProfilEssai, ProfilIcc, ProfilSource,
    Rendu, L_NOIR_MAX,
};

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b) : noir à L* 25.
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Inverse exact de `lineaire`, borné au cube RVB.
fn lineaire_inverse(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

/// Comme `lineaire_inverse`, chroma d'abord ramené à 40 au plus.
fn inverse_chroma_40(lab: [f64; 3]) -> [f64; 3] {
    let c = lab[1].hypot(lab[2]);
    let k = if c > 40.0 { 40.0 / c } else { 1.0 };
    lineaire_inverse([lab[0], lab[1] * k, lab[2] * k])
}

/// Un papier sans vrai noir : RVB 0, 0, 0 à L* 70.
fn delave(rvb: [f64; 3]) -> [f64; 3] {
    [70.0 + 30.0 * (rvb[0] + rvb[1] + rvb[2]) / 3.0, 0.0, 0.0]
}

fn profil(cible: fn([f64; 3]) -> [f64; 3], inverse: fn([f64; 3]) -> [f64; 3]) -> ProfilIcc {
    ProfilIcc::lire(&profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible,
        inverse: Some(inverse),
        description: "Profil d'essai".into(),
        mesures: None,
    }))
    .expect("profil d'essai lisible")
}

fn proche(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tolerance)
}

#[test]
fn les_noirs_sont_lus_neutres_dans_les_deux_profils() {
    let p = PointNoir::nouveau(&ProfilSource::srvb(), &profil(lineaire, lineaire_inverse));
    assert!(proche(p.noir_image, [0.0, 0.0, 0.0], 0.01), "{p:?}");
    assert!(proche(p.noir_sortie, [25.0, 0.0, 0.0], 0.05), "{p:?}");
    // Sans vrai noir, la clarté du noir de sortie est bornée.
    let p = PointNoir::nouveau(&ProfilSource::srvb(), &profil(delave, lineaire_inverse));
    assert!(proche(p.noir_sortie, [L_NOIR_MAX, 0.0, 0.0], 1e-9), "{p:?}");
}

#[test]
fn un_noir_pur_tombe_sur_le_noir_du_papier_sans_ecart() {
    let sortie = profil(lineaire, lineaire_inverse);
    let source = ProfilSource::srvb();
    let p = PointNoir::nouveau(&source, &sortie);
    assert!(proche(p.viser([0.0; 3]), [25.0, 0.0, 0.0], 0.05));
    let avec = paire_couleur([0, 0, 0], &source, &sortie, Rendu::PointNoir)
        .expect("table inverse lue")
        .expect("écart calculable");
    assert!(avec.ecart < 0.5, "écart avec compensation : {}", avec.ecart);
    // La pastille « Image » reste le noir de l'image ; la visée est le noir
    // du papier (sur la base du blanc du papier, comme le nuage).
    let visee = avec.visee.expect("couleur visée");
    assert!(visee[0] > avec.demandee[0] + 15.0, "{avec:?}");
    assert!(avec.ecart_visee.expect("écart de la visée") > 10.0);
    assert!(avec.couleur_visee.is_some());
    let sans = paire_couleur([0, 0, 0], &source, &sortie, Rendu::Relatif)
        .expect("table inverse lue")
        .expect("écart calculable");
    assert!(
        sans.ecart > 10.0,
        "écart sans compensation : {}",
        sans.ecart
    );
}

#[test]
fn le_blanc_reste_le_blanc() {
    let sortie = profil(lineaire, lineaire_inverse);
    let source = ProfilSource::srvb();
    let p = PointNoir::nouveau(&source, &sortie);
    assert!(proche(p.viser([100.0, 0.0, 0.0]), [100.0, 0.0, 0.0], 1e-6));
    let blanc = paire_couleur([255, 255, 255], &source, &sortie, Rendu::PointNoir)
        .expect("table inverse lue")
        .expect("écart calculable");
    assert!(
        blanc.ecart_visee.expect("écart de la visée") < 0.01,
        "{blanc:?}"
    );
    // Un gris moyen s'éclaircit, sans prendre de couleur.
    let gris = p.viser([50.0, 0.0, 0.0]);
    assert!(gris[0] > 50.0 && gris[0] < 100.0, "{gris:?}");
    assert!(gris[1].abs() < 1e-6 && gris[2].abs() < 1e-6, "{gris:?}");
}

#[test]
fn une_couleur_saturee_hors_gamut_reste_signalee() {
    let sortie = profil(lineaire, inverse_chroma_40);
    let rouge = paire_couleur(
        [255, 0, 0],
        &ProfilSource::srvb(),
        &sortie,
        Rendu::PointNoir,
    )
    .expect("table inverse lue")
    .expect("écart calculable");
    assert!(rouge.ecart > 10.0, "écart {}", rouge.ecart);
}

#[test]
fn sans_compensation_la_couleur_visee_est_la_couleur_demandee() {
    let sortie = profil(lineaire, lineaire_inverse);
    let paire = paire_couleur(
        [10, 200, 30],
        &ProfilSource::srvb(),
        &sortie,
        Rendu::Relatif,
    )
    .expect("table inverse lue")
    .expect("écart calculable");
    assert_eq!(paire.visee, None);
    assert_eq!(paire.couleur_visee, None);
    assert_eq!(paire.ecart_visee, None);
}

#[test]
fn l_analyse_avec_compensation_ne_signale_plus_le_noir() {
    let sortie = profil(lineaire, lineaire_inverse);
    let pixels = vec![[0, 0, 0, 255]; 20];
    let analyse = |rendu| {
        analyser_pixels(&pixels, &ProfilSource::srvb(), &sortie, rendu, &|| false)
            .expect("non annulée")
            .expect("table inverse lue")
    };
    let sans = analyse(Rendu::Relatif);
    assert!(sans.ecarts.iter().all(|&e| e >= 100), "{:?}", sans.ecarts);
    assert_eq!(sans.pastilles.len(), 1);
    let avec = analyse(Rendu::PointNoir);
    assert!(avec.ecarts.iter().all(|&e| e < 5), "{:?}", avec.ecarts);
    assert!(avec.pastilles.is_empty());
    // Le nuage reste placé sur les couleurs de l'image.
    assert_eq!(avec.nuage[0].lab, sans.nuage[0].lab);
}
