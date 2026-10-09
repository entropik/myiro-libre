//! Annuler une mesure en attente et récupérer proprement (ticket #26), au
//! seam `Pont` avec le pont simulé : le module `instrument` ouvre, dit les
//! mesures prises en charge, mesure, annule et ferme ; les gestes passent par
//! le trait `Gestes`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use app::instrument::{Accord, Etat, Geste, Instrument, Issue, TypeMesure};
use app::pont::{Annulation, Architecture, AttenteSimulee, Panne, PontSimule};
use pont_protocole::{Declenchement, Palier, RemiseAuRepos, Reponse, Requete};

const SERIE: u32 = 12345678;

fn dossier_sdk(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join("myiro-libre-tests")
        .join("annulation")
        .join(nom);
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    // Fausse DLL : seul l'en-tête PE compte, elle n'est jamais chargée.
    let mut octets = vec![0u8; 0x48];
    octets[..2].copy_from_slice(b"MZ");
    octets[0x3c] = 0x40;
    octets[0x40..0x44].copy_from_slice(b"PE\0\0");
    octets[0x44..0x46].copy_from_slice(&0x8664u16.to_le_bytes());
    std::fs::write(dossier.join("FDXSDK.dll"), octets).unwrap();
    dossier
}

fn ponts() -> Vec<(Architecture, PathBuf)> {
    vec![(Architecture::X64, PathBuf::from("pont-myiro1-x64.exe"))]
}

fn fait(_: Geste) -> Accord {
    Accord::Fait
}

/// Instrument ouvert et étalonné sur le pont simulé donné.
fn etalonne(simule: PontSimule, nom: &str) -> Instrument<PontSimule> {
    let mut instrument =
        Instrument::ouvrir(&[dossier_sdk(nom)], &ponts(), |_: &Path, _: &Path, _| {
            Ok(simule)
        });
    instrument.etalonner(&mut fait);
    assert!(matches!(instrument.etat(), Etat::Etalonne(_)));
    instrument
}

fn mesurer(instrument: &mut Instrument<PontSimule>) -> Issue {
    instrument.mesurer(&mut fait, TypeMesure::Ponctuelle, Declenchement::Manuel)
}

/// L'opérateur clique sur « Annuler » d'un autre fil, dès qu'une mesure est
/// en cours.
fn annuler_des_que_possible(annulation: Annulation) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for _ in 0..2000 {
            if annulation.annuler() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("aucune mesure à annuler");
    })
}

fn mesures_envoyees(requetes: &[Requete]) -> usize {
    requetes
        .iter()
        .filter(|r| matches!(r, Requete::MesurerPonctuelle { .. }))
        .count()
}

#[test]
fn le_myiro1_ouvert_prend_en_charge_la_mesure_ponctuelle() {
    let instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "prises-en-charge");
    assert_eq!(
        instrument.mesures_prises_en_charge(),
        vec![TypeMesure::Ponctuelle]
    );
    let absent: Instrument<PontSimule> =
        Instrument::ouvrir(&[], &ponts(), |_: &Path, _: &Path, _| {
            panic!("rien à lancer")
        });
    assert!(absent.mesures_prises_en_charge().is_empty());
}

#[test]
fn une_mesure_en_attente_s_annule_et_l_instrument_reste_pret() {
    let simule = PontSimule::avec_instruments(&[SERIE])
        .attendre_annulation(AttenteSimulee::Annulee(RemiseAuRepos::AuRepos {}));
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "annulee");
    let operateur = annuler_des_que_possible(instrument.annulation());

    let issue = mesurer(&mut instrument);
    operateur.join().unwrap();

    assert_eq!(
        issue,
        Issue::Annulee {
            remise_au_repos: RemiseAuRepos::AuRepos {}
        }
    );
    let requetes = journal.requetes();
    assert_eq!(
        requetes[requetes.len() - 2..],
        [
            Requete::MesurerPonctuelle {
                declenchement: Declenchement::Manuel
            },
            Requete::Annuler {}
        ]
    );
    // Rien à réparer : l'instrument reste étalonné, une nouvelle mesure part.
    assert!(matches!(instrument.etat(), Etat::Etalonne(_)));
    assert_eq!(instrument.probleme(), None);
    assert!(instrument.vue().mesurable);
    assert!(matches!(mesurer(&mut instrument), Issue::Acquise(_)));
}

