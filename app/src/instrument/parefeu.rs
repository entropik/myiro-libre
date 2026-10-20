//! Le pare-feu de Windows et la détection du FD-9 (ticket #47).
//!
//! La DLL du FD-9 lit la réponse à sa diffusion sur le port UDP 49152, depuis
//! une autre socket que celle qui envoie : sans règle entrante pour le pont,
//! Windows bloque cette réponse et la liste revient vide, sans erreur (cause
//! confirmée le 9 octobre 2026, fiche `docs/abi/FD9_GetDeviceList.md`).
//!
//! Le module `instrument` décide quand demander la règle ; ce fichier la
//! décrit, et le trait [`PareFeu`] la vérifie et l'ajoute. Deux adapters :
//! [`PareFeuWindows`] (lecture par `netsh` sans droits, ajout par la fenêtre
//! de contrôle de compte de Windows) et [`PareFeuSimule`] pour les tests, qui
//! ne touche jamais au vrai pare-feu.

use std::path::{Path, PathBuf};

/// Nom fixe de la seule règle que myiro-libre crée. Sans accent : il passe
/// tel quel dans `netsh` et dans le désinstallateur.
pub const NOM_REGLE: &str = "myiro-libre - FD-9 - detection (UDP 49152)";

/// Port local où la DLL du FD-9 attend les réponses à sa diffusion.
pub const PORT_DETECTION: u16 = 49152;

/// La règle entrante qui laisse passer la réponse du FD-9 : UDP, port local
/// 49152, réseau local seulement, tous profils, limitée au programme pont.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegleFd9 {
    /// Le programme `pont-fd9` que l'application lance : le pont installé à
    /// côté d'elle, ou celui de développement (chemin réel calculé).
    pub programme: PathBuf,
}

impl RegleFd9 {
    /// Arguments de `netsh` qui ajoutent la règle (droits d'administrateur).
    pub fn arguments_ajout(&self) -> String {
        format!(
            "advfirewall firewall add rule name=\"{NOM_REGLE}\" dir=in action=allow \
             protocol=UDP localport={PORT_DETECTION} remoteip=localsubnet profile=any \
             program=\"{}\"",
            self.programme.display()
        )
    }

    /// Commande équivalente, telle qu'on la tape dans une invite de commandes
    /// ouverte en administrateur.
    pub fn commande_ajout(&self) -> String {
        format!("netsh {}", self.arguments_ajout())
    }

    /// Arguments de `cmd.exe` qui posent la règle en une seule élévation :
    /// retrait de toute règle de ce nom (une ancienne qui visait un autre
    /// programme), puis ajout d'une seule. Le code de sortie est celui de
    /// l'ajout. `netsh` : chemin du programme `netsh.exe`.
    pub fn arguments_pose(&self, netsh: &Path) -> String {
        let netsh = netsh.display();
        format!(
            "/d /s /c \"\"{netsh}\" advfirewall firewall delete rule name=\"{NOM_REGLE}\" & \
             \"{netsh}\" {}\"",
            self.arguments_ajout()
        )
    }

    /// Arguments de `netsh` qui lisent la règle et son programme, sans rien
    /// modifier et sans droits particuliers. `netsh` sort avec le code 0 si
    /// elle existe.
    pub fn arguments_lecture() -> String {
        format!("advfirewall firewall show rule name=\"{NOM_REGLE}\" verbose")
    }

    /// Commande qui retire la règle (droits d'administrateur).
    pub fn commande_retrait() -> String {
        format!("netsh advfirewall firewall delete rule name=\"{NOM_REGLE}\"")
    }
}

/// Ce que la lecture du pare-feu dit de la règle [`NOM_REGLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatRegle {
    Absente,
    /// Une règle de ce nom existe, mais pour un autre programme (par exemple
    /// créée en développement, puis l'application installée) : elle ne laisse
    /// pas passer la réponse au pont lancé.
    AutreProgramme,
    /// La règle existe pour le pont lancé.
    Presente,
}

/// Pourquoi la règle n'a pas été posée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EchecPareFeu {
    /// L'opérateur a répondu « Non » à la fenêtre de contrôle de compte.
    Refuse,
    /// La fenêtre n'a pas pu s'ouvrir, ou `netsh` a échoué.
    Echec { detail: String },
}

