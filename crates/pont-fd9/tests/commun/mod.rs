//! FD-9 simulé partagé par les tests : répond comme FD9SDK d'après docs/abi/FD9_*.
//! Il valide la logique du pont, pas l'ABI : seul l'instrument tranche.

#![allow(dead_code)]

use fd9_sys::{
    Appareil, CleOption, CoordonneesMire, CouleursMire, DonneesQr, EtatSdk, ModeReconnaissance,
    NomApplication, PointMesure, PositionPlage, TamponInfosSysteme, DONNEES_BRUTES,
    DONNEES_BRUTES_TYPE, LAB_TYPE, SPECTRE_TYPE,
};
use pont_fd9::{Annulation, Evenement, SdkFd9};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

/// SDK simulé, à échecs injectables ; note chaque appel.
#[derive(Default)]
pub struct Fd9Simule {
    /// Instruments que `FD9_GetDeviceList` trouve.
    pub appareils: Vec<Appareil>,
    /// Code public rendu par `FD9_GetDeviceList` quand une étape échoue.
    pub code_liste: i32,
    /// Code interne rendu ensuite par `FD9_GetLastError`.
    pub code_interne: i32,
    /// Compteur écrit par la DLL, s'il faut en simuler un faux.
    pub trouves_force: Option<u32>,
    pub version_fichier: Option<[u32; 4]>,
    pub empreinte: Option<String>,
    pub appels: Vec<String>,
    /// Code rendu par chaque export, par son nom (0 si absent).
    pub codes: HashMap<&'static str, i32>,
    /// Identité écrite par `FD9_GetSystemInfo` ; `None` : rien d'écrit.
    pub infos: Option<TamponInfosSysteme>,
    /// Rendu par `FD9_GetStatus`, l'état du SDK à +0x10 (n° 4) en plus.
    pub statut: [u32; 8],
    /// Rappel inscrit, session ouverte : comme les drapeaux de la DLL.
    pub inscrit: bool,
    pub connecte: bool,
    /// Événements déjà reçus par le rappel, pas encore lus par la session.
    pub recus: VecDeque<Evenement>,
    /// Événements que l'instrument enverra pendant la prochaine attente de
    /// la session (une feuille glissée, par exemple).
    pub a_venir: VecDeque<u32>,
    /// États envoyés au rappel après un appel réussi, par nom d'appel.
    pub reactions: HashMap<&'static str, Vec<u32>>,
    /// Rappels arrivés sans rappel inscrit.
    pub orphelins: u64,
    /// État du SDK : le dernier envoyé au rappel. Chaque fonction de la
    /// feuille n'est admise que dans certains états (1104 sinon), comme la DLL.
    pub etat: u32,
    /// Ce que `FD9_GetCaptureResult` rend : résultat, appareil et texte du QR,
    /// taille de l'image.
    pub qr_resultat: u32,
    pub qr_appareil: [u8; 4],
    pub qr_texte: String,
    pub taille_image: u32,
    /// Coordonnées reçues par `FD9_StartPatchRecognition`.
    pub coordonnees_recues: Option<Box<CoordonneesMire>>,
    /// Résultat de la reconnaissance : la définition reçue, décalée de
    /// `decalage` à gauche (zone 1), avec un autre nombre de plages si dit.
    pub decalage: u32,
    pub plages_reconnues: Option<u32>,
    pub fiabilite: u32,
    /// Cause rendue par `FD9_GetPremeasurementError`.
    pub premesure: i32,
    /// Annulation de l'opérateur, faite pendant l'attente d'un événement.
    pub annuler_pendant_l_attente: Option<Annulation>,
    /// Points demandés par `FD9_GetMeasurePoint` : les plages à mesurer.
    pub points: u32,
    /// Couleurs reçues par `FD9_StartPatchRecognition` (RVB de 0 à 255),
    /// dans l'ordre de la définition.
    pub couleurs_recues: Option<Vec<[f32; 4]>>,
    /// Décalage de la feuille vu sur l'image (x, y), en 1/100 mm : les
    /// points de mesure sont au centre théorique des plages, décalés d'autant.
    pub decalage_points: (i32, i32),
    /// Point rendu à la place du vrai, par rang de point.
    pub points_forces: HashMap<usize, PointMesure>,
    /// Les L* calculés sont ceux de la plage voisine de gauche : une mesure
    /// faite à côté des plages (essai E4 bis).
    pub lab_decale: bool,
    /// Réglages de `FD9_SetOption`, par clé (rangés dans la DLL).
    pub options: [u32; 4],
    /// Ordre de lecture : n° de la plage de la mire (ordre de la définition,
    /// depuis 0) rendue par chaque `FD9_GetData(index)`. `None` : serpentin,
    /// comme le FD-9 à l'essai E4 (rangées paires de gauche à droite,
    /// rangées impaires de droite à gauche, zone par zone).
    pub ordre: Option<Vec<u32>>,
    /// Position rendue à la place de la vraie, par index lu.
    pub positions_forcees: HashMap<u32, PositionPlage>,
    /// `FD9_GetData` rend 0 sans écrire la position.
    pub position_non_ecrite: bool,
    /// État que l'instrument atteint juste avant le prochain
    /// `FD9_StopMeasurement` : l'arrêt croise la fin de la mesure.
    pub avant_arret: Option<u32>,
    /// Comme `avant_arret`, mais le rappel n'annonce l'état qu'à l'attente
    /// suivante : la DLL y est passée avant que l'événement arrive.
    pub avant_arret_annonce_apres: Option<u32>,
    /// Valeurs qu'un spectre demande (`FD9_CalculateData`, 1005 au-delà de
    /// la sortie donnée).
    pub spectre_longueur: usize,
    /// `FD9_Disconnect` ne rend la main qu'au signal reçu ici (essai E2 : la
    /// DLL est restée bloquée dans l'appel). 10 s au plus, pour ne jamais
    /// bloquer un test.
    pub deconnexion_retenue: Option<std::sync::mpsc::Receiver<()>>,
    /// Codes rendus par les prochains `FD9_GetStatus`, dans l'ordre ; puis
    /// le code ordinaire.
    pub codes_statut: VecDeque<i32>,
    /// Après `FD9_Disconnect`, la DLL rend encore cet état avec le code 0, au
    /// lieu de 1102 (hors connexion).
    pub etat_apres_deconnexion: Option<u32>,
    /// Image de la feuille rendue par `FD9_GetChartImage` (largeur, hauteur
    /// à 300 ppp) ; `None` : aucune (refus 1999).
    pub image_feuille: Option<(u32, u32)>,
    /// Gris de chaque pixel (x, y) de cette image ; sans motif,
    /// `(x + y) % 256`. Toujours synthétique.
    pub motif_image: Option<fn(u32, u32) -> u8>,
    /// Dimensions écrites par `FD9_GetChartImage` avec le code 0, sans rien
    /// écrire d'autre : une DLL qui dirait plus que le tampon.
    pub dimensions_forcees: Option<(u32, u32)>,
    /// Comptes des événements 11 envoyés pendant la mesure, dans l'ordre ;
    /// `None` : un seul, toutes les plages ; vide : aucun.
    pub comptes_mesure: Option<Vec<u32>>,
}

