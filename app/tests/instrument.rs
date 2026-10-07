//! Module `instrument` contre le pont simulé (ADR 0005) : l'ordre des paliers,
//! le plafond et les échecs lui appartiennent, pas aux écrans.

use std::path::{Path, PathBuf};

use app::instrument::{chercher_dll, Ecran, Etat, Instrument, Probleme, PLAFOND};
use app::pont::{Panne, PontSimule};
use app::textes::{texte, Langue};
use pont_protocole::{ErreurPont, Palier, Requete};

/// Numéro de série fictif : jamais celui d'un instrument réel.
const SERIE: u32 = 12345678;

/// Dossier temporaire qui contient un faux `FDXSDK.dll` (fichier vide, jamais
/// chargé : le pont simulé ne touche à aucune DLL).
fn sdk_factice(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join(nom)
        .join("Logiciel du fabricant");
    std::fs::create_dir_all(&dossier).unwrap();
    std::fs::write(dossier.join("FDXSDK.dll"), b"").unwrap();
    dossier.parent().unwrap().to_path_buf()
}

fn ouvrir(simule: PontSimule, sdk: Option<&Path>) -> Instrument<PontSimule> {
    Instrument::ouvrir(sdk, |_dll, _plafond| Ok(simule))
}

#[test]
fn un_myiro1_branche_est_connecte_et_demande_son_etalonnage() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();
    let sdk = sdk_factice("branche");

    let instrument = ouvrir(simule, Some(&sdk));

    match instrument.etat() {
        Etat::EtalonnageRequis(fiche) => {
            assert_eq!(fiche.modele, "MYIRO-1");
            assert_eq!(fiche.numero_serie, SERIE);
        }
        autre => panic!("état inattendu : {autre:?}"),
    }
    assert_eq!(instrument.probleme(), None);
    assert_eq!(
        journal.requetes(),
        vec![
            Requete::Version {},
            Requete::Detecter {},
            Requete::Connecter { instrument: 0 },
        ]
    );
}

fn ne_pas_lancer(_dll: &Path, _plafond: Palier) -> Result<PontSimule, Panne> {
    panic!("le pont ne doit pas être lancé")
}

#[test]
fn sans_emplacement_du_sdk_l_application_le_demande_sans_lancer_le_pont() {
    let instrument = Instrument::ouvrir(None, ne_pas_lancer);

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(instrument.probleme(), Some(&Probleme::SdkNonIndique));
    assert_eq!(
        instrument.probleme().unwrap().ecran(),
        Ecran::EmplacementSdk
    );
}

#[test]
fn un_emplacement_sans_dll_est_signale_clairement() {
    let vide = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("sans-dll");
    std::fs::create_dir_all(vide.join("sous-dossier")).unwrap();

    let instrument = Instrument::ouvrir(Some(&vide), ne_pas_lancer);

    assert_eq!(
        instrument.probleme(),
        Some(&Probleme::AucuneDll {
            emplacement: vide.display().to_string()
        })
    );
    assert_eq!(
        instrument.probleme().unwrap().ecran(),
        Ecran::EmplacementSdk
    );
}

#[test]
fn la_dll_est_trouvee_dans_un_sous_dossier_ou_designee_directement() {
    let sdk = sdk_factice("chercher");
    let dll = sdk.join("Logiciel du fabricant").join("FDXSDK.dll");
    assert_eq!(chercher_dll(&sdk), Some(dll.clone()));
    assert_eq!(chercher_dll(&dll), Some(dll));
}

/// Le pont chargerait n'importe quel fichier désigné : seul un fichier nommé
/// FDXSDK.dll (casse indifférente) est accepté.
#[test]
fn un_fichier_designe_qui_n_est_pas_fdxsdk_est_refuse() {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("mauvais-nom");
    std::fs::create_dir_all(&dossier).unwrap();
    let autre = dossier.join("autre.dll");
    std::fs::write(&autre, b"").unwrap();
    let casse = dossier.join("fdxsdk.DLL");
    std::fs::write(&casse, b"").unwrap();

    assert_eq!(chercher_dll(&autre), None);
    assert_eq!(chercher_dll(&casse), Some(casse));
}

