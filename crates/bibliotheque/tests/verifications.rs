//! Jeux de tolérances versionnés et vérifications immuables (organisation 7,
//! ticket #41), vus par l'API publique de la bibliothèque. Critères, fiche
//! et calcul y sont du JSON opaque : la bibliothèque ne les lit pas.

use bibliotheque::{
    Bibliotheque, ErreurBibliotheque, EtatVerification, IdCondition, IdMesure, IdProfil,
    IdTolerances, IdVerification, NouveauProfil, NouvelleVerification,
};
use tempfile::TempDir;

fn icc() -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets
}

const CGATS: &str = "CGATS.17\nMEASUREMENT_CONDITION\t\"M1\"\nBEGIN_DATA_FORMAT\nSAMPLE_ID\tRGB_R\tRGB_G\tRGB_B\tLAB_L\tLAB_A\tLAB_B\n\
                     END_DATA_FORMAT\nNUMBER_OF_SETS\t1\nBEGIN_DATA\n1\t255\t255\t255\t95\t0\t2\nEND_DATA\n";

const ATELIER: &str = r#"{"fidelite":{"moyenne":2.0,"p95":4.0,"max":6.0},"prediction":{"moyenne":1.5,"p95":3.0,"max":5.0},"seuil_gamut":1.0}"#;

struct Banc {
    _dossier: TempDir,
    b: Bibliotheque,
    condition: IdCondition,
    mesure_profil: IdMesure,
    seconde: IdMesure,
    profil: IdProfil,
}

/// Une résolution avec deux mesures importées et un profil calculé depuis
/// la première.
fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let epson = b.creer_imprimante("Epson SC-P5300").unwrap();
    let papier = b.creer_papier(epson.id, "Semigloss 250").unwrap();
    let condition = b.creer_reglage(papier.id, "1440 dpi").unwrap().id;
    let mesure_profil = b.importer_mesure(condition, "profil.txt", CGATS).unwrap();
    let seconde = b.importer_mesure(condition, "seconde.txt", CGATS).unwrap();
    let profil = b
        .enregistrer_profil(&NouveauProfil {
            condition,
            nom: "Epson_Semigloss",
            icc: &icc(),
            horodatage: "2026-10-28T09:00:00Z",
            parametres: r#"{"origine":"calcul"}"#,
            mesures: &[mesure_profil],
        })
        .unwrap();
    Banc {
        _dossier: dossier,
        b,
        condition,
        mesure_profil,
        seconde,
        profil,
    }
}

impl Banc {
    fn verifier(
        &self,
        tolerances: IdTolerances,
        horodatage: &str,
        etat: EtatVerification,
        recalcul_de: Option<IdVerification>,
    ) -> Result<IdVerification, ErreurBibliotheque> {
        self.b.enregistrer_verification(&NouvelleVerification {
            profil: self.profil,
            mesure: self.seconde,
            tolerances,
            horodatage,
            fiche: r#"{"logiciel":"Essai"}"#,
            calcul: r#"{"resultats":[]}"#,
            etat,
            recalcul_de,
        })
    }

    fn etat_profil(&self) -> EtatVerification {
        self.b
            .historique_profils(self.condition)
            .unwrap()
            .into_iter()
            .find(|p| p.id == self.profil)
            .unwrap()
            .etat
    }
}

