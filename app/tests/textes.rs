//! Catalogue des textes de l'écran « Mesurer une feuille au FD-9 » (ticket
//! #34, tâche 9) : chaque refus d'une feuille a sa cause et son action dans
//! les deux langues, chaque étape annoncée son texte, et la page ne demande
//! que des clés du catalogue.

use std::path::Path;

use app::instrument::feuille::{ErreurFeuille, ETATS_INCIDENT};
use app::mesurer_feuille::{RAISON_OCCUPE, RAISON_SANS_FD9};
use app::textes::{cles, texte, Langue};
use pont_protocole::{DefautPositions, ErreurPont, Palier, RemiseAuRepos};

const LANGUES: [Langue; 2] = [Langue::Francais, Langue::Anglais];

fn present(cle: &str) {
    for langue in LANGUES {
        let lu = texte(langue, cle);
        assert_ne!(lu, cle, "{cle} manque en {}", langue.code());
        assert!(!lu.trim().is_empty(), "{cle} vide en {}", langue.code());
    }
}

/// Toutes les erreurs que le pont peut rendre, une par type du protocole.
fn toutes_les_erreurs() -> Vec<ErreurPont> {
    let repos = RemiseAuRepos::AuRepos {};
    vec![
        ErreurPont::EtatInvalide {
            attendu: Palier::Connexion,
        },
        ErreurPont::PalierNonAutorise {
            demande: Palier::Feuille,
            plafond: Palier::Connexion,
        },
        ErreurPont::InstrumentInconnu {},
        ErreurPont::ParametreRefuse {},
        ErreurPont::EtatIncompatible {},
        ErreurPont::EtalonnageEchoue { erreur: 1 },
        ErreurPont::NonEtalonne {},
        ErreurPont::EtalonnageRequis {},
        ErreurPont::SessionInexploitable {},
        ErreurPont::MesureEchouee { erreur: 1 },
        ErreurPont::DeclenchementRefuse { code: 1 },
        ErreurPont::MesureAnnulee {
            remise_au_repos: repos.clone(),
        },
        ErreurPont::ReponseInattendue { detail: "x".into() },
        ErreurPont::Delai {},
        ErreurPont::InstrumentPerdu {},
        ErreurPont::ReposIncertain {
            remise_au_repos: repos.clone(),
        },
        ErreurPont::DeconnexionEchouee {
            code: 1,
            remise_au_repos: repos,
        },
        ErreurPont::SessionFermee {},
        ErreurPont::MireNonReconnue {
            zones: 1,
            plages: 2_520,
            ecart_max: 420,
        },
        ErreurPont::AutreMire { texte: "x".into() },
        ErreurPont::FeuilleAbsente {},
        ErreurPont::InstrumentIndisponible { etat: 0 },
        ErreurPont::IncidentFeuille {
            etat: 150,
            premesure: None,
        },
        ErreurPont::ReconnaissanceDouteuse { fiabilite: 1 },
        ErreurPont::MesureInvraisemblable {
            correlation_milliemes: 100,
        },
        ErreurPont::PositionsPlages {
            defaut: DefautPositions::Manquante {
                zone: 0,
                rangee: 0,
                colonne: 0,
            },
        },
        ErreurPont::ComposantQrIntrouvable { chemin: "x".into() },
        ErreurPont::Sdk { code: 1 },
    ]
}

#[test]
fn chaque_refus_d_une_feuille_a_sa_cause_et_son_action() {
    let mut cles_vues = Vec::new();
    for erreur in toutes_les_erreurs() {
        cles_vues.push(ErreurFeuille::Feuille(erreur).cle());
    }
    for etat in ETATS_INCIDENT {
        cles_vues.push(
            ErreurFeuille::Feuille(ErreurPont::IncidentFeuille {
                etat,
                premesure: None,
            })
            .cle(),
        );
    }
    cles_vues.push(ErreurFeuille::SansFd9.cle());
    cles_vues.push(ErreurFeuille::Definition(String::new()).cle());
    // Repli de la page pour une clé qu'elle ne connaît pas.
    cles_vues.push("feuille_fd9.autre".into());
    assert_eq!(cles_vues.len(), 28 + 10 + 3);
    for cle in &cles_vues {
        assert!(cle.starts_with("feuille_fd9."), "{cle}");
        for suite in ["cause", "action"] {
            present(&format!("{cle}.{suite}"));
        }
    }
    // Un incident de la table du SDK n'a pas le texte général.
    assert_ne!(
        texte(Langue::Francais, "feuille_fd9.incident_feuille.102.cause"),
        texte(Langue::Francais, "feuille_fd9.incident_feuille.cause")
    );
}

#[test]
fn chaque_etape_et_chaque_raison_a_son_texte() {
    for etape in [
        "connexion",
        "glisser_feuille",
        "feuille_engagee",
        "lecture",
        "calage_pret",
        "mesure_lancee",
        "mesure",
        "rangement",
    ] {
        present(&format!("feuille_fd9.etape.{etape}"));
    }
    for raison in [RAISON_SANS_FD9, RAISON_OCCUPE] {
        present(raison);
    }
    // Le compteur n'invente rien : il reprend les deux nombres du pont.
    for langue in LANGUES {
        let mesure = texte(langue, "feuille_fd9.etape.mesure");
        assert!(
            mesure.contains("{n}") && mesure.contains("{total}"),
            "{mesure}"
        );
    }
}

/// Chaque clé écrite en toutes lettres dans `feuille-fd9.js` et
/// `feuille-vue.js` (`"feuille_fd9.…"`, dans un `t("…")` comme dans une
/// condition) est au catalogue ; les clés qu'ils composent
/// (`feuille_fd9.etape.${…}`, `${cle}.cause`) sont vérifiées ci-dessus.
#[test]
fn la_page_de_la_feuille_ne_demande_que_des_cles_du_catalogue() {
    let mut demandees = Vec::new();
    for fichier in ["feuille-fd9.js", "feuille-vue.js"] {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("interface")
                .join(fichier),
        )
        .unwrap();
        for (i, _) in script.match_indices("\"feuille_fd9.") {
            let cle: String = script[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            demandees.push(cle);
        }
    }
    assert!(demandees.len() > 20, "clés lues : {demandees:?}");
    // Une clé d'erreur (`feuille_fd9.autre`) vaut par sa cause et son action.
    let inconnues: Vec<_> = demandees
        .iter()
        .filter(|c| !cles().any(|k| k == c.as_str() || k == format!("{c}.cause")))
        .collect();
    assert!(
        inconnues.is_empty(),
        "clés absentes du catalogue : {inconnues:?}"
    );
}
