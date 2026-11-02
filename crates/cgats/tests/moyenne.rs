//! Écriture d'une mesure moyennée (ticket #35) : relue par `lire`, elle
//! rend la même mesure, RVB de la mire confirmés, sans provenance de pont.

use cgats::{ecrire_moyenne, lire, MesureImportee, PlageImportee};
use pont_protocole::{ConditionMesure, Echantillonnage, Info, Lab, Spectre};

fn spectre(base: f32) -> Spectre {
    Spectre::new((0..36).map(|i| base + i as f32 * 0.001).collect()).unwrap()
}

fn plage(identifiant: &str, rvb: [f32; 3], base: f32) -> PlageImportee {
    PlageImportee {
        identifiant: identifiant.into(),
        spectres: [
            Info::Inconnue,
            Info::Confirmee(spectre(base)),
            Info::Inconnue,
        ],
        lab: [
            Info::Inconnue,
            Info::Confirmee(Lab::new([base * 100.0, 1.25, -3.5]).unwrap()),
            Info::Inconnue,
        ],
        rvb: Info::Confirmee(rvb),
    }
}

fn moyenne(condition: Info<ConditionMesure>) -> MesureImportee {
    MesureImportee {
        provenance: Info::Inconnue,
        instrument: Info::Confirmee("FD-9".into()),
        numero_serie: Info::Inconnue,
        date: Info::Confirmee("2026-11-02T10:00:00Z".into()),
        conditions: [Info::Inconnue, condition, Info::Inconnue],
        longueurs_onde: Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10,
        }),
        plages: vec![
            plage("1", [1.0, 1.0, 1.0], 0.9),
            plage("2", [0.0, 0.5, 0.25], 0.3),
        ],
    }
}

#[test]
fn une_moyenne_relue_rend_la_meme_mesure() {
    let m = moyenne(Info::Confirmee(ConditionMesure::M1));
    let texte = ecrire_moyenne(&m, "empreinte-fictive", &["12", "15"]).unwrap();
    assert_eq!(lire(&texte).unwrap(), m);
}

#[test]
fn une_condition_supposee_reste_supposee() {
    let m = moyenne(Info::Supposee(ConditionMesure::M1));
    let texte = ecrire_moyenne(&m, "empreinte-fictive", &["12"]).unwrap();
    assert_eq!(lire(&texte).unwrap(), m);
}

#[test]
fn la_mire_et_les_sources_sont_dans_l_en_tete() {
    let m = moyenne(Info::Confirmee(ConditionMesure::M1));
    let texte = ecrire_moyenne(&m, "empreinte-fictive", &["12", "15"]).unwrap();
    assert!(texte.contains("MYIRO_LIBRE_FORMAT\t\"myiro-libre/moyenne/1\""));
    assert!(texte.contains("MYIRO_LIBRE_MIRE\t\"empreinte-fictive\""));
    assert!(texte.contains("MYIRO_LIBRE_SOURCES\t\"12 15\""));
    assert!(!texte.contains("SERIAL"));
}

#[test]
fn l_instrument_ecrit_est_le_seul_modele_jamais_le_numero_de_serie() {
    let mut m = moyenne(Info::Confirmee(ConditionMesure::M1));
    m.instrument =
        Info::Confirmee("KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678".into());
    let texte = ecrire_moyenne(&m, "empreinte-fictive", &["12"]).unwrap();
    assert!(texte.contains("INSTRUMENTATION\t\"FD-9\""), "{texte}");
    assert!(!texte.contains("12345678"));
    assert_eq!(
        lire(&texte).unwrap().instrument,
        Info::Confirmee("FD-9".into())
    );
}

#[test]
fn un_instrument_sans_modele_connu_n_est_pas_ecrit() {
    let mut m = moyenne(Info::Confirmee(ConditionMesure::M1));
    m.instrument = Info::Confirmee("Appareil fictif n° 12345678".into());
    let texte = ecrire_moyenne(&m, "empreinte-fictive", &["12"]).unwrap();
    assert!(!texte.contains("INSTRUMENTATION"));
    assert!(!texte.contains("12345678"));
}

#[test]
fn une_moyenne_sans_rvb_est_refusee() {
    let mut m = moyenne(Info::Confirmee(ConditionMesure::M1));
    m.plages[1].rvb = Info::Inconnue;
    let e = ecrire_moyenne(&m, "empreinte-fictive", &["12"]).unwrap_err();
    assert!(e.0.contains("RVB"), "{e}");
}

#[test]
fn des_rvb_supposes_sont_refuses() {
    // Les RVB d'une moyenne sont ceux de la mire : jamais supposés.
    let mut m = moyenne(Info::Confirmee(ConditionMesure::M1));
    m.plages[0].rvb = Info::Supposee([1.0, 1.0, 1.0]);
    assert!(ecrire_moyenne(&m, "empreinte-fictive", &["12"]).is_err());
}

#[test]
fn un_spectre_manquant_sur_une_seule_plage_est_refuse() {
    let mut m = moyenne(Info::Confirmee(ConditionMesure::M1));
    m.plages[1].spectres[1] = Info::Inconnue;
    let e = ecrire_moyenne(&m, "empreinte-fictive", &["12"]).unwrap_err();
    assert!(e.0.contains("spectre"), "{e}");
}
