//! Choix de l'instrument (tickets #13 et #49) : quels instruments sont
//! présents sur le poste, lequel est actif, le passage de l'un à l'autre, le
//! dernier choix retenu, et le refus pendant une opération en cours.
//!
//! Un seul pont est ouvert à la fois : changer d'instrument ferme d'abord le
//! pont en cours (fermeture du ticket #24, résultat rapporté), puis ouvre
//! l'autre jusqu'à son plafond, sans aller plus loin. Les écrans ne décident
//! de rien : ils montrent [`VueSelection`] et transmettent le choix.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pont_protocole::{Palier, RemiseAuRepos, Reponse, Requete};
use serde::Serialize;

use super::fd9::NOM_DLL_FD9;
use super::parefeu::{AutorisationPareFeu, PareFeu};
use super::{chercher_dlls, Etat, Instrument, Probleme, Vue, NOM_DLL};
use crate::pont::{Architecture, Panne, Pont};

/// Où chercher la DLL d'un instrument, et ses ponts par architecture.
#[derive(Clone, Copy)]
pub struct Recherche<'a> {
    pub emplacements: &'a [PathBuf],
    pub ponts: &'a [(Architecture, PathBuf)],
}

/// Ouvre le MYIRO-1 ; s'il n'est pas détecté, cherche un FD-9. Si aucun des
/// deux ne l'est, on montre le problème du MYIRO-1, sauf quand son logiciel
/// est absent alors que celui du FD-9 a été trouvé : le vrai problème est
/// alors celui du FD-9 (pont manquant, DLL refusée, détection en erreur…).
/// `pare_feu` sert au FD-9 seul (ticket #47).
pub fn ouvrir_l_un_ou_l_autre<P: Pont, F: PareFeu>(
    myiro1: Recherche,
    fd9: Recherche,
    lancer: impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
    pare_feu: &mut AutorisationPareFeu<F>,
) -> Instrument<P> {
    let mut ouverture = Ouverture {
        myiro1,
        fd9,
        lancer,
        pare_feu,
    };
    l_un_ou_l_autre(&mut ouverture).1
}

/// Rend aussi le modèle retenu, et l'autre s'il a été essayé sans succès.
fn l_un_ou_l_autre<P: Pont, L, F>(
    o: &mut Ouverture<'_, L, F>,
) -> (Modele, Instrument<P>, Option<Modele>)
where
    L: FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
    F: PareFeu,
{
    let premier = o.ouvrir(Modele::Myiro1);
    if premier.etat != Etat::NonDetecte {
        return (Modele::Myiro1, premier, None);
    }
    let second = o.ouvrir(Modele::Fd9);
    let logiciel_absent =
        |i: &Instrument<P>| matches!(i.probleme(), Some(Probleme::LogicielAbsent { .. }));
    if second.etat != Etat::NonDetecte {
        return (Modele::Fd9, second, Some(Modele::Myiro1));
    }
    if logiciel_absent(&premier) && !logiciel_absent(&second) {
        return (Modele::Fd9, second, Some(Modele::Myiro1));
    }
    (Modele::Myiro1, premier, Some(Modele::Fd9))
}

/// Les deux instruments que l'application sait ouvrir.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modele {
    /// En USB, pont `pont-myiro1`.
    Myiro1,
    /// En réseau, pont `pont-fd9`.
    Fd9,
}

impl Modele {
    /// Dans l'ordre de la liste.
    pub const TOUS: [Modele; 2] = [Modele::Myiro1, Modele::Fd9];

