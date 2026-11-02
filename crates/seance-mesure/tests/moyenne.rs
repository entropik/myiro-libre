//! Moyennage de plusieurs exemplaires d'une même mire : composition
//! conservée, écarts signalés sans exclusion, refus explicites.

mod commun;

use cgats::MesureImportee;
use commun::{lignes, plan, Export, EMPREINTE};
use pont_protocole::{
    Calcul, ConditionMesure, Geometrie, Horodatage, Info, InstrumentMesurant, Provenance,
};
use seance_mesure::{
    moyenner, Apport, Complement, ConditionEcartee, Incompatible, Role, Source, SEUIL_ECART_DE00,
};

fn exemplaire(id: i64, mesure: MesureImportee) -> Source {
    Source {
        id,
        mire: EMPREINTE.into(),
        role: Role::Exemplaire,
        mesure,
    }
}

fn exemplaire_decale(id: i64, decalage: f32) -> Source {
    exemplaire(id, Export::fd9(lignes(decalage)).mesure())
}

fn l_de(mesure: &MesureImportee, plage: usize) -> f32 {
    mesure.plages[plage].lab[1].valeur().unwrap().valeurs()[0]
}

#[test]
fn la_moyenne_est_celle_des_lab_et_des_spectres_plage_par_plage() {
    let mut a = lignes(0.0);
    let mut b = lignes(2.0);
    a[0].reflectance = 0.8;
    b[0].reflectance = 0.6;
    let m = moyenner(
        &plan(),
        &[
            exemplaire(1, Export::fd9(a).mesure()),
            exemplaire(2, Export::fd9(b).mesure()),
        ],
        &[],
    )
    .unwrap();
    assert_eq!(m.mesure.plages.len(), 3);
    assert_eq!(l_de(&m.mesure, 0), 91.0);
    let spectre = m.mesure.plages[0].spectres[1].valeur().unwrap();
    assert!(
        spectre.iter().all(|v| (v - 0.7).abs() < 1e-6),
        "{spectre:?}"
    );
    // Les RVB sont ceux de la mire, confirmés.
    assert_eq!(m.mesure.plages[1].rvb, Info::Confirmee([0.0, 0.5, 0.25]));
    assert_eq!(m.mesure.instrument, Info::Confirmee("FD-9".into()));
    assert_eq!(m.mesure.numero_serie, Info::Inconnue);
    assert!(m.complete());
}

#[test]
fn la_composition_nomme_les_mesures_de_chaque_plage() {
    let m = moyenner(
        &plan(),
        &[exemplaire_decale(4, 0.0), exemplaire_decale(9, 0.5)],
        &[],
    )
    .unwrap();
    assert_eq!(m.composition.len(), 3);
    assert_eq!(m.composition[2].plage, "3");
    assert_eq!(
        m.composition[2].apports,
        [
            Apport {
                source: 4,
                identifiant: "3".into(),
                manuel: false
            },
            Apport {
                source: 9,
                identifiant: "3".into(),
                manuel: false
            }
        ]
    );
}

#[test]
fn un_seul_exemplaire_est_rendu_tel_quel_sans_ecart() {
    let source = exemplaire_decale(1, 0.0);
    let m = moyenner(&plan(), std::slice::from_ref(&source), &[]).unwrap();
    for (moyenne, origine) in m.mesure.plages.iter().zip(&source.mesure.plages) {
        assert_eq!(moyenne.lab, origine.lab);
        assert_eq!(moyenne.spectres, origine.spectres);
    }
    assert!(m.ecarts.is_empty());
}

#[test]
fn les_ecarts_sous_le_seuil_sont_ecrits_sans_etre_signales() {
    let m = moyenner(
        &plan(),
        &[
            exemplaire_decale(1, 0.0),
            exemplaire_decale(2, SEUIL_ECART_DE00 as f32 / 10.0),
        ],
        &[],
    )
    .unwrap();
    assert_eq!(m.ecarts.len(), 6);
    assert!(m.ecarts.iter().all(|e| e.de00.unwrap() > 0.0));
    assert!(m.ecarts.iter().all(|e| !e.signale));
    assert_eq!(m.ecarts_signales().count(), 0);
}

#[test]
fn un_ecart_au_dela_du_seuil_est_signale_sans_rien_exclure() {
    let m = moyenner(
        &plan(),
        &[
            exemplaire_decale(1, 0.0),
            exemplaire_decale(2, SEUIL_ECART_DE00 as f32 * 10.0),
        ],
        &[],
    )
    .unwrap();
    assert!(m
        .ecarts
        .iter()
        .all(|e| e.signale == (e.de00.unwrap() > SEUIL_ECART_DE00)));
    assert_eq!(m.ecarts_signales().count(), 6);
    assert!(m.composition.iter().all(|c| c.apports.len() == 2));
}

