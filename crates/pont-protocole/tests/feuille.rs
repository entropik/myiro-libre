//! Mesure d'une feuille au FD-9 : définition de la mire, paliers et requêtes.
//! Nombres de la RGB-2520 tirés de l'étude statique (plan 6) ; valeurs
//! d'instrument fictives.

use pont_protocole::{lire_requete, DefinitionFeuille, Palier, Requete, ZoneFeuille};

/// La RGB-2520 : une zone de 60 rangées × 42 colonnes, plage 601 × 601 au
/// pas de 601, origine (1 349 ; 2 300), sur une feuille de 27 940 × 41 660
/// (1/100 mm).
fn rgb2520() -> DefinitionFeuille {
    DefinitionFeuille {
        nom: "RGB-2520 - 1P MY9".into(),
        largeur: 27_940,
        hauteur: 41_660,
        zones: vec![ZoneFeuille {
            gauche: 1_349,
            haut: 2_300,
            largeur_plage: 601,
            hauteur_plage: 601,
            pas_x: 601,
            pas_y: 601,
            rangees: 60,
            colonnes: 42,
        }],
    }
}

#[test]
fn prenumerisation_et_feuille_viennent_apres_bande() {
    assert!(Palier::Prenumerisation > Palier::Bande);
    assert!(Palier::Feuille > Palier::Prenumerisation);
    assert_eq!(
        serde_json::to_string(&Palier::Prenumerisation).unwrap(),
        r#""prenumerisation""#
    );
    assert_eq!(
        serde_json::to_string(&Palier::Feuille).unwrap(),
        r#""feuille""#
    );
}

#[test]
fn une_requete_mesurer_feuille_se_relit() {
    for requete in [
        Requete::MesurerFeuille { mire: rgb2520() },
        Requete::Prenumeriser { mire: rgb2520() },
    ] {
        let texte = serde_json::to_string(&requete).unwrap();
        assert_eq!(lire_requete(&texte), Ok(requete));
    }
    let texte = serde_json::to_string(&Requete::MesurerFeuille { mire: rgb2520() }).unwrap();
    assert!(
        texte.starts_with(r#"{"cmd":"mesurer_feuille","mire":{"nom":"RGB-2520 - 1P MY9","#),
        "{texte}"
    );
}

#[test]
fn une_definition_sans_zone_est_refusee() {
    let mut mire = rgb2520();
    mire.zones.clear();
    assert!(mire.verifier().is_err());
    // Même refus quand la définition arrive par le protocole.
    let texte = serde_json::to_string(&Requete::Prenumeriser { mire }).unwrap();
    assert!(lire_requete(&texte).is_err(), "{texte}");
}

#[test]
fn une_zone_qui_deborde_de_la_feuille_est_refusee() {
    // 1 349 + 601 × 42 = 26 591 : une feuille de 26 590 est trop étroite.
    let mut mire = rgb2520();
    mire.largeur = 26_590;
    let erreur = mire.verifier().unwrap_err().to_string();
    assert!(erreur.contains("zone 1"), "{erreur}");
    mire.largeur = 26_591;
    assert!(mire.verifier().is_ok());
    // 2 300 + 601 × 60 = 38 360.
    let mut mire = rgb2520();
    mire.hauteur = 38_359;
    assert!(mire.verifier().is_err());
}

#[test]
fn une_zone_vide_ou_un_pas_nul_est_refuse() {
    let abimes: [fn(&mut ZoneFeuille); 6] = [
        |z| z.rangees = 0,
        |z| z.colonnes = 0,
        |z| z.pas_x = 0,
        |z| z.pas_y = 0,
        |z| z.largeur_plage = 0,
        |z| z.hauteur_plage = 0,
    ];
    for abimer in abimes {
        let mut mire = rgb2520();
        abimer(&mut mire.zones[0]);
        assert!(mire.verifier().is_err(), "{:?}", mire.zones[0]);
    }
}

#[test]
fn une_definition_sans_nom_ou_avec_trop_de_zones_est_refusee() {
    let mut mire = rgb2520();
    mire.nom = "  ".into();
    assert!(mire.verifier().is_err());
    // Le SDK admet 50 zones au plus (fiche FD9_StartPatchRecognition).
    let mut mire = rgb2520();
    let zone = ZoneFeuille {
        rangees: 1,
        colonnes: 1,
        ..mire.zones[0].clone()
    };
    mire.zones = vec![zone; 50];
    assert!(mire.verifier().is_ok());
    mire.zones.push(mire.zones[0].clone());
    assert!(mire.verifier().is_err());
}

#[test]
fn des_nombres_geants_ne_debordent_pas_le_calcul() {
    let mut mire = rgb2520();
    mire.zones[0].pas_x = u32::MAX;
    mire.zones[0].gauche = u32::MAX;
    assert!(mire.verifier().is_err());
}

#[test]
fn la_definition_rgb2520_compte_2520_plages() {
    let mire = rgb2520();
    assert_eq!(mire.verifier(), Ok(()));
    assert_eq!(mire.plages(), 2_520);
}

#[test]
fn un_champ_inconnu_dans_la_definition_est_refuse() {
    let texte = serde_json::to_string(&Requete::MesurerFeuille { mire: rgb2520() }).unwrap();
    for abime in [
        texte.replacen(r#""nom":"#, r#""couleurs":[1],"nom":"#, 1),
        texte.replacen(r#""gauche":"#, r#""angle":0,"gauche":"#, 1),
    ] {
        assert_ne!(abime, texte);
        assert!(lire_requete(&abime).is_err(), "{abime}");
    }
}

// --- Mesure en feuille : la lecture de la feuille dans la provenance ---

use pont_protocole::{
    ecrire_mesure, ecrire_reponse, lire_mesure, lire_reponse, Calcul, DonneesBrutes, ErreurPont,
    Geometrie, Horodatage, Info, InstrumentMesurant, Lab, LectureFeuille, Mesure, Plage,
    Provenance, Reponse, Spectre, MODELE_FD9,
};

fn lecture() -> LectureFeuille {
    LectureFeuille {
        qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
        zones: 1,
        plages: 2_520,
        fiabilite: 97,
        ecart_max: 12,
    }
}

fn provenance(geometrie: Geometrie, feuille: Option<LectureFeuille>) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: MODELE_FD9.into(),
            numero_serie: 12345678,
            micrologiciel: "1.02.0005".into(),
            code_produit: "9C1A".into(),
        },
        version_sdk: [1, 0x20, 3],
        empreinte_dll: Info::Inconnue,
        version_pont: "0.1.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new("2026-10-28T15:04:05+01:00").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "M0/M1/M2 ; Lab D50, 2°".into(),
            demande: Info::Inconnue,
            observe: Info::Inconnue,
        },
        feuille,
    }
}

