//! Modèle d'instrument tiré d'une ligne `INSTRUMENTATION` : seul un modèle
//! connu est gardé, jamais le reste de la ligne (numéro de série). Numéros
//! fictifs.

use cgats::modele_instrument;

#[test]
fn un_export_fd_s2w_ne_garde_que_le_modele() {
    assert_eq!(
        modele_instrument("KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678"),
        Some("FD-9".to_string())
    );
}

#[test]
fn myiro_9_seul_est_un_fd_9() {
    assert_eq!(
        modele_instrument("MYIRO-9 Serial No. 12345678"),
        Some("FD-9".to_string())
    );
}

#[test]
fn les_autres_modeles_connus_sont_reconnus_sans_le_reste() {
    for (ligne, modele) in [
        ("KONICA MINOLTA MYIRO-1 Serial No. 12345678", "MYIRO-1"),
        ("Konica Minolta FD-7 #12345678", "FD-7"),
        ("konica minolta fd-5 12345678", "FD-5"),
    ] {
        assert_eq!(
            modele_instrument(ligne),
            Some(modele.to_string()),
            "{ligne}"
        );
    }
}

#[test]
fn une_ligne_sans_modele_connu_ne_donne_rien() {
    assert_eq!(
        modele_instrument("Spectrophotomètre fictif, Serial No. 12345678"),
        None
    );
    assert_eq!(modele_instrument(""), None);
}
