//! Densités d'une mesure pour la page (ticket #31) : calculées par la crate
//! `colorimetrie` depuis le spectre M0, absolues ou relatives à un blanc
//! papier mesuré dans la même condition ; inconnues, avec leur raison, quand
//! elles ne se calculent pas.

use std::path::PathBuf;

use app::instrument::{Accord, Geste, Instrument};
use app::mesurer::{densites_plage, FicheDensites, Seance, ValeursDensite};
use app::pont::{Architecture, PontSimule};
use app::textes::{texte, Langue};
use bibliotheque::{Bibliotheque, ConditionImpression, IdMesure};
use colorimetrie::StatutDensite;
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Geometrie,
    Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Palier, Plage,
    Provenance, RemiseAuRepos, Reponse, Spectre,
};

/// Numéro de série fictif : jamais celui d'un instrument réel.
const SERIE: u32 = 12345678;

use ConditionMesure::{M0, M1, M2};

fn conditions(spectres: [Info<ConditionMesure>; 3]) -> Info<ConditionsCalcul> {
    Info::Confirmee(ConditionsCalcul {
        conditions_spectres: spectres,
        longueurs_onde: Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10,
        }),
        illuminant_lab: Info::Confirmee(Illuminant::D50),
        observateur_lab: Info::Supposee(Observateur::DeuxDegres),
    })
}

fn myiro1() -> Info<ConditionsCalcul> {
    conditions([
        Info::Confirmee(M0),
        Info::Confirmee(M1),
        Info::Confirmee(M2),
    ])
}

/// Mesure dont chaque plage a des spectres uniformes : `plages[i]` donne les
/// réflectances des spectres 1, 2 et 3 de la plage i.
fn mesure(plages: &[[f32; 3]], demande: Info<ConditionsCalcul>, geometrie: Geometrie) -> Mesure {
    let plages = plages
        .iter()
        .map(|r| {
            let spectre = |v: f32| Spectre::new(vec![v; 36]).unwrap();
            let lab = || Lab::new([0.0, 0.0, 0.0]).unwrap();
            Plage::new(
                [spectre(r[0]), spectre(r[1]), spectre(r[2])],
                DonneesBrutes::new(vec![1.0; 152]).unwrap(),
                [lab(), lab(), lab()],
            )
            .unwrap()
        })
        .collect();
    let provenance = Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie: SERIE,
            micrologiciel: "1.00".into(),
            code_produit: "simule".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Inconnue,
        version_pont: "simulé".into(),
        architecture: "x86_64".into(),
        horodatage: Horodatage::new("2026-10-09T09:31:00+02:00").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "test".into(),
            demande,
            observe: Info::Inconnue,
        },
    };
    Mesure::new(plages, provenance).unwrap()
}

/// Mesure ponctuelle : M0 à `m0`, M1 et M2 ailleurs.
fn ponctuelle(m0: f32) -> Mesure {
    mesure(&[[m0, 0.5, 0.5]], myiro1(), Geometrie::Ponctuelle {})
}

fn valeurs(fiche: &FicheDensites) -> [&str; 4] {
    let v: &ValeursDensite = fiche.valeurs.as_ref().expect("densités calculées");
    [&v.cyan, &v.magenta, &v.jaune, &v.visuel].map(String::as_str)
}

#[test]
fn la_densite_absolue_vient_du_spectre_m0_a_deux_decimales() {
    // Un dixième de la lumière renvoyé : densité 1 par définition.
    let fiche = densites_plage(
        &ponctuelle(0.1),
        0,
        StatutDensite::T,
        None,
        Langue::Francais,
    );
    assert_eq!(valeurs(&fiche), ["1,00"; 4]);
    assert_eq!(fiche.statut, "T");
    assert!(!fiche.relative);
    assert_eq!(fiche.condition, Info::Confirmee(M0));
    assert_eq!(fiche.raison, None);

    let anglais = densites_plage(
        &ponctuelle(0.01),
        0,
        StatutDensite::E,
        None,
        Langue::Anglais,
    );
    assert_eq!(valeurs(&anglais), ["2.00"; 4]);
    assert_eq!(anglais.statut, "E");
}

#[test]
fn le_spectre_m0_est_cherche_par_sa_condition_et_non_par_sa_place() {
    // Le pont rend M0 en troisième : c'est lui qui compte.
    let demande = conditions([
        Info::Confirmee(M1),
        Info::Confirmee(M2),
        Info::Confirmee(M0),
    ]);
    let m = mesure(&[[0.5, 0.5, 0.1]], demande, Geometrie::Ponctuelle {});
    let fiche = densites_plage(&m, 0, StatutDensite::T, None, Langue::Francais);
    assert_eq!(valeurs(&fiche), ["1,00"; 4]);
}

