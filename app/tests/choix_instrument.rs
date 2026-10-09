//! Choisir l'instrument dans la barre (ticket #49) : le module `instrument`
//! porte la liste des instruments présents sur le poste, l'instrument actif,
//! le changement (fermeture du pont en cours, puis ouverture de l'autre), le
//! dernier choix retenu et le refus pendant une opération en cours. Contre le
//! pont simulé du MYIRO-1 (`PontSimule`), un pont FD-9 simulé propre à ces
//! tests et le pare-feu simulé du ticket #47.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use app::instrument::choix::{
    lire_choix, retenir_choix, Modele, Occupation, Operation, Ouverture, Recherche, Refus,
    Selection,
};
use app::instrument::parefeu::{AutorisationPareFeu, PareFeuSimule};
use app::instrument::{Accord, Etat, Geste};
use app::pont::{Architecture, Panne, Pont, PontSimule};
use app::textes::{cles, texte, Langue};
use pont_protocole::{
    ErreurPont, Info, InstrumentFd9, LiaisonFd9, Palier, RemiseAuRepos, Reponse, Requete,
};

/// Numéro de série, identifiant et adresse fictifs.
const SERIE: u32 = 12345678;
const IDENTIFIANT: &str = "12345678";
const ADRESSE: &str = "192.0.2.40";

fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("choix-instrument")
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

/// Pont FD-9 simulé : répond comme `pont-fd9` (version, détection, fermeture).
#[derive(Clone)]
struct PontFd9Simule {
    vus: bool,
    journal: Rc<RefCell<Vec<Requete>>>,
}

