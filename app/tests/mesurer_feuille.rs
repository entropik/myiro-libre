//! Mesurer la RGB-2520 au FD-9 et la ranger dans une résolution (ticket
//! #34), vu par l'application : pont FD-9 et mire simulés, moteur de profil
//! simulé, jamais la DLL ni l'instrument. Valeurs fictives (n° 12345678,
//! adresse 192.0.2.40).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::instrument::fd9::PLAFOND_FD9;
use app::instrument::feuille::{
    definition_rgb2520, ErreurFeuille, EtapeFeuille, FEUILLE_RGB2520, ZONE_RGB2520,
};
use app::instrument::parefeu::{AutorisationPareFeu, PareFeuSimule};
use app::instrument::{Etat, Instrument, DELAI_FEUILLE};
use app::mesurer_feuille::{lire_rgb2520, DOSSIER_MESURES_FD9};
use app::pont::{Architecture, Panne, Pont};
use bibliotheque::IdCondition;
use mires::{MireFd9, RGB2520_GEOMETRIE, RGB2520_NOM};
use pont_protocole::{
    AdresseReseau, Calcul, ConditionMesure, ConditionsCalcul, DefinitionFeuille, DonneesBrutes,
    Echantillonnage, ErreurPont, Geometrie, Horodatage, Identite, Illuminant, Info, InstrumentFd9,
    InstrumentMesurant, Lab, LectureFeuille, LiaisonFd9, Mesure, Observateur, Palier, Plage,
    Provenance, RemiseAuRepos, Reponse, Requete, Spectre, MODELE_FD9,
};
use profilage::{MoteurSimule, Recette};
use tempfile::TempDir;

const ADRESSE: &str = "192.0.2.40";

/// Couleur imprimée sur la plage `i` de la mire fictive, de 0 à 255 : un
/// cube RVB de 14 niveaux par composante (2 744 couleurs, les 2 520 premières).
fn octets_plage(i: usize) -> [u8; 3] {
    let niveau = |n: usize| (n * 255 / 13) as u8;
    [niveau(i / 196 % 14), niveau(i / 14 % 14), niveau(i % 14)]
}

/// Mire fictive de même forme que la RGB-2520 : 2 520 plages, valeurs de 0
/// à 100 comme `lire_mire_fd9` les rend (octet / 2,55).
fn mire() -> MireFd9 {
    MireFd9 {
        octets: Vec::new(),
        empreinte: "0".repeat(64),
        geometrie: RGB2520_GEOMETRIE,
        valeurs: (0..2_520)
            .map(|i| octets_plage(i).map(|o| o as f64 / 2.55))
            .collect(),
    }
}