/// L'annulation rapporte le retour au repos ; s'il n'est pas prouvé, aucune
/// nouvelle mesure ne part avant la récupération (nouveau pont).
#[test]
fn sans_repos_prouve_apres_l_annulation_la_mesure_suivante_attend_la_recuperation() {
    let simule = PontSimule::avec_instruments(&[SERIE])
        .attendre_annulation(AttenteSimulee::Annulee(RemiseAuRepos::ReposNonSignale {}));
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "annulee-sans-repos");
    let operateur = annuler_des_que_possible(instrument.annulation());

    let issue = mesurer(&mut instrument);
    operateur.join().unwrap();

    assert_eq!(
        issue,
        Issue::Annulee {
            remise_au_repos: RemiseAuRepos::ReposNonSignale {}
        }
    );
    assert_eq!(instrument.probleme().unwrap().code(), "repos_incertain");
    assert!(!instrument.vue().mesurable);
    let avant = journal.requetes().len();
    let suivante = instrument.mesurer(
        &mut |g: Geste| -> Accord { panic!("geste demandé : {g:?}") },
        TypeMesure::Ponctuelle,
        Declenchement::Manuel,
    );
    assert_eq!(suivante, Issue::Refusee);
    assert_eq!(journal.requetes().len(), avant, "rien vers le pont");
}

/// Course entre la fin de la mesure et l'annulation : la mesure déjà acquise
/// est le résultat unique, avec sa provenance ; aucun abandon n'est annoncé.
#[test]
fn un_resultat_arrive_malgre_l_annulation_est_garde() {
    let simule =
        PontSimule::avec_instruments(&[SERIE]).attendre_annulation(AttenteSimulee::TermineeAvant);
    let mut instrument = etalonne(simule, "resultat-tardif");
    let operateur = annuler_des_que_possible(instrument.annulation());

    let issue = mesurer(&mut instrument);
    operateur.join().unwrap();

    let Issue::Acquise(acquise) = issue else {
        panic!("mesure attendue : {issue:?}");
    };
    assert_eq!(acquise.mesure.provenance().instrument.numero_serie, SERIE);
    assert_eq!(instrument.probleme(), None);
    assert!(instrument.vue().mesurable);
}

/// L'opérateur annule pendant le geste : rien ne part vers le pont.
#[test]
fn une_annulation_pendant_le_geste_n_envoie_rien() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "annulee-au-geste");
    let annulation = instrument.annulation();
    let avant = journal.requetes().len();

    let issue = instrument.mesurer(
        &mut |_: Geste| {
            assert!(annulation.annuler());
            Accord::Fait
        },
        TypeMesure::Ponctuelle,
        Declenchement::Manuel,
    );

    assert_eq!(issue, Issue::Abandonnee);
    assert_eq!(journal.requetes().len(), avant);
    assert!(instrument.vue().mesurable);
}

/// Sans mesure en cours, il n'y a rien à annuler : la demande est refusée.
#[test]
fn annuler_sans_mesure_en_cours_est_refuse() {
    let instrument = etalonne(PontSimule::avec_instruments(&[SERIE]), "rien-a-annuler");
    assert!(!instrument.annulation().annuler());
}

/// L'annulation est faite, mais le pont ne la confirme jamais : la mesure
/// acquise est gardée, et l'état de l'instrument est dit à part (pont arrêté,
/// état incertain, récupération à faire).
#[test]
fn une_mesure_valide_est_gardee_malgre_une_erreur_de_nettoyage() {
    let simule =
        PontSimule::avec_instruments(&[SERIE]).attendre_annulation(AttenteSimulee::SansAccuse);
    let mut instrument = etalonne(simule, "sans-accuse");
    let operateur = annuler_des_que_possible(instrument.annulation());

    let issue = mesurer(&mut instrument);
    operateur.join().unwrap();

    assert!(matches!(issue, Issue::Acquise(_)), "{issue:?}");
    assert_eq!(instrument.probleme().unwrap().code(), "pont_bloque");
    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert!(!instrument.vue().mesurable);
}

