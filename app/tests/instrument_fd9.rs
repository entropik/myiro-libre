//! Module `instrument` et le FD-9 (ticket #13) : il lance le pont FD-9 avec ses
//! propres paliers (version, détection) et la barre montre un FD-9 détecté.
//! Un pont FD-9 simulé, propre à ces tests, répond comme `pont-fd9`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use app::instrument::choix::{ouvrir_l_un_ou_l_autre, Recherche};
use app::instrument::fd9::{emplacements_fd9, EMPLACEMENTS_FD9, PLAFOND_FD9};
use app::instrument::parefeu::{AutorisationPareFeu, EchecPareFeu, PareFeuSimule, RegleFd9};
use app::instrument::{Etat, Instrument, Probleme};
use app::pont::{chercher_ponts_nommes, Architecture, Panne, Pont, PontSimule};
use app::textes::{texte, Langue};
use pont_protocole::{ErreurPont, Info, InstrumentFd9, LiaisonFd9, Palier, Reponse, Requete};

/// Identifiant et adresse fictifs : jamais ceux d'un instrument réel.
const IDENTIFIANT: &str = "12345678";
const ADRESSE: &str = "192.0.2.40";

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
    /// Nombre de détections qui reviennent vides avant de voir la liste
    /// (réponse bloquée par le pare-feu, puis règle ajoutée).
    vides_d_abord: usize,
    journal: Rc<RefCell<Vec<Requete>>>,
}

