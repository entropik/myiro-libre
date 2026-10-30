//! Texte imprimé sur les pages d'une mire MYIRO-1 dans la langue de
//! l'application au moment de la préparation (ticket #79). La langue est
//! rangée avec le tirage : une page redessinée garde son texte d'origine.
//! Une mire rangée avant, sans langue, garde son ancien texte. La mire
//! FD-9 RGB-2520 n'est jamais touchée. Moteur simulé, jamais ArgyllCMS.

mod commun_mires;

use app::colonne::emplacement;
use app::mires::{lignes_etiquette, DemandeMire, TexteEtiquette, CLES_ETIQUETTE};
use app::textes::{texte, Langue};
use bibliotheque::{Bibliotheque, MireRangee, NouvelleMire};
use commun_mires::*;
use mires::{
    caracteres_par_ligne, contenu_qr, couper_ligne, ecrire_tiff, nombre_plages_myiro1, rendre_page,
    CodeTirage, Etiquette, Format, Gabarit, MiseEnPage, TailleMire,
};

/// La mire du seul tirage pas encore mesuré de l'instrument, telle qu'elle
/// est rangée dans la bibliothèque du banc.
fn mire_rangee(b: &Banc, instrument: &str) -> (MireRangee, String) {
    let bibliotheque = Bibliotheque::ouvrir(&emplacement(b.dossier.path())).unwrap();
    let mut tirages = bibliotheque
        .tirages_prepares(b.condition, instrument)
        .unwrap();
    assert_eq!(tirages.len(), 1);
    let t = tirages.remove(0);
    (t.mire, t.horodatage)
}

fn langue_rangee(mire: &MireRangee) -> Option<String> {
    let generation: serde_json::Value = serde_json::from_str(&mire.generation).unwrap();
    generation
        .get("langue")
        .and_then(|l| l.as_str())
        .map(String::from)
}

fn rapide(page: usize, pages: usize) -> TexteEtiquette<'static> {
    TexteEtiquette {
        imprimante: "Canon PRO-4100",
        papier: "Hahnemühle Photo Rag 308",
        resolution: "720 dpi",
        taille: Some(TailleMire::Rapide),
        plages: 232,
        page,
        pages,
        horodatage: "2026-10-23T09:15:00Z",
    }
}

#[test]
fn l_etiquette_francaise_est_en_francais() {
    assert_eq!(
        lignes_etiquette(&rapide(1, 1), Langue::Francais),
        [
            "Canon PRO-4100 · Hahnemühle Photo Rag 308 · 720 dpi",
            "Mire rapide · 232 plages · MYIRO-1 · page 1 sur 1",
            "23 oct. 2026 · Imprimer à 100\u{a0}%, sans gestion des couleurs",
        ]
    );
}

#[test]
fn l_etiquette_anglaise_est_en_anglais() {
    assert_eq!(
        lignes_etiquette(&rapide(2, 3), Langue::Anglais),
        [
            "Canon PRO-4100 · Hahnemühle Photo Rag 308 · 720 dpi",
            "Chart: quick · 232 patches · MYIRO-1 · page 2 of 3",
            "Oct 23, 2026 · Print at 100%, no colour management",
        ]
    );
}

