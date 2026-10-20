//! Tâche Mesurer, de bout en bout contre le pont simulé (ticket #7) : la
//! mesure ponctuelle du module `instrument`, ses valeurs calculées par la crate
//! `colorimetrie` à partir du spectre, et son rangement dans la condition
//! d'impression choisie de la bibliothèque.

use std::path::PathBuf;

use app::instrument::{Accord, Geste, Instrument};
use app::mesurer::{
    ecrire_declenchement, lire_declenchement, retenir_declenchement, FicheMesure, Seance,
};
use app::pont::{Architecture, PontSimule};
use app::textes::Langue;
use bibliotheque::{Bibliotheque, ConditionImpression, IdMesure};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, Declenchement, DonneesBrutes, Echantillonnage,
    Geometrie, Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Palier,
    Plage, Provenance, RemiseAuRepos, Reponse, Requete, Spectre,
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

// ---- Mesure automatique ou manuelle (ticket #51) ----

/// Par défaut, « Mesurer » déclenche la mesure (demande du mainteneur).
#[test]
fn la_seance_mesure_en_automatique_par_defaut_puis_comme_choisi() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "declenchement");
    let (biblio, condition) = bibliotheque("declenchement-biblio");
    let mut seance = Seance::default();
    assert_eq!(seance.declenchement(), Declenchement::Automatique);

    for choix in [Declenchement::Automatique, Declenchement::Manuel] {
        seance.choisir_declenchement(choix);
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
            journal.requetes().last(),
            Some(&Requete::MesurerPonctuelle {
                declenchement: choix
            })
        );
    }
}

/// Le choix est retenu d'une fois sur l'autre dans un petit fichier texte ;
/// illisible ou absent, c'est l'automatique.
#[test]
fn le_choix_du_declenchement_se_relit_tel_qu_il_a_ete_ecrit() {
    for choix in [Declenchement::Automatique, Declenchement::Manuel] {
        assert_eq!(lire_declenchement(Some(ecrire_declenchement(choix))), choix);
    }
    assert_eq!(lire_declenchement(Some("manuel\n")), Declenchement::Manuel);
    assert_eq!(lire_declenchement(None), Declenchement::Automatique);
    assert_eq!(lire_declenchement(Some("??")), Declenchement::Automatique);
}

#[test]
fn le_choix_retenu_s_ecrit_dans_son_fichier_et_un_echec_est_rapporte() {
    let dossier = dossier_vide("declenchement-fichier");
    let fichier = dossier.join("config").join("mode-mesure.txt");
    retenir_declenchement(&fichier, Declenchement::Manuel).expect("écrit");
    let relu = std::fs::read_to_string(&fichier).unwrap();
    assert_eq!(lire_declenchement(Some(&relu)), Declenchement::Manuel);

    // Le « dossier » est un fichier : l'écriture échoue, et on le dit.
    let bloque = dossier.join("bloque");
    std::fs::write(&bloque, "").unwrap();
    assert!(retenir_declenchement(&bloque.join("mode-mesure.txt"), Declenchement::Manuel).is_err());
}

#[test]
fn le_choix_de_la_mesure_a_ses_textes_dans_les_deux_langues() {
    use app::textes::texte;
    for langue in [Langue::Francais, Langue::Anglais] {
        for cle in [
            "mesurer.declenchement",
            "mesurer.declenchement.automatique",
            "mesurer.declenchement.manuel",
            "mesurer.consigne.automatique",
            "mesurer.consigne.manuel",
            "mesurer.declenchement.non_retenu",
        ] {
            assert_ne!(texte(langue, cle), cle, "texte manquant : {cle}");
        }
    }
    let consigne = texte(Langue::Francais, "mesurer.consigne.automatique");
    assert!(
        consigne.contains("Posez le MYIRO-1 sur la couleur"),
        "{consigne}"
    );
    assert!(!consigne.contains("bouton"), "{consigne}");
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
    fiche_de(nom, gris(demande), langue)
}

