//! Le pare-feu de Windows et la détection du FD-9 (tickets #47 et #85).
//!
//! La DLL du FD-9 lit la réponse à sa diffusion sur le port UDP 49152, depuis
//! une autre socket que celle qui envoie : sans règle entrante, Windows
//! bloque cette réponse et la liste revient vide, sans erreur (cause
//! confirmée le 9 octobre 2026, fiche `docs/abi/FD9_GetDeviceList.md`).
//!
//! Depuis le ticket #85, la règle est permanente : par port, sans programme,
//! posée par l'installateur (`app/installateur/pare-feu.nsh`) et retirée par
//! le désinstallateur. L'application la vérifie et, si elle manque ou est
//! d'un ancien modèle, la redemande en secours.
//!
//! Le module `instrument` décide quand demander la règle ; ce fichier la
//! décrit, et le trait [`PareFeu`] la vérifie et l'ajoute. Deux adapters :
//! [`PareFeuWindows`] (lecture par `netsh` sans droits, ajout par la fenêtre
//! de contrôle de compte de Windows) et [`PareFeuSimule`] pour les tests, qui
//! ne touche jamais au vrai pare-feu.

use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

/// Nom fixe de la seule règle que myiro-libre crée. Sans accent : il passe
/// tel quel dans `netsh`, l'installateur et le désinstallateur.
pub const NOM_REGLE: &str = "myiro-libre - FD-9 - detection (UDP 49152)";

/// Port local où la DLL du FD-9 attend les réponses à sa diffusion.
pub const PORT_DETECTION: u16 = 49152;

/// Étendue de la règle, telle que `netsh` l'écrit (dans toutes les langues) :
/// le réseau local de l'ordinateur, rien au-delà.
const ETENDUE: &str = "LocalSubnet";

/// La règle entrante qui laisse passer la réponse du FD-9 : UDP, port local
/// 49152, réseau local seulement, tous profils, sans programme. Elle vaut
/// quel que soit l'emplacement du pont lancé (installé, de développement,
/// exécutable de test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegleFd9;

impl RegleFd9 {
    /// Arguments de `netsh` qui ajoutent la règle (droits d'administrateur).
    /// L'installateur passe les mêmes (test du crochet `pare-feu.nsh`).
    pub fn arguments_ajout() -> String {
        format!(
            "advfirewall firewall add rule name=\"{NOM_REGLE}\" dir=in action=allow \
             protocol=UDP localport={PORT_DETECTION} remoteip=localsubnet profile=any"
        )
    }

    /// Commande équivalente, telle qu'on la tape dans une invite de commandes
    /// ouverte en administrateur.
    pub fn commande_ajout() -> String {
        format!("netsh {}", Self::arguments_ajout())
    }

    /// Arguments de `cmd.exe` qui posent la règle en une seule élévation :
    /// retrait de toute règle de ce nom (une ancienne, liée à un programme),
    /// puis ajout d'une seule. Le code de sortie est celui de l'ajout.
    /// `netsh` : chemin du programme `netsh.exe`.
    pub fn arguments_pose(netsh: &Path) -> String {
        let netsh = netsh.display();
        format!(
            "/d /s /c \"\"{netsh}\" advfirewall firewall delete rule name=\"{NOM_REGLE}\" & \
             \"{netsh}\" {}\"",
            Self::arguments_ajout()
        )
    }

    /// Arguments de `netsh` qui lisent la règle en détail, sans rien
    /// modifier et sans droits particuliers. `netsh` sort avec le code 0 si
    /// elle existe ; [`etat_selon_netsh`] lit sa sortie.
    pub fn arguments_lecture() -> String {
        format!("advfirewall firewall show rule name=\"{NOM_REGLE}\" verbose")
    }

    /// Commande qui retire la règle (droits d'administrateur).
    pub fn commande_retrait() -> String {
        format!("netsh advfirewall firewall delete rule name=\"{NOM_REGLE}\"")
    }
}

/// Ce que la lecture du pare-feu dit de la règle [`NOM_REGLE`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EtatRegle {
    #[default]
    Absente,
    /// Une règle de ce nom existe, mais pas telle qu'on la pose : ancienne
    /// règle liée à un programme (avant le ticket #85), autre port, autre
    /// protocole ou autre étendue. Elle est remplacée comme si elle manquait.
    ARefaire,
    /// La règle existe, conforme.
    Presente,
}

