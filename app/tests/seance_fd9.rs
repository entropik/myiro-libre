//! Séance de mesure d'un tirage au FD-9 (ticket #35), vue par
//! l'application : pont FD-9 simulé, exports FD-S2w synthétiques, moteur
//! de profil simulé ; jamais la DLL ni l'instrument. Valeurs fictives
//! (n° 12345678, adresse 192.0.2.40).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::instrument::feuille::EtapeFeuille;
use app::instrument::parefeu::{AutorisationPareFeu, PareFeuSimule};
use app::instrument::Instrument;
use app::mesurer_feuille::{EchecFeuille, DOSSIER_MESURES_FD9};
use app::pont::{Architecture, Panne, Pont};
use app::seance_fd9::{FinDemandee, RefusSeance, SourceAjoutee};
use app::verifier::{DemandeVerification, FicheTirage};
use bibliotheque::{EtatSeance, IdCondition, IdMesure, IdProfil, IdSeance, IdTirage, IdTolerances};
use mires::{MireFd9, RGB2520_GEOMETRIE};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DefinitionFeuille, DonneesBrutes, Echantillonnage,
    ErreurPont, Geometrie, Horodatage, Identite, Illuminant, Info, InstrumentFd9,
    InstrumentMesurant, Lab, LectureFeuille, LiaisonFd9, Mesure, Observateur, Palier, Plage,
    Provenance, RemiseAuRepos, Reponse, Requete, Spectre, MODELE_FD9,
};
use profilage::{Impression, MoteurSimule, MoteurVerificationSimule, Recette};
use seance_mesure::{Complement, Role};
use tempfile::TempDir;
use verification::{Criteres, Seuils};

const ADRESSE: &str = "192.0.2.40";
const PLAGES: usize = 2_520;

/// Couleur imprimée sur la plage `i` de la mire fictive, de 0 à 255.
fn octets_plage(i: usize) -> [u8; 3] {
    let niveau = |n: usize| (n * 255 / 13) as u8;
    [niveau(i / 196 % 14), niveau(i / 14 % 14), niveau(i % 14)]
}

