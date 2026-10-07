//! Seam 2 : `comparer(&JeuValidation, &impl CalculSpectres) -> Rapport`, sur les
//! données synthétiques versionnées de `tests/donnees/mesures-synthetiques.csv`.
//!
//! Le jeu synthétique a deux plages :
//! - A : brutes[0] = 0,5 ; M0 = 0,5 partout ; M1 = 0,6 partout ; M2 = 0,9 à 380 nm, 0,5 ailleurs ;
//! - B : brutes[0] = 0,2 ; M0 = M1 = M2 = 0,2 partout.

use jeu_validation::{comparer, extraire, CalculSpectres, Condition, SortieArchivee, Spectres};

/// Candidat synthétique : chaque spectre vaut brutes[0] à toutes les longueurs d'onde.
struct Plat;

impl CalculSpectres for Plat {
    fn calculer(&self, brutes: &[f32]) -> Result<Spectres, String> {
        let v = vec![brutes[0]; 36];
        Ok([v.clone(), v.clone(), v])
    }
}

/// Candidat qui ne sait encore rien calculer.
struct Muet;

impl CalculSpectres for Muet {
    fn calculer(&self, _: &[f32]) -> Result<Spectres, String> {
        Err("pas encore écrit".into())
    }
}

fn jeu() -> jeu_validation::JeuValidation {
    extraire(&[SortieArchivee {
        nom: "synthetique",
        contenu: include_str!("donnees/mesures-synthetiques.csv"),
    }])
    .unwrap()
}

fn proche(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn un_calcul_identique_a_la_dll_a_un_ecart_nul() {
    let rapport = comparer(&jeu(), &Plat);
    let m0 = rapport.condition(Condition::M0);
    assert_eq!(m0.paires, 2);
    assert_eq!(m0.ecart_maximal.unwrap(), 0.0);
    assert_eq!(m0.ecart_moyen.unwrap(), 0.0);
    assert!(m0
        .par_longueur_onde
        .iter()
        .all(|l| l.ecart_maximal == Some(0.0)));
}

#[test]
fn un_ecart_constant_se_lit_a_chaque_longueur_d_onde() {
    let rapport = comparer(&jeu(), &Plat);
    let m1 = rapport.condition(Condition::M1);
    // Plage A : |0,5 - 0,6| = 0,1 partout ; plage B : 0.
    assert!(proche(m1.ecart_maximal.unwrap(), 0.1));
    assert!(proche(m1.ecart_moyen.unwrap(), 0.05));
    assert_eq!(m1.par_longueur_onde.len(), 36);
    assert_eq!(m1.par_longueur_onde[35].longueur_onde, 730);
    assert!(proche(m1.par_longueur_onde[35].ecart_moyen.unwrap(), 0.05));
    assert!(proche(m1.par_longueur_onde[35].ecart_maximal.unwrap(), 0.1));
}

#[test]
fn un_ecart_localise_est_attribue_a_sa_longueur_d_onde() {
    let rapport = comparer(&jeu(), &Plat);
    let m2 = rapport.condition(Condition::M2);
    // Plage A : |0,5 - 0,9| = 0,4 à 380 nm seulement.
    let l380 = &m2.par_longueur_onde[0];
    assert_eq!(l380.longueur_onde, 380);
    assert!(proche(l380.ecart_maximal.unwrap(), 0.4));
    assert!(proche(l380.ecart_moyen.unwrap(), 0.2));
    assert_eq!(m2.par_longueur_onde[1].ecart_maximal.unwrap(), 0.0);
    assert!(proche(m2.ecart_maximal.unwrap(), 0.4));
    assert!(proche(m2.ecart_moyen.unwrap(), 0.4 / 72.0));
}

#[test]
fn les_echecs_du_candidat_sont_comptes_sans_fausser_les_ecarts() {
    let rapport = comparer(&jeu(), &Muet);
    assert_eq!(rapport.echecs.len(), 2);
    assert_eq!(rapport.echecs[0].plage, "A");
    assert_eq!(rapport.echecs[0].detail, "pas encore écrit");
    let m0 = rapport.condition(Condition::M0);
    assert_eq!(m0.paires, 0);
    // Sans paire comparée, l'écart est inconnu, jamais mis à zéro.
    assert_eq!(m0.ecart_moyen, None);
    assert_eq!(m0.par_longueur_onde[0].ecart_maximal, None);
}

/// Candidat qui rend des spectres trop courts.
struct Court;

impl CalculSpectres for Court {
    fn calculer(&self, _: &[f32]) -> Result<Spectres, String> {
        Ok([vec![0.0; 35], vec![0.0; 36], vec![0.0; 36]])
    }
}

#[test]
fn un_spectre_de_mauvaise_longueur_est_un_echec_du_candidat() {
    let rapport = comparer(&jeu(), &Court);
    assert_eq!(rapport.echecs.len(), 2);
    assert_eq!(rapport.condition(Condition::M1).paires, 0);
}

/// Candidat qui rend NaN à 550 nm en M1, et des valeurs exactes partout ailleurs.
struct Troue;

impl CalculSpectres for Troue {
    fn calculer(&self, brutes: &[f32]) -> Result<Spectres, String> {
        let [m0, mut m1, m2] = Plat.calculer(brutes)?;
        m1[17] = f32::NAN;
        Ok([m0, m1, m2])
    }
}

#[test]
fn une_valeur_non_finie_du_candidat_est_un_echec_jamais_un_ecart() {
    let rapport = comparer(&jeu(), &Troue);
    assert_eq!(rapport.echecs.len(), 2);
    assert!(
        rapport.echecs[0].detail.contains("M1"),
        "{}",
        rapport.echecs[0].detail
    );
    let m1 = rapport.condition(Condition::M1);
    assert_eq!(m1.paires, 0);
    assert_eq!(m1.par_longueur_onde[17].ecart_maximal, None);
}

#[test]
fn une_paire_courte_du_jeu_est_un_echec_jamais_un_ecart() {
    // Un jeu relu depuis un fichier peut avoir été abîmé.
    let mut jeu = jeu();
    jeu.paires[0].spectres[2].truncate(1);
    let rapport = comparer(&jeu, &Plat);
    assert_eq!(rapport.echecs.len(), 1);
    assert_eq!(rapport.echecs[0].plage, "A");
    assert_eq!(rapport.condition(Condition::M2).paires, 1);
    // Seule B est comparée : son M2 est exact, l'écart de A à 380 nm n'apparaît pas.
    assert_eq!(rapport.condition(Condition::M2).ecart_maximal, Some(0.0));
}

#[test]
fn des_donnees_brutes_de_mauvaise_longueur_dans_le_jeu_sont_un_echec() {
    let mut jeu = jeu();
    jeu.paires[1].brutes.pop();
    let rapport = comparer(&jeu, &Plat);
    assert_eq!(rapport.echecs.len(), 1);
    assert_eq!(rapport.echecs[0].plage, "B");
}

#[test]
fn une_valeur_non_finie_dans_le_jeu_est_un_echec() {
    let mut jeu = jeu();
    jeu.paires[1].spectres[0][3] = f32::INFINITY;
    let rapport = comparer(&jeu, &Plat);
    assert_eq!(rapport.echecs.len(), 1);
    assert_eq!(rapport.echecs[0].plage, "B");
}