impl Pont for PontFd9Simule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.journal.borrow_mut().push(requete.clone());
        Ok(match requete {
            Requete::Version {} => Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Inconnue,
            },
            Requete::Detecter {} => Reponse::InstrumentsFd9 {
                liste: if self.vus {
                    vec![InstrumentFd9 {
                        liaison: Info::Confirmee(LiaisonFd9::Reseau),
                        adresse: ADRESSE.into(),
                        identifiant: Info::Supposee(IDENTIFIANT.into()),
                    }]
                } else {
                    Vec::new()
                },
            },
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

/// Pont lancé : celui du MYIRO-1 ou celui du FD-9, selon le programme.
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

/// Le poste simulé : logiciels installés, instruments allumés, et le journal
/// de tout ce que l'application a demandé aux ponts, dans l'ordre.
struct Poste {
    myiro1: Vec<PathBuf>,
    fd9: Vec<PathBuf>,
    ponts_myiro1: Vec<(Architecture, PathBuf)>,
    ponts_fd9: Vec<(Architecture, PathBuf)>,
    /// MYIRO-1 branchés (numéros de série fictifs).
    series: Vec<u32>,
    /// Un FD-9 répond sur le réseau.
    fd9_allume: bool,
    /// Réponse du pont MYIRO-1 à `fermer` (par défaut : celle d'un pont
    /// fermé juste après la connexion, repos supposé).
    fermeture_myiro1: Result<Reponse, Panne>,
    /// (pont lancé, requête) dans l'ordre.
    journal: Rc<RefCell<Vec<(&'static str, Requete)>>>,
    pare_feu: AutorisationPareFeu<PareFeuSimule>,
}

impl Poste {
    /// Les deux logiciels installés, les deux instruments présents.
    fn deux_instruments(nom: &str) -> Poste {
        let myiro1 = dossier_vide(&format!("{nom}-myiro1"));
        fausse_dll(&myiro1.join("FDXSDK.dll"), Architecture::X64);
        let fd9 = dossier_vide(&format!("{nom}-fd9"));
        fausse_dll(&fd9.join("Module").join("FD9SDK.dll"), Architecture::X86);
        Poste {
            myiro1: vec![myiro1],
            fd9: vec![fd9],
            ponts_myiro1: vec![(Architecture::X64, PathBuf::from("pont-myiro1-x64.exe"))],
            ponts_fd9: vec![(Architecture::X86, PathBuf::from("pont-fd9-x86.exe"))],
            series: vec![SERIE],
            fd9_allume: true,
            fermeture_myiro1: Ok(Reponse::FermetureIncertaine {
                remise_au_repos: RemiseAuRepos::ReposSuppose {},
            }),
            journal: Rc::default(),
            pare_feu: AutorisationPareFeu::new(PareFeuSimule {
                regle: Some(PathBuf::from("pont-fd9-x86.exe")),
                ..Default::default()
            }),
        }
    }

    /// Seul le logiciel du MYIRO-1 est installé.
    fn myiro1_seul(nom: &str) -> Poste {
        let mut poste = Poste::deux_instruments(nom);
        poste.fd9 = vec![dossier_vide(&format!("{nom}-sans-fd9"))];
        poste
    }

    fn requetes(&self) -> Vec<(&'static str, Requete)> {
        self.journal.borrow().clone()
    }

    fn vider_journal(&self) {
        self.journal.borrow_mut().clear();
    }

    /// Ce que l'application donne au module pour ouvrir un instrument.
    fn ouverture(
        &mut self,
    ) -> Ouverture<
        '_,
        impl FnMut(&Path, &Path, Palier) -> Result<Journalise, Panne> + '_,
        PareFeuSimule,
    > {
        let journal = self.journal.clone();
        let series = self.series.clone();
        let fd9_allume = self.fd9_allume;
        let fermeture = self.fermeture_myiro1.clone();
        Ouverture {
            myiro1: Recherche {
                emplacements: &self.myiro1,
                ponts: &self.ponts_myiro1,
            },
            fd9: Recherche {
                emplacements: &self.fd9,
                ponts: &self.ponts_fd9,
            },
            lancer: move |programme: &Path, _: &Path, _: Palier| {
                Ok(if programme.to_string_lossy().contains("fd9") {
                    Simule::Fd9(PontFd9Simule {
                        vus: fd9_allume,
                        journal: Rc::default(),
                    })
                } else {
                    Simule::Myiro1(
                        PontSimule::avec_instruments(&series).fermer_par(fermeture.clone()),
                    )
                })
                .map(|pont| Journalise {
                    nom: if programme.to_string_lossy().contains("fd9") {
                        "fd9"
                    } else {
                        "myiro1"
                    },
                    pont,
                    journal: journal.clone(),
                })
            },
            pare_feu: &mut self.pare_feu,
        }
    }
}

/// Pont qui note chaque requête dans le journal du poste, avec son nom.
struct Journalise {
    nom: &'static str,
    pont: Simule,
    journal: Rc<RefCell<Vec<(&'static str, Requete)>>>,
}

impl Pont for Journalise {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.journal.borrow_mut().push((self.nom, requete.clone()));
        self.pont.demander(requete)
    }
}

fn lancer(poste: &mut Poste, retenu: Option<Modele>) -> Selection<Journalise> {
    Selection::lancer(retenu, &mut poste.ouverture())
}

/// Textes d'une ligne de la liste, tels que la page les compose.
fn ligne(selection: &Selection<Journalise>, langue: Langue, modele: Modele) -> String {
    let vue = selection.vue();
    let l = vue
        .instruments
        .iter()
        .find(|l| l.code == modele.code())
        .unwrap_or_else(|| panic!("{modele:?} absent de la liste"));
    texte(langue, "choix.ligne")
        .replace("{modele}", &l.modele)
        .replace(
            "{liaison}",
            texte(langue, &format!("choix.liaison.{}", l.liaison)),
        )
        .replace("{etat}", texte(langue, &format!("choix.etat.{}", l.etat)))
}

#[test]
fn la_liste_montre_les_deux_instruments_presents_avec_leur_etat() {
    let mut poste = Poste::deux_instruments("liste");
    let selection = lancer(&mut poste, None);

    let vue = selection.vue();
    assert_eq!(vue.instrument.modele.as_deref(), Some("MYIRO-1"));
    let codes: Vec<_> = vue.instruments.iter().map(|l| l.code).collect();
    assert_eq!(codes, vec!["myiro1", "fd9"]);
    assert!(vue.instruments[0].actif && !vue.instruments[1].actif);
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Myiro1),
        "MYIRO-1 (USB), étalonnage requis"
    );
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Fd9),
        "FD-9 (réseau), en attente"
    );
    assert_eq!(
        ligne(&selection, Langue::Anglais, Modele::Fd9),
        "FD-9 (network), on standby"
    );
    // Un seul pont lancé : celui du MYIRO-1.
    assert!(poste.requetes().iter().all(|(nom, _)| *nom == "myiro1"));
}

