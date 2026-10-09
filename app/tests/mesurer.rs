//! Tâche Mesurer, de bout en bout contre le pont simulé (ticket #7) : la
//! mesure ponctuelle du module `instrument`, ses valeurs calculées par la crate
//! `colorimetrie` à partir du spectre, et son rangement dans la condition
//! d'impression choisie de la bibliothèque.

use std::path::PathBuf;

use app::instrument::{Accord, Geste, Instrument};
use app::mesurer::{FicheMesure, Seance};
use app::pont::{Architecture, PontSimule};
use app::textes::Langue;
use bibliotheque::{Bibliotheque, ConditionImpression, IdMesure};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Geometrie,
    Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Palier, Plage,
    Provenance, RemiseAuRepos, Reponse, Spectre,
};

/// Numéro de série fictif : jamais celui d'un instrument réel.
const SERIE: u32 = 12345678;

fn dossier_vide(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("mesurer")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    dossier
}

/// Instrument étalonné sur le pont simulé donné. La DLL n'est qu'un en-tête
/// PE : le pont simulé n'en charge aucune.
fn etalonne(simule: PontSimule, nom: &str) -> Instrument<PontSimule> {
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
    instrument
}

/// Opérateur simulé qui fait chaque geste demandé.
fn fait(_: Geste) -> Accord {
    Accord::Fait
}

/// Bibliothèque neuve avec une condition d'impression.
fn bibliotheque(nom: &str) -> (Bibliotheque, ConditionImpression) {
    let biblio = Bibliotheque::ouvrir(&dossier_vide(nom)).unwrap();
    let condition = biblio.creer_condition("Offset, couché mat").unwrap();
    (biblio, condition)
}

/// Range dans la bibliothèque, comme l'application.
fn ranger<'a>(
    biblio: &'a Bibliotheque,
    condition: &'a ConditionImpression,
) -> impl FnOnce(&Mesure, &str) -> Result<IdMesure, String> + 'a {
    move |mesure, nom| {
        biblio
            .enregistrer_mesure_nommee(condition.id, mesure, nom)
            .map_err(|e| e.to_string())
    }
}

/// Renomme dans la bibliothèque, comme l'application.
fn renommer_dans(biblio: &Bibliotheque) -> impl FnOnce(IdMesure, &str) -> Result<(), String> + '_ {
    move |id, nom| biblio.renommer_mesure(id, nom).map_err(|e| e.to_string())
}

/// Nom conservé par la bibliothèque pour sa seule mesure (ou la plus récente).
fn nom_range(biblio: &Bibliotheque) -> Option<String> {
    biblio.arborescence("").unwrap()[0].mesures[0].nom.clone()
}

#[test]
fn une_mesure_rejoint_la_condition_d_impression_choisie_avec_sa_provenance() {
    let mut instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "rangee");
    let (biblio, condition) = bibliotheque("rangee-biblio");
    let mut seance = Seance::default();

    let numero = seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            ranger(&biblio, &condition),
            Langue::Francais,
        )
        .expect("mesure faite");

    let branche = &biblio.arborescence("").unwrap()[0];
    assert_eq!(branche.condition.id, condition.id);
    assert_eq!(branche.mesures.len(), 1);
    let rangee = biblio.mesure(branche.mesures[0].id).unwrap();
    // La bibliothèque garde la mesure du pont, provenance comprise.
    assert_eq!(&rangee.mesure, seance.mesure(numero).unwrap());
    assert_eq!(rangee.mesure.provenance().instrument.numero_serie, SERIE);
    // Son nom aussi, celui de la liste de Mesurer.
    assert_eq!(rangee.nom.as_deref(), Some("Couleur 1"));

    let fiches = seance.fiches(Langue::Francais);
    assert_eq!(fiches.len(), 1);
    assert_eq!(fiches[0].condition_impression, "Offset, couché mat");
    assert_eq!(fiches[0].erreur_rangement, None);
}