/// Mesure rendue telle quelle par le pont simulé, et sa fiche.
fn fiche_de(nom: &str, mesure: Mesure, langue: Langue) -> FicheMesure {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::MesurePonctuelle,
        Ok(Reponse::Mesure {
            mesure,
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

// ---- Corrections de relecture ----

/// Le carré de couleur vient de notre colorimétrie (Lab D50 → sRGB), pour
/// chaque spectre ; le gris 18 % (L* 49,50) donne un gris sRGB de 118.
#[test]
fn chaque_spectre_a_sa_couleur_a_l_ecran() {
    let fiche = fiche_du_gris("gris-ecran", demande_myiro1(), Langue::Francais);

    for spectre in &fiche.spectres {
        let v = spectre.valeurs.as_ref().unwrap();
        assert_eq!(v.ecran, "#767676");
        assert!(!v.approchee);
    }
    // Sans valeurs, pas de couleur : « inconnu » à l'écran.
    let inconnu = fiche_du_gris("gris-ecran-inconnu", Info::Inconnue, Langue::Francais);
    assert!(inconnu.spectres.iter().all(|s| s.valeurs.is_none()));
}

/// Un micrologiciel que le pont n'a pas su lire (texte vide) est inconnu,
/// jamais une case vide du cartouche.
#[test]
fn un_micrologiciel_vide_est_inconnu() {
    let mut mesure = gris(demande_myiro1());
    let mut provenance = mesure.provenance().clone();
    provenance.instrument.micrologiciel = "  ".into();
    mesure = Mesure::new(mesure.plages().to_vec(), provenance).unwrap();

    let fiche = fiche_de("micrologiciel-vide", mesure, Langue::Francais);

    assert_eq!(fiche.micrologiciel, None);
    assert_eq!(fiche.modele.as_deref(), Some("MYIRO-1"));
    let lue = fiche_du_gris("micrologiciel-lu", demande_myiro1(), Langue::Francais);
    assert_eq!(lue.micrologiciel.as_deref(), Some("1.00"));
}

/// Sans instrument, sans la condition d'impression choisie ou sans
/// bibliothèque, rien n'est mesuré, et l'écran dit pourquoi et quoi faire,
/// dans les deux langues.
#[test]
fn chaque_refus_avant_la_mesure_a_sa_cause_et_son_action() {
    use app::mesurer::{choisir_condition, RefusMesure};
    use app::textes::texte;
    let (_biblio, condition) = bibliotheque("refus-avant");

    assert_eq!(
        choisir_condition(Ok(vec![condition.clone()]), condition.id.0),
        Ok(condition.clone())
    );
    assert_eq!(
        choisir_condition(Ok(vec![condition.clone()]), condition.id.0 + 1),
        Err(RefusMesure::ConditionAbsente)
    );
    assert_eq!(
        choisir_condition(Err("bibliotheque.erreur.ouverture".into()), 1),
        Err(RefusMesure::BibliothequeFermee)
    );

    for refus in [
        RefusMesure::SansInstrument,
        RefusMesure::ConditionAbsente,
        RefusMesure::BibliothequeFermee,
    ] {
        assert!(refus.code().starts_with("mesurer.erreur."), "{refus:?}");
        for langue in [Langue::Francais, Langue::Anglais] {
            for partie in ["cause", "action"] {
                let cle = format!("{}.{partie}", refus.code());
                assert_ne!(texte(langue, &cle), cle, "texte manquant : {cle}");
            }
        }
    }
}

/// Une mesure que la bibliothèque a refusée peut être rangée à nouveau,
/// avec son nom, dans sa condition d'impression ; une mesure déjà rangée ne
/// l'est jamais deux fois.
#[test]
fn une_mesure_refusee_se_range_a_nouveau() {
    let mut instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "ranger-a-nouveau");
    let (biblio, condition) = bibliotheque("ranger-a-nouveau-biblio");
    let mut seance = Seance::default();
    seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            ranger(&biblio, &condition),
            Langue::Francais,
        )
        .expect("mesure rangée");
    let numero = seance
        .mesurer(
            &mut instrument,
            &mut fait,
            &condition,
            |_, _| Err("base verrouillée".into()),
            Langue::Francais,
        )
        .expect("mesure gardée");
    seance
        .renommer(numero, "Cyan", |_, _| panic!("pas encore rangée"))
        .unwrap();

    // Encore refusée : rien ne change.
    assert_eq!(seance.ranger_a_nouveau(|_, _, _| Err("toujours".into())), 1);
    assert_eq!(biblio.arborescence("").unwrap()[0].mesures.len(), 1);

    let mut appels = 0;
    let restantes = seance.ranger_a_nouveau(|cond, mesure, nom| {
        appels += 1;
        biblio
            .enregistrer_mesure_nommee(cond.id, mesure, nom)
            .map_err(|e| e.to_string())
    });

    assert_eq!(
        (restantes, appels),
        (0, 1),
        "la mesure déjà rangée n'est pas refaite"
    );
    assert!(seance
        .fiches(Langue::Francais)
        .iter()
        .all(|f| f.erreur_rangement.is_none()));
    assert_eq!(nom_range(&biblio).as_deref(), Some("Cyan"));
    assert_eq!(
        seance.ranger_a_nouveau(|_, _, _| panic!("rien à ranger")),
        0
    );
}