fn changer(
    selection: &mut Selection<Journalise>,
    poste: &mut Poste,
    vers: Modele,
) -> Result<(), Refus> {
    let occupation = Occupation::default();
    let jeton = occupation.commencer(Operation::Changement).unwrap();
    selection.changer(&jeton, vers, &mut poste.ouverture())
}

/// Texte de l'annonce, tel que la page le compose.
fn annonce(selection: &Selection<Journalise>, langue: Langue) -> String {
    let a = selection.vue().annonce.expect("une annonce");
    texte(langue, &format!("choix.annonce.{}", a.code))
        .replace("{modele}", &a.modele)
        .replace("{autre}", a.autre.as_deref().unwrap_or(""))
}

#[test]
fn passer_au_fd9_ferme_d_abord_le_myiro1_puis_ouvre_le_fd9_jusqu_a_la_detection() {
    let mut poste = Poste::deux_instruments("vers-fd9");
    let mut selection = lancer(&mut poste, None);
    poste.vider_journal();

    changer(&mut selection, &mut poste, Modele::Fd9).unwrap();

    // Fermeture du pont du MYIRO-1 d'abord, puis le pont du FD-9, sans aller
    // plus loin que la détection.
    assert_eq!(
        poste.requetes(),
        vec![
            ("myiro1", Requete::Fermer {}),
            ("fd9", Requete::Version {}),
            ("fd9", Requete::Detecter {}),
        ]
    );
    assert_eq!(selection.actif(), Modele::Fd9);
    assert_eq!(selection.choisi(), Some(Modele::Fd9));
    assert!(matches!(
        selection.instrument().etat(),
        Etat::Detecte { modele, .. } if modele == "FD-9"
    ));
    // Le FD-9 ne mesure pas : son pont s'arrête à la détection.
    assert!(!selection.vue().instrument.pret);
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Fd9),
        "FD-9 (réseau), détecté"
    );
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Myiro1),
        "MYIRO-1 (USB), en attente"
    );
    // Fermé juste après la connexion : repos supposé, et on le dit.
    let vue = selection.vue().annonce.unwrap();
    assert_eq!(vue.code, "ferme_repos_suppose");
    assert!(!vue.alerte);
    assert_eq!(
        annonce(&selection, Langue::Francais),
        "Le MYIRO-1 a été fermé. Aucune mesure n’était en cours\u{202f}: il devrait être au repos."
    );
}

#[test]
fn revenir_au_myiro1_ferme_le_fd9_et_redemande_l_etalonnage() {
    let mut poste = Poste::deux_instruments("retour-myiro1");
    let mut selection = lancer(&mut poste, Some(Modele::Fd9));
    poste.vider_journal();

    changer(&mut selection, &mut poste, Modele::Myiro1).unwrap();

    assert_eq!(
        poste.requetes(),
        vec![
            ("fd9", Requete::Fermer {}),
            ("myiro1", Requete::Version {}),
            ("myiro1", Requete::Detecter {}),
            ("myiro1", Requete::Connecter { instrument: 0 }),
        ]
    );
    // Un nouveau pont : l'étalonnage est à refaire, jamais demandé tout seul.
    assert!(matches!(
        selection.instrument().etat(),
        Etat::EtalonnageRequis(_)
    ));
    assert_eq!(selection.vue().annonce.unwrap().code, "ferme");
    assert_eq!(
        annonce(&selection, Langue::Anglais),
        "The FD-9 has been closed."
    );
}