/// Taille libre (section « Avancé ») : le nombre de plages une seule fois.
/// Autres tailles dans les deux langues ; la date suit le mois.
#[test]
fn chaque_taille_et_chaque_mois_ont_leur_texte() {
    let libre = TexteEtiquette {
        taille: None,
        plages: 500,
        horodatage: "2026-02-01T00:00:00Z",
        ..rapide(1, 2)
    };
    assert_eq!(
        lignes_etiquette(&libre, Langue::Francais)[1..],
        [
            "Mire 500 plages · MYIRO-1 · page 1 sur 2",
            "1 févr. 2026 · Imprimer à 100\u{a0}%, sans gestion des couleurs",
        ]
    );
    assert_eq!(
        lignes_etiquette(&libre, Langue::Anglais)[1..],
        [
            "Chart: 500 patches · MYIRO-1 · page 1 of 2",
            "Feb 1, 2026 · Print at 100%, no colour management",
        ]
    );
    let approfondi = TexteEtiquette {
        taille: Some(TailleMire::Approfondi),
        plages: 928,
        horodatage: "2026-08-15T12:00:00Z",
        ..rapide(1, 2)
    };
    assert_eq!(
        lignes_etiquette(&approfondi, Langue::Francais)[1..],
        [
            "Mire approfondi · 928 plages · MYIRO-1 · page 1 sur 2",
            "15 août 2026 · Imprimer à 100\u{a0}%, sans gestion des couleurs",
        ]
    );
    assert_eq!(
        lignes_etiquette(&approfondi, Langue::Anglais)[1],
        "Chart: thorough · 928 patches · MYIRO-1 · page 1 of 2"
    );
    let standard = TexteEtiquette {
        taille: Some(TailleMire::Standard),
        plages: 464,
        horodatage: "2026-12-31T23:59:59Z",
        ..rapide(1, 1)
    };
    assert_eq!(
        lignes_etiquette(&standard, Langue::Anglais)[1..],
        [
            "Chart: standard · 464 patches · MYIRO-1 · page 1 of 1",
            "Dec 31, 2026 · Print at 100%, no colour management",
        ]
    );
}

/// En A4, le format par défaut, les lignes 2 et 3 tiennent sans être
/// coupées par « … », pour chaque taille (libre avec un nombre à quatre
/// chiffres), chaque langue, chaque mois (« 30 sept. 2026 » est le plus
/// long) et le plus grand nombre de pages. La place utile est celle du
/// calcul de rendu, à côté d'un QR code de grand numéro de tirage. La
/// ligne 1 dépend des noms saisis : elle n'entre pas ici.
#[test]
fn les_lignes_2_et_3_tiennent_en_a4_sans_etre_coupees() {
    let gabarit = Gabarit::par_defaut(Format::A4);
    let tailles = [
        (
            Some(TailleMire::Rapide),
            nombre_plages_myiro1(TailleMire::Rapide),
        ),
        (
            Some(TailleMire::Standard),
            nombre_plages_myiro1(TailleMire::Standard),
        ),
        (
            Some(TailleMire::Approfondi),
            nombre_plages_myiro1(TailleMire::Approfondi),
        ),
        (None, 3000),
    ];
    let mut essais = 0;
    for (taille, plages) in tailles {
        let pages = gabarit.pages(plages);
        let code = contenu_qr(&CodeTirage {
            tirage: 999_999,
            page: pages,
            pages,
            empreinte: "ABCDEF01".to_string(),
        });
        let place = caracteres_par_ligne(&gabarit, &code).unwrap();
        for mois in 1..=12 {
            let horodatage = format!("2026-{mois:02}-30T00:00:00Z");
            let t = TexteEtiquette {
                taille,
                plages,
                page: pages,
                pages,
                horodatage: &horodatage,
                ..rapide(1, 1)
            };
            for langue in [Langue::Francais, Langue::Anglais] {
                for ligne in &lignes_etiquette(&t, langue)[1..] {
                    assert_eq!(&couper_ligne(ligne, place), ligne, "{place} caractères");
                    essais += 1;
                }
            }
        }
    }
    assert_eq!(essais, 4 * 12 * 2 * 2);
}

/// Chaque clé du texte imprimé existe en français et en anglais.
#[test]
fn les_cles_de_l_etiquette_existent_en_deux_langues() {
    assert!(CLES_ETIQUETTE.len() >= 8, "{CLES_ETIQUETTE:?}");
    for cle in CLES_ETIQUETTE {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), *cle, "{cle} absente en {langue:?}");
        }
    }
}

