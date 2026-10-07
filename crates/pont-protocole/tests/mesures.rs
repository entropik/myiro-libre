//! Le contrat des mesures : types validés à la construction comme à la lecture,
//! format versionné, lecture du format initial. Valeurs fictives (n° 12345678).

use pont_protocole::{DonneesBrutes, Lab, Plage, Spectre};

fn plage_myiro1() -> Plage {
    Plage::new(
        [
            Spectre::new(vec![0.3; 36]).unwrap(),
            Spectre::new(vec![0.4; 36]).unwrap(),
            Spectre::new(vec![0.5; 36]).unwrap(),
        ],
        DonneesBrutes::new(vec![1000.0; 152]).unwrap(),
        [
            Lab::new([50.0, 1.0, -2.0]).unwrap(),
            Lab::new([51.0, 1.5, -2.5]).unwrap(),
            Lab::new([52.0, 2.0, -3.0]).unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn un_spectre_refuse_les_nombres_non_finis() {
    assert!(Spectre::new(vec![0.5, f32::NAN]).is_err());
    assert!(Spectre::new(vec![f32::INFINITY]).is_err());
    assert!(Spectre::new(vec![]).is_err());
}

#[test]
fn une_reflectance_superieure_a_1_est_acceptee() {
    let spectre = Spectre::new(vec![1.07; 36]).unwrap();
    assert_eq!(spectre[0], 1.07);
}

#[test]
fn un_lab_refuse_un_nombre_non_fini() {
    assert!(Lab::new([50.0, f32::NAN, 0.0]).is_err());
    assert_eq!(
        Lab::new([50.0, 1.0, -2.0]).unwrap().valeurs(),
        [50.0, 1.0, -2.0]
    );
}

#[test]
fn les_trois_spectres_d_une_plage_ont_la_meme_longueur() {
    let erreur = Plage::new(
        [
            Spectre::new(vec![0.5; 36]).unwrap(),
            Spectre::new(vec![0.5; 35]).unwrap(),
            Spectre::new(vec![0.5; 36]).unwrap(),
        ],
        DonneesBrutes::new(vec![1.0; 152]).unwrap(),
        [Lab::new([50.0, 0.0, 0.0]).unwrap(); 3],
    )
    .unwrap_err();
    assert!(erreur.to_string().contains("35"), "{erreur}");
}

#[test]
fn une_plage_se_relit_a_l_identique() {
    let plage = plage_myiro1();
    let texte = serde_json::to_string(&plage).unwrap();
    assert_eq!(serde_json::from_str::<Plage>(&texte).unwrap(), plage);
    assert_eq!(plage.m1()[0], 0.4);
    assert_eq!(plage.lab()[2].valeurs(), [52.0, 2.0, -3.0]);
}

#[test]
fn une_plage_relue_est_validee_comme_a_la_construction() {
    let texte = serde_json::to_string(&plage_myiro1()).unwrap();
    // Un Lab à deux valeurs, un spectre vide, un nombre en texte.
    for abime in [
        texte.replacen("[50.0,1.0,-2.0]", "[50.0,1.0]", 1),
        texte.replacen("\"m2\":[", "\"m2\":[],\"x\":[", 1),
        texte.replacen("1000.0", "\"NaN\"", 1),
    ] {
        assert_ne!(abime, texte);
        assert!(serde_json::from_str::<Plage>(&abime).is_err(), "{abime}");
    }
}

// --- Mesure complète et format conservable ---

use pont_protocole::{
    ecrire_mesure, ecrire_reponse, lire_mesure, lire_reponse, Calcul, ConditionMesure,
    ConditionsCalcul, Echantillonnage, Empreinte, Geometrie, Horodatage, Illuminant, Info,
    InstrumentMesurant, Mesure, Observateur, Provenance, RemiseAuRepos, Reponse, FORMAT_MESURE,
};

fn provenance(modele: &str, geometrie: Geometrie) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: modele.into(),
            numero_serie: 12345678,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1D".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Confirmee(Empreinte::new("ab".repeat(32)).unwrap()),
        version_pont: "0.1.0".into(),
        architecture: "x86_64".into(),
        horodatage: Horodatage::new("2026-10-07T15:04:05+02:00").unwrap(),
        etalonnage: Info::Confirmee(Horodatage::new("2026-10-07T15:00:00+02:00").unwrap()),
        geometrie,
        calcul: Calcul {
            libelle: "M0/M1/M2 ; Lab D50, 2°".into(),
            demande: Info::Confirmee(ConditionsCalcul {
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
            }),
            observe: Info::Inconnue,
        },
    }
}

fn mesure_ponctuelle() -> Mesure {
    Mesure::new(
        vec![plage_myiro1()],
        provenance("MYIRO-1", Geometrie::Ponctuelle {}),
    )
    .unwrap()
}

#[test]
fn une_mesure_enregistree_se_relit_sans_perte() {
    let mesure = mesure_ponctuelle();
    let texte = ecrire_mesure(&mesure);
    assert!(
        texte.starts_with(r#"{"format":"myiro-libre/mesure/1","#),
        "{texte}"
    );
    assert_eq!(FORMAT_MESURE, "myiro-libre/mesure/1");
    assert_eq!(lire_mesure(&texte), Ok(mesure));
}

#[test]
fn une_mesure_traverse_le_protocole_sans_perte() {
    let reponse = Reponse::Mesure {
        mesure: mesure_ponctuelle(),
        remise_au_repos: Info::Confirmee(RemiseAuRepos::ArretRefuse { code: -9987 }),
    };
    let texte = ecrire_reponse(&reponse);
    assert!(texte.starts_with(r#"{"rep":"mesure","mesure":{"format":"myiro-libre/mesure/1","#));
    assert!(texte.ends_with(
        r#","remise_au_repos":{"statut":"confirmee","valeur":{"etat":"arret_refuse","code":-9987}}}"#
    ));
    assert_eq!(lire_reponse(&texte), Ok(reponse));
}

#[test]
fn une_reponse_mesure_sans_retour_au_repos_se_relit_avec_un_repos_inconnu() {
    // Réponse écrite par un pont antérieur au ticket #24.
    let texte = ecrire_reponse(&Reponse::Mesure {
        mesure: mesure_ponctuelle(),
        remise_au_repos: Info::Inconnue,
    });
    let ancienne = texte.replace(r#","remise_au_repos":{"statut":"inconnue"}"#, "");
    assert_ne!(ancienne, texte);
    let Ok(Reponse::Mesure {
        remise_au_repos, ..
    }) = lire_reponse(&ancienne)
    else {
        panic!("{ancienne}")
    };
    assert_eq!(remise_au_repos, Info::Inconnue, "jamais « au repos »");
}

#[test]
fn un_retour_au_repos_nu_ou_avec_un_champ_inconnu_est_refuse() {
    let texte = ecrire_reponse(&Reponse::Mesure {
        mesure: mesure_ponctuelle(),
        remise_au_repos: Info::Confirmee(RemiseAuRepos::AuRepos {}),
    });
    let qualifie = r#""remise_au_repos":{"statut":"confirmee","valeur":{"etat":"au_repos"}}"#;
    assert!(texte.contains(qualifie), "{texte}");
    for abime in [
        r#""remise_au_repos":{"etat":"au_repos"}"#,
        r#""remise_au_repos":{"statut":"confirmee","valeur":{"etat":"au_repos","x":1}}"#,
        r#""remise_au_repos":{"statut":"confirmee","valeur":{"etat":"repos_suppose"}}"#,
    ] {
        let ligne = texte.replace(qualifie, abime);
        assert!(lire_reponse(&ligne).is_err(), "{ligne}");
    }
}

#[test]
fn une_reponse_du_protocole_avec_un_champ_inconnu_est_refusee() {
    let texte = ecrire_reponse(&Reponse::Mesure {
        mesure: mesure_ponctuelle(),
        remise_au_repos: Info::Inconnue,
    });
    let abime = texte.replacen(r#"{"rep":"mesure","#, r#"{"rep":"mesure","note":"x","#, 1);
    assert_ne!(abime, texte);
    assert!(lire_reponse(&abime).is_err(), "{abime}");
    for ligne in [
        r#"{"rep":"etalonne","force":true}"#,
        r#"{"rep":"ferme","detail":"x"}"#,
        r#"{"rep":"version","parties":[1,0,1],"protocole":2}"#,
        r#"{"rep":"instruments","liste":[{"liaison":"usb","port":"COM3","numero_serie":12345678,"x":1}]}"#,
        r#"{"rep":"erreur","erreur":{"type":"mesure_echouee","erreur":-1,"x":1}}"#,
        r#"{"rep":"erreur","erreur":{"type":"delai","x":1}}"#,
        r#"{"rep":"erreur","erreur":{"type":"instrument_perdu","x":1}}"#,
        r#"{"rep":"connecte","identite":{"numero_serie":12345678,"micrologiciel":"1.00","code_produit":"9C1D","adresse_mac":"02:00:00:00:00:01","date_initiale":null,"anomalie_date_initiale":false,"brute_hex":"00","x":1}}"#,
    ] {
        assert!(lire_reponse(ligne).is_err(), "{ligne}");
    }
}

#[test]
fn une_version_non_prise_en_charge_est_refusee_en_clair() {
    let texte = ecrire_mesure(&mesure_ponctuelle()).replace("mesure/1", "mesure/2");
    let erreur = lire_mesure(&texte).unwrap_err().to_string();
    assert!(erreur.contains("myiro-libre/mesure/2"), "{erreur}");
    assert!(erreur.contains("non pris en charge"), "{erreur}");
    // Même refus quand la mesure arrive par le protocole.
    let ligne = format!(r#"{{"rep":"mesure","mesure":{texte}}}"#);
    let erreur = lire_reponse(&ligne).unwrap_err();
    assert!(erreur.contains("non pris en charge"), "{erreur}");
}

#[test]
fn une_mesure_sans_format_est_refusee() {
    let texte =
        ecrire_mesure(&mesure_ponctuelle()).replace(r#""format":"myiro-libre/mesure/1","#, "");
    assert!(lire_mesure(&texte).is_err());
}

#[test]
fn un_champ_inconnu_est_refuse() {
    let texte = ecrire_mesure(&mesure_ponctuelle());
    for abime in [
        texte.replacen(r#""plages":"#, r#""note":"x","plages":"#, 1),
        texte.replacen(r#""version_pont":"#, r#""lot":"A","version_pont":"#, 1),
    ] {
        assert_ne!(abime, texte);
        assert!(lire_mesure(&abime).is_err(), "{abime}");
    }
}

fn plage_de(longueur_spectre: usize, nombre_brutes: usize) -> Plage {
    Plage::new(
        [
            Spectre::new(vec![0.5; longueur_spectre]).unwrap(),
            Spectre::new(vec![0.5; longueur_spectre]).unwrap(),
            Spectre::new(vec![0.5; longueur_spectre]).unwrap(),
        ],
        DonneesBrutes::new(vec![1.0; nombre_brutes]).unwrap(),
        [Lab::new([50.0, 0.0, 0.0]).unwrap(); 3],
    )
    .unwrap()
}

#[test]
fn le_myiro1_exige_36_valeurs_par_spectre_et_152_donnees_brutes() {
    let ponctuelle = || provenance("MYIRO-1", Geometrie::Ponctuelle {});
    let erreur = Mesure::new(vec![plage_de(35, 152)], ponctuelle()).unwrap_err();
    assert!(erreur.to_string().contains("36"), "{erreur}");
    let erreur = Mesure::new(vec![plage_de(36, 151)], ponctuelle()).unwrap_err();
    assert!(erreur.to_string().contains("152"), "{erreur}");
    assert!(Mesure::new(vec![plage_de(36, 152)], ponctuelle()).is_ok());
    // Un autre instrument n'est pas tenu aux dimensions du MYIRO-1.
    assert!(Mesure::new(
        vec![plage_de(31, 0)],
        provenance("FD-9", Geometrie::Feuille {})
    )
    .is_ok());
}

#[test]
fn les_plages_d_une_mesure_ont_les_memes_dimensions() {
    assert!(Mesure::new(
        vec![plage_de(31, 0), plage_de(32, 0)],
        provenance("FD-9", Geometrie::Feuille {})
    )
    .is_err());
}

#[test]
fn une_mesure_relue_avec_une_longueur_erronee_est_refusee() {
    let texte = ecrire_mesure(&mesure_ponctuelle());
    let abime = texte.replacen("[0.3,", "[", 1);
    assert_ne!(abime, texte);
    assert!(lire_mesure(&abime).is_err());
}

#[test]
fn une_mesure_relue_avec_un_nombre_non_fini_est_refusee() {
    let texte = ecrire_mesure(&mesure_ponctuelle());
    // JSON n'écrit pas NaN : un fichier abîmé y met null ou un texte.
    for remplacant in ["[null,", r#"["NaN","#, "[1e999,"] {
        let abime = texte.replacen("[0.3,", remplacant, 1);
        assert_ne!(abime, texte);
        assert!(lire_mesure(&abime).is_err(), "{remplacant}");
    }
}

#[test]
fn une_mesure_a_au_moins_une_plage_et_une_seule_en_ponctuelle() {
    assert!(Mesure::new(vec![], provenance("MYIRO-1", Geometrie::Bande { sens: 0 })).is_err());
    assert!(Mesure::new(
        vec![plage_myiro1(), plage_myiro1()],
        provenance("MYIRO-1", Geometrie::Ponctuelle {})
    )
    .is_err());
    let bande = Mesure::new(
        vec![plage_myiro1(), plage_myiro1()],
        provenance("MYIRO-1", Geometrie::Bande { sens: 2 }),
    )
    .unwrap();
    assert_eq!(bande.plages().len(), 2);
    assert_eq!(bande.provenance().geometrie, Geometrie::Bande { sens: 2 });
}

#[test]
fn une_provenance_invalide_est_refusee() {
    assert!(
        Horodatage::new("2026-10-07T15:04:05").is_err(),
        "sans fuseau"
    );
    assert!(Horodatage::new("07/10/2026 15:04").is_err());
    assert!(Horodatage::new("2026-13-07T15:04:05+02:00").is_err());
    assert!(
        Horodatage::new("2026-02-31T15:04:05+02:00").is_err(),
        "31 février"
    );
    assert!(
        Horodatage::new("2026-04-31T15:04:05+02:00").is_err(),
        "31 avril"
    );
    assert!(
        Horodatage::new("2026-02-29T15:04:05+02:00").is_err(),
        "2026 non bissextile"
    );
    assert!(
        Horodatage::new("2028-02-29T15:04:05+02:00").is_ok(),
        "2028 bissextile"
    );
    assert!(
        Horodatage::new("2100-02-29T15:04:05+02:00").is_err(),
        "2100 non bissextile"
    );
    assert!(
        Horodatage::new("2000-02-29T15:04:05+02:00").is_ok(),
        "2000 bissextile"
    );
    assert!(Horodatage::new("2026-10-07T15:04:05Z").is_ok());
    assert!(Empreinte::new("empreinte").is_err());
    assert!(
        Empreinte::new("AB".repeat(32)).is_err(),
        "minuscules seulement"
    );
    let mut p = provenance("", Geometrie::Ponctuelle {});
    assert!(Mesure::new(vec![plage_myiro1()], p.clone()).is_err());
    p.instrument.modele = "MYIRO-1".into();
    p.version_pont = String::new();
    assert!(Mesure::new(vec![plage_myiro1()], p).is_err());

    let texte = ecrire_mesure(&mesure_ponctuelle());
    let abime = texte.replace("2026-10-07T15:04:05+02:00", "hier");
    assert_ne!(abime, texte);
    assert!(lire_mesure(&abime).is_err());
}

#[test]
fn une_information_inconnue_reste_inconnue() {
    let mut p = provenance("MYIRO-1", Geometrie::Ponctuelle {});
    p.empreinte_dll = Info::Inconnue;
    let mesure = Mesure::new(vec![plage_myiro1()], p).unwrap();
    let texte = ecrire_mesure(&mesure);
    assert!(
        texte.contains(r#""empreinte_dll":{"statut":"inconnue"}"#),
        "{texte}"
    );
    assert!(
        texte.contains(r#""observe":{"statut":"inconnue"}"#),
        "{texte}"
    );
    assert_eq!(lire_mesure(&texte), Ok(mesure));
}

#[test]
fn une_donnee_qualifiee_ne_se_remplace_pas_par_une_valeur_nue() {
    let texte = ecrire_mesure(&mesure_ponctuelle());
    let empreinte = "ab".repeat(32);
    let abime = texte.replacen(
        &format!(r#"{{"statut":"confirmee","valeur":"{empreinte}"}}"#),
        &format!(r#""{empreinte}""#),
        1,
    );
    assert_ne!(abime, texte);
    assert!(lire_mesure(&abime).is_err());
    let inventee = texte.replacen(
        r#"{"statut":"inconnue"}"#,
        r#"{"statut":"inconnue","valeur":0}"#,
        1,
    );
    assert_ne!(inventee, texte);
    assert!(lire_mesure(&inventee).is_err());
}

// --- Format initial : réponses `mesure` du pont 0.1.0, sans numéro de format ---

/// Une réponse `mesure` telle que l'écrivait le pont 0.1.0.
fn ligne_initiale(geometrie: &str, sens: u32, etalonnage: Option<&str>) -> serde_json::Value {
    let plage = serde_json::json!({
        "m0": vec![0.25; 36], "m1": vec![1.5; 36], "m2": vec![0.75; 36],
        "brutes": vec![1000.5; 152],
        "lab_m0": [50.0, 1.0, -2.0], "lab_m1": [51.0, 1.5, -2.5], "lab_m2": [52.0, 2.0, -3.0],
    });
    serde_json::json!({
        "rep": "mesure",
        "plages": [plage],
        "sens": sens,
        "provenance": {
            "instrument": {
                "modele": "MYIRO-1", "numero_serie": 12345678,
                "micrologiciel": "1.00", "code_produit": "FDX-0000"
            },
            "version_sdk": [1, 0, 1],
            "empreinte_dll": "0".repeat(64),
            "version_pont": "0.1.0",
            "architecture": "x86",
            "horodatage": "2026-01-01T10:00:00+01:00",
            "etalonnage": etalonnage,
            "geometrie": geometrie,
            "calcul": "D50, 2 degres"
        }
    })
}

#[test]
fn le_format_initial_se_relit_avec_ses_seuls_faits_demontrables() {
    let ligne = ligne_initiale("ponctuelle", 0, Some("2026-01-01T09:55:00+01:00")).to_string();
    let mesure = lire_mesure(&ligne).unwrap();
    let plage = &mesure.plages()[0];
    assert_eq!(plage.m0()[0], 0.25);
    assert_eq!(plage.m1()[35], 1.5, "réflectance > 1 conservée");
    assert_eq!(plage.brutes().len(), 152);
    assert_eq!(plage.lab()[1].valeurs(), [51.0, 1.5, -2.5]);
    let p = mesure.provenance();
    assert_eq!(p.instrument.numero_serie, 12345678);
    assert_eq!(p.version_sdk, [1, 0, 1]);
    assert_eq!(p.horodatage.texte(), "2026-01-01T10:00:00+01:00");
    assert_eq!(
        p.etalonnage,
        Info::Confirmee(Horodatage::new("2026-01-01T09:55:00+01:00").unwrap())
    );
    assert_eq!(
        p.empreinte_dll,
        Info::Confirmee(Empreinte::new("0".repeat(64)).unwrap())
    );
    assert_eq!(p.geometrie, Geometrie::Ponctuelle {});
    // Le texte libre du calcul est gardé, mais rien n'en est déduit.
    assert_eq!(p.calcul.libelle, "D50, 2 degres");
    assert_eq!(p.calcul.demande, Info::Inconnue);
    assert_eq!(p.calcul.observe, Info::Inconnue);
}

#[test]
fn un_etalonnage_absent_du_format_initial_reste_inconnu() {
    let ligne = ligne_initiale("bande", 2, None).to_string();
    let mesure = lire_mesure(&ligne).unwrap();
    assert_eq!(mesure.provenance().etalonnage, Info::Inconnue);
    assert_eq!(mesure.provenance().geometrie, Geometrie::Bande { sens: 2 });
}

#[test]
fn le_format_initial_se_lit_aussi_comme_reponse_du_protocole() {
    let ligne = ligne_initiale("ponctuelle", 0, None).to_string();
    let Ok(Reponse::Mesure {
        mesure,
        remise_au_repos,
    }) = lire_reponse(&ligne)
    else {
        panic!("{:?}", lire_reponse(&ligne))
    };
    assert_eq!(Ok(mesure), lire_mesure(&ligne));
    assert_eq!(
        remise_au_repos,
        Info::Inconnue,
        "le pont 0.1.0 ne le disait pas"
    );
}

#[test]
fn une_mesure_reprise_du_format_initial_se_reecrit_dans_le_format_courant() {
    let ligne = ligne_initiale("bande", 1, None).to_string();
    let mesure = lire_mesure(&ligne).unwrap();
    let texte = ecrire_mesure(&mesure);
    assert!(texte.contains(FORMAT_MESURE));
    assert_eq!(lire_mesure(&texte), Ok(mesure));
}

#[test]
fn un_format_initial_irrecuperable_est_refuse_avec_une_explication() {
    let geometrie_inconnue = ligne_initiale("diagonale", 0, None).to_string();
    let erreur = lire_mesure(&geometrie_inconnue).unwrap_err().to_string();
    assert!(erreur.contains("diagonale"), "{erreur}");

    let mut sans_provenance = ligne_initiale("ponctuelle", 0, None);
    sans_provenance
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    assert!(lire_mesure(&sans_provenance.to_string()).is_err());

    let mut champ_en_trop = ligne_initiale("ponctuelle", 0, None);
    champ_en_trop["provenance"]["lot"] = "A".into();
    assert!(lire_mesure(&champ_en_trop.to_string()).is_err());

    let mut spectre_court = ligne_initiale("ponctuelle", 0, None);
    spectre_court["plages"][0]["m1"] = serde_json::json!(vec![0.5; 35]);
    assert!(lire_mesure(&spectre_court.to_string()).is_err());

    let mut horodatage_abime = ligne_initiale("ponctuelle", 0, None);
    horodatage_abime["provenance"]["horodatage"] = "hier".into();
    assert!(lire_mesure(&horodatage_abime.to_string()).is_err());
}
