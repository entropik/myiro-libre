//! Texte imprimé sur les pages d'une mire MYIRO-1 dans la langue de
//! l'application au moment de la préparation (ticket #79). La langue est
//! rangée avec le tirage : une page redessinée garde son texte d'origine.
//! La mire FD-9 RGB-2520 n'est jamais touchée. Moteur simulé, jamais
//! ArgyllCMS ; aucune image comparée pour le texte, seulement ses lignes.

mod commun_mires;

use app::colonne::emplacement;
use app::mires::{lignes_etiquette, TexteEtiquette, CLES_ETIQUETTE};
use app::textes::{texte, Langue};
use bibliotheque::{Bibliotheque, MireRangee, NouvelleMire};
use commun_mires::*;
use mires::TailleMire;

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
            "Mire rapide · 232 plages · MYIRO-1 · page 1 sur 1 · 23 oct. 2026",
            "Imprimer à 100\u{a0}%, sans gestion des couleurs",
        ]
    );
}

#[test]
fn l_etiquette_anglaise_est_en_anglais() {
    assert_eq!(
        lignes_etiquette(&rapide(2, 3), Langue::Anglais),
        [
            "Canon PRO-4100 · Hahnemühle Photo Rag 308 · 720 dpi",
            "Chart: quick · 232 patches · MYIRO-1 · page 2 of 3 · Oct 23, 2026",
            "Print at 100%, without colour management",
        ]
    );
}

/// Taille libre (section « Avancé ») et autres tailles, dans les deux
/// langues ; la date suit le mois.
#[test]
fn chaque_taille_et_chaque_mois_ont_leur_texte() {
    let libre = TexteEtiquette {
        taille: None,
        plages: 500,
        horodatage: "2026-02-01T00:00:00Z",
        ..rapide(1, 2)
    };
    assert_eq!(
        lignes_etiquette(&libre, Langue::Francais)[1],
        "Mire 500 plages · 500 plages · MYIRO-1 · page 1 sur 2 · 1 févr. 2026"
    );
    assert_eq!(
        lignes_etiquette(&libre, Langue::Anglais)[1],
        "Chart: 500 patches · 500 patches · MYIRO-1 · page 1 of 2 · Feb 1, 2026"
    );
    let approfondi = TexteEtiquette {
        taille: Some(TailleMire::Approfondi),
        plages: 928,
        horodatage: "2026-08-15T12:00:00Z",
        ..rapide(1, 2)
    };
    assert_eq!(
        lignes_etiquette(&approfondi, Langue::Francais)[1],
        "Mire approfondi · 928 plages · MYIRO-1 · page 1 sur 2 · 15 août 2026"
    );
    assert_eq!(
        lignes_etiquette(&approfondi, Langue::Anglais)[1],
        "Chart: thorough · 928 patches · MYIRO-1 · page 1 of 2 · Aug 15, 2026"
    );
    let standard = TexteEtiquette {
        taille: Some(TailleMire::Standard),
        plages: 464,
        horodatage: "2026-12-31T23:59:59Z",
        ..rapide(1, 1)
    };
    assert_eq!(
        lignes_etiquette(&standard, Langue::Anglais)[1],
        "Chart: standard · 464 patches · MYIRO-1 · page 1 of 1 · Dec 31, 2026"
    );
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

/// Une mire rangée avant le ticket #79, sans langue dans `generation`,
/// compte comme du français : redessinée depuis une application en
/// anglais, sa page est celle d'une préparation en français.
#[test]
fn une_mire_rangee_sans_langue_est_redessinee_en_francais() {
    let reference = banc();
    let francaise = reference
        .preparer_dans(
            Langue::Francais,
            &myiro1(TailleMire::Rapide),
            &moteur(232),
            None,
        )
        .unwrap();
    let page_fr = std::fs::read(&francaise.fichiers[0]).unwrap();
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
    let reprise = b
        .preparer_dans(Langue::Anglais, &myiro1(TailleMire::Rapide), &m, None)
        .unwrap();
    assert!(reprise.deja_preparee.is_some());
    assert_eq!(reprise.tirage, francaise.tirage);
    assert!(m.plages_demandees().is_empty());
    assert_eq!(std::fs::read(&reprise.fichiers[0]).unwrap(), page_fr);
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
