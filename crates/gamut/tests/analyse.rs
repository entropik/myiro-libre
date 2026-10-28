//! Surface du cube RVB, volume, table de chroma, couleurs de base et chemins,
//! sur le profil d'essai linéaire (blanc D50).

use gamut::{analyser, chemin, profil_essai, surface_cube, ProfilEssai, ProfilIcc};

/// Profil d'essai linéaire : L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

fn profil() -> ProfilIcc {
    let octets = profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [100.0, 0.0, 0.0],
        cible: lineaire,
        description: "Profil d'essai lineaire".into(),
        mesures: None,
    });
    ProfilIcc::lire(&octets).expect("profil d'essai lisible")
}

fn proche(obtenu: [f64; 3], attendu: [f64; 3], tolerance: f64) {
    for k in 0..3 {
        assert!(
            (obtenu[k] - attendu[k]).abs() <= tolerance,
            "obtenu {obtenu:?}, attendu {attendu:?} à {tolerance} près"
        );
    }
}

#[test]
fn la_surface_du_cube_a_ses_sommets_et_triangles() {
    let (sommets, triangles) = surface_cube(17);
    assert_eq!(sommets.len(), 1_734);
    assert_eq!(triangles.len(), 3_072);
    for t in &triangles {
        let [p0, p1, p2] = t.map(|i| sommets[i as usize]);
        let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let normale = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let centre: Vec<f64> = (0..3)
            .map(|k| (p0[k] + p1[k] + p2[k]) / 3.0 - 0.5)
            .collect();
        let sortant: f64 = (0..3).map(|k| normale[k] * centre[k]).sum();
        assert!(sortant > 0.0, "triangle {t:?} tourné vers l'intérieur");
    }
}

#[test]
fn le_volume_d_une_transformation_lineaire_est_son_determinant() {
    let analyse = analyser(&profil());
    // |det| de ((25, 25, 25), (60, −60, 0), (0, 60, −60)).
    let attendu = 270_000.0;
    assert!(
        (analyse.volume_surface - attendu).abs() <= attendu * 0.001,
        "volume {}",
        analyse.volume_surface
    );
}

#[test]
fn les_couleurs_de_base_sont_dans_l_ordre_avec_leur_teinte() {
    let analyse = analyser(&profil());
    let noms: Vec<&str> = analyse.primaires.iter().map(|p| p.nom).collect();
    assert_eq!(
        noms,
        ["blanc", "noir", "rouge", "vert", "bleu", "cyan", "magenta", "jaune"]
    );
    let rouge = &analyse.primaires[2];
    assert_eq!(rouge.rvb, [255, 0, 0]);
    proche(rouge.lab, [50.0, 60.0, 0.0], 0.01);
    let h = rouge.h.expect("le rouge a une teinte");
    assert!(h.abs() <= 0.01, "teinte du rouge {h}");
    assert!((rouge.c - 60.0).abs() <= 0.01);
    assert!(rouge.ecran.starts_with('#') && rouge.ecran.len() == 7);
    let blanc = &analyse.primaires[0];
    assert_eq!(blanc.h, None, "le blanc est un gris");
    assert_eq!(blanc.ecran, "#ffffff");
    assert!(!blanc.ramenee);
}

#[test]
fn la_densite_du_noir_vient_de_y() {
    let analyse = analyser(&profil());
    proche(analyse.noir, [25.0, 0.0, 0.0], 0.01);
    proche(analyse.blanc, [100.0, 0.0, 0.0], 0.01);
    let attendu = -(((25.0f64 + 16.0) / 116.0).powi(3)).log10();
    assert!((attendu - 1.36).abs() <= 0.01);
    assert!(
        (analyse.dmax_approche - attendu).abs() <= 0.01,
        "densité {}",
        analyse.dmax_approche
    );
}

