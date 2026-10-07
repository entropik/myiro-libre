//! `PontProcessus` parle à un vrai processus par l'entrée et la sortie standard.
//! Le processus est `examples/pont_factice.rs`, qui joue le rôle de
//! `pont-myiro1` sans charger aucune DLL ; son scénario est passé à la place du
//! chemin de la DLL.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use app::instrument::{Etat, Instrument, Probleme, PLAFOND};
use app::pont::{architecture, chercher_ponts, Architecture, Panne, Pont, PontProcessus};
use pont_protocole::{Palier, Reponse, Requete};

/// L'exemple est compilé par `cargo test` (toutes cibles) ou par
/// `cargo build -p app --example pont_factice`.
fn pont_factice() -> PathBuf {
    let deps = std::env::current_exe().unwrap();
    let profil = deps.parent().unwrap().parent().unwrap();
    let chemin = profil
        .join("examples")
        .join(format!("pont_factice{}", std::env::consts::EXE_SUFFIX));
    assert!(
        chemin.exists(),
        "{} absent : lancer `cargo test -p app` sans filtre de cible, \
         ou `cargo build -p app --example pont_factice`",
        chemin.display()
    );
    chemin
}

fn lancer(scenario: &str) -> PontProcessus {
    PontProcessus::lancer(&pont_factice(), Path::new(scenario), PLAFOND).expect("lancement")
}

#[test]
fn le_dialogue_suit_une_ligne_json_par_message() {
    let mut pont = lancer("normal");

    assert_eq!(
        pont.demander(&Requete::Version {}),
        Ok(Reponse::Version { parties: [1, 0, 1] })
    );
    assert!(matches!(
        pont.demander(&Requete::Detecter {}),
        Ok(Reponse::Instruments { liste }) if liste.len() == 1
    ));
    assert!(matches!(
        pont.demander(&Requete::Connecter { instrument: 0 }),
        Ok(Reponse::Connecte { .. })
    ));
}

#[test]
fn le_pont_recoit_la_dll_et_le_plafond_en_arguments() {
    let mut pont = lancer("echo");

    match pont.demander(&Requete::Version {}) {
        Ok(Reponse::RequeteInvalide { detail }) => {
            assert_eq!(detail, "--dll echo --plafond etalonnage")
        }
        autre => panic!("réponse inattendue : {autre:?}"),
    }
}

#[test]
fn une_dll_refusee_au_demarrage_est_reconnue() {
    let mut pont = lancer("dll_refusee");

    match pont.demander(&Requete::Version {}) {
        Err(Panne::DllRefusee { detail }) => assert!(detail.contains("DllIntrouvable")),
        autre => panic!("réponse inattendue : {autre:?}"),
    }
}

#[test]
fn un_pont_qui_s_arrete_rend_son_code_et_son_message() {
    let mut pont = lancer("muet");

    match pont.demander(&Requete::Version {}) {
        Err(Panne::Arret { code, detail }) => {
            assert_eq!(code, Some(1));
            assert!(detail.contains("arrêt simulé"));
        }
        autre => panic!("réponse inattendue : {autre:?}"),
    }
}

/// Un pont bloqué (DLL qui ne rend pas la main) ne bloque pas l'application :
/// passé le délai, il est arrêté de force et la panne le dit.
#[test]
fn sans_reponse_dans_le_delai_le_pont_est_arrete_de_force() {
    let mut pont = lancer("bloque").avec_delai(Duration::from_millis(300));
    let debut = Instant::now();

    assert!(matches!(
        pont.demander(&Requete::Version {}),
        Err(Panne::SansReponse { .. })
    ));
    // Le processus a été arrêté : la demande suivante échoue aussitôt.
    assert!(pont.demander(&Requete::Detecter {}).is_err());
    assert!(
        debut.elapsed() < Duration::from_secs(5),
        "{:?}",
        debut.elapsed()
    );
}

/// Un pont qui ferme sa sortie sans se terminer est arrêté de force, sans
/// attente sans fin.
#[test]
fn une_sortie_fermee_sans_fin_du_pont_est_bornee() {
    let mut pont = lancer("sortie_fermee").avec_delai(Duration::from_millis(300));
    let debut = Instant::now();

    assert!(matches!(
        pont.demander(&Requete::Version {}),
        Err(Panne::SansReponse { .. })
    ));
    assert!(
        debut.elapsed() < Duration::from_secs(5),
        "{:?}",
        debut.elapsed()
    );
}

/// L'étalonnage attend jusqu'à 30 s dans le pont, en plus de l'appel à la DLL :
/// l'application lui laisse deux fois le délai d'une autre demande, pour ne
/// pas couper un pont qui allait répondre.
#[test]
fn l_etalonnage_a_deux_fois_le_delai_d_une_autre_demande() {
    let mut pont = lancer("etalonnage_lent").avec_delai(Duration::from_millis(300));

    assert_eq!(
        pont.demander(&Requete::Etalonner {}),
        Ok(Reponse::Etalonne {})
    );
}