impl PontFd9Simule {
    fn avec_un_fd9() -> Self {
        PontFd9Simule {
            liste: vec![InstrumentFd9 {
                liaison: Info::Confirmee(LiaisonFd9::Reseau),
                adresse: ADRESSE.into(),
                identifiant: Info::Supposee(IDENTIFIANT.into()),
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
            Requete::Detecter {} if self.vides_d_abord > 0 => {
                self.vides_d_abord -= 1;
                Reponse::InstrumentsFd9 { liste: Vec::new() }
            }
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

/// Pare-feu simulé où la règle de myiro-libre existe déjà pour le pont lancé
/// (`pont-fd9-x86.exe`) : rien n'est demandé.
fn regle_presente() -> AutorisationPareFeu<PareFeuSimule> {
    AutorisationPareFeu::new(PareFeuSimule {
        regle: Some(PathBuf::from("pont-fd9-x86.exe")),
        ..Default::default()
    })
}

fn ouvrir_fd9(simule: PontFd9Simule, sdk: &Path) -> Instrument<PontFd9Simule> {
    ouvrir_fd9_avec(simule, sdk, &mut regle_presente())
}

fn ouvrir_fd9_avec(
    simule: PontFd9Simule,
    sdk: &Path,
    autorisation: &mut AutorisationPareFeu<PareFeuSimule>,
) -> Instrument<PontFd9Simule> {
    Instrument::ouvrir_fd9(
        &[sdk.to_path_buf()],
        &ponts_fd9(),
        |_, _, _| Ok(simule),
        autorisation,
    )
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
        &mut regle_presente(),
    );

    assert_eq!(
        instrument.etat(),
        &Etat::Detecte {
            modele: "FD-9".into(),
            identifiant: Info::Supposee(IDENTIFIANT.into()),
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

/// Le pont FD-9 s'arrête à la détection : un FD-9 détecté ne propose jamais
/// « Mesurer », et une mesure demandée quand même ne part pas (ticket #7).
#[test]
fn un_fd9_detecte_ne_propose_jamais_de_mesurer() {
    let simule = PontFd9Simule::avec_un_fd9();
    let mut instrument = ouvrir_fd9(simule.clone(), &fd_s2w("sans-mesure"));
    assert!(matches!(instrument.etat(), Etat::Detecte { .. }));

    assert!(!instrument.vue().mesurable);
    let rendue = instrument.mesurer_ponctuelle(&mut |g: app::instrument::Geste| {
        panic!("geste demandé : {g:?}")
    });

    assert_eq!(rendue, None);
    assert_eq!(
        simule.requetes(),
        vec![Requete::Version {}, Requete::Detecter {}]
    );
}

#[test]
fn sans_fd9_visible_le_pare_feu_est_cite_comme_cause_possible() {
    // Essai du 9 octobre 2026 : sans règle entrante du pare-feu, la réponse du
    // FD-9 à la diffusion est bloquée et la liste revient vide, sans erreur.
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.liste.clear();
    let instrument = ouvrir_fd9(simule, &fd_s2w("aucun"));
    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    let probleme = instrument.probleme().expect("un problème");
    assert!(matches!(probleme, Probleme::AucunFd9 { .. }));
    let vue = instrument.vue().probleme.unwrap();
    assert_eq!(vue.code, "aucun_fd9");
    assert_eq!(vue.ecran, "non_detecte");
    // Câble et port USB ne servent pas : le FD-9 est sur le réseau.
    assert!(!vue.guide_cablage);
    let detail = vue.detail.unwrap();
    assert!(
        detail.contains("49152") && detail.contains("pont-fd9"),
        "{detail}"
    );
    for (langue, mot) in [
        (Langue::Francais, "pare-feu"),
        (Langue::Anglais, "firewall"),
    ] {
        let cause = texte(langue, "probleme.aucun_fd9.cause");
        let action = texte(langue, "probleme.aucun_fd9.action");
        assert!(
            cause.contains(mot) || action.contains(mot),
            "{cause} {action}"
        );
    }
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
        &mut regle_presente(),
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
    Fd9(Box<PontFd9Simule>),
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
                Simule::Fd9(Box::new(PontFd9Simule::avec_un_fd9()))
            } else {
                Simule::Myiro1(PontSimule::avec_instruments(series_myiro1))
            })
        },
        &mut regle_presente(),
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
fn sans_logiciel_myiro1_le_vrai_probleme_du_fd9_est_montre() {
    // FD-S2w est installé, mais aucun pont 32 bits n'est livré pour sa DLL.
    let mut lances = Vec::new();
    let instrument = ouvrir_l_un_ou_l_autre(
        Recherche {
            emplacements: &[dossier_vide("fd9-sans-pont-myiro1")],
            ponts: &ponts_myiro1(),
        },
        Recherche {
            emplacements: &[fd_s2w("fd9-sans-pont")],
            ponts: &[(Architecture::X64, PathBuf::from("pont-fd9-x64.exe"))],
        },
        |programme: &Path, _: &Path, _| -> Result<Simule, Panne> {
            lances.push(programme.to_path_buf());
            Ok(Simule::Fd9(Box::new(PontFd9Simule::avec_un_fd9())))
        },
        &mut regle_presente(),
    );
    assert!(lances.is_empty());
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PontIntrouvable { detail }) if detail.contains("pont-fd9")
    ));
}

#[test]
fn sans_logiciel_myiro1_une_detection_fd9_en_erreur_est_montree() {
    let instrument = ouvrir_l_un_ou_l_autre(
        Recherche {
            emplacements: &[dossier_vide("fd9-erreur-myiro1")],
            ponts: &ponts_myiro1(),
        },
        Recherche {
            emplacements: &[fd_s2w("fd9-erreur")],
            ponts: &ponts_fd9(),
        },
        |_: &Path, _: &Path, _| -> Result<Simule, Panne> {
            let mut simule = PontFd9Simule::avec_un_fd9();
            simule.detection = Some(Reponse::Erreur {
                erreur: ErreurPont::Sdk { code: 1002 },
            });
            Ok(Simule::Fd9(Box::new(simule)))
        },
        &mut regle_presente(),
    );
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::DetectionImpossible { .. })
    ));
}

