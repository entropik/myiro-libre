//! Le dialogue JSON du pont FD-9 : même protocole que le pont MYIRO-1, une
//! ligne par requête et une par réponse, avec ses propres paliers.

mod commun;

use commun::{Fd9Simule, EMPREINTE_SIMULEE};
use pont_fd9::serveur::{servir, Garde, DELAI_FERMETURE};
use pont_fd9::{Session, ATTENTE_APRES_DECONNEXION, PALIER_MAX};
use pont_protocole::{
    lire_reponse, AvancementFeuille, Empreinte, ErreurPont, Info, InstrumentFd9, LectureFeuille,
    LiaisonFd9, Palier, RemiseAuRepos, Reponse,
};
use std::sync::mpsc;
use std::time::Duration;

/// Envoie `requetes` au pont ; rend les lignes écrites et la session.
fn dialoguer(
    sdk: Fd9Simule,
    plafond: Palier,
    requetes: &[&str],
) -> (Vec<String>, Session<Fd9Simule>) {
    let mut session = Session::new(sdk, plafond);
    let entree = requetes.join("\n");
    let mut sortie = Vec::new();
    servir(
        &mut session,
        std::io::Cursor::new(entree.into_bytes()),
        &mut sortie,
        // Ici la fermeture rend toujours la main : la garde n'agit pas.
        Garde::new(Duration::from_secs(60), |_| {}),
    )
    .unwrap();
    let lignes = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    (lignes, session)
}

/// Réponses finales, une par requête : l'avancement d'une feuille, écrit
/// avant sa réponse, est écarté.
fn reponses(lignes: &[String]) -> Vec<Reponse> {
    lignes
        .iter()
        .map(|l| lire_reponse(l).unwrap())
        .filter(|r| !matches!(r, Reponse::Avancement { .. }))
        .collect()
}

/// Calage annoncé par l'avancement (`calage_pret`).
fn calage_annonce(lignes: &[String]) -> pont_protocole::CalageFeuille {
    lignes
        .iter()
        .find_map(|l| match lire_reponse(l).unwrap() {
            Reponse::Avancement {
                avancement: AvancementFeuille::CalagePret { calage },
            } => Some(calage),
            _ => None,
        })
        .expect("calage annoncé")
}

/// Lignes des réponses finales, sans celles de l'avancement.
fn finales(lignes: &[String]) -> Vec<&String> {
    lignes
        .iter()
        .filter(|l| !l.starts_with(r#"{"rep":"avancement""#))
        .collect()
}

#[test]
fn version_puis_detection_rendent_le_fd9_du_reseau() {
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"version"}"#, r#"{"cmd":"detecter"}"#],
    );
    assert_eq!(
        reponses(&lignes),
        vec![
            Reponse::VersionDll {
                version_fichier: Info::Confirmee([1, 3, 2, 3]),
                empreinte: Info::Confirmee(Empreinte::new(EMPREINTE_SIMULEE.repeat(32)).unwrap()),
            },
            Reponse::InstrumentsFd9 {
                liste: vec![InstrumentFd9 {
                    liaison: Info::Confirmee(LiaisonFd9::Reseau),
                    adresse: "192.0.2.40".into(),
                    identifiant: Info::Supposee("12345678".into()),
                }],
            },
        ]
    );
}

#[test]
fn une_liaison_hors_fiche_et_un_identifiant_vide_sont_inconnus() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.appareils[0].code_liaison = 7;
    sdk.appareils[0].identifiant = [0; 8];
    let (lignes, _) = dialoguer(
        sdk,
        Palier::Detection,
        &[r#"{"cmd":"version"}"#, r#"{"cmd":"detecter"}"#],
    );
    assert_eq!(
        reponses(&lignes)[1],
        Reponse::InstrumentsFd9 {
            liste: vec![InstrumentFd9 {
                liaison: Info::Inconnue,
                adresse: "192.0.2.40".into(),
                identifiant: Info::Inconnue,
            }],
        }
    );
}

