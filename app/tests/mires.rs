//! Préparer une mire, vu par l'application : méthodes de
//! `BibliothequeOuverte` (sans Tauri), moteur simulé (jamais ArgyllCMS) et
//! mire FD-9 fictive de même forme que RGB-2520. Les erreurs sont des clés
//! du catalogue de textes.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::mires::{chercher_mire_fd9, nom_fichier_page, DemandeMire};
use app::textes::{texte, Langue};
use bibliotheque::IdCondition;
use mires::{
    ecrire_tiff, lire_mire_fd9_sans_empreinte, ErreurMire, Format, GeometrieFd9, ImageRvb,
    Instrument, MireFd9, TailleMire, RGB2520_FICHIER,
};
use profilage::{ErreurMoteur, MoteurSimule};
use tempfile::TempDir;

/// Le 23 octobre 2026 à 9 h 15, UTC.
fn maintenant() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_792_746_900)
}

/// `.ti1` fictif de `n` plages : les premières valeurs d'un cube RVB 8 × 8 × 8.
fn ti1(n: usize) -> String {
    let niveau = |i: usize| i as f64 * 100.0 / 7.0;
    let mut lignes = String::new();
    for k in 0..n {
        let (r, g, b) = (k / 64 % 8, k / 8 % 8, k % 8);
        lignes.push_str(&format!(
            "{} {:.4} {:.4} {:.4} 0 0 0\n",
            k + 1,
            niveau(r),
            niveau(g),
            niveau(b)
        ));
    }
    format!(
        "CTI1\n\nCOLOR_REP \"iRGB\"\n\nNUMBER_OF_FIELDS 7\nBEGIN_DATA_FORMAT\n\
         SAMPLE_ID RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z\nEND_DATA_FORMAT\n\n\
         NUMBER_OF_SETS {n}\nBEGIN_DATA\n{lignes}END_DATA\n"
    )
}

/// `.ti1` de plages toutes proches du blanc papier.
fn ti1_clair(n: usize) -> String {
    let lignes: String = (0..n)
        .map(|k| format!("{} {} 100 {} 0 0 0\n", k + 1, 95 + k % 6, 96 + k % 5))
        .collect();
    format!(
        "CTI1\n\nBEGIN_DATA_FORMAT\nSAMPLE_ID RGB_R RGB_G RGB_B XYZ_X XYZ_Y XYZ_Z\n\
         END_DATA_FORMAT\n\nBEGIN_DATA\n{lignes}END_DATA\n"
    )
}

struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    condition: IdCondition,
}

/// Canon PRO-4100 → Hahnemühle Photo Rag 308 → 720 dpi.
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "Hahnemühle Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        dossier,
        ouverte,
        condition,
    }
}

impl Banc {
    fn mires(&self) -> PathBuf {
        self.dossier.path().join("Mires")
    }

    fn fichiers(&self) -> Vec<String> {
        let mut noms: Vec<String> = std::fs::read_dir(self.mires())
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        noms.sort();
        noms
    }

    fn tirages(&self) -> usize {
        self.ouverte
            .detail_reglage(self.condition)
            .unwrap()
            .tirages
            .len()
    }

    fn preparer(
        &self,
        demande: &DemandeMire,
        moteur: &MoteurSimule,
        mire_fd9: Option<&Path>,
    ) -> Result<app::mires::MirePreparee, String> {
        self.ouverte.preparer_mire(
            self.condition,
            demande,
            moteur,
            mire_fd9,
            &self.mires(),
            maintenant(),
        )
    }
}

fn myiro1(taille: TailleMire) -> DemandeMire {
    DemandeMire {
        instrument: Instrument::Myiro1,
        taille: Some(taille),
        nombre: None,
        format: Some(Format::A4),
        cote_mm: None,
    }
}

fn fd9() -> DemandeMire {
    DemandeMire {
        instrument: Instrument::Fd9,
        taille: None,
        nombre: None,
        format: None,
        cote_mm: None,
    }
}

fn moteur(n: usize) -> MoteurSimule {
    MoteurSimule::reussit(Vec::new()).avec_plages(ti1(n))
}