#[test]
fn une_condition_m0_supposee_donne_des_densites_a_confirmer() {
    let demande = conditions([Info::Supposee(M0), Info::Inconnue, Info::Inconnue]);
    let m = mesure(&[[0.1, 0.5, 0.5]], demande, Geometrie::Ponctuelle {});
    let fiche = densites_plage(&m, 0, StatutDensite::T, None, Langue::Francais);
    assert_eq!(valeurs(&fiche), ["1,00"; 4]);
    assert_eq!(fiche.condition, Info::Supposee(M0));
}

#[test]
fn sans_spectre_m0_la_densite_est_inconnue_et_jamais_nulle() {
    let demande = conditions([Info::Confirmee(M1), Info::Confirmee(M2), Info::Inconnue]);
    let m = mesure(&[[0.1, 0.1, 0.1]], demande, Geometrie::Ponctuelle {});
    let fiche = densites_plage(&m, 0, StatutDensite::T, None, Langue::Francais);
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.condition"));
    assert_eq!(fiche.condition, Info::Inconnue);

    // Conditions de calcul inconnues : rien n'est deviné d'après la place.
    let m = mesure(&[[0.1, 0.1, 0.1]], Info::Inconnue, Geometrie::Ponctuelle {});
    let fiche = densites_plage(&m, 0, StatutDensite::E, None, Langue::Francais);
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.condition"));
}

#[test]
fn des_longueurs_d_onde_inconnues_donnent_une_densite_inconnue() {
    let demande = Info::Confirmee(ConditionsCalcul {
        conditions_spectres: [
            Info::Confirmee(M0),
            Info::Confirmee(M1),
            Info::Confirmee(M2),
        ],
        longueurs_onde: Info::Inconnue,
        illuminant_lab: Info::Confirmee(Illuminant::D50),
        observateur_lab: Info::Supposee(Observateur::DeuxDegres),
    });
    let m = mesure(&[[0.1, 0.1, 0.1]], demande, Geometrie::Ponctuelle {});
    let fiche = densites_plage(&m, 0, StatutDensite::T, None, Langue::Francais);
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.spectre"));
}

#[test]
fn un_noir_total_a_une_densite_inconnue() {
    let fiche = densites_plage(
        &ponctuelle(0.0),
        0,
        StatutDensite::T,
        None,
        Langue::Francais,
    );
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.calcul"));
}

#[test]
fn la_densite_relative_se_rapporte_au_blanc_papier_de_meme_condition() {
    let papier = ponctuelle(0.9);
    let fiche = densites_plage(
        &ponctuelle(0.09),
        0,
        StatutDensite::T,
        Some((&papier, 0)),
        Langue::Francais,
    );
    assert!(fiche.relative);
    assert_eq!(valeurs(&fiche), ["1,00"; 4]);
    assert_eq!(fiche.condition_papier, Some(Info::Confirmee(M0)));
}

#[test]
fn un_papier_sans_spectre_m0_ne_sert_pas_de_reference() {
    let demande = conditions([
        Info::Confirmee(M1),
        Info::Confirmee(M1),
        Info::Confirmee(M2),
    ]);
    let papier = mesure(&[[0.9, 0.9, 0.9]], demande, Geometrie::Ponctuelle {});
    let fiche = densites_plage(
        &ponctuelle(0.09),
        0,
        StatutDensite::T,
        Some((&papier, 0)),
        Langue::Francais,
    );
    assert!(fiche.relative);
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.papier_condition"));
}

#[test]
fn la_densite_d_une_plage_de_mire_se_lit_plage_par_plage() {
    let bande = mesure(
        &[[0.9, 0.9, 0.9], [0.09, 0.5, 0.5], [0.009, 0.5, 0.5]],
        myiro1(),
        Geometrie::Bande { sens: 0 },
    );
    let absolue = densites_plage(&bande, 2, StatutDensite::E, None, Langue::Francais);
    assert_eq!(valeurs(&absolue), ["2,05"; 4]);
    // Le papier est la première plage de la bande.
    let relative = densites_plage(
        &bande,
        1,
        StatutDensite::E,
        Some((&bande, 0)),
        Langue::Anglais,
    );
    assert_eq!(valeurs(&relative), ["1.00"; 4]);
    // Une plage qui n'existe pas n'a pas de densité.
    let absente = densites_plage(&bande, 3, StatutDensite::E, None, Langue::Francais);
    assert_eq!(absente.valeurs, None);
    assert_eq!(absente.raison, Some("densite.inconnue.spectre"));
}

