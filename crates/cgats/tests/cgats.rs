//! Export et import CGATS.17, vus par l'API publique de la crate. Toutes les
//! mesures et tous les fichiers sont fictifs (n° 12345678), écrits ici.

use cgats::{ecrire, lire, MesureImportee};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Empreinte,
    Geometrie, Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Plage,
    Provenance, Spectre,
};

/// Plage fictive : chaque valeur est distincte, pour voir la moindre perte.
fn plage(graine: f32) -> Plage {
    let spectre = |decalage: f32| {
        Spectre::new(
            (0..36)
                .map(|i| graine + decalage + i as f32 * 0.013_7)
                .collect(),
        )
        .unwrap()
    };
    Plage::new(
        [spectre(0.0), spectre(0.001), spectre(-0.002)],
        DonneesBrutes::new((0..152).map(|i| graine * 1000.0 + i as f32 * 3.5).collect()).unwrap(),
        [
            Lab::new([52.31 + graine, 1.07, -2.389]).unwrap(),
            Lab::new([52.4, 1.9 + graine, -6.01]).unwrap(),
            Lab::new([52.0, 1.1, -1.8 - graine]).unwrap(),
        ],
    )
    .unwrap()
}

fn conditions_confirmees() -> ConditionsCalcul {
    ConditionsCalcul {
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
    }
}

fn provenance(geometrie: Geometrie, demande: Info<ConditionsCalcul>) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie: 12345678,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1D".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Confirmee(Empreinte::new("ab".repeat(32)).unwrap()),
        version_pont: "0.2.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new("2026-10-07T15:04:05+02:00").unwrap(),
        // Étalonnage et conditions observées : inconnus, ils doivent le rester.
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "spectres : Illuminant 0/1/2 = M0/M1/M2 ; Lab : D50, 2°".into(),
            demande,
            observe: Info::Inconnue,
        },
    }
}

fn bande() -> Mesure {
    Mesure::new(
        vec![plage(0.1), plage(0.4), plage(0.85)],
        provenance(
            Geometrie::Bande { sens: 2 },
            Info::Confirmee(conditions_confirmees()),
        ),
    )
    .unwrap()
}

/// Ce qu'une relecture doit rendre d'une mesure de myiro-libre.
fn verifier_sans_perte(mesure: &Mesure, relue: &MesureImportee) {
    assert_eq!(
        relue.provenance,
        Info::Confirmee(mesure.provenance().clone())
    );
    assert_eq!(relue.plages.len(), mesure.plages().len());
    for (n, (origine, lue)) in mesure.plages().iter().zip(&relue.plages).enumerate() {
        assert_eq!(lue.identifiant, (n + 1).to_string());
        let spectres = [origine.m0(), origine.m1(), origine.m2()];
        for (i, spectre) in spectres.into_iter().enumerate() {
            assert_eq!(lue.spectres[i], Info::Confirmee(spectre.clone()));
            assert_eq!(lue.lab[i], Info::Confirmee(origine.lab()[i]));
        }
    }
}

#[test]
fn une_bande_ecrite_puis_relue_garde_spectres_lab_et_provenance() {
    let mesure = bande();
    let texte = ecrire(&mesure).unwrap();
    let relue = lire(&texte).unwrap();
    verifier_sans_perte(&mesure, &relue);
}

/// Lignes de l'en-tête, sans les déclarations `KEYWORD`.
fn mots_cles(texte: &str) -> Vec<&str> {
    texte
        .lines()
        .take_while(|l| *l != "BEGIN_DATA_FORMAT")
        .filter(|l| !l.starts_with("KEYWORD"))
        .collect()
}

