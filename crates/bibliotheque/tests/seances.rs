//! Séances de mesure et leurs mesures sources (organisation 8, ticket #35),
//! vues par l'API publique de la bibliothèque. Plan, fiche, association et
//! composition y sont du JSON opaque : la bibliothèque ne les lit pas.

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, EtatSeance, EtatVerification, FinSeance, IdCondition,
    IdMesure, IdSeance, IdTirage, NouveauProfil, NouvelleMire, NouvelleSeance, NouvelleSource,
    NouvelleVerification, OrigineSource, PlaceEnSeance, RoleSource,
};
use tempfile::TempDir;

struct Banc {
    dossier: TempDir,
    b: Bibliotheque,
    condition: IdCondition,
    tirage: IdTirage,
}

fn mire_fd9(empreinte: &str) -> NouvelleMire<'_> {
    NouvelleMire {
        nom: "RGB-2520",
        instrument: "fd9",
        plages: 1,
        generation: "{}",
        valeurs: "[[100.0,100.0,100.0]]",
        mise_en_page: "{}",
        empreinte,
        horodatage: "2026-11-02T09:00:00Z",
    }
}

/// Une résolution et un tirage de mire FD-9.
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let epson = b.creer_imprimante("Epson SC-P5300").unwrap();
    let papier = b.creer_papier(epson.id, "Semigloss 250").unwrap();
    let condition = b.creer_reglage(papier.id, "1440 dpi").unwrap().id;
    let (_, tirage) = b
        .enregistrer_mire_et_tirage(condition, &mire_fd9("fd9-essai"), 1)
        .unwrap();
    Banc {
        dossier,
        b,
        condition,
        tirage,
    }
}

impl Banc {
    fn commencer(&self) -> IdSeance {
        self.b
            .creer_seance(&NouvelleSeance {
                condition: self.condition,
                tirage: self.tirage,
                plan: r#"{"plan":1}"#,
                fiche: r#"{"confirmee":true}"#,
                horodatage: "2026-11-02T10:00:00Z",
            })
            .unwrap()
    }

    fn ajouter(&self, seance: IdSeance, fichier: &str, origine: OrigineSource) -> IdMesure {
        self.b
            .ajouter_source(
                seance,
                &NouvelleSource {
                    fichier,
                    texte: CGATS,
                    origine,
                    role: RoleSource::Exemplaire,
                    association: r#"{"certaine":true}"#,
                    horodatage: "2026-11-02T10:05:00Z",
                },
            )
            .unwrap()
    }

    fn rouvrir(self) -> Banc {
        let Banc {
            dossier,
            b,
            condition,
            tirage,
        } = self;
        drop(b);
        let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
        Banc {
            dossier,
            b,
            condition,
            tirage,
        }
    }
}

fn fin(etat: EtatSeance, resultat: Option<(&'static str, &'static str)>) -> FinSeance<'static> {
    FinSeance {
        etat,
        horodatage: "2026-11-02T11:00:00Z",
        resultat,
        composition: r#"{"composition":[]}"#,
    }
}

