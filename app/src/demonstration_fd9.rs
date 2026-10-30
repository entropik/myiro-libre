//! FD-9 de démonstration (`cargo run -p app -- --demo`) : de quoi voir
//! l'écran « Mesurer une feuille au FD-9 » sans FD-9, sans DLL et sans la
//! mire du fabricant. Un pont simulé écrit l'avancement d'un vrai
//! `pont-fd9` (ticket #34), avec un calage et une image synthétiques, puis
//! rend une mesure fictive, ou l'erreur choisie par `--demo-feuille`.
//!
//! Tout est fictif : n° de série 12345678, adresse 192.0.2.40, couleurs
//! tirées d'un cube RVB mélangé, image dessinée d'après la mise en page.
//! Rien ne dépend de Tauri.

use std::path::Path;
use std::time::{Duration, Instant};

use mires::{MireFd9, RGB2520_GEOMETRIE};
use pont_protocole::{
    AvancementFeuille, CalageFeuille, Calcul, ConditionMesure, ConditionsCalcul, DefinitionFeuille,
    DonneesBrutes, Echantillonnage, EffetAnnulation, ErreurPont, Geometrie, Horodatage, Identite,
    Illuminant, ImageFeuille, Info, InstrumentFd9, InstrumentMesurant, Lab, LectureFeuille,
    LiaisonFd9, Mesure, Observateur, Palier, Plage, PointVise, Provenance, RemiseAuRepos, Reponse,
    Requete, Spectre, MODELE_FD9,
};

use crate::instrument::Instrument;
use crate::pont::{Annulation, Echange, Panne, Pont};

/// Adresse fictive du FD-9 de démonstration (bloc réservé à la
/// documentation).
pub const ADRESSE_DEMONSTRATION: &str = "192.0.2.40";

/// Rythme de la démonstration à l'écran : chaque étape dure environ une
/// seconde, la mesure des 2 520 plages une dizaine.
pub const RYTHME: Duration = Duration::from_secs(1);

/// Décalage de la feuille sur la grille, en 1/100 mm : celui de l'essai E4
/// ter, arrondi.
const DECALAGE: [i32; 2] = [870, -70];

/// Résolution de l'image synthétique, comme celle réduite par le pont.
const PPP: u32 = 60;

/// Comment finit une feuille de démonstration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueDemonstration {
    /// Mesurée et rangée.
    Mesure,
    /// Aucune feuille glissée dans le délai.
    FeuilleAbsente,
    /// Le QR désigne une autre mire.
    AutreMire,
    /// Plages trop loin de la grille.
    MireNonReconnue,
    /// Bourrage pendant la mesure (état 102).
    Bourrage,
    /// Mesure faite à côté des plages.
    MesureInvraisemblable,
}

impl IssueDemonstration {
    /// Valeur de `--demo-feuille` : `mesure`, `feuille_absente`,
    /// `autre_mire`, `mire_non_reconnue`, `bourrage` ou
    /// `mesure_invraisemblable`.
    pub fn depuis_argument(texte: &str) -> Option<Self> {
        Some(match texte {
            "mesure" => IssueDemonstration::Mesure,
            "feuille_absente" => IssueDemonstration::FeuilleAbsente,
            "autre_mire" => IssueDemonstration::AutreMire,
            "mire_non_reconnue" => IssueDemonstration::MireNonReconnue,
            "bourrage" => IssueDemonstration::Bourrage,
            "mesure_invraisemblable" => IssueDemonstration::MesureInvraisemblable,
            _ => return None,
        })
    }

    /// Issue demandée au lancement (`--demo-feuille <issue>`), sinon la
    /// mesure.
    pub fn du_lancement() -> Self {
        let args: Vec<String> = std::env::args().collect();
        args.windows(2)
            .find(|a| a[0] == "--demo-feuille")
            .and_then(|a| Self::depuis_argument(&a[1]))
            .unwrap_or(IssueDemonstration::Mesure)
    }
}