#[test]
fn un_myiro1_etalonne_est_ferme_avant_le_passage_au_fd9() {
    let mut poste = Poste::deux_instruments("etalonne");
    let mut selection = lancer(&mut poste, Some(Modele::Myiro1));
    selection
        .instrument_mut()
        .etalonner(&mut |_: Geste| Accord::Fait);
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Myiro1),
        "MYIRO-1 (USB), étalonné"
    );
    poste.vider_journal();
    changer(&mut selection, &mut poste, Modele::Fd9).unwrap();
    assert_eq!(poste.requetes()[0], ("myiro1", Requete::Fermer {}));
    assert_eq!(selection.instrument().etalonnage(), None);
    assert_eq!(selection.actif(), Modele::Fd9);
}

#[test]
fn une_fermeture_incertaine_est_rapportee_en_alerte() {
    let mut poste = Poste::deux_instruments("incertaine");
    poste.fermeture_myiro1 = Ok(Reponse::FermetureIncertaine {
        remise_au_repos: RemiseAuRepos::ReposNonSignale {},
    });
    let mut selection = lancer(&mut poste, None);
    changer(&mut selection, &mut poste, Modele::Fd9).unwrap();
    let vue = selection.vue().annonce.unwrap();
    assert_eq!(vue.code, "ferme_incertain");
    assert!(vue.alerte);
    assert!(vue.detail.unwrap().contains("ReposNonSignale"));
    assert!(annonce(&selection, Langue::Francais).contains("débranchez-le puis rebranchez-le"));
    // Le FD-9 est ouvert quand même : le pont du MYIRO-1 est arrêté.
    assert_eq!(selection.actif(), Modele::Fd9);
}

#[test]
fn une_deconnexion_refusee_ou_un_pont_muet_sont_rapportes_sans_bloquer_le_changement() {
    for (nom, fermeture) in [
        (
            "deconnexion",
            Ok(Reponse::Erreur {
                erreur: ErreurPont::DeconnexionEchouee {
                    code: -1,
                    remise_au_repos: RemiseAuRepos::AuRepos {},
                },
            }),
        ),
        (
            "muet",
            Err(Panne::SansReponse {
                detail: "aucune réponse ; pont arrêté de force".into(),
            }),
        ),
    ] {
        let mut poste = Poste::deux_instruments(nom);
        poste.fermeture_myiro1 = fermeture;
        let mut selection = lancer(&mut poste, None);
        changer(&mut selection, &mut poste, Modele::Fd9).unwrap();
        let vue = selection.vue().annonce.unwrap();
        assert_eq!(vue.code, "ferme_echec", "{nom}");
        assert!(vue.alerte && vue.detail.is_some(), "{nom}");
        assert_eq!(selection.actif(), Modele::Fd9, "{nom}");
    }
}

#[test]
fn un_fd9_choisi_mais_eteint_montre_son_probleme_et_reste_choisi() {
    let mut poste = Poste::deux_instruments("fd9-eteint");
    poste.fd9_allume = false;
    let mut selection = lancer(&mut poste, None);
    changer(&mut selection, &mut poste, Modele::Fd9).unwrap();
    assert_eq!(selection.actif(), Modele::Fd9);
    assert_eq!(selection.choisi(), Some(Modele::Fd9));
    assert_eq!(
        selection.vue().instrument.probleme.unwrap().code,
        "aucun_fd9"
    );
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Fd9),
        "FD-9 (réseau), non trouvé"
    );
    // Le MYIRO-1 a bien été fermé : aucun pont ne reste ouvert à côté.
    assert_eq!(selection.vue().annonce.unwrap().code, "ferme_repos_suppose");
}

#[test]
fn un_instrument_sans_logiciel_sur_le_poste_n_est_pas_propose_et_refuse() {
    let mut poste = Poste::myiro1_seul("sans-fd9");
    let mut selection = lancer(&mut poste, None);
    let codes: Vec<_> = selection.vue().instruments.iter().map(|l| l.code).collect();
    assert_eq!(codes, vec!["myiro1"]);
    poste.vider_journal();
    assert_eq!(
        changer(&mut selection, &mut poste, Modele::Fd9),
        Err(Refus::Absent(Modele::Fd9))
    );
    assert!(poste.requetes().is_empty(), "rien n'est fermé ni lancé");
    assert_eq!(selection.actif(), Modele::Myiro1);
}