/// Motif répété 32 fois : empreinte SHA-256 fictive de la DLL simulée.
pub const EMPREINTE_SIMULEE: &str = "5e";

/// FD-9 en réseau à l'adresse fictive 192.0.2.40, identifiant fictif.
pub fn fd9_reseau() -> Appareil {
    let mut appareil = Appareil::reseau("192.0.2.40").unwrap();
    appareil.identifiant.copy_from_slice(b"12345678");
    appareil
}

/// FD-9 branché en USB sur COM4, identifiant fictif.
pub fn fd9_usb() -> Appareil {
    let mut adresse = [0u8; 32];
    adresse[..4].copy_from_slice(b"COM4");
    Appareil {
        code_liaison: 1,
        adresse,
        identifiant: *b"87654321",
    }
}

/// Identité fictive, octet par octet, comme `FD9_GetSystemInfo` l'écrirait.
pub fn infos_fictives(numero_serie: &[u8; 8]) -> TamponInfosSysteme {
    let mut tampon = TamponInfosSysteme::vide();
    let i = &mut tampon.infos;
    i.version_sdk = [1, 0x20, 3];
    i.version_micrologiciel = [1, 2, 5];
    i.code_produit = *b"9C1A";
    i.numero_serie = *numero_serie;
    i.adresse_mac = [0x02, 0, 0, 0, 0, 0x01];
    i.nom_produit[..19].copy_from_slice(b"KONICA MINOLTA FD-9");
    tampon
}