/// Mire FD-9 fictive de même forme que RGB-2520 : 6 × 4 plages de 5 pixels,
/// origine (3, 3), écrite en TIFF.
fn mire_fd9_fictive() -> (Vec<u8>, GeometrieFd9) {
    let g = GeometrieFd9 {
        largeur_px: 36,
        hauteur_px: 26,
        origine_px: (3, 3),
        pas_px: 5,
        colonnes: 6,
        rangees: 4,
    };
    let mut pixels = Vec::new();
    for y in 0..g.hauteur_px {
        for x in 0..g.largeur_px {
            let dans = (3..33).contains(&x) && (3..23).contains(&y);
            let k = ((y.saturating_sub(3)) / 5 * 6 + x.saturating_sub(3) / 5) as u8;
            pixels.extend(if dans {
                [k * 10, 200 - k, k * 7]
            } else {
                [255; 3]
            });
        }
    }
    let mut octets = Vec::new();
    ecrire_tiff(
        &ImageRvb {
            largeur: g.largeur_px,
            hauteur: g.hauteur_px,
            pixels,
        },
        &mut octets,
    )
    .unwrap();
    (octets, g)
}

fn lire_fictive() -> (Vec<u8>, MireFd9) {
    let (octets, g) = mire_fd9_fictive();
    let mire = lire_mire_fd9_sans_empreinte(octets.clone(), &g).unwrap();
    (octets, mire)
}

#[test]
fn une_mire_preparee_ecrit_ses_pages_et_range_son_tirage() {
    let b = banc();
    let m = moteur(232);
    let preparee = b.preparer(&myiro1(TailleMire::Rapide), &m, None).unwrap();
    assert_eq!((preparee.plages, preparee.pages), (232, 1));
    assert_eq!(preparee.commande, "targen -v -d2 -G -f232 mire");
    assert_eq!(preparee.dossier, b.mires());
    assert_eq!(preparee.fichiers.len(), 1);
    assert!(preparee.fichiers[0].is_file());
    assert_eq!(
        b.fichiers(),
        [format!(
            "Canon PRO-4100 - Hahnemühle Photo Rag 308 - 720 dpi - 2026-10-23 - T{} - page 1 sur 1.tif",
            preparee.tirage
        )]
    );
    assert_eq!(m.plages_demandees(), [232]);
    assert_eq!(b.tirages(), 1);
    // Une page TIFF : en-tête petit-boutiste.
    let octets = std::fs::read(&preparee.fichiers[0]).unwrap();
    assert_eq!(&octets[..4], b"II*\0");
}

#[test]
fn une_mire_de_deux_pages_ecrit_deux_fichiers_nommes() {
    let b = banc();
    let preparee = b
        .preparer(&myiro1(TailleMire::Standard), &moteur(464), None)
        .unwrap();
    assert_eq!(preparee.pages, 2);
    let fichiers = b.fichiers();
    assert_eq!(fichiers.len(), 2);
    assert!(fichiers[0].ends_with("page 1 sur 2.tif"), "{fichiers:?}");
    assert!(fichiers[1].ends_with("page 2 sur 2.tif"), "{fichiers:?}");
    assert!(!fichiers.iter().any(|f| f.ends_with(".part")));
}

#[test]
fn la_mire_fd9_est_copiee_telle_quelle() {
    let b = banc();
    let (octets, mire) = lire_fictive();
    let preparee = b
        .ouverte
        .preparer_mire_fd9(b.condition, mire, &b.mires(), maintenant())
        .unwrap();
    assert_eq!((preparee.plages, preparee.pages), (24, 1));
    assert_eq!(preparee.commande, "");
    assert_eq!(preparee.fichiers.len(), 1);
    assert_eq!(std::fs::read(&preparee.fichiers[0]).unwrap(), octets);
    assert_eq!(b.tirages(), 1);
}