#[test]
fn les_mesures_s_empilent_la_plus_recente_en_haut_avec_un_nom_modifiable() {
    let mut instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "pile");
    let (biblio, condition) = bibliotheque("pile-biblio");
    let mut seance = Seance::default();
    for _ in 0..2 {
        seance
            .mesurer(
                &mut instrument,
                &mut fait,
                &condition,
                ranger(&biblio, &condition),
                Langue::Francais,
            )
            .expect("mesure faite");
    }

    let noms = |s: &Seance| -> Vec<String> {
        s.fiches(Langue::Francais)
            .into_iter()
            .map(|f| f.nom)
            .collect()
    };
    assert_eq!(noms(&seance), ["Couleur 2", "Couleur 1"]);

    seance
        .renommer(1, "  Magenta du logo ", renommer_dans(&biblio))
        .unwrap();
    assert_eq!(noms(&seance), ["Couleur 2", "Magenta du logo"]);
    // Le nouveau nom est conservé par la bibliothèque.
    let rangees: Vec<_> = biblio.arborescence("").unwrap()[0]
        .mesures
        .iter()
        .map(|m| m.nom.clone().unwrap())
        .collect();
    assert!(
        rangees.contains(&"Magenta du logo".to_string()),
        "{rangees:?}"
    );

    assert_eq!(
        seance.renommer(2, "   ", renommer_dans(&biblio)),
        Err("bibliotheque.erreur.nom_vide")
    );
    assert_eq!(
        seance.renommer(9, "Absente", renommer_dans(&biblio)),
        Err("bibliotheque.erreur.autre")
    );
    assert_eq!(noms(&seance), ["Couleur 2", "Magenta du logo"]);
}

/// Si la bibliothèque refuse le nouveau nom, la liste garde l'ancien : les
/// deux disent toujours la même chose.
#[test]
fn un_renommage_refuse_par_la_bibliotheque_ne_change_rien() {
    let mut instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "renommage-refuse");
    let (biblio, condition) = bibliotheque("renommage-refuse-biblio");
    let mut seance = Seance::default();
    seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            ranger(&biblio, &condition),
            Langue::Francais,
        )
        .expect("mesure faite");

    assert_eq!(
        seance.renommer(1, "Cyan", |_, _| Err("base verrouillée".into())),
        Err("bibliotheque.erreur.autre")
    );

    assert_eq!(seance.fiches(Langue::Francais)[0].nom, "Couleur 1");
    assert_eq!(nom_range(&biblio).as_deref(), Some("Couleur 1"));
}

/// Une mesure lue n'est jamais perdue : si la bibliothèque ne peut pas la
/// ranger, elle reste dans la séance et l'écran le dit.
#[test]
fn une_mesure_que_la_bibliotheque_refuse_reste_dans_la_seance() {
    let mut instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "refusee");
    let (_biblio, condition) = bibliotheque("refusee-biblio");
    let mut seance = Seance::default();

    let numero = seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            |_, _| Err("disque plein".to_string()),
            Langue::Francais,
        )
        .expect("mesure faite");

    assert!(seance.mesure(numero).is_some());
    let fiche = &seance.fiches(Langue::Francais)[0];
    assert_eq!(fiche.erreur_rangement, Some("mesurer.erreur.rangement"));
    // Une mesure non rangée se renomme dans la séance seule.
    seance
        .renommer(numero, "Cyan", |_, _| {
            panic!("rien à renommer dans la bibliothèque")
        })
        .unwrap();
    assert_eq!(seance.fiches(Langue::Francais)[0].nom, "Cyan");
}

/// Sans mesure (instrument non étalonné), rien n'entre dans la séance ni
/// dans la bibliothèque.
#[test]
fn sans_mesure_rien_n_est_range() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let dossier = dossier_vide("sans-mesure-sdk");
    let mut instrument = Instrument::ouvrir(&[dossier], &[], |_, _, _| Ok(simule));
    let (biblio, condition) = bibliotheque("sans-mesure-biblio");
    let mut seance = Seance::default();

    let numero = seance.mesurer(
        &mut instrument,
        &mut fait,
        &condition,
        ranger(&biblio, &condition),
        Langue::Francais,
    );

    assert_eq!(numero, None);
    assert!(seance.fiches(Langue::Francais).is_empty());
    assert!(biblio.arborescence("").unwrap()[0].mesures.is_empty());
}

/// Conditions de calcul telles qu'un pont MYIRO-1 les déclare.
fn demande_myiro1() -> Info<ConditionsCalcul> {
    Info::Confirmee(ConditionsCalcul {
        conditions_spectres: [
            Info::Confirmee(ConditionMesure::M0),
            Info::Confirmee(ConditionMesure::M1),
            Info::Confirmee(ConditionMesure::M2),
        ],
        longueurs_onde: Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10,
        }),
        illuminant_lab: Info::Confirmee(Illuminant::D50),
        observateur_lab: Info::Supposee(Observateur::DeuxDegres),
    })
}