pub fn evenement(etat: u32) -> Evenement {
    Evenement {
        etat: EtatSdk::depuis(etat),
        travail: 0,
        page: 0,
        compte: 0,
    }
}

impl Fd9Simule {
    /// La DLL de FD-S2w (1.3.2.3) et un FD-9 sur le réseau.
    pub fn avec_un_fd9() -> Self {
        Fd9Simule {
            appareils: vec![fd9_reseau()],
            version_fichier: Some([1, 3, 2, 3]),
            empreinte: Some(EMPREINTE_SIMULEE.repeat(32)),
            infos: Some(infos_fictives(b"12345678")),
            statut: [0; 8],
            ..Default::default()
        }
    }

    /// Un FD-9 sur le réseau, qui donne la main à la connexion ; l'opérateur
    /// glisse une RGB-2520 dès que la session l'attend. Réactions de l'instrument d'après les fiches.
    pub fn avec_une_rgb2520() -> Self {
        Fd9Simule {
            a_venir: [2, 3].into(),
            qr_appareil: *b"FD-9",
            qr_texte: "RGB-2520 - 1P MY9*1+".into(),
            taille_image: 1_000,
            image_feuille: Some((40, 25)),
            fiabilite: 0,
            spectre_longueur: 36,
            reactions: [
                // Essai E3 : 0 (main pas encore obtenue), puis 1.
                ("FD9_Connect", vec![0, 1]),
                ("FD9_StartCapture", vec![4, 5]),
                ("FD9_StartPatchRecognition", vec![6, 7]),
                ("FD9_StopMeasurement", vec![10]),
            ]
            .into(),
            ..Fd9Simule::avec_un_fd9()
        }
    }

    /// Code imposé à un export.
    pub fn rendre(mut self, export: &'static str, code: i32) -> Self {
        self.codes.insert(export, code);
        self
    }

    /// La DLL appelle le rappel : l'événement est reçu si un rappel est
    /// inscrit, compté comme orphelin sinon.
    pub fn rappeler(&mut self, etat: u32) {
        if self.inscrit {
            self.etat = etat;
            self.recus.push_back(evenement(etat));
        } else {
            self.orphelins += 1;
        }
    }

    /// La plage rendue par `FD9_GetData(index)` : son n° dans la mire et sa
    /// position (zone, rangée, colonne), selon les zones reçues par la
    /// reconnaissance.
    fn plage_lue(&self, index: u32) -> Option<(u32, PositionPlage)> {
        let c = self.coordonnees_recues.as_ref()?;
        let zones = &c.compte_zones[..c.zones as usize];
        let total: u32 = zones.iter().map(|z| z.rangees * z.colonnes).sum();
        // Serpentin : l'ordre de mesure du FD-9 (essai E4).
        let serpentin = |n: u32| {
            let mut reste = n;
            for (zone, z) in zones.iter().enumerate() {
                let compte = z.rangees * z.colonnes;
                if reste < compte {
                    let (rangee, k) = (reste / z.colonnes, reste % z.colonnes);
                    let colonne = if rangee % 2 == 0 {
                        k
                    } else {
                        z.colonnes - 1 - k
                    };
                    return (zone as u32, rangee, colonne);
                }
                reste -= compte;
            }
            unreachable!()
        };
        let numero_de = |zone: u32, rangee: u32, colonne: u32| {
            let avant: u32 = zones[..zone as usize]
                .iter()
                .map(|z| z.rangees * z.colonnes)
                .sum();
            avant + rangee * zones[zone as usize].colonnes + colonne
        };
        if index >= total {
            return None;
        }
        let (zone, rangee, colonne) = match &self.ordre {
            None => serpentin(index),
            Some(ordre) => {
                // n° dans la mire → position, dans l'ordre de la définition.
                let mut reste = *ordre.get(index as usize)?;
                let mut trouve = None;
                for (zone, z) in zones.iter().enumerate() {
                    let compte = z.rangees * z.colonnes;
                    if reste < compte {
                        trouve = Some((zone as u32, reste / z.colonnes, reste % z.colonnes));
                        break;
                    }
                    reste -= compte;
                }
                trouve?
            }
        };
        Some((
            numero_de(zone, rangee, colonne),
            PositionPlage {
                zone_ou_index: zone,
                rangee: rangee as i32,
                colonne: colonne as i32,
            },
        ))
    }

