//! Seam entre l'application et le processus pont (ADR 0005) : le trait `Pont`,
//! le vrai processus `pont-myiro1` (`PontProcessus`) et un pont simulé à
//! échecs injectables (`PontSimule`). Rien ici ne dépend de Tauri.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pont_protocole::{
    lire_reponse, ErreurPont, Identite, InstrumentDetecte, Palier, Reponse, Requete,
};

/// Le pont n'a pas pu répondre : ce n'est pas une erreur de l'instrument, mais
/// du programme pont lui-même.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Panne {
    /// Le programme pont n'a pas pu être lancé (introuvable, refusé).
    Lancement { detail: String },
    /// Le pont s'est arrêté au démarrage : la DLL indiquée n'a pas pu être
    /// chargée (absente, d'une autre architecture ou incomplète).
    DllRefusee { detail: String },
    /// Le pont s'est arrêté sans répondre.
    Arret { code: Option<i32>, detail: String },
    /// Le pont a écrit une ligne qui n'est pas une réponse du protocole.
    ReponseIllisible { detail: String },
}

/// Une requête, une réponse : tout le dialogue avec un pont.
pub trait Pont {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne>;
}

/// Code de sortie de `pont-myiro1` quand la DLL n'a pas pu être chargée.
const CODE_DLL_REFUSEE: i32 = 3;

/// Temps laissé au pont pour désarmer et se déconnecter après la fermeture de
/// son entrée, avant de l'arrêter de force.
const DELAI_ARRET: Duration = Duration::from_secs(10);

/// Le vrai programme `pont-myiro1`, lancé avec la DLL et le plafond, auquel on
/// parle par une ligne JSON par message (crate `pont-protocole`).
pub struct PontProcessus {
    enfant: Child,
    entree: Option<ChildStdin>,
    sortie: BufReader<ChildStdout>,
}

impl PontProcessus {
    pub fn lancer(programme: &Path, dll: &Path, plafond: Palier) -> Result<Self, Panne> {
        let mut commande = Command::new(programme);
        commande
            .arg("--dll")
            .arg(dll)
            .arg("--plafond")
            .arg(nom_palier(plafond))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            // Pas de fenêtre de console derrière l'application.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            commande.creation_flags(CREATE_NO_WINDOW);
        }
        let mut enfant = commande.spawn().map_err(|e| Panne::Lancement {
            detail: format!("{} : {e}", programme.display()),
        })?;
        let entree = enfant.stdin.take();
        let sortie = BufReader::new(enfant.stdout.take().expect("sortie du pont"));
        Ok(PontProcessus {
            enfant,
            entree,
            sortie,
        })
    }

    /// Le pont ne répond plus : on attend sa fin et on lit ce qu'il a dit.
    fn arret(&mut self) -> Panne {
        self.entree = None;
        let code = self.enfant.wait().ok().and_then(|s| s.code());
        let mut detail = String::new();
        if let Some(mut erreurs) = self.enfant.stderr.take() {
            let _ = erreurs.read_to_string(&mut detail);
        }
        let detail = detail.trim().to_string();
        match code {
            Some(CODE_DLL_REFUSEE) => Panne::DllRefusee { detail },
            code => Panne::Arret { code, detail },
        }
    }
}

/// Nom du palier tel que `pont-myiro1 --plafond` l'attend.
fn nom_palier(palier: Palier) -> String {
    serde_json::to_value(palier)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .expect("un palier se nomme toujours")
}

impl Pont for PontProcessus {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        let ligne = serde_json::to_string(requete).expect("une requête se sérialise toujours");
        let envoi = match self.entree.as_mut() {
            Some(entree) => writeln!(entree, "{ligne}").and_then(|_| entree.flush()),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        if envoi.is_err() {
            return Err(self.arret());
        }
        let mut reponse = String::new();
        match self.sortie.read_line(&mut reponse) {
            Ok(0) | Err(_) => Err(self.arret()),
            Ok(_) => lire_reponse(reponse.trim()).map_err(|e| Panne::ReponseIllisible {
                detail: format!("{e} : {}", reponse.trim()),
            }),
        }
    }
}