#[test]
fn le_pont_est_lance_avec_le_plafond_connexion() {
    assert_eq!(PLAFOND, Palier::Connexion);
    let sdk = sdk_factice("plafond");
    let mut recu = None;
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();

    Instrument::ouvrir(Some(&sdk), |dll, plafond| {
        recu = Some((dll.to_path_buf(), plafond));
        Ok(simule)
    });

    let (dll, plafond) = recu.expect("pont lancé");
    assert_eq!(plafond, Palier::Connexion);
    assert!(dll.ends_with("FDXSDK.dll"));
    assert!(!journal.requetes().iter().any(|r| matches!(
        r,
        Requete::Etalonner {} | Requete::MesurerPonctuelle {} | Requete::MesurerBande { .. }
    )));
}

#[test]
fn un_pont_introuvable_est_signale() {
    let sdk = sdk_factice("pont-introuvable");
    let instrument = Instrument::ouvrir(Some(&sdk), |_, _| -> Result<PontSimule, Panne> {
        Err(Panne::Lancement {
            detail: "programme absent".into(),
        })
    });

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(
        instrument.probleme(),
        Some(&Probleme::PontIntrouvable {
            detail: "programme absent".into()
        })
    );
}

#[test]
fn une_dll_refusee_par_le_pont_renvoie_a_l_emplacement_du_sdk() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Version,
        Err(Panne::DllRefusee {
            detail: "DllIntrouvable".into(),
        }),
    );
    let sdk = sdk_factice("dll-refusee");

    let instrument = ouvrir(simule, Some(&sdk));

    assert_eq!(
        instrument.probleme(),
        Some(&Probleme::SdkInutilisable {
            detail: "DllIntrouvable".into()
        })
    );
    assert_eq!(
        instrument.probleme().unwrap().ecran(),
        Ecran::EmplacementSdk
    );
}

#[test]
fn une_connexion_refusee_par_l_instrument_garde_le_detail_technique() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Connexion,
        Ok(pont_protocole::Reponse::Erreur {
            erreur: ErreurPont::Delai,
        }),
    );
    let sdk = sdk_factice("connexion-refusee");

    let instrument = ouvrir(simule, Some(&sdk));

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    match instrument.probleme() {
        Some(Probleme::ConnexionImpossible { detail }) => assert!(detail.contains("Delai")),
        autre => panic!("problème inattendu : {autre:?}"),
    }
    assert_eq!(instrument.probleme().unwrap().ecran(), Ecran::NonDetecte);
}

/// Une détection en échec ne dit pas qu'un instrument a été vu.
#[test]
fn une_detection_en_echec_a_son_propre_probleme() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Detection,
        Ok(pont_protocole::Reponse::Erreur {
            erreur: ErreurPont::Sdk { code: -9999 },
        }),
    );
    let sdk = sdk_factice("detection");

    let instrument = ouvrir(simule, Some(&sdk));

    match instrument.probleme() {
        Some(p @ Probleme::DetectionImpossible { detail }) => {
            assert!(detail.contains("-9999"));
            assert_eq!(p.ecran(), Ecran::NonDetecte);
            assert!(p.guide_cablage());
        }
        autre => panic!("problème inattendu : {autre:?}"),
    }
}

/// Les étapes câble et port USB ne servent à rien quand c'est le programme
/// pont qui manque ou ne répond pas.
#[test]
fn le_guide_de_cablage_n_apparait_que_pour_un_probleme_d_instrument() {
    let d = || String::from("détail");
    assert!(Probleme::AucunInstrument.guide_cablage());
    assert!(Probleme::ConnexionImpossible { detail: d() }.guide_cablage());
    assert!(!Probleme::PontIntrouvable { detail: d() }.guide_cablage());
    assert!(!Probleme::PontEnPanne { detail: d() }.guide_cablage());
    assert!(!Probleme::PontBloque { detail: d() }.guide_cablage());
}

