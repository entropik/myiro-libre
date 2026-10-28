//! Comparaison de profils ICC vue par l'application : méthodes de
//! `BibliothequeOuverte` (sans Tauri), fichiers ouverts le temps de la
//! séance, mesures embarquées et limite du gamut par `MoteurGamutSimule`,
//! jamais ArgyllCMS. Profils d'essai synthétiques de la crate `gamut`.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use app::colonne::{emplacement, BibliothequeOuverte};
use app::comparer::embarquees::{mesures_embarquees, CLES_FIDELITE};
use app::comparer::{
    cle_limite, EtatFidelite, EtatLimite, EtatProfil, FichiersOuverts, SourceComparaison,
    CLES_COMPARAISON,
};
use app::textes::{texte, Langue};
use bibliotheque::{IdCondition, IdProfil};
use gamut::{ErreurIcc, Famille, ProfilEssai, Statistiques, Zone};
use pont_protocole::ConditionMesure;
use profilage::{ErreurMoteur, Etape, LimiteCalculee, MoteurGamut, MoteurGamutSimule};
use tempfile::TempDir;

/// Profil d'essai linéaire : L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Le même, plus sombre d'une unité de L\*.
fn plus_sombre(rvb: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lineaire(rvb);
    [l - 1.0, a, b]
}

fn essai(cible: fn([f64; 3]) -> [f64; 3], grille: u8, mesures: Option<String>) -> Vec<u8> {
    gamut::profil_essai(&ProfilEssai {
        grille,
        blanc: [100.0, 0.0, 0.0],
        cible,
        description: "Essai".into(),
        mesures,
    })
}

/// Mesures embarquées à la manière de MYIROtools : condition M1 écrite en
/// clair, RVB de 0 à 255, Lab du profil d'essai linéaire.
fn targ_myirotools(rvb: &[[u8; 3]]) -> String {
    let lignes: String = rvb
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let lab = lineaire(c.map(|v| f64::from(v) / 255.0));
            format!(
                "{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\r\n",
                i + 1,
                c[0],
                c[1],
                c[2],
                lab[0],
                lab[1],
                lab[2]
            )
        })
        .collect();
    format!(
        "CGATS.17\r\nORIGINATOR\t\"MYIROtools\"\r\nMEASUREMENT_SOURCE\t\"M1\"\r\n\
         BEGIN_DATA_FORMAT\r\nSAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\r\n\
         END_DATA_FORMAT\r\nNUMBER_OF_SETS\t{}\r\nBEGIN_DATA\r\n{lignes}END_DATA\r\n",
        rvb.len()
    )
}

const PLAGES: [[u8; 3]; 6] = [
    [255, 255, 255],
    [0, 0, 0],
    [255, 0, 0],
    [0, 0, 255],
    [0, 128, 255],
    [0, 255, 255],
];

/// Limite d'un tétraèdre, au format `.gam` d'iccgamut.
const GAM: &str = "GAMUT\n\
BEGIN_DATA_FORMAT\nVERTEX_NO LAB_L LAB_A LAB_B\nEND_DATA_FORMAT\n\
BEGIN_DATA\n0 0 0 0\n1 0 10 0\n2 0 0 10\n3 10 0 0\nEND_DATA\n\
BEGIN_DATA_FORMAT\nVERTEX_0 VERTEX_1 VERTEX_2\nEND_DATA_FORMAT\n\
BEGIN_DATA\n0 2 1\n0 1 3\n0 3 2\n1 2 3\nEND_DATA\n";

struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    fichiers: FichiersOuverts,
    canon: IdCondition,
}

/// Canon PRO-4100 → Photo Rag 308 → 720 dpi.
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let canon = ouverte
        .ajouter_papier(canon.id, "Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        dossier,
        ouverte,
        fichiers: FichiersOuverts::default(),
        canon,
    }
}

