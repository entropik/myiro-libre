//! Module `instrument` et le FD-9 (ticket #13) : il lance le pont FD-9 avec ses
//! propres paliers (version, détection) et la barre montre un FD-9 détecté.
//! Un pont FD-9 simulé, propre à ces tests, répond comme `pont-fd9`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use app::instrument::fd9::{
    emplacements_fd9, ouvrir_l_un_ou_l_autre, Recherche, EMPLACEMENTS_FD9, PLAFOND_FD9,
};
use app::instrument::{Etat, Instrument, Probleme};
use app::pont::{chercher_ponts_nommes, Architecture, Panne, Pont, PontSimule};
use pont_protocole::{ErreurPont, Info, InstrumentFd9, Palier, Reponse, Requete};

/// Identifiant et adresse fictifs : jamais ceux d'un instrument réel.
const IDENTIFIANT: &str = "12345678";
const ADRESSE: &str = "192.168.1.40";

fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("instrument-fd9")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    dossier
}

/// Fausse DLL : seul l'en-tête PE compte, elle n'est jamais chargée.
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

/// FD-S2w installé : sa DLL FD-9 est 32 bits, dans `Module`.
fn fd_s2w(nom: &str) -> PathBuf {
    let dossier = dossier_vide(nom);
    fausse_dll(
        &dossier.join("Module").join("FD9SDK.dll"),
        Architecture::X86,
    );
    dossier
}

fn ponts_fd9() -> Vec<(Architecture, PathBuf)> {
    vec![
        (Architecture::X64, PathBuf::from("pont-fd9-x64.exe")),
        (Architecture::X86, PathBuf::from("pont-fd9-x86.exe")),
    ]
}

fn ponts_myiro1() -> Vec<(Architecture, PathBuf)> {
    vec![(Architecture::X64, PathBuf::from("pont-myiro1-x64.exe"))]
}

/// Pont FD-9 simulé : répond comme `pont-fd9` (paliers version et détection),
/// ou rend la réponse donnée à un palier.
#[derive(Clone, Default)]
struct PontFd9Simule {
    liste: Vec<InstrumentFd9>,
    version: Option<Reponse>,
    detection: Option<Reponse>,
    journal: Rc<RefCell<Vec<Requete>>>,
}

impl PontFd9Simule {
    fn avec_un_fd9() -> Self {
        PontFd9Simule {
            liste: vec![InstrumentFd9 {
                liaison: "reseau".into(),
                adresse: ADRESSE.into(),
                identifiant: IDENTIFIANT.into(),
            }],
            ..Default::default()
        }
    }

    fn requetes(&self) -> Vec<Requete> {
        self.journal.borrow().clone()
    }
}

impl Pont for PontFd9Simule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.journal.borrow_mut().push(requete.clone());
        Ok(match requete {
            Requete::Version {} => self.version.clone().unwrap_or(Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Inconnue,
            }),
            Requete::Detecter {} => self.detection.clone().unwrap_or(Reponse::InstrumentsFd9 {
                liste: self.liste.clone(),
            }),
            Requete::Fermer {} => Reponse::Ferme {},
            _ => Reponse::Erreur {
                erreur: ErreurPont::PalierNonAutorise {
                    demande: Palier::Connexion,
                    plafond: Palier::Detection,
                },
            },
        })
    }
}

fn ouvrir_fd9(simule: PontFd9Simule, sdk: &Path) -> Instrument<PontFd9Simule> {
    Instrument::ouvrir_fd9(&[sdk.to_path_buf()], &ponts_fd9(), |_, _, _| Ok(simule))
}