#[test]
fn un_pont_qui_ne_repond_plus_laisse_l_instrument_dans_un_etat_incertain() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Connexion,
        Err(Panne::SansReponse {
            detail: "aucune réponse en 30 s".into(),
        }),
    );
    let sdk = sdk_factice("bloque");

    let instrument = ouvrir(simule, Some(&sdk));

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PontBloque { .. })
    ));
    let vue = serde_json::to_value(instrument.vue()).unwrap();
    assert_eq!(vue["probleme"]["code"], "pont_bloque");
    assert_eq!(vue["probleme"]["guide_cablage"], false);
}

#[test]
fn un_pont_qui_s_arrete_en_cours_de_route_est_signale() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Detection,
        Err(Panne::Arret {
            code: Some(1),
            detail: "plus de sortie".into(),
        }),
    );
    let journal = simule.journal();
    let sdk = sdk_factice("arret");

    let instrument = ouvrir(simule, Some(&sdk));

    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PontEnPanne { .. })
    ));
    assert_eq!(journal.requetes().len(), 2, "rien après la panne");
}

#[test]
fn la_barre_recoit_le_modele_et_l_etat_sans_numero_de_serie() {
    let sdk = sdk_factice("vue");
    let instrument = ouvrir(PontSimule::avec_instruments(&[SERIE]), Some(&sdk));

    let vue = serde_json::to_value(instrument.vue()).unwrap();

    assert_eq!(
        vue,
        serde_json::json!({
            "etat": "etalonnage_requis",
            "modele": "MYIRO-1",
            "probleme": null,
        })
    );
}

#[test]
fn un_probleme_arrive_a_l_ecran_avec_son_ecran_et_son_detail_replie() {
    let sdk = sdk_factice("vue-probleme");
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::Connexion,
        Ok(pont_protocole::Reponse::Erreur {
            erreur: ErreurPont::InstrumentPerdu,
        }),
    );
    let instrument = ouvrir(simule, Some(&sdk));

    let vue = serde_json::to_value(instrument.vue()).unwrap();

    assert_eq!(vue["etat"], "non_detecte");
    assert_eq!(vue["modele"], serde_json::Value::Null);
    assert_eq!(vue["probleme"]["code"], "connexion_impossible");
    assert_eq!(vue["probleme"]["ecran"], "non_detecte");
    assert!(vue["probleme"]["detail"]
        .as_str()
        .unwrap()
        .contains("InstrumentPerdu"));
}

/// Chaque problème a sa cause probable et son action, en français et en
/// anglais : aucune erreur du pont n'arrive brute à l'écran.
#[test]
fn chaque_probleme_a_sa_cause_et_son_action_dans_les_deux_langues() {
    let d = || String::from("détail");
    let tous = [
        Probleme::SdkNonIndique,
        Probleme::AucuneDll { emplacement: d() },
        Probleme::SdkInutilisable { detail: d() },
        Probleme::PontIntrouvable { detail: d() },
        Probleme::PontEnPanne { detail: d() },
        Probleme::PontBloque { detail: d() },
        Probleme::AucunInstrument,
        Probleme::DetectionImpossible { detail: d() },
        Probleme::ConnexionImpossible { detail: d() },
    ];
    for probleme in tous {
        for langue in [Langue::Francais, Langue::Anglais] {
            for partie in ["cause", "action"] {
                let cle = format!("probleme.{}.{partie}", probleme.code());
                assert_ne!(texte(langue, &cle), cle, "texte manquant : {cle}");
            }
        }
    }
}

#[test]
fn sans_instrument_branche_l_ecran_non_detecte_guide_l_operateur() {
    let simule = PontSimule::avec_instruments(&[]);
    let journal = simule.journal();
    let sdk = sdk_factice("aucun");

    let instrument = ouvrir(simule, Some(&sdk));

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(instrument.probleme(), Some(&Probleme::AucunInstrument));
    assert_eq!(instrument.probleme().unwrap().ecran(), Ecran::NonDetecte);
    assert!(!journal
        .requetes()
        .iter()
        .any(|r| matches!(r, Requete::Connecter { .. })));
}
