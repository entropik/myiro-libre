//! Écarts des pixels d'une image à un profil de sortie (ticket #80), sur des
//! profils d'essai synthétiques. Le chronomètre d'un million de couleurs se
//! lance à part, en release :
//!
//! `cargo test -p gamut --release --test image -- --ignored --nocapture`

use std::time::Instant;

use gamut::{
    analyser_pixels, profil_essai, ErreurIcc, ProfilEssai, ProfilIcc, ProfilSource, ECART_INCONNU,
    POINTS_NUAGE, SECTEURS_TEINTE,
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

fn jamais() -> bool {
    false
}

/// Pixels au-delà de `seuil` ΔE00 (cases de 0,1 ΔE00).
fn au_dela(histogramme: &[u64], seuil: f64) -> u64 {
    let case = (seuil * 10.0).round() as usize;
    histogramme[case..].iter().sum()
}

#[test]
fn une_image_dans_le_gamut_n_a_aucun_pixel_au_dela_du_seuil() {
    let pixels: Vec<[u8; 4]> = (100..=160).map(|v| [v, v, v, 255]).collect();
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(lineaire_inverse)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    assert_eq!(analyse.pixels_comptes, 61);
    assert_eq!(analyse.histogramme.len(), 255);
    assert_eq!(analyse.histogramme.iter().sum::<u64>(), 61);
    assert_eq!(au_dela(&analyse.histogramme, 2.0), 0);
    assert!(analyse.ecarts.iter().all(|&e| e < 20));
}

#[test]
fn une_image_saturee_sur_un_gamut_etroit_est_comptee_hors_gamut() {
    let pixels = vec![[255, 0, 0, 255]; 50];
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(inverse_chroma_40)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    assert_eq!(au_dela(&analyse.histogramme, 10.0), 50);
    assert_eq!(analyse.par_teinte.len(), SECTEURS_TEINTE);
    let totaux: Vec<u64> = analyse
        .par_teinte
        .iter()
        .map(|t| {
            assert_eq!(t.len(), 255);
            t.iter().sum()
        })
        .collect();
    let tete = (0..SECTEURS_TEINTE)
        .max_by_key(|&k| totaux[k])
        .expect("secteurs");
    assert!(tete <= 1, "secteur en tête : {tete}, totaux {totaux:?}");
    assert_eq!(totaux[tete], 50);
    let point = &analyse.nuage[0];
    assert_eq!(point.pixels, 50);
    assert!(point.ecart_max > 10.0, "écart {}", point.ecart_max);
    assert!(point.couleur.starts_with('#') && point.couleur.len() == 7);
}

#[test]
fn un_gris_va_dans_le_secteur_gris() {
    let pixels = vec![[128, 128, 128, 255]; 7];
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(lineaire_inverse)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    let gris: u64 = analyse.par_teinte[SECTEURS_TEINTE - 1].iter().sum();
    assert_eq!(gris, 7);
}

#[test]
fn les_pixels_transparents_ne_comptent_pas() {
    let mut pixels = vec![[255, 0, 0, 255]; 10];
    for k in [1, 3, 5, 7] {
        pixels[k] = [255, 0, 0, 0];
    }
    // Un alpha partiel compte comme opaque.
    pixels[0][3] = 10;
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(inverse_chroma_40)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    assert_eq!(analyse.pixels_comptes, 6);
    assert_eq!(analyse.histogramme.iter().sum::<u64>(), 6);
    assert_eq!(analyse.ecarts.len(), 10);
    for (k, &e) in analyse.ecarts.iter().enumerate() {
        if [1, 3, 5, 7].contains(&k) {
            assert_eq!(e, ECART_INCONNU, "pixel {k}");
        } else {
            assert!((100..ECART_INCONNU).contains(&e), "pixel {k} : {e}");
        }
    }
    assert_eq!(analyse.nuage.iter().map(|p| p.pixels).sum::<u32>(), 6);
}

#[test]
fn deux_pixels_de_meme_couleur_ne_sont_calcules_qu_une_fois() {
    let pixels = vec![[12, 200, 90, 255]; 1_000];
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(lineaire_inverse)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    assert_eq!(analyse.couleurs_distinctes, 1);
    assert_eq!(analyse.pixels_comptes, 1_000);
    assert!(analyse.ecarts.iter().all(|&e| e == analyse.ecarts[0]));
}

#[test]
fn le_nuage_ne_depasse_pas_4000_points_et_garde_les_plus_peuples() {
    // Toutes les couleurs d'un pas de 8 (32³ = 32 768 couleurs), puis un
    // rouge très présent : sa cellule doit venir en tête.
    let mut pixels: Vec<[u8; 4]> = Vec::new();
    for r in (0..=255u16).step_by(8) {
        for g in (0..=255u16).step_by(8) {
            for b in (0..=255u16).step_by(8) {
                pixels.push([r as u8, g as u8, b as u8, 255]);
            }
        }
    }
    pixels.extend(std::iter::repeat_n([200, 30, 30, 255], 5_000));
    let analyse = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(lineaire_inverse)),
        &jamais,
    )
    .expect("non annulée")
    .expect("table inverse lue");
    assert_eq!(analyse.nuage.len(), POINTS_NUAGE);
    assert!(analyse.nuage[0].pixels > 5_000);
    assert!(analyse.nuage.windows(2).all(|p| p[0].pixels >= p[1].pixels));
}