#[test]
fn un_jeu_se_range_en_version_1_et_se_relit() {
    let banc = banc();
    let id = banc
        .b
        .enregistrer_tolerances("  Atelier ", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    let jeu = banc.b.jeu_tolerances(id).unwrap();
    assert_eq!(
        (jeu.nom.as_str(), jeu.version, jeu.criteres.as_str()),
        ("Atelier", 1, ATELIER)
    );
    assert_eq!(jeu.horodatage, "2026-10-28T09:30:00Z");
    assert!(!jeu.utilise);
    assert_eq!(banc.b.jeux_tolerances().unwrap(), vec![jeu]);
}

#[test]
fn modifier_un_jeu_cree_une_version_sans_toucher_l_ancienne() {
    let banc = banc();
    let v1 = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    let avant = banc.b.jeu_tolerances(v1).unwrap();
    let v2 = banc
        .b
        .enregistrer_tolerances("Atelier", r#"{"autre":1}"#, "2026-10-29T09:30:00Z")
        .unwrap();
    let autre = banc
        .b
        .enregistrer_tolerances("Client", ATELIER, "2026-10-29T10:00:00Z")
        .unwrap();
    assert_eq!(banc.b.jeu_tolerances(v2).unwrap().version, 2);
    assert_eq!(banc.b.jeu_tolerances(v1).unwrap(), avant);
    let jeux: Vec<_> = banc
        .b
        .jeux_tolerances()
        .unwrap()
        .into_iter()
        .map(|j| (j.id, j.nom, j.version))
        .collect();
    assert_eq!(
        jeux,
        [
            (v2, "Atelier".to_string(), 2),
            (v1, "Atelier".to_string(), 1),
            (autre, "Client".to_string(), 1)
        ]
    );
}

#[test]
fn un_nom_de_jeu_vide_ou_une_date_illisible_sont_refuses() {
    let banc = banc();
    assert_eq!(
        banc.b
            .enregistrer_tolerances("  ", ATELIER, "2026-10-28T09:30:00Z"),
        Err(ErreurBibliotheque::NomVide)
    );
    assert!(matches!(
        banc.b.enregistrer_tolerances("Atelier", ATELIER, "hier"),
        Err(ErreurBibliotheque::HorodatageInvalide(_))
    ));
    assert_eq!(
        banc.b.jeu_tolerances(IdTolerances(99)),
        Err(ErreurBibliotheque::ToleranceInconnue(IdTolerances(99)))
    );
    assert!(banc.b.jeux_tolerances().unwrap().is_empty());
}

#[test]
fn un_jeu_utilise_le_dit() {
    let banc = banc();
    let t = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    banc.verifier(t, "2026-10-28T10:00:00Z", EtatVerification::Accepte, None)
        .unwrap();
    assert!(banc.b.jeu_tolerances(t).unwrap().utilise);
}

#[test]
fn une_verification_donne_son_etat_au_profil() {
    let banc = banc();
    let t = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    let v = banc
        .verifier(
            t,
            "2026-10-28T10:00:00Z",
            EtatVerification::HorsTolerance,
            None,
        )
        .unwrap();
    assert_eq!(banc.etat_profil(), EtatVerification::HorsTolerance);
    let actif = banc.b.profil_actif(banc.condition).unwrap().unwrap();
    assert_eq!(actif.id, banc.profil, "toujours actif");
    let rangee = banc.b.verification(v).unwrap();
    assert_eq!(rangee.profil, banc.profil);
    assert_eq!(rangee.mesure, banc.seconde);
    assert_eq!(rangee.tolerances.id, t);
    assert_eq!(rangee.fiche, r#"{"logiciel":"Essai"}"#);
    assert_eq!(rangee.calcul, r#"{"resultats":[]}"#);
    assert_eq!(rangee.etat, EtatVerification::HorsTolerance);
    assert_eq!(rangee.recalcul_de, None);
}

#[test]
fn un_recalcul_s_ajoute_et_garde_l_original() {
    let banc = banc();
    let t1 = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    let t2 = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T11:00:00Z")
        .unwrap();
    let premiere = banc
        .verifier(
            t1,
            "2026-10-28T10:00:00Z",
            EtatVerification::HorsTolerance,
            None,
        )
        .unwrap();
    let avant = banc.b.verification(premiere).unwrap();
    let recalcul = banc
        .verifier(
            t2,
            "2026-10-28T11:05:00Z",
            EtatVerification::Accepte,
            Some(premiere),
        )
        .unwrap();
    let toutes = banc.b.verifications(banc.profil).unwrap();
    assert_eq!(
        toutes.iter().map(|v| v.id).collect::<Vec<_>>(),
        [recalcul, premiere]
    );
    assert_eq!(toutes[0].recalcul_de, Some(premiere));
    assert_eq!(toutes[0].tolerances.version, 2);
    let mut avant_relue = avant.clone();
    avant_relue.tolerances.utilise = true;
    assert_eq!(banc.b.verification(premiere).unwrap(), avant_relue);
    assert_eq!(banc.etat_profil(), EtatVerification::Accepte);
}

#[test]
fn la_base_refuse_de_modifier_une_verification() {
    let banc = banc();
    let t = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    banc.verifier(t, "2026-10-28T10:00:00Z", EtatVerification::Accepte, None)
        .unwrap();
    let base =
        rusqlite::Connection::open(banc._dossier.path().join(bibliotheque::FICHIER_BASE)).unwrap();
    for requete in [
        "UPDATE verifications SET etat = 'hors_tolerance'",
        "DELETE FROM verifications",
        "UPDATE tolerances SET criteres = '{}'",
        "DELETE FROM tolerances",
    ] {
        assert!(base.execute(requete, []).is_err(), "{requete}");
    }
}

#[test]
fn non_verifie_n_est_pas_un_resultat() {
    let banc = banc();
    let t = banc
        .b
        .enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    assert_eq!(
        banc.verifier(
            t,
            "2026-10-28T10:00:00Z",
            EtatVerification::NonVerifie,
            None
        ),
        Err(ErreurBibliotheque::EtatVerificationInvalide)
    );
    assert!(banc.b.verifications(banc.profil).unwrap().is_empty());
    assert_eq!(
        banc.verifier(
            IdTolerances(99),
            "2026-10-28T10:00:00Z",
            EtatVerification::Accepte,
            None
        ),
        Err(ErreurBibliotheque::ToleranceInconnue(IdTolerances(99)))
    );
    assert_eq!(
        banc.b.verification(IdVerification(42)),
        Err(ErreurBibliotheque::VerificationInconnue(IdVerification(42)))
    );
    assert_eq!(banc.etat_profil(), EtatVerification::NonVerifie);
}

/// Une base de l'organisation 5 (avant les mires #33 et les vérifications)
/// passe à l'organisation 7 sans rien perdre : profil, état, mesures.
#[test]
fn une_base_de_l_organisation_precedente_passe_sans_rien_perdre() {
    let banc = banc();
    let dossier = banc._dossier;
    let (profil, condition) = (banc.profil, banc.condition);
    drop(banc.b);
    let chemin = dossier.path().join(bibliotheque::FICHIER_BASE);
    let base = rusqlite::Connection::open(&chemin).unwrap();
    // Retour à l'organisation 5 : rien de ce que l'organisation 7 ajoute.
    base.execute_batch(
        "DROP TRIGGER tolerances_immuables; DROP TRIGGER tolerances_gardees;
         DROP TRIGGER verifications_immuables; DROP TRIGGER verifications_gardees;
         DROP TABLE verifications; DROP TABLE tolerances; DROP TABLE mires_verification;
         PRAGMA user_version = 5;",
    )
    .unwrap();
    drop(base);

    let b = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let actif = b.profil_actif(condition).unwrap().unwrap();
    assert_eq!(
        (actif.id, actif.etat),
        (profil, EtatVerification::NonVerifie)
    );
    assert_eq!(b.fichier_profil(profil).unwrap(), icc());
    assert!(b.verifications(profil).unwrap().is_empty());
    b.enregistrer_tolerances("Atelier", ATELIER, "2026-10-28T09:30:00Z")
        .unwrap();
    drop(b);
    let base = rusqlite::Connection::open(&chemin).unwrap();
    let version: i32 = base
        .pragma_query_value(None, "user_version", |l| l.get(0))
        .unwrap();
    assert_eq!(version, 7);
}

/// Une mire de vérification est rangée une fois par contenu : la ranger de
/// nouveau rend le même numéro, sans doublon.
#[test]
fn une_mire_de_verification_se_range_une_seule_fois() {
    let banc = banc();
    let xml = "<?xml version=\"1.0\"?><Data/>";
    let premiere = banc
        .b
        .ranger_mire_verification("Mire v1", "fd9", 1, xml, "2026-10-28T09:00:00Z")
        .unwrap();
    let encore = banc
        .b
        .ranger_mire_verification("Mire v1", "fd9", 1, xml, "2026-10-29T09:00:00Z")
        .unwrap();
    assert_eq!(premiere, encore);
    assert_eq!(
        banc.b.mire_verification("fd9", 1).unwrap(),
        Some(premiere.clone())
    );
    assert_eq!(banc.b.mire_verification("fd9", 2).unwrap(), None);
    let mires = banc.b.mires_verification().unwrap();
    assert_eq!(mires.len(), 1);
    let m = &mires[0];
    assert_eq!(
        (
            m.nom.as_str(),
            m.instrument.as_str(),
            m.version,
            m.contenu.as_str()
        ),
        ("Mire v1", "fd9", 1, xml)
    );
    assert_eq!(m.horodatage, "2026-10-28T09:00:00Z");
    assert_eq!(m.empreinte.len(), 64);
}

#[test]
fn les_mesures_d_un_profil_calcule_se_relisent() {
    let banc = banc();
    assert_eq!(
        banc.b.mesures_profil(banc.profil).unwrap(),
        [banc.mesure_profil]
    );
    assert_eq!(
        banc.b.condition_profil(banc.profil).unwrap(),
        banc.condition
    );
    assert_eq!(
        banc.b.mesures_profil(IdProfil(99)),
        Err(ErreurBibliotheque::ProfilInconnu(IdProfil(99)))
    );
}
