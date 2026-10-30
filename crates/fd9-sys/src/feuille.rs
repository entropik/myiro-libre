//! Structures et constantes de la mesure d'une feuille (fiches
//! `docs/abi/FD9_StartPatchRecognition.md`, `FD9_GetCaptureResult.md`,
//! `FD9_GetData.md`, `FD9_SetOption.md`, `FD9_RegisterDeviceEventHandler.md`).
//! Dispositions x86 confirmées par lecture statique de `FD9SDK.dll` 1.3.2.3 ;
//! la disposition x64 en découle (seul le pointeur de l'image change).

use pont_protocole::DefinitionFeuille;

use crate::{texte_c, ErreurParametre};

/// Zones par feuille, au plus (contrôle de `FD9_StartPatchRecognition`).
pub const ZONES_MAX: usize = 50;
/// Plages par feuille, au plus.
pub const PLAGES_MAX: usize = 15_000;
/// Rangées et colonnes d'une zone, au plus.
pub const RANGEES_MAX: u32 = 250;
pub const COLONNES_MAX: u32 = 60;
/// Données brutes d'une plage : 4 blocs de 38 (fiche `FD9_GetData`).
pub const DONNEES_BRUTES: usize = 152;

/// Valeurs de la clé [`CleOption::TypeDonnees`] : données brutes, spectre, Lab.
pub const DONNEES_BRUTES_TYPE: u32 = 0x0c;
pub const SPECTRE_TYPE: u32 = 0x0b;
pub const LAB_TYPE: u32 = 0;

/// Nombre de plages d'une zone (`{lignes, colonnes, plages}` du SDK).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompteZone {
    pub rangees: u32,
    pub colonnes: u32,
    pub plages: u32,
}

/// Position et pas d'une zone, en 1/100 mm : origine de la première plage,
/// taille d'une plage, pas d'une plage à la suivante.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeometrieZone {
    pub gauche: u32,
    pub haut: u32,
    pub largeur: u32,
    pub hauteur: u32,
    pub pas_x: u32,
    pub pas_y: u32,
}

/// `tFD9_Image` : 16 octets en x86, 24 en x64.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImageFd9 {
    pub largeur: u32,
    pub hauteur: u32,
    /// Bits par pixel : 8, 16 ou 24.
    pub bits: u32,
    pub donnees: *mut u8,
}

/// `tFD9_ChartCoordinate` : 0xd70 octets en x86, 0xd78 en x64.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct CoordonneesMire {
    /// Feuille, 1/100 mm.
    pub largeur: u32,
    pub hauteur: u32,
    /// Au plus [`ZONES_MAX`].
    pub zones: u32,
    /// Au plus [`PLAGES_MAX`].
    pub plages: u32,
    pub compte_zones: [CompteZone; ZONES_MAX],
    pub geometries: [GeometrieZone; ZONES_MAX],
    /// Toujours 0 ici : aucune image modèle n'est passée.
    pub modele_present: u32,
    /// Champs non décodés, laissés à zéro.
    pub reserve: [u8; 0xd60 - 0x71c],
    pub modele: ImageFd9,
}

impl CoordonneesMire {
    /// Tout à zéro, sans image modèle : tampon à passer à la DLL.
    pub fn vide() -> Box<Self> {
        Box::new(CoordonneesMire {
            largeur: 0,
            hauteur: 0,
            zones: 0,
            plages: 0,
            compte_zones: [CompteZone::default(); ZONES_MAX],
            geometries: [GeometrieZone::default(); ZONES_MAX],
            modele_present: 0,
            reserve: [0; 0xd60 - 0x71c],
            modele: ImageFd9 {
                largeur: 0,
                hauteur: 0,
                bits: 0,
                donnees: std::ptr::null_mut(),
            },
        })
    }

