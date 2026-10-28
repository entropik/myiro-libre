//! Contrôle de cohérence des mesures avant un calcul de profil (ticket #1),
//! vu par l'API publique de la crate. Toutes les mesures sont synthétiques,
//! sauf celle du test ignoré, lue sur le poste et jamais versionnée.

use colorimetrie::{blanc_d50, xyz_vers_lab, Xyz};
use pont_protocole::ConditionMesure;
use profilage::{controler_coherence, Critere, PlageProfil, PlagesProfil};

/// Suite pseudo-aléatoire splitmix64 : graine fixe, mêmes valeurs à chaque
/// exécution.
struct Hasard(u64);

impl Hasard {
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Réel de 0 (inclus) à 1 (exclu).
    fn reel(&mut self) -> f64 {
        (self.suivant() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn melanger<T>(&mut self, valeurs: &mut [T]) {
        for i in (1..valeurs.len()).rev() {
            let j = (self.suivant() % (i as u64 + 1)) as usize;
            valeurs.swap(i, j);
        }
    }
}

/// RVB de 0 à 100 lu comme du sRGB, vers Lab D50 (matrice sRGB adaptée à
/// D50 par Bradford).
fn srgb_vers_lab(rvb: [f64; 3]) -> [f64; 3] {
    let lineaire = rvb.map(|c| {
        let c = c / 100.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let [r, g, b] = lineaire;
    let xyz = Xyz {
        x: 100.0 * (0.436_074_7 * r + 0.385_064_9 * g + 0.143_080_4 * b),
        y: 100.0 * (0.222_504_5 * r + 0.716_878_6 * g + 0.060_616_9 * b),
        z: 100.0 * (0.013_932_2 * r + 0.097_104_5 * g + 0.714_173_3 * b),
    };
    let blanc = blanc_d50();
    let lab = xyz_vers_lab(xyz, blanc).unwrap();
    [lab.l, lab.a, lab.b]
}

/// Lab mesuré sur un papier simulé : noir à L* 15, couleurs un peu ternies,
/// plus un bruit fixe de ±0,3.
fn papier(rvb: [f64; 3], hasard: &mut Hasard) -> [f64; 3] {
    let [l, a, b] = srgb_vers_lab(rvb);
    let mut bruit = || (hasard.reel() * 2.0 - 1.0) * 0.3;
    [
        15.0 + 0.8 * l + bruit(),
        a * 0.85 + bruit(),
        b * 0.85 + bruit(),
    ]
}

fn plages(rvb: &[[f64; 3]], graine: u64) -> PlagesProfil {
    let mut hasard = Hasard(graine);
    PlagesProfil {
        plages: rvb
            .iter()
            .map(|&rvb| PlageProfil {
                rvb,
                lab: papier(rvb, &mut hasard),
                spectre: None,
            })
            .collect(),
        condition: ConditionMesure::M1,
        condition_supposee: false,
        rvb_supposes: false,
        longueurs_onde: None,
    }
}

/// Mesure synthétique : grille RVB `cote`³ (dont `cote` gris, le blanc et
/// le noir), `aleatoires` plages tirées au hasard et `doublons` plages de la
/// grille répétées, dans un ordre mélangé, comme une mire de targen.
fn mesure_synthetique(cote: usize, aleatoires: usize, doublons: usize) -> PlagesProfil {
    let pas = 100.0 / (cote - 1) as f64;
    let mut rvb = Vec::new();
    for r in 0..cote {
        for g in 0..cote {
            for b in 0..cote {
                rvb.push([r as f64 * pas, g as f64 * pas, b as f64 * pas]);
            }
        }
    }
    let grille = rvb.len();
    let mut hasard = Hasard(0x00C0_FFEE);
    for _ in 0..aleatoires {
        let c = [hasard.reel(), hasard.reel(), hasard.reel()];
        rvb.push(c.map(|v| (v * 10_000.0).round() / 100.0));
    }
    for k in 0..doublons {
        rvb.push(rvb[(k * 97 + 13) % grille]);
    }
    hasard.melanger(&mut rvb);
    plages(&rvb, 0x5EED)
}

/// 840 plages : grille 9 × 9 × 9, 100 plages au hasard, 11 doublons.
fn mesure_saine() -> PlagesProfil {
    mesure_synthetique(9, 100, 11)
}

#[test]
fn une_mesure_saine_de_840_plages_passe_sans_alerte() {
    let mesure = mesure_saine();
    assert_eq!(mesure.plages.len(), 840);
    let controle = controler_coherence(&mesure);
    assert!(controle.accepte(), "{:?}", controle.suspectes);
    assert_eq!(
        controle.faits,
        [
            Critere::DegradeGris,
            Critere::BlancNoir,
            Critere::Doublons,
            Critere::Voisines
        ]
    );
    assert!(controle.non_faits.is_empty());
    assert_eq!(controle.decalage, None);
}

/// Les Lab décalés d'une plage : le Lab de la plage i est donné à la plage
/// i + 1, celui de la dernière revient à la première.
fn decaler(mesure: &PlagesProfil, n: usize) -> PlagesProfil {
    let mut decalee = mesure.clone();
    let total = mesure.plages.len();
    for (i, plage) in decalee.plages.iter_mut().enumerate() {
        plage.lab = mesure.plages[(i + total - n) % total].lab;
    }
    decalee
}

#[test]
fn des_mesures_decalees_d_une_plage_sont_refusees_et_le_decalage_est_dit() {
    let controle = controler_coherence(&decaler(&mesure_saine(), 1));
    assert!(!controle.accepte());
    assert!(controle.suspectes.len() > 10, "{:?}", controle.suspectes);
    assert_eq!(controle.decalage, Some(1));
}

/// Rang (à partir de 1) de la première plage de ce RVB.
fn rang(mesure: &PlagesProfil, rvb: [f64; 3]) -> usize {
    mesure.plages.iter().position(|p| p.rvb == rvb).unwrap() + 1
}

/// Rangs signalés par un contrôle.
fn rangs(controle: &profilage::ControleCoherence, critere: Critere) -> Vec<usize> {
    controle
        .suspectes
        .iter()
        .filter(|s| s.critere == critere)
        .map(|s| s.rang)
        .collect()
}

#[test]
fn des_mesures_melangees_sont_refusees_avec_les_plages_en_cause() {
    let mut mesure = mesure_saine();
    let mut labs: Vec<[f64; 3]> = mesure.plages.iter().map(|p| p.lab).collect();
    Hasard(42).melanger(&mut labs);
    for (plage, lab) in mesure.plages.iter_mut().zip(labs) {
        plage.lab = lab;
    }
    let controle = controler_coherence(&mesure);
    assert!(!controle.accepte());
    assert_eq!(controle.decalage, None);
    assert!(
        controle.suspectes.len() > 100,
        "{}",
        controle.suspectes.len()
    );
    assert!(controle
        .suspectes
        .windows(2)
        .all(|p| p[0].rang <= p[1].rang));
}

#[test]
fn un_gris_qui_recule_est_signale() {
    let mut mesure = mesure_saine();
    let quart = rang(&mesure, [25.0; 3]);
    let moitie = rang(&mesure, [50.0; 3]);
    mesure.plages[moitie - 1].lab[0] = mesure.plages[quart - 1].lab[0];
    let controle = controler_coherence(&mesure);
    assert!(!controle.accepte());
    // Le gris 50 % recule par rapport au gris 37,5 % qui le précède.
    let signales = rangs(&controle, Critere::DegradeGris);
    assert!(signales.contains(&moitie), "{signales:?}");
    assert!(signales.contains(&rang(&mesure, [37.5; 3])), "{signales:?}");
}

/// Cas du ticket : la plage RVB 0 a la clarté d'un gris moyen.
#[test]
fn un_blanc_papier_trop_sombre_est_signale() {
    let mut mesure = mesure_saine();
    let noir = rang(&mesure, [0.0; 3]);
    let gris = rang(&mesure, [50.0; 3]);
    mesure.plages[noir - 1].lab = mesure.plages[gris - 1].lab;
    let blanc = rang(&mesure, [100.0; 3]);
    mesure.plages[blanc - 1].lab[0] -= 10.0;
    let controle = controler_coherence(&mesure);
    assert!(!controle.accepte());
    let signales = rangs(&controle, Critere::BlancNoir);
    assert!(signales.contains(&noir), "{signales:?}");
    assert!(signales.contains(&blanc), "{signales:?}");
}

#[test]
fn des_doublons_trop_differents_sont_signales() {
    let mut mesure = mesure_saine();
    // Le premier doublon du fichier, et sa première plage de même RVB.
    let second = (1..mesure.plages.len())
        .find(|&i| {
            mesure.plages[..i]
                .iter()
                .any(|p| p.rvb == mesure.plages[i].rvb)
        })
        .unwrap();
    let premier = rang(&mesure, mesure.plages[second].rvb) - 1;
    let doublon = [premier, second];
    mesure.plages[second].lab[0] += 8.0;
    let controle = controler_coherence(&mesure);
    assert!(!controle.accepte());
    let mut attendus: Vec<usize> = doublon.iter().map(|i| i + 1).collect();
    attendus.sort();
    assert_eq!(rangs(&controle, Critere::Doublons), attendus);
}

#[test]
fn une_plage_isolee_est_signalee_par_son_rang() {
    let mut mesure = mesure_saine();
    let isolee = rang(&mesure, [37.5, 62.5, 25.0]);
    let lab = &mut mesure.plages[isolee - 1].lab;
    lab[0] += 30.0;
    lab[1] += 30.0;
    let controle = controler_coherence(&mesure);
    assert!(!controle.accepte());
    let suspectes: Vec<_> = controle
        .suspectes
        .iter()
        .map(|s| (s.rang, s.critere))
        .collect();
    assert_eq!(suspectes, [(isolee, Critere::Voisines)]);
    assert!(controle.suspectes[0].ecart > 20.0);
}

#[test]
fn sans_gris_le_critere_est_dit_non_fait() {
    let mut mesure = mesure_saine();
    mesure.plages.retain(|p| {
        let [r, g, b] = p.rvb;
        r.max(g).max(b) - r.min(g).min(b) >= 0.5
    });
    let controle = controler_coherence(&mesure);
    assert!(controle.accepte(), "{:?}", controle.suspectes);
    // Sans gris, ni blanc ni noir non plus.
    assert_eq!(
        controle.non_faits,
        [Critere::DegradeGris, Critere::BlancNoir]
    );
    assert_eq!(controle.faits, [Critere::Doublons, Critere::Voisines]);
}

#[test]
fn le_controle_de_2520_plages_tient_en_2_secondes() {
    // Grille 13 × 13 × 13, 300 plages au hasard, 23 doublons.
    let saine = mesure_synthetique(13, 300, 23);
    assert_eq!(saine.plages.len(), 2520);
    let decalee = decaler(&saine, 60);
    let debut = std::time::Instant::now();
    assert!(controler_coherence(&saine).accepte());
    assert_eq!(controler_coherence(&decalee).decalage, Some(60));
    let duree = debut.elapsed();
    assert!(duree.as_secs_f64() < 2.0, "{duree:?}");
}

/// La mesure réelle du 26 octobre 2026 (export MYIROtools de 2 520 plages,
/// FD-9, M1) passe sans alerte. Le fichier reste sur le poste, jamais
/// versionné : il porte un numéro de série réel. Son chemin est donné par la
/// variable `MESURE_REELLE`.
#[test]
#[ignore = "lit la mesure réelle locale (variable MESURE_REELLE), non versionnée"]
fn la_mesure_reelle_du_26_octobre_passe() {
    let chemin = std::env::var_os("MESURE_REELLE")
        .expect("donner le chemin de l'export dans la variable MESURE_REELLE");
    let texte = cgats::decoder(&std::fs::read(&chemin).unwrap());
    let mesure = cgats::lire(&texte).unwrap();
    let plages = profilage::preparer_plages(&mesure, ConditionMesure::M1).unwrap();
    assert_eq!(plages.plages.len(), 2520);
    let controle = controler_coherence(&plages);
    println!("faits : {:?}", controle.faits);
    println!("non faits : {:?}", controle.non_faits);
    for s in &controle.suspectes {
        println!("plage {} : {:?}, écart {:.2}", s.rang, s.critere, s.ecart);
    }
    assert!(controle.accepte(), "décalage : {:?}", controle.decalage);
}