#[test]
fn l_en_tete_dit_la_provenance_et_les_conditions_confirmees_en_mots_cles_standard() {
    let texte = ecrire(&bande()).unwrap();
    let en_tete = mots_cles(&texte);
    for attendu in [
        "CGATS.17",
        "ORIGINATOR\t\"myiro-libre\"",
        "CREATED\t\"2026-10-07T15:04:05+02:00\"",
        "INSTRUMENTATION\t\"MYIRO-1\"",
        "SERIAL\t\"12345678\"",
        "MYIRO_LIBRE_FORMAT\t\"myiro-libre/cgats/1\"",
        "MYIRO_LIBRE_EMPREINTE_DLL\t\"confirmee:abababababababababababababababababababababababababababababababab\"",
        "MYIRO_LIBRE_ETALONNAGE\t\"inconnue\"",
        "MYIRO_LIBRE_GEOMETRIE\t\"bande 2\"",
        "MYIRO_LIBRE_CALCUL_DEMANDE_OBSERVATEUR\t\"supposee:2_degres\"",
        "MYIRO_LIBRE_CALCUL_OBSERVE\t\"inconnue\"",
        "MYIRO_LIBRE_SPECTRE\t\"m0\"",
        "MEASUREMENT_CONDITION\t\"M0\"",
        "MEASUREMENT_SOURCE\t\"A\"",
        "WEIGHTING_FUNCTION\t\"ILLUMINANT,D50\"",
    ] {
        assert!(
            en_tete.contains(&attendu),
            "manque {attendu:?} dans {en_tete:#?}"
        );
    }
    // L'observateur n'est que supposé : il n'entre pas dans le mot-clé
    // standard, que les autres logiciels liraient sans réserve.
    assert!(!texte.contains("OBSERVER"));
    // Chaque mot-clé hors de CGATS.17 est déclaré.
    assert!(texte.contains("KEYWORD\t\"MYIRO_LIBRE_ETALONNAGE\""));
    let format = texte
        .lines()
        .skip_while(|l| *l != "BEGIN_DATA_FORMAT")
        .nth(1)
        .unwrap();
    // Spectre à la manière d'ArgyllCMS : colonnes SPEC_380…, en pourcentage,
    // bandes décrites dans l'en-tête.
    assert!(format.starts_with("SAMPLE_ID\tLAB_L\tLAB_A\tLAB_B\tSPEC_380\tSPEC_390\t"));
    assert!(format.ends_with("\tSPEC_730"));
    for attendu in [
        "SPECTRAL_BANDS\t\"36\"",
        "SPECTRAL_START_NM\t\"380\"",
        "SPECTRAL_END_NM\t\"730\"",
    ] {
        assert!(
            en_tete.contains(&attendu),
            "manque {attendu:?} dans {en_tete:#?}"
        );
    }
    let premiere_ligne = texte
        .lines()
        .skip_while(|l| *l != "BEGIN_DATA")
        .nth(1)
        .unwrap();
    let valeur: f64 = premiere_ligne.split('\t').nth(4).unwrap().parse().unwrap();
    // 0,1 de réflectance (plage fictive 0.1, M0) s'écrit 10 %.
    assert!((valeur - 10.0).abs() < 1e-4, "{premiere_ligne}");
    // Un tableau par emplacement de spectre.
    assert_eq!(texte.lines().filter(|l| *l == "CGATS.17").count(), 3);
    assert!(texte.contains("MEASUREMENT_SOURCE\t\"UVCUT\""));
}

#[test]
fn ce_que_le_pont_n_a_pas_dit_reste_inconnu_dans_le_fichier_et_a_la_relecture() {
    // Comme une mesure du format initial : aucune condition de calcul connue.
    let mesure = Mesure::new(
        vec![plage(0.3)],
        provenance(Geometrie::Ponctuelle {}, Info::Inconnue),
    )
    .unwrap();
    let texte = ecrire(&mesure).unwrap();
    let en_tete = mots_cles(&texte);
    assert!(en_tete.contains(&"MYIRO_LIBRE_CALCUL_DEMANDE\t\"inconnue\""));
    for absent in [
        "MEASUREMENT_CONDITION",
        "MEASUREMENT_SOURCE",
        "WEIGHTING_FUNCTION",
        "SPEC_380",
        "SPECTRAL_BANDS",
    ] {
        assert!(!texte.contains(absent), "{absent} écrit sans être connu");
    }
    // Longueurs d'onde inconnues : les colonnes sont numérotées, pas nommées.
    assert!(texte.contains("\tMYIRO_LIBRE_SPECTRE_1\t"));

    let relue = lire(&texte).unwrap();
    verifier_sans_perte(&mesure, &relue);
    assert_eq!(
        relue.conditions,
        [Info::Inconnue, Info::Inconnue, Info::Inconnue]
    );
    assert_eq!(relue.longueurs_onde, Info::Inconnue);
}