/// Mire fictive de même forme que la RGB-2520.
fn mire() -> MireFd9 {
    MireFd9 {
        octets: b"mire fictive".to_vec(),
        empreinte: "0".repeat(64),
        geometrie: RGB2520_GEOMETRIE,
        valeurs: (0..PLAGES)
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

/// Mesure d'une feuille telle que `pont-fd9` la rend.
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
        horodatage: Horodatage::new("2026-11-02T15:15:00+01:00").unwrap(),
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

/// Réponse imposée à la prochaine feuille ; rien : une feuille mesurée.
type Feuille = Rc<RefCell<Option<Reponse>>>;

/// Pont FD-9 simulé : répond comme `pont-fd9`, au plafond de son lancement.
struct PontSimule {
    plafond: Palier,
    feuille: Option<Reponse>,
}

impl Pont for PontSimule {
    fn demander(&mut self, requete: &Requete) -> Result<Reponse, Panne> {
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
                Some(reponse) => reponse.clone(),
                None => Reponse::Mesure {
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

fn lanceur(feuille: &Feuille) -> impl FnMut(&Path, &Path, Palier) -> Result<PontSimule, Panne> {
    let feuille = feuille.clone();
    move |_, _, plafond| {
        Ok(PontSimule {
            plafond,
            feuille: feuille.borrow().clone(),
        })
    }
}

fn maintenant() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_793_610_000)
}

fn fiche(confirmee: bool) -> FicheTirage {
    FicheTirage {
        impression: Impression::SansGestion,
        logiciel: "Logiciel fictif".into(),
        support_pilote: "Papier fictif".into(),
        qualite: "Haute".into(),
        encres: "Pigmentaires".into(),
        remarques: String::new(),
        confirmee,
    }
}

/// Bibliothèque temporaire, une résolution, la mire préparée (un tirage) et
/// un FD-9 détecté par son pont simulé.
struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    condition: IdCondition,
    tirage: IdTirage,
    feuille: Feuille,
    instrument: Instrument<PontSimule>,
}

fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    let tirage = IdTirage(
        ouverte
            .preparer_mire_fd9(
                condition,
                mire(),
                &dossier.path().join("mires"),
                maintenant(),
            )
            .unwrap()
            .tirage,
    );
    let sdk = dossier.path().join("FD-S2w");
    fausse_dll(&sdk.join("Module").join("FD9SDK.dll"));
    let feuille = Feuille::default();
    let mut autorisation = AutorisationPareFeu::new(PareFeuSimule {
        regle: Some(PathBuf::from("pont-fd9-x86.exe")),
        ..Default::default()
    });
    let instrument = Instrument::ouvrir_fd9(
        &[sdk],
        &[(Architecture::X86, PathBuf::from("pont-fd9-x86.exe"))],
        lanceur(&feuille),
        &mut autorisation,
    );
    Banc {
        dossier,
        ouverte,
        condition,
        tirage,
        feuille,
        instrument,
    }
}

/// Une ligne d'un export FD-S2w fictif : RVB sur 255, Lab, spectre plat.
struct LigneExport {
    id: String,
    rvb: [f64; 3],
    lab: [f32; 3],
}

/// Les plages de la mire, telles que FD-S2w les exporterait, Lab décalé de
/// `decalage` sur L.
fn lignes(decalage: f32) -> Vec<LigneExport> {
    (0..PLAGES)
        .map(|i| {
            let o = octets_plage(i);
            let mut lab = lab_de(o);
            lab[0] += decalage;
            LigneExport {
                id: (i + 1).to_string(),
                rvb: o.map(f64::from),
                lab,
            }
        })
        .collect()
}

/// Export FD-S2w fictif, condition M1 supposée par la source « D50 ».
fn export_fd_s2w(instrument: &str, lignes: &[LigneExport]) -> String {
    let mut t = String::from(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"FD-S2w\"\r\n",
    );
    t += &format!("INSTRUMENTATION\t\"{instrument}\"\r\nSERIAL\t\"12345678\"\r\n");
    t += "MEASUREMENT_SOURCE\t\"D50\"\r\nCHAMP_INCONNU\t\"gardé tel quel\"\r\n";
    let mut champs = vec![
        "SAMPLE_ID".to_string(),
        "RGB_R".into(),
        "RGB_G".into(),
        "RGB_B".into(),
        "LAB_L".into(),
        "LAB_A".into(),
        "LAB_B".into(),
    ];
    champs.extend((0..36).map(|i| format!("nm{}", 380 + 10 * i)));
    t += &format!(
        "NUMBER_OF_FIELDS\t{}\r\nBEGIN_DATA_FORMAT\r\n",
        champs.len()
    );
    t += &champs.join("\t");
    t += &format!(
        "\r\nEND_DATA_FORMAT\r\nNUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n",
        lignes.len()
    );
    for l in lignes {
        let mut v = vec![l.id.clone()];
        v.extend(l.rvb.iter().map(|c| c.to_string()));
        v.extend(l.lab.iter().map(|c| c.to_string()));
        v.extend((0..36).map(|_| (l.lab[0] / 100.0).to_string()));
        t += &v.join("\t");
        t += "\r\n";
    }
    t += "END_DATA\r\n";
    t
}

const FD9: &str = "KONICA MINOLTA, INC. MYIRO-9 (FD-9), Serial No. 12345678";

impl Banc {
    fn bibliotheque(&self) -> PathBuf {
        emplacement(self.dossier.path())
    }

    fn commencer(&self) -> IdSeance {
        self.ouverte
            .commencer_seance_fd9(self.condition, self.tirage, &fiche(true), maintenant())
            .unwrap()
            .seance
    }

    fn mesurer(&mut self, seance: IdSeance) -> Result<SourceAjoutee, EchecFeuille> {
        let dossier = self.bibliotheque();
        self.ouverte.mesurer_exemplaire_fd9(
            seance,
            &mut self.instrument,
            &dossier,
            &mut lanceur(&self.feuille),
            &mut |_: EtapeFeuille| {},
        )
    }

    fn importer(&self, seance: IdSeance, texte: &str, role: Role) -> SourceAjoutee {
        self.ouverte
            .importer_dans_seance(
                seance,
                "export FD-S2w.txt",
                texte.as_bytes(),
                role,
                maintenant(),
            )
            .unwrap()
    }

    fn terminer(&self, seance: IdSeance, fin: FinDemandee) -> Result<IdMesure, RefusSeance> {
        self.ouverte
            .terminer_seance(seance, fin, maintenant() + Duration::from_secs(600))
    }

    fn raison_profil(&self, mesure: IdMesure) -> Option<&'static str> {
        self.ouverte
            .detail_reglage(self.condition)
            .unwrap()
            .sources
            .into_iter()
            .find(|s| s.mesure == mesure)
            .expect("mesure de la résolution")
            .raison
    }

    fn calculer(&self, mesure: IdMesure) -> Result<(), &'static str> {
        self.ouverte
            .calculer_profil(
                self.condition,
                mesure,
                ConditionMesure::M1,
                Recette::Rapide,
                &MoteurSimule::reussit(icc()),
                maintenant(),
                &mut |_| {},
            )
            .map(|_| ())
            .map_err(|e| e.cle)
    }
}

