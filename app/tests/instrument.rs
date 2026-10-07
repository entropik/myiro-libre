//! Module `instrument` contre le pont simulé (ADR 0005) : la recherche du
//! logiciel du fabricant, l'ordre des paliers, le plafond et les échecs lui
//! appartiennent, pas aux écrans.

use std::path::{Path, PathBuf};

use app::instrument::{Ecran, Etat, Instrument, Probleme, PLAFOND};
use app::pont::{Architecture, Panne, PontSimule};
use app::textes::{texte, Langue};
use pont_protocole::{ErreurPont, Palier, Requete};

/// Numéro de série fictif : jamais celui d'un instrument réel.
const SERIE: u32 = 12345678;

/// Dossier temporaire neuf, propre à un test.
fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("instrument")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    dossier
}

/// Fausse DLL : seul l'en-tête PE (type de machine) compte. Elle n'est jamais
/// chargée : le pont simulé ne touche à aucune DLL.
fn fausse_dll(chemin: &Path, architecture: Architecture) {
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    let mut octets = vec![0u8; 0x48];
    octets[..2].copy_from_slice(b"MZ");
    octets[0x3c] = 0x40;
    octets[0x40..0x44].copy_from_slice(b"PE\0\0");
    let machine: u16 = match architecture {
        Architecture::X86 => 0x014c,
        Architecture::X64 => 0x8664,
    };
    octets[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
    std::fs::write(chemin, octets).unwrap();
}

/// Emplacement d'un logiciel du fabricant installé, avec sa DLL 64 bits.
fn sdk_factice(nom: &str) -> PathBuf {
    let dossier = dossier_vide(nom);
    fausse_dll(&dossier.join("Logiciel").join("FDXSDK.dll"), Architecture::X64);
    dossier
}

/// Les deux ponts livrés avec l'application.
fn ponts() -> Vec<(Architecture, PathBuf)> {
    vec![
        (Architecture::X64, PathBuf::from("pont-myiro1-x64.exe")),
        (Architecture::X86, PathBuf::from("pont-myiro1-x86.exe")),
    ]
}

fn ouvrir(simule: PontSimule, sdk: &Path) -> Instrument<PontSimule> {
    Instrument::ouvrir(&[sdk.to_path_buf()], &ponts(), |_, _, _| Ok(simule))
}

/// Programme et DLL avec lesquels le pont a été lancé.
fn lancement(emplacements: &[PathBuf], ponts: &[(Architecture, PathBuf)]) -> Option<(PathBuf, PathBuf)> {
    let mut recu = None;
    Instrument::ouvrir(emplacements, ponts, |programme, dll, _| {
        recu = Some((programme.to_path_buf(), dll.to_path_buf()));
        Ok(PontSimule::avec_instruments(&[SERIE]))
    });
    recu
}

fn ne_pas_lancer(_: &Path, _: &Path, _: Palier) -> Result<PontSimule, Panne> {
    panic!("le pont ne doit pas être lancé")
}

#[test]
fn un_myiro1_branche_est_connecte_et_demande_son_etalonnage() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();
    let sdk = sdk_factice("branche");

    let instrument = ouvrir(simule, &sdk);

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

#[test]
fn le_logiciel_du_fabricant_est_trouve_seul_et_retenu() {
    let sdk = sdk_factice("retenu");
    let dll = sdk.join("Logiciel").join("FDXSDK.dll");

    let instrument = ouvrir(PontSimule::avec_instruments(&[SERIE]), &sdk);

    assert_eq!(instrument.sdk(), Some(dll.as_path()));
}

#[test]
fn sans_logiciel_du_fabricant_l_application_propose_de_choisir_un_dossier() {
    let vide = dossier_vide("sans-logiciel");
    std::fs::create_dir_all(vide.join("sous-dossier")).unwrap();
    let absent = vide.join("absent");

    let instrument = Instrument::ouvrir(&[vide.clone(), absent.clone()], &ponts(), ne_pas_lancer);

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    let probleme = instrument.probleme().unwrap();
    assert_eq!(probleme.ecran(), Ecran::ChoixDossier);
    assert_eq!(probleme.code(), "logiciel_absent");
    let detail = probleme.detail().unwrap();
    assert!(detail.contains(&vide.display().to_string()), "{detail}");
    assert!(detail.contains(&absent.display().to_string()), "{detail}");
}

#[test]
fn la_dll_64_bits_est_preferee_et_lancee_avec_le_pont_64_bits() {
    let x86 = dossier_vide("preference-x86");
    fausse_dll(&x86.join("FDXSDK.dll"), Architecture::X86);
    let x64 = dossier_vide("preference-x64");
    fausse_dll(&x64.join("FDXSDK.dll"), Architecture::X64);

    let (programme, dll) = lancement(&[x86, x64.clone()], &ponts()).expect("pont lancé");

    assert_eq!(programme, PathBuf::from("pont-myiro1-x64.exe"));
    assert_eq!(dll, x64.join("FDXSDK.dll"));
}

/// Un logiciel qui range ses DLL par architecture dans des sous-dossiers.
#[test]
fn dans_un_meme_logiciel_la_dll_64_bits_est_preferee() {
    let logiciel = dossier_vide("sous-dossiers");
    fausse_dll(&logiciel.join("plugins").join("win.x86").join("FDXSDK.dll"), Architecture::X86);
    fausse_dll(
        &logiciel.join("plugins").join("win.x86_64").join("FDXSDK.dll"),
        Architecture::X64,
    );

    let (_, dll) = lancement(&[logiciel.clone()], &ponts()).expect("pont lancé");

    assert_eq!(dll, logiciel.join("plugins").join("win.x86_64").join("FDXSDK.dll"));
}

#[test]
fn une_dll_32_bits_est_lancee_avec_le_pont_32_bits() {
    let x86 = dossier_vide("seule-x86");
    fausse_dll(&x86.join("FDXSDK.dll"), Architecture::X86);

    let (programme, _) = lancement(&[x86], &ponts()).expect("pont lancé");

    assert_eq!(programme, PathBuf::from("pont-myiro1-x86.exe"));
}

/// Sans pont 32 bits, la DLL 32 bits est écartée : c'est un composant de
/// l'application qui manque, pas le logiciel du fabricant.
#[test]
fn sans_pont_32_bits_une_dll_32_bits_est_ecartee() {
    let x86 = dossier_vide("x86-sans-pont");
    fausse_dll(&x86.join("FDXSDK.dll"), Architecture::X86);
    let seul_x64 = vec![(Architecture::X64, PathBuf::from("pont-myiro1-x64.exe"))];

    let instrument = Instrument::ouvrir(&[x86], &seul_x64, ne_pas_lancer);

    match instrument.probleme() {
        Some(Probleme::PontIntrouvable { detail }) => assert!(detail.contains("32"), "{detail}"),
        autre => panic!("problème inattendu : {autre:?}"),
    }
}

#[test]
fn un_fichier_fdxsdk_qui_n_est_pas_une_dll_est_ecarte() {
    let dossier = dossier_vide("pas-une-dll");
    std::fs::write(dossier.join("FDXSDK.dll"), b"rien").unwrap();

    let instrument = Instrument::ouvrir(&[dossier], &ponts(), ne_pas_lancer);

    assert_eq!(instrument.probleme().unwrap().code(), "logiciel_absent");
}

/// Le pont exécuterait le code de n'importe quel fichier désigné : seul un
/// fichier nommé FDXSDK.dll (casse indifférente) est accepté.
#[test]
fn un_fichier_designe_qui_n_est_pas_fdxsdk_est_refuse() {
    let dossier = dossier_vide("mauvais-nom");
    let autre = dossier.join("autre.dll");
    fausse_dll(&autre, Architecture::X64);
    let casse = dossier.join("fdxsdk.DLL");
    fausse_dll(&casse, Architecture::X64);

    assert_eq!(lancement(&[autre], &ponts()), None);
    assert_eq!(lancement(&[casse.clone()], &ponts()).map(|(_, d)| d), Some(casse));
}

#[test]
fn le_pont_est_lance_avec_le_plafond_connexion() {
    assert_eq!(PLAFOND, Palier::Connexion);
    let sdk = sdk_factice("plafond");
    let mut recu = None;
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();

    Instrument::ouvrir(&[sdk], &ponts(), |_, _, plafond| {
        recu = Some(plafond);
        Ok(simule)
    });

    assert_eq!(recu, Some(Palier::Connexion));
    assert!(!journal.requetes().iter().any(|r| matches!(
        r,
        Requete::Etalonner {} | Requete::MesurerPonctuelle {} | Requete::MesurerBande { .. }
    )));
}

#[test]
fn un_pont_introuvable_est_signale() {
    let sdk = sdk_factice("pont-introuvable");
    let instrument = Instrument::ouvrir(&[sdk], &ponts(), |_, _, _| -> Result<PontSimule, Panne> {
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

    let instrument = ouvrir(simule, &sdk);

    assert_eq!(
        instrument.probleme(),
        Some(&Probleme::LogicielInutilisable {
            detail: "DllIntrouvable".into()
        })
    );
    assert_eq!(
        instrument.probleme().unwrap().ecran(),
        Ecran::ChoixDossier
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

    let instrument = ouvrir(simule, &sdk);

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

    let instrument = ouvrir(simule, &sdk);

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

    let instrument = ouvrir(simule, &sdk);

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

    let instrument = ouvrir(simule, &sdk);

    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PontEnPanne { .. })
    ));
    assert_eq!(journal.requetes().len(), 2, "rien après la panne");
}

#[test]
fn la_barre_recoit_le_modele_et_l_etat_sans_numero_de_serie() {
    let sdk = sdk_factice("vue");
    let instrument = ouvrir(PontSimule::avec_instruments(&[SERIE]), &sdk);

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
    let instrument = ouvrir(simule, &sdk);

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
        Probleme::LogicielAbsent { examines: d() },
        Probleme::LogicielInutilisable { detail: d() },
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

    let instrument = ouvrir(simule, &sdk);

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(instrument.probleme(), Some(&Probleme::AucunInstrument));
    assert_eq!(instrument.probleme().unwrap().ecran(), Ecran::NonDetecte);
    assert!(!journal
        .requetes()
        .iter()
        .any(|r| matches!(r, Requete::Connecter { .. })));
}