#[test]
fn un_fd9_du_reseau_est_detecte_et_la_barre_le_montre() {
    let simule = PontFd9Simule::avec_un_fd9();
    let sdk = fd_s2w("detecte");
    let mut lance = None;

    let instrument = Instrument::ouvrir_fd9(
        std::slice::from_ref(&sdk),
        &ponts_fd9(),
        |programme, dll, plafond| {
            lance = Some((programme.to_path_buf(), dll.to_path_buf(), plafond));
            Ok(simule.clone())
        },
    );

    assert_eq!(
        instrument.etat(),
        &Etat::Detecte {
            modele: "FD-9".into(),
            identifiant: IDENTIFIANT.into(),
        }
    );
    assert_eq!(instrument.probleme(), None);
    // Le pont de 32 bits, pour la DLL 32 bits de FD-S2w, avec ses paliers.
    assert_eq!(
        lance,
        Some((
            PathBuf::from("pont-fd9-x86.exe"),
            sdk.join("Module").join("FD9SDK.dll"),
            Palier::Detection
        ))
    );
    assert_eq!(PLAFOND_FD9, Palier::Detection);
    // Jamais de connexion : le pont FD-9 n'a que deux paliers.
    assert_eq!(
        simule.requetes(),
        vec![Requete::Version {}, Requete::Detecter {}]
    );
    let vue = instrument.vue();
    assert_eq!(vue.etat, "detecte");
    assert_eq!(vue.modele.as_deref(), Some("FD-9"));
    assert!(!vue.pret);
    assert_eq!(vue.probleme, None);
}

#[test]
fn sans_fd9_visible_le_probleme_est_aucun_instrument() {
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.liste.clear();
    let instrument = ouvrir_fd9(simule, &fd_s2w("aucun"));
    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(instrument.probleme(), Some(&Probleme::AucunInstrument));
}

#[test]
fn une_detection_en_erreur_est_une_detection_impossible() {
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.detection = Some(Reponse::Erreur {
        erreur: ErreurPont::Sdk { code: 1002 },
    });
    let instrument = ouvrir_fd9(simule, &fd_s2w("detection-impossible"));
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::DetectionImpossible { detail }) if detail.contains("1002")
    ));
}

#[test]
fn une_dll_qui_ne_rend_pas_sa_version_est_inutilisable() {
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.version = Some(Reponse::Erreur {
        erreur: ErreurPont::Sdk { code: 1999 },
    });
    let journal = simule.journal.clone();
    let instrument = ouvrir_fd9(simule, &fd_s2w("inutilisable"));
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::LogicielInutilisable { .. })
    ));
    assert_eq!(journal.borrow().len(), 1, "pas de détection après l'échec");
}

#[test]
fn une_reponse_d_un_autre_pont_est_une_panne() {
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.version = Some(Reponse::Version { parties: [1, 0, 1] });
    let instrument = ouvrir_fd9(simule, &fd_s2w("autre-pont"));
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PontEnPanne { .. })
    ));
}

#[test]
fn la_recherche_du_fd9_ne_retient_que_fd9sdk() {
    // Un logiciel qui livre les deux DLL : seule celle du FD-9 est prise.
    let dossier = dossier_vide("deux-dll");
    fausse_dll(&dossier.join("FDXSDK.dll"), Architecture::X64);
    fausse_dll(&dossier.join("FD9SDK.dll"), Architecture::X64);
    let mut recue = None;
    Instrument::ouvrir_fd9(
        std::slice::from_ref(&dossier),
        &ponts_fd9(),
        |programme, dll, _| {
            recue = Some((programme.to_path_buf(), dll.to_path_buf()));
            Ok(PontFd9Simule::avec_un_fd9())
        },
    );
    assert_eq!(
        recue,
        Some((
            PathBuf::from("pont-fd9-x64.exe"),
            dossier.join("FD9SDK.dll")
        ))
    );
}

#[test]
fn le_fd9_est_cherche_dans_le_dossier_choisi_puis_chez_fd_s2w_et_ergosoft() {
    let choisi = PathBuf::from("D:/choisi");
    let emplacements = emplacements_fd9(Some(choisi.clone()));
    assert_eq!(emplacements[0], choisi);
    assert_eq!(
        emplacements[1..],
        EMPLACEMENTS_FD9
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>()[..]
    );
    assert!(EMPLACEMENTS_FD9[0].contains("FD-S2w"));
    assert!(EMPLACEMENTS_FD9[1].contains("Ergosoft"));
}