#[test]
fn les_conditions_confirmees_se_relisent_confirmees() {
    let relue = lire(&ecrire(&bande()).unwrap()).unwrap();
    assert_eq!(
        relue.conditions,
        [
            Info::Confirmee(ConditionMesure::M0),
            Info::Confirmee(ConditionMesure::M1),
            Info::Confirmee(ConditionMesure::M2),
        ]
    );
    assert_eq!(
        relue.longueurs_onde,
        Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10
        })
    );
    assert_eq!(relue.instrument, Info::Confirmee("MYIRO-1".into()));
    assert_eq!(relue.numero_serie, Info::Confirmee("12345678".into()));
}

/// Fichier synthétique à la manière d'un export d'un logiciel du fabricant
/// pour le FD-9 : un tableau par condition, la condition donnée seulement par
/// la source lumineuse, densités et RVB en plus, fins de ligne Windows.
fn export_tiers(source: &str, avec_spectre: bool) -> String {
    let spectre_champs: String = (0..36).map(|i| format!("\tnm{}", 380 + 10 * i)).collect();
    let spectre = |graine: f32| -> String {
        (0..36)
            .map(|i| format!("\t{:.7}", graine + i as f32 * 0.01))
            .collect()
    };
    let (champs, n) = if avec_spectre {
        (spectre_champs.as_str(), 48)
    } else {
        ("", 12)
    };
    let ligne = |id: u32, loc: &str, lab: &str, graine: f32| {
        let sp = if avec_spectre {
            spectre(graine)
        } else {
            String::new()
        };
        format!("{id}\t{loc}\t10.00\t120.00\t200.00\t2.100\t0.700\t0.300\t1.000\t{lab}{sp}\r\n")
    };
    format!(
        "CGATS.17\r\nORIGINATOR\t\"Fabricant fictif\"\r\nFILE_DESCRIPTOR\t\"Logiciel fictif\"\r\n\
         CREATED\t\"\"\r\nINSTRUMENTATION\t\"Fabricant fictif FD-9\"\r\nSERIAL\t\"12345678\"\r\n\
         MEASUREMENT_GEOMETRY\t\"45/0\"\r\nMEASUREMENT_SOURCE\t\"{source}\"\r\n\
         WEIGHTING_FUNCTION\t\"OBSERVER,2 degree\"\r\nWEIGHTING_FUNCTION\t\"ILLUMINANT,D50\"\r\n\
         KEYWORD\t\"SAMPLE_LOC\" #Patch location in printing form\r\n\
         NUMBER_OF_FIELDS\t{n}\r\nBEGIN_DATA_FORMAT\r\n\
         SAMPLE_ID\tSAMPLE_LOC\tRGB_R\tRGB_G\tRGB_B\tD_RED\tD_GREEN\tD_BLUE\tD_VIS\tLAB_L\tLAB_A\tLAB_B{champs}\r\n\
         END_DATA_FORMAT\r\nNUMBER_OF_SETS\t2\r\nBEGIN_DATA\r\n{}{}END_DATA\r\n",
        // Valeurs inventées, sans rapport avec une mesure réelle.
        ligne(1, "A1", "41.50\t-27.25\t-58.75", 0.02),
        ligne(2, "A2", "82.25\t6.50\t101.75", 0.4),
    )
}