#[test]
fn une_mire_fd9_absente_ne_prepare_rien() {
    let b = banc();
    let m = moteur(232);
    assert_eq!(
        b.preparer(&fd9(), &m, None).map(|_| ()),
        Err("mire.erreur.fd9_introuvable".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
    assert!(m.plages_demandees().is_empty());
}

#[test]
fn une_mire_fd9_d_une_autre_empreinte_est_refusee() {
    let b = banc();
    let (octets, _) = mire_fd9_fictive();
    let fichier = b.dossier.path().join(RGB2520_FICHIER);
    std::fs::write(&fichier, &octets).unwrap();
    assert_eq!(
        b.preparer(&fd9(), &moteur(232), Some(&fichier)).map(|_| ()),
        Err("mire.erreur.fd9_autre".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn une_mire_fd9_n_accepte_ni_taille_ni_format() {
    let b = banc();
    let m = moteur(232);
    for demande in [
        DemandeMire {
            taille: Some(TailleMire::Rapide),
            ..fd9()
        },
        DemandeMire {
            format: Some(Format::A3),
            ..fd9()
        },
        DemandeMire {
            nombre: Some(100),
            ..fd9()
        },
        DemandeMire {
            cote_mm: Some(10.0),
            ..fd9()
        },
    ] {
        assert_eq!(
            b.ouverte.apercu_mire(&demande, &m).map(|_| ()),
            Err("mire.erreur.fd9_fixe".to_string())
        );
        assert_eq!(
            b.preparer(&demande, &m, None).map(|_| ()),
            Err("mire.erreur.fd9_fixe".to_string())
        );
    }
    assert_eq!(b.tirages(), 0);
}

#[test]
fn deux_preparations_fd9_donnent_deux_tirages_d_une_meme_mire() {
    let b = banc();
    let premiere = b
        .ouverte
        .preparer_mire_fd9(b.condition, lire_fictive().1, &b.mires(), maintenant())
        .unwrap();
    let seconde = b
        .ouverte
        .preparer_mire_fd9(
            b.condition,
            lire_fictive().1,
            &b.mires(),
            maintenant() + Duration::from_secs(60),
        )
        .unwrap();
    assert_ne!(premiere.tirage, seconde.tirage);
    let tirages = b.ouverte.detail_reglage(b.condition).unwrap().tirages;
    assert_eq!(tirages.len(), 2);
    assert_eq!(tirages[0].mire, tirages[1].mire);
    assert_eq!(tirages[0].instrument, "fd9");
    assert_eq!(b.fichiers().len(), 2);
}

#[test]
fn le_nom_de_fichier_est_nettoye() {
    assert_eq!(
        nom_fichier_page(
            "Canon PRO-4100",
            "Canson « Baryta » / 2",
            "600 dpi",
            "2026-10-23",
            42,
            1,
            2
        ),
        "Canon PRO-4100 - Canson Baryta 2 - 600 dpi - 2026-10-23 - T42 - page 1 sur 2.tif"
    );
    let nom = nom_fichier_page(
        "HP: <Latex>",
        "a\\b|c?d*e\"f",
        "\t1200",
        "2026-10-23",
        1,
        1,
        1,
    );
    assert!(
        !nom.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\t']),
        "{nom}"
    );
}

#[test]
fn l_apercu_dit_plages_pages_et_commande_sans_rien_lancer() {
    let b = banc();
    let m = moteur(928);
    let apercu = b
        .ouverte
        .apercu_mire(&myiro1(TailleMire::Approfondi), &m)
        .unwrap();
    assert_eq!((apercu.plages, apercu.pages), (928, 4));
    assert_eq!(apercu.cote_mm, 8.0);
    assert_eq!(apercu.limites_cote, (7.0, None));
    assert_eq!(apercu.commande, "targen -v -d2 -G -f928 mire");
    let libre = DemandeMire {
        taille: None,
        nombre: Some(500),
        format: Some(Format::A3),
        cote_mm: Some(10.0),
        ..myiro1(TailleMire::Rapide)
    };
    let apercu = b.ouverte.apercu_mire(&libre, &m).unwrap();
    assert_eq!((apercu.plages, apercu.cote_mm), (500, 10.0));
    let apercu = b.ouverte.apercu_mire(&fd9(), &m).unwrap();
    assert_eq!((apercu.plages, apercu.pages), (2520, 1));
    assert_eq!(apercu.commande, "");
    assert!(m.plages_demandees().is_empty());
}

#[test]
fn un_cote_hors_limites_ne_lance_pas_targen() {
    let b = banc();
    let m = moteur(232);
    let petit = DemandeMire {
        cote_mm: Some(6.0),
        ..myiro1(TailleMire::Rapide)
    };
    assert_eq!(
        b.preparer(&petit, &m, None).map(|_| ()),
        Err("mire.erreur.cote".to_string())
    );
    assert_eq!(
        b.ouverte.apercu_mire(&petit, &m).map(|_| ()),
        Err("mire.erreur.cote".to_string())
    );
    for nombre in [23, 3001] {
        let libre = DemandeMire {
            taille: None,
            nombre: Some(nombre),
            ..myiro1(TailleMire::Rapide)
        };
        assert_eq!(
            b.preparer(&libre, &m, None).map(|_| ()),
            Err("mire.erreur.nombre_libre".to_string())
        );
    }
    assert!(m.plages_demandees().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn argyll_introuvable_ne_prepare_rien() {
    let b = banc();
    let m = MoteurSimule::echoue(ErreurMoteur::Introuvable).avec_plages(ti1(232));
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &m, None)
            .map(|_| ()),
        Err("profil.erreur.argyll_introuvable".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn un_ti1_au_nombre_inattendu_ne_range_rien() {
    let b = banc();
    let m = MoteurSimule::reussit(Vec::new()).avec_plages(ti1(200));
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &m, None)
            .map(|_| ()),
        Err("mire.erreur.nombre".to_string())
    );
    assert_eq!(b.tirages(), 0);
}

#[test]
fn un_refus_de_mise_en_page_ne_range_rien() {
    let b = banc();
    let m = MoteurSimule::reussit(Vec::new()).avec_plages(ti1_clair(30));
    let libre = DemandeMire {
        taille: None,
        nombre: Some(30),
        ..myiro1(TailleMire::Rapide)
    };
    assert_eq!(
        b.preparer(&libre, &m, None).map(|_| ()),
        Err("mire.erreur.contraste".to_string())
    );
    assert!(b.fichiers().is_empty());
    assert_eq!(b.tirages(), 0);
}

#[test]
fn une_ecriture_impossible_ne_laisse_aucun_fichier() {
    let b = banc();
    // Le dossier des mires est occupé par un fichier : rien ne peut s'y écrire.
    std::fs::write(b.mires(), b"occupe").unwrap();
    assert_eq!(
        b.preparer(&myiro1(TailleMire::Rapide), &moteur(232), None)
            .map(|_| ()),
        Err("mire.erreur.ecriture".to_string())
    );
}

#[test]
fn la_mire_fd9_se_cherche_a_cote_de_l_application_puis_a_la_racine_du_depot() {
    let racine = tempfile::tempdir().unwrap();
    let exe = racine.path().join("target").join("debug");
    std::fs::create_dir_all(&exe).unwrap();
    assert_eq!(chercher_mire_fd9(&exe), None);
    let depot = racine.path().join("mire-fd9");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::write(depot.join(RGB2520_FICHIER), b"x").unwrap();
    assert_eq!(
        chercher_mire_fd9(&exe).map(|c| c.canonicalize().unwrap()),
        Some(depot.join(RGB2520_FICHIER).canonicalize().unwrap())
    );
    let installe = exe.join("mire-fd9");
    std::fs::create_dir_all(&installe).unwrap();
    std::fs::write(installe.join(RGB2520_FICHIER), b"x").unwrap();
    assert_eq!(
        chercher_mire_fd9(&exe),
        Some(installe.join(RGB2520_FICHIER))
    );
}

#[test]
fn toutes_les_cles_mire_existent_en_deux_langues() {
    let mut cles: Vec<&str> = [
        ErreurMire::CoteHorsLimites {
            min: 7.0,
            max: None,
        },
        ErreurMire::Ti1(String::new()),
        ErreurMire::NombreInattendu {
            attendu: 1,
            trouve: 0,
        },
        ErreurMire::Contraste { defauts: 1 },
        ErreurMire::Ecriture(String::new()),
        ErreurMire::MireFd9Introuvable,
        ErreurMire::MireFd9Autre {
            empreinte: String::new(),
        },
        ErreurMire::MireFd9Illisible(String::new()),
    ]
    .iter()
    .map(ErreurMire::cle)
    .collect();
    cles.extend([
        "mire.erreur.generation",
        "mire.erreur.fd9_fixe",
        "mire.erreur.nombre_libre",
    ]);
    for cle in cles {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} absente en {langue:?}");
        }
    }
}