#[test]
fn une_plage_d_un_seul_exemplaire_vient_de_lui_seul() {
    let mut incomplet = lignes(0.0);
    incomplet.remove(2);
    let m = moyenner(
        &plan(),
        &[
            exemplaire(1, Export::fd9(incomplet).mesure()),
            exemplaire_decale(2, 4.0),
        ],
        &[],
    )
    .unwrap();
    assert!(m.complete());
    assert_eq!(m.composition[2].apports.len(), 1);
    assert_eq!(m.composition[2].apports[0].source, 2);
    assert_eq!(l_de(&m.mesure, 2), 34.0);
}

#[test]
fn une_plage_que_personne_ne_donne_reste_manquante() {
    let mut l = lignes(0.0);
    l.remove(1);
    let m = moyenner(&plan(), &[exemplaire(1, Export::fd9(l).mesure())], &[]).unwrap();
    assert!(!m.complete());
    assert_eq!(m.manquantes, ["2"]);
    let ids: Vec<&str> = m
        .mesure
        .plages
        .iter()
        .map(|p| p.identifiant.as_str())
        .collect();
    assert_eq!(ids, ["1", "3"]);
}

/// Un complément : une mesure d'une seule plage, sans RVB, désignée à la
/// main pour la plage 3.
fn complement_plage_3() -> (Source, Complement) {
    let mut l = lignes(1.0);
    let mut e = Export::fd9(vec![l.remove(2)]);
    e.lignes[0].id = "A1".into();
    e.rvb = false;
    (
        Source {
            id: 30,
            mire: EMPREINTE.into(),
            role: Role::Complement,
            mesure: e.mesure(),
        },
        Complement {
            plage: "3".into(),
            source: 30,
            identifiant: "A1".into(),
        },
    )
}

fn sans_plage_3() -> Source {
    let mut l = lignes(0.0);
    l.remove(2);
    exemplaire(1, Export::fd9(l).mesure())
}

#[test]
fn un_complement_manuel_est_distingue_de_la_source() {
    let (source, complement) = complement_plage_3();
    let m = moyenner(&plan(), &[sans_plage_3(), source], &[complement]).unwrap();
    assert!(m.complete());
    assert_eq!(
        m.composition[2].apports,
        [Apport {
            source: 30,
            identifiant: "A1".into(),
            manuel: true
        }]
    );
    assert!(m.composition[0].apports.iter().all(|a| !a.manuel));
}

#[test]
fn sans_exemplaire_rien_n_est_moyenne() {
    let (source, complement) = complement_plage_3();
    assert_eq!(
        moyenner(&plan(), &[source], &[complement]).unwrap_err(),
        Incompatible::AucunExemplaire
    );
    assert_eq!(
        moyenner(&plan(), &[], &[]).unwrap_err(),
        Incompatible::AucunExemplaire
    );
}

#[test]
fn une_mesure_d_une_autre_mire_est_refusee() {
    let mut autre = exemplaire_decale(2, 0.0);
    autre.mire = "autre-empreinte".into();
    assert_eq!(
        moyenner(&plan(), &[exemplaire_decale(1, 0.0), autre], &[]).unwrap_err(),
        Incompatible::AutreMire { source: 2 }
    );
}

#[test]
fn un_exemplaire_a_l_association_incertaine_est_refuse() {
    let mut l = lignes(0.0);
    l[0].rvb = [0.0, 0.0, 0.0];
    assert_eq!(
        moyenner(&plan(), &[exemplaire(5, Export::fd9(l).mesure())], &[]).unwrap_err(),
        Incompatible::AssociationIncertaine { source: 5 }
    );
}

#[test]
fn un_instrument_inconnu_est_refuse() {
    let mut e = Export::fd9(lignes(0.0));
    e.instrument = "Instrument fictif".into();
    assert_eq!(
        moyenner(&plan(), &[exemplaire(3, e.mesure())], &[]).unwrap_err(),
        Incompatible::ModeleInconnu { source: 3 }
    );
}

#[test]
fn le_myiro1_et_le_fd9_ne_se_melangent_pas() {
    let mut e = Export::fd9(lignes(0.0));
    e.instrument = "KONICA MINOLTA, INC. MYIRO-1, Serial No. 12345678".into();
    assert_eq!(
        moyenner(
            &plan(),
            &[exemplaire_decale(1, 0.0), exemplaire(2, e.mesure())],
            &[]
        )
        .unwrap_err(),
        Incompatible::ModelesDifferents { source: 2 }
    );
}