    /// Zéro partout, puis la définition, champ par champ et sans conversion
    /// d'unité, comme FD-S2w. Refuse, avant tout appel à la DLL, une
    /// définition sans zone, plus de 50 zones ou de 15 000 plages, une zone
    /// vide ou hors des bornes du SDK, et une zone qui déborde de la feuille
    /// (`gauche + pas × colonnes ≤ largeur`, de même en hauteur).
    pub fn depuis_definition(d: &DefinitionFeuille) -> Result<Box<Self>, ErreurParametre> {
        if d.zones.is_empty() {
            return Err(ErreurParametre::Vide);
        }
        if d.zones.len() > ZONES_MAX {
            return Err(ErreurParametre::TropDeZones { maximum: ZONES_MAX });
        }
        let mut c = CoordonneesMire::vide();
        c.largeur = d.largeur;
        c.hauteur = d.hauteur;
        let mut plages = 0usize;
        for (i, z) in d.zones.iter().enumerate() {
            let zone = i + 1;
            let dans_les_bornes = (1..=RANGEES_MAX).contains(&z.rangees)
                && (1..=COLONNES_MAX).contains(&z.colonnes)
                && [z.pas_x, z.pas_y, z.largeur_plage, z.hauteur_plage]
                    .iter()
                    .all(|&v| v > 0);
            if !dans_les_bornes {
                return Err(ErreurParametre::ZoneHorsBornes { zone });
            }
            // En u64 : aucun nombre reçu ne peut faire déborder le calcul.
            let droite = u64::from(z.gauche) + u64::from(z.pas_x) * u64::from(z.colonnes);
            let bas = u64::from(z.haut) + u64::from(z.pas_y) * u64::from(z.rangees);
            if droite > u64::from(d.largeur) || bas > u64::from(d.hauteur) {
                return Err(ErreurParametre::ZoneHorsFeuille { zone });
            }
            // Au plus 250 × 60 : le produit tient dans un u32.
            let compte = z.rangees * z.colonnes;
            plages += compte as usize;
            c.compte_zones[i] = CompteZone {
                rangees: z.rangees,
                colonnes: z.colonnes,
                plages: compte,
            };
            c.geometries[i] = GeometrieZone {
                gauche: z.gauche,
                haut: z.haut,
                largeur: z.largeur_plage,
                hauteur: z.hauteur_plage,
                pas_x: z.pas_x,
                pas_y: z.pas_y,
            };
        }
        if plages > PLAGES_MAX {
            return Err(ErreurParametre::TropDePlages {
                maximum: PLAGES_MAX,
            });
        }
        // Bornés par ZONES_MAX et PLAGES_MAX : les conversions sont exactes.
        c.zones = d.zones.len() as u32;
        c.plages = plages as u32;
        Ok(c)
    }
}

/// `tFD9_BarcodeData` : 1 172 octets, rempli par `FD9_GetCaptureResult`.
/// Pour un QR au format `011`, seuls l'appareil, la version et le texte
/// libre sont remplis.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct DonneesQr {
    /// 4 caractères sans zéro final (« FD-9 »).
    pub appareil: [u8; 4],
    /// 4 caractères sans zéro final (« 011M »).
    pub version: [u8; 4],
    /// Champs de l'ancien format de QR, non lus par le pont.
    pub ancien: [u8; 0x294 - 8],
    /// Texte libre du QR, 80 caractères au plus en format `011`.
    pub texte: [u8; 0x200],
}

impl DonneesQr {
    /// Tout à zéro, à passer à la DLL.
    pub fn vide() -> Box<Self> {
        Box::new(DonneesQr {
            appareil: [0; 4],
            version: [0; 4],
            ancien: [0; 0x294 - 8],
            texte: [0; 0x200],
        })
    }

    pub fn appareil(&self) -> String {
        texte_c(&self.appareil)
    }

    pub fn texte(&self) -> String {
        texte_c(&self.texte)
    }
}

/// Résultat du QR rendu par `FD9_GetCaptureResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultatQr {
    Trouve,
    Absent,
    /// QR lu, mais contenu refusé par le SDK (DATA_ERROR).
    Illisible,
    Inconnu(u32),
}

impl ResultatQr {
    pub fn depuis(code: u32) -> Self {
        match code {
            0 => ResultatQr::Trouve,
            1 => ResultatQr::Absent,
            2 => ResultatQr::Illisible,
            autre => ResultatQr::Inconnu(autre),
        }
    }
}

/// `tFD9_PatchPosition` : 12 octets. En entrée de `FD9_GetData`,
/// `(index, -1, -1)` désigne la plage n° index ; en sortie, la DLL rend
/// `(zone, rangée, colonne)`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PositionPlage {
    pub zone_ou_index: u32,
    pub rangee: i32,
    pub colonne: i32,
}

impl PositionPlage {
    /// La plage n° `index`, comptée depuis 0, toutes zones confondues.
    pub fn numero(index: u32) -> Self {
        PositionPlage {
            zone_ou_index: index,
            rangee: -1,
            colonne: -1,
        }
    }
}

/// `tFD9_PatchMeasurePoint` : 20 octets par plage, contenu non décodé.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointMesure(pub [u8; 20]);

/// Modes de `FD9_StartPatchRecognition` connus du pont. Les autres
/// (automatique, définition, matrice) exigent une image modèle ou des
/// couleurs que le pont ne passe pas.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeReconnaissance {
    /// Coordonnées seules : le mode du pont.
    Coordonnees = 2,
    /// Reconnaissance automatique comparée aux coordonnées (MYIROtools, qui
    /// passe toujours les couleurs des plages). Sans image modèle, le fil de
    /// reconnaissance de la DLL lit les couleurs sans tester le pointeur nul
    /// (`fd9-x86` 0x100a14f1, plantage de l'essai E3) : jamais appelé tant
    /// que le pont ne les passe pas.
    Combine = 4,
}

impl ModeReconnaissance {
    /// Vrai si la DLL lit les couleurs des plages quand aucune image modèle
    /// n'est donnée (fiche `FD9_StartPatchRecognition`).
    pub fn lit_les_couleurs_sans_modele(self) -> bool {
        self == ModeReconnaissance::Combine
    }
}