impl Banc {
    fn fichier(&self, nom: &str, octets: &[u8]) -> PathBuf {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, octets).unwrap();
        chemin
    }

    fn ranger(&self, condition: IdCondition, date: &str, icc: &[u8]) -> IdProfil {
        let chemin = self.fichier(&format!("{date}.icc"), icc);
        self.ouverte
            .ranger_profil(condition, Path::new(&chemin), date)
            .unwrap()
    }

    fn ouvrir(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let resume = self.fichiers.ouvrir(&self.fichier(nom, icc)).unwrap();
        assert_eq!(resume.nom, nom);
        SourceComparaison::Fichier {
            jeton: resume.jeton,
        }
    }
}

fn bibliotheque(profil: IdProfil) -> SourceComparaison {
    SourceComparaison::Bibliotheque { profil: profil.0 }
}

fn est_au_catalogue(cle: &str) -> bool {
    app::textes::cles().any(|c| c == cle)
}

#[test]
fn les_profils_comparables_listent_actifs_et_historique() {
    let b = banc();
    let ancien = b.ranger(b.canon, "2026-10-01", &essai(lineaire, 9, None));
    let recent = b.ranger(b.canon, "2026-10-20", &essai(plus_sombre, 9, None));
    let epson = b.ouverte.ajouter_imprimante("Epson SC-P5300").unwrap();
    let luster = b
        .ouverte
        .ajouter_papier(epson.id, "Luster 260", "1440 dpi")
        .unwrap()
        .id;
    let autre = b.ranger(luster, "2026-10-15", &essai(lineaire, 9, None));

    let liste = b.ouverte.profils_comparables().unwrap();
    assert_eq!(liste.len(), 3);
    let trouver = |id: IdProfil| liste.iter().find(|p| p.profil == id).unwrap();
    let (a, r, e) = (trouver(ancien), trouver(recent), trouver(autre));
    assert_eq!(
        (
            a.imprimante.as_str(),
            a.papier.as_str(),
            a.resolution.as_str()
        ),
        ("Canon PRO-4100", "Photo Rag 308", "720 dpi")
    );
    assert!(!a.actif);
    assert!(r.actif);
    assert!(r.horodatage.starts_with("2026-10-20"));
    assert_eq!(
        (
            e.imprimante.as_str(),
            e.papier.as_str(),
            e.resolution.as_str()
        ),
        ("Epson SC-P5300", "Luster 260", "1440 dpi")
    );
    assert!(e.actif);
}

#[test]
fn un_profil_de_la_bibliotheque_et_un_fichier_se_comparent() {
    let b = banc();
    let range = b.ranger(b.canon, "2026-10-20", &essai(lineaire, 9, None));
    let fichier = b.ouvrir("sombre.icc", &essai(plus_sombre, 9, None));

    let c = b
        .ouverte
        .comparer(&[bibliotheque(range), fichier], &b.fichiers, None)
        .unwrap();
    assert_eq!(c.profils.len(), 2);
    assert!(c.ecarts[0].is_none());
    let ecarts = c.ecarts[1]
        .as_ref()
        .expect("écarts du profil 1 au profil 0");
    assert!(ecarts.grille.moyenne > 0.5, "{:?}", ecarts.grille);
    assert!(!ecarts.identiques);
    assert_eq!(c.memes_mesures, vec![false, false]);
    assert!(c.profils[0]
        .origine
        .contains("Canon PRO-4100 › Photo Rag 308 › 720 dpi"));
    assert_eq!(c.profils[0].date.as_deref(), Some("20/10/2026"));
    assert_eq!(c.profils[1].nom, "sombre.icc");
    assert_eq!(c.profils[1].date, None);
}

#[test]
fn un_fichier_ouvert_n_est_jamais_range() {
    let b = banc();
    b.ranger(b.canon, "2026-10-20", &essai(lineaire, 9, None));
    let avant = b.ouverte.profils_comparables().unwrap();
    let fichier = b.ouvrir("autre.icc", &essai(lineaire, 9, None));
    b.ouverte.comparer(&[fichier], &b.fichiers, None).unwrap();
    assert_eq!(b.ouverte.profils_comparables().unwrap(), avant);
}

#[test]
fn un_jeton_inconnu_est_refuse() {
    let b = banc();
    let erreur = b
        .ouverte
        .comparer(
            &[SourceComparaison::Fichier { jeton: 42 }],
            &b.fichiers,
            None,
        )
        .unwrap_err();
    assert_eq!(erreur, "comparer.refus.fichier_ferme");
    assert!(est_au_catalogue(&erreur));
}