#[test]
fn deux_fd9_differents_ne_se_melangent_pas() {
    let mut e = Export::fd9(lignes(0.0));
    e.serie = "87654321".into();
    let refus = moyenner(
        &plan(),
        &[exemplaire_decale(1, 0.0), exemplaire(2, e.mesure())],
        &[],
    )
    .unwrap_err();
    assert_eq!(refus, Incompatible::InstrumentsDifferents { source: 2 });
    // Le refus ne répète aucun numéro de série.
    assert!(!format!("{refus:?}").contains("8765"));
}

#[test]
fn sans_condition_commune_rien_n_est_moyenne() {
    let mut m0 = Export::fd9(lignes(0.0));
    m0.source = "A".into();
    let mut m2 = Export::fd9(lignes(0.0));
    m2.source = "UVCUT".into();
    assert_eq!(
        moyenner(
            &plan(),
            &[exemplaire(1, m0.mesure()), exemplaire(2, m2.mesure())],
            &[]
        )
        .unwrap_err(),
        Incompatible::AucuneConditionCommune
    );
}

#[test]
fn une_autre_condition_au_meme_emplacement_est_refusee() {
    let mut autre = exemplaire_decale(2, 0.0);
    autre.mesure.conditions[1] = Info::Supposee(ConditionMesure::M2);
    assert_eq!(
        moyenner(&plan(), &[exemplaire_decale(1, 0.0), autre], &[]).unwrap_err(),
        Incompatible::ConditionsDifferentes { source: 2 }
    );
}

/// Une mesure à la manière d'une feuille mesurée par myiro-libre : les
/// trois emplacements M0, M1, M2, conditions supposées.
fn acquisition(id: i64) -> Source {
    let mut s = exemplaire_decale(id, 0.0);
    s.mesure.conditions = [
        Info::Supposee(ConditionMesure::M0),
        Info::Supposee(ConditionMesure::M1),
        Info::Supposee(ConditionMesure::M2),
    ];
    for p in &mut s.mesure.plages {
        p.lab[0] = p.lab[1].clone();
        p.lab[2] = p.lab[1].clone();
        p.spectres[0] = p.spectres[1].clone();
        p.spectres[2] = p.spectres[1].clone();
    }
    s
}

#[test]
fn une_acquisition_et_un_export_fd_s2w_se_moyennent_sur_leur_condition_commune() {
    let m = moyenner(&plan(), &[acquisition(1), exemplaire_decale(2, 1.0)], &[]).unwrap();
    assert_eq!(
        m.mesure.conditions,
        [
            Info::Inconnue,
            Info::Supposee(ConditionMesure::M1),
            Info::Inconnue
        ]
    );
    assert!(m
        .mesure
        .plages
        .iter()
        .all(|p| p.lab[0] == Info::Inconnue && p.spectres[2] == Info::Inconnue));
    assert_eq!(l_de(&m.mesure, 0), 90.5);
    // Ce qui est écarté est dit, avec la mesure qui l'avait.
    assert_eq!(
        m.conditions_ecartees,
        [
            ConditionEcartee {
                emplacement: "m0".into(),
                source: 1,
                condition: Some(ConditionMesure::M0)
            },
            ConditionEcartee {
                emplacement: "m2".into(),
                source: 1,
                condition: Some(ConditionMesure::M2)
            }
        ]
    );
}

#[test]
fn des_conditions_communes_a_toutes_n_ecartent_rien() {
    let m = moyenner(&plan(), &[acquisition(1), acquisition(2)], &[]).unwrap();
    assert!(m.conditions_ecartees.is_empty());
    assert!(m.mesure.conditions.iter().all(|c| c.valeur().is_some()));
}

#[test]
fn d_autres_longueurs_d_onde_sont_refusees() {
    let mut e = Export::fd9(lignes(0.0));
    e.debut_nm = 390;
    assert_eq!(
        moyenner(
            &plan(),
            &[exemplaire_decale(1, 0.0), exemplaire(2, e.mesure())],
            &[]
        )
        .unwrap_err(),
        Incompatible::LongueursOndeDifferentes { source: 2 }
    );
}

fn provenance(geometrie: Geometrie) -> Provenance {
    Provenance {
        instrument: InstrumentMesurant {
            modele: "FD-9".into(),
            numero_serie: 12345678,
            micrologiciel: "1.0".into(),
            code_produit: "0000".into(),
        },
        version_sdk: [1, 3, 2],
        empreinte_dll: Info::Inconnue,
        version_pont: "0.1.0".into(),
        architecture: "x86".into(),
        horodatage: Horodatage::new("2026-11-02T10:00:00Z").unwrap(),
        etalonnage: Info::Inconnue,
        geometrie,
        calcul: Calcul {
            libelle: "fictif".into(),
            demande: Info::Inconnue,
            observe: Info::Inconnue,
        },
        feuille: None,
    }
}