#[test]
fn les_ponts_fd9_sont_cherches_sous_leur_propre_nom() {
    let dossier = dossier_vide("ponts");
    let exe = |nom: &str| dossier.join(format!("{nom}{}", std::env::consts::EXE_SUFFIX));
    fausse_dll(&exe("pont-fd9-x86"), Architecture::X86);
    fausse_dll(&exe("pont-myiro1-x64"), Architecture::X64);
    assert_eq!(
        chercher_ponts_nommes(&dossier, "pont-fd9"),
        vec![(Architecture::X86, exe("pont-fd9-x86"))]
    );
}

/// Pont lancé par l'application : l'un ou l'autre selon le programme.
enum Simule {
    Myiro1(PontSimule),
    Fd9(PontFd9Simule),
}

impl Pont for Simule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        match self {
            Simule::Myiro1(p) => p.demander(requete),
            Simule::Fd9(p) => p.demander(requete),
        }
    }
}

/// Ouvre l'un ou l'autre instrument ; rend l'instrument et les programmes lancés.
fn ouvrir_un_des_deux(
    myiro1: &[PathBuf],
    fd9: &[PathBuf],
    series_myiro1: &[u32],
) -> (Instrument<Simule>, Vec<PathBuf>) {
    let mut lances = Vec::new();
    let instrument = ouvrir_l_un_ou_l_autre(
        Recherche {
            emplacements: myiro1,
            ponts: &ponts_myiro1(),
        },
        Recherche {
            emplacements: fd9,
            ponts: &ponts_fd9(),
        },
        |programme: &Path, _: &Path, _| {
            lances.push(programme.to_path_buf());
            Ok(if programme.to_string_lossy().contains("fd9") {
                Simule::Fd9(PontFd9Simule::avec_un_fd9())
            } else {
                Simule::Myiro1(PontSimule::avec_instruments(series_myiro1))
            })
        },
    );
    (instrument, lances)
}

fn logiciel_myiro1(nom: &str) -> PathBuf {
    let dossier = dossier_vide(nom);
    fausse_dll(&dossier.join("FDXSDK.dll"), Architecture::X64);
    dossier
}

#[test]
fn un_myiro1_branche_passe_avant_le_fd9_qui_n_est_pas_lance() {
    let (instrument, lances) = ouvrir_un_des_deux(
        &[logiciel_myiro1("deux-myiro1")],
        &[fd_s2w("deux-fd9")],
        &[12345678],
    );
    assert!(matches!(instrument.etat(), Etat::EtalonnageRequis(_)));
    assert_eq!(lances, vec![PathBuf::from("pont-myiro1-x64.exe")]);
}

#[test]
fn sans_myiro1_le_fd9_detecte_est_montre() {
    let (instrument, lances) = ouvrir_un_des_deux(
        &[logiciel_myiro1("seul-fd9-myiro1")],
        &[fd_s2w("seul-fd9")],
        &[],
    );
    assert_eq!(instrument.vue().etat, "detecte");
    assert_eq!(instrument.vue().modele.as_deref(), Some("FD-9"));
    assert_eq!(
        lances,
        vec![
            PathBuf::from("pont-myiro1-x64.exe"),
            PathBuf::from("pont-fd9-x86.exe")
        ]
    );
}

#[test]
fn sans_aucun_des_deux_le_probleme_du_myiro1_reste_montre() {
    let (instrument, _) = ouvrir_un_des_deux(
        &[dossier_vide("rien-myiro1")],
        &[dossier_vide("rien-fd9")],
        &[],
    );
    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::LogicielAbsent { examines }) if examines.contains("FDXSDK.dll")
    ));
}