#[test]
fn rechoisir_l_instrument_en_service_ne_ferme_rien() {
    let mut poste = Poste::deux_instruments("meme");
    let mut selection = lancer(&mut poste, None);
    poste.vider_journal();
    changer(&mut selection, &mut poste, Modele::Myiro1).unwrap();
    assert!(poste.requetes().is_empty());
    assert_eq!(selection.choisi(), Some(Modele::Myiro1));
}

// ---- Dernier choix retenu, repris au lancement ----

#[test]
fn le_choix_est_retenu_dans_un_fichier_et_relu() {
    let fichier = dossier_vide("fichier")
        .join("config")
        .join("instrument.txt");
    assert_eq!(lire_choix(&fichier), None);
    retenir_choix(&fichier, Modele::Fd9).unwrap();
    assert_eq!(lire_choix(&fichier), Some(Modele::Fd9));
    retenir_choix(&fichier, Modele::Myiro1).unwrap();
    assert_eq!(lire_choix(&fichier), Some(Modele::Myiro1));
    std::fs::write(&fichier, "autre chose").unwrap();
    assert_eq!(lire_choix(&fichier), None);
}

#[test]
fn au_lancement_le_choix_retenu_est_ouvert_seul() {
    let mut poste = Poste::deux_instruments("retenu-fd9");
    let selection = lancer(&mut poste, Some(Modele::Fd9));
    assert_eq!(selection.actif(), Modele::Fd9);
    assert!(poste.requetes().iter().all(|(nom, _)| *nom == "fd9"));
    assert_eq!(selection.vue().annonce, None);
    assert!(!selection.vue().montrer_liste);
}

#[test]
fn sans_choix_retenu_l_instrument_present_est_pris_et_on_le_dit() {
    let mut poste = Poste::deux_instruments("sans-choix");
    let selection = lancer(&mut poste, None);
    assert_eq!(selection.choisi(), None);
    assert_eq!(
        annonce(&selection, Langue::Francais),
        "Aucun instrument n’avait été choisi\u{202f}: le MYIRO-1, trouvé sur ce poste, est en service. Choisissez ici celui que vous voulez utiliser."
    );
    // Deux instruments présents : la liste s'ouvre pour que ce soit vu.
    assert!(selection.vue().montrer_liste);

    let mut seul = Poste::myiro1_seul("sans-choix-seul");
    let selection = lancer(&mut seul, None);
    assert_eq!(selection.vue().annonce.unwrap().code, "sans_choix");
    assert!(!selection.vue().montrer_liste, "rien d'autre à choisir");
}

#[test]
fn un_choix_retenu_introuvable_cede_la_place_a_l_autre_et_on_le_dit() {
    let mut poste = Poste::deux_instruments("retenu-eteint");
    poste.fd9_allume = false;
    let selection = lancer(&mut poste, Some(Modele::Fd9));
    assert_eq!(selection.actif(), Modele::Myiro1);
    // Le choix de l'opérateur reste le sien.
    assert_eq!(selection.choisi(), Some(Modele::Fd9));
    assert_eq!(
        annonce(&selection, Langue::Francais),
        "Le FD-9, choisi la dernière fois, n’a pas été trouvé\u{202f}: le MYIRO-1 est en service à sa place."
    );
    assert!(selection.vue().montrer_liste);
    assert_eq!(
        ligne(&selection, Langue::Francais, Modele::Fd9),
        "FD-9 (réseau), non trouvé la dernière fois"
    );
    // Le pont du FD-9 a été essayé avant celui du MYIRO-1, jamais en même temps.
    let noms: Vec<_> = poste.requetes().iter().map(|(n, _)| *n).collect();
    let premier_myiro1 = noms.iter().position(|n| *n == "myiro1").unwrap();
    assert!(noms[..premier_myiro1].iter().all(|n| *n == "fd9"));
}

#[test]
fn un_choix_retenu_introuvable_sans_autre_instrument_garde_son_probleme() {
    let mut poste = Poste::deux_instruments("aucun");
    poste.fd9_allume = false;
    poste.series.clear();
    let selection = lancer(&mut poste, Some(Modele::Fd9));
    assert_eq!(selection.actif(), Modele::Fd9);
    assert_eq!(
        selection.vue().instrument.probleme.unwrap().code,
        "aucun_fd9"
    );
    assert_eq!(selection.vue().annonce, None);
}