/// Mire fictive de la forme de la RGB-2520 : 2 520 couleurs d'un cube RVB
/// de 14 niveaux, mélangées dans un ordre fixe (comme la vraie mire, sans en
/// être une copie). Valeurs de 0 à 100, comme `lire_mire_fd9` les rend.
pub fn mire() -> MireFd9 {
    let niveau = |n: usize| (n * 255 / 13) as u8;
    let mut couleurs: Vec<[u8; 3]> = (0..14 * 14 * 14)
        .map(|i| [niveau(i / 196), niveau(i / 14 % 14), niveau(i % 14)])
        .collect();
    // Mélange de Fisher-Yates, graine fixe.
    let mut graine: u32 = 2_520;
    for i in (1..couleurs.len()).rev() {
        graine = graine.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        couleurs.swap(i, (graine >> 8) as usize % (i + 1));
    }
    MireFd9 {
        octets: Vec::new(),
        empreinte: "0".repeat(64),
        geometrie: RGB2520_GEOMETRIE,
        valeurs: couleurs
            .into_iter()
            .take(2_520)
            .map(|c| c.map(|o| f64::from(o) / 2.55))
            .collect(),
    }
}

/// FD-9 de démonstration, détecté, prêt à mesurer une feuille.
pub fn instrument(issue: IssueDemonstration, rythme: Duration) -> Instrument<PontDemonstrationFd9> {
    Instrument::ouvrir_fd9_simule(PontDemonstrationFd9::new(Palier::Connexion, issue, rythme))
}

/// Lanceur des ponts de démonstration, au plafond demandé.
pub fn lanceur(
    issue: IssueDemonstration,
    rythme: Duration,
) -> impl FnMut(&Path, &Path, Palier) -> Result<PontDemonstrationFd9, Panne> {
    move |_, _, plafond| Ok(PontDemonstrationFd9::new(plafond, issue, rythme))
}

/// Pont FD-9 simulé : répond comme `pont-fd9`, sans DLL.
pub struct PontDemonstrationFd9 {
    plafond: Palier,
    issue: IssueDemonstration,
    rythme: Duration,
}

impl PontDemonstrationFd9 {
    pub fn new(plafond: Palier, issue: IssueDemonstration, rythme: Duration) -> Self {
        PontDemonstrationFd9 {
            plafond,
            issue,
            rythme,
        }
    }

    /// Attend `duree` ; `false` si l'annulation est demandée entre-temps.
    fn patienter(&self, duree: Duration, annulation: &Annulation) -> bool {
        let fin = Instant::now() + duree;
        loop {
            if annulation.demandee() {
                return false;
            }
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                return true;
            }
            std::thread::sleep(reste.min(Duration::from_millis(20)));
        }
    }

    fn feuille(
        &self,
        mire: &DefinitionFeuille,
        annulation: &Annulation,
        suivi: &mut dyn FnMut(AvancementFeuille),
    ) -> Echange {
        let r = self.rythme;
        let erreur = |erreur| Echange {
            reponse: Ok(Reponse::Erreur { erreur }),
            annulation: None,
        };
        let annulee = Echange {
            reponse: Ok(Reponse::Erreur {
                erreur: ErreurPont::MesureAnnulee {
                    remise_au_repos: RemiseAuRepos::AuRepos {},
                },
            }),
            annulation: Some(Ok(EffetAnnulation::Appliquee)),
        };
        if !self.patienter(r * 3, annulation) {
            return annulee;
        }
        if self.issue == IssueDemonstration::FeuilleAbsente {
            return erreur(ErreurPont::FeuilleAbsente {});
        }
        suivi(AvancementFeuille::FeuilleEngagee {});
        if !self.patienter(r, annulation) {
            return annulee;
        }
        suivi(AvancementFeuille::Lecture {});
        if !self.patienter(r * 2, annulation) {
            return annulee;
        }
        match self.issue {
            IssueDemonstration::AutreMire => {
                return erreur(ErreurPont::AutreMire {
                    texte: "CMYK-2520 - 1P MY9*1+".into(),
                })
            }
            IssueDemonstration::MireNonReconnue => {
                return erreur(ErreurPont::MireNonReconnue {
                    zones: 1,
                    plages: 2_520,
                    ecart_max: 420,
                })
            }
            _ => {}
        }
        suivi(AvancementFeuille::CalagePret {
            calage: calage(mire),
        });
        let plages = mire.plages() as u32;
        let pas = r / 4;
        for mesurees in (0..=plages).step_by(60) {
            suivi(AvancementFeuille::Mesure { mesurees, plages });
            if self.issue == IssueDemonstration::Bourrage && mesurees >= plages / 2 {
                return erreur(ErreurPont::IncidentFeuille {
                    etat: 102,
                    premesure: None,
                });
            }
            std::thread::sleep(pas);
        }
        if self.issue == IssueDemonstration::MesureInvraisemblable {
            return erreur(ErreurPont::MesureInvraisemblable {
                correlation_milliemes: 140,
            });
        }
        // Calage déjà annoncé : la réponse n'en porte pas, comme un pont qui
        // ne le répéterait pas.
        Echange {
            reponse: Ok(Reponse::Mesure {
                mesure: mesure(mire),
                remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
                calage: None,
            }),
            annulation: None,
        }
    }
}