#[test]
fn une_version_illisible_est_dite_inconnue() {
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.version_fichier = None;
    sdk.empreinte = Some("pas une empreinte".into());
    let (lignes, _) = dialoguer(sdk, Palier::Version, &[r#"{"cmd":"version"}"#]);
    assert_eq!(
        reponses(&lignes),
        vec![Reponse::VersionDll {
            version_fichier: Info::Inconnue,
            empreinte: Info::Inconnue,
        }]
    );
}

#[test]
fn la_connexion_est_refusee_au_plafond_detection_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"detecter"}"#,
            r#"{"cmd":"connecter","instrument":0}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        ],
    );
    let refus = Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande: Palier::Connexion,
            plafond: Palier::Detection,
        },
    };
    assert_eq!(reponses(&lignes)[2..], [refus.clone(), refus]);
    assert_eq!(session.sdk().appels.len(), 2, "{:?}", session.sdk().appels);
}

#[test]
fn etalonnage_et_mesures_du_myiro1_sont_refuses_par_les_paliers_du_fd9() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Feuille,
        &[
            r#"{"cmd":"etalonner"}"#,
            r#"{"cmd":"mesurer_ponctuelle"}"#,
            r#"{"cmd":"mesurer_bande"}"#,
        ],
    );
    let refus = |demande| Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande,
            plafond: PALIER_MAX,
        },
    };
    assert_eq!(
        reponses(&lignes),
        [
            refus(Palier::Etalonnage),
            refus(Palier::MesurePonctuelle),
            refus(Palier::Bande),
        ]
    );
    assert!(
        session.sdk().appels.is_empty(),
        "{:?}",
        session.sdk().appels
    );
}

#[test]
fn connecter_puis_fermer_rendent_l_identite_puis_ferme() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    let reponses = reponses(&lignes);
    let Reponse::Connecte { identite } = &reponses[1] else {
        panic!("{lignes:?}")
    };
    assert_eq!(identite.numero_serie, 12345678);
    assert_eq!(reponses[2], Reponse::Ferme {});
    assert_eq!(
        session.sdk().appels_dll().last(),
        Some(&"FD9_RegisterDeviceEventHandler nul")
    );
}

#[test]
fn une_deconnexion_refusee_rend_fermeture_incertaine_et_arrete_le_pont() {
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_un_fd9().rendre("FD9_Disconnect", 1104),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"version"}"#,
        ],
    );
    assert_eq!(lignes.len(), 3);
    assert_eq!(
        lire_reponse(&lignes[2]),
        Ok(Reponse::FermetureIncertaine {
            remise_au_repos: RemiseAuRepos::ArretRefuse { code: 1104 }
        })
    );
}

#[test]
fn la_fin_de_l_entree_deconnecte() {
    let (_, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        ],
    );
    assert!(!session.sdk().connecte && !session.sdk().inscrit);
}

#[test]
fn une_adresse_trop_longue_est_une_requete_invalide() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"connecter_adresse","adresse":"fd9-atelier.exemple.lan2"}"#],
    );
    assert!(matches!(
        reponses(&lignes)[0],
        Reponse::RequeteInvalide { .. }
    ));
    assert!(session.sdk().appels.is_empty());
}

#[test]
fn fermer_est_confirme_et_arrete_le_pont() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"fermer"}"#,
            r#"{"cmd":"detecter"}"#,
        ],
    );
    assert_eq!(lignes.len(), 2);
    assert_eq!(lire_reponse(&lignes[1]), Ok(Reponse::Ferme {}));
    assert_eq!(session.sdk().appels.len(), 1);
}

#[test]
fn une_ligne_illisible_ne_declenche_rien() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &["version", r#"{"cmd":"jig_get_sdk_version"}"#],
    );
    assert!(reponses(&lignes)
        .iter()
        .all(|r| matches!(r, Reponse::RequeteInvalide { .. })));
    assert!(session.sdk().appels.is_empty());
}

/// Le FD-9 ne mesure pas : `annuler` n'a jamais rien à interrompre (#26).
#[test]
fn annuler_est_sans_effet_et_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_un_fd9(),
        Palier::Detection,
        &[r#"{"cmd":"annuler"}"#],
    );
    assert_eq!(
        reponses(&lignes),
        vec![Reponse::Annulation {
            effet: pont_protocole::EffetAnnulation::SansEffet
        }]
    );
    assert!(session.sdk().appels.is_empty());
}

