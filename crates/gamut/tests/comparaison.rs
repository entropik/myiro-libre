//! Écarts entre profils, fidélité aux mesures, répétabilité et lecture de la
//! limite du gamut, sur des profils d'essai.

use gamut::volume;
use gamut::{
    comparer, fidelite, lire_gam, memes_mesures, points_du_chemin, profil_essai, repetabilite,
    statistiques, Mesure, ProfilEssai, ProfilIcc,
};

/// Profil d'essai linéaire : L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

fn profil(cible: fn([f64; 3]) -> [f64; 3], mesures: Option<String>) -> ProfilIcc {
    let octets = profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [100.0, 0.0, 0.0],
        cible,
        description: "Profil d'essai".into(),
        mesures,
    });
    ProfilIcc::lire(&octets).expect("profil d'essai lisible")
}

#[test]
fn la_mediane_et_le_p95_suivent_numpy() {
    let s = statistiques(&[0.0, 1.0, 2.0, 3.0, 4.0]).expect("cinq écarts");
    assert_eq!(s.nombre, 5);
    assert!((s.moyenne - 2.0).abs() < 1e-12);
    assert!((s.mediane - 2.0).abs() < 1e-12);
    assert!((s.p95 - 3.8).abs() < 1e-12);
    assert!((s.max - 4.0).abs() < 1e-12);
    // 2, 3 et 4 dépassent 1 ; 3 et 4 dépassent 2.
    assert!((s.part_sup_1 - 60.0).abs() < 1e-12);
    assert!((s.part_sup_2 - 40.0).abs() < 1e-12);
    assert!(statistiques(&[]).is_none());
}

#[test]
fn deux_profils_identiques_donnent_des_ecarts_nuls() {
    let p = profil(lineaire, None);
    let e = comparer(&p, &p);
    assert!(e.identiques);
    assert_eq!(e.grille.nombre, 35_937);
    assert_eq!(e.grille.moyenne, 0.0);
    assert_eq!(e.grille.p95, 0.0);
    assert_eq!(e.grille.max, 0.0);
    assert_eq!(e.primaires.len(), 8);
    assert!(e.primaires.iter().all(|&x| x == 0.0));
    assert!(!e.zones.is_empty());
    for z in &e.zones {
        assert!(z.nombre > 0, "zone vide listée : {:?}", z.zone);
        assert_eq!((z.moyenne, z.p95, z.max), (0.0, 0.0, 0.0));
    }
    assert_eq!(e.pires.len(), 10);
    assert!(e.pires.iter().all(|p| p.ecart == 0.0 && !p.ecart.is_nan()));
}

/// Le profil linéaire, presque une unité de clarté plus bas (L\* + 1
/// sortirait de l'échelle du PCS au blanc). Le décalage vaut 652 pas du
/// codage 16 bits du PCS (L\* = w / 652,80) : décalé d'un nombre entier de
/// pas, chaque nœud garde exactement le même écart. Un décalage de 1,0 tout
/// rond donnerait 1,00004 à cause des arrondis du codage.
fn plus_sombre(rvb: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lineaire(rvb);
    [l - 652.0 / 652.8, a, b]
}

#[test]
fn un_decalage_de_clarte_se_retrouve_partout() {
    // Pour un pur ΔL* = 1, ΔE00 = 1 / S_L, de 0,57 (L* 100) à 1 (L* 50).
    let e = comparer(&profil(lineaire, None), &profil(plus_sombre, None));
    assert!(!e.identiques);
    assert!(e.grille.max <= 1.0 + 1e-6, "max {}", e.grille.max);
    assert!(e.grille.moyenne >= 0.55, "moyenne {}", e.grille.moyenne);
    assert_eq!(e.grille.part_sup_1, 0.0);
    assert!(e.pires.windows(2).all(|p| p[0].ecart >= p[1].ecart));
    assert!(e.zones.windows(2).all(|z| z[0].moyenne >= z[1].moyenne));
}

#[test]
fn la_fidelite_d_un_profil_a_ses_propres_noeuds_est_nulle() {
    let mut mesures = Vec::new();
    for r in 0..9 {
        for g in 0..9 {
            for b in 0..9 {
                let rvb = [r, g, b].map(|v| f64::from(v) / 8.0);
                mesures.push(Mesure {
                    rvb,
                    lab: lineaire(rvb),
                });
            }
        }
    }
    let s = fidelite(&profil(lineaire, None), &mesures).expect("729 mesures");
    assert_eq!(s.nombre, 729);
    // Seuls restent les arrondis du codage 16 bits du PCS.
    assert!(s.max < 0.01, "max {}", s.max);
    assert!(fidelite(&profil(lineaire, None), &[]).is_none());
}