// ---- Une seule opération à la fois ----

#[test]
fn un_changement_pendant_un_etalonnage_ou_une_mesure_est_refuse_avec_sa_raison() {
    let occupation = Occupation::default();
    for operation in [Operation::Etalonnage, Operation::Mesure] {
        let jeton = occupation.commencer(operation).unwrap();
        let refus = occupation
            .commencer(Operation::Changement)
            .err()
            .expect("refusé");
        assert_eq!(refus, Refus::Occupe(operation));
        assert_eq!(occupation.en_cours(), Some(operation));
        drop(jeton);
    }
    assert_eq!(occupation.en_cours(), None);
    assert!(occupation.commencer(Operation::Changement).is_ok());
    assert_eq!(
        texte(
            Langue::Francais,
            &Refus::Occupe(Operation::Etalonnage).cle()
        ),
        "Un étalonnage est en cours\u{202f}: attendez qu’il se termine."
    );
}

#[test]
fn un_etalonnage_pendant_un_changement_est_refuse_aussi() {
    let occupation = Occupation::default();
    let _changement = occupation.commencer(Operation::Changement).unwrap();
    assert_eq!(
        occupation.commencer(Operation::Etalonnage).err(),
        Some(Refus::Occupe(Operation::Changement))
    );
}

#[test]
fn chaque_refus_et_chaque_annonce_ont_leur_texte_en_francais_et_en_anglais() {
    let mut attendues: Vec<String> = [
        Operation::Recherche,
        Operation::Etalonnage,
        Operation::Mesure,
        Operation::Changement,
    ]
    .into_iter()
    .map(|o| Refus::Occupe(o).cle())
    .collect();
    attendues.push(Refus::Absent(Modele::Fd9).cle());
    for code in [
        "sans_choix",
        "remplace",
        "ferme",
        "ferme_repos_suppose",
        "ferme_incertain",
        "ferme_echec",
    ] {
        attendues.push(format!("choix.annonce.{code}"));
    }
    for etat in [
        "non_detecte",
        "detecte",
        "connecte",
        "etalonnage_requis",
        "etalonne",
        "en_attente",
        "non_trouve",
    ] {
        attendues.push(format!("choix.etat.{etat}"));
    }
    for cle in &attendues {
        assert!(cles().any(|c| c == cle), "{cle} absente");
        assert_ne!(
            texte(Langue::Francais, cle),
            texte(Langue::Anglais, cle),
            "{cle}"
        );
    }
}

// ---- « Réessayer » ----

#[test]
fn reessayer_rouvre_l_instrument_choisi_seulement() {
    let mut poste = Poste::deux_instruments("reessayer");
    poste.fd9_allume = false;
    let mut selection = lancer(&mut poste, None);
    changer(&mut selection, &mut poste, Modele::Fd9).unwrap();
    poste.vider_journal();
    poste.fd9_allume = true;
    let occupation = Occupation::default();
    let jeton = occupation.commencer(Operation::Recherche).unwrap();
    selection.rouvrir(&jeton, &mut poste.ouverture());
    assert!(poste.requetes().iter().all(|(nom, _)| *nom == "fd9"));
    assert_eq!(selection.vue().instrument.etat, "detecte");
}

#[test]
fn reessayer_sans_choix_cherche_comme_au_lancement() {
    let mut poste = Poste::deux_instruments("reessayer-sans-choix");
    poste.series.clear();
    poste.fd9_allume = false;
    let mut selection = lancer(&mut poste, None);
    assert_eq!(
        selection.vue().instrument.probleme.unwrap().code,
        "aucun_instrument"
    );
    poste.fd9_allume = true;
    let occupation = Occupation::default();
    let jeton = occupation.commencer(Operation::Recherche).unwrap();
    selection.rouvrir(&jeton, &mut poste.ouverture());
    assert_eq!(selection.actif(), Modele::Fd9);
    assert_eq!(selection.vue().instrument.etat, "detecte");
}
