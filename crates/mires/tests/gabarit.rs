//! Gabarit des pages en bandes du MYIRO-1 : tailles validées, capacité d'une
//! page et contraintes du manuel (bande de 257 mm au plus, blancs compris).

use mires::{
    nombre_plages_myiro1, ErreurMire, Format, Gabarit, TailleMire, DISTANCE_TEXTE_MM,
    MYIRO1_BANDE_MAX_MM, MYIRO1_BLANC_BOUT_MM, MYIRO1_MARGE_PAGE_MM,
};

#[test]
fn les_tailles_validees_sont_celles_du_mainteneur() {
    use TailleMire::*;
    assert_eq!(
        [Rapide, Standard, Approfondi].map(nombre_plages_myiro1),
        [232, 464, 928]
    );
}

/// Page en portrait (demande du mainteneur, 28 octobre 2026) : bandes de 22
/// plages de 8 mm, 3 mm entre deux bandes.
#[test]
fn la_capacite_par_defaut_tient_les_tailles_en_1_1_2_pages_a4() {
    let my = Gabarit::par_defaut(Format::A4);
    assert_eq!(my.cote_mm, 8.0);
    assert_eq!(my.page_mm(), (210.0, 297.0));
    assert_eq!(
        (my.colonnes(), my.rangees(), my.plages_par_page()),
        (22, 24, 528)
    );
    assert_eq!([232, 464, 928].map(|n| my.pages(n)), [1, 1, 2]);
}

#[test]
fn la_capacite_a3_est_1015() {
    let my = Gabarit::par_defaut(Format::A3);
    assert_eq!(my.page_mm(), (297.0, 420.0));
    assert_eq!(
        (my.colonnes(), my.rangees(), my.plages_par_page()),
        (29, 35, 1015)
    );
    assert_eq!([232, 464, 928].map(|n| my.pages(n)), [1, 1, 1]);
}

/// Marge technique d'impression en feuille, donnée par le mainteneur :
/// aucune plage, aucun blanc de bout de bande ni aucun texte à moins de
/// 5 mm d'un bord de la page.
#[test]
fn rien_n_est_a_moins_de_5_mm_d_un_bord() {
    for format in [Format::A4, Format::A3] {
        for cote in [7.0, 8.0, 12.5, 20.0] {
            let g = Gabarit::nouveau(format, cote).unwrap();
            let (largeur, hauteur) = g.page_mm();
            let dans_la_marge = |x: f64, y: f64, l: f64, h: f64| {
                x >= MYIRO1_MARGE_PAGE_MM - 1e-9
                    && y >= MYIRO1_MARGE_PAGE_MM - 1e-9
                    && x + l <= largeur - MYIRO1_MARGE_PAGE_MM + 1e-9
                    && y + h <= hauteur - MYIRO1_MARGE_PAGE_MM + 1e-9
            };
            for r in 0..g.rangees() {
                let (x, y) = g.origine_plage(r, 0);
                let bande = g.colonnes() as f64 * cote + 2.0 * MYIRO1_BLANC_BOUT_MM;
                assert!(
                    dans_la_marge(x - MYIRO1_BLANC_BOUT_MM, y, bande, cote),
                    "{format:?} {cote} mm, bande {r}"
                );
            }
            let z = g.zone_texte();
            assert!(dans_la_marge(z.x, z.y, z.largeur, z.hauteur), "{format:?}");
        }
    }
}

/// La bande, blancs de bout compris, est centrée dans la largeur de la page.
#[test]
fn chaque_bande_est_centree_dans_la_largeur() {
    for format in [Format::A4, Format::A3] {
        let g = Gabarit::par_defaut(format);
        let (largeur, _) = g.page_mm();
        for r in [0, g.rangees() - 1] {
            let debut = g.origine_plage(r, 0).0 - MYIRO1_BLANC_BOUT_MM;
            let fin = g.origine_plage(r, g.colonnes() - 1).0 + g.cote_mm + MYIRO1_BLANC_BOUT_MM;
            assert!(
                (debut - (largeur - fin)).abs() < 1e-9,
                "{format:?} : {debut} et {fin}"
            );
        }
    }
    // A4 : 22 plages de 8 mm, soit 176 mm, à 17 mm de chaque bord.
    assert_eq!(Gabarit::par_defaut(Format::A4).origine_plage(0, 0).0, 17.0);
}

