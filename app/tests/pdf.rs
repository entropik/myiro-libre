//! Écriture d'un rapport PDF à sa place (ticket #89) : jamais à moitié
//! écrit, un PDF existant remplacé seulement après succès, aucun temporaire
//! laissé derrière.

use app::pdf::{enregistrer_pdf, fabriquer_pdf, ErreurPdf};

const PDF: &[u8] = b"%PDF-1.7 essai\n%%EOF\n";

#[test]
fn un_pdf_non_ecrit_laisse_rien_derriere() {
    let dossier = tempfile::tempdir().unwrap();
    let sortie = dossier.path().join("absent").join("Rapport.pdf");
    assert!(matches!(
        enregistrer_pdf(PDF, &sortie),
        Err(ErreurPdf::Destination(_))
    ));
    assert!(!sortie.exists());
    assert_eq!(std::fs::read_dir(dossier.path()).unwrap().count(), 0);
    assert_eq!(
        ErreurPdf::Destination(String::new()).cle(),
        "rapport.erreur.destination"
    );
}

#[test]
fn le_pdf_est_ecrit_a_cote_puis_renomme() {
    let dossier = tempfile::tempdir().unwrap();
    let sortie = dossier.path().join("Rapport 2026-0001.pdf");
    std::fs::write(&sortie, b"ancien").unwrap();
    // Le temporaire ne peut pas être écrit : l'ancien PDF reste intact.
    let tmp = dossier.path().join("Rapport 2026-0001.pdf.tmp");
    std::fs::create_dir(&tmp).unwrap();
    assert!(enregistrer_pdf(PDF, &sortie).is_err());
    assert_eq!(std::fs::read(&sortie).unwrap(), b"ancien");
    std::fs::remove_dir(&tmp).unwrap();
    // Sinon il est remplacé, sans temporaire restant.
    enregistrer_pdf(PDF, &sortie).unwrap();
    assert_eq!(std::fs::read(&sortie).unwrap(), PDF);
    assert_eq!(std::fs::read_dir(dossier.path()).unwrap().count(), 1);
}

#[test]
fn un_document_devient_un_pdf_ou_une_erreur_du_moteur() {
    let document = rapport::Document {
        langue: "fr".into(),
        ..Default::default()
    };
    assert!(fabriquer_pdf(&document).unwrap().starts_with(b"%PDF-"));
    let illisible = rapport::Document {
        sections: vec![rapport::Section {
            titre: "x".into(),
            blocs: vec![rapport::Bloc::Images(vec![rapport::Figure {
                png: b"pas une image".to_vec(),
                legende: String::new(),
                largeur_mm: 50.0,
            }])],
        }],
        ..document
    };
    let erreur = fabriquer_pdf(&illisible).unwrap_err();
    assert!(matches!(erreur, ErreurPdf::Moteur(_)));
    assert_eq!(erreur.cle(), "rapport.erreur.moteur");
}