/// Lab D50 approché d'une couleur RVB imprimée : sRVB, sans adaptation.
/// Assez vraisemblable pour le contrôle de cohérence du profil.
fn lab_de(rvb: [u8; 3]) -> [f32; 3] {
    let lineaire = rvb.map(|o| {
        let c = o as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    let [r, g, b] = lineaire;
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.9505;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.089;
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

/// Mesure d'une feuille telle que `pont-fd9` la rend : une plage par
/// couleur de la définition, conditions supposées.
fn mesure_de(mire: &DefinitionFeuille) -> Mesure {
    let plages = mire
        .couleurs
        .as_ref()
        .expect("couleurs envoyées au pont")
        .iter()
        .map(|&rvb| {
            let lab = Lab::new(lab_de(rvb)).unwrap();
            let spectre = || Spectre::new(vec![lab.valeurs()[0] / 100.0; 36]).unwrap();
            Plage::new(
                [spectre(), spectre(), spectre()],
                DonneesBrutes::new(vec![1_000.0; 152]).unwrap(),
                [lab, lab, lab],
            )
            .unwrap()
        })
        .collect();
    let supposee = |c| Info::Supposee(c);
    let provenance = Provenance {
        instrument: InstrumentMesurant {
            modele: MODELE_FD9.into(),
            numero_serie: 12_345_678,
            micrologiciel: "1.00".into(),
            code_produit: "fictif".into(),
        },
        version_sdk: [1, 3, 2],
        empreinte_dll: Info::Inconnue,
        version_pont: "simulé".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new("2026-10-30T15:15:00+01:00").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie: Geometrie::Feuille {},
        calcul: Calcul {
            libelle: "pont simulé, données fictives".into(),
            demande: Info::Confirmee(ConditionsCalcul {
                conditions_spectres: [
                    supposee(ConditionMesure::M0),
                    supposee(ConditionMesure::M1),
                    supposee(ConditionMesure::M2),
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
            plages: 2_520,
            fiabilite: 0,
            ecart_max: 101,
        }),
    };
    Mesure::new(plages, provenance).unwrap()
}

/// Requêtes reçues par les ponts lancés, avec le plafond de chacun.
type Journal = Rc<RefCell<Vec<(Palier, Requete)>>>;

/// Pont FD-9 simulé : répond comme `pont-fd9`, au plafond de son lancement.
struct PontFeuilleSimule {
    plafond: Palier,
    journal: Journal,
    /// Résultat imposé à `mesurer_feuille` ; une panne arrête le pont, comme
    /// `PontProcessus` arrêté de force.
    feuille: Option<Result<Reponse, Panne>>,
    arrete: bool,
}

impl Pont for PontFeuilleSimule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
        self.journal
            .borrow_mut()
            .push((self.plafond, requete.clone()));
        if self.arrete {
            return Err(Panne::Arret {
                code: None,
                detail: "pont arrêté".into(),
            });
        }
        if let (Requete::MesurerFeuille { .. }, Some(Err(panne))) = (requete, &self.feuille) {
            self.arrete = true;
            return Err(panne.clone());
        }
        Ok(match requete {
            Requete::Version {} => Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Inconnue,
            },
            Requete::Detecter {} => Reponse::InstrumentsFd9 {
                liste: vec![InstrumentFd9 {
                    liaison: Info::Confirmee(LiaisonFd9::Reseau),
                    adresse: ADRESSE.into(),
                    identifiant: Info::Supposee("12345678".into()),
                }],
            },
            Requete::ConnecterAdresse { .. } => Reponse::Connecte {
                identite: Identite {
                    numero_serie: 12_345_678,
                    micrologiciel: "1.00".into(),
                    code_produit: "fictif".into(),
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
            Requete::MesurerFeuille { mire } => match &self.feuille {
                Some(Ok(reponse)) => reponse.clone(),
                _ => Reponse::Mesure {
                    mesure: mesure_de(mire),
                    remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
                },
            },
            Requete::Fermer {} => Reponse::Ferme {},
            _ => Reponse::RequeteInvalide {
                detail: "hors du simulé".into(),
            },
        })
    }
}

/// Fausse DLL 32 bits : seul l'en-tête PE compte, elle n'est jamais chargée.
fn fausse_dll(chemin: &Path) {
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    let mut octets = vec![0u8; 0x48];
    octets[..2].copy_from_slice(b"MZ");
    octets[0x3c] = 0x40;
    octets[0x40..0x44].copy_from_slice(b"PE\0\0");
    octets[0x44..0x46].copy_from_slice(&0x014c_u16.to_le_bytes());
    std::fs::write(chemin, octets).unwrap();
}

/// Bibliothèque temporaire, une résolution, FD-S2w installé, et un FD-9
/// détecté par son pont simulé.
struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    condition: IdCondition,
    journal: Journal,
    /// Réponse imposée à `mesurer_feuille`, pour les ponts lancés ensuite.
    feuille: Feuille,
    instrument: Instrument<PontFeuilleSimule>,
}

/// Résultat imposé à `mesurer_feuille`, partagé avec le lanceur.
type Feuille = Rc<RefCell<Option<Result<Reponse, Panne>>>>;

fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    let sdk = dossier.path().join("FD-S2w");
    fausse_dll(&sdk.join("Module").join("FD9SDK.dll"));
    let journal = Journal::default();
    let feuille = Rc::new(RefCell::new(None));
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        regle: Some(PathBuf::from("pont-fd9-x86.exe")),
        ..Default::default()
    });
    let instrument = Instrument::ouvrir_fd9(
        &[sdk],
        &[(Architecture::X86, PathBuf::from("pont-fd9-x86.exe"))],
        lanceur(&journal, &feuille),
        &mut autorisation,
    );
    Banc {
        dossier,
        ouverte,
        condition,
        journal,
        feuille,
        instrument,
    }
}

fn lanceur(
    journal: &Journal,
    feuille: &Feuille,
) -> impl FnMut(&Path, &Path, Palier) -> Result<PontFeuilleSimule, Panne> {
    let (journal, feuille) = (journal.clone(), feuille.clone());
    move |_, _, plafond| {
        Ok(PontFeuilleSimule {
            plafond,
            journal: journal.clone(),
            feuille: feuille.borrow().clone(),
            arrete: false,
        })
    }
}