/// Mesure ponctuelle d'un gris neutre (réflectance 0,18 partout), rendue par
/// le pont avec les conditions de calcul données.
fn gris(demande: Info<ConditionsCalcul>) -> Mesure {
    let spectre = || Spectre::new(vec![0.18; 36]).unwrap();
    let lab = || Lab::new([0.0, 0.0, 0.0]).unwrap();
    let plage = Plage::new(
        [spectre(), spectre(), spectre()],
        DonneesBrutes::new(vec![1.0; 152]).unwrap(),
        [lab(), lab(), lab()],
    )
    .unwrap();
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
        horodatage: Horodatage::new("2026-10-07T09:31:00+02:00").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie: Geometrie::Ponctuelle {},
        calcul: Calcul {
            libelle: "test".into(),
            demande,
            observe: Info::Inconnue,
        },
    };
    Mesure::new(vec![plage], provenance).unwrap()
}

/// Mesure le gris et rend sa fiche, dans la langue donnée.
fn fiche_du_gris(nom: &str, demande: Info<ConditionsCalcul>, langue: Langue) -> FicheMesure {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::MesurePonctuelle,
        Ok(Reponse::Mesure {
            mesure: gris(demande),
            remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
        }),
    );
    let mut instrument = etalonne(simule, nom);
    let (biblio, condition) = bibliotheque(&format!("{nom}-biblio"));
    let mut seance = Seance::default();
    seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            ranger(&biblio, &condition),
            langue,
        )
        .expect("mesure faite");
    seance.fiches(langue).remove(0)
}

/// Lab, LCH et XYZ sont calculés par notre colorimétrie à partir du
/// spectre (pas repris des Lab de la DLL, nuls ici), à deux décimales, avec la
/// virgule en français. Un gris n'a pas de teinte : elle est inconnue.
#[test]
fn les_valeurs_viennent_du_spectre_a_deux_decimales() {
    let fiche = fiche_du_gris("gris-fr", demande_myiro1(), Langue::Francais);

    assert_eq!(fiche.spectres.len(), 3);
    for (i, condition) in [
        ConditionMesure::M0,
        ConditionMesure::M1,
        ConditionMesure::M2,
    ]
    .into_iter()
    .enumerate()
    {
        let spectre = &fiche.spectres[i];
        assert_eq!(spectre.condition, Info::Confirmee(condition));
        let v = spectre.valeurs.as_ref().expect("valeurs calculées");
        assert_eq!(
            [v.l.as_str(), v.a.as_str(), v.b.as_str()],
            ["49,50", "0,00", "0,00"]
        );
        assert_eq!(v.c, "0,00");
        assert_eq!(v.h, None, "un gris n'a pas de teinte");
        assert_eq!(v.y, "18,00");
        assert!(v.x.contains(',') && v.z.contains(','), "{v:?}");
    }

    let anglais = fiche_du_gris("gris-en", demande_myiro1(), Langue::Anglais);
    let v = anglais.spectres[0].valeurs.as_ref().unwrap();
    assert_eq!((v.l.as_str(), v.y.as_str()), ("49.50", "18.00"));
}

#[test]
fn les_nombres_ont_deux_decimales_et_la_virgule_en_francais() {
    use app::mesurer::decimal;
    assert_eq!(decimal(-12.3456, Langue::Francais), "-12,35");
    assert_eq!(decimal(7.0, Langue::Francais), "7,00");
    assert_eq!(decimal(-0.001, Langue::Francais), "0,00");
    assert_eq!(decimal(-12.3456, Langue::Anglais), "-12.35");
}

/// Ce que le pont n'a pas dit reste inconnu : ni condition de mesure devinée
/// d'après la place du spectre, ni valeurs calculées sur des longueurs
/// d'onde supposées.
#[test]
fn sans_conditions_de_calcul_tout_reste_inconnu() {
    let fiche = fiche_du_gris("gris-inconnu", Info::Inconnue, Langue::Francais);

    for spectre in &fiche.spectres {
        assert_eq!(spectre.condition, Info::Inconnue);
        assert_eq!(spectre.valeurs, None);
    }
}