    fn appel(&mut self, nom: &'static str, detail: String) -> i32 {
        self.appels.push(if detail.is_empty() {
            nom.to_string()
        } else {
            format!("{nom} {detail}")
        });
        let code = self.codes.get(nom).copied().unwrap_or(0);
        if code == 0 {
            for etat in self.reactions.get(nom).cloned().unwrap_or_default() {
                self.rappeler(etat);
            }
        }
        code
    }

    pub fn appels_dll(&self) -> Vec<&str> {
        self.appels.iter().map(String::as_str).collect()
    }

    /// Comme la DLL : hors des états admis, 1104 sans rien faire.
    fn appel_admis(
        &mut self,
        nom: &'static str,
        detail: String,
        admis: impl Fn(u32) -> bool,
    ) -> i32 {
        if admis(self.etat) {
            self.appel(nom, detail)
        } else {
            self.appels
                .push(format!("{nom} {detail}").trim_end().to_string());
            1104
        }
    }
}

impl SdkFd9 for Fd9Simule {
    fn dernier_code(&mut self) -> i32 {
        self.appels.push("FD9_GetLastError".into());
        self.code_interne
    }

    fn lister(&mut self, tableau: &mut [Appareil], trouves: &mut u32) -> i32 {
        self.appels
            .push(format!("FD9_GetDeviceList {}", tableau.len()));
        // Comme la DLL : capacité de 1 à 100, compteur remis à zéro, tableau
        // effacé, au plus `capacité` entrées.
        if tableau.is_empty() || tableau.len() > 100 {
            return 1001;
        }
        *trouves = 0;
        for (place, appareil) in tableau.iter_mut().zip(&self.appareils) {
            *place = *appareil;
            *trouves += 1;
        }
        if let Some(force) = self.trouves_force {
            *trouves = force;
        }
        // Succès partiel : une étape en échec n'empêche pas de rendre ce qui
        // a été vu (fiche FD9_GetDeviceList).
        if self.code_liste != 0 && *trouves == 0 {
            self.code_liste
        } else {
            0
        }
    }

    fn version_fichier(&self) -> Option<[u32; 4]> {
        self.version_fichier
    }

    fn empreinte(&self) -> Option<String> {
        self.empreinte.clone()
    }

    fn inscrire_rappel(&mut self, actif: bool) -> i32 {
        let detail = if actif { "rappel" } else { "nul" };
        // Comme la DLL : refusé pendant une connexion (1103).
        if self.connecte {
            self.appels
                .push(format!("FD9_RegisterDeviceEventHandler {detail}"));
            return 1103;
        }
        let code = self.appel("FD9_RegisterDeviceEventHandler", detail.into());
        if code == 0 {
            self.inscrit = actif;
        }
        code
    }

    fn connecter(&mut self, appareil: &Appareil, nom: &NomApplication) -> i32 {
        let nom = String::from_utf8_lossy(nom.octets())
            .trim_end_matches('\0')
            .to_string();
        let code = self.appel("FD9_Connect", format!("{} {nom}", appareil.adresse()));
        if fd9_sys::connexion_etablie(code) {
            self.connecte = true;
        }
        code
    }

    fn infos(&mut self, tampon: &mut TamponInfosSysteme) -> i32 {
        let code = self.appel("FD9_GetSystemInfo", String::new());
        if code == 0 {
            if let Some(infos) = self.infos {
                *tampon = infos;
            }
        }
        code
    }