    /// Nom affiché.
    pub fn nom(self) -> &'static str {
        match self {
            Modele::Myiro1 => "MYIRO-1",
            Modele::Fd9 => super::fd9::MODELE_FD9,
        }
    }

    /// Code court, celui de la page et du fichier du dernier choix.
    pub fn code(self) -> &'static str {
        match self {
            Modele::Myiro1 => "myiro1",
            Modele::Fd9 => "fd9",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Modele> {
        Modele::TOUS.into_iter().find(|m| m.code() == code)
    }

    /// Liaison avec l'ordinateur : clé `choix.liaison.<liaison>` du catalogue.
    pub fn liaison(self) -> &'static str {
        match self {
            Modele::Myiro1 => "usb",
            Modele::Fd9 => "reseau",
        }
    }

    fn autre(self) -> Modele {
        match self {
            Modele::Myiro1 => Modele::Fd9,
            Modele::Fd9 => Modele::Myiro1,
        }
    }
}

/// Dernier choix retenu dans `fichier`, s'il y en a un lisible.
pub fn lire_choix(fichier: &Path) -> Option<Modele> {
    Modele::depuis_code(std::fs::read_to_string(fichier).ok()?.trim())
}

/// Retient le choix pour le lancement suivant.
pub fn retenir_choix(fichier: &Path, modele: Modele) -> std::io::Result<()> {
    if let Some(dossier) = fichier.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    std::fs::write(fichier, modele.code())
}

/// Ce dont le module a besoin pour ouvrir un instrument : où chercher les
/// logiciels des deux instruments, comment lancer un pont, et le pare-feu
/// (FD-9 seul, ticket #47).
pub struct Ouverture<'a, L, F: PareFeu> {
    pub myiro1: Recherche<'a>,
    pub fd9: Recherche<'a>,
    /// Reçoit le programme pont, la DLL et le plafond.
    pub lancer: L,
    pub pare_feu: &'a mut AutorisationPareFeu<F>,
}

impl<L, F: PareFeu> Ouverture<'_, L, F> {
    fn ouvrir<P: Pont>(&mut self, modele: Modele) -> Instrument<P>
    where
        L: FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
    {
        match modele {
            Modele::Myiro1 => Instrument::ouvrir(
                self.myiro1.emplacements,
                self.myiro1.ponts,
                &mut self.lancer,
            ),
            Modele::Fd9 => Instrument::ouvrir_fd9(
                self.fd9.emplacements,
                self.fd9.ponts,
                &mut self.lancer,
                &mut *self.pare_feu,
            ),
        }
    }

    /// Instruments dont le logiciel du fabricant est trouvé sur le poste,
    /// sans rien lancer : la DLL seule compte.
    fn presents(&self) -> Vec<Modele> {
        let trouve = |r: &Recherche, nom: &str| {
            r.emplacements
                .iter()
                .any(|e| !chercher_dlls(e, nom).is_empty())
        };
        Modele::TOUS
            .into_iter()
            .filter(|m| match m {
                Modele::Myiro1 => trouve(&self.myiro1, NOM_DLL),
                Modele::Fd9 => trouve(&self.fd9, NOM_DLL_FD9),
            })
            .collect()
    }
}

/// Ce que l'instrument fait, et qui interdit d'en commencer une autre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Ouverture de l'instrument (lancement, « Réessayer », dossier choisi).
    Recherche,
    Etalonnage,
    Mesure,
    /// Passage d'un instrument à l'autre.
    Changement,
}

impl Operation {
    fn code(self) -> &'static str {
        match self {
            Operation::Recherche => "recherche",
            Operation::Etalonnage => "etalonnage",
            Operation::Mesure => "mesure",
            Operation::Changement => "changement",
        }
    }
}

/// Pourquoi une demande est refusée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refus {
    /// Une autre opération est en cours sur l'instrument.
    Occupe(Operation),
    /// Le logiciel de cet instrument n'est pas trouvé sur le poste.
    Absent(Modele),
}

impl Refus {
    /// Clé du catalogue qui explique le refus.
    pub fn cle(self) -> String {
        match self {
            Refus::Occupe(operation) => format!("refus.occupe.{}", operation.code()),
            Refus::Absent(_) => "refus.absent".into(),
        }
    }
}

