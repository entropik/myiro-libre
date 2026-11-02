//! Catalogue des textes de la séance de mesure (ticket #35) : chaque clé
//! que le Rust peut rendre existe en français et en anglais, refus du
//! moyennage compris, et les erreurs de séance de la bibliothèque ont la
//! leur.

use app::colonne::cle_erreur;
use app::seance_fd9::{cles_seance, RefusSeance};
use app::textes::{texte, Langue};
use bibliotheque::{ErreurBibliotheque, IdSeance};
use seance_mesure::Incompatible;

const LANGUES: [Langue; 2] = [Langue::Francais, Langue::Anglais];

fn present(cle: &str) {
    for langue in LANGUES {
        let lu = texte(langue, cle);
        assert_ne!(lu, cle, "{cle} manque en {}", langue.code());
        assert!(!lu.trim().is_empty(), "{cle} vide en {}", langue.code());
    }
}

#[test]
fn chaque_cle_de_la_seance_a_son_texte_dans_les_deux_langues() {
    let cles = cles_seance();
    assert!(cles.len() > 20, "{cles:?}");
    for cle in &cles {
        present(cle);
    }
}

#[test]
fn chaque_regle_du_moyennage_a_son_texte() {
    let exemples = [
        Incompatible::AucunExemplaire,
        Incompatible::AutreMire { source: 1 },
        Incompatible::AssociationIncertaine { source: 1 },
        Incompatible::ModeleInconnu { source: 1 },
        Incompatible::ModelesDifferents { source: 1 },
        Incompatible::InstrumentsDifferents { source: 1 },
        Incompatible::ConditionsDifferentes { source: 1 },
        Incompatible::AucuneConditionCommune,
        Incompatible::LongueursOndeDifferentes { source: 1 },
        Incompatible::GeometriesDifferentes { source: 1 },
        Incompatible::SpectresMeles {
            emplacement: "m1".into(),
        },
        Incompatible::LabMeles {
            emplacement: "m1".into(),
        },
        Incompatible::ComplementInvalide { plage: "3".into() },
    ];
    let cles = cles_seance();
    for exemple in exemples {
        let cle = RefusSeance::from(exemple).cle;
        assert!(cles.contains(&cle), "{cle} absente de cles_seance");
        present(&cle);
    }
}

#[test]
fn une_seance_fermee_ou_inconnue_a_sa_propre_cle() {
    let fermee = cle_erreur(&ErreurBibliotheque::SeanceFermee(IdSeance(1)));
    let inconnue = cle_erreur(&ErreurBibliotheque::SeanceInconnue(IdSeance(1)));
    assert_eq!(fermee, "seance.erreur.fermee");
    assert_eq!(inconnue, "seance.erreur.inconnue");
    present(fermee);
    present(inconnue);
}