/// Clés de `FD9_SetOption` : réglages de calcul rangés dans la DLL, sans
/// échange avec l'instrument (fiche `FD9_SetOption`).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleOption {
    /// Condition de mesure (M0, M1, M2), moins de 16.
    Condition = 0,
    /// Illuminant des Lab, moins de 16.
    Illuminant = 1,
    /// Observateur des Lab, 0 ou 1.
    Observateur = 2,
    /// Type de données rendues, moins de 15.
    TypeDonnees = 3,
}

/// État du SDK reçu par le rappel d'événements (`FD9_eSDKStatus`). Noms du
/// SDK entre parenthèses, sans leur préfixe `SDK_` ou `SDK_WAIT_` (table de la
/// fiche `docs/abi/FD9_RegisterDeviceEventHandler.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatSdk {
    /// 0 (`WAIT_NO_AUTHORITY`) : main pas encore obtenue. Passage normal
    /// juste après la connexion (essai E3) ; s'il dure, un autre logiciel
    /// tient peut-être l'instrument (supposé).
    AttenteSansMain,
    /// 1 (`WAIT_AUTHORITY`) : main obtenue, attente d'une feuille.
    Attente,
    /// 2 (`RUN_FEED`) : feuille en cours d'engagement.
    Alimentation,
    /// 3 (`WAIT_FEED_COMPLETION`) : feuille engagée.
    AlimentationFinie,
    /// 4 (`RUN_CAPTURE`) : image en cours.
    Capture,
    /// 5 (`WAIT_CAPTURE_COMPLETION`) : image prise.
    CaptureFinie,
    /// 6 (`RUN_PATCHRECOG`) : reconnaissance des plages en cours.
    Reconnaissance,
    /// 7 (`WAIT_PATCHRECOG_COMPLETION`) : plages reconnues.
    ReconnaissanceFinie,
    /// 8 (`RUN_MEASURE`) : mesure en cours.
    Mesure,
    /// 9 (`WAIT_MEASURE_COMPLETION`) : mesure finie.
    MesureFinie,
    /// 10 (`WAIT_CANCEL_COMPLETION`) : arrêt fini.
    AnnulationFinie,
    /// 11 (`NOTYFY_PATCHCOUNT`) : nombre de plages déjà mesurées.
    AvisPlages,
    /// 101 (`ERROR_COVEROPEN`) : capot ouvert.
    CouvercleOuvert,
    /// 102 (`ERROR_PAPERJAM`) : bourrage.
    Bourrage,
    /// 103 (`ERROR_COMMUNICATION`) : liaison perdue.
    Communication,
    /// 201 (`ERROR_PREMEASURE`) : échec de la préparation.
    Premesure,
    /// 202 (`ERROR_MEASURE`) : échec de la mesure.
    ErreurMesure,
    /// 203 (`ERROR_SENSOR`) : erreur du capteur.
    Capteur,
    /// 204 (`ERROR_CALCULATION`) : erreur de calcul.
    Calcul,
    /// 205 (`ERROR_CHARTTOOLONG`) : feuille trop longue.
    FeuilleTropLongue,
    /// 206 (`ERROR_MEMORY`) : mémoire insuffisante.
    Memoire,
    /// 301 (`ERROR_FATAL`) : erreur fatale.
    Fatale,
    /// Code absent de la table du SDK : gardé brut, jamais interprété.
    Inconnu(u32),
}

const ETATS: [(u32, EtatSdk); 22] = [
    (0, EtatSdk::AttenteSansMain),
    (1, EtatSdk::Attente),
    (2, EtatSdk::Alimentation),
    (3, EtatSdk::AlimentationFinie),
    (4, EtatSdk::Capture),
    (5, EtatSdk::CaptureFinie),
    (6, EtatSdk::Reconnaissance),
    (7, EtatSdk::ReconnaissanceFinie),
    (8, EtatSdk::Mesure),
    (9, EtatSdk::MesureFinie),
    (10, EtatSdk::AnnulationFinie),
    (11, EtatSdk::AvisPlages),
    (101, EtatSdk::CouvercleOuvert),
    (102, EtatSdk::Bourrage),
    (103, EtatSdk::Communication),
    (201, EtatSdk::Premesure),
    (202, EtatSdk::ErreurMesure),
    (203, EtatSdk::Capteur),
    (204, EtatSdk::Calcul),
    (205, EtatSdk::FeuilleTropLongue),
    (206, EtatSdk::Memoire),
    (301, EtatSdk::Fatale),
];

impl EtatSdk {
    pub fn depuis(code: u32) -> Self {
        ETATS
            .iter()
            .find(|(c, _)| *c == code)
            .map_or(EtatSdk::Inconnu(code), |(_, etat)| *etat)
    }

    /// Le code brut du SDK, pour le journal et les erreurs.
    pub fn code(&self) -> u32 {
        match self {
            EtatSdk::Inconnu(code) => *code,
            etat => ETATS
                .iter()
                .find(|(_, e)| e == etat)
                .map_or(u32::MAX, |(c, _)| *c),
        }
    }

    /// Vrai pour les incidents connus (101 à 103, 201 à 206, 301). Un état
    /// inconnu n'est pas classé : le pont l'arrête à part.
    pub fn est_erreur(&self) -> bool {
        !matches!(self, EtatSdk::Inconnu(_)) && self.code() >= 101
    }
}
