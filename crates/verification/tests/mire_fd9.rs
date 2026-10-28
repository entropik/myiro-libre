//! Mire de vérification FD-9 (plan 7, tâche 10) : couleurs tirées une fois,
//! d'une graine figée, différentes de la mire de profilage RGB-2520, et
//! écrites en XML « ChartDefinition » de FD-S2w.
//!
//! Le test réel compare à un export CGATS de la RGB-2520, local et non
//! versionné : `MIRE_RGB2520=<export.txt> cargo test -p verification --test
//! mire_fd9 -- --ignored`.

use verification::{
    couleurs_verification_fd9, ecrire_xml_fds2w, lire_xml_fds2w, mire_verification_fd9,
    MIRE_FD9_VERSION, RGB2520_NIVEAUX,
};

/// Les huit coins du cube RVB : blanc du papier, noir, primaires et
/// secondaires. Toute mire les contient, la RGB-2520 comprise.
fn coin(c: &[u8; 3]) -> bool {
    c.iter().all(|v| *v == 0 || *v == 255)
}

#[test]
fn les_couleurs_de_verification_sont_toutes_hors_de_la_rgb2520() {
    let couleurs = couleurs_verification_fd9();
    assert!((190..=230).contains(&couleurs.len()), "{}", couleurs.len());
    // Même graine, même liste.
    assert_eq!(couleurs_verification_fd9(), couleurs);
    // Aucune couleur deux fois.
    let mut uniques = couleurs.clone();
    uniques.sort_unstable();
    uniques.dedup();
    assert_eq!(uniques.len(), couleurs.len());
    // Hors des coins, chaque couleur a au moins une composante sur un niveau
    // que la RGB-2520 n'emploie jamais : elle ne peut pas en faire partie.
    for c in couleurs.iter().filter(|c| !coin(c)) {
        assert!(
            c.iter().any(|v| !RGB2520_NIVEAUX.contains(v)),
            "{c:?} pourrait être une plage de la RGB-2520"
        );
    }
    // Ancres : les huit coins, une rampe de gris et des tons chair.
    for r in [0, 255] {
        for g in [0, 255] {
            for b in [0, 255] {
                assert!(couleurs.contains(&[r, g, b]), "coin {r} {g} {b} absent");
            }
        }
    }
    let gris = couleurs
        .iter()
        .filter(|c| c[0] == c[1] && c[1] == c[2] && !coin(c))
        .count();
    assert!(gris >= 12, "{gris} gris");
    let chair = couleurs
        .iter()
        .filter(|c| c[0] > c[1] && c[1] > c[2] && c[0] - c[2] < 110 && c[0] > 100)
        .count();
    assert!(chair >= 10, "{chair} tons chair");
}

#[test]
fn le_xml_fds2w_relu_redonne_les_memes_couleurs_et_la_meme_geometrie() {
    let mire = mire_verification_fd9();
    let xml = ecrire_xml_fds2w(&mire);
    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml.contains("Identifier=\"ChartDefinition\""));
    assert!(xml.contains("Application=\"FD-S2w\""));
    assert!(xml.contains("ColorFormat=\"RGB\""));
    // Valeurs RVB 0 à 255 multipliées par 100, comme les mires FD-9 de FD-S2w.
    assert!(xml.contains("ColorValue1=\"25500\""));
    let lue = lire_xml_fds2w(&xml).unwrap();
    assert_eq!(lue, mire);
    assert_eq!(lue.plages.len(), lue.colonnes * lue.rangees);
    let mut relues: Vec<[u8; 3]> = lue.plages.iter().map(|p| p.rvb).collect();
    let mut tirees = couleurs_verification_fd9();
    relues.sort_unstable();
    tirees.sort_unstable();
    assert_eq!(relues, tirees);
}

#[test]
fn les_noms_sont_echappes_dans_le_xml() {
    let mut mire = mire_verification_fd9();
    mire.nom = "Vérif <A&B> \"1\"".into();
    let xml = ecrire_xml_fds2w(&mire);
    assert!(xml.contains("ChartName=\"Vérif &lt;A&amp;B&gt; &quot;1&quot;\""));
    assert_eq!(lire_xml_fds2w(&xml).unwrap().nom, mire.nom);
}