/// L'action sur le pare-feu, injectée dans le module `instrument`.
pub trait PareFeu {
    /// État de la règle [`NOM_REGLE`] pour ce programme. Lecture seule, sans
    /// élévation.
    fn lire(&mut self, regle: &RegleFd9) -> EtatRegle;
    /// Pose la règle : toute règle de ce nom est remplacée par une seule,
    /// pour ce programme. C'est le seul endroit qui demande l'élévation.
    fn poser(&mut self, regle: &RegleFd9) -> Result<(), EchecPareFeu>;
}

/// Mémoire de la demande pendant une utilisation de l'application : la règle
/// n'est demandée automatiquement qu'une fois. Après un refus ou un échec,
/// seul le bouton « Autoriser le FD-9 dans le pare-feu » la redemande.
pub struct AutorisationPareFeu<F: PareFeu> {
    pare_feu: F,
    demandee: bool,
}

impl<F: PareFeu> AutorisationPareFeu<F> {
    pub fn new(pare_feu: F) -> Self {
        AutorisationPareFeu {
            pare_feu,
            demandee: false,
        }
    }

    pub fn pare_feu(&self) -> &F {
        &self.pare_feu
    }

    pub fn pare_feu_mut(&mut self) -> &mut F {
        &mut self.pare_feu
    }

    /// L'opérateur demande lui-même l'autorisation (bouton) : la prochaine
    /// détection vide la redemandera, une fois.
    pub fn autoriser_a_nouveau(&mut self) {
        self.demandee = false;
    }

    /// Demande la règle si elle n'a pas déjà été demandée. `None` si elle
    /// l'a déjà été : rien n'est redemandé.
    pub(crate) fn demander(&mut self, regle: &RegleFd9) -> Option<Result<(), EchecPareFeu>> {
        if self.demandee {
            return None;
        }
        self.demandee = true;
        Some(self.pare_feu.poser(regle))
    }
}

/// Pare-feu simulé : ne touche à rien, enregistre les demandes. Il garde au
/// plus une règle de ce nom, comme la pose réelle ; elle vise le programme
/// de la dernière pose réussie.
#[derive(Clone, Debug, Default)]
pub struct PareFeuSimule {
    /// Programme visé par la règle de myiro-libre, si elle existe.
    pub regle: Option<PathBuf>,
    /// Réponse imposée à la pose (refus, échec) ; `None` : la pose réussit.
    pub echec: Option<EchecPareFeu>,
    /// Règles dont la pose a été demandée, dans l'ordre.
    pub demandes: Vec<RegleFd9>,
}

impl PareFeu for PareFeuSimule {
    fn lire(&mut self, regle: &RegleFd9) -> EtatRegle {
        match &self.regle {
            None => EtatRegle::Absente,
            Some(programme) if *programme == regle.programme => EtatRegle::Presente,
            Some(_) => EtatRegle::AutreProgramme,
        }
    }

    fn poser(&mut self, regle: &RegleFd9) -> Result<(), EchecPareFeu> {
        self.demandes.push(regle.clone());
        match &self.echec {
            Some(echec) => Err(echec.clone()),
            None => {
                self.regle = Some(regle.programme.clone());
                Ok(())
            }
        }
    }
}

/// Le vrai pare-feu de Windows, par `netsh` (`System32`). La lecture se fait
/// sans droits ; la pose passe par la fenêtre de contrôle de compte
/// (`ShellExecuteExW`, verbe `runas`), seul dialogue montré à l'opérateur.
#[derive(Clone, Copy, Debug, Default)]
pub struct PareFeuWindows {
    /// Fenêtre de l'application (`HWND`), parente de la fenêtre de contrôle
    /// de compte pour qu'elle s'ouvre au premier plan ; 0 si inconnue.
    pub fenetre: isize,
}

/// La sortie de `netsh … show rule … verbose` cite-t-elle ce programme ?
/// Comparaison sans casse ; le pare-feu peut écrire le dossier de
/// l'utilisateur `%USERPROFILE%` (`profil`).
pub fn sortie_cite_le_programme(sortie: &str, programme: &Path, profil: Option<&Path>) -> bool {
    let sortie = sortie.to_lowercase();
    let chemin = programme.display().to_string();
    let mut formes = vec![chemin.to_lowercase()];
    if let Some(profil) = profil {
        let profil = profil.display().to_string();
        let suite = chemin.get(profil.len()..).filter(|_| {
            !profil.is_empty() && chemin.to_lowercase().starts_with(&profil.to_lowercase())
        });
        if let Some(suite) = suite {
            formes.push(format!("%userprofile%{suite}").to_lowercase());
        }
    }
    formes.iter().any(|forme| sortie.contains(forme.as_str()))
}