/// Le pont se perd pendant la mesure : un seul envoi, jamais répété ; puis
/// reconnexion (nouveau pont, même plafond), étalonnage et nouvelle mesure.
#[test]
fn apres_la_perte_du_pont_on_se_reconnecte_puis_on_mesure_de_nouveau() {
    let perdu = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::MesurePonctuelle,
        Err(Panne::Arret {
            code: Some(1),
            detail: "fin du processus".into(),
        }),
    );
    let journal = perdu.journal();
    let mut instrument = etalonne(perdu, "perte");

    let issue = mesurer(&mut instrument);

    assert_eq!(issue, Issue::Echouee);
    assert_eq!(instrument.probleme().unwrap().code(), "pont_en_panne");
    assert_eq!(instrument.etat(), &Etat::NonDetecte);
    assert_eq!(
        mesures_envoyees(&journal.requetes()),
        1,
        "aucune répétition"
    );
    // Rien ne part plus vers le pont perdu.
    assert_eq!(mesurer(&mut instrument), Issue::Refusee);
    assert_eq!(mesures_envoyees(&journal.requetes()), 1);

    let mut plafonds = Vec::new();
    let mut instrument =
        Instrument::ouvrir(&[dossier_sdk("reconnexion")], &ponts(), |_, _, plafond| {
            plafonds.push(plafond);
            Ok(PontSimule::avec_instruments(&[SERIE]))
        });
    assert_eq!(plafonds, vec![app::instrument::PLAFOND]);
    assert!(matches!(instrument.etat(), Etat::EtalonnageRequis(_)));
    instrument.etalonner(&mut fait);
    assert!(matches!(mesurer(&mut instrument), Issue::Acquise(_)));
}

/// Interruption forcée (pont muet) : l'état matériel est incertain et une
/// récupération est demandée, sans répétition de la mesure.
#[test]
fn un_pont_muet_pendant_la_mesure_laisse_un_etat_incertain() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::MesurePonctuelle,
        Err(Panne::SansReponse {
            detail: "arrêté de force".into(),
        }),
    );
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "muet");

    assert_eq!(mesurer(&mut instrument), Issue::Echouee);
    let probleme = instrument.probleme().unwrap();
    assert_eq!(probleme.code(), "pont_bloque");
    assert!(instrument.vue().probleme.unwrap().ecran == "non_detecte");
    assert_eq!(mesures_envoyees(&journal.requetes()), 1);
}

#[test]
fn la_fermeture_repetee_ne_refait_rien() {
    let simule = PontSimule::avec_instruments(&[SERIE]);
    let journal = simule.journal();
    let mut instrument = etalonne(simule, "fermeture-repetee");

    assert!(instrument.fermer().is_some());
    assert_eq!(instrument.fermer(), None);
    let fermetures = journal
        .requetes()
        .iter()
        .filter(|r| matches!(r, Requete::Fermer {}))
        .count();
    assert_eq!(fermetures, 1);
    assert!(instrument.mesures_prises_en_charge().is_empty());
}

/// Une réponse `annulation` à la place d'un résultat de mesure (dialogue
/// décalé) n'est jamais prise pour une mesure : pont fermé.
#[test]
fn une_reponse_decalee_n_est_jamais_prise_pour_une_mesure() {
    let simule = PontSimule::avec_instruments(&[SERIE]).echouer_a(
        Palier::MesurePonctuelle,
        Ok(Reponse::Annulation {
            effet: pont_protocole::EffetAnnulation::SansEffet,
        }),
    );
    let mut instrument = etalonne(simule, "decalee");
    assert_eq!(mesurer(&mut instrument), Issue::Echouee);
    assert_eq!(instrument.probleme().unwrap().code(), "pont_en_panne");
}