/// Définition de la RGB-2520, telle que l'application l'enverra.
/// Définition JSON d'une RGB-2520 de `rangees` × `colonnes` plages, avec
/// ses couleurs fictives.
fn mire(rangees: u32, colonnes: u32) -> String {
    let couleurs: Vec<String> = commun::couleurs_fictives((rangees * colonnes) as usize)
        .iter()
        .map(|[r, v, b]| format!("[{r},{v},{b}]"))
        .collect();
    format!(
        r#"{{"nom":"RGB-2520 - 1P MY9","largeur":27940,"hauteur":41660,"zones":[{{"gauche":1349,"haut":2300,"largeur_plage":601,"hauteur_plage":601,"pas_x":601,"pas_y":601,"rangees":{rangees},"colonnes":{colonnes}}}],"couleurs":[{}]}}"#,
        couleurs.join(",")
    )
}

#[test]
fn la_feuille_est_refusee_au_plafond_connexion_sans_appel_a_la_dll() {
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_une_rgb2520(),
        Palier::Connexion,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &format!(r#"{{"cmd":"prenumeriser","mire":{}}}"#, mire(60, 42)),
            &format!(r#"{{"cmd":"mesurer_feuille","mire":{}}}"#, mire(60, 42)),
        ],
    );
    let refus = |demande| Reponse::Erreur {
        erreur: ErreurPont::PalierNonAutorise {
            demande,
            plafond: Palier::Connexion,
        },
    };
    assert_eq!(
        reponses(&lignes)[2..],
        [refus(Palier::Prenumerisation), refus(Palier::Feuille)]
    );
    // Après la connexion, plus rien jusqu'à la fermeture.
    assert_eq!(
        session.sdk().appels_dll()[1..5],
        [
            "FD9_RegisterDeviceEventHandler rappel",
            "FD9_Connect 192.0.2.40 myiro-libre",
            "FD9_GetSystemInfo",
            "FD9_GetStatus"
        ]
    );
    assert_eq!(
        session.sdk().appels_dll()[5..],
        [
            "FD9_ExitMeasurement 0 nul",
            "FD9_Disconnect",
            "FD9_GetStatus",
            "FD9_RegisterDeviceEventHandler nul"
        ]
    );
}

#[test]
fn mesurer_feuille_rend_la_mesure_de_toutes_les_plages() {
    // 2 rangées × 3 colonnes : six plages.
    let mire = mire(2, 3);
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_une_rgb2520(),
        Palier::Feuille,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &format!(r#"{{"cmd":"mesurer_feuille","mire":{mire}}}"#),
            r#"{"cmd":"fermer"}"#,
        ],
    );
    let reponses = reponses(&lignes);
    let Reponse::Mesure {
        mesure,
        remise_au_repos,
        calage,
    } = &reponses[2]
    else {
        panic!("{}", finales(&lignes)[2])
    };
    assert_eq!(mesure.plages().len(), 6);
    // Points visés et image de la feuille, annoncés avant la mesure, une
    // seule fois.
    assert_eq!(*calage, None);
    let calage = calage_annonce(&lignes);
    assert_eq!(calage.points.len(), 6);
    assert!(calage.image.is_some());
    assert!(mesure.provenance().feuille.is_some());
    // La feuille est close avant la réponse.
    assert_eq!(*remise_au_repos, Info::Confirmee(RemiseAuRepos::AuRepos {}));
    assert_eq!(reponses[3], Reponse::Ferme {});
}

/// Pendant la feuille, le pont écrit son avancement (ticket #34), une ligne
/// par étape, avant la réponse finale : l'écran suit la feuille au lieu de
/// rester sur « glissez la feuille ».
#[test]
fn mesurer_feuille_ecrit_son_avancement_avant_la_mesure() {
    let mire = mire(2, 3);
    let (lignes, _) = dialoguer(
        Fd9Simule::avec_une_rgb2520(),
        Palier::Feuille,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &format!(r#"{{"cmd":"mesurer_feuille","mire":{mire}}}"#),
            r#"{"cmd":"fermer"}"#,
        ],
    );
    let toutes: Vec<Reponse> = lignes.iter().map(|l| lire_reponse(l).unwrap()).collect();
    let etapes: Vec<&str> = toutes[2..]
        .iter()
        .map(|r| match r {
            Reponse::Avancement { avancement } => match avancement {
                AvancementFeuille::FeuilleEngagee {} => "engagee",
                AvancementFeuille::Lecture {} => "lecture",
                AvancementFeuille::CalagePret { .. } => "calage",
                AvancementFeuille::MesureLancee {} => "lancee",
                AvancementFeuille::Mesure { .. } => "mesure",
            },
            // Le calage est déjà parti dans l'avancement : la réponse ne le
            // renvoie pas.
            Reponse::Mesure { calage: None, .. } => "MESURE",
            Reponse::Ferme {} => "FERME",
            autre => panic!("{autre:?}"),
        })
        .collect();
    assert_eq!(
        etapes,
        ["engagee", "lecture", "calage", "lancee", "mesure", "MESURE", "FERME"]
    );
    // Le compte de plages est celui du rappel : 6 sur 6, jamais inventé.
    assert_eq!(
        toutes[6],
        Reponse::Avancement {
            avancement: AvancementFeuille::Mesure {
                mesurees: 6,
                plages: 6
            }
        }
    );
}

