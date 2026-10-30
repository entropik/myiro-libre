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
use app::mesurer_feuille::{
    etat_feuille_fd9, lire_calage, lire_rgb2520, DOSSIER_MESURES_FD9, RAISON_SANS_FD9,
};
use app::pont::{Architecture, Panne, Pont};
use bibliotheque::IdCondition;
use mires::{MireFd9, RGB2520_GEOMETRIE, RGB2520_NOM};
use pont_protocole::{
    AdresseReseau, AvancementFeuille, CalageFeuille, Calcul, ConditionMesure, ConditionsCalcul,
    DefinitionFeuille, DonneesBrutes, Echantillonnage, ErreurPont, Geometrie, Horodatage, Identite,
    Illuminant, ImageFeuille, Info, InstrumentFd9, InstrumentMesurant, Lab, LectureFeuille,
    LiaisonFd9, Mesure, Observateur, Palier, Plage, Provenance, RemiseAuRepos, Reponse, Requete,
    Spectre, MODELE_FD9,
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
    /// Lignes d'avancement écrites avant la réponse de la feuille ; vide :
    /// un pont d'avant le ticket #34.
    avancements: Vec<AvancementFeuille>,
    arrete: bool,
}

impl Pont for PontFeuilleSimule {
    fn demander_suivi(
        &mut self,
        requete: &Requete,
        _annulation: &app::pont::Annulation,
        suivi: &mut dyn FnMut(AvancementFeuille),
    ) -> app::pont::Echange {
        if matches!(requete, Requete::MesurerFeuille { .. }) && self.plafond == Palier::Feuille {
            for avancement in self.avancements.clone() {
                suivi(avancement);
            }
        }
        app::pont::Echange {
            reponse: self.demander(requete),
            annulation: None,
        }
    }

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
                    calage: None,
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
    /// Avancement écrit par les ponts lancés ensuite.
    avancements: Avancements,
    instrument: Instrument<PontFeuilleSimule>,
}

type Avancements = Rc<RefCell<Vec<AvancementFeuille>>>;

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
    let avancements = Avancements::default();
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        regle: Some(PathBuf::from("pont-fd9-x86.exe")),
        ..Default::default()
    });
    let instrument = Instrument::ouvrir_fd9(
        &[sdk],
        &[(Architecture::X86, PathBuf::from("pont-fd9-x86.exe"))],
        lanceur(&journal, &feuille, &avancements),
        &mut autorisation,
    );
    Banc {
        dossier,
        ouverte,
        condition,
        journal,
        feuille,
        avancements,
        instrument,
    }
}