impl Banc {
    /// Profil calculé depuis une séance close d'une feuille, et un jeu de
    /// tolérances pour le vérifier.
    fn profil_et_tolerances(&mut self) -> (IdProfil, IdTolerances) {
        let seance = self.commencer();
        self.mesurer(seance).unwrap();
        let resultat = self.terminer(seance, SANS_CHOIX).unwrap();
        let profil = self
            .ouverte
            .calculer_profil(
                self.condition,
                resultat,
                ConditionMesure::M1,
                Recette::Rapide,
                &MoteurSimule::reussit(icc()),
                maintenant(),
                &mut |_| {},
            )
            .unwrap()
            .profil;
        let seuils = Seuils {
            moyenne: Some(2.0),
            p95: Some(4.0),
            max: Some(6.0),
        };
        let criteres = Criteres {
            fidelite: seuils,
            prediction: seuils,
            seuil_gamut: 1.0,
        };
        let tolerances = self
            .ouverte
            .enregistrer_tolerances("Atelier", &criteres, maintenant())
            .unwrap();
        (profil, tolerances)
    }

    /// Un nouveau tirage de la même mire, et sa séance.
    fn nouvelle_seance(&self) -> IdSeance {
        let tirage = self
            .ouverte
            .preparer_mire_fd9(
                self.condition,
                mire(),
                &self.dossier.path().join("mires"),
                maintenant(),
            )
            .unwrap()
            .tirage;
        self.ouverte
            .commencer_seance_fd9(self.condition, IdTirage(tirage), &fiche(true), maintenant())
            .unwrap()
            .seance
    }

    fn verifier(
        &self,
        profil: IdProfil,
        tolerances: IdTolerances,
        mesure: IdMesure,
    ) -> Result<(), &'static str> {
        self.ouverte
            .verifier_profil(
                &DemandeVerification {
                    profil,
                    mesure,
                    mesure_condition: ConditionMesure::M1,
                    tolerances,
                    fiche: fiche(true),
                },
                &MoteurVerificationSimule::fidele(),
                maintenant(),
            )
            .map(|_| ())
            .map_err(|e| e.cle)
    }

    /// Une mesure importée hors séance, comme au ticket #9.
    fn importer_hors_seance(&self, texte: &str) -> IdMesure {
        let fichier = self.dossier.path().join("hors seance.txt");
        std::fs::write(&fichier, texte).unwrap();
        let apercu = self.ouverte.apercu_cgats(&fichier).unwrap();
        self.ouverte
            .ranger_apercu(apercu.numero, self.condition)
            .unwrap()
    }
}

fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