/// Une seule opération à la fois sur l'instrument. Partagée entre les
/// demandes de l'écran : elle se consulte sans attendre la fin de
/// l'opération en cours.
#[derive(Default)]
pub struct Occupation(Mutex<Option<Operation>>);

impl Occupation {
    /// Commence une opération, ou la refuse avec celle qui est en cours.
    /// L'opération dure tant que le jeton rendu est gardé.
    pub fn commencer(&self, operation: Operation) -> Result<Jeton<'_>, Refus> {
        let mut en_cours = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(autre) = *en_cours {
            return Err(Refus::Occupe(autre));
        }
        *en_cours = Some(operation);
        Ok(Jeton(self))
    }

    pub fn en_cours(&self) -> Option<Operation> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Preuve qu'une opération a été commencée ; la rendre la termine.
pub struct Jeton<'a>(&'a Occupation);

impl Drop for Jeton<'_> {
    fn drop(&mut self) {
        *self.0 .0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Résultat de la fermeture du pont en cours (ticket #24), jamais masqué.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fermeture {
    /// Déconnecté, repos prouvé.
    Confirmee,
    /// Déconnecté, repos supposé ou non prouvé.
    Incertaine(RemiseAuRepos),
    /// La déconnexion a échoué, ou le pont n'a pas répondu : il est arrêté
    /// quand même ; l'instrument est peut-être resté dans un état incertain.
    Echec { detail: String },
}

impl<P: Pont> Instrument<P> {
    /// Ferme le pont : `fermer`, puis fin du pont. `None` si aucun pont
    /// n'était ouvert. L'instrument n'est plus connecté ensuite.
    pub fn fermer(&mut self) -> Option<Fermeture> {
        let mut pont = self.pont.take()?;
        self.etat = Etat::NonDetecte;
        self.probleme = None;
        self.etalonnage = None;
        let fermeture = match pont.demander(&Requete::Fermer {}) {
            Ok(Reponse::Ferme {}) => Fermeture::Confirmee,
            Ok(Reponse::FermetureIncertaine { remise_au_repos }) => {
                Fermeture::Incertaine(remise_au_repos)
            }
            Ok(Reponse::Erreur { erreur }) => Fermeture::Echec {
                detail: format!(
                    "fermeture : {erreur:?} ; pont arrêté quand même (dernière tentative de \
                     déconnexion par le pont)"
                ),
            },
            Ok(autre) => Fermeture::Echec {
                detail: format!("fermeture : réponse inattendue : {autre:?}"),
            },
            Err(panne) => Fermeture::Echec {
                detail: format!("fermeture : {panne:?}"),
            },
        };
        // Le pont est rendu ici : `PontProcessus` attend sa fin, ou l'arrête.
        drop(pont);
        Some(fermeture)
    }
}

/// Ce que la sélection a à dire à l'opérateur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Annonce {
    /// Aucun choix retenu : l'instrument trouvé est pris.
    SansChoix { pris: Modele },
    /// L'instrument choisi la dernière fois n'est pas trouvé : l'autre est pris.
    Remplace { choisi: Modele, pris: Modele },
    /// Le pont de cet instrument a été fermé, avec ce résultat.
    Ferme {
        modele: Modele,
        fermeture: Fermeture,
    },
}

/// Une ligne de la liste des instruments présents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ligne {
    pub code: &'static str,
    pub modele: String,
    /// Clé `choix.liaison.<liaison>`.
    pub liaison: &'static str,
    /// Clé `choix.etat.<etat>` : l'état de l'instrument actif, sinon
    /// `en_attente` ou `non_trouve` (pas trouvé au dernier essai).
    pub etat: &'static str,
    pub actif: bool,
}

/// L'annonce, pour la page : clé `choix.annonce.<code>`, où `{modele}` et
/// `{autre}` sont remplacés.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueAnnonce {
    pub code: &'static str,
    pub modele: String,
    pub autre: Option<String>,
    /// Montrer l'avis en orange : le repos n'est pas prouvé.
    pub alerte: bool,
    pub detail: Option<String>,
}