/// Lit la sortie de `netsh advfirewall firewall show rule name=… verbose`
/// ([`RegleFd9::arguments_lecture`]), en français ou en anglais. Chaque règle
/// de ce nom doit être conforme : port local 49152, UDP, étendue
/// `LocalSubnet`, sans programme (pas de ligne `Program`, ou `Any`/`Tout`).
///
/// Supposé, à confirmer à l'essai sur le poste (relevé dans les ressources de
/// langue de `netsh`, sans le lancer) : les libellés `LocalPort`, `RemoteIP`
/// et `Program` restent en anglais dans le Windows français ; `Protocol` y
/// devient `Protocole`. L'espace
/// insécable avant « : » du français arrive parfois illisible (`\u{fffd}`).
pub fn etat_selon_netsh(sortie: &str) -> EtatRegle {
    // Chaque règle commence par son nom, souligné d'une ligne de tirets :
    // ce qui suit une ligne de tirets est une règle.
    let mut regles: Vec<Vec<(String, &str)>> = Vec::new();
    for ligne in sortie.lines() {
        let ligne = ligne.trim();
        if ligne.len() >= 10 && ligne.chars().all(|c| c == '-') {
            regles.push(Vec::new());
        } else if let (Some(regle), Some((libelle, valeur))) =
            (regles.last_mut(), ligne.split_once(':'))
        {
            let libelle = libelle
                .trim_matches(|c: char| c.is_whitespace() || c == '\u{fffd}')
                .to_ascii_lowercase();
            regle.push((libelle, valeur.trim()));
        }
    }
    if regles.is_empty() {
        return EtatRegle::Absente;
    }
    let conforme = |regle: &Vec<(String, &str)>| {
        let valeur = |noms: &[&str]| {
            regle
                .iter()
                .find(|(libelle, _)| noms.contains(&libelle.as_str()))
                .map(|(_, valeur)| *valeur)
        };
        let port = PORT_DETECTION.to_string();
        valeur(&["localport"]) == Some(port.as_str())
            && valeur(&["protocol", "protocole"]).is_some_and(|v| v.eq_ignore_ascii_case("UDP"))
            && valeur(&["remoteip"]).is_some_and(|v| v.eq_ignore_ascii_case(ETENDUE))
            && valeur(&["program", "programme"])
                .is_none_or(|v| v.eq_ignore_ascii_case("Any") || v.eq_ignore_ascii_case("Tout"))
    };
    if regles.iter().all(conforme) {
        EtatRegle::Presente
    } else {
        EtatRegle::ARefaire
    }
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
    /// État de la règle [`NOM_REGLE`]. Lecture seule, sans élévation.
    fn lire(&mut self) -> EtatRegle;
    /// Pose la règle : toute règle de ce nom est remplacée par une seule,
    /// conforme. C'est le seul endroit de l'application qui demande
    /// l'élévation.
    fn poser(&mut self) -> Result<(), EchecPareFeu>;
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
    pub(crate) fn demander(&mut self) -> Option<Result<(), EchecPareFeu>> {
        if self.demandee {
            return None;
        }
        self.demandee = true;
        Some(self.pare_feu.poser())
    }
}

/// Pare-feu simulé : ne touche à rien, compte les poses. Une pose réussie
/// rend la règle conforme, comme la pose réelle.
#[derive(Clone, Debug, Default)]
pub struct PareFeuSimule {
    /// État de la règle de myiro-libre.
    pub regle: EtatRegle,
    /// Réponse imposée à la pose (refus, échec) ; `None` : la pose réussit.
    pub echec: Option<EchecPareFeu>,
    /// Nombre de poses demandées.
    pub poses: usize,
}

impl PareFeu for PareFeuSimule {
    fn lire(&mut self) -> EtatRegle {
        self.regle
    }