/// Attente maximale de `netsh` une fois l'élévation acceptée.
#[cfg(windows)]
const DELAI_NETSH_MS: u32 = 60_000;

#[cfg(windows)]
fn system32(programme: &str) -> PathBuf {
    let racine = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    PathBuf::from(racine).join("System32").join(programme)
}

#[cfg(windows)]
impl PareFeu for PareFeuWindows {
    fn lire(&mut self, regle: &RegleFd9) -> EtatRegle {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let Ok(sortie) = Command::new(system32("netsh.exe"))
            .raw_arg(RegleFd9::arguments_lecture())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        else {
            return EtatRegle::Absente;
        };
        if !sortie.status.success() {
            return EtatRegle::Absente;
        }
        let profil = std::env::var_os("USERPROFILE").map(PathBuf::from);
        let texte = String::from_utf8_lossy(&sortie.stdout);
        if sortie_cite_le_programme(&texte, &regle.programme, profil.as_deref()) {
            EtatRegle::Presente
        } else {
            EtatRegle::AutreProgramme
        }
    }

    fn poser(&mut self, regle: &RegleFd9) -> Result<(), EchecPareFeu> {
        elevation::lancer_en_administrateur(
            self.fenetre,
            &system32("cmd.exe"),
            &regle.arguments_pose(&system32("netsh.exe")),
            DELAI_NETSH_MS,
        )
    }
}

#[cfg(not(windows))]
impl PareFeu for PareFeuWindows {
    fn lire(&mut self, _: &RegleFd9) -> EtatRegle {
        EtatRegle::Absente
    }

    fn poser(&mut self, _: &RegleFd9) -> Result<(), EchecPareFeu> {
        Err(EchecPareFeu::Echec {
            detail: "pare-feu de Windows : pas sous Windows".into(),
        })
    }
}

/// Lancement d'un programme avec les droits d'administrateur, par la fenêtre
/// de contrôle de compte de Windows.
#[cfg(windows)]
mod elevation {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use super::EchecPareFeu;

    /// `SHELLEXECUTEINFOW` (`shellapi.h`). En 32 bits, l'en-tête la déclare
    /// sans bourrage ; tous ses champs y font 4 octets, la forme est la même.
    #[repr(C)]
    pub(super) struct ShellExecuteInfoW {
        pub taille: u32,
        pub masque: u32,
        pub fenetre: *mut c_void,
        pub verbe: *const u16,
        pub fichier: *const u16,
        pub parametres: *const u16,
        pub dossier: *const u16,
        pub affichage: i32,
        pub instance: *mut c_void,
        pub liste_id: *mut c_void,
        pub classe: *const u16,
        pub cle_classe: *mut c_void,
        pub raccourci: u32,
        pub icone: *mut c_void,
        pub processus: *mut c_void,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn WaitForSingleObject(objet: *mut c_void, millisecondes: u32) -> u32;
        fn GetExitCodeProcess(processus: *mut c_void, code: *mut u32) -> i32;
        fn CloseHandle(objet: *mut c_void) -> i32;
    }

    const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;
    const SEE_MASK_NOASYNC: u32 = 0x0000_0100;
    const SW_HIDE: i32 = 0;
    const WAIT_OBJECT_0: u32 = 0;
    /// L'opérateur a répondu « Non » à la fenêtre de contrôle de compte.
    const ERROR_CANCELLED: i32 = 1223;

    fn large(texte: &std::ffi::OsStr) -> Vec<u16> {
        texte.encode_wide().chain([0]).collect()
    }