    fn statut(&mut self, tampon: &mut [u32; 8]) -> i32 {
        match self.codes_statut.pop_front() {
            Some(code) if code != 0 => {
                self.appels.push("FD9_GetStatus".into());
                return code;
            }
            _ => {}
        }
        // Comme la DLL : hors connexion, 1102 (fiche FD9_GetStatus).
        if !self.connecte && self.etat_apres_deconnexion.is_none() {
            self.appels.push("FD9_GetStatus".into());
            return 1102;
        }
        let code = self.appel("FD9_GetStatus", String::new());
        if code == 0 {
            *tampon = self.statut;
            tampon[4] = self.etat;
        }
        code
    }

    fn deconnecter(&mut self) -> i32 {
        if let Some(signal) = self.deconnexion_retenue.take() {
            let _ = signal.recv_timeout(Duration::from_secs(10));
        }
        let code = self.appel("FD9_Disconnect", String::new());
        // 1004 : la DLL n'avait plus de session (fiche FD9_Disconnect).
        if code == 0 || code == 1004 {
            self.connecte = false;
            if let Some(etat) = self.etat_apres_deconnexion {
                self.etat = etat;
            }
        }
        code
    }

    fn evenements(&mut self, attente: Duration) -> Vec<Evenement> {
        if self.recus.is_empty() && !attente.is_zero() {
            // L'instrument agit pendant que la session attend.
            if self.a_venir.is_empty() {
                if let Some(annulation) = self.annuler_pendant_l_attente.take() {
                    annulation.annuler();
                }
            }
            while let Some(etat) = self.a_venir.pop_front() {
                self.rappeler(etat);
            }
            if self.recus.is_empty() {
                std::thread::sleep(attente.min(Duration::from_millis(1)));
            }
        }
        self.recus.drain(..).collect()
    }

    fn rappels_orphelins(&self) -> u64 {
        self.orphelins
    }

    fn capturer(&mut self) -> i32 {
        self.appel_admis("FD9_StartCapture", String::new(), |e| e == 3)
    }

    fn resultat_capture(
        &mut self,
        qr_resultat: &mut u32,
        qr: &mut DonneesQr,
        taille: &mut u32,
        image: Option<&mut [u8]>,
    ) -> i32 {
        let detail = match &image {
            None => "image nulle".to_string(),
            Some(tampon) => format!("image {}", tampon.len()),
        };
        let code = self.appel_admis("FD9_GetCaptureResult", detail, |e| e == 5);
        if code == 0 {
            *qr_resultat = self.qr_resultat;
            // Rien de lu : la DLL ne touche pas au QR (fiche).
            if self.qr_resultat != 1 {
                qr.appareil = self.qr_appareil;
                qr.version = *b"011M";
                let texte = self.qr_texte.as_bytes();
                qr.texte[..texte.len()].copy_from_slice(texte);
            }
            if image.is_none() {
                *taille = self.taille_image;
            }
        }
        code
    }

    fn reconnaitre(
        &mut self,
        mode: ModeReconnaissance,
        coordonnees: &CoordonneesMire,
        couleurs: Option<&CouleursMire>,
    ) -> i32 {
        // Essai E3 : en COMBINE, sans image modèle, le fil de reconnaissance
        // de la DLL lit les couleurs sans tester le pointeur nul
        // (`fd9-x86` 0x100a14f1) et le pont s'arrête net. Le simulé fait de
        // même, en plus bruyant.
        assert!(
            mode != ModeReconnaissance::Combine
                || coordonnees.modele_present != 0
                || couleurs.is_some(),
            "FD9SDK lirait le pointeur nul des couleurs (COMBINE sans modèle ni couleurs)"
        );
        let teinte = couleurs.map_or("nulles".to_string(), |c| {
            format!("RVB {}", c.entrees().len())
        });
        let detail = format!(
            "mode {}, couleurs {teinte}, rangées 0, colonnes 0",
            mode as u32
        );
        let code = self.appel_admis("FD9_StartPatchRecognition", detail, |e| e == 5 || e == 7);
        if code == 0 {
            self.coordonnees_recues = Some(Box::new(coordonnees.clone()));
            self.couleurs_recues =
                couleurs.map(|c| c.entrees().iter().map(|e| e.valeurs).collect());
        }
        code
    }