#[test]
fn un_myiro1_en_echec_garde_son_probleme_devant_un_fd9_absent() {
    // Logiciel du MYIRO-1 présent mais aucun MYIRO-1 ; FD-S2w présent mais
    // aucun FD-9 : le problème du MYIRO-1, premier cherché, reste montré.
    let instrument = ouvrir_l_un_ou_l_autre(
        Recherche {
            emplacements: &[logiciel_myiro1("deux-absents-myiro1")],
            ponts: &ponts_myiro1(),
        },
        Recherche {
            emplacements: &[fd_s2w("deux-absents-fd9")],
            ponts: &ponts_fd9(),
        },
        |programme: &Path, _: &Path, _| -> Result<Simule, Panne> {
            Ok(if programme.to_string_lossy().contains("fd9") {
                Simule::Fd9(Box::default())
            } else {
                Simule::Myiro1(PontSimule::avec_instruments(&[]))
            })
        },
        &mut regle_presente(),
    );
    assert_eq!(instrument.probleme(), Some(&Probleme::AucunInstrument));
    assert!(instrument.sdk().is_some(), "le problème vient du MYIRO-1");
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

// ---- Pare-feu (ticket #47) : à la première détection vide, l'application
// demande à Windows d'ajouter sa règle, puis relance la détection une fois. ----

/// Une détection vide (réponse bloquée), puis le FD-9 une fois la règle ajoutée.
fn bloque_par_le_pare_feu() -> PontFd9Simule {
    PontFd9Simule {
        vides_d_abord: 1,
        ..PontFd9Simule::avec_un_fd9()
    }
}

fn regle_du_pont_x86() -> RegleFd9 {
    RegleFd9 {
        programme: PathBuf::from("pont-fd9-x86.exe"),
    }
}

#[test]
fn a_la_premiere_detection_vide_la_regle_est_demandee_puis_la_detection_relancee() {
    let simule = bloque_par_le_pare_feu();
    let journal = simule.journal.clone();
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule::default());

    let instrument = ouvrir_fd9_avec(simule, &fd_s2w("pare-feu-ouvert"), &mut autorisation);

    assert_eq!(
        instrument.etat(),
        &Etat::Detecte {
            modele: "FD-9".into(),
            identifiant: Info::Supposee(IDENTIFIANT.into()),
        }
    );
    assert_eq!(instrument.probleme(), None);
    // Une seule demande, pour le pont lancé, et une seule nouvelle détection.
    assert_eq!(autorisation.pare_feu().demandes, vec![regle_du_pont_x86()]);
    assert_eq!(
        *journal.borrow(),
        vec![
            Requete::Version {},
            Requete::Detecter {},
            Requete::Detecter {}
        ]
    );
}

#[test]
fn une_regle_deja_presente_n_est_jamais_redemandee() {
    let mut simule = PontFd9Simule::avec_un_fd9();
    simule.liste.clear();
    let journal = simule.journal.clone();
    let mut autorisation = regle_presente();

    let instrument = ouvrir_fd9_avec(simule, &fd_s2w("regle-presente"), &mut autorisation);

    assert!(autorisation.pare_feu().demandes.is_empty());
    assert_eq!(
        *journal.borrow(),
        vec![Requete::Version {}, Requete::Detecter {}]
    );
    // Le pare-feu n'est plus en cause : l'avis ordinaire, sans bouton.
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::AucunFd9 { .. })
    ));
    assert!(!instrument.vue().probleme.unwrap().autoriser_pare_feu);
}

#[test]
fn la_regle_ajoutee_sans_fd9_visible_n_est_pas_redemandee() {
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule::default());
    for essai in ["sans-fd9-1", "sans-fd9-2"] {
        let mut simule = PontFd9Simule::avec_un_fd9();
        simule.liste.clear();
        let instrument = ouvrir_fd9_avec(simule, &fd_s2w(essai), &mut autorisation);
        assert!(matches!(
            instrument.probleme(),
            Some(Probleme::AucunFd9 { .. })
        ));
    }
    assert_eq!(autorisation.pare_feu().demandes, vec![regle_du_pont_x86()]);
}