#[test]
fn un_profil_refuse_n_empeche_pas_les_autres() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let mut mab = essai(lineaire, 9, None);
    // A2B0 et A2B1 changent de structure : `mAB ` (lutAtoB).
    for i in 0..mab.len() - 4 {
        if &mab[i..i + 4] == b"mft2" {
            mab[i..i + 4].copy_from_slice(b"mAB ");
        }
    }
    let v4 = b.ouvrir("v4.icc", &mab);
    let c = b.ouvrir("c.icc", &essai(plus_sombre, 9, None));

    let comparaison = b.ouverte.comparer(&[a, v4, c], &b.fichiers, None).unwrap();
    assert!(matches!(
        comparaison.profils[0].etat,
        EtatProfil::Analyse { .. }
    ));
    match &comparaison.profils[1].etat {
        EtatProfil::Refuse { cle, detail } => {
            assert_eq!(*cle, "comparer.refus.structure");
            assert_eq!(detail.as_deref(), Some("mAB"));
        }
        autre => panic!("{autre:?}"),
    }
    assert!(matches!(
        comparaison.profils[2].etat,
        EtatProfil::Analyse { .. }
    ));
    assert!(comparaison.ecarts[1].is_none());
    assert!(comparaison.ecarts[2].is_some());
}

#[test]
fn sans_mesures_embarquees_la_fidelite_est_inconnue() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let c = b.ouverte.comparer(&[a], &b.fichiers, None).unwrap();
    match &c.profils[0].etat {
        EtatProfil::Analyse {
            fidelite,
            repetabilite,
            ..
        } => {
            assert_eq!(
                *fidelite,
                EtatFidelite::Inconnue {
                    cle: "comparer.fidelite.sans_mesures"
                }
            );
            assert_eq!(*repetabilite, None);
        }
        autre => panic!("{autre:?}"),
    }
}

#[test]
fn des_mesures_lab_donnent_la_fidelite_et_la_repetabilite() {
    let b = banc();
    let mut plages = PLAGES.to_vec();
    plages.push([255, 255, 255]);
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, Some(targ_myirotools(&plages))));
    let c = b.ouverte.comparer(&[a], &b.fichiers, None).unwrap();
    let EtatProfil::Analyse {
        fidelite,
        repetabilite,
        ..
    } = &c.profils[0].etat
    else {
        panic!("{:?}", c.profils[0].etat);
    };
    match fidelite {
        EtatFidelite::Connue {
            statistiques,
            plages,
            condition,
            condition_supposee,
            rvb_supposes,
            lab_calcule,
        } => {
            assert_eq!(*plages, 7);
            assert!(statistiques.max < 0.05, "{statistiques:?}");
            assert_eq!(*condition, ConditionMesure::M1);
            assert!(!condition_supposee);
            assert!(rvb_supposes);
            assert!(!lab_calcule);
        }
        autre => panic!("{autre:?}"),
    }
    let r = repetabilite.as_ref().expect("blanc mesuré deux fois");
    assert_eq!((r.couleurs, r.paires), (1, 1));
}

/// Spectre plat (réflectance constante) sur 380–730 nm par 10 nm.
fn ligne_spectrale(id: usize, rvb: [u8; 3], reflexion: f64) -> String {
    let spectre: Vec<String> = (0..36).map(|_| format!("{reflexion}")).collect();
    format!(
        "{id}\t{}\t{}\t{}\t{}\r\n",
        rvb[0],
        rvb[1],
        rvb[2],
        spectre.join("\t")
    )
}