impl Banc {
    fn bibliotheque(&self) -> PathBuf {
        emplacement(self.dossier.path())
    }

    /// Mesure la feuille avec la mire donnée ; les étapes annoncées sont
    /// rendues avec le résultat.
    fn mesurer(
        &mut self,
        mire: Result<MireFd9, String>,
    ) -> (
        Result<app::mesurer_feuille::FeuilleRangee, app::mesurer_feuille::EchecFeuille>,
        Vec<EtapeFeuille>,
    ) {
        let mut etapes = Vec::new();
        let dossier = self.bibliotheque();
        let resultat = self.ouverte.mesurer_feuille_fd9(
            self.condition,
            &mut self.instrument,
            mire,
            &dossier,
            &mut lanceur(&self.journal, &self.feuille),
            &mut |etape| etapes.push(etape),
        );
        (resultat, etapes)
    }

    /// Plafonds des ponts lancés, dans l'ordre.
    fn plafonds(&self) -> Vec<Palier> {
        let mut plafonds: Vec<Palier> = Vec::new();
        for (plafond, requete) in self.journal.borrow().iter() {
            if matches!(requete, Requete::Version {}) {
                plafonds.push(*plafond);
            }
        }
        plafonds
    }
}

/// Le 30 octobre 2026 à 16 h, UTC.
fn maintenant() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_793_376_000)
}

/// En-tête ICC RVB minimal de 132 octets : taille, `RGB `, `acsp`.
fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

#[test]
fn la_definition_rgb2520_correspond_a_la_geometrie_de_la_mire() {
    let definition = definition_rgb2520(&mire()).unwrap();
    assert_eq!(definition.nom, RGB2520_NOM);
    assert_eq!((definition.largeur, definition.hauteur), FEUILLE_RGB2520);
    assert_eq!(definition.zones, vec![ZONE_RGB2520]);
    let zone = &definition.zones[0];
    let g = RGB2520_GEOMETRIE;
    assert_eq!(
        (zone.colonnes as usize, zone.rangees as usize),
        (g.colonnes, g.rangees)
    );
    assert_eq!((zone.colonnes, zone.rangees), (42, 60));
    // 71 px à 300 ppp = 601,1 en 1/100 mm : 601 à 1/100 mm près.
    let pas_tiff = g.pas_px as f64 / 300.0 * 2_540.0;
    for pas in [
        zone.pas_x,
        zone.pas_y,
        zone.largeur_plage,
        zone.hauteur_plage,
    ] {
        assert_eq!(pas, 601);
        assert!((pas as f64 - pas_tiff).abs() < 1.0, "{pas} / {pas_tiff}");
    }
    // Une couleur par plage, les octets imprimés, dans l'ordre de la mire.
    let couleurs = definition.couleurs.expect("couleurs de la mire");
    assert_eq!(couleurs.len(), 2_520);
    for (i, couleur) in couleurs.iter().enumerate() {
        assert_eq!(*couleur, octets_plage(i), "plage {}", i + 1);
    }
}

#[test]
fn le_plafond_par_defaut_de_l_application_est_connexion() {
    let b = banc();
    assert_eq!(PLAFOND_FD9, Palier::Connexion);
    assert_eq!(b.plafonds(), vec![Palier::Connexion]);
    // L'ouverture ne connecte pas : le FD-9 reste libre.
    assert!(matches!(b.instrument.etat(), Etat::Detecte { .. }));
    let requetes: Vec<Requete> = b.journal.borrow().iter().map(|(_, r)| r.clone()).collect();
    assert_eq!(requetes, vec![Requete::Version {}, Requete::Detecter {}]);
}