#[test]
fn un_refus_donne_un_probleme_redige_sans_nouvelle_demande_automatique() {
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        echec: Some(EchecPareFeu::Refuse),
        ..Default::default()
    });
    let simule = bloque_par_le_pare_feu();
    let journal = simule.journal.clone();

    let instrument = ouvrir_fd9_avec(simule, &fd_s2w("refus"), &mut autorisation);

    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PareFeuFerme { .. })
    ));
    // Pas de nouvelle détection après un refus.
    assert_eq!(
        *journal.borrow(),
        vec![Requete::Version {}, Requete::Detecter {}]
    );
    let vue = instrument.vue().probleme.unwrap();
    assert_eq!(vue.code, "pare_feu_ferme");
    assert_eq!(vue.ecran, "non_detecte");
    assert!(vue.autoriser_pare_feu);
    assert!(!vue.guide_cablage);
    // Le détail replié donne la commande équivalente, et comment la retirer.
    let detail = vue.detail.unwrap();
    assert!(
        detail.contains(&regle_du_pont_x86().commande_ajout()),
        "{detail}"
    );
    assert!(detail.contains(&RegleFd9::commande_retrait()), "{detail}");

    // « Réessayer » : la règle manque toujours, mais rien n'est redemandé.
    let encore = ouvrir_fd9_avec(
        bloque_par_le_pare_feu(),
        &fd_s2w("refus-encore"),
        &mut autorisation,
    );
    assert!(matches!(
        encore.probleme(),
        Some(Probleme::PareFeuFerme { .. })
    ));
    assert_eq!(autorisation.pare_feu().demandes.len(), 1);
}

#[test]
fn un_echec_de_l_ajout_est_rapporte_avec_son_detail() {
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        echec: Some(EchecPareFeu::Echec {
            detail: "netsh : code 1".into(),
        }),
        ..Default::default()
    });
    let instrument = ouvrir_fd9_avec(
        bloque_par_le_pare_feu(),
        &fd_s2w("echec"),
        &mut autorisation,
    );
    assert!(matches!(
        instrument.probleme(),
        Some(Probleme::PareFeuFerme { detail }) if detail.contains("netsh : code 1")
    ));
}

#[test]
fn le_bouton_autoriser_redemande_une_fois_apres_un_refus() {
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        echec: Some(EchecPareFeu::Refuse),
        ..Default::default()
    });
    ouvrir_fd9_avec(
        bloque_par_le_pare_feu(),
        &fd_s2w("bouton-1"),
        &mut autorisation,
    );
    // L'opérateur accepte cette fois, depuis le bouton.
    autorisation.pare_feu_mut().echec = None;
    autorisation.autoriser_a_nouveau();
    let instrument = ouvrir_fd9_avec(
        bloque_par_le_pare_feu(),
        &fd_s2w("bouton-2"),
        &mut autorisation,
    );
    assert_eq!(instrument.vue().etat, "detecte");
    assert_eq!(autorisation.pare_feu().demandes.len(), 2);
}

#[test]
fn une_regle_qui_vise_un_autre_programme_est_remplacee_une_fois() {
    // Règle créée en développement, puis application installée : le pont
    // lancé n'est plus celui de la règle, la réponse reste bloquée.
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        regle: Some(PathBuf::from(
            r"C:\depot\target\i686-pc-windows-msvc\debug\pont-fd9.exe",
        )),
        ..Default::default()
    });
    let instrument = ouvrir_fd9_avec(
        bloque_par_le_pare_feu(),
        &fd_s2w("autre-programme"),
        &mut autorisation,
    );
    assert_eq!(instrument.vue().etat, "detecte");
    assert_eq!(autorisation.pare_feu().demandes, vec![regle_du_pont_x86()]);
    // Une seule règle de ce nom, pour le pont lancé.
    assert_eq!(
        autorisation.pare_feu().regle,
        Some(PathBuf::from("pont-fd9-x86.exe"))
    );

    // Ensuite, elle vise le bon programme : rien n'est redemandé.
    let mut vide = PontFd9Simule::avec_un_fd9();
    vide.liste.clear();
    let encore = ouvrir_fd9_avec(vide, &fd_s2w("autre-programme-2"), &mut autorisation);
    assert!(matches!(encore.probleme(), Some(Probleme::AucunFd9 { .. })));
    assert_eq!(autorisation.pare_feu().demandes.len(), 1);
}