#[test]
fn des_mesures_spectrales_seules_donnent_un_lab_calcule() {
    let champs: Vec<String> = (0..36).map(|i| format!("nm{}", 380 + 10 * i)).collect();
    let targ = format!(
        "CGATS.17\r\nORIGINATOR\t\"MYIROtools\"\r\nMEASUREMENT_SOURCE\t\"M1\"\r\n\
         BEGIN_DATA_FORMAT\r\nSAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\t{}\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t2\r\nBEGIN_DATA\r\n{}{}END_DATA\r\n",
        champs.join("\t"),
        ligne_spectrale(1, [255, 255, 255], 0.9),
        ligne_spectrale(2, [0, 0, 0], 0.02),
    );
    let mesures = mesures_embarquees(targ.as_bytes()).unwrap();
    assert!(mesures.lab_calcule);
    assert!(mesures.rvb_supposes);
    assert_eq!(mesures.condition, ConditionMesure::M1);
    assert!(!mesures.condition_supposee);
    assert_eq!(mesures.mesures.len(), 2);
    // Réflectance de 0,9 : L* = 116·0,9^(1/3) − 16, gris neutre.
    let blanc = mesures.mesures[0].lab;
    assert!(
        (blanc[0] - (116.0 * 0.9f64.cbrt() - 16.0)).abs() < 0.05,
        "{blanc:?}"
    );
    assert!(blanc[1].abs() < 0.05 && blanc[2].abs() < 0.05, "{blanc:?}");
    assert_eq!(mesures.mesures[0].rvb, [1.0, 1.0, 1.0]);

    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, Some(targ)));
    let c = b.ouverte.comparer(&[a], &b.fichiers, None).unwrap();
    let EtatProfil::Analyse { fidelite, .. } = &c.profils[0].etat else {
        panic!("{:?}", c.profils[0].etat);
    };
    assert!(
        matches!(
            fidelite,
            EtatFidelite::Connue {
                lab_calcule: true,
                rvb_supposes: true,
                ..
            }
        ),
        "{fidelite:?}"
    );
}

#[test]
fn des_mesures_sans_rvb_ni_lab_sont_dites_inconnues() {
    // Sans ORIGINATOR connu, l'échelle RVB n'est pas établie.
    let sans_echelle = targ_myirotools(&PLAGES).replace("MYIROtools", "Autre logiciel");
    assert_eq!(
        mesures_embarquees(sans_echelle.as_bytes()).unwrap_err(),
        "comparer.fidelite.rvb_inconnus"
    );
    let sans_lab = "CGATS.17\r\nORIGINATOR\t\"MYIROtools\"\r\nMEASUREMENT_SOURCE\t\"M1\"\r\n\
         BEGIN_DATA_FORMAT\r\nSAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\r\nEND_DATA_FORMAT\r\n\
         NUMBER_OF_SETS\t1\r\nBEGIN_DATA\r\n1\t0\t0\t0\r\nEND_DATA\r\n";
    assert_eq!(
        mesures_embarquees(sans_lab.as_bytes()).unwrap_err(),
        "comparer.fidelite.lab_inconnu"
    );
    assert_eq!(
        mesures_embarquees(b"pas du CGATS").unwrap_err(),
        "comparer.fidelite.illisible"
    );
}

#[test]
fn les_memes_mesures_sont_reconnues() {
    let b = banc();
    let targ = targ_myirotools(&PLAGES);
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, Some(targ.clone())));
    let c = b.ouvrir("c.icc", &essai(lineaire, 17, Some(targ)));
    let d = b.ouvrir("d.icc", &essai(lineaire, 9, None));
    let comparaison = b.ouverte.comparer(&[a, c, d], &b.fichiers, None).unwrap();
    assert_eq!(comparaison.memes_mesures, vec![false, true, false]);
}

#[test]
fn sans_argyll_la_limite_est_indisponible_et_le_reste_est_calcule() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let c = b.ouverte.comparer(&[a], &b.fichiers, None).unwrap();
    assert_eq!(c.commande_limite, None);
    let EtatProfil::Analyse {
        analyse, limite, ..
    } = &c.profils[0].etat
    else {
        panic!("{:?}", c.profils[0].etat);
    };
    assert_eq!(
        *limite,
        EtatLimite::Indisponible {
            cle: "comparer.limite.sans_argyll",
            cause: None
        }
    );
    assert_eq!(analyse.primaires.len(), 8);
    assert!((analyse.volume_surface - 270_000.0).abs() < 270.0);
}