#[test]
fn apres_la_mesure_le_pont_revient_au_plafond_connexion() {
    let mut b = banc();
    let (resultat, etapes) = b.mesurer(Ok(mire()));
    resultat.unwrap();
    // Ouverture, feuille, retour : le plafond Feuille pour la mesure seule.
    assert_eq!(
        b.plafonds(),
        vec![Palier::Connexion, Palier::Feuille, Palier::Connexion]
    );
    let feuille: Vec<Requete> = b
        .journal
        .borrow()
        .iter()
        .filter(|(p, _)| *p == Palier::Feuille)
        .map(|(_, r)| r.clone())
        .collect();
    assert_eq!(feuille.len(), 4, "{feuille:?}");
    assert_eq!(feuille[0], Requete::Version {});
    assert_eq!(
        feuille[1],
        Requete::ConnecterAdresse {
            adresse: AdresseReseau::new(ADRESSE).unwrap()
        }
    );
    assert!(matches!(&feuille[2], Requete::MesurerFeuille { mire } if mire.plages() == 2_520));
    assert_eq!(feuille[3], Requete::Fermer {});
    // Le pont ordinaire a été fermé avant, l'instrument est de nouveau détecté.
    assert!(b
        .journal
        .borrow()
        .iter()
        .any(|(p, r)| *p == Palier::Connexion && *r == Requete::Fermer {}));
    assert!(matches!(b.instrument.etat(), Etat::Detecte { .. }));
    assert_eq!(
        etapes,
        vec![
            EtapeFeuille::Connexion,
            EtapeFeuille::GlisserFeuille,
            EtapeFeuille::Rangement
        ]
    );
}

#[test]
fn une_feuille_mesuree_entre_dans_la_resolution_comme_mesure_importee() {
    let mut b = banc();
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    assert_eq!(rangee.plages, 2_520);
    assert_eq!(rangee.fichier, "FD-9 RGB-2520 2026-10-30 1515.txt");
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.importees.len(), 1);
    let importee = &detail.importees[0];
    assert_eq!(importee.id, rangee.mesure);
    assert_eq!(importee.fichier, rangee.fichier);
    assert_eq!(importee.plages, 2_520);
    assert_eq!(importee.instrument, Info::Confirmee("FD-9".into()));
    let contenu = b.ouverte.detail_importee(rangee.mesure).unwrap().contenu;
    assert!(
        contenu.myiro_libre,
        "provenance du pont FD-9 dans l'en-tête"
    );
    // Le calcul du profil y voit les trois conditions, supposées par le pont.
    assert_eq!(
        detail.sources[0].conditions,
        vec![
            ConditionMesure::M0,
            ConditionMesure::M1,
            ConditionMesure::M2
        ]
    );
    assert_eq!(detail.sources[0].raison, None);
}