const SANS_CHOIX: FinDemandee = FinDemandee {
    incomplet_assume: false,
    ecarts_vus: false,
};

fn cle(refus: Result<IdMesure, RefusSeance>) -> String {
    refus.unwrap_err().cle
}

#[test]
fn une_fiche_non_confirmee_ne_commence_rien() {
    let b = banc();
    let refus = b
        .ouverte
        .commencer_seance_fd9(b.condition, b.tirage, &fiche(false), maintenant())
        .unwrap_err();
    assert_eq!(refus.cle, "seance.erreur.fiche_non_confirmee");
    assert!(b.ouverte.seances_en_cours(b.condition).unwrap().is_empty());
}

#[test]
fn commencer_deux_fois_reprend_la_meme_seance() {
    let b = banc();
    let premiere = b
        .ouverte
        .commencer_seance_fd9(b.condition, b.tirage, &fiche(true), maintenant())
        .unwrap();
    assert!(!premiere.reprise);
    let seconde = b
        .ouverte
        .commencer_seance_fd9(b.condition, b.tirage, &fiche(true), maintenant())
        .unwrap();
    assert!(seconde.reprise);
    assert_eq!(seconde.seance, premiere.seance);
    assert_eq!(
        b.ouverte.seances_en_cours(b.condition).unwrap(),
        [premiere.seance]
    );
}

#[test]
fn une_reprise_avec_une_autre_fiche_est_refusee() {
    let b = banc();
    let seance = b.commencer();
    let mut autre = fiche(true);
    autre.encres = "Autres encres".into();
    let refus = b
        .ouverte
        .commencer_seance_fd9(b.condition, b.tirage, &autre, maintenant())
        .unwrap_err();
    assert_eq!(refus.cle, "seance.erreur.fiche_differente");
    // La fiche figée n'a pas changé.
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert_eq!(
        serde_json::from_str::<FicheTirage>(&etat.plan.fiche).unwrap(),
        fiche(true)
    );
}

#[test]
fn le_plan_est_fige_au_depart_avec_la_mire_et_la_fiche() {
    let b = banc();
    let seance = b.commencer();
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert_eq!(etat.plan.mire.empreinte, "0".repeat(64));
    assert_eq!(etat.plan.plages.len(), PLAGES);
    // La résolution d'une feuille de la séance est celle de la séance.
    assert_eq!(b.ouverte.condition_seance(seance).unwrap(), b.condition);
    assert_eq!(
        serde_json::from_str::<FicheTirage>(&etat.plan.fiche).unwrap(),
        fiche(true)
    );
    assert_eq!(etat.etat, EtatSeance::EnCours);
}

#[test]
fn une_feuille_mesuree_au_fd9_devient_une_mesure_source() {
    let mut b = banc();
    let seance = b.commencer();
    let source = b.mesurer(seance).unwrap();
    assert!(source.association.certaine());
    assert!(source.association.complete());
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert_eq!(etat.sources.len(), 1);
    assert_eq!(etat.sources[0].mesure, source.mesure);
    // La mesure complète du pont est gardée à côté, comme au ticket #34.
    assert!(b
        .bibliotheque()
        .join(DOSSIER_MESURES_FD9)
        .join(format!("{}.json", source.mesure.0))
        .is_file());
}

#[test]
fn une_feuille_refusee_n_ajoute_rien_et_la_seance_se_reprend() {
    let mut b = banc();
    let seance = b.commencer();
    *b.feuille.borrow_mut() = Some(Reponse::Erreur {
        erreur: ErreurPont::MireNonReconnue {
            zones: 1,
            plages: 2_520,
            ecart_max: 900,
        },
    });
    let echec = b.mesurer(seance).unwrap_err();
    assert_eq!(echec.cle, "feuille_fd9.mire_non_reconnue");
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert!(etat.sources.is_empty());
    assert_eq!(etat.etat, EtatSeance::EnCours);
    // La feuille remise en place, la séance reprend là où elle était.
    *b.feuille.borrow_mut() = None;
    b.mesurer(seance).unwrap();
    assert_eq!(b.ouverte.etat_seance(seance).unwrap().sources.len(), 1);
}