#[test]
fn la_limite_calculee_est_rendue_avec_son_journal() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let moteur = MoteurGamutSimule::reussit(GAM.to_string());
    let c = b
        .ouverte
        .comparer(&[a], &b.fichiers, Some(&moteur))
        .unwrap();
    assert_eq!(
        c.commande_limite.as_deref(),
        Some("iccgamut -v -ia -pl profil.icc")
    );
    let EtatProfil::Analyse { limite, .. } = &c.profils[0].etat else {
        panic!("{:?}", c.profils[0].etat);
    };
    match limite {
        EtatLimite::Calculee {
            maillage,
            volume,
            journal,
        } => {
            assert_eq!(maillage.sommets.len(), 4);
            assert_eq!(maillage.triangles.len(), 4);
            assert_eq!(maillage.couleurs.len(), 4);
            assert_eq!(*volume, None);
            assert!(journal.contains("iccgamut"), "{journal}");
        }
        autre => panic!("{autre:?}"),
    }
}

#[test]
fn un_echec_d_iccgamut_est_dit_avec_sa_cause() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let moteur = MoteurGamutSimule::echoue(ErreurMoteur::Echec {
        etape: Etape::Gamut,
        cause: "iccgamut: Error - profil illisible".into(),
    });
    let c = b
        .ouverte
        .comparer(&[a], &b.fichiers, Some(&moteur))
        .unwrap();
    let EtatProfil::Analyse { limite, .. } = &c.profils[0].etat else {
        panic!("{:?}", c.profils[0].etat);
    };
    assert_eq!(
        *limite,
        EtatLimite::Indisponible {
            cle: "comparer.erreur.limite",
            cause: Some("iccgamut: Error - profil illisible".into())
        }
    );
}

#[test]
fn sans_iccgamut_la_limite_ne_dit_pas_argyll_introuvable() {
    let erreur = ErreurMoteur::OutilManquant {
        outil: "iccgamut".into(),
    };
    assert_eq!(cle_limite(&erreur), "comparer.limite.sans_iccgamut");
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let moteur = MoteurGamutSimule::echoue(erreur);
    let c = b
        .ouverte
        .comparer(&[a], &b.fichiers, Some(&moteur))
        .unwrap();
    let EtatProfil::Analyse { limite, .. } = &c.profils[0].etat else {
        panic!("{:?}", c.profils[0].etat);
    };
    assert!(
        matches!(
            limite,
            EtatLimite::Indisponible {
                cle: "comparer.limite.sans_iccgamut",
                ..
            }
        ),
        "{limite:?}"
    );
    let fr = texte(Langue::Francais, "comparer.limite.sans_iccgamut");
    assert!(
        fr.contains("iccgamut") && !fr.contains("introuvable"),
        "{fr}"
    );
    let en = texte(Langue::Anglais, "comparer.limite.sans_iccgamut");
    assert!(
        en.contains("iccgamut") && !en.contains("cannot be found"),
        "{en}"
    );
}

#[test]
fn une_limite_illisible_est_dite() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let moteur = MoteurGamutSimule::reussit("pas un .gam".into());
    let c = b
        .ouverte
        .comparer(&[a], &b.fichiers, Some(&moteur))
        .unwrap();
    let EtatProfil::Analyse { limite, .. } = &c.profils[0].etat else {
        panic!("{:?}", c.profils[0].etat);
    };
    assert!(
        matches!(
            limite,
            EtatLimite::Indisponible {
                cle: "comparer.limite.illisible",
                cause: Some(_)
            }
        ),
        "{limite:?}"
    );
}

/// Moteur qui regarde, pendant son calcul, si la bibliothèque reste
/// utilisable par un autre fil (la page) : rien ne doit être verrouillé.
struct MoteurQuiRegarde {
    ouverte: Arc<BibliothequeOuverte>,
    fichiers: Arc<FichiersOuverts>,
    libre: std::sync::Mutex<Vec<bool>>,
}

impl MoteurGamut for MoteurQuiRegarde {
    fn commande_limite(&self) -> String {
        String::new()
    }