/// Conditions de la charte du FD-9 (manuel de FD-S2w) : plages de 6 à 30 mm,
/// espace entre plages sous 17 % de leur largeur, marges d'au moins 23 mm en
/// tête, 33 mm en pied et 4 mm sur les côtés, zone de 300 mm de large au plus.
#[test]
fn la_mire_tient_dans_la_surface_numerisable_du_fd9() {
    let m = mire_verification_fd9();
    let (largeur, hauteur) = m.page_mm;
    assert!(
        [(210.0, 297.0), (297.0, 420.0)].contains(&(largeur, hauteur)),
        "A4 ou A3 : {largeur} × {hauteur}"
    );
    assert!((6.0..=30.0).contains(&m.plage_mm));
    let espace = m.pas_mm - m.plage_mm;
    assert!(
        espace >= 0.0 && espace < 0.17 * m.plage_mm,
        "espace {espace}"
    );
    let zone_l = m.colonnes as f64 * m.pas_mm - espace;
    let zone_h = m.rangees as f64 * m.pas_mm - espace;
    assert!(zone_l <= 300.0);
    assert!(m.origine_mm.0 >= 4.0 && m.origine_mm.0 + zone_l <= largeur - 4.0);
    assert!(m.origine_mm.1 >= 23.0 && m.origine_mm.1 + zone_h <= hauteur - 33.0);
    assert_eq!(m.version, MIRE_FD9_VERSION);
}

/// Deux voisines (côte à côte ou l'une sous l'autre) sont l'une claire,
/// l'autre foncée : le scan du FD-9 sépare mieux les plages.
#[test]
fn deux_voisines_sont_l_une_claire_l_autre_foncee() {
    let m = mire_verification_fd9();
    let luma =
        |c: [u8; 3]| 0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2]);
    let mut lumas: Vec<f64> = m.plages.iter().map(|p| luma(p.rvb)).collect();
    lumas.sort_by(f64::total_cmp);
    let mediane = lumas[lumas.len() / 2];
    let a = |rangee: usize, colonne: usize| {
        let p = m
            .plages
            .iter()
            .find(|p| p.rangee == rangee && p.colonne == colonne)
            .unwrap();
        luma(p.rvb) >= mediane
    };
    for r in 1..=m.rangees {
        for c in 1..m.colonnes {
            assert_ne!(
                a(r, c),
                a(r, c + 1),
                "rangée {r}, colonnes {c} et {}",
                c + 1
            );
        }
    }
}

#[test]
#[ignore = "demande un export CGATS de la RGB-2520 (MIRE_RGB2520)"]
fn aucune_couleur_hors_des_coins_n_est_dans_la_vraie_rgb2520() {
    let chemin = std::env::var("MIRE_RGB2520").expect("variable MIRE_RGB2520");
    let texte = std::fs::read_to_string(chemin).unwrap();
    let mut rgb2520 = Vec::new();
    let mut donnees = false;
    for ligne in texte.lines() {
        match ligne.trim() {
            "BEGIN_DATA" => donnees = true,
            "END_DATA" => donnees = false,
            l if donnees => {
                let v: Vec<f64> = l
                    .split_whitespace()
                    .skip(1)
                    .take(3)
                    .map(|x| x.parse().unwrap())
                    .collect();
                rgb2520.push([v[0], v[1], v[2]].map(|x| x.round() as u8));
            }
            _ => {}
        }
    }
    assert_eq!(rgb2520.len(), 2520);
    let mut niveaux: Vec<u8> = rgb2520.iter().flatten().copied().collect();
    niveaux.sort_unstable();
    niveaux.dedup();
    assert_eq!(niveaux, RGB2520_NIVEAUX);
    let communes: Vec<_> = couleurs_verification_fd9()
        .into_iter()
        .filter(|c| !coin(c) && rgb2520.contains(c))
        .collect();
    assert!(communes.is_empty(), "{communes:?}");
}