#[test]
fn le_choix_des_ponts_passe_l_autorisation_au_fd9() {
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule::default());
    let instrument = ouvrir_l_un_ou_l_autre(
        Recherche {
            emplacements: &[dossier_vide("choix-pare-feu-myiro1")],
            ponts: &ponts_myiro1(),
        },
        Recherche {
            emplacements: &[fd_s2w("choix-pare-feu")],
            ponts: &ponts_fd9(),
        },
        |_: &Path, _: &Path, _| -> Result<Simule, Panne> {
            Ok(Simule::Fd9(Box::new(bloque_par_le_pare_feu())))
        },
        &mut autorisation,
    );
    assert_eq!(instrument.vue().etat, "detecte");
    assert_eq!(autorisation.pare_feu().demandes, vec![regle_du_pont_x86()]);
}

#[test]
fn la_regle_est_entrante_udp_49152_reseau_local_pour_le_seul_pont() {
    let regle = RegleFd9 {
        programme: PathBuf::from(r"C:\Program Files\myiro-libre\pont-fd9-x86.exe"),
    };
    assert_eq!(
        regle.arguments_ajout(),
        "advfirewall firewall add rule name=\"myiro-libre - FD-9 - detection (UDP 49152)\" \
         dir=in action=allow protocol=UDP localport=49152 remoteip=localsubnet profile=any \
         program=\"C:\\Program Files\\myiro-libre\\pont-fd9-x86.exe\""
    );
    assert_eq!(
        RegleFd9::arguments_lecture(),
        "advfirewall firewall show rule name=\"myiro-libre - FD-9 - detection (UDP 49152)\" verbose"
    );
    // Pose en une seule élévation : la règle de ce nom est retirée, puis une
    // seule est ajoutée. Jamais deux règles du même nom.
    assert_eq!(
        regle.arguments_pose(Path::new(r"C:\Windows\System32\netsh.exe")),
        "/d /s /c \"\"C:\\Windows\\System32\\netsh.exe\" advfirewall firewall delete rule \
         name=\"myiro-libre - FD-9 - detection (UDP 49152)\" & \
         \"C:\\Windows\\System32\\netsh.exe\" advfirewall firewall add rule \
         name=\"myiro-libre - FD-9 - detection (UDP 49152)\" dir=in action=allow protocol=UDP \
         localport=49152 remoteip=localsubnet profile=any \
         program=\"C:\\Program Files\\myiro-libre\\pont-fd9-x86.exe\"\""
    );
    assert_eq!(
        RegleFd9::commande_retrait(),
        "netsh advfirewall firewall delete rule name=\"myiro-libre - FD-9 - detection (UDP 49152)\""
    );
}

#[test]
fn le_probleme_du_pare_feu_est_explique_en_francais_et_en_anglais() {
    for (langue, mot) in [
        (Langue::Francais, "pare-feu"),
        (Langue::Anglais, "firewall"),
    ] {
        let cause = texte(langue, "probleme.pare_feu_ferme.cause");
        let action = texte(langue, "probleme.pare_feu_ferme.action");
        let bouton = texte(langue, "ecran.autoriser_pare_feu");
        assert!(cause.contains(mot), "{cause}");
        assert!(action.contains(bouton), "{action} / {bouton}");
    }
    assert_eq!(
        texte(Langue::Francais, "ecran.autoriser_pare_feu"),
        "Autoriser le FD-9 dans le pare-feu"
    );
}