#[test]
fn une_autre_geometrie_connue_est_refusee() {
    let mut a = exemplaire_decale(1, 0.0);
    let mut b = exemplaire_decale(2, 0.0);
    a.mesure.provenance = Info::Confirmee(provenance(Geometrie::Feuille {}));
    b.mesure.provenance = Info::Confirmee(provenance(Geometrie::Bande { sens: 0 }));
    assert_eq!(
        moyenner(&plan(), &[a, b], &[]).unwrap_err(),
        Incompatible::GeometriesDifferentes { source: 2 }
    );
}

#[test]
fn une_geometrie_inconnue_d_un_cote_n_empeche_rien() {
    let mut a = exemplaire_decale(1, 0.0);
    a.mesure.provenance = Info::Confirmee(provenance(Geometrie::Feuille {}));
    assert!(moyenner(&plan(), &[a, exemplaire_decale(2, 0.0)], &[]).is_ok());
}

#[test]
fn des_spectres_d_un_cote_seulement_sont_refuses() {
    let mut e = Export::fd9(lignes(0.0));
    e.spectres = false;
    assert_eq!(
        moyenner(
            &plan(),
            &[exemplaire_decale(1, 0.0), exemplaire(2, e.mesure())],
            &[]
        )
        .unwrap_err(),
        Incompatible::SpectresMeles {
            emplacement: "m1".into()
        }
    );
}

#[test]
fn des_lab_d_un_cote_seulement_sont_refuses() {
    let mut e = Export::fd9(lignes(0.0));
    e.lab = false;
    assert_eq!(
        moyenner(
            &plan(),
            &[exemplaire_decale(1, 0.0), exemplaire(2, e.mesure())],
            &[]
        )
        .unwrap_err(),
        Incompatible::LabMeles {
            emplacement: "m1".into()
        }
    );
}

#[test]
fn un_complement_hors_du_plan_est_refuse() {
    let (source, mut complement) = complement_plage_3();
    complement.plage = "99".into();
    assert_eq!(
        moyenner(&plan(), &[sans_plage_3(), source], &[complement]).unwrap_err(),
        Incompatible::ComplementInvalide { plage: "99".into() }
    );
}

#[test]
fn un_complement_d_une_source_absente_est_refuse() {
    let (_, complement) = complement_plage_3();
    assert_eq!(
        moyenner(&plan(), &[sans_plage_3()], &[complement]).unwrap_err(),
        Incompatible::ComplementInvalide { plage: "3".into() }
    );
}

#[test]
fn un_complement_tire_d_un_exemplaire_est_refuse() {
    let (_, mut complement) = complement_plage_3();
    complement.source = 7;
    complement.identifiant = "3".into();
    assert_eq!(
        moyenner(
            &plan(),
            &[sans_plage_3(), exemplaire_decale(7, 0.0)],
            &[complement]
        )
        .unwrap_err(),
        Incompatible::ComplementInvalide { plage: "3".into() }
    );
}

#[test]
fn un_complement_d_une_plage_absente_de_sa_source_est_refuse() {
    let (source, mut complement) = complement_plage_3();
    complement.identifiant = "B2".into();
    assert_eq!(
        moyenner(&plan(), &[sans_plage_3(), source], &[complement]).unwrap_err(),
        Incompatible::ComplementInvalide { plage: "3".into() }
    );
}

#[test]
fn un_complement_d_une_plage_deja_mesuree_est_refuse() {
    let (source, complement) = complement_plage_3();
    assert_eq!(
        moyenner(&plan(), &[exemplaire_decale(1, 0.0), source], &[complement]).unwrap_err(),
        Incompatible::ComplementInvalide { plage: "3".into() }
    );
}

#[test]
fn deux_complements_pour_une_plage_sont_refuses() {
    let (source, complement) = complement_plage_3();
    assert_eq!(
        moyenner(
            &plan(),
            &[sans_plage_3(), source],
            &[complement.clone(), complement]
        )
        .unwrap_err(),
        Incompatible::ComplementInvalide { plage: "3".into() }
    );
}

#[test]
fn la_moyenne_s_ecrit_en_cgats_et_se_relit() {
    let m = moyenner(
        &plan(),
        &[exemplaire_decale(1, 0.0), exemplaire_decale(2, 1.0)],
        &[],
    )
    .unwrap();
    let texte = cgats::ecrire_moyenne(&m.mesure, EMPREINTE, &["1", "2"]).unwrap();
    assert_eq!(cgats::lire(&texte).unwrap(), m.mesure);
}
