//! Une mesure de l'instrument simulé traverse le protocole, puis s'enregistre
//! et se relit sans perte de valeurs ni de provenance.

mod commun;

use commun::{evenement, SdkSimule};
use pont_myiro1::serveur::servir;
use pont_myiro1::Session;
use pont_protocole::{
    ecrire_mesure, lire_mesure, lire_reponse, ConditionMesure, ErreurPont, Geometrie, Info, Mesure,
    Observateur, Palier, Reponse,
};

/// Va jusqu'à l'étalonnage, puis envoie `mesure` ; rend la dernière réponse.
fn derniere_reponse(mut sdk: SdkSimule, mesure: &str) -> Reponse {
    sdk.evenements.extend([7, 8].map(evenement));
    sdk.salves.push_back([1, 2, 3].map(evenement).to_vec());
    let mut session = Session::new(sdk, Palier::Bande);
    let entree = [
        r#"{"cmd":"version"}"#,
        r#"{"cmd":"detecter"}"#,
        r#"{"cmd":"connecter","instrument":0}"#,
        r#"{"cmd":"etalonner"}"#,
        mesure,
    ]
    .join("\n");
    let mut sortie = Vec::new();
    servir(&mut session, std::io::Cursor::new(entree), &mut sortie).unwrap();
    let texte = String::from_utf8(sortie).unwrap();
    lire_reponse(texte.lines().last().unwrap()).unwrap()
}

fn mesure_recue(sdk: SdkSimule, requete: &str) -> Mesure {
    match derniere_reponse(sdk, requete) {
        Reponse::Mesure { mesure, .. } => mesure,
        autre => panic!("{autre:?}"),
    }
}

#[test]
fn une_mesure_simulee_traverse_le_protocole_puis_l_archivage_sans_perte() {
    let mesure = mesure_recue(
        SdkSimule::avec_un_myiro1(),
        r#"{"cmd":"mesurer_ponctuelle"}"#,
    );
    // Valeurs repérables de l'instrument simulé : condition × 10 + type de données.
    let plage = &mesure.plages()[0];
    assert_eq!(&plage.m0()[..], &[10.0; 36], "réflectance > 1 acceptée");
    assert_eq!(&plage.m2()[..], &[30.0; 36]);
    assert_eq!(&plage.brutes()[..], &[21.0; 152]);
    assert_eq!(plage.lab()[1].valeurs(), [10.0; 3]);

    let relue = lire_mesure(&ecrire_mesure(&mesure)).unwrap();
    assert_eq!(relue, mesure);
}

#[test]
fn le_pont_pose_une_provenance_structuree() {
    let mesure = mesure_recue(
        SdkSimule::avec_un_myiro1(),
        r#"{"cmd":"mesurer_ponctuelle"}"#,
    );
    let p = mesure.provenance();
    assert_eq!(p.instrument.modele, "MYIRO-1");
    assert_eq!(p.instrument.numero_serie, 12345678);
    assert_eq!(p.version_sdk, [1, 1, 0]);
    assert!(matches!(p.empreinte_dll, Info::Confirmee(_)));
    assert!(matches!(p.etalonnage, Info::Confirmee(_)));
    assert_eq!(p.geometrie, Geometrie::Ponctuelle {});
    let Info::Confirmee(demande) = &p.calcul.demande else {
        panic!("{:?}", p.calcul.demande)
    };
    assert_eq!(
        demande.conditions_spectres,
        [
            Info::Confirmee(ConditionMesure::M0),
            Info::Confirmee(ConditionMesure::M1),
            Info::Confirmee(ConditionMesure::M2)
        ]
    );
    // Fiche FDX_GetMeasureData : « 0 = 2° » n'est que supposé.
    assert_eq!(
        demande.observateur_lab,
        Info::Supposee(Observateur::DeuxDegres)
    );
    // Aucune condition n'est relue sur l'instrument.
    assert_eq!(p.calcul.observe, Info::Inconnue);
}

#[test]
fn une_bande_garde_le_sens_de_passage_rendu_par_la_dll() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.resultats_par_lecture = 3;
    sdk.sens = 2;
    let mesure = mesure_recue(sdk, r#"{"cmd":"mesurer_bande","plages_attendues":3}"#);
    assert_eq!(mesure.plages().len(), 3);
    assert_eq!(mesure.provenance().geometrie, Geometrie::Bande { sens: 2 });
    assert_eq!(lire_mesure(&ecrire_mesure(&mesure)), Ok(mesure));
}

#[test]
fn une_valeur_non_finie_de_la_dll_n_est_pas_transmise() {
    let mut sdk = SdkSimule::avec_un_myiro1();
    sdk.valeur_non_finie = true;
    let reponse = derniere_reponse(sdk, r#"{"cmd":"mesurer_ponctuelle"}"#);
    let Reponse::Erreur {
        erreur: ErreurPont::ReponseInattendue { detail },
    } = reponse
    else {
        panic!("{reponse:?}")
    };
    assert!(detail.contains("non finie"), "{detail}");
}