#[test]
fn le_nuage_est_en_colorimetrie_absolue_sur_le_blanc_du_papier() {
    let pixels = vec![[255, 255, 255, 255]; 3];
    let profil = sortie(Some(lineaire_inverse));
    let analyse = analyser_pixels(&pixels, &ProfilSource::srvb(), &profil, &jamais)
        .expect("non annulée")
        .expect("table inverse lue");
    let blanc = profil.blanc_support();
    for k in 0..3 {
        assert!(
            (analyse.nuage[0].lab[k] - blanc[k]).abs() < 0.5,
            "{:?} contre {blanc:?}",
            analyse.nuage[0].lab
        );
    }
}

#[test]
fn sans_b2a1_l_analyse_est_refusee_avec_sa_raison() {
    let pixels = vec![[10, 20, 30, 255]; 4];
    let resultat = analyser_pixels(&pixels, &ProfilSource::srvb(), &sortie(None), &jamais);
    assert!(matches!(resultat, Ok(Err(ErreurIcc::SansTableInverse))));
}

#[test]
fn l_analyse_s_arrete_quand_on_annule() {
    let pixels = vec![[10, 20, 30, 255]; 4];
    let resultat = analyser_pixels(
        &pixels,
        &ProfilSource::srvb(),
        &sortie(Some(lineaire_inverse)),
        &|| true,
    );
    assert!(resultat.is_err());
}

#[test]
#[ignore = "chronomètre, à lancer en release"]
fn une_image_d_un_million_de_couleurs_tient_en_dix_secondes() {
    // Un million de couleurs distinctes, réparties sur tout le cube RVB.
    let pixels: Vec<[u8; 4]> = (0..1_000_000u32)
        .map(|i| {
            let v = i * 16 + i % 16;
            [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
        })
        .collect();
    let profil = sortie(Some(inverse_chroma_40));
    let source = ProfilSource::srvb();
    let debut = Instant::now();
    let analyse = analyser_pixels(&pixels, &source, &profil, &jamais)
        .expect("non annulée")
        .expect("table inverse lue");
    let duree = debut.elapsed();
    println!(
        "{} couleurs distinctes en {:.2} s",
        analyse.couleurs_distinctes,
        duree.as_secs_f64()
    );
    assert_eq!(analyse.couleurs_distinctes, 1_000_000);
    assert!(duree.as_secs_f64() < 10.0, "{duree:?}");
}