    /// Lance `programme parametres` en administrateur, sans fenêtre, et
    /// attend sa fin au plus `delai_ms`. Réussit si le programme sort avec 0.
    /// `fenetre` : `HWND` de l'application, parente de la fenêtre de contrôle
    /// de compte (premier plan) ; 0 sans fenêtre connue.
    pub(super) fn lancer_en_administrateur(
        fenetre: isize,
        programme: &Path,
        parametres: &str,
        delai_ms: u32,
    ) -> Result<(), EchecPareFeu> {
        let verbe = large("runas".as_ref());
        let fichier = large(programme.as_os_str());
        let parametres = large(parametres.as_ref());
        let mut info = ShellExecuteInfoW {
            taille: std::mem::size_of::<ShellExecuteInfoW>() as u32,
            masque: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            fenetre: fenetre as *mut c_void,
            verbe: verbe.as_ptr(),
            fichier: fichier.as_ptr(),
            parametres: parametres.as_ptr(),
            dossier: std::ptr::null(),
            affichage: SW_HIDE,
            instance: std::ptr::null_mut(),
            liste_id: std::ptr::null_mut(),
            classe: std::ptr::null(),
            cle_classe: std::ptr::null_mut(),
            raccourci: 0,
            icone: std::ptr::null_mut(),
            processus: std::ptr::null_mut(),
        };
        // SAFETY : structure de la taille annoncée, textes terminés par zéro
        // et vivants pendant l'appel ; la poignée rendue est fermée ici.
        unsafe {
            if ShellExecuteExW(&mut info) == 0 {
                let erreur = std::io::Error::last_os_error();
                return Err(if erreur.raw_os_error() == Some(ERROR_CANCELLED) {
                    EchecPareFeu::Refuse
                } else {
                    EchecPareFeu::Echec {
                        detail: format!("élévation impossible : {erreur}"),
                    }
                });
            }
            if info.processus.is_null() {
                return Err(EchecPareFeu::Echec {
                    detail: "élévation : aucun processus rendu par Windows".into(),
                });
            }
            let attente = WaitForSingleObject(info.processus, delai_ms);
            let mut code = 0u32;
            let lu = GetExitCodeProcess(info.processus, &mut code);
            CloseHandle(info.processus);
            if attente != WAIT_OBJECT_0 {
                return Err(EchecPareFeu::Echec {
                    detail: format!("netsh n'a pas fini en {} s", delai_ms / 1000),
                });
            }
            if lu == 0 || code != 0 {
                return Err(EchecPareFeu::Echec {
                    detail: format!("netsh a échoué, code {code}"),
                });
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::ShellExecuteInfoW;

        /// Taille de `SHELLEXECUTEINFOW` dans le SDK de Windows : 60 octets en
        /// 32 bits, 112 en 64 bits. Vérifiée sans rien lancer.
        #[test]
        fn la_structure_a_la_taille_de_windows() {
            let attendue = if cfg!(target_pointer_width = "64") {
                112
            } else {
                60
            };
            assert_eq!(std::mem::size_of::<ShellExecuteInfoW>(), attendue);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{sortie_cite_le_programme, NOM_REGLE};

    /// Lecture de la règle : le programme est reconnu sans casse, et sous la
    /// forme `%USERPROFILE%` ; un autre programme ne l'est pas. Chemins fictifs.
    #[test]
    fn la_lecture_reconnait_le_programme_de_la_regle() {
        let installe = Path::new(r"C:\Users\exemple\AppData\Local\myiro-libre\pont-fd9-x86.exe");
        let profil = Path::new(r"C:\Users\exemple");
        let sortie =
            |programme: &str| format!("Rule Name: {NOM_REGLE}\r\nProgram: {programme}\r\n");
        assert!(sortie_cite_le_programme(
            &sortie(r"c:\users\exemple\appdata\local\myiro-libre\pont-fd9-x86.exe"),
            installe,
            Some(profil)
        ));
        assert!(sortie_cite_le_programme(
            &sortie(r"%USERPROFILE%\AppData\Local\myiro-libre\pont-fd9-x86.exe"),
            installe,
            Some(profil)
        ));
        assert!(!sortie_cite_le_programme(
            &sortie(r"C:\depot\target\i686-pc-windows-msvc\debug\pont-fd9.exe"),
            installe,
            Some(profil)
        ));
    }

    /// Le désinstallateur retire la règle sous le même nom.
    #[test]
    fn le_desinstallateur_retire_la_meme_regle() {
        let crochets = include_str!("../../installateur/pare-feu.nsh");
        assert!(crochets.contains(&format!("\"{NOM_REGLE}\"")), "{crochets}");
        // Une mise à jour garde la règle.
        assert!(crochets.contains("${If} $UpdateMode <> 1"), "{crochets}");
        let config = include_str!("../../tauri.conf.json");
        assert!(config.contains("installateur/pare-feu.nsh"), "{config}");
    }
}