#[test]
fn la_coupe_et_la_saturation_par_teinte_ont_leurs_dimensions() {
    let analyse = analyser(&profil());
    assert_eq!(analyse.coupes.len(), 100);
    assert!(analyse.coupes.iter().all(|ligne| ligne.len() == 180));
    assert_eq!(analyse.chroma_par_teinte.len(), 180);
    for (h, &max) in analyse.chroma_par_teinte.iter().enumerate() {
        for ligne in &analyse.coupes {
            assert!(max >= ligne[h], "teinte {h}");
        }
    }
    // Rien sous le noir du profil (L* 25) ; de la couleur à mi-clarté.
    assert!(analyse.coupes[10].iter().all(|&c| c == 0.0));
    assert!(analyse.coupes[50].iter().all(|&c| c > 0.0));
    // Valeurs arrondies à 0,1.
    for &c in analyse.coupes.iter().flatten() {
        assert!(((c * 10.0).round() - c * 10.0).abs() < 1e-9, "{c}");
    }
}

#[test]
fn la_coupe_suit_la_section_du_gamut_sans_dents() {
    let analyse = analyser(&profil());
    // Ligne 50 : section par le plan L* = 50,5, soit r + g + b = 1,02. Sur
    // le rayon de 1° (milieu de la colonne 0), le bord est l'arête r = 1 :
    // g = (0,02 + tan 1°) / (2 + tan 1°), C* = 60·(1 − g) / cos 1° = 58,9.
    let c = analyse.coupes[50][0];
    assert!((c - 58.9).abs() <= 0.05, "C* à L* 50, teinte 1° : {c}");
    // Une coupe à clarté constante est un contour lisse : aucune case ne
    // rentre vers le centre entre ses deux voisines.
    for l in 30..=90 {
        let ligne = &analyse.coupes[l];
        for h in 0..180 {
            let voisines = ligne[(h + 179) % 180].min(ligne[(h + 1) % 180]);
            assert!(
                ligne[h] >= 0.9 * voisines,
                "dent à L* {l}, teinte {} : {} entre {} et {}",
                2 * h + 1,
                ligne[h],
                ligne[(h + 179) % 180],
                ligne[(h + 1) % 180]
            );
        }
    }
}

#[test]
fn l_affichage_est_la_surface_17_avec_ses_couleurs() {
    let analyse = analyser(&profil());
    let affichage = &analyse.affichage;
    assert_eq!(affichage.sommets.len(), 1_734);
    assert_eq!(affichage.couleurs.len(), 1_734);
    assert_eq!(affichage.triangles.len(), 3_072);
    assert_eq!(analyse.grille, 9);
    assert_eq!(analyse.version, "2.4");
    assert_eq!(
        analyse.description.as_deref(),
        Some("Profil d'essai lineaire")
    );
}

#[test]
fn un_chemin_a_256_points_du_depart_a_l_arrivee() {
    let p = profil();
    let points = chemin(&p, [0, 0, 255], [0, 255, 255]);
    assert_eq!(points.len(), 256);
    let lc = |lab: [f64; 3]| [lab[0], lab[1].hypot(lab[2])];
    let depart = lc(p.lab_absolu([0.0, 0.0, 1.0]));
    let arrivee = lc(p.lab_absolu([0.0, 1.0, 1.0]));
    assert!((points[0][0] - depart[0]).abs() < 1e-9 && (points[0][1] - depart[1]).abs() < 1e-9);
    assert!(
        (points[255][0] - arrivee[0]).abs() < 1e-9 && (points[255][1] - arrivee[1]).abs() < 1e-9
    );
}

#[test]
fn chaque_carre_du_cube_est_coupe_par_la_diagonale_du_prototype() {
    // Sur une surface courbe, le volume dépend de la diagonale choisie : le
    // prototype coupe chaque carré entre ses deux coins où une seule des
    // deux coordonnées libres de la face vaut 1.
    let (sommets, triangles) = surface_cube(2);
    assert_eq!(triangles.len(), 12);
    for t in &triangles {
        let coins = t.map(|i| sommets[i as usize]);
        let fixe = (0..3)
            .find(|&k| coins.iter().all(|c| c[k] == coins[0][k]))
            .expect("triangle posé sur une face");
        let libres: Vec<usize> = (0..3).filter(|&k| k != fixe).collect();
        let mixtes = coins
            .iter()
            .filter(|c| c[libres[0]] != c[libres[1]])
            .count();
        assert_eq!(mixtes, 2, "triangle {coins:?} coupé par l'autre diagonale");
    }
}