#[test]
fn la_repetabilite_compte_les_paires() {
    let blanc = |l: f64| Mesure {
        rvb: [1.0, 1.0, 1.0],
        lab: [l, 0.0, 0.0],
    };
    let noir = |l: f64| Mesure {
        rvb: [0.0, 0.0, 0.0],
        lab: [l, 0.0, 0.0],
    };
    let rouge = Mesure {
        rvb: [1.0, 0.0, 0.0],
        lab: [50.0, 60.0, 40.0],
    };
    let mesures = [
        blanc(95.0),
        blanc(95.0),
        blanc(96.0),
        noir(20.0),
        noir(20.0),
        rouge.clone(),
    ];
    let r = repetabilite(&mesures).expect("plages répétées");
    assert_eq!((r.couleurs, r.paires), (2, 4));
    // Paires des blancs : 0, puis deux fois ΔL* = 1 ; noirs : 0.
    assert!(r.max > 0.5 && r.max < 1.0, "max {}", r.max);
    assert!((r.moyenne - 2.0 * r.max / 4.0).abs() < 1e-9);
    assert!(repetabilite(&[rouge, blanc(95.0)]).is_none());
}

/// Quelques plages d'une mire, Lab tiré du profil linéaire.
fn mire() -> Vec<Mesure> {
    [
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.25, 0.5, 0.75],
        [0.5, 0.5, 0.5],
    ]
    .map(|rvb| Mesure {
        rvb,
        lab: lineaire(rvb),
    })
    .to_vec()
}

#[test]
fn les_memes_mesures_dans_un_autre_ordre_sont_reconnues() {
    let a = mire();
    let mut b = mire();
    b.reverse();
    // Un RVB écrit à 0,001 % près reste le même.
    b[0].rvb[0] -= 0.000_01;
    b[1].lab[0] += 0.05;
    assert!(memes_mesures(&a, &b));
    assert!(!memes_mesures(&a, &b[1..]));
}

#[test]
fn un_ecart_de_0_2_n_est_plus_la_meme_mesure() {
    let a = mire();
    let mut b = mire();
    // Le gris moyen (L* 62,5) décalé de 0,2 en L* : ΔE00 ≈ 0,17.
    b[4].lab[0] += 0.2;
    assert!(!memes_mesures(&a, &b));
    let mut c = mire();
    c[3].rvb[1] += 0.001;
    assert!(!memes_mesures(&a, &c), "un RVB différent de 0,1 %");
}

#[test]
fn les_plages_proches_d_un_chemin_y_sont_placees() {
    let plage = |rvb: [f64; 3], lab: [f64; 3]| Mesure { rvb, lab };
    let mesures = [
        plage([0.0, 0.5, 1.0], [50.0, 30.0, 40.0]),
        // À 0,001 du segment : moins de 0,5 / 255, la plage est gardée.
        plage([0.001, 0.25, 1.0], [40.0, 0.0, -20.0]),
        // À 0,01 : trop loin.
        plage([0.0, 0.5, 0.99], [50.0, 0.0, 0.0]),
        plage([1.0, 0.0, 0.0], [50.0, 60.0, 0.0]),
    ];
    let points = points_du_chemin(&mesures, [0, 0, 255], [0, 255, 255]);
    assert_eq!(points.len(), 2);
    // Rangées le long du chemin : position de 0 à 255, L*, C*.
    let attendus = [[63.75, 40.0, 20.0], [127.5, 50.0, 50.0]];
    for (p, a) in points.iter().zip(attendus) {
        for k in 0..3 {
            assert!((p[k] - a[k]).abs() < 1e-9, "obtenu {p:?}, attendu {a:?}");
        }
    }
}

/// Fichier `.gam` fabriqué sur le modèle de ceux d'iccgamut : un tétraèdre
/// de sommets (L*, a*, b*) (0, 0, 0), (0, 10, 0), (0, 0, 10), (10, 0, 0).
fn gam(triangles: &str) -> String {
    format!(
        "GAMUT  

DESCRIPTOR \"Argyll Gamut surface poligon data\"
         COLOR_REP \"LAB\"
# First come the triangle verticy location

         NUMBER_OF_FIELDS 4
BEGIN_DATA_FORMAT
VERTEX_NO LAB_L LAB_A LAB_B 
         END_DATA_FORMAT

NUMBER_OF_SETS 4
BEGIN_DATA
         0 0.0 0.0 0.0 
1 0.0 10.0 0.0 
2 0.0 0.0 10.0 
3 10.0 0.0 0.0 
         END_DATA

# And then come the triangles

         NUMBER_OF_FIELDS 3
BEGIN_DATA_FORMAT
VERTEX_0 VERTEX_1 VERTEX_2 
         END_DATA_FORMAT

NUMBER_OF_SETS 4
BEGIN_DATA
{triangles}END_DATA
"
    )
}

#[test]
fn un_fichier_gam_se_lit_et_un_indice_faux_est_refuse() {
    let m = lire_gam(&gam("0 2 1 
0 1 3 
0 3 2 
1 2 3 
"))
    .expect("fichier lisible");
    assert_eq!(m.sommets.len(), 4);
    assert_eq!(m.sommets[3], [10.0, 0.0, 0.0]);
    assert_eq!(m.triangles, [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]);
    // Tétraèdre aux arêtes de 10 : 1000 / 6.
    assert!((volume(&m) - 1000.0 / 6.0).abs() < 1e-9);
    let erreur = lire_gam(&gam("0 2 1 
0 1 3 
0 3 2 
1 2 9 
"));
    assert!(erreur.is_err(), "indice 9 accepté : {erreur:?}");
    assert!(lire_gam(
        "GAMUT
"
    )
    .is_err());
}