impl Pont for PontDemonstrationFd9 {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.demander_suivi(requete, &Annulation::default(), &mut |_| {})
            .reponse
    }

    fn demander_annulable(&mut self, requete: &Requete, annulation: &Annulation) -> Echange {
        self.demander_suivi(requete, annulation, &mut |_| {})
    }

    fn demander_suivi(
        &mut self,
        requete: &Requete,
        annulation: &Annulation,
        suivi: &mut dyn FnMut(AvancementFeuille),
    ) -> Echange {
        let reponse = match requete {
            Requete::Version {} => Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Inconnue,
            },
            Requete::Detecter {} => Reponse::InstrumentsFd9 {
                liste: vec![InstrumentFd9 {
                    liaison: Info::Confirmee(LiaisonFd9::Reseau),
                    adresse: ADRESSE_DEMONSTRATION.into(),
                    identifiant: Info::Supposee("12345678".into()),
                }],
            },
            Requete::ConnecterAdresse { .. } => Reponse::Connecte {
                identite: Identite {
                    numero_serie: 12_345_678,
                    micrologiciel: "1.00".into(),
                    code_produit: "démonstration".into(),
                    adresse_mac: "00:00:00:00:00:00".into(),
                    date_initiale: None,
                    anomalie_date_initiale: false,
                    brute_hex: String::new(),
                    recertification_due: false,
                },
            },
            Requete::MesurerFeuille { .. } if self.plafond < Palier::Feuille => Reponse::Erreur {
                erreur: ErreurPont::PalierNonAutorise {
                    demande: Palier::Feuille,
                    plafond: self.plafond,
                },
            },
            Requete::MesurerFeuille { mire } => return self.feuille(mire, annulation, suivi),
            Requete::Fermer {} => Reponse::Ferme {},
            Requete::Annuler {} => Reponse::Annulation {
                effet: EffetAnnulation::SansEffet,
            },
            _ => Reponse::RequeteInvalide {
                detail: "hors de la démonstration".into(),
            },
        };
        Echange {
            reponse: Ok(reponse),
            annulation: None,
        }
    }
}

/// Points visés et image synthétiques : la feuille décalée de [`DECALAGE`],
/// chaque point à quelques centièmes de millimètre du centre de sa plage,
/// l'image dessinée en gris d'après les couleurs de consigne.
pub fn calage(mire: &DefinitionFeuille) -> CalageFeuille {
    let [dx, dy] = DECALAGE;
    let mut points = Vec::with_capacity(mire.plages());
    let mut graine: u32 = 34;
    let mut alea = move || {
        graine = graine.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        ((graine >> 16) % 61) as i32 - 30
    };
    let mut ecart_max = 0u32;
    for (zone, z) in mire.zones.iter().enumerate() {
        for rangee in 0..z.rangees {
            for colonne in 0..z.colonnes {
                let (ex, ey) = (alea(), alea());
                ecart_max = ecart_max.max(ex.unsigned_abs()).max(ey.unsigned_abs());
                points.push(PointVise {
                    zone: zone as u32,
                    rangee,
                    colonne,
                    x: (z.gauche + z.pas_x * colonne + z.largeur_plage / 2) as i32 + dx + ex,
                    y: (z.haut + z.pas_y * rangee + z.hauteur_plage / 2) as i32 + dy + ey,
                });
            }
        }
    }
    CalageFeuille {
        decalage: DECALAGE,
        ecart_max,
        points,
        image: Some(image(mire)),
    }
}

