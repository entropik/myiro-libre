//! Placement des plages MYIRO-1 dans les bandes : chaque plage tranche sur
//! sa voisine de gauche, et les bouts de bande sur le blanc papier (ΔE*ab >
//! 10, sur un Lab estimé), mise en page reproductible et vérifiable.

use std::time::Instant;

use mires::{
    estimer_lab, mettre_en_page, verifier, Defaut, ErreurMire, Format, Gabarit, MiseEnPage, Page,
    PlagePlacee,
};

/// Cube RVB régulier de `pas`³ valeurs, de 0 à 100.
fn grille(pas: usize) -> Vec<[f64; 3]> {
    let niveau = |i: usize| i as f64 * 100.0 / (pas - 1) as f64;
    let mut v = Vec::new();
    for r in 0..pas {
        for g in 0..pas {
            for b in 0..pas {
                v.push([niveau(r), niveau(g), niveau(b)]);
            }
        }
    }
    v
}

#[test]
fn le_lab_estime_du_blanc_et_du_noir() {
    let blanc = estimer_lab([100.0, 100.0, 100.0]);
    assert!((blanc.l - 100.0).abs() < 1e-6 && blanc.a.abs() < 1e-6 && blanc.b.abs() < 1e-6);
    let noir = estimer_lab([0.0, 0.0, 0.0]);
    assert!(noir.l.abs() < 1e-6);
    // Gris moyen sRGB (128/255) : L* de 53,6 environ.
    let gris = estimer_lab([128.0 / 2.55; 3]);
    assert!((gris.l - 53.585).abs() < 0.01, "{}", gris.l);
}

#[test]
fn une_mire_myiro1_approfondie_se_place_sans_defaut() {
    let v = grille(10)[..928].to_vec();
    let g = Gabarit::par_defaut(Format::A4);
    let debut = Instant::now();
    let m = mettre_en_page(&v, &g, 7).unwrap();
    println!("mise en page de 928 plages : {:?}", debut.elapsed());
    assert_eq!(m.pages.len(), 4);
    assert_eq!(
        m.pages.iter().map(|p| p.numero).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert!(verifier(&m, &v).is_empty());
}

#[test]
fn une_mire_myiro1_rapide_se_place_sans_defaut() {
    let v = grille(6);
    let g = Gabarit::par_defaut(Format::A4);
    let m = mettre_en_page(&v, &g, 1).unwrap();
    assert_eq!(m.pages.len(), 1);
    assert_eq!(m.pages[0].plages.len(), 216);
    assert!(verifier(&m, &v).is_empty());
}

#[test]
fn chaque_valeur_est_placee_une_seule_fois() {
    let v = grille(8);
    let g = Gabarit::par_defaut(Format::A3);
    let m = mettre_en_page(&v, &g, 3).unwrap();
    let mut index: Vec<usize> = m
        .pages
        .iter()
        .flat_map(|p| p.plages.iter().map(|q| q.index))
        .collect();
    index.sort();
    assert_eq!(index, (0..v.len()).collect::<Vec<_>>());
    // Les bandes se remplissent dans l'ordre de lecture : seule la dernière
    // bande de la dernière page peut rester incomplète.
    for p in &m.pages[..m.pages.len() - 1] {
        assert_eq!(p.plages.len(), g.plages_par_page());
    }
}

#[test]
fn la_meme_graine_donne_la_meme_mise_en_page() {
    let v = grille(6);
    let g = Gabarit::par_defaut(Format::A4);
    assert_eq!(
        mettre_en_page(&v, &g, 42).unwrap(),
        mettre_en_page(&v, &g, 42).unwrap()
    );
}

#[test]
fn deux_graines_donnent_deux_empreintes() {
    let v = grille(6);
    let g = Gabarit::par_defaut(Format::A4);
    let a = mettre_en_page(&v, &g, 1).unwrap();
    let b = mettre_en_page(&v, &g, 2).unwrap();
    assert_ne!(a.pages, b.pages);
    assert_ne!(a.empreinte(), b.empreinte());
    assert_eq!(a.empreinte().len(), 64);
    assert!(a
        .empreinte()
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
}

#[test]
fn des_plages_toutes_claires_sont_refusees() {
    let v: Vec<[f64; 3]> = (0..30)
        .map(|i| {
            let x = 95.0 + (i % 6) as f64;
            [x.min(100.0), 100.0 - (i % 5) as f64, 97.0 + (i % 4) as f64]
        })
        .collect();
    let g = Gabarit::par_defaut(Format::A4);
    assert!(matches!(
        mettre_en_page(&v, &g, 7),
        Err(ErreurMire::Contraste { .. })
    ));
}

fn a_la_main(plages: &[(usize, usize, usize)]) -> MiseEnPage {
    MiseEnPage {
        gabarit: Gabarit::par_defaut(Format::A4),
        graine: 0,
        pages: vec![Page {
            numero: 1,
            plages: plages
                .iter()
                .map(|&(index, rangee, colonne)| PlagePlacee {
                    index,
                    rangee,
                    colonne,
                })
                .collect(),
        }],
    }
}

#[test]
fn verifier_signale_deux_voisines_identiques() {
    let v = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 100.0]];
    let m = a_la_main(&[(0, 0, 0), (1, 0, 1), (2, 0, 2)]);
    assert_eq!(
        verifier(&m, &v),
        [Defaut::ContrasteVoisine {
            page: 1,
            rangee: 0,
            colonne: 1
        }]
    );
}

#[test]
fn verifier_signale_une_plage_claire_au_bout_d_une_bande() {
    let v = [[0.0, 0.0, 0.0], [100.0, 100.0, 96.0]];
    let m = a_la_main(&[(0, 0, 0), (1, 0, 1)]);
    assert_eq!(
        verifier(&m, &v),
        [Defaut::ContrasteBlanc {
            page: 1,
            rangee: 0,
            colonne: 1
        }]
    );
}

#[test]
fn verifier_signale_nombre_doublon_et_hors_gabarit() {
    let v = [[0.0, 0.0, 0.0], [0.0, 0.0, 100.0], [100.0, 0.0, 0.0]];
    let m = a_la_main(&[(0, 0, 0), (1, 0, 1), (0, 0, 2), (2, 8, 0)]);
    let defauts = verifier(&m, &v);
    assert!(
        defauts.contains(&Defaut::Doublon { index: 0 }),
        "{defauts:?}"
    );
    assert!(defauts.contains(&Defaut::HorsGabarit {
        page: 1,
        rangee: 8,
        colonne: 0
    }));
    let m = a_la_main(&[(0, 0, 0), (1, 0, 1)]);
    assert_eq!(
        verifier(&m, &v),
        [Defaut::Nombre {
            attendu: 3,
            trouve: 2
        }]
    );
}

#[test]
fn la_mise_en_page_se_relit_a_l_identique() {
    let v = grille(6);
    let m = mettre_en_page(&v, &Gabarit::par_defaut(Format::A4), 9).unwrap();
    let json = serde_json::to_string(&m).unwrap();
    let relue: MiseEnPage = serde_json::from_str(&json).unwrap();
    assert_eq!(relue, m);
    assert_eq!(relue.empreinte(), m.empreinte());
}