/// Les accents restent dessinés : chaque caractère des clés du texte
/// imprimé, dans les deux langues, est dans la police des pages
/// (`font8x8`, table latine). Une espace fine insécable, par exemple,
/// n'y est pas : elle sortirait en « ? ».
#[test]
fn chaque_caractere_de_l_etiquette_est_dessinable() {
    let mut vus = 0;
    for cle in CLES_ETIQUETTE {
        for langue in [Langue::Francais, Langue::Anglais] {
            for c in texte(langue, cle).chars() {
                assert!(
                    mires::dessinable(c),
                    "{c:?} ({:04X}) de {cle} en {langue:?} n'est pas dessinable",
                    c as u32
                );
                vus += 1;
            }
        }
    }
    assert!(vus > 100);
    for c in ['é', 'è', 'à', 'û', '·', '\u{a0}', ' ', 'A', '%'] {
        assert!(mires::dessinable(c), "{c:?}");
    }
    for c in ['\u{202f}', '’', '—', '\u{7f}', '\n'] {
        assert!(!mires::dessinable(c), "{c:?}");
    }
}

/// Préparée en anglais, la mire range sa langue avec le tirage et ses pages
/// diffèrent de celles de la même mire préparée en français (mêmes plages,
/// même heure : seul le texte change).
#[test]
fn la_langue_de_la_preparation_est_rangee_avec_le_tirage() {
    let (fr, en) = (banc(), banc());
    let page_fr = fr
        .preparer_dans(
            Langue::Francais,
            &myiro1(TailleMire::Rapide),
            &moteur(232),
            None,
        )
        .unwrap();
    let page_en = en
        .preparer_dans(
            Langue::Anglais,
            &myiro1(TailleMire::Rapide),
            &moteur(232),
            None,
        )
        .unwrap();
    assert_eq!(
        langue_rangee(&mire_rangee(&fr, "myiro1").0).as_deref(),
        Some("fr")
    );
    assert_eq!(
        langue_rangee(&mire_rangee(&en, "myiro1").0).as_deref(),
        Some("en")
    );
    // Le nom des fichiers ne change pas avec la langue.
    assert_eq!(fr.fichiers(), en.fichiers());
    assert_ne!(
        std::fs::read(&page_fr.fichiers[0]).unwrap(),
        std::fs::read(&page_en.fichiers[0]).unwrap()
    );
}

/// Une page effacée à la main puis redessinée alors que l'application est
/// passée en français garde son texte anglais d'origine, à l'octet près. La
/// langue ne fait pas partie de l'identité de la mire : pas de doublon.
#[test]
fn une_page_effacee_puis_redessinee_garde_la_langue_d_origine() {
    let b = banc();
    let m = moteur(928);
    let premiere = b
        .preparer_dans(Langue::Anglais, &myiro1(TailleMire::Approfondi), &m, None)
        .unwrap();
    let page_2 = std::fs::read(&premiere.fichiers[1]).unwrap();
    std::fs::remove_file(&premiere.fichiers[1]).unwrap();
    let seconde = b
        .preparer_dans(Langue::Francais, &myiro1(TailleMire::Approfondi), &m, None)
        .unwrap();
    assert_eq!(seconde.tirage, premiere.tirage);
    assert!(seconde.deja_preparee.is_some());
    assert_eq!(std::fs::read(&seconde.fichiers[1]).unwrap(), page_2);
    assert_eq!(m.plages_demandees(), [928]);
    assert_eq!(b.tirages(), 1);
    assert_eq!(
        langue_rangee(&mire_rangee(&b, "myiro1").0).as_deref(),
        Some("en")
    );
}

