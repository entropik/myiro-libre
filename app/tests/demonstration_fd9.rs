//! FD-9 de démonstration (`cargo run -p app -- --demo`) : de quoi voir
//! l'écran « Mesurer une feuille au FD-9 » sans FD-9 ni DLL. Le pont simulé
//! écrit l'avancement d'un vrai pont, avec un calage et une image
//! synthétiques, puis une mesure fictive, ou l'erreur choisie
//! (`--demo-feuille <issue>`). Rien ici ne vient d'un instrument réel.

use std::time::Duration;

use app::colonne::{emplacement, BibliothequeOuverte};
use app::demonstration_fd9::{self, IssueDemonstration};
use app::instrument::feuille::{definition_rgb2520, EtapeFeuille};
use app::instrument::Etat;
use app::mesurer_feuille::lire_calage;

fn banc() -> (
    tempfile::TempDir,
    BibliothequeOuverte,
    bibliotheque::IdCondition,
) {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    (dossier, ouverte, condition)
}

#[test]
fn la_mire_de_demonstration_a_la_forme_de_la_rgb2520() {
    let mire = demonstration_fd9::mire();
    let definition = definition_rgb2520(&mire).unwrap();
    assert_eq!(definition.plages(), 2_520);
    assert_eq!(definition.couleurs.as_ref().unwrap().len(), 2_520);
}

#[test]
fn le_fd9_de_demonstration_est_detecte_sans_dll() {
    let instrument = demonstration_fd9::instrument(IssueDemonstration::Mesure, Duration::ZERO);
    assert!(matches!(instrument.etat(), Etat::Detecte { modele, .. } if modele == "FD-9"));
    assert!(instrument.feuille_possible());
}

#[test]
fn une_feuille_de_demonstration_suit_toutes_les_etapes_et_se_range() {
    let (dossier, ouverte, condition) = banc();
    let mut instrument = demonstration_fd9::instrument(IssueDemonstration::Mesure, Duration::ZERO);
    let mut etapes = Vec::new();
    let rangee = ouverte
        .mesurer_feuille_fd9(
            condition,
            &mut instrument,
            Ok(demonstration_fd9::mire()),
            &emplacement(dossier.path()),
            &mut demonstration_fd9::lanceur(IssueDemonstration::Mesure, Duration::ZERO),
            &mut |e| etapes.push(e),
        )
        .unwrap();
    assert_eq!(rangee.plages, 2_520);
    let noms: Vec<&str> = etapes
        .iter()
        .map(|e| match e {
            EtapeFeuille::Connexion => "connexion",
            EtapeFeuille::GlisserFeuille => "glisser",
            EtapeFeuille::FeuilleEngagee => "engagee",
            EtapeFeuille::Lecture => "lecture",
            EtapeFeuille::CalagePret { .. } => "calage",
            EtapeFeuille::Mesure { .. } => "mesure",
            EtapeFeuille::Rangement => "rangement",
        })
        .collect();
    let mut attendu = vec!["connexion", "glisser", "engagee", "lecture", "calage"];
    attendu.extend(std::iter::repeat_n("mesure", noms.len() - 6));
    attendu.push("rangement");
    assert_eq!(noms, attendu);
    assert!(matches!(
        etapes[etapes.len() - 2],
        EtapeFeuille::Mesure {
            mesurees: 2_520,
            plages: 2_520
        }
    ));
    // Calage synthétique : un point par plage, image A3 à 60 ppp.
    let EtapeFeuille::CalagePret { calage } = &etapes[4] else {
        panic!("{:?}", etapes[4])
    };
    assert_eq!(calage.points.len(), 2_520);
    let image = calage.image.as_ref().unwrap();
    assert_eq!(
        (image.largeur(), image.hauteur(), image.ppp()),
        (660, 984, 60)
    );
    assert_eq!(
        lire_calage(&emplacement(dossier.path()), rangee.mesure).as_ref(),
        Some(calage)
    );
    // Le FD-9 de démonstration est de nouveau prêt.
    assert!(instrument.feuille_possible());
}

#[test]
fn une_issue_choisie_rend_son_erreur() {
    for (argument, issue, cle) in [
        (
            "feuille_absente",
            IssueDemonstration::FeuilleAbsente,
            "feuille_fd9.feuille_absente",
        ),
        (
            "autre_mire",
            IssueDemonstration::AutreMire,
            "feuille_fd9.autre_mire",
        ),
        (
            "mire_non_reconnue",
            IssueDemonstration::MireNonReconnue,
            "feuille_fd9.mire_non_reconnue",
        ),
        (
            "bourrage",
            IssueDemonstration::Bourrage,
            "feuille_fd9.incident_feuille.102",
        ),
        (
            "mesure_invraisemblable",
            IssueDemonstration::MesureInvraisemblable,
            "feuille_fd9.mesure_invraisemblable",
        ),
    ] {
        assert_eq!(IssueDemonstration::depuis_argument(argument), Some(issue));
        let (dossier, ouverte, condition) = banc();
        let mut instrument = demonstration_fd9::instrument(issue, Duration::ZERO);
        let echec = ouverte
            .mesurer_feuille_fd9(
                condition,
                &mut instrument,
                Ok(demonstration_fd9::mire()),
                &emplacement(dossier.path()),
                &mut demonstration_fd9::lanceur(issue, Duration::ZERO),
                &mut |_| {},
            )
            .unwrap_err();
        assert_eq!(echec.cle, cle, "{argument}");
    }
    assert_eq!(IssueDemonstration::depuis_argument("inconnue"), None);
}

#[test]
fn une_feuille_de_demonstration_s_annule_avant_la_mesure() {
    let (dossier, ouverte, condition) = banc();
    let rythme = Duration::from_millis(200);
    let mut instrument = demonstration_fd9::instrument(IssueDemonstration::Mesure, rythme);
    let annulation = instrument.annulation();
    let mut etapes = Vec::new();
    let echec = ouverte
        .mesurer_feuille_fd9(
            condition,
            &mut instrument,
            Ok(demonstration_fd9::mire()),
            &emplacement(dossier.path()),
            &mut demonstration_fd9::lanceur(IssueDemonstration::Mesure, rythme),
            &mut |e| {
                if e == EtapeFeuille::FeuilleEngagee {
                    assert!(annulation.annuler());
                }
                etapes.push(e);
            },
        )
        .unwrap_err();
    assert_eq!(echec.cle, "feuille_fd9.mesure_annulee");
    assert!(!etapes
        .iter()
        .any(|e| matches!(e, EtapeFeuille::CalagePret { .. })));
}