fn lanceur(
    journal: &Journal,
    feuille: &Feuille,
    avancements: &Avancements,
) -> impl FnMut(&Path, &Path, Palier) -> Result<PontFeuilleSimule, Panne> {
    let (journal, feuille, avancements) = (journal.clone(), feuille.clone(), avancements.clone());
    move |_, _, plafond| {
        Ok(PontFeuilleSimule {
            plafond,
            journal: journal.clone(),
            feuille: feuille.borrow().clone(),
            avancements: avancements.borrow().clone(),
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
            &mut lanceur(&self.journal, &self.feuille, &self.avancements),
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

/// Sans `zxing.dll` à côté du pont, la feuille est refusée avant d'être
/// lancée (ticket #87) : c'est l'installation qui est incomplète, dite par
/// le problème `composant_qr_introuvable` et ses textes ; rien n'est rangé
/// et le pont ordinaire est relancé.
#[test]
fn une_feuille_refusee_faute_de_zxing_ne_range_rien_et_dit_le_composant_manquant() {
    let mut b = banc();
    let chemin = r"C:\Program Files\myiro-libre\zxing.dll";
    *b.feuille.borrow_mut() = Some(Ok(Reponse::Erreur {
        erreur: ErreurPont::ComposantQrIntrouvable {
            chemin: chemin.into(),
        },
    }));
    let echec = b.mesurer(Ok(mire())).0.unwrap_err();
    assert_eq!(echec.cle, "probleme.composant_qr_introuvable");
    assert_eq!(echec.detail.as_deref(), Some(chemin));
    for suite in ["cause", "action"] {
        let cle = format!("{}.{suite}", echec.cle);
        assert!(app::textes::cles().any(|c| c == cle), "{cle}");
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
    assert!(!b.bibliotheque().join(DOSSIER_MESURES_FD9).exists());
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
            "feuille_fd9.incident_feuille.102",
        ),
        // Un état d'incident hors de la table du SDK garde la clé générale.
        (
            ErreurPont::IncidentFeuille {
                etat: 150,
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

// ---- Avancement de la feuille et calage (ticket #34) ----

/// Calage synthétique : deux points, image de 4 × 3 pixels à 60 ppp.
fn calage(decalage: i32) -> CalageFeuille {
    CalageFeuille {
        decalage: [decalage, -110],
        ecart_max: 40,
        points: vec![
            pont_protocole::PointVise {
                zone: 0,
                rangee: 0,
                colonne: 0,
                x: 1_649 + decalage,
                y: 2_490,
            },
            pont_protocole::PointVise {
                zone: 0,
                rangee: 0,
                colonne: 1,
                x: 2_250 + decalage,
                y: 2_490,
            },
        ],
        image: Some(ImageFeuille::new(4, 3, 60, (0..12).collect()).unwrap()),
    }
}

fn avancement_complet(calage: CalageFeuille) -> Vec<AvancementFeuille> {
    vec![
        AvancementFeuille::FeuilleEngagee {},
        AvancementFeuille::Lecture {},
        AvancementFeuille::CalagePret { calage },
        AvancementFeuille::MesureLancee {},
        AvancementFeuille::Mesure {
            mesurees: 0,
            plages: 2_520,
        },
        AvancementFeuille::Mesure {
            mesurees: 2_520,
            plages: 2_520,
        },
    ]
}

/// L'avancement écrit par le pont est relayé à l'écran, dans l'ordre, entre
/// « glissez la feuille » et le rangement.
#[test]
fn l_avancement_du_pont_est_relaye_a_l_ecran_dans_l_ordre() {
    let mut b = banc();
    *b.avancements.borrow_mut() = avancement_complet(calage(880));
    let (resultat, etapes) = b.mesurer(Ok(mire()));
    resultat.unwrap();
    assert_eq!(
        etapes,
        vec![
            EtapeFeuille::Connexion,
            EtapeFeuille::GlisserFeuille,
            EtapeFeuille::FeuilleEngagee,
            EtapeFeuille::Lecture,
            EtapeFeuille::CalagePret {
                calage: calage(880)
            },
            EtapeFeuille::MesureLancee,
            EtapeFeuille::Mesure {
                mesurees: 0,
                plages: 2_520
            },
            EtapeFeuille::Mesure {
                mesurees: 2_520,
                plages: 2_520
            },
            EtapeFeuille::Rangement,
        ]
    );
}

/// L'étape « calage prêt » part à la page sans les pixels de l'image : ses
/// dimensions seulement (les pixels passent en binaire, ADR 0003).
#[test]
fn le_calage_annonce_a_la_page_n_a_pas_les_pixels() {
    let texte = serde_json::to_string(&EtapeFeuille::CalagePret {
        calage: calage(880),
    })
    .unwrap();
    assert!(
        texte.starts_with(r#"{"etape":"calage_pret","calage":{"decalage":[880,-110],"ecart_max":40,"points":[{"zone":0,"rangee":0,"colonne":0,"x":2529,"y":2490}"#),
        "{texte}"
    );
    assert!(
        texte.ends_with(r#""image":{"largeur":4,"hauteur":3,"ppp":60}}}"#),
        "{texte}"
    );
    assert!(!texte.contains("gris"), "{texte}");
    assert_eq!(
        serde_json::to_string(&EtapeFeuille::GlisserFeuille).unwrap(),
        r#"{"etape":"glisser_feuille"}"#
    );
    assert_eq!(
        serde_json::to_string(&EtapeFeuille::Mesure {
            mesurees: 12,
            plages: 2_520
        })
        .unwrap(),
        r#"{"etape":"mesure","mesurees":12,"plages":2520}"#
    );
    assert_eq!(
        serde_json::to_string(&EtapeFeuille::MesureLancee).unwrap(),
        r#"{"etape":"mesure_lancee"}"#
    );
}

/// Le calage est gardé à côté de la mesure complète, fichier local de la
/// bibliothèque (`mesures-fd9/<n°>.calage.json`), et se relit : c'est le
/// dernier de la résolution.
#[test]
fn le_calage_est_garde_a_cote_de_la_mesure_et_se_relit() {
    let mut b = banc();
    *b.avancements.borrow_mut() = avancement_complet(calage(880));
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    let attendu = b
        .bibliotheque()
        .join(DOSSIER_MESURES_FD9)
        .join(format!("{}.calage.json", rangee.mesure.0));
    assert_eq!(rangee.calage.as_deref(), Some(attendu.as_path()));
    assert_eq!(
        lire_calage(&b.bibliotheque(), rangee.mesure),
        Some(calage(880))
    );
    assert_eq!(
        b.ouverte.dernier_calage_fd9(b.condition, &b.bibliotheque()),
        Some((rangee.mesure, calage(880)))
    );

    // Une deuxième feuille : son calage devient le dernier.
    *b.avancements.borrow_mut() = avancement_complet(calage(500));
    let seconde = b.mesurer(Ok(mire())).0.unwrap();
    assert_ne!(seconde.mesure, rangee.mesure);
    assert_eq!(
        b.ouverte.dernier_calage_fd9(b.condition, &b.bibliotheque()),
        Some((seconde.mesure, calage(500)))
    );
}

/// Le calage de la réponse finale l'emporte sur celui de l'avancement ; un
/// pont d'avant l'avancement le donne seulement dans sa réponse.
#[test]
fn le_calage_de_la_reponse_finale_l_emporte() {
    let mut b = banc();
    *b.avancements.borrow_mut() = avancement_complet(calage(880));
    let mesure = mesure_de(&definition_rgb2520(&mire()).unwrap());
    *b.feuille.borrow_mut() = Some(Ok(Reponse::Mesure {
        mesure: mesure.clone(),
        remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
        calage: Some(calage(300)),
    }));
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    assert_eq!(
        lire_calage(&b.bibliotheque(), rangee.mesure),
        Some(calage(300))
    );

    b.avancements.borrow_mut().clear();
    let (rangee, etapes) = b.mesurer(Ok(mire()));
    let rangee = rangee.unwrap();
    assert_eq!(
        etapes,
        vec![
            EtapeFeuille::Connexion,
            EtapeFeuille::GlisserFeuille,
            EtapeFeuille::Rangement
        ]
    );
    assert_eq!(
        lire_calage(&b.bibliotheque(), rangee.mesure),
        Some(calage(300))
    );
}

/// Sans calage ni dans l'avancement ni dans la réponse, rien n'est écrit et
/// la résolution n'a pas de dernier calage.
#[test]
fn sans_calage_rien_n_est_garde() {
    let mut b = banc();
    let rangee = b.mesurer(Ok(mire())).0.unwrap();
    assert_eq!(rangee.calage, None);
    assert_eq!(lire_calage(&b.bibliotheque(), rangee.mesure), None);
    assert_eq!(
        b.ouverte.dernier_calage_fd9(b.condition, &b.bibliotheque()),
        None
    );
}

/// Une feuille refusée après le calage (mesure invraisemblable) ne garde
/// rien : ni mesure ni calage.
#[test]
fn une_feuille_refusee_apres_le_calage_ne_garde_rien() {
    let mut b = banc();
    *b.avancements.borrow_mut() = avancement_complet(calage(880));
    *b.feuille.borrow_mut() = Some(Ok(Reponse::Erreur {
        erreur: ErreurPont::MesureInvraisemblable {
            correlation_milliemes: 120,
        },
    }));
    let (resultat, etapes) = b.mesurer(Ok(mire()));
    assert_eq!(
        resultat.unwrap_err().cle,
        "feuille_fd9.mesure_invraisemblable"
    );
    assert!(etapes
        .iter()
        .any(|e| matches!(e, EtapeFeuille::CalagePret { .. })));
    assert!(!b.bibliotheque().join(DOSSIER_MESURES_FD9).exists());
}

// ---- Bouton « Mesurer une feuille au FD-9 » (tâche 9) ----

/// Actif quand le FD-9 est prêt et la mire trouvée ; sinon chaque raison
/// est une clé du catalogue. La mire part à l'onglet « Mire » dès qu'elle
/// est lue, même sans FD-9.
#[test]
fn le_bouton_dit_pourquoi_il_est_inactif() {
    let pret = etat_feuille_fd9(None, Ok(mire()), false);
    assert!(pret.disponible);
    assert!(pret.raisons.is_empty());
    assert_eq!(pret.mire.as_ref().map(|m| m.plages()), Some(2_520));

    let sans_fd9 = etat_feuille_fd9(Some(RAISON_SANS_FD9), Ok(mire()), false);
    assert!(!sans_fd9.disponible);
    assert_eq!(sans_fd9.raisons, vec![RAISON_SANS_FD9.to_string()]);
    assert!(sans_fd9.mire.is_some(), "la mire se voit sans FD-9");

    let sans_rien = etat_feuille_fd9(Some(RAISON_SANS_FD9), lire_rgb2520(None), false);
    assert!(!sans_rien.disponible);
    assert_eq!(
        sans_rien.raisons,
        vec![
            RAISON_SANS_FD9.to_string(),
            "mire.erreur.fd9_introuvable".to_string()
        ]
    );
    assert_eq!(sans_rien.mire, None);

    let mut courte = mire();
    courte.valeurs.pop();
    let refusee = etat_feuille_fd9(None, Ok(courte), false);
    assert!(!refusee.disponible);
    assert_eq!(
        refusee.raisons,
        vec!["feuille_fd9.definition_refusee.cause".to_string()]
    );

    for cle in pret
        .raisons
        .iter()
        .chain(&sans_rien.raisons)
        .chain(&refusee.raisons)
    {
        assert!(app::textes::cles().any(|c| c == cle), "{cle}");
    }
    assert!(etat_feuille_fd9(None, Ok(mire()), true).demonstration);
}

#[test]
fn le_fd9_detecte_permet_la_feuille() {
    let b = banc();
    assert!(b.instrument.feuille_possible());
}