#[test]
fn les_feuilles_validees_survivent_a_la_fermeture_de_l_application() {
    let mut b = banc();
    let seance = b.commencer();
    let premiere = b.mesurer(seance).unwrap().mesure;
    let Banc {
        dossier,
        ouverte,
        condition,
        ..
    } = b;
    drop(ouverte);
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    assert_eq!(ouverte.seances_en_cours(condition).unwrap(), [seance]);
    let etat = ouverte.etat_seance(seance).unwrap();
    assert_eq!(etat.sources.len(), 1);
    assert_eq!(etat.sources[0].mesure, premiere);
    assert!(etat.sources[0].association.certaine());
}

#[test]
fn un_export_fd_s2w_est_garde_tel_quel_et_marque_importe() {
    let b = banc();
    let seance = b.commencer();
    let texte = export_fd_s2w(FD9, &lignes(0.0));
    let source = b.importer(seance, &texte, Role::Exemplaire);
    assert!(source.association.certaine());
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert_eq!(etat.sources[0].origine, bibliotheque::OrigineSource::Import);
    assert_eq!(etat.sources[0].fichier, "export FD-S2w.txt");
    // Le texte d'origine, champ inconnu compris, est gardé tel quel (déjà en UTF-8).
    assert_eq!(
        b.ouverte.original_source(seance, source.mesure).unwrap(),
        texte
    );
    // Seulement pour une mesure de cette séance.
    let ailleurs = b
        .ouverte
        .original_source(seance, IdMesure(9_999))
        .unwrap_err();
    assert_eq!(ailleurs.cle, "seance.erreur.source_inconnue");
}

#[test]
fn un_import_illisible_n_est_pas_range() {
    let b = banc();
    let seance = b.commencer();
    let refus = b
        .ouverte
        .importer_dans_seance(
            seance,
            "abime.txt",
            b"pas un CGATS",
            Role::Exemplaire,
            maintenant(),
        )
        .unwrap_err();
    assert_eq!(refus.cle, "bibliotheque.erreur.cgats_illisible");
    assert!(b.ouverte.etat_seance(seance).unwrap().sources.is_empty());
}

#[test]
fn un_import_incomplet_se_range_s_inspecte_et_ne_se_termine_que_par_choix() {
    let b = banc();
    let seance = b.commencer();
    let mut l = lignes(0.0);
    l.truncate(PLAGES - 20);
    let source = b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    assert!(source.association.certaine());
    assert_eq!(source.association.manquantes.len(), 20);
    let plages = b.ouverte.plages_source(seance, source.mesure).unwrap();
    assert_eq!(plages.len(), PLAGES);
    assert_eq!(plages[PLAGES - 1].etat, seance_mesure::EtatPlage::Manquante);
    assert_eq!(plages[0].rvb_attendu, mire().valeurs[0]);

    assert_eq!(
        cle(b.terminer(seance, SANS_CHOIX)),
        "seance.fin.plages_manquantes"
    );
    let resultat = b
        .terminer(
            seance,
            FinDemandee {
                incomplet_assume: true,
                ecarts_vus: false,
            },
        )
        .unwrap();
    assert_eq!(
        b.ouverte.etat_seance(seance).unwrap().etat,
        EtatSeance::Incomplete
    );
    // Le choix ne contourne pas les préconditions du profil.
    assert_eq!(
        b.raison_profil(resultat),
        Some("profil.refus.seance_incomplete")
    );
    assert_eq!(b.calculer(resultat), Err("profil.refus.seance_incomplete"));
    assert_eq!(
        b.raison_profil(source.mesure),
        Some("profil.refus.mesure_incomplete")
    );
}