/// Fermer un pont bloqué ne bloque pas non plus.
#[test]
fn la_fermeture_d_un_pont_bloque_est_bornee() {
    let mut pont = lancer("bloque").avec_delai(Duration::from_millis(300));
    let _ = pont.demander(&Requete::Version {});
    let debut = Instant::now();
    drop(pont);
    assert!(
        debut.elapsed() < Duration::from_secs(5),
        "{:?}",
        debut.elapsed()
    );
}

#[test]
fn une_ligne_hors_protocole_est_signalee() {
    let mut pont = lancer("illisible");

    assert!(matches!(
        pont.demander(&Requete::Version {}),
        Err(Panne::ReponseIllisible { .. })
    ));
}

#[test]
fn un_programme_absent_ne_se_lance_pas() {
    let resultat = PontProcessus::lancer(
        Path::new("C:/nulle-part/pont-myiro1.exe"),
        Path::new("normal"),
        Palier::Connexion,
    );
    assert!(matches!(resultat, Err(Panne::Lancement { .. })));
}

fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("ponts")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    dossier
}

/// Faux exécutable : seul l'en-tête PE (type de machine) compte.
fn faux_programme(chemin: &Path, architecture: Architecture) {
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

/// Application installée : les deux ponts sont à côté de son exécutable.
#[test]
fn les_ponts_installes_a_cote_de_l_application_sont_trouves() {
    let installe = dossier_vide("installe");
    faux_programme(&installe.join("pont-myiro1-x64.exe"), Architecture::X64);
    faux_programme(&installe.join("pont-myiro1-x86.exe"), Architecture::X86);

    assert_eq!(
        chercher_ponts(&installe),
        vec![
            (Architecture::X64, installe.join("pont-myiro1-x64.exe")),
            (Architecture::X86, installe.join("pont-myiro1-x86.exe")),
        ]
    );
}

/// En développement : `target/debug/` pour le pont de l'hôte et
/// `target/i686-pc-windows-msvc/debug/` pour le pont 32 bits.
#[test]
fn en_developpement_les_ponts_sont_cherches_dans_target() {
    let target = dossier_vide("target");
    let debug = target.join("debug");
    faux_programme(&debug.join("pont-myiro1.exe"), Architecture::X64);
    let x86 = target
        .join("i686-pc-windows-msvc")
        .join("debug")
        .join("pont-myiro1.exe");
    faux_programme(&x86, Architecture::X86);

    assert_eq!(
        chercher_ponts(&debug),
        vec![
            (Architecture::X64, debug.join("pont-myiro1.exe")),
            (Architecture::X86, x86),
        ]
    );
}

/// L'architecture d'un pont est lue dans son en-tête, pas dans son nom ; un
/// fichier absent ou illisible n'est pas proposé.
#[test]
fn un_pont_est_classe_par_son_en_tete() {
    let dossier = dossier_vide("en-tete");
    faux_programme(&dossier.join("pont-myiro1-x86.exe"), Architecture::X64);
    std::fs::write(dossier.join("pont-myiro1-x64.exe"), b"rien").unwrap();

    assert_eq!(
        chercher_ponts(&dossier),
        vec![(Architecture::X64, dossier.join("pont-myiro1-x86.exe"))]
    );
    assert_eq!(architecture(&dossier.join("absent.exe")), None);
}

/// Une erreur que l'application ne connaît pas (pont plus récent) devient un
/// problème explicite, jamais un instrument prêt.
#[test]
fn une_erreur_inconnue_du_pont_devient_un_probleme_explicite() {
    let programme = pont_factice();
    let arch = architecture(&programme).expect("le pont factice est un exécutable");
    let dossier = dossier_vide("erreur-inconnue");
    faux_programme(&dossier.join("FDXSDK.dll"), arch);

    let instrument = Instrument::ouvrir(&[dossier], &[(arch, programme)], |prog, _dll, plafond| {
        PontProcessus::lancer(prog, Path::new("erreur_inconnue"), plafond)
    });

    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    match instrument.probleme() {
        Some(Probleme::PontEnPanne { detail }) => {
            assert!(detail.contains("panne_future"), "{detail}")
        }
        autre => panic!("problème inattendu : {autre:?}"),
    }
    assert!(!instrument.vue().pret);
}

/// De bout en bout : le module instrument, le vrai transport, un pont factice.
#[test]
fn l_instrument_s_ouvre_a_travers_un_vrai_processus() {
    let programme = pont_factice();
    let arch = architecture(&programme).expect("le pont factice est un exécutable");
    // Le module cherche un FDXSDK.dll de la même architecture que le pont : on
    // lui donne un faux ; le pont factice ne le charge pas et suit le
    // scénario « normal ».
    let dossier = dossier_vide("processus");
    faux_programme(&dossier.join("FDXSDK.dll"), arch);

    let instrument = Instrument::ouvrir(&[dossier], &[(arch, programme)], |prog, _dll, plafond| {
        PontProcessus::lancer(prog, Path::new("normal"), plafond)
    });

    assert!(
        matches!(instrument.etat(), Etat::EtalonnageRequis(f) if f.modele == "MYIRO-1"),
        "{:?} / {:?}",
        instrument.etat(),
        instrument.probleme()
    );
    assert_eq!(instrument.probleme(), None::<&Probleme>);
}