    fn poser(&mut self) -> Result<(), EchecPareFeu> {
        self.poses += 1;
        match &self.echec {
            Some(echec) => Err(echec.clone()),
            None => {
                self.regle = EtatRegle::Presente;
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
    fn lire(&mut self) -> EtatRegle {
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
        match etat_selon_netsh(&String::from_utf8_lossy(&sortie.stdout)) {
            // `netsh` dit qu'elle existe, mais rien n'en est lisible (langue
            // inconnue) : elle est refaite plutôt que crue conforme.
            EtatRegle::Absente => EtatRegle::ARefaire,
            etat => etat,
        }
    }

    fn poser(&mut self) -> Result<(), EchecPareFeu> {
        elevation::lancer_en_administrateur(
            self.fenetre,
            &system32("cmd.exe"),
            &RegleFd9::arguments_pose(&system32("netsh.exe")),
            DELAI_NETSH_MS,
        )
    }
}

#[cfg(not(windows))]
impl PareFeu for PareFeuWindows {
    fn lire(&mut self) -> EtatRegle {
        EtatRegle::Absente
    }

    fn poser(&mut self) -> Result<(), EchecPareFeu> {
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

    use super::{etat_selon_netsh, EtatRegle, RegleFd9, NOM_REGLE};

    /// Sortie de `netsh advfirewall firewall show rule … verbose` sur un
    /// Windows en anglais, pour la règle conforme (aucune adresse réelle).
    const SORTIE_ANGLAISE: &str = "\r\n\
Rule Name:                            myiro-libre - FD-9 - detection (UDP 49152)\r\n\
----------------------------------------------------------------------\r\n\
Enabled:                              Yes\r\n\
Direction:                            In\r\n\
Profiles:                             Domain,Private,Public\r\n\
Grouping:                             \r\n\
LocalIP:                              Any\r\n\
RemoteIP:                             LocalSubnet\r\n\
Protocol:                             UDP\r\n\
LocalPort:                            49152\r\n\
RemotePort:                           Any\r\n\
Edge traversal:                       No\r\n\
InterfaceTypes:                       Any\r\n\
Security:                             NotRequired\r\n\
Rule source:                          Local Setting\r\n\
Action:                               Allow\r\n\
Ok.\r\n\r\n";

    /// La même sur un Windows en français, telle que l'application la lit :
    /// l'espace insécable avant « : » et les lettres accentuées arrivent
    /// illisibles (page de code de la console), d'où les `\u{fffd}`.
    const SORTIE_FRANCAISE: &str = "\r\n\
Nom de la r\u{fffd}gle\u{fffd}:                     myiro-libre - FD-9 - detection (UDP 49152)\r\n\
----------------------------------------------------------------------\r\n\
Activ\u{fffd}\u{fffd}:                              Oui\r\n\
Direction\u{fffd}:                           Actif\r\n\
Profiles :                            Domaine,Priv\u{fffd},Public\r\n\
Groupement\u{fffd}:                          \r\n\
LocalIP\u{fffd}:                             Tout\r\n\
RemoteIP\u{fffd}:                            LocalSubnet\r\n\
Protocole\u{fffd}:                           UDP\r\n\
LocalPort\u{fffd}:                           49152\r\n\
RemotePort\u{fffd}:                          Tout\r\n\
Travers\u{fffd}e de bordure\u{fffd}:                Non\r\n\
InterfaceTypes\u{fffd}:                      Tout\r\n\
Security\u{fffd}:                            NotRequired\r\n\
Source de la r\u{fffd}gle\u{fffd}:                  Param\u{fffd}tre local\r\n\
Action\u{fffd}:                              Autoriser\r\n\
Ok.\r\n\r\n";

    /// Ajoute la ligne du programme d'une ancienne règle (avant le ticket
    /// #85), juste avant `InterfaceTypes`. Chemins fictifs.
    fn avec_programme(sortie: &str, ligne: &str) -> String {
        sortie.replacen("InterfaceTypes", &format!("{ligne}\r\nInterfaceTypes"), 1)
    }

    #[test]
    fn la_regle_conforme_est_reconnue_en_anglais_et_en_francais() {
        assert_eq!(etat_selon_netsh(SORTIE_ANGLAISE), EtatRegle::Presente);
        assert_eq!(etat_selon_netsh(SORTIE_FRANCAISE), EtatRegle::Presente);
        // Une ligne « Program » qui dit « tout » ne lie la règle à rien.
        assert_eq!(
            etat_selon_netsh(&avec_programme(
                SORTIE_ANGLAISE,
                "Program:                              Any"
            )),
            EtatRegle::Presente
        );
        assert_eq!(
            etat_selon_netsh(&avec_programme(
                SORTIE_FRANCAISE,
                "Program\u{fffd}:                             Tout"
            )),
            EtatRegle::Presente
        );
    }

    #[test]
    fn une_ancienne_regle_liee_a_un_programme_est_a_refaire() {
        assert_eq!(
            etat_selon_netsh(&avec_programme(
                SORTIE_ANGLAISE,
                r"Program:                              C:\Program Files\myiro-libre\pont-fd9-x86.exe"
            )),
            EtatRegle::ARefaire
        );
        assert_eq!(
            etat_selon_netsh(&avec_programme(
                SORTIE_FRANCAISE,
                "Program\u{fffd}:                             \
                 %USERPROFILE%\\AppData\\Local\\myiro-libre\\pont-fd9-x86.exe"
            )),
            EtatRegle::ARefaire
        );
    }

    #[test]
    fn une_regle_d_un_autre_port_protocole_ou_etendue_est_a_refaire() {
        for (sortie, tout) in [(SORTIE_ANGLAISE, "Any"), (SORTIE_FRANCAISE, "Tout")] {
            let autre_port = sortie.replace("49152\r\n", "5000\r\n");
            let plage = sortie.replace("49152\r\n", "49152-49153\r\n");
            let adresse = sortie.replace("LocalSubnet", "192.0.2.40/255.255.255.255");
            // Ouverte à toutes les adresses, au-delà du réseau local.
            let partout = sortie.replace("LocalSubnet", tout);
            let tcp = sortie.replace("UDP\r\n", "TCP\r\n");
            for mauvaise in [autre_port, plage, adresse, partout, tcp] {
                assert_eq!(
                    etat_selon_netsh(&mauvaise),
                    EtatRegle::ARefaire,
                    "{mauvaise}"
                );
            }
        }
    }

    #[test]
    fn deux_regles_du_meme_nom_dont_une_ancienne_sont_a_refaire() {
        let ancienne = avec_programme(
            SORTIE_ANGLAISE,
            r"Program:                              C:\depot\target\debug\pont-fd9.exe",
        );
        let deux = format!(
            "{}{}",
            SORTIE_ANGLAISE.trim_end_matches("Ok.\r\n\r\n"),
            ancienne
        );
        assert_eq!(etat_selon_netsh(&deux), EtatRegle::ARefaire);
    }

    #[test]
    fn sans_regle_de_ce_nom_elle_est_absente() {
        for sortie in [
            "\r\nNo rules match the specified criteria.\r\n\r\n",
            "\r\nAucune r\u{fffd}gle ne correspond aux crit\u{fffd}res sp\u{fffd}cifi\u{fffd}s.\r\n\r\n",
            "",
        ] {
            assert_eq!(etat_selon_netsh(sortie), EtatRegle::Absente, "{sortie}");
        }
    }

    /// L'installateur pose la règle, le désinstallateur la retire : même nom,
    /// mêmes arguments que l'application.
    #[test]
    fn l_installateur_pose_et_le_desinstallateur_retire_la_meme_regle() {
        let crochets = include_str!("../../installateur/pare-feu.nsh");
        let dans_nsis = |texte: String| texte.replace(NOM_REGLE, "${MYIRO_REGLE_FD9}");
        assert!(
            crochets.contains(&format!("!define MYIRO_REGLE_FD9 \"{NOM_REGLE}\"")),
            "{crochets}"
        );
        // Lecture sans droits, puis pose en une seule demande de droits :
        // retrait de toute règle de ce nom, ajout d'une seule.
        assert!(
            crochets.contains("!macro NSIS_HOOK_POSTINSTALL"),
            "{crochets}"
        );
        assert!(
            crochets.contains(&dans_nsis(format!(
                "\"$SYSDIR\\netsh.exe\" {}",
                RegleFd9::arguments_lecture()
            ))),
            "{crochets}"
        );
        assert!(
            crochets.contains(&format!(
                "ExecShellWait \"runas\" \"$SYSDIR\\cmd.exe\" '{}' SW_HIDE",
                dans_nsis(RegleFd9::arguments_pose(Path::new(r"$SYSDIR\netsh.exe")))
            )),
            "{crochets}"
        );
        assert!(!crochets.contains("program="), "{crochets}");
        // Le désinstallateur la retire ; une mise à jour la garde.
        assert!(
            crochets.contains("!macro NSIS_HOOK_POSTUNINSTALL"),
            "{crochets}"
        );
        assert!(
            crochets.contains(&dans_nsis(format!(
                "'advfirewall firewall delete rule name=\"{NOM_REGLE}\"'"
            ))),
            "{crochets}"
        );
        assert!(crochets.contains("${If} $UpdateMode <> 1"), "{crochets}");
        let config = include_str!("../../tauri.conf.json");
        assert!(config.contains("installateur/pare-feu.nsh"), "{config}");
    }
}