#[test]
fn une_mauvaise_association_est_gardee_mais_bloque_le_profil_et_la_fin() {
    let b = banc();
    let seance = b.commencer();
    let mut l = lignes(0.0);
    l.swap(0, 1);
    l[0].id = "1".into();
    l[1].id = "2".into();
    let source = b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    assert!(!source.association.certaine());
    assert_eq!(
        b.raison_profil(source.mesure),
        Some("profil.refus.association_incertaine")
    );
    assert_eq!(
        b.calculer(source.mesure),
        Err("profil.refus.association_incertaine")
    );
    let refus = b.terminer(seance, SANS_CHOIX).unwrap_err();
    assert_eq!(refus.cle, "seance.refus.association_incertaine");
    assert!(refus.incompatible.is_some());
}

#[test]
fn un_exemplaire_ecarte_a_la_main_garde_son_motif() {
    let mut b = banc();
    let seance = b.commencer();
    b.mesurer(seance).unwrap();
    let mut l = lignes(0.0);
    l[5].rvb = [1.0, 2.0, 3.0];
    let douteux = b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    b.ouverte
        .retenir_source(
            seance,
            douteux.mesure,
            false,
            "export d'une autre feuille",
            maintenant(),
        )
        .unwrap();
    let etat = b.ouverte.etat_seance(seance).unwrap();
    let ecarte = etat
        .sources
        .iter()
        .find(|s| s.mesure == douteux.mesure)
        .unwrap();
    assert!(!ecarte.retenue);
    assert!(ecarte
        .selection
        .as_deref()
        .unwrap()
        .contains("export d'une autre feuille"));
    b.terminer(seance, SANS_CHOIX).unwrap();
    let composition = b.ouverte.etat_seance(seance).unwrap().composition.unwrap();
    assert_eq!(composition.exclusions.len(), 1);
    assert_eq!(composition.exclusions[0].source, douteux.mesure.0);
}

#[test]
fn un_myiro1_ne_se_moyenne_pas_avec_le_fd9() {
    let mut b = banc();
    let seance = b.commencer();
    b.mesurer(seance).unwrap();
    b.importer(
        seance,
        &export_fd_s2w(
            "KONICA MINOLTA, INC. MYIRO-1, Serial No. 12345678",
            &lignes(0.0),
        ),
        Role::Exemplaire,
    );
    let refus = b.terminer(seance, SANS_CHOIX).unwrap_err();
    assert_eq!(refus.cle, "seance.refus.modeles_differents");
    assert_eq!(
        b.ouverte.etat_seance(seance).unwrap().etat,
        EtatSeance::EnCours
    );
}

#[test]
fn une_feuille_et_un_export_se_moyennent_sur_m1_et_le_disent() {
    let mut b = banc();
    let seance = b.commencer();
    let feuille = b.mesurer(seance).unwrap().mesure;
    let export = b.importer(seance, &export_fd_s2w(FD9, &lignes(0.0)), Role::Exemplaire);
    // Pendant la séance, ses mesures ne font pas encore de profil.
    assert_eq!(
        b.raison_profil(feuille),
        Some("profil.refus.seance_en_cours")
    );

    let apercu = b.ouverte.etat_seance(seance).unwrap().apercu.unwrap();
    assert!(apercu.manquantes.is_empty());
    let ecartees: Vec<_> = apercu
        .conditions_ecartees
        .iter()
        .map(|c| (c.emplacement.as_str(), c.source))
        .collect();
    assert_eq!(ecartees, [("m0", feuille.0), ("m2", feuille.0)]);

    let resultat = b.terminer(seance, SANS_CHOIX).unwrap();
    let etat = b.ouverte.etat_seance(seance).unwrap();
    assert_eq!(etat.etat, EtatSeance::Terminee);
    assert_eq!(etat.resultat, Some(resultat));
    // La composition plage par plage est rangée en base.
    let composition = etat.composition.unwrap();
    assert_eq!(composition.composition.len(), PLAGES);
    let sources: Vec<i64> = composition.composition[0]
        .apports
        .iter()
        .map(|a| a.source)
        .collect();
    assert_eq!(sources, [feuille.0, export.mesure.0]);
    // Le résultat et les mesures sources font des profils.
    assert_eq!(b.raison_profil(resultat), None);
    assert_eq!(b.raison_profil(feuille), None);
    b.calculer(resultat).unwrap();
}