/// Image en gris de la feuille, à [`PPP`] : papier clair, plages à la
/// clarté de leur couleur de consigne, décalées comme la feuille.
fn image(mire: &DefinitionFeuille) -> ImageFeuille {
    let px = |centiemes: u32| (u64::from(centiemes) * u64::from(PPP) / 2_540) as u32;
    let (largeur, hauteur) = (px(mire.largeur), px(mire.hauteur));
    let mut gris = vec![236u8; (largeur * hauteur) as usize];
    let couleurs = mire.couleurs.as_deref().unwrap_or_default();
    let mut n = 0;
    for z in &mire.zones {
        for rangee in 0..z.rangees {
            for colonne in 0..z.colonnes {
                let clarte = couleurs.get(n).map_or(128, |[r, v, b]| {
                    ((u32::from(*r) * 299 + u32::from(*v) * 587 + u32::from(*b) * 114) / 1_000)
                        as u8
                });
                n += 1;
                // Bords arrondis chacun depuis la feuille : pas de jour entre
                // deux plages jointives.
                let gauche = ((z.gauche + z.pas_x * colonne) as i32 + DECALAGE[0]).max(0) as u32;
                let haut = ((z.haut + z.pas_y * rangee) as i32 + DECALAGE[1]).max(0) as u32;
                let (x0, y0) = (px(gauche), px(haut));
                let (x1, y1) = (
                    px(gauche + z.largeur_plage).min(largeur),
                    px(haut + z.hauteur_plage).min(hauteur),
                );
                for y in y0..y1 {
                    for x in x0..x1 {
                        gris[(y * largeur + x) as usize] = clarte;
                    }
                }
            }
        }
    }
    ImageFeuille::new(largeur, hauteur, PPP, gris).expect("image de démonstration cohérente")
}

/// Lab D50 approché d'une couleur RVB imprimée : sRVB, sans adaptation.
fn lab_de([r, v, b]: [u8; 3]) -> [f32; 3] {
    let lineaire = |o: u8| {
        let c = f64::from(o) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, v, b) = (lineaire(r), lineaire(v), lineaire(b));
    let x = (0.4124 * r + 0.3576 * v + 0.1805 * b) / 0.9505;
    let y = 0.2126 * r + 0.7152 * v + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * v + 0.9505 * b) / 1.089;
    let f = |t: f64| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    [
        (116.0 * f(y) - 16.0) as f32,
        (500.0 * (f(x) - f(y))) as f32,
        (200.0 * (f(y) - f(z))) as f32,
    ]
}

/// Mesure fictive d'une feuille : une plage par couleur de la définition,
/// spectres plats à la clarté de la couleur, provenance du FD-9 de
/// démonstration.
fn mesure(mire: &DefinitionFeuille) -> Mesure {
    let plages = mire
        .couleurs
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|&rvb| {
            let lab = Lab::new(lab_de(rvb)).expect("Lab fini");
            let spectre = || Spectre::new(vec![lab.valeurs()[0] / 100.0; 36]).expect("spectre");
            Plage::new(
                [spectre(), spectre(), spectre()],
                DonneesBrutes::new(vec![1_000.0; 152]).expect("données brutes"),
                [lab, lab, lab],
            )
            .expect("plage fictive valable")
        })
        .collect();
    let maintenant = crate::parc::instant_utc(std::time::SystemTime::now());
    let provenance = Provenance {
        instrument: InstrumentMesurant {
            modele: MODELE_FD9.into(),
            numero_serie: 12_345_678,
            micrologiciel: "1.00".into(),
            code_produit: "démonstration".into(),
        },
        version_sdk: [1, 3, 2],
        empreinte_dll: Info::Inconnue,
        version_pont: "démonstration".into(),
        architecture: std::env::consts::ARCH.into(),
        horodatage: Horodatage::new(maintenant).expect("heure locale valable"),
        etalonnage: Info::Inconnue,
        geometrie: Geometrie::Feuille {},
        calcul: Calcul {
            libelle: "FD-9 de démonstration, données fictives".into(),
            demande: Info::Confirmee(ConditionsCalcul {
                conditions_spectres: [
                    Info::Supposee(ConditionMesure::M0),
                    Info::Supposee(ConditionMesure::M1),
                    Info::Supposee(ConditionMesure::M2),
                ],
                longueurs_onde: Info::Supposee(Echantillonnage {
                    debut_nm: 380,
                    pas_nm: 10,
                }),
                illuminant_lab: Info::Supposee(Illuminant::D50),
                observateur_lab: Info::Supposee(Observateur::DeuxDegres),
            }),
            observe: Info::Inconnue,
        },
        feuille: Some(LectureFeuille {
            qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
            zones: 1,
            plages: mire.plages() as u32,
            fiabilite: 0,
            ecart_max: 30,
        }),
    };
    Mesure::new(plages, provenance).expect("mesure fictive valable")
}