    fn resultat_reconnaissance(
        &mut self,
        fiabilite: &mut u32,
        coordonnees: &mut CoordonneesMire,
    ) -> i32 {
        let code = self.appel_admis("FD9_GetPatchRecognitionResult2", String::new(), |e| e == 7);
        if code == 0 {
            *fiabilite = self.fiabilite;
            if let Some(recue) = &self.coordonnees_recues {
                *coordonnees = (**recue).clone();
                coordonnees.geometries[0].gauche += self.decalage;
                if let Some(plages) = self.plages_reconnues {
                    coordonnees.plages = plages;
                    coordonnees.compte_zones[0].plages = plages;
                }
            }
        }
        code
    }

    fn arreter(&mut self, feuille_suivante: bool) -> i32 {
        if let Some(etat) = self.avant_arret.take() {
            self.rappeler(etat);
        }
        if let Some(etat) = self.avant_arret_annonce_apres.take() {
            self.etat = etat;
            self.a_venir.push_back(etat);
        }
        let detail = if feuille_suivante { "vrai" } else { "faux" };
        self.appel_admis("FD9_StopMeasurement", detail.into(), |e| {
            (1..=8).contains(&e)
        })
    }

    fn sortir(&mut self) -> i32 {
        let fin = |e: u32| matches!(e, 9 | 10 | 101..=103 | 201..=206 | 301);
        let code = self.appel_admis("FD9_ExitMeasurement", "0 nul".into(), fin);
        if code == 0 {
            // Supposé : l'instrument attend une nouvelle feuille (fiche).
            self.etat = 1;
        }
        code
    }

    fn erreur_premesure(&mut self, erreur_systeme: &mut i32) -> i32 {
        self.appels.push("FD9_GetPremeasurementError".into());
        *erreur_systeme = 0;
        self.premesure
    }

    fn points_mesure(&mut self, points: &mut [PointMesure]) -> i32 {
        let code = self.appel_admis("FD9_GetMeasurePoint", points.len().to_string(), |e| e == 7);
        if code != 0 {
            return code;
        }
        self.points = points.len() as u32;
        // Centre théorique de chaque plage, dans l'ordre de la définition,
        // décalé comme la feuille vue sur l'image.
        if let Some(c) = &self.coordonnees_recues {
            let mut centres = Vec::new();
            for zone in 0..c.zones as usize {
                let (n, g) = (c.compte_zones[zone], c.geometries[zone]);
                for rangee in 0..n.rangees {
                    for colonne in 0..n.colonnes {
                        centres.push(PointMesure {
                            x: (g.gauche + g.pas_x * colonne + g.largeur / 2) as i32
                                + self.decalage_points.0,
                            y: (g.haut + g.pas_y * rangee + g.hauteur / 2) as i32
                                + self.decalage_points.1,
                            zone: zone as u32,
                            rangee,
                            colonne,
                        });
                    }
                }
            }
            for (i, point) in points.iter_mut().enumerate() {
                if let Some(centre) = centres.get(i) {
                    *point = *centre;
                }
                if let Some(force) = self.points_forces.get(&i) {
                    *point = *force;
                }
            }
        }
        0
    }

    fn image_feuille(&mut self, largeur: &mut u32, hauteur: &mut u32, gris: &mut [u8]) -> i32 {
        let detail = format!("{} × {}, 8 bits", *largeur, *hauteur);
        // Comme la DLL : refusée aux états 1 à 6 (fiche FD9_GetChartImage).
        let code = self.appel_admis("FD9_GetChartImage", detail, |e| !(1..=6).contains(&e));
        if code != 0 {
            return code;
        }
        if let Some((l, h)) = self.dimensions_forcees {
            *largeur = l;
            *hauteur = h;
            return 0;
        }
        let Some((l, h)) = self.image_feuille else {
            return 1999;
        };
        // Tampon trop court pour l'image : 1005, rien d'écrit.
        if u64::from(*largeur) * u64::from(*hauteur) < u64::from(l) * u64::from(h) {
            return 1005;
        }
        let motif = self.motif_image.unwrap_or(|x, y| ((x + y) % 256) as u8);
        for y in 0..h {
            for x in 0..l {
                gris[(y * l + x) as usize] = motif(x, y);
            }
        }
        *largeur = l;
        *hauteur = h;
        0
    }