#[test]
fn la_limite_connue_et_les_raisons_ont_un_texte_dans_les_deux_langues() {
    let fiche = densites_plage(
        &ponctuelle(0.1),
        0,
        StatutDensite::T,
        None,
        Langue::Francais,
    );
    for cle in [
        fiche.limite,
        "densite.inconnue.condition",
        "densite.inconnue.spectre",
        "densite.inconnue.calcul",
        "densite.inconnue.papier",
        "densite.inconnue.papier_condition",
        // Libellés que le panneau compose (`densite.${…}` dans mesurer.js).
        "densite.statut",
        "densite.mode",
        "densite.cyan",
        "densite.magenta",
        "densite.jaune",
        "densite.visuel",
    ] {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} sans texte");
        }
    }
}

// ---- Depuis la séance, comme la commande de la page ----

fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("densites")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    dossier
}

fn fait(_: Geste) -> Accord {
    Accord::Fait
}

/// Séance de deux mesures : la mesure simulée du pont (n° 1), puis la
/// mesure imposée (n° 2).
fn seance_de_deux(nom: &str, seconde: Mesure) -> Seance {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_apres(
        Palier::MesurePonctuelle,
        1,
        Ok(Reponse::Mesure {
            mesure: seconde,
            remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
        }),
    );
    let dossier = dossier_vide(&format!("{nom}-sdk"));
    let mut octets = vec![0u8; 0x48];
    octets[..2].copy_from_slice(b"MZ");
    octets[0x3c] = 0x40;
    octets[0x40..0x44].copy_from_slice(b"PE\0\0");
    octets[0x44..0x46].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(dossier.join("FDXSDK.dll"), octets).unwrap();
    let ponts = [(Architecture::X64, PathBuf::from("pont-myiro1-x64.exe"))];
    let mut instrument = Instrument::ouvrir(&[dossier], &ponts, |_, _, _| Ok(simule));
    instrument.etalonner(&mut fait);
    let biblio = Bibliotheque::ouvrir(&dossier_vide(&format!("{nom}-biblio"))).unwrap();
    let condition: ConditionImpression = biblio.creer_condition("Offset, couché mat").unwrap();
    let mut seance = Seance::default();
    for _ in 0..2 {
        seance
            .mesurer(
                &mut instrument,
                &mut fait,
                &condition,
                |m: &Mesure, n: &str| -> Result<IdMesure, String> {
                    biblio
                        .enregistrer_mesure_nommee(condition.id, m, n)
                        .map_err(|e| e.to_string())
                },
                Langue::Francais,
            )
            .expect("mesure faite");
    }
    seance
}

#[test]
fn la_seance_donne_les_densites_d_une_mesure_et_de_son_papier() {
    let seance = seance_de_deux("seance", ponctuelle(0.1));

    let absolue = seance
        .densites(2, StatutDensite::T, None, Langue::Francais)
        .expect("mesure n° 2");
    assert_eq!(valeurs(&absolue), ["1,00"; 4]);
    assert_eq!(absolue.papier_nom, None);

    // La mesure n° 2 prise comme son propre papier : densité relative nulle.
    let elle_meme = seance
        .densites(2, StatutDensite::T, Some(2), Langue::Francais)
        .unwrap();
    assert_eq!(valeurs(&elle_meme), ["0,00"; 4]);
    assert_eq!(elle_meme.papier_nom.as_deref(), Some("Couleur 2"));

    // Mesure n° 1 (simulée, magenta) rapportée au papier n° 2 : le magenta
    // y est plus dense que le jaune.
    let magenta = seance
        .densites(1, StatutDensite::E, Some(2), Langue::Anglais)
        .unwrap();
    assert!(magenta.relative);
    let v = magenta.valeurs.as_ref().unwrap();
    assert!(v.magenta.parse::<f64>().unwrap() > v.jaune.parse::<f64>().unwrap());

    assert_eq!(
        seance.densites(3, StatutDensite::T, None, Langue::Francais),
        None
    );
}

#[test]
fn un_papier_absent_de_la_seance_rend_la_densite_relative_inconnue() {
    let seance = seance_de_deux("papier-absent", ponctuelle(0.1));
    let fiche = seance
        .densites(2, StatutDensite::T, Some(7), Langue::Francais)
        .unwrap();
    assert!(fiche.relative);
    assert_eq!(fiche.valeurs, None);
    assert_eq!(fiche.raison, Some("densite.inconnue.papier"));
}