impl Drop for PontProcessus {
    /// Fermer l'entrée termine la boucle du pont, qui désarme et se déconnecte.
    /// Passé le délai, il est arrêté de force : cela ne prouve pas que
    /// l'instrument est revenu au repos.
    fn drop(&mut self) {
        self.entree = None;
        let echeance = Instant::now() + DELAI_ARRET;
        while Instant::now() < echeance {
            match self.enfant.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        let _ = self.enfant.kill();
        let _ = self.enfant.wait();
    }
}

/// Requêtes reçues par un pont simulé, lisibles après coup par les tests.
#[derive(Clone, Default)]
pub struct Journal(Arc<Mutex<Vec<Requete>>>);

impl Journal {
    pub fn requetes(&self) -> Vec<Requete> {
        self.0.lock().unwrap().clone()
    }
}

/// Pont simulé : répond comme `pont-myiro1` avec les instruments qu'on lui
/// donne, et rend à la demande une panne ou une erreur à un palier donné.
/// Il ne dit rien de l'ABI de la DLL : seul l'instrument réel la confirme.
pub struct PontSimule {
    instruments: Vec<u32>,
    echecs: BTreeMap<Palier, Result<Reponse, Panne>>,
    journal: Journal,
}

impl PontSimule {
    /// Instruments branchés, par leur numéro de série (toujours fictif).
    pub fn avec_instruments(series: &[u32]) -> Self {
        PontSimule {
            instruments: series.to_vec(),
            echecs: BTreeMap::new(),
            journal: Journal::default(),
        }
    }

    /// Au palier donné, rend ce résultat au lieu de la réponse normale.
    pub fn echouer_a(mut self, palier: Palier, resultat: Result<Reponse, Panne>) -> Self {
        self.echecs.insert(palier, resultat);
        self
    }

    pub fn journal(&self) -> Journal {
        self.journal.clone()
    }
}

impl Pont for PontSimule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.journal.0.lock().unwrap().push(requete.clone());
        let palier = match requete {
            Requete::Version {} => Palier::Version,
            Requete::Detecter {} => Palier::Detection,
            Requete::Connecter { .. } => Palier::Connexion,
            Requete::Etalonner {} => Palier::Etalonnage,
            Requete::MesurerPonctuelle {} => Palier::MesurePonctuelle,
            Requete::MesurerBande { .. } => Palier::Bande,
            Requete::Fermer {} => return Ok(Reponse::Ferme {}),
        };
        if let Some(resultat) = self.echecs.get(&palier) {
            return resultat.clone();
        }
        Ok(match requete {
            Requete::Version {} => Reponse::Version { parties: [1, 0, 1] },
            Requete::Detecter {} => Reponse::Instruments {
                liste: self
                    .instruments
                    .iter()
                    .enumerate()
                    .map(|(n, serie)| InstrumentDetecte {
                        liaison: "usb".into(),
                        port: format!("COM{}", n + 3),
                        numero_serie: *serie,
                    })
                    .collect(),
            },
            Requete::Connecter { instrument } => match self.instruments.get(*instrument as usize) {
                Some(serie) => Reponse::Connecte {
                    identite: Identite {
                        numero_serie: *serie,
                        micrologiciel: "1.00".into(),
                        code_produit: "simule".into(),
                        adresse_mac: "00:00:00:00:00:00".into(),
                        date_initiale: None,
                        anomalie_date_initiale: false,
                        brute_hex: String::new(),
                    },
                },
                None => Reponse::Erreur {
                    erreur: ErreurPont::InstrumentInconnu,
                },
            },
            // Le simulé ne va pas plus loin que la connexion, comme le plafond
            // de ce ticket.
            _ => Reponse::Erreur {
                erreur: ErreurPont::PalierNonAutorise {
                    demande: palier,
                    plafond: Palier::Connexion,
                },
            },
        })
    }
}