#[test]
fn prenumeriser_rend_la_lecture_de_la_feuille() {
    let prenumeriser = format!(r#"{{"cmd":"prenumeriser","mire":{}}}"#, mire(60, 42));
    let (lignes, session) = dialoguer(
        Fd9Simule::avec_une_rgb2520(),
        Palier::Prenumerisation,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &prenumeriser,
            r#"{"cmd":"fermer"}"#,
        ],
    );
    let Reponse::Prenumerisee { lecture, calage } = &reponses(&lignes)[2] else {
        panic!("{}", finales(&lignes)[2])
    };
    assert_eq!(
        *lecture,
        LectureFeuille {
            qr: Info::Confirmee("RGB-2520 - 1P MY9*1+".into()),
            zones: 1,
            plages: 2_520,
            fiabilite: 0,
            ecart_max: 0,
        }
    );
    // Le calage part une fois, dans l'avancement.
    assert_eq!(*calage, None);
    let calage = calage_annonce(&lignes);
    assert_eq!(calage.points.len(), 2_520);
    assert_eq!(calage.image.as_ref().unwrap().largeur(), 40);
    assert!(lignes.iter().any(|l| l.contains(
        r#""etape":"calage_pret","calage":{"decalage":[0,0],"ecart_max":0,"points":[{"zone":0"#
    )));
    assert_eq!(reponses(&lignes)[3], Reponse::Ferme {});
    assert!(!session
        .sdk()
        .appels_dll()
        .iter()
        .any(|a| a.starts_with("FD9_StartMeasurement")));
}

#[test]
fn annuler_interrompt_l_attente_de_la_feuille() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let prenumeriser = format!(r#"{{"cmd":"prenumeriser","mire":{}}}"#, mire(60, 42));
    let (lignes, session) = dialoguer(
        sdk,
        Palier::Prenumerisation,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &prenumeriser,
            r#"{"cmd":"annuler"}"#,
        ],
    );
    // La feuille reçoit son résultat unique, puis `annuler` son accusé.
    assert_eq!(
        reponses(&lignes)[2..],
        [
            Reponse::Erreur {
                erreur: ErreurPont::MesureAnnulee {
                    remise_au_repos: RemiseAuRepos::AuRepos {}
                }
            },
            Reponse::Annulation {
                effet: pont_protocole::EffetAnnulation::Appliquee
            },
        ]
    );
    // État 1 relu : ni arrêt ni sortie pour la feuille ; la seule sortie est
    // celle de la fermeture, juste avant la déconnexion.
    let appels = session.sdk().appels_dll();
    assert!(!appels.iter().any(|a| a.starts_with("FD9_StopMeasurement")));
    let sortie = appels
        .iter()
        .position(|a| *a == "FD9_ExitMeasurement 0 nul")
        .unwrap();
    assert_eq!(appels[sortie + 1], "FD9_Disconnect");
}