#[test]
fn un_cote_hors_limites_est_refuse() {
    let refus = Err(ErreurMire::CoteHorsLimites {
        min: 7.0,
        max: None,
    });
    assert_eq!(Gabarit::limites_cote(), (7.0, None));
    assert_eq!(Gabarit::nouveau(Format::A4, 6.0), refus);
    assert_eq!(Gabarit::nouveau(Format::A4, f64::NAN), refus);
    assert!(Gabarit::nouveau(Format::A4, 7.0).is_ok());
}

#[test]
fn un_cote_sans_place_sur_la_page_est_refuse() {
    // Aucune bande de 250 mm de haut ne tient sur une page A4.
    assert!(matches!(
        Gabarit::nouveau(Format::A4, 250.0),
        Err(ErreurMire::CoteHorsLimites { .. })
    ));
}

#[test]
fn une_bande_myiro1_ne_depasse_jamais_257_mm_blancs_compris() {
    for format in [Format::A4, Format::A3] {
        for cote in 7..=30 {
            let g = Gabarit::nouveau(format, cote as f64).unwrap();
            let bande = g.colonnes() as f64 * g.cote_mm + 2.0 * MYIRO1_BLANC_BOUT_MM;
            assert!(
                bande <= MYIRO1_BANDE_MAX_MM,
                "{format:?} {cote} mm : {bande}"
            );
            // Blancs compris, la bande tient dans la page, marges respectées.
            let (x, _) = g.origine_plage(0, 0);
            let fin = g.origine_plage(0, g.colonnes() - 1).0 + g.cote_mm;
            assert!(x - MYIRO1_BLANC_BOUT_MM >= MYIRO1_MARGE_PAGE_MM);
            assert!(fin + MYIRO1_BLANC_BOUT_MM <= g.page_mm().0 - MYIRO1_MARGE_PAGE_MM);
            let bas = g.origine_plage(g.rangees() - 1, 0).1 + g.cote_mm;
            assert!(
                bas <= g.page_mm().1 - MYIRO1_MARGE_PAGE_MM,
                "{format:?} {cote}"
            );
        }
    }
}

#[test]
fn deux_plages_voisines_se_touchent_et_les_bandes_sont_separees() {
    let g = Gabarit::par_defaut(Format::A4);
    let (x0, y0) = g.origine_plage(0, 0);
    let (x1, y1) = g.origine_plage(0, 1);
    assert_eq!((x1 - x0, y1 - y0), (8.0, 0.0));
    let (_, y_bande_2) = g.origine_plage(1, 0);
    assert_eq!(y_bande_2 - y0, 8.0 + 3.0);
}

#[test]
fn la_zone_de_texte_est_a_2_mm_au_moins_des_plages() {
    for format in [Format::A4, Format::A3] {
        let g = Gabarit::par_defaut(format);
        let z = g.zone_texte();
        let haut_des_plages = g.origine_plage(0, 0).1;
        assert!(z.hauteur > 10.0);
        assert!(z.y + z.hauteur + DISTANCE_TEXTE_MM <= haut_des_plages + 1e-9);
        assert!(z.x >= MYIRO1_MARGE_PAGE_MM && z.y >= MYIRO1_MARGE_PAGE_MM);
        assert!(z.x + z.largeur <= g.page_mm().0 - MYIRO1_MARGE_PAGE_MM);
    }
}

#[test]
fn chaque_erreur_a_sa_cle_de_texte() {
    assert_eq!(
        ErreurMire::CoteHorsLimites {
            min: 7.0,
            max: None
        }
        .cle(),
        "mire.erreur.cote"
    );
    assert_eq!(
        ErreurMire::Contraste { defauts: 3 }.cle(),
        "mire.erreur.contraste"
    );
    assert_eq!(
        ErreurMire::MireFd9Introuvable.cle(),
        "mire.erreur.fd9_introuvable"
    );
}