#[test]
fn des_ecarts_signales_se_voient_avant_la_fin() {
    let mut b = banc();
    let seance = b.commencer();
    b.mesurer(seance).unwrap();
    b.importer(
        seance,
        &export_fd_s2w(FD9, &lignes(10.0 * seance_mesure::SEUIL_ECART_DE00 as f32)),
        Role::Exemplaire,
    );
    let apercu = b.ouverte.etat_seance(seance).unwrap().apercu.unwrap();
    assert!(apercu.ecarts_signales > 0);
    assert_eq!(
        cle(b.terminer(seance, SANS_CHOIX)),
        "seance.fin.ecarts_a_voir"
    );
    b.terminer(
        seance,
        FinDemandee {
            incomplet_assume: false,
            ecarts_vus: true,
        },
    )
    .unwrap();
}

#[test]
fn un_complement_manuel_complete_la_seance() {
    let b = banc();
    let seance = b.commencer();
    let mut l = lignes(0.0);
    let derniere = l.pop().unwrap();
    b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    let complement = b.importer(
        seance,
        &export_fd_s2w(
            FD9,
            &[LigneExport {
                id: "A1".into(),
                ..derniere
            }],
        ),
        Role::Complement,
    );
    b.ouverte
        .completer_seance(
            seance,
            vec![Complement {
                plage: PLAGES.to_string(),
                source: complement.mesure.0,
                identifiant: "A1".into(),
            }],
        )
        .unwrap();
    let apercu = b.ouverte.etat_seance(seance).unwrap().apercu.unwrap();
    assert!(apercu.manquantes.is_empty());
    b.terminer(seance, SANS_CHOIX).unwrap();
    let composition = b.ouverte.etat_seance(seance).unwrap().composition.unwrap();
    let derniere = composition.composition.last().unwrap();
    assert!(derniere.apports[0].manuel);
}

#[test]
fn une_seance_terminee_ne_change_plus() {
    let mut b = banc();
    let seance = b.commencer();
    b.mesurer(seance).unwrap();
    b.terminer(seance, SANS_CHOIX).unwrap();
    assert_eq!(cle(b.terminer(seance, SANS_CHOIX)), "seance.erreur.fermee");
    assert_eq!(b.mesurer(seance).unwrap_err().cle, "seance.erreur.fermee");
}

#[test]
fn un_tirage_mesure_se_prepare_de_nouveau_en_un_nouveau_tirage() {
    let mut b = banc();
    let seance = b.commencer();
    b.mesurer(seance).unwrap();
    // Séance en cours : préparer la même mire reprend le même tirage.
    let reprise = b
        .ouverte
        .preparer_mire_fd9(
            b.condition,
            mire(),
            &b.dossier.path().join("mires"),
            maintenant(),
        )
        .unwrap();
    assert_eq!(IdTirage(reprise.tirage), b.tirage);
    b.terminer(seance, SANS_CHOIX).unwrap();
    // Séance terminée : le tirage est mesuré, un nouveau tirage est créé.
    let nouveau = b
        .ouverte
        .preparer_mire_fd9(
            b.condition,
            mire(),
            &b.dossier.path().join("mires"),
            maintenant(),
        )
        .unwrap();
    assert_ne!(IdTirage(nouveau.tirage), b.tirage);
    assert_eq!(nouveau.deja_preparee, None);
    // Et une nouvelle séance sur l'ancien tirage est refusée.
    let refus = b
        .ouverte
        .commencer_seance_fd9(b.condition, b.tirage, &fiche(true), maintenant())
        .unwrap_err();
    assert_eq!(refus.cle, "seance.erreur.tirage_indisponible");
}

