//! `PontProcessus` parle à un vrai processus par l'entrée et la sortie standard.
//! Le processus est `examples/pont_factice.rs`, qui joue le rôle de
//! `pont-myiro1` sans charger aucune DLL ; son scénario est passé à la place du
//! chemin de la DLL.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use app::instrument::{Etat, Instrument, Probleme, PLAFOND};
use app::pont::{Panne, Pont, PontProcessus};
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
            assert_eq!(detail, "--dll echo --plafond connexion")
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
    assert!(debut.elapsed() < Duration::from_secs(5), "{:?}", debut.elapsed());
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
    assert!(debut.elapsed() < Duration::from_secs(5), "{:?}", debut.elapsed());
}

/// Fermer un pont bloqué ne bloque pas non plus.
#[test]
fn la_fermeture_d_un_pont_bloque_est_bornee() {
    let mut pont = lancer("bloque").avec_delai(Duration::from_millis(300));
    let _ = pont.demander(&Requete::Version {});
    let debut = Instant::now();
    drop(pont);
    assert!(debut.elapsed() < Duration::from_secs(5), "{:?}", debut.elapsed());
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

/// De bout en bout : le module instrument, le vrai transport, un pont factice.
#[test]
fn l_instrument_s_ouvre_a_travers_un_vrai_processus() {
    let programme = pont_factice();
    // Le module cherche un FDXSDK.dll : on lui donne un fichier de ce nom,
    // vide ; le pont factice ne le charge pas et suit le scénario « normal ».
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("processus");
    std::fs::create_dir_all(&dossier).unwrap();
    std::fs::write(dossier.join("FDXSDK.dll"), b"").unwrap();

    let instrument = Instrument::ouvrir(Some(&dossier), |_dll, plafond| {
        PontProcessus::lancer(&programme, Path::new("normal"), plafond)
    });

    assert!(
        matches!(instrument.etat(), Etat::EtalonnageRequis(f) if f.modele == "MYIRO-1"),
        "{:?} / {:?}",
        instrument.etat(),
        instrument.probleme()
    );
    assert_eq!(instrument.probleme(), None::<&Probleme>);
}