#[test]
fn la_mesure_importee_sert_au_calcul_du_profil() {
    let mut b = banc();
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    let resultat = b
        .ouverte
        .calculer_profil(
            b.condition,
            rangee.mesure,
            ConditionMesure::M1,
            Recette::Standard,
            &MoteurSimule::reussit(icc()),
            maintenant(),
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(resultat.plages, 2_520);
    let detail = b.ouverte.detail_reglage(b.condition).unwrap();
    assert_eq!(detail.actif.map(|p| p.id), Some(resultat.profil));
}

#[test]
fn le_json_complet_est_garde_a_cote() {
    let mut b = banc();
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    let attendu = b
        .bibliotheque()
        .join(DOSSIER_MESURES_FD9)
        .join(format!("{}.json", rangee.mesure.0));
    assert_eq!(rangee.json.as_deref(), Some(attendu.as_path()));
    // La mesure au format conservable `myiro-libre/mesure/1`, sans perte :
    // spectres, données brutes et provenance du pont, rien d'autre.
    let texte = std::fs::read_to_string(&attendu).unwrap();
    assert!(texte.contains("\"myiro-libre/mesure/1\""), "{texte:.200}");
    assert!(!texte.contains("remise_au_repos"));
    let mesure = pont_protocole::lire_mesure(&texte).unwrap();
    assert_eq!(mesure, mesure_de(&definition_rgb2520(&mire()).unwrap()));
}

#[test]
fn sans_mire_rgb2520_la_mesure_est_refusee_en_clair() {
    let mut b = banc();
    let (resultat, etapes) = b.mesurer(lire_rgb2520(None));
    assert_eq!(resultat.unwrap_err().cle, "mire.erreur.fd9_introuvable");
    // Rien n'est lancé ni rangé.
    assert_eq!(b.plafonds(), vec![Palier::Connexion]);
    assert_eq!(etapes, vec![]);
    assert!(b
        .ouverte
        .detail_reglage(b.condition)
        .unwrap()
        .importees
        .is_empty());
    // Un autre fichier que la RGB-2520 retenue : refusé, sans rien lancer.
    let autre = b.dossier.path().join("autre.tif");
    std::fs::write(&autre, b"pas la mire").unwrap();
    let refus = lire_rgb2520(Some(&autre)).unwrap_err();
    assert_eq!(refus, "mire.erreur.fd9_autre");
    for cle in ["mire.erreur.fd9_introuvable", "mire.erreur.fd9_autre"] {
        assert!(app::textes::cles().any(|c| c == cle), "{cle}");
    }
}

/// Le pont attend la feuille 2 min, chaque étape 30 s, la mesure 10 min
/// (`pont_fd9::Delais`), puis lit 2 520 plages : l'application lui laisse
/// 20 min avant de le dire bloqué, et pas plus longtemps aux autres demandes.
#[test]
fn la_feuille_a_le_temps_d_etre_mesuree() {
    assert_eq!(DELAI_FEUILLE, Duration::from_secs(20 * 60));
    let delais = app::instrument::Delais::default();
    let feuille = Requete::MesurerFeuille {
        mire: definition_rgb2520(&mire()).unwrap(),
    };
    assert_eq!(delais.reponse(&feuille), DELAI_FEUILLE);
    assert_eq!(
        delais.reponse(&Requete::Version {}),
        Duration::from_secs(30)
    );
}

/// Le pont ne répond pas à la feuille dans [`DELAI_FEUILLE`] : arrêté de
/// force (`PontProcessus`), il est dit bloqué ; rien n'est rangé et le pont
/// ordinaire est relancé.
#[test]
fn un_delai_depasse_en_pleine_mesure_ne_range_rien_et_relance_le_pont_ordinaire() {
    let mut b = banc();
    *b.feuille.borrow_mut() = Some(Err(Panne::SansReponse {
        detail: "aucune réponse en 1200000 ms à mesurer_feuille ; pont arrêté de force".into(),
    }));
    let echec = b.mesurer(Ok(mire())).0.unwrap_err();
    assert_eq!(echec.cle, "probleme.pont_bloque");
    assert!(echec.detail.unwrap().contains("arrêté de force"));
    assert_eq!(
        b.plafonds(),
        vec![Palier::Connexion, Palier::Feuille, Palier::Connexion]
    );
    assert!(matches!(b.instrument.etat(), Etat::Detecte { .. }));
    assert!(b
        .ouverte
        .detail_reglage(b.condition)
        .unwrap()
        .importees
        .is_empty());
    assert!(!b.bibliotheque().join(DOSSIER_MESURES_FD9).exists());
}

/// La mire a été lue, mais elle n'a pas une couleur par plage de la
/// RGB-2520 : la définition est refusée sous son propre nom, rien n'est lancé.
#[test]
fn une_definition_refusee_n_est_pas_une_mire_illisible() {
    let mut b = banc();
    let mut courte = mire();
    courte.valeurs.pop();
    assert!(definition_rgb2520(&courte).is_err());
    let echec = b.mesurer(Ok(courte)).0.unwrap_err();
    assert_eq!(echec.cle, "feuille_fd9.definition_refusee");
    assert!(echec
        .detail
        .unwrap()
        .contains("2519 couleurs pour 2520 plages"));
    assert_eq!(b.plafonds(), vec![Palier::Connexion]);
}

#[test]
fn une_feuille_refusee_par_le_pont_ne_range_rien_et_revient_au_plafond_connexion() {
    let mut b = banc();
    *b.feuille.borrow_mut() = Some(Ok(Reponse::Erreur {
        erreur: ErreurPont::FeuilleAbsente {},
    }));
    let echec = b.mesurer(Ok(mire())).0.unwrap_err();
    assert_eq!(echec.cle, "feuille_fd9.feuille_absente");
    // Chaque refus du pont a sa clé, du nom de son type dans le protocole.
    for (erreur, cle) in [
        (
            ErreurPont::IncidentFeuille {
                etat: 102,
                premesure: None,
            },
            "feuille_fd9.incident_feuille",
        ),
        (
            ErreurPont::AutreMire {
                texte: "RGB-1617".into(),
            },
            "feuille_fd9.autre_mire",
        ),
        (
            ErreurPont::MesureInvraisemblable {
                correlation_milliemes: 120,
            },
            "feuille_fd9.mesure_invraisemblable",
        ),
        (ErreurPont::Delai {}, "feuille_fd9.delai"),
    ] {
        assert_eq!(ErreurFeuille::Feuille(erreur).cle(), cle);
    }
    assert_eq!(
        b.plafonds(),
        vec![Palier::Connexion, Palier::Feuille, Palier::Connexion]
    );
    assert!(matches!(b.instrument.etat(), Etat::Detecte { .. }));
    assert!(b
        .ouverte
        .detail_reglage(b.condition)
        .unwrap()
        .importees
        .is_empty());
}
