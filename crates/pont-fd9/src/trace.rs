//! Trace des appels à la DLL, pour les essais sur l'instrument (`--trace`).
//!
//! [`SdkTrace`] enveloppe un [`SdkFd9`] : chaque appel est écrit **avant**
//! d'entrer dans la DLL (`→`), puis son retour (`←`). Si la DLL arrête le
//! processus, la dernière ligne `→` sans `←` désigne l'appel en cours. Les
//! lignes portent le temps écoulé depuis le lancement et le n° du fil
//! Windows, comme celles du rappel ([`crate::evenements`]) et du rapport de
//! plantage ([`crate::plantage`]). Aucune adresse d'instrument, aucun
//! identifiant n'y est écrit.

use crate::{plantage, Evenement, SdkFd9};
use fd9_sys::{
    Appareil, CleOption, CoordonneesMire, CouleursMire, DonneesQr, ModeReconnaissance,
    NomApplication, PointMesure, PositionPlage, TamponInfosSysteme, DONNEES_BRUTES,
};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static LANCEMENT: OnceLock<Instant> = OnceLock::new();

/// Temps écoulé depuis le premier appel de cette fonction (fait au lancement
/// par l'exécutable).
pub fn depuis_le_lancement() -> Duration {
    LANCEMENT.get_or_init(Instant::now).elapsed()
}

/// Début commun des lignes de trace : temps et fil.
pub fn entete() -> String {
    format!(
        "trace {:>9.3} s fil {}",
        depuis_le_lancement().as_secs_f64(),
        plantage::fil_courant()
    )
}

/// Un [`SdkFd9`] dont chaque appel est écrit à `sortie`, à l'entrée et au
/// retour.
pub struct SdkTrace<S: SdkFd9> {
    sdk: S,
    sortie: Box<dyn FnMut(&str)>,
}

impl<S: SdkFd9> SdkTrace<S> {
    pub fn new(sdk: S, sortie: impl FnMut(&str) + 'static) -> Self {
        depuis_le_lancement();
        SdkTrace {
            sdk,
            sortie: Box::new(sortie),
        }
    }

    /// Le SDK enveloppé.
    pub fn interieur(&self) -> &S {
        &self.sdk
    }

    fn ecrire(&mut self, sens: &str, texte: &str) {
        let ligne = format!("{} {sens} {texte}", entete());
        (self.sortie)(&ligne);
    }

    /// Écrit `entree`, fait l'appel, puis écrit le nom de l'export et ce que
    /// `retour` en dit.
    fn appel<T>(
        &mut self,
        export: &str,
        arguments: &str,
        f: impl FnOnce(&mut S) -> T,
        retour: impl FnOnce(&T) -> String,
    ) -> T {
        self.ecrire("→", &format!("{export}({arguments})"));
        let resultat = f(&mut self.sdk);
        let texte = retour(&resultat);
        self.ecrire("←", &format!("{export} : {texte}"));
        resultat
    }
}

fn code(c: &i32) -> String {
    c.to_string()
}

impl<S: SdkFd9> SdkFd9 for SdkTrace<S> {
    fn dernier_code(&mut self) -> i32 {
        self.appel("FD9_GetLastError", "", |s| s.dernier_code(), code)
    }

    fn lister(&mut self, tableau: &mut [Appareil], trouves: &mut u32) -> i32 {
        let arguments = format!("capacité {}", tableau.len());
        let c = self.appel(
            "FD9_GetDeviceList",
            &arguments,
            |s| s.lister(tableau, trouves),
            code,
        );
        self.ecrire("·", &format!("FD9_GetDeviceList : {trouves} trouvé(s)"));
        c
    }

    fn version_fichier(&self) -> Option<[u32; 4]> {
        self.sdk.version_fichier()
    }

    fn empreinte(&self) -> Option<String> {
        self.sdk.empreinte()
    }

    fn inscrire_rappel(&mut self, actif: bool) -> i32 {
        let arguments = if actif { "rappel du pont" } else { "nul" };
        self.appel(
            "FD9_RegisterDeviceEventHandler",
            arguments,
            |s| s.inscrire_rappel(actif),
            code,
        )
    }

    fn connecter(&mut self, appareil: &Appareil, nom: &NomApplication) -> i32 {
        // Ni l'adresse ni l'identifiant de l'instrument : la liaison seule.
        let arguments = format!("liaison {}", appareil.code_liaison);
        self.appel(
            "FD9_Connect",
            &arguments,
            |s| s.connecter(appareil, nom),
            code,
        )
    }

    fn infos(&mut self, tampon: &mut TamponInfosSysteme) -> i32 {
        self.appel("FD9_GetSystemInfo", "", |s| s.infos(tampon), code)
    }

    fn statut(&mut self, tampon: &mut [u32; 8]) -> i32 {
        let c = self.appel("FD9_GetStatus", "", |s| s.statut(tampon), code);
        self.ecrire("·", &format!("FD9_GetStatus : état {}", tampon[4]));
        c
    }

    fn deconnecter(&mut self) -> i32 {
        self.appel("FD9_Disconnect", "", |s| s.deconnecter(), code)
    }

