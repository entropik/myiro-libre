//! Le numéro de série d'un instrument ne part vers aucune vue de la page
//! (ticket #84) : ni la vue d'une résolution, ni la feuille des mesures. Seul
//! le modèle reconnu est montré ; le fichier d'origine rangé reste intact.
//! Méthodes de `BibliothequeOuverte`, sans Tauri ; numéros fictifs.

use std::path::Path;

use app::colonne::{emplacement, BibliothequeOuverte};
use pont_protocole::Info;

const SERIE: &str = "12345678";

/// Export fictif à la manière de FD-S2w : la ligne `INSTRUMENTATION` et
/// `SERIAL` portent le numéro de série fictif.
fn export_fd_s2w(instrumentation: &str) -> String {
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n\
         INSTRUMENTATION\t\"{instrumentation}\"\r\nSERIAL\t\"{SERIE}\"\r\n\
         MEASUREMENT_SOURCE\t\"D50\"\r\nBEGIN_DATA_FORMAT\r\n\
         SAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t3\r\nBEGIN_DATA\r\n\
         1\t255\t255\t255\t95\t0\t2\r\n\
         2\t0\t117.61\t199.05\t42.09\t-28\t-60.57\r\n\
         3\t0\t0\t0\t3.1\t0.2\t-0.4\r\nEND_DATA\r\n"
    )
}

/// Ce que la page recevrait, en JSON.
fn json(vue: &impl serde::Serialize) -> String {
    serde_json::to_string(vue).unwrap()
}

fn sans_numero(vue: &str, nom: &str) {
    assert!(!vue.contains(SERIE), "{nom} : {vue}");
    assert!(!vue.contains("numero_serie"), "{nom} : {vue}");
}

#[test]
fn une_mesure_importee_n_envoie_son_numero_a_aucune_vue() {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "HMPhoto Luster 290", "1200 ppp")
        .unwrap()
        .id;
    let texte = export_fd_s2w("KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678");
    let chemin = dossier.path().join("mesure00001_M1.txt");
    std::fs::write(&chemin, &texte).unwrap();

    // Aperçu de l'import (feuille des mesures).
    let apercu = ouverte.apercu_cgats(&chemin).unwrap();
    assert_eq!(apercu.contenu.instrument, Info::Confirmee("FD-9".into()));
    sans_numero(&json(&apercu), "aperçu");

    // Mesure rangée : cartouche de la feuille, colonne de gauche, résolution.
    let id = ouverte.ranger_apercu(apercu.numero, condition).unwrap();
    let detail = ouverte.detail_importee(id).unwrap();
    assert_eq!(detail.contenu.instrument, Info::Confirmee("FD-9".into()));
    sans_numero(&json(&detail), "mesure importée");
    let branches = ouverte.arborescence("").unwrap();
    assert_eq!(
        branches[0].importees[0].instrument,
        Info::Confirmee("FD-9".into())
    );
    sans_numero(&json(&branches), "colonne de gauche");
    sans_numero(
        &json(&ouverte.detail_reglage(condition).unwrap()),
        "résolution",
    );

    // Le fichier d'origine rangé reste celui de FD-S2w, numéro compris :
    // la recherche le retrouve encore par son numéro.
    let trouvees = ouverte.arborescence(SERIE).unwrap();
    assert_eq!(trouvees[0].importees.len(), 1);
}

#[test]
fn une_ligne_sans_modele_connu_donne_un_instrument_inconnu_dans_la_feuille() {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let chemin = dossier.path().join("releve.txt");
    std::fs::write(
        &chemin,
        export_fd_s2w("Spectrophotomètre fictif, Serial No. 12345678"),
    )
    .unwrap();

    let apercu = ouverte.apercu_cgats(&chemin).unwrap();
    assert_eq!(apercu.contenu.instrument, Info::Inconnue);
    let vue = json(&apercu);
    sans_numero(&vue, "aperçu");
    assert!(!vue.contains("Spectrophotomètre fictif"), "{vue}");
}

/// Les mesures d'un pont de la démonstration (n° fictif 12345678) : la
/// colonne de gauche et le cartouche montrent le modèle, pas le numéro.
#[test]
fn une_mesure_d_un_pont_n_envoie_pas_son_numero_a_la_feuille() {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::demonstration(Path::new(dossier.path()));
    let branches = ouverte.arborescence("").unwrap();
    let mesure = branches[0].mesures[0].id;
    let vue = json(&branches);
    sans_numero(&vue, "colonne de gauche");
    assert!(vue.contains("\"modele\":\"MYIRO-1\""), "{vue}");
    let detail = json(&ouverte.detail_mesure(mesure).unwrap());
    sans_numero(&detail, "cartouche d'une mesure");
    assert!(detail.contains("\"modele\":\"MYIRO-1\""), "{detail}");
}