    fn mesurer(&mut self) -> i32 {
        let code = self.appel_admis("FD9_StartMeasurement2", "1 1".into(), |e| e == 7);
        // Sans réaction imposée : mesure, nombre de plages mesurées, fin.
        if code == 0 && !self.reactions.contains_key("FD9_StartMeasurement2") {
            self.rappeler(8);
            let comptes = self
                .comptes_mesure
                .clone()
                .unwrap_or_else(|| vec![self.points]);
            if self.inscrit {
                for compte in comptes {
                    self.recus.push_back(Evenement {
                        compte,
                        ..evenement(11)
                    });
                }
            }
            self.rappeler(9);
        }
        code
    }

    fn regler(&mut self, cle: CleOption, valeur: u32) -> i32 {
        let code = self.appel("FD9_SetOption", format!("{} {valeur}", cle as u32));
        // Bornes de la DLL (fiche FD9_SetOption) : refus 1001.
        let borne = [16, 16, 2, 15][cle as usize];
        if code != 0 || valeur >= borne {
            return 1001;
        }
        self.options[cle as usize] = valeur;
        0
    }

    fn donnees(
        &mut self,
        rendue: &mut PositionPlage,
        donnees: &mut [f32; DONNEES_BRUTES],
        demandee: &PositionPlage,
    ) -> i32 {
        let i = demandee.zone_ou_index;
        let code = self.appel("FD9_GetData", i.to_string());
        if code != 0 {
            return code;
        }
        if self.options[CleOption::TypeDonnees as usize] != DONNEES_BRUTES_TYPE {
            // Le simulé ne calcule pas dans GetData.
            return 1001;
        }
        if self.position_non_ecrite {
            return 0;
        }
        let Some((numero, position)) = self.plage_lue(i) else {
            // Au-delà des plages mesurées (code interne 0x1775, supposé 1999).
            return 1999;
        };
        *rendue = self.positions_forcees.get(&i).copied().unwrap_or(position);
        // Données fictives : 1 000 + n° de la plage dans la mire, puis + k.
        for (k, valeur) in donnees.iter_mut().enumerate() {
            *valeur = 1_000.0 + numero as f32 + k as f32;
        }
        0
    }

    fn calculer(&mut self, sortie: &mut [f32], brutes: &[f32; DONNEES_BRUTES]) -> i32 {
        let code = self.appel("FD9_CalculateData", sortie.len().to_string());
        if code != 0 {
            return code;
        }
        let [condition, illuminant, observateur, type_donnees] = self.options;
        match type_donnees {
            SPECTRE_TYPE => {
                if sortie.len() < self.spectre_longueur {
                    return 1005;
                }
                for (k, valeur) in sortie.iter_mut().enumerate() {
                    *valeur = condition as f32 + k as f32 / 4.0;
                }
                0
            }
            LAB_TYPE if sortie.len() >= 3 => {
                // L* : clarté de la couleur imprimée sur la plage (celle de la
                // voisine si la mesure est décalée), 50 sans couleurs ;
                // a* : la condition posée ; b* : illuminant × 10 + observateur.
                let numero = (brutes[0] - 1_000.0) as usize;
                let lue = if self.lab_decale {
                    numero.saturating_sub(1)
                } else {
                    numero
                };
                let clarte = self
                    .couleurs_recues
                    .as_ref()
                    .and_then(|c| c.get(lue))
                    .map_or(50.0, |v| {
                        pont_fd9::calage::clarte_attendue([v[0] as u8, v[1] as u8, v[2] as u8])
                            as f32
                    });
                sortie[..3].copy_from_slice(&[
                    clarte,
                    condition as f32,
                    (illuminant * 10 + observateur) as f32,
                ]);
                0
            }
            _ => 1005,
        }
    }
}

/// Couleurs RVB fictives de `n` plages, sans ordre de clarté entre voisines
/// (générateur congruentiel fixe) : une mesure décalée d'une plage ne les
/// suit plus.
pub fn couleurs_fictives(n: usize) -> Vec<[u8; 3]> {
    let mut graine: u32 = 2_520;
    let mut suivant = move || {
        graine = graine.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (graine >> 16) as u8
    };
    (0..n).map(|_| [suivant(), suivant(), suivant()]).collect()
}