/// Ce que la barre et la liste reçoivent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueSelection {
    pub instrument: Vue,
    pub instruments: Vec<Ligne>,
    pub annonce: Option<VueAnnonce>,
    /// Ouvrir la liste d'elle-même : l'annonce doit être vue.
    pub montrer_liste: bool,
}

/// L'instrument actif, parmi ceux présents sur le poste.
pub struct Selection<P: Pont> {
    presents: Vec<Modele>,
    actif: Modele,
    instrument: Instrument<P>,
    /// Choix fait par l'opérateur, retenu d'un lancement à l'autre.
    choisi: Option<Modele>,
    /// Instruments inactifs qui n'ont pas été trouvés au dernier essai.
    non_trouves: Vec<Modele>,
    annonce: Option<Annonce>,
    montrer_liste: bool,
}

impl<P: Pont> Selection<P> {
    /// Au lancement : l'instrument retenu la fois précédente, s'il est
    /// trouvé ; sinon l'autre s'il l'est, et on le dit. Sans choix retenu,
    /// le MYIRO-1 d'abord, puis le FD-9, et on dit lequel est pris.
    pub fn lancer<L, F>(retenu: Option<Modele>, o: &mut Ouverture<'_, L, F>) -> Self
    where
        L: FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        F: PareFeu,
    {
        let presents = o.presents();
        let Some(choisi) = retenu else {
            let (actif, instrument, essaye) = l_un_ou_l_autre(o);
            let trouve = instrument.etat != Etat::NonDetecte;
            return Selection {
                montrer_liste: trouve && presents.len() > 1,
                annonce: trouve.then_some(Annonce::SansChoix { pris: actif }),
                presents,
                actif,
                instrument,
                choisi: None,
                non_trouves: essaye.into_iter().collect(),
            };
        };
        let instrument = o.ouvrir(choisi);
        let autre = choisi.autre();
        if instrument.etat == Etat::NonDetecte && presents.contains(&autre) {
            let second = o.ouvrir(autre);
            if second.etat != Etat::NonDetecte {
                return Selection {
                    presents,
                    actif: autre,
                    instrument: second,
                    choisi: Some(choisi),
                    non_trouves: vec![choisi],
                    annonce: Some(Annonce::Remplace {
                        choisi,
                        pris: autre,
                    }),
                    montrer_liste: true,
                };
            }
        }
        Selection {
            presents,
            actif: choisi,
            instrument,
            choisi: Some(choisi),
            non_trouves: Vec::new(),
            annonce: None,
            montrer_liste: false,
        }
    }

    /// Passe à l'instrument `vers`, choisi par l'opérateur : ferme le pont en
    /// cours, puis ouvre celui de `vers` jusqu'à son plafond. Le choix est
    /// retenu même si `vers` n'est pas trouvé : son problème est montré.
    /// `_jeton` : l'opération `Changement` est commencée.
    pub fn changer<L, F>(
        &mut self,
        _jeton: &Jeton,
        vers: Modele,
        o: &mut Ouverture<'_, L, F>,
    ) -> Result<(), Refus>
    where
        L: FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        F: PareFeu,
    {
        self.presents = o.presents();
        if !self.presents.contains(&vers) {
            return Err(Refus::Absent(vers));
        }
        self.choisi = Some(vers);
        self.montrer_liste = false;
        if vers == self.actif && self.instrument.etat != Etat::NonDetecte {
            self.annonce = None;
            return Ok(());
        }
        let ancien = self.actif;
        let ancien_trouve = self.instrument.etat != Etat::NonDetecte;
        self.annonce = self.instrument.fermer().map(|fermeture| Annonce::Ferme {
            modele: ancien,
            fermeture,
        });
        self.non_trouves.retain(|m| *m != vers && *m != ancien);
        if ancien != vers && !ancien_trouve {
            self.non_trouves.push(ancien);
        }
        self.actif = vers;
        self.instrument = o.ouvrir(vers);
        Ok(())
    }

    /// Rouvre l'instrument (« Réessayer », dossier choisi, pare-feu) : celui
    /// qui est actif si l'opérateur a fait un choix, sinon comme au lancement
    /// sans choix retenu. Le pont en cours est d'abord fermé.
    pub fn rouvrir<L, F>(&mut self, _jeton: &Jeton, o: &mut Ouverture<'_, L, F>)
    where
        L: FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        F: PareFeu,
    {
        let ancien = self.actif;
        self.annonce = self.instrument.fermer().map(|fermeture| Annonce::Ferme {
            modele: ancien,
            fermeture,
        });
        self.montrer_liste = false;
        self.presents = o.presents();
        if self.choisi.is_some() {
            self.instrument = o.ouvrir(self.actif);
            return;
        }
        let (actif, instrument, essaye) = l_un_ou_l_autre(o);
        self.actif = actif;
        self.instrument = instrument;
        self.non_trouves = essaye.into_iter().collect();
    }

    pub fn actif(&self) -> Modele {
        self.actif
    }

    /// Le choix à retenir pour le lancement suivant.
    pub fn choisi(&self) -> Option<Modele> {
        self.choisi
    }

    pub fn instrument(&self) -> &Instrument<P> {
        &self.instrument
    }

    /// L'instrument actif, pour l'étalonner ou mesurer.
    pub fn instrument_mut(&mut self) -> &mut Instrument<P> {
        &mut self.instrument
    }

    pub fn annonce(&self) -> Option<&Annonce> {
        self.annonce.as_ref()
    }

    pub fn vue(&self) -> VueSelection {
        let instrument = self.instrument.vue();
        let mut modeles = self.presents.clone();
        if !modeles.contains(&self.actif) && self.instrument.etat != Etat::NonDetecte {
            modeles.push(self.actif);
            modeles.sort();
        }
        let instruments = modeles
            .into_iter()
            .map(|m| {
                let actif = m == self.actif;
                Ligne {
                    code: m.code(),
                    modele: m.nom().into(),
                    liaison: m.liaison(),
                    etat: if actif {
                        instrument.etat
                    } else if self.non_trouves.contains(&m) {
                        "non_trouve"
                    } else {
                        "en_attente"
                    },
                    actif,
                }
            })
            .collect();
        VueSelection {
            instrument,
            instruments,
            annonce: self.annonce.as_ref().map(vue_annonce),
            montrer_liste: self.montrer_liste,
        }
    }
}

fn vue_annonce(annonce: &Annonce) -> VueAnnonce {
    let simple = |code, modele: Modele, autre: Option<Modele>| VueAnnonce {
        code,
        modele: modele.nom().into(),
        autre: autre.map(|m| m.nom().into()),
        alerte: false,
        detail: None,
    };
    match annonce {
        Annonce::SansChoix { pris } => simple("sans_choix", *pris, None),
        Annonce::Remplace { choisi, pris } => simple("remplace", *choisi, Some(*pris)),
        Annonce::Ferme { modele, fermeture } => match fermeture {
            Fermeture::Confirmee => simple("ferme", *modele, None),
            Fermeture::Incertaine(RemiseAuRepos::ReposSuppose {}) => VueAnnonce {
                detail: Some("fermeture : repos supposé (aucune mesure lancée)".into()),
                ..simple("ferme_repos_suppose", *modele, None)
            },
            Fermeture::Incertaine(repos) => VueAnnonce {
                alerte: true,
                detail: Some(format!("fermeture incertaine : {repos:?}")),
                ..simple("ferme_incertain", *modele, None)
            },
            Fermeture::Echec { detail } => VueAnnonce {
                alerte: true,
                detail: Some(detail.clone()),
                ..simple("ferme_echec", *modele, None)
            },
        },
    }
}