    fn limite(&self, _icc: &[u8]) -> Result<LimiteCalculee, ErreurMoteur> {
        let (envoi, reponse) = mpsc::channel();
        let (ouverte, fichiers) = (Arc::clone(&self.ouverte), Arc::clone(&self.fichiers));
        // Fil détaché : s'il reste bloqué, il se libère après le calcul.
        std::thread::spawn(move || {
            let _ = ouverte.parc("");
            let _ = fichiers.ouvrir(Path::new("absent.icc"));
            let _ = envoi.send(());
        });
        let libre = reponse.recv_timeout(Duration::from_secs(5)).is_ok();
        self.libre.lock().unwrap().push(libre);
        Ok(LimiteCalculee {
            gam: GAM.to_string(),
            volume: None,
            version: "3.5.0".into(),
            journal: String::new(),
        })
    }
}

#[test]
fn la_limite_se_calcule_hors_de_tout_verrou() {
    let b = banc();
    let profil = b.ranger(b.canon, "2026-10-20", &essai(lineaire, 9, None));
    let source = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let Banc {
        dossier: _dossier,
        ouverte,
        fichiers,
        ..
    } = b;
    let moteur = MoteurQuiRegarde {
        ouverte: Arc::new(ouverte),
        fichiers: Arc::new(fichiers),
        libre: Default::default(),
    };
    moteur
        .ouverte
        .comparer(
            &[bibliotheque(profil), source],
            &moteur.fichiers,
            Some(&moteur),
        )
        .unwrap();
    assert_eq!(*moteur.libre.lock().unwrap(), vec![true, true]);
}

#[test]
fn plus_de_quatre_profils_est_refuse() {
    let b = banc();
    let sources: Vec<_> = (0..5)
        .map(|i| b.ouvrir(&format!("{i}.icc"), &essai(lineaire, 9, None)))
        .collect();
    let erreur = b.ouverte.comparer(&sources, &b.fichiers, None).unwrap_err();
    assert_eq!(erreur, "comparer.refus.trop_de_profils");
    assert!(est_au_catalogue(&erreur));
    let erreur = b
        .ouverte
        .chemin_comparaison(&sources, &b.fichiers, [0, 0, 255], [0, 255, 255])
        .unwrap_err();
    assert_eq!(erreur, "comparer.refus.trop_de_profils");
}

#[test]
fn le_chemin_rend_les_points_et_les_mesures_du_chemin() {
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, Some(targ_myirotools(&PLAGES))));
    let sans = b.ouvrir("b.icc", &essai(lineaire, 9, None));
    let chemins = b
        .ouverte
        .chemin_comparaison(&[a, sans], &b.fichiers, [0, 0, 255], [0, 255, 255])
        .unwrap();
    assert_eq!(chemins.len(), 2);
    assert_eq!(chemins[0].points.len(), 256);
    // Bleu, bleu-cyan et cyan sont sur le chemin ; blanc, noir et rouge non.
    assert_eq!(chemins[0].mesures.len(), 3);
    assert!(chemins[0].mesures[0][0].abs() < 1e-9);
    assert!((chemins[0].mesures[2][0] - 255.0).abs() < 1e-9);
    assert_eq!(chemins[1].points.len(), 256);
    assert!(chemins[1].mesures.is_empty());
}

/// Les chiffres incalculables partent en `null` vers la page, sans planter :
/// la page les affiche « inconnu ».
#[test]
fn un_chiffre_incalculable_part_en_null() {
    let fidelite = EtatFidelite::Connue {
        statistiques: Statistiques {
            nombre: 1,
            moyenne: f64::NAN,
            mediane: f64::NAN,
            p95: f64::INFINITY,
            max: f64::NAN,
            part_sup_1: 0.0,
            part_sup_2: 0.0,
        },
        plages: 1,
        condition: ConditionMesure::M1,
        condition_supposee: false,
        rvb_supposes: true,
        lab_calcule: false,
    };
    let json = serde_json::to_value(&fidelite).unwrap();
    assert_eq!(json["statistiques"]["moyenne"], serde_json::Value::Null);
    assert_eq!(json["statistiques"]["p95"], serde_json::Value::Null);
    assert_eq!(json["statistiques"]["nombre"], 1);

    // Et toute une comparaison passe en JSON.
    let b = banc();
    let a = b.ouvrir("a.icc", &essai(lineaire, 9, None));
    let c = b.ouverte.comparer(&[a], &b.fichiers, None).unwrap();
    let json = serde_json::to_value(&c).unwrap();
    assert_eq!(json["profils"][0]["source"]["type"], "fichier");
    assert_eq!(json["profils"][0]["etat"]["etat"], "analyse");
}