#[test]
fn une_mesure_de_seance_en_cours_ne_sert_pas_de_seconde_mire() {
    let mut b = banc();
    let (profil, tolerances) = b.profil_et_tolerances();
    let seance = b.nouvelle_seance();
    let en_cours = b
        .importer(seance, &export_fd_s2w(FD9, &lignes(0.0)), Role::Exemplaire)
        .mesure;
    assert_eq!(
        b.verifier(profil, tolerances, en_cours),
        Err("profil.refus.seance_en_cours")
    );
    // Une mesure hors séance reste vérifiable, comme avant.
    let hors = b.importer_hors_seance(&export_fd_s2w(FD9, &lignes(0.0)));
    assert_eq!(b.verifier(profil, tolerances, hors), Ok(()));
}

#[test]
fn une_mesure_incomplete_ou_incertaine_ne_sert_pas_de_seconde_mire() {
    let mut b = banc();
    let (profil, tolerances) = b.profil_et_tolerances();
    let seance = b.nouvelle_seance();
    let mut l = lignes(0.0);
    l.truncate(PLAGES - 1);
    let incomplete = b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    let mut l = lignes(0.0);
    l[0].rvb = [1.0, 2.0, 3.0];
    let incertaine = b.importer(seance, &export_fd_s2w(FD9, &l), Role::Exemplaire);
    assert_eq!(
        b.verifier(profil, tolerances, incomplete.mesure),
        Err("profil.refus.mesure_incomplete")
    );
    assert_eq!(
        b.verifier(profil, tolerances, incertaine.mesure),
        Err("profil.refus.association_incertaine")
    );
    // Le résultat d'une séance fermée incomplète non plus.
    b.ouverte
        .retenir_source(
            seance,
            incertaine.mesure,
            false,
            "autre feuille",
            maintenant(),
        )
        .unwrap();
    let resultat = b
        .terminer(
            seance,
            FinDemandee {
                incomplet_assume: true,
                ecarts_vus: false,
            },
        )
        .unwrap();
    assert_eq!(
        b.verifier(profil, tolerances, resultat),
        Err("profil.refus.seance_incomplete")
    );
}

#[test]
fn une_source_ecartee_ne_sert_plus_meme_seance_fermee() {
    let mut b = banc();
    let (profil, tolerances) = b.profil_et_tolerances();
    let seance = b.nouvelle_seance();
    b.mesurer(seance).unwrap();
    let ecartee = b.mesurer(seance).unwrap().mesure;
    b.ouverte
        .retenir_source(seance, ecartee, false, "feuille froissée", maintenant())
        .unwrap();
    b.terminer(seance, SANS_CHOIX).unwrap();
    assert_eq!(
        b.raison_profil(ecartee),
        Some("profil.refus.mesure_ecartee")
    );
    assert_eq!(b.calculer(ecartee), Err("profil.refus.mesure_ecartee"));
    assert_eq!(
        b.verifier(profil, tolerances, ecartee),
        Err("profil.refus.mesure_ecartee")
    );
}

#[test]
fn ecarter_une_mesure_demande_un_motif() {
    let mut b = banc();
    let seance = b.commencer();
    let source = b.mesurer(seance).unwrap().mesure;
    let refus = b
        .ouverte
        .retenir_source(seance, source, false, "  ", maintenant())
        .unwrap_err();
    assert_eq!(refus.cle, "seance.erreur.motif_vide");
    assert!(b.ouverte.etat_seance(seance).unwrap().sources[0].retenue);
    // La retenir de nouveau n'en demande pas.
    b.ouverte
        .retenir_source(seance, source, true, "", maintenant())
        .unwrap();
}

#[test]
fn un_complement_ne_s_ecarte_pas_comme_un_exemplaire() {
    let b = banc();
    let seance = b.commencer();
    let complement = b.importer(
        seance,
        &export_fd_s2w(FD9, &lignes(0.0)[..1]),
        Role::Complement,
    );
    let refus = b
        .ouverte
        .retenir_source(seance, complement.mesure, false, "inutile", maintenant())
        .unwrap_err();
    assert_eq!(refus.cle, "seance.erreur.complement_non_ecartable");
}