    fn evenements(&mut self, attente: Duration) -> Vec<Evenement> {
        // Ne parle pas à la DLL ; le rappel a sa propre trace.
        self.sdk.evenements(attente)
    }

    fn rappels_orphelins(&self) -> u64 {
        self.sdk.rappels_orphelins()
    }

    fn capturer(&mut self) -> i32 {
        self.appel("FD9_StartCapture", "", |s| s.capturer(), code)
    }

    fn resultat_capture(
        &mut self,
        qr_resultat: &mut u32,
        qr: &mut DonneesQr,
        taille: &mut u32,
        image: Option<&mut [u8]>,
    ) -> i32 {
        let arguments = match &image {
            None => "image nulle".to_string(),
            Some(tampon) => format!("tampon de {} octets", tampon.len()),
        };
        let c = self.appel(
            "FD9_GetCaptureResult",
            &arguments,
            |s| s.resultat_capture(qr_resultat, qr, taille, image),
            code,
        );
        self.ecrire(
            "·",
            &format!("FD9_GetCaptureResult : QR {qr_resultat}, taille {taille}"),
        );
        c
    }

    fn reconnaitre(
        &mut self,
        mode: ModeReconnaissance,
        coordonnees: &CoordonneesMire,
        couleurs: Option<&CouleursMire>,
    ) -> i32 {
        let teinte = couleurs.map_or("nulles".to_string(), |c| {
            format!("RVB × {}", c.entrees().len())
        });
        let arguments = format!(
            "mode {}, feuille {} × {}, zones {}, plages {}, modèle {}, couleurs {teinte}, rangées 0, colonnes 0",
            mode as u32,
            coordonnees.largeur,
            coordonnees.hauteur,
            coordonnees.zones,
            coordonnees.plages,
            coordonnees.modele_present
        );
        self.appel(
            "FD9_StartPatchRecognition",
            &arguments,
            |s| s.reconnaitre(mode, coordonnees, couleurs),
            code,
        )
    }

    fn resultat_reconnaissance(
        &mut self,
        fiabilite: &mut u32,
        coordonnees: &mut CoordonneesMire,
    ) -> i32 {
        let c = self.appel(
            "FD9_GetPatchRecognitionResult2",
            "disposition nulle",
            |s| s.resultat_reconnaissance(fiabilite, coordonnees),
            code,
        );
        self.ecrire(
            "·",
            &format!(
                "FD9_GetPatchRecognitionResult2 : fiabilité {fiabilite}, zones {}, plages {}",
                coordonnees.zones, coordonnees.plages
            ),
        );
        c
    }

    fn arreter(&mut self, feuille_suivante: bool) -> i32 {
        let arguments = if feuille_suivante { "vrai" } else { "faux" };
        self.appel(
            "FD9_StopMeasurement",
            arguments,
            |s| s.arreter(feuille_suivante),
            code,
        )
    }

    fn sortir(&mut self) -> i32 {
        self.appel("FD9_ExitMeasurement", "0, nul", |s| s.sortir(), code)
    }

    fn erreur_premesure(&mut self, erreur_systeme: &mut i32) -> i32 {
        let c = self.appel(
            "FD9_GetPremeasurementError",
            "texte nul",
            |s| s.erreur_premesure(erreur_systeme),
            code,
        );
        self.ecrire(
            "·",
            &format!("FD9_GetPremeasurementError : erreur système {erreur_systeme}"),
        );
        c
    }

    fn points_mesure(&mut self, points: &mut [PointMesure]) -> i32 {
        let arguments = format!("{} points", points.len());
        self.appel(
            "FD9_GetMeasurePoint",
            &arguments,
            |s| s.points_mesure(points),
            code,
        )
    }

    fn mesurer(&mut self) -> i32 {
        self.appel("FD9_StartMeasurement2", "1, 1", |s| s.mesurer(), code)
    }

    fn regler(&mut self, cle: CleOption, valeur: u32) -> i32 {
        let arguments = format!("{cle:?}, {valeur}");
        self.appel("FD9_SetOption", &arguments, |s| s.regler(cle, valeur), code)
    }

    fn donnees(
        &mut self,
        rendue: &mut PositionPlage,
        donnees: &mut [f32; DONNEES_BRUTES],
        demandee: &PositionPlage,
    ) -> i32 {
        let arguments = format!(
            "plage {}, {} valeurs",
            demandee.zone_ou_index, DONNEES_BRUTES
        );
        let c = self.appel(
            "FD9_GetData",
            &arguments,
            |s| s.donnees(rendue, donnees, demandee),
            code,
        );
        self.ecrire(
            "·",
            &format!(
                "FD9_GetData : position (zone {}, rangée {}, colonne {})",
                rendue.zone_ou_index, rendue.rangee, rendue.colonne
            ),
        );
        c
    }

    fn calculer(&mut self, sortie: &mut [f32], brutes: &[f32; DONNEES_BRUTES]) -> i32 {
        let arguments = format!("sortie de {} valeurs", sortie.len());
        self.appel(
            "FD9_CalculateData",
            &arguments,
            |s| s.calculer(sortie, brutes),
            code,
        )
    }
}