fn plage() -> Plage {
    Plage::new(
        [
            Spectre::new(vec![0.3; 36]).unwrap(),
            Spectre::new(vec![0.4; 36]).unwrap(),
            Spectre::new(vec![0.5; 36]).unwrap(),
        ],
        DonneesBrutes::new(vec![1000.0; 152]).unwrap(),
        [Lab::new([50.0, 1.0, -2.0]).unwrap(); 3],
    )
    .unwrap()
}

#[test]
fn le_modele_du_fd9_s_ecrit_fd_9() {
    assert_eq!(MODELE_FD9, "FD-9");
}

#[test]
fn une_mesure_en_feuille_sans_lecture_est_refusee() {
    let erreur = Mesure::new(vec![plage()], provenance(Geometrie::Feuille {}, None)).unwrap_err();
    assert!(erreur.to_string().contains("feuille"), "{erreur}");
    assert!(Mesure::new(
        vec![plage()],
        provenance(Geometrie::Feuille {}, Some(lecture()))
    )
    .is_ok());
}

#[test]
fn une_lecture_de_feuille_sur_une_autre_geometrie_est_refusee() {
    for geometrie in [Geometrie::Ponctuelle {}, Geometrie::Bande { sens: 0 }] {
        assert!(Mesure::new(vec![plage()], provenance(geometrie, Some(lecture()))).is_err());
    }
}

#[test]
fn une_mesure_en_feuille_se_relit_avec_sa_lecture() {
    let mesure = Mesure::new(
        vec![plage(), plage()],
        provenance(Geometrie::Feuille {}, Some(lecture())),
    )
    .unwrap();
    let texte = ecrire_mesure(&mesure);
    assert!(texte.contains(r#""feuille":{"qr":{"statut":"confirmee","valeur":"RGB-2520 - 1P MY9*1+"},"zones":1,"plages":2520,"#), "{texte}");
    assert_eq!(lire_mesure(&texte), Ok(mesure.clone()));
    // Une ligne conservée sans sa lecture n'est plus une mesure en feuille.
    let sans = texte.replacen(r#","feuille":{"#, r#","x":{"#, 1);
    assert!(lire_mesure(&sans).is_err());
}

#[test]
fn une_ancienne_mesure_bande_se_relit_sans_feuille() {
    let mesure = Mesure::new(
        vec![plage(), plage()],
        provenance(Geometrie::Bande { sens: 2 }, None),
    )
    .unwrap();
    let texte = ecrire_mesure(&mesure);
    // Écrite comme avant ce champ : aucun « feuille » dans la ligne.
    assert!(!texte.contains("\"feuille\""), "{texte}");
    let relue = lire_mesure(&texte).unwrap();
    assert_eq!(relue.provenance().feuille, None);
    assert_eq!(relue, mesure);
}

#[test]
fn la_prenumerisation_et_les_erreurs_de_feuille_traversent_le_protocole() {
    for reponse in [
        Reponse::Prenumerisee { lecture: lecture() },
        Reponse::Prenumerisee {
            lecture: LectureFeuille {
                qr: Info::Inconnue,
                ..lecture()
            },
        },
        Reponse::Erreur {
            erreur: ErreurPont::MireNonReconnue {
                zones: 1,
                plages: 2_478,
                ecart_max: 400,
            },
        },
        Reponse::Erreur {
            erreur: ErreurPont::AutreMire {
                texte: "CMYK-1617".into(),
            },
        },
        Reponse::Erreur {
            erreur: ErreurPont::FeuilleAbsente {},
        },
        Reponse::Erreur {
            erreur: ErreurPont::IncidentFeuille {
                etat: 201,
                premesure: Some(3),
            },
        },
    ] {
        let texte = ecrire_reponse(&reponse);
        assert_eq!(lire_reponse(&texte), Ok(reponse), "{texte}");
    }
    assert_eq!(
        ecrire_reponse(&Reponse::Erreur {
            erreur: ErreurPont::IncidentFeuille {
                etat: 102,
                premesure: None
            }
        }),
        r#"{"rep":"erreur","erreur":{"type":"incident_feuille","etat":102,"premesure":null}}"#
    );
    assert!(lire_reponse(r#"{"rep":"erreur","erreur":{"type":"feuille_absente","x":1}}"#).is_err());
}