#[test]
fn la_fin_de_l_entree_abandonne_une_feuille_pas_encore_glissee() {
    let mut sdk = Fd9Simule::avec_une_rgb2520();
    sdk.a_venir.clear();
    let prenumeriser = format!(r#"{{"cmd":"prenumeriser","mire":{}}}"#, mire(60, 42));
    let (lignes, session) = dialoguer(
        sdk,
        Palier::Prenumerisation,
        &[
            r#"{"cmd":"version"}"#,
            r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
            &prenumeriser,
        ],
    );
    assert!(matches!(
        reponses(&lignes)[2],
        Reponse::Erreur {
            erreur: ErreurPont::MesureAnnulee { .. }
        }
    ));
    assert!(!session.sdk().connecte && !session.sdk().inscrit);
}

/// Dialogue où `FD9_Disconnect` reste bloqué jusqu'à ce que la garde ait
/// agi. Rend les lignes écrites par la boucle, ce que la garde a reçu (une
/// réponse ou rien) et la session.
fn dialoguer_deconnexion_bloquee(
    requetes: &[&str],
) -> (Vec<String>, Vec<Option<Reponse>>, Session<Fd9Simule>) {
    let (liberer, retenue) = mpsc::channel();
    let mut sdk = Fd9Simule::avec_un_fd9();
    sdk.deconnexion_retenue = Some(retenue);
    let mut session = Session::new(sdk, Palier::Connexion);
    let (envoi, recu) = mpsc::channel();
    let garde = Garde::new(Duration::from_millis(50), move |reponse| {
        envoi.send(reponse).unwrap();
        // En vrai, le processus s'arrête ici ; le test laisse la DLL simulée
        // rendre la main, pour vérifier que rien d'autre n'est écrit.
        let _ = liberer.send(());
    });
    let mut sortie = Vec::new();
    servir(
        &mut session,
        std::io::Cursor::new(
            requetes
                .join(
                    "
",
                )
                .into_bytes(),
        ),
        &mut sortie,
        garde,
    )
    .unwrap();
    let lignes = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    (lignes, recu.try_iter().collect(), session)
}

#[test]
fn une_deconnexion_bloquee_recoit_quand_meme_une_reponse_a_fermer() {
    let (lignes, garde, session) = dialoguer_deconnexion_bloquee(&[
        r#"{"cmd":"version"}"#,
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
        r#"{"cmd":"fermer"}"#,
    ]);
    // La réponse à `fermer` vient de la garde, une seule fois ; la boucle
    // n'écrit rien de plus quand la DLL finit par rendre la main.
    assert_eq!(lignes.len(), 2, "{lignes:?}");
    assert_eq!(
        garde,
        [Some(Reponse::FermetureIncertaine {
            remise_au_repos: RemiseAuRepos::DeconnexionBloquee {},
        })]
    );
    assert!(
        session
            .journal()
            .iter()
            .any(|l| l.contains("après le délai")),
        "{:?}",
        session.journal()
    );
}

#[test]
fn une_deconnexion_bloquee_a_la_fin_de_l_entree_arrete_aussi_le_pont() {
    let (lignes, garde, _) = dialoguer_deconnexion_bloquee(&[
        r#"{"cmd":"version"}"#,
        r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
    ]);
    assert_eq!(lignes.len(), 2, "{lignes:?}");
    // Personne n'a demandé la fermeture : aucune réponse à écrire.
    assert_eq!(garde, [None]);
}

#[test]
fn une_fermeture_rapide_ne_reveille_pas_la_garde() {
    let (envoi, recu) = mpsc::channel();
    let mut session = Session::new(Fd9Simule::avec_un_fd9(), Palier::Connexion);
    let mut sortie = Vec::new();
    servir(
        &mut session,
        std::io::Cursor::new(
            [
                r#"{"cmd":"version"}"#,
                r#"{"cmd":"connecter_adresse","adresse":"192.0.2.40"}"#,
                r#"{"cmd":"fermer"}"#,
            ]
            .join(
                "
",
            )
            .into_bytes(),
        ),
        &mut sortie,
        Garde::new(Duration::from_millis(200), move |reponse| {
            envoi.send(reponse).unwrap()
        }),
    )
    .unwrap();
    let lignes: Vec<String> = String::from_utf8(sortie)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(lire_reponse(&lignes[2]), Ok(Reponse::Ferme {}));
    std::thread::sleep(Duration::from_millis(300));
    assert!(recu.try_recv().is_err());
}

#[test]
fn la_garde_laisse_finir_la_fermeture_et_repond_avant_l_application() {
    // Sortie et déconnexion ont au moins 5 s en plus de l'attente de la fin
    // de session ; l'application attend la réponse à `fermer` 30 s.
    assert!(DELAI_FERMETURE >= ATTENTE_APRES_DECONNEXION + Duration::from_secs(5));
    assert!(DELAI_FERMETURE < Duration::from_secs(30));
}