/// Une mire rangée avant le ticket #79 n'a pas de langue dans `generation`.
/// Redessinée, même depuis une application en anglais, sa page porte
/// exactement l'ancien texte : français, date ISO, anciennes lignes. Les
/// lignes attendues sont recopiées de l'ancien code
/// (`app/src/mires/myiro1.rs` avant #79), et la page attendue est dessinée
/// avec elles par la crate `mires`.
#[test]
fn une_mire_rangee_sans_langue_garde_l_ancien_texte() {
    let rapide = myiro1(TailleMire::Rapide);
    let libre = DemandeMire {
        taille: None,
        nombre: Some(232),
        ..myiro1(TailleMire::Rapide)
    };
    for (demande, ligne_2) in [
        (
            &rapide,
            "Mire rapide · 232 plages · MYIRO-1 · page 1 sur 1 · 2026-10-23",
        ),
        (
            &libre,
            "Mire 232 plages · 232 plages · MYIRO-1 · page 1 sur 1 · 2026-10-23",
        ),
    ] {
        // Plages et mise en page d'une mire préparée, rangées sans langue
        // dans une autre bibliothèque, comme avant #79.
        let reference = banc();
        reference
            .preparer_dans(Langue::Francais, demande, &moteur(232), None)
            .unwrap();
        let (mire, horodatage) = mire_rangee(&reference, "myiro1");
        let mut generation: serde_json::Value = serde_json::from_str(&mire.generation).unwrap();
        generation.as_object_mut().unwrap().remove("langue");
        let generation = generation.to_string();
        let b = banc();
        {
            let bibliotheque = Bibliotheque::ouvrir(&emplacement(b.dossier.path())).unwrap();
            bibliotheque
                .enregistrer_mire_et_tirage(
                    b.condition,
                    &NouvelleMire {
                        nom: &mire.nom,
                        instrument: "myiro1",
                        plages: mire.plages,
                        generation: &generation,
                        valeurs: &mire.valeurs,
                        mise_en_page: &mire.mise_en_page,
                        empreinte: &mire.empreinte,
                        horodatage: &horodatage,
                    },
                    1,
                )
                .unwrap();
        }
        let m = moteur(232);
        let reprise = b.preparer_dans(Langue::Anglais, demande, &m, None).unwrap();
        assert!(reprise.deja_preparee.is_some());
        assert!(m.plages_demandees().is_empty());

        let valeurs: Vec<[f64; 3]> = serde_json::from_str(&mire.valeurs).unwrap();
        let mise: MiseEnPage = serde_json::from_str(&mire.mise_en_page).unwrap();
        let etiquette = Etiquette {
            lignes: vec![
                "Canon PRO-4100 · Hahnemühle Photo Rag 308 · 720 dpi".to_string(),
                ligne_2.to_string(),
                "Imprimer à 100 %, sans gestion des couleurs".to_string(),
            ],
            code: contenu_qr(&CodeTirage {
                tirage: reprise.tirage,
                page: 1,
                pages: 1,
                empreinte: mire.empreinte.clone(),
            }),
        };
        let mut attendue = Vec::new();
        ecrire_tiff(
            &rendre_page(&mise, &valeurs, 1, &etiquette).unwrap(),
            &mut attendue,
        )
        .unwrap();
        assert!(
            std::fs::read(&reprise.fichiers[0]).unwrap() == attendue,
            "{ligne_2}"
        );
    }
}

/// La mire FD-9 RGB-2520 n'est pas touchée : copiée octet pour octet,
/// rangée sans langue, et refusée de la même façon dans les deux langues.
#[test]
fn la_mire_fd9_est_inchangee() {
    let b = banc();
    let (octets, mire) = lire_fictive();
    let preparee = b
        .ouverte
        .preparer_mire_fd9(b.condition, mire, &b.mires(), maintenant())
        .unwrap();
    assert_eq!(std::fs::read(&preparee.fichiers[0]).unwrap(), octets);
    assert_eq!(langue_rangee(&mire_rangee(&b, "fd9").0), None);
    let fichier = b.dossier.path().join("autre.tif");
    std::fs::write(&fichier, &octets).unwrap();
    for langue in [Langue::Francais, Langue::Anglais] {
        assert_eq!(
            b.preparer_dans(langue, &fd9(), &moteur(232), Some(&fichier))
                .map(|_| ()),
            Err("mire.erreur.fd9_autre".to_string())
        );
    }
    assert_eq!(b.tirages(), 1);
}