#[test]
fn une_seance_et_ses_sources_survivent_a_la_fermeture() {
    let banc = banc();
    let seance = banc.commencer();
    let premiere = banc.ajouter(
        seance,
        "C:/mesures/feuille 1.txt",
        OrigineSource::Acquisition,
    );
    let seconde = banc.ajouter(seance, "export FD-S2w.txt", OrigineSource::Import);
    let banc = banc.rouvrir();
    let s = banc.b.seance(seance).unwrap();
    assert_eq!(s.condition, banc.condition);
    assert_eq!(s.tirage, banc.tirage);
    assert_eq!(s.plan, r#"{"plan":1}"#);
    assert_eq!(s.fiche, r#"{"confirmee":true}"#);
    assert_eq!(s.etat, EtatSeance::EnCours);
    assert_eq!(s.fin, None);
    let mesures: Vec<_> = s.sources.iter().map(|x| (x.mesure, x.origine)).collect();
    assert_eq!(
        mesures,
        [
            (premiere, OrigineSource::Acquisition),
            (seconde, OrigineSource::Import)
        ]
    );
    assert_eq!(s.sources[0].fichier, "feuille 1.txt");
    assert_eq!(s.sources[0].association, r#"{"certaine":true}"#);
    assert!(s.sources[0].retenue);
    assert_eq!(banc.b.seances_en_cours(banc.condition).unwrap(), [seance]);
}

#[test]
fn une_source_est_une_mesure_importee_gardee_telle_quelle() {
    let banc = banc();
    let seance = banc.commencer();
    let mesure = banc.ajouter(seance, "export.txt", OrigineSource::Import);
    let importee = banc.b.mesure_importee(mesure).unwrap();
    assert_eq!(importee.condition, banc.condition);
    assert_eq!(importee.fichier, "export.txt");
    assert_eq!(banc.b.texte_importee(mesure).unwrap(), CGATS);
}

#[test]
fn une_source_illisible_n_est_pas_rangee() {
    let banc = banc();
    let seance = banc.commencer();
    let refus = banc
        .b
        .ajouter_source(
            seance,
            &NouvelleSource {
                fichier: "abime.txt",
                texte: "pas un CGATS",
                origine: OrigineSource::Import,
                role: RoleSource::Exemplaire,
                association: "{}",
                horodatage: "2026-11-02T10:05:00Z",
            },
        )
        .unwrap_err();
    assert!(matches!(refus, ErreurBibliotheque::ImportIllisible(_)));
    assert!(banc.b.seance(seance).unwrap().sources.is_empty());
}

#[test]
fn un_tirage_d_une_autre_resolution_est_refuse() {
    let banc = banc();
    let epson = banc.b.creer_imprimante("Canon PRO-4100").unwrap();
    let papier = banc.b.creer_papier(epson.id, "Photo Rag").unwrap();
    let autre = banc.b.creer_reglage(papier.id, "720 dpi").unwrap().id;
    let refus = banc
        .b
        .creer_seance(&NouvelleSeance {
            condition: autre,
            tirage: banc.tirage,
            plan: "{}",
            fiche: "{}",
            horodatage: "2026-11-02T10:00:00Z",
        })
        .unwrap_err();
    assert_eq!(refus, ErreurBibliotheque::TirageInconnu(banc.tirage));
}

#[test]
fn une_exclusion_garde_son_motif() {
    let banc = banc();
    let seance = banc.commencer();
    let mesure = banc.ajouter(seance, "feuille.txt", OrigineSource::Acquisition);
    banc.b
        .retenir_source(
            seance,
            mesure,
            false,
            Some(r#"{"motif":"feuille froissée"}"#),
        )
        .unwrap();
    let s = banc.b.seance(seance).unwrap();
    assert!(!s.sources[0].retenue);
    assert_eq!(
        s.sources[0].selection.as_deref(),
        Some(r#"{"motif":"feuille froissée"}"#)
    );
    assert_eq!(
        banc.b.retenir_source(seance, IdMesure(999), true, None),
        Err(ErreurBibliotheque::MesureInconnue(IdMesure(999)))
    );
}

#[test]
fn les_complements_sont_gardes_dans_la_seance() {
    let banc = banc();
    let seance = banc.commencer();
    assert_eq!(banc.b.seance(seance).unwrap().complements, "[]");
    banc.b
        .changer_complements(seance, r#"[{"plage":"3"}]"#)
        .unwrap();
    assert_eq!(
        banc.b.seance(seance).unwrap().complements,
        r#"[{"plage":"3"}]"#
    );
}

#[test]
fn une_seance_terminee_range_son_resultat_et_ne_change_plus() {
    let banc = banc();
    let seance = banc.commencer();
    let source = banc.ajouter(seance, "feuille.txt", OrigineSource::Acquisition);
    let resultat = banc
        .b
        .terminer_seance(
            seance,
            &fin(EtatSeance::Terminee, Some(("Moyenne séance 1.txt", CGATS))),
        )
        .unwrap()
        .unwrap();
    let s = banc.b.seance(seance).unwrap();
    assert_eq!(s.etat, EtatSeance::Terminee);
    assert_eq!(s.fin.as_deref(), Some("2026-11-02T11:00:00Z"));
    assert_eq!(s.resultat, Some(resultat));
    assert_eq!(s.composition.as_deref(), Some(r#"{"composition":[]}"#));
    assert_eq!(
        banc.b.mesure_importee(resultat).unwrap().fichier,
        "Moyenne séance 1.txt"
    );
    assert!(banc.b.seances_en_cours(banc.condition).unwrap().is_empty());

    let fermee = Err(ErreurBibliotheque::SeanceFermee(seance));
    assert_eq!(
        banc.b
            .terminer_seance(seance, &fin(EtatSeance::Terminee, None))
            .map(|_| ()),
        fermee
    );
    assert_eq!(banc.b.retenir_source(seance, source, false, None), fermee);
    assert_eq!(banc.b.changer_complements(seance, "[]"), fermee);
    assert_eq!(
        banc.b
            .ajouter_source(
                seance,
                &NouvelleSource {
                    fichier: "tard.txt",
                    texte: CGATS,
                    origine: OrigineSource::Import,
                    role: RoleSource::Exemplaire,
                    association: "{}",
                    horodatage: "2026-11-02T12:00:00Z",
                },
            )
            .map(|_| ()),
        fermee
    );
}

#[test]
fn une_seance_ne_se_termine_pas_en_cours() {
    let banc = banc();
    let seance = banc.commencer();
    assert!(banc
        .b
        .terminer_seance(seance, &fin(EtatSeance::EnCours, None))
        .is_err());
    assert_eq!(banc.b.seance(seance).unwrap().etat, EtatSeance::EnCours);
}

#[test]
fn une_seance_inconnue_est_dite() {
    let banc = banc();
    assert_eq!(
        banc.b.seance(IdSeance(42)).map(|_| ()),
        Err(ErreurBibliotheque::SeanceInconnue(IdSeance(42)))
    );
}

#[test]
fn la_place_d_une_mesure_dans_une_seance_se_retrouve() {
    let banc = banc();
    let seance = banc.commencer();
    let source = banc.ajouter(seance, "feuille.txt", OrigineSource::Acquisition);
    let hors_seance = banc
        .b
        .importer_mesure(banc.condition, "seule.txt", CGATS)
        .unwrap();
    let situation = banc.b.situation_en_seance(source).unwrap().unwrap();
    assert_eq!(situation.seance, seance);
    assert_eq!(situation.etat, EtatSeance::EnCours);
    assert_eq!(
        situation.place,
        PlaceEnSeance::Source {
            association: r#"{"certaine":true}"#.into()
        }
    );
    assert_eq!(banc.b.situation_en_seance(hors_seance).unwrap(), None);
    let resultat = banc
        .b
        .terminer_seance(seance, &fin(EtatSeance::Incomplete, Some(("m.txt", CGATS))))
        .unwrap()
        .unwrap();
    let situation = banc.b.situation_en_seance(resultat).unwrap().unwrap();
    assert_eq!(situation.etat, EtatSeance::Incomplete);
    assert_eq!(situation.place, PlaceEnSeance::Resultat);
}

#[test]
fn un_tirage_mesure_n_est_plus_un_tirage_prepare() {
    let banc = banc();
    let seance = banc.commencer();
    // En cours : le tirage reste à reprendre.
    assert_eq!(
        banc.b
            .tirages_prepares(banc.condition, "fd9")
            .unwrap()
            .len(),
        1
    );
    banc.b
        .terminer_seance(seance, &fin(EtatSeance::Terminee, Some(("m.txt", CGATS))))
        .unwrap();
    assert!(banc
        .b
        .tirages_prepares(banc.condition, "fd9")
        .unwrap()
        .is_empty());
    // La même mire préparée à nouveau donne un nouveau tirage.
    let (_, nouveau) = banc
        .b
        .enregistrer_mire_et_tirage(banc.condition, &mire_fd9("fd9-essai"), 1)
        .unwrap();
    assert_ne!(nouveau, banc.tirage);
    let prepares = banc.b.tirages_prepares(banc.condition, "fd9").unwrap();
    assert_eq!(prepares.iter().map(|t| t.id).collect::<Vec<_>>(), [nouveau]);
}

fn organisation(chemin: &std::path::Path) -> i32 {
    rusqlite::Connection::open(chemin)
        .unwrap()
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap()
}

/// Une bibliothèque en organisation 7 passe à la 8 sans rien perdre : ses
/// tirages restent préparés, son profil et sa vérification restent, et une
/// séance se commence sur son tirage.
#[test]
fn une_base_d_organisation_7_passe_a_la_8_sans_rien_perdre() {
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    std::fs::copy(FIXTURE_ORGANISATION_7, &chemin).unwrap();
    assert_eq!(organisation(&chemin), 7);

    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    assert_eq!(organisation(&chemin), 8);
    let condition = b.conditions().unwrap()[0].id;
    let prepares = b.tirages_prepares(condition, "fd9").unwrap();
    assert_eq!(prepares.len(), 1);
    assert_eq!(prepares[0].mire.empreinte, "fd9-essai");
    let profil = b.profil_actif(condition).unwrap().unwrap();
    assert_eq!(b.fichier_profil(profil.id).unwrap(), icc());
    assert_eq!(b.verifications(profil.id).unwrap().len(), 1);
    assert_eq!(b.jeux_tolerances().unwrap().len(), 1);

    let seance = b
        .creer_seance(&NouvelleSeance {
            condition,
            tirage: prepares[0].id,
            plan: "{}",
            fiche: "{}",
            horodatage: "2026-11-02T10:00:00Z",
        })
        .unwrap();
    assert_eq!(b.seances_en_cours(condition).unwrap(), [seance]);
    drop(b);
    let base = rusqlite::Connection::open(&chemin).unwrap();
    let mut requete = base.prepare("PRAGMA foreign_key_check").unwrap();
    assert!(requete.query([]).unwrap().next().unwrap().is_none());
}

fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

const CGATS: &str = "CGATS.17\nMEASUREMENT_CONDITION\t\"M1\"\nBEGIN_DATA_FORMAT\nSAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\n\
                     END_DATA_FORMAT\nNUMBER_OF_SETS\t1\nBEGIN_DATA\n1\t255\t255\t255\t95\t0\t2\nEND_DATA\n";

const ATELIER: &str = r#"{"fidelite":{"moyenne":2.0,"p95":4.0,"max":6.0}}"#;

/// Base d'organisation 7 figée, produite une fois par
/// [`produire_fixture_organisation_7`] avec le code de `dev` d'avant #35
/// (PR2) : une résolution, une mesure importée, un profil vérifié, un jeu de
/// tolérances et une mire FD-9 avec un tirage.
const FIXTURE_ORGANISATION_7: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/donnees/organisation-7.sqlite"
);

/// A servi une seule fois, avec la bibliothèque de `dev` encore en
/// organisation 7 (après #41, avant les séances de #35), à produire
/// `tests/donnees/organisation-7.sqlite`. Ne plus le relancer : il écrirait
/// une base de l'organisation d'aujourd'hui.
#[test]
#[ignore]
fn produire_fixture_organisation_7() {
    let dossier = tempfile::tempdir().unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let epson = b.creer_imprimante("Epson SC-P5300").unwrap();
    let papier = b.creer_papier(epson.id, "Semigloss 250").unwrap();
    let condition = b.creer_reglage(papier.id, "1440 dpi").unwrap().id;
    let mesure = b.importer_mesure(condition, "profil.txt", CGATS).unwrap();
    let profil = b
        .enregistrer_profil(&NouveauProfil {
            condition,
            nom: "Epson_Semigloss",
            icc: &icc(),
            horodatage: "2026-10-28T09:00:00Z",
            parametres: r#"{"origine":"calcul"}"#,
            mesures: &[mesure],
        })
        .unwrap();
    let tolerances = b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    b.enregistrer_verification(&NouvelleVerification {
        profil,
        mesure,
        tolerances,
        horodatage: "2026-10-28T10:00:00Z",
        fiche: "{}",
        calcul: "{}",
        etat: EtatVerification::Accepte,
        recalcul_de: None,
    })
    .unwrap();
    b.enregistrer_mire_et_tirage(
        condition,
        &NouvelleMire {
            nom: "RGB-2520",
            instrument: "fd9",
            plages: 2,
            generation: r#"{"origine":"reprise","fichier":"RGB-2520 A3 - 1p.tif","nom":"RGB-2520"}"#,
            valeurs: "[[0.0,0.0,0.0],[100.0,100.0,100.0]]",
            mise_en_page: r#"{"colonnes":2,"rangees":1}"#,
            empreinte: "fd9-essai",
            horodatage: "2026-10-28T11:00:00Z",
        },
        1,
    )
    .unwrap();
    drop(b);
    let cible = std::path::Path::new(FIXTURE_ORGANISATION_7);
    std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
    std::fs::copy(dossier.path().join(bibliotheque::FICHIER_BASE), cible).unwrap();
}