#[test]
fn un_export_tiers_se_lit_en_mesure_importee_sans_provenance_myiro_libre() {
    let importee = lire(&export_tiers("D50", true)).unwrap();
    assert_eq!(importee.provenance, Info::Inconnue);
    assert_eq!(
        importee.instrument,
        Info::Confirmee("Fabricant fictif FD-9".into())
    );
    assert_eq!(importee.numero_serie, Info::Confirmee("12345678".into()));
    // Date vide dans le fichier : inconnue, pas une chaîne vide.
    assert_eq!(importee.date, Info::Inconnue);
    // D50 désigne M1 par convention du fabricant : déduit, donc supposé.
    assert_eq!(
        importee.conditions,
        [
            Info::Inconnue,
            Info::Supposee(ConditionMesure::M1),
            Info::Inconnue
        ]
    );
    assert_eq!(
        importee.longueurs_onde,
        Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10
        })
    );
    let ids: Vec<_> = importee
        .plages
        .iter()
        .map(|p| p.identifiant.as_str())
        .collect();
    assert_eq!(ids, ["1", "2"]);
    let premiere = &importee.plages[0];
    assert_eq!(
        premiere.lab[1],
        Info::Confirmee(Lab::new([41.5, -27.25, -58.75]).unwrap())
    );
    let Info::Confirmee(spectre) = &premiere.spectres[1] else {
        panic!("le spectre M1 est dans le fichier");
    };
    assert_eq!(spectre.len(), 36);
    assert_eq!(spectre[0], 0.02);
    // Les deux autres conditions ne sont pas dans ce fichier.
    for e in [0, 2] {
        assert_eq!(premiere.spectres[e], Info::Inconnue);
        assert_eq!(premiere.lab[e], Info::Inconnue);
    }
}

#[test]
fn un_fichier_sans_spectre_laisse_les_spectres_inconnus() {
    let importee = lire(&export_tiers("A", false)).unwrap();
    assert_eq!(importee.conditions[0], Info::Supposee(ConditionMesure::M0));
    assert_eq!(importee.longueurs_onde, Info::Inconnue);
    for plage in &importee.plages {
        assert_eq!(
            plage.spectres,
            [Info::Inconnue, Info::Inconnue, Info::Inconnue]
        );
        assert!(matches!(plage.lab[0], Info::Confirmee(_)));
    }
}

#[test]
fn sans_condition_de_mesure_le_fichier_est_refuse_plutot_que_devine() {
    let erreur = lire(&export_tiers("D65", true)).unwrap_err();
    assert!(
        erreur.0.contains("condition de mesure introuvable"),
        "{erreur}"
    );
}

#[test]
fn un_fichier_mal_forme_est_refuse_en_clair() {
    let tronque = export_tiers("D50", true).replace("END_DATA\r\n", "");
    assert!(lire(&tronque).unwrap_err().0.contains("incomplet"));
    let mal_compte = export_tiers("D50", true).replace("NUMBER_OF_SETS\t2", "NUMBER_OF_SETS\t3");
    assert!(lire(&mal_compte).unwrap_err().0.contains("annoncées"));
    let illisible = export_tiers("D50", true).replace("41.50", "4x.50");
    assert!(lire(&illisible).unwrap_err().0.contains("« 4x.50 »"));
    assert!(lire("").is_err());
}

/// Fichier fictif d'une plage : `champs` nomme les colonnes de spectre,
/// `valeurs` les remplit.
fn fichier_spectre(champs: &str, valeurs: &str) -> String {
    format!(
        "CGATS.17\nMEASUREMENT_SOURCE\t\"D50\"\nBEGIN_DATA_FORMAT\nSAMPLE_ID\t{champs}\n\
         END_DATA_FORMAT\nNUMBER_OF_SETS\t1\nBEGIN_DATA\n1\t{valeurs}\nEND_DATA\n"
    )
}

#[test]
fn des_longueurs_d_onde_demesurees_sont_refusees_sans_planter() {
    let texte = fichier_spectre("nm0\tnm3000000000\tnm4000000000", "0.1\t0.2\t0.3");
    assert!(lire(&texte).unwrap_err().0.contains("longueurs d'onde"));
    let texte = fichier_spectre("SPEC_0\tSPEC_4294967295", "10\t20");
    assert!(lire(&texte).is_ok());
}