#[test]
fn les_sources_se_lisent_depuis_la_page() {
    let source: SourceComparaison =
        serde_json::from_str(r#"{"type":"bibliotheque","profil":3}"#).unwrap();
    assert_eq!(source, SourceComparaison::Bibliotheque { profil: 3 });
    let source: SourceComparaison =
        serde_json::from_str(r#"{"type":"fichier","jeton":1}"#).unwrap();
    assert_eq!(source, SourceComparaison::Fichier { jeton: 1 });
}

/// Toutes les erreurs que la limite du gamut peut rendre.
fn erreurs_moteur() -> Vec<ErreurMoteur> {
    vec![
        ErreurMoteur::Introuvable,
        ErreurMoteur::VersionNonValidee {
            trouvee: "3.4.0".into(),
        },
        ErreurMoteur::OutilManquant {
            outil: "iccgamut".into(),
        },
        ErreurMoteur::Echec {
            etape: Etape::Version,
            cause: String::new(),
        },
        ErreurMoteur::Echec {
            etape: Etape::Profil,
            cause: String::new(),
        },
        ErreurMoteur::Echec {
            etape: Etape::Controle,
            cause: String::new(),
        },
        ErreurMoteur::Echec {
            etape: Etape::Gamut,
            cause: String::new(),
        },
        ErreurMoteur::Fichier(String::new()),
    ]
}

#[test]
fn les_cles_de_la_comparaison_sont_au_catalogue() {
    let icc = [
        ErreurIcc::EnTete(String::new()),
        ErreurIcc::PasRvb {
            espace: "CMYK".into(),
        },
        ErreurIcc::SansTable,
        ErreurIcc::StructureNonPriseEnCharge {
            structure: "mAB".into(),
        },
        ErreurIcc::Tronque {
            balise: "A2B1".into(),
        },
        ErreurIcc::SansBlanc,
    ]
    .map(|e| e.cle().to_string());
    let familles = [
        Famille::Rouge,
        Famille::Jaune,
        Famille::Vert,
        Famille::Cyan,
        Famille::Bleu,
        Famille::Magenta,
    ];
    let zones = [Zone::Gris, Zone::Ombres, Zone::Clairs]
        .into_iter()
        .chain(familles.map(Zone::Saturee))
        .chain(familles.map(Zone::Moyenne))
        .map(|z| z.cle());
    let moteur = erreurs_moteur()
        .iter()
        .flat_map(|e| [e.cle().to_string(), cle_limite(e).to_string()])
        .collect::<Vec<_>>();
    let cles: Vec<String> = icc
        .into_iter()
        .chain(zones)
        .chain(moteur)
        .chain(CLES_FIDELITE.iter().map(|c| c.to_string()))
        .chain(CLES_COMPARAISON.iter().map(|c| c.to_string()))
        .collect();
    assert!(cles.contains(&"comparer.erreur.limite".to_string()));
    let absentes: Vec<_> = cles.iter().filter(|c| !est_au_catalogue(c)).collect();
    assert!(absentes.is_empty(), "clés absentes : {absentes:?}");
}

/// La comparaison ne range rien et ne calcule pas de profil : ses textes
/// d'erreur ne le laissent pas croire.
#[test]
fn les_erreurs_de_la_comparaison_ne_parlent_pas_d_un_profil_range() {
    let cles: Vec<&str> = erreurs_moteur()
        .iter()
        .map(cle_limite)
        .chain(CLES_COMPARAISON.iter().copied())
        .chain(CLES_FIDELITE.iter().copied())
        .collect();
    for cle in cles {
        assert!(cle.starts_with("comparer."), "{cle}");
        let fr = texte(Langue::Francais, cle);
        let en = texte(Langue::Anglais, cle);
        for interdit in ["rangé", "profil actuel", "calcul du profil"] {
            assert!(!fr.contains(interdit), "{cle} : {fr}");
        }
        for interdit in ["stored", "current profile", "computing the profile"] {
            assert!(!en.to_lowercase().contains(interdit), "{cle} : {en}");
        }
    }
}
