//! Seam entre l'application et le processus pont (ADR 0005) : le trait `Pont`,
//! le vrai processus `pont-myiro1` (`PontProcessus`) et un pont simulé à
//! échecs injectables (`PontSimule`). Rien ici ne dépend de Tauri.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
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
    /// Le pont n'a pas répondu dans le délai ; il a été arrêté de force, ce
    /// qui ne prouve pas que l'instrument est revenu au repos.
    SansReponse { detail: String },
}

/// Une requête, une réponse : tout le dialogue avec un pont.
pub trait Pont {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne>;
}

/// Architecture d'un exécutable ou d'une DLL Windows. Un pont ne charge
/// qu'une DLL de sa propre architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Architecture {
    X64,
    X86,
}

/// Lit le type de machine dans l'en-tête PE d'un fichier, sans le charger.
pub fn architecture(fichier: &Path) -> Option<Architecture> {
    let mut f = std::fs::File::open(fichier).ok()?;
    let mut entete = [0u8; 0x40];
    f.read_exact(&mut entete).ok()?;
    if &entete[..2] != b"MZ" {
        return None;
    }
    let decalage = u32::from_le_bytes(entete[0x3c..0x40].try_into().ok()?);
    let mut pe = [0u8; 6];
    std::io::Seek::seek(&mut f, std::io::SeekFrom::Start(decalage.into())).ok()?;
    f.read_exact(&mut pe).ok()?;
    if &pe[..4] != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes([pe[4], pe[5]]) {
        0x014c => Some(Architecture::X86),
        0x8664 => Some(Architecture::X64),
        _ => None,
    }
}

/// Cherche les ponts livrés avec l'application, à partir du dossier de son
/// exécutable : d'abord les ponts installés à côté (`pont-myiro1-x64.exe`,
/// `pont-myiro1-x86.exe`), puis, en développement, `pont-myiro1.exe` dans le
/// même dossier `target/<profil>` et dans `target/<cible>/<profil>`.
/// L'architecture est lue dans l'en-tête ; un seul pont par architecture.
pub fn chercher_ponts(dossier_exe: &Path) -> Vec<(Architecture, PathBuf)> {
    let nom = |base: &str| format!("{base}{}", std::env::consts::EXE_SUFFIX);
    let mut candidats = vec![
        dossier_exe.join(nom("pont-myiro1-x64")),
        dossier_exe.join(nom("pont-myiro1-x86")),
        dossier_exe.join(nom("pont-myiro1")),
    ];
    if let (Some(profil), Some(target)) = (dossier_exe.file_name(), dossier_exe.parent()) {
        for cible in ["x86_64-pc-windows-msvc", "i686-pc-windows-msvc"] {
            candidats.push(target.join(cible).join(profil).join(nom("pont-myiro1")));
        }
    }
    let mut ponts: Vec<(Architecture, PathBuf)> = Vec::new();
    for chemin in candidats {
        if let Some(arch) = architecture(&chemin) {
            if !ponts.iter().any(|(a, _)| *a == arch) {
                ponts.push((arch, chemin));
            }
        }
    }
    ponts.sort_by_key(|(a, _)| *a);
    ponts
}

/// Code de sortie de `pont-myiro1` quand la DLL n'a pas pu être chargée.
const CODE_DLL_REFUSEE: i32 = 3;

/// Délai de réponse du pont. La connexion la plus lente attend 10 s dans la
/// DLL (`pont_myiro1::DELAI_CONNEXION`), plus la lecture de l'identité.
pub const DELAI_REPONSE: Duration = Duration::from_secs(30);

/// Le vrai programme `pont-myiro1`, lancé avec la DLL et le plafond, auquel on
/// parle par une ligne JSON par message (crate `pont-protocole`). Chaque
/// réponse est attendue au plus `delai` ; au-delà, le pont est arrêté de force.
pub struct PontProcessus {
    enfant: Child,
    entree: Option<ChildStdin>,
    /// Lignes lues par un fil à part, pour pouvoir attendre avec un délai.
    lignes: Receiver<String>,
    delai: Duration,
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
        let (envoi, lignes) = mpsc::channel();
        std::thread::spawn(move || {
            for ligne in sortie.lines() {
                let Ok(ligne) = ligne else { break };
                if envoi.send(ligne).is_err() {
                    break;
                }
            }
        });
        Ok(PontProcessus {
            enfant,
            entree,
            lignes,
            delai: DELAI_REPONSE,
        })
    }

    /// Change le délai de réponse (et d'arrêt) du pont.
    pub fn avec_delai(mut self, delai: Duration) -> Self {
        self.delai = delai;
        self
    }

    /// Attend la fin du pont au plus `delai`, puis l'arrête de force. Rend
    /// `None` si le pont a dû être arrêté de force.
    fn attendre_fin(&mut self) -> Option<ExitStatus> {
        self.entree = None;
        let echeance = Instant::now() + self.delai;
        loop {
            match self.enfant.try_wait() {
                Ok(Some(statut)) => return Some(statut),
                Ok(None) if Instant::now() < echeance => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => {
                    let _ = self.enfant.kill();
                    let _ = self.enfant.wait();
                    return None;
                }
            }
        }
    }

    /// Le pont a fermé sa sortie : on attend sa fin et on lit ce qu'il a dit.
    fn arret(&mut self) -> Panne {
        let Some(statut) = self.attendre_fin() else {
            return Panne::SansReponse {
                detail: "sortie du pont fermée sans fin du processus ; arrêté de force".into(),
            };
        };
        let mut detail = String::new();
        if let Some(mut erreurs) = self.enfant.stderr.take() {
            let _ = erreurs.read_to_string(&mut detail);
        }
        let detail = detail.trim().to_string();
        match statut.code() {
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
        match self.lignes.recv_timeout(self.delai) {
            Ok(reponse) => lire_reponse(reponse.trim()).map_err(|e| Panne::ReponseIllisible {
                detail: format!("{e} : {}", reponse.trim()),
            }),
            Err(RecvTimeoutError::Disconnected) => Err(self.arret()),
            Err(RecvTimeoutError::Timeout) => {
                self.entree = None;
                let _ = self.enfant.kill();
                let _ = self.enfant.wait();
                Err(Panne::SansReponse {
                    detail: format!(
                        "aucune réponse en {} ms à {ligne} ; pont arrêté de force",
                        self.delai.as_millis()
                    ),
                })
            }
        }
    }
}

impl Drop for PontProcessus {
    /// Fermer l'entrée termine la boucle du pont, qui désarme et se déconnecte.
    /// Passé le délai, il est arrêté de force : cela ne prouve pas que
    /// l'instrument est revenu au repos.
    fn drop(&mut self) {
        self.attendre_fin();
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
                    erreur: ErreurPont::InstrumentInconnu {},
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