#[test]
fn un_spectre_en_pourcentage_ou_de_0_a_1_se_relit_en_reflectance() {
    // ArgyllCMS : SPEC_xxx en pourcentage.
    let argyll = lire(&fichier_spectre("SPEC_400\tSPEC_410", "50\t87.5")).unwrap();
    // Fabricant : nmxxx de 0 à 1.
    let fabricant = lire(&fichier_spectre("nm400\tnm410", "0.5\t0.875")).unwrap();
    for importee in [argyll, fabricant] {
        assert_eq!(
            importee.plages[0].spectres[1],
            Info::Confirmee(Spectre::new(vec![0.5, 0.875]).unwrap())
        );
        assert_eq!(
            importee.longueurs_onde,
            Info::Confirmee(Echantillonnage {
                debut_nm: 400,
                pas_nm: 10
            })
        );
    }
    // Deux échelles dans le même tableau : refusé plutôt que deviné.
    assert!(lire(&fichier_spectre("SPEC_400\tnm410", "50\t0.5")).is_err());
}

#[test]
fn un_fichier_en_latin_1_garde_ses_accents() {
    let mut octets = fichier_spectre("nm400", "0.5").into_bytes();
    let fin = octets.len();
    octets.splice(fin..fin, b"# Relev\xe9 \x80\n".iter().copied());
    let texte = cgats::decoder(&octets);
    assert!(texte.ends_with("# Relevé €\n"), "{texte:?}");
    assert!(lire(&texte).is_ok());
    // Un fichier UTF-8 reste tel quel.
    assert_eq!(cgats::decoder("Relevé".as_bytes()), "Relevé");
}

#[test]
fn un_fichier_myiro_libre_d_une_autre_version_est_refuse() {
    let texte = ecrire(&bande())
        .unwrap()
        .replace("myiro-libre/cgats/1", "myiro-libre/cgats/2");
    let erreur = lire(&texte).unwrap_err();
    assert!(erreur.0.contains("myiro-libre/cgats/2"), "{erreur}");
    // Une provenance amputée ne se relit pas à moitié.
    let ampute = ecrire(&bande())
        .unwrap()
        .replace("MYIRO_LIBRE_ARCHITECTURE\t\"x86\"\r\n", "");
    assert!(lire(&ampute)
        .unwrap_err()
        .0
        .contains("MYIRO_LIBRE_ARCHITECTURE"));
}

/// Export réel d'un logiciel officiel, gardé sur le poste et jamais versionné :
/// `CGATS_REEL=<fichier> cargo test -p cgats -- --ignored`.
#[test]
#[ignore = "lit un fichier local désigné par CGATS_REEL"]
fn un_export_reel_se_lit_en_mesure_importee() {
    let chemin = std::env::var("CGATS_REEL").expect("CGATS_REEL désigne le fichier");
    let octets = std::fs::read(&chemin).unwrap();
    let importee = lire(&String::from_utf8_lossy(&octets)).unwrap();
    assert_eq!(importee.provenance, Info::Inconnue);
    assert!(!importee.plages.is_empty());
    let connues: Vec<_> = importee
        .conditions
        .iter()
        .filter(|c| **c != Info::Inconnue)
        .collect();
    assert_eq!(
        connues.len(),
        1,
        "un export du fabricant porte une condition"
    );
    println!(
        "{} plages, conditions {:?}, longueurs d'onde {:?}, instrument {:?}",
        importee.plages.len(),
        importee.conditions,
        importee.longueurs_onde,
        importee.instrument
    );
}

#[test]
fn un_guillemet_se_relit_et_un_retour_a_la_ligne_est_refuse() {
    let mut p = bande().provenance().clone();
    p.calcul.libelle = "calcul « fin » et \"brut\" ; # pas un commentaire".into();
    let mesure = Mesure::new(bande().plages().to_vec(), p.clone()).unwrap();
    verifier_sans_perte(&mesure, &lire(&ecrire(&mesure).unwrap()).unwrap());

    p.calcul.libelle = "deux\nlignes".into();
    let mesure = Mesure::new(bande().plages().to_vec(), p).unwrap();
    assert!(ecrire(&mesure).is_err());
}
