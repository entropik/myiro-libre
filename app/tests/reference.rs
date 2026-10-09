//! Couleur de référence et écart ΔE00 dans la tâche Mesurer (ticket #8) :
//! une mesure de la séance est désignée couleur de référence, les autres
//! affichent leur écart à elle et un verdict selon le seuil choisi, conservé
//! avec la référence dans la bibliothèque.
//!
//! Les mesures sont des gris fictifs (réflectance constante) : leur Lab se
//! calcule à la main, L* = 116 r^(1/3) − 16, et leur ΔE00 aussi (a* = b* = 0,
//! seul le terme de clarté reste, S_L ≈ 1 autour de L* = 50).

use app::mesurer::{
    Comparaison, FicheMesure, ReferenceReprise, ReferencesConservees, Seance, VerdictEcart,
};
use app::textes::Langue;
use bibliotheque::{Bibliotheque, ConditionImpression, IdMesure, ReferenceCouleur};
use pont_protocole::{
    Calcul, ConditionMesure, ConditionsCalcul, DonneesBrutes, Echantillonnage, Geometrie,
    Horodatage, Illuminant, Info, InstrumentMesurant, Lab, Mesure, Observateur, Plage, Provenance,
    Spectre,
};

use ConditionMesure::{M0, M1, M2};

fn confirmees(conditions: [ConditionMesure; 3]) -> Info<ConditionsCalcul> {
    Info::Confirmee(ConditionsCalcul {
        conditions_spectres: conditions.map(Info::Confirmee),
        longueurs_onde: Info::Confirmee(Echantillonnage {
            debut_nm: 380,
            pas_nm: 10,
        }),
        illuminant_lab: Info::Confirmee(Illuminant::D50),
        observateur_lab: Info::Supposee(Observateur::DeuxDegres),
    })
}

/// Gris de réflectance `r` (mêmes trois spectres), numéro de série fictif.
fn gris(r: f32, demande: Info<ConditionsCalcul>) -> Mesure {
    gris_spectres([r, r, r], demande)
}

fn gris_spectres(r: [f32; 3], demande: Info<ConditionsCalcul>) -> Mesure {
    let lab = || Lab::new([0.0, 0.0, 0.0]).unwrap();
    let plage = Plage::new(
        r.map(|r| Spectre::new(vec![r; 36]).unwrap()),
        DonneesBrutes::new(vec![1.0; 152]).unwrap(),
        [lab(), lab(), lab()],
    )
    .unwrap();
    let provenance = Provenance {
        instrument: InstrumentMesurant {
            modele: "MYIRO-1".into(),
            numero_serie: 12345678,
            micrologiciel: "1.00".into(),
            code_produit: "simule".into(),
        },
        version_sdk: [1, 0, 1],
        empreinte_dll: Info::Inconnue,
        version_pont: "simulé".into(),
        architecture: "x86_64".into(),
        horodatage: Horodatage::new("2026-10-09T09:31:00+02:00").unwrap(),
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

fn bibliotheque() -> (tempfile::TempDir, Bibliotheque, ConditionImpression) {
    let dossier = tempfile::tempdir().unwrap();
    let biblio = Bibliotheque::ouvrir(dossier.path()).unwrap();
    let condition = biblio.creer_condition("Offset, couché mat").unwrap();
    (dossier, biblio, condition)
}

/// Ajoute une mesure à la séance, rangée dans la bibliothèque.
fn ajouter(
    seance: &mut Seance,
    biblio: &Bibliotheque,
    c: &ConditionImpression,
    m: Mesure,
) -> usize {
    seance.ajouter(
        m,
        c,
        |mesure, nom| {
            biblio
                .enregistrer_mesure_nommee(c.id, mesure, nom)
                .map_err(|e| e.to_string())
        },
        Langue::Francais,
    )
}

fn fiche(seance: &Seance, numero: usize, langue: Langue) -> FicheMesure {
    seance
        .fiches(langue)
        .into_iter()
        .find(|f| f.numero == numero)
        .unwrap()
}

/// Séance de trois gris : 0,18 (référence), 0,185 et 0,20.
fn trois_gris() -> (tempfile::TempDir, Bibliotheque, Seance, [usize; 3]) {
    let (dossier, biblio, c) = bibliotheque();
    let mut seance = Seance::default();
    let demande = || confirmees([M0, M1, M2]);
    let n = [0.18, 0.185, 0.20].map(|r| ajouter(&mut seance, &biblio, &c, gris(r, demande())));
    (dossier, biblio, seance, n)
}

#[test]
fn sans_reference_aucune_mesure_n_a_d_ecart() {
    let (_d, _biblio, seance, _) = trois_gris();
    assert_eq!(seance.reference(Langue::Francais), None);
    for f in seance.fiches(Langue::Francais) {
        assert!(!f.reference);
        assert!(f.spectres.iter().all(|s| s.ecart.is_none()));
    }
}

#[test]
fn les_autres_mesures_affichent_leur_ecart_a_la_reference() {
    let (_d, biblio, mut seance, [a, b, c]) = trois_gris();
    seance.designer_reference(a, &biblio).unwrap();

    let reference = fiche(&seance, a, Langue::Francais);
    assert!(reference.reference);
    assert!(reference.spectres.iter().all(|s| s.ecart.is_none()));

    // ΔL* = 50,10 − 49,50 et 51,84 − 49,50, à la main.
    for (numero, attendu) in [(b, "0,60"), (c, "2,34")] {
        for s in fiche(&seance, numero, Langue::Francais).spectres {
            let e = s.ecart.expect("écart à la référence");
            assert_eq!(e.delta_e00.as_deref(), Some(attendu));
            // Deux gris : ni écart de saturation ni écart de teinte.
            assert_eq!(e.delta_c.as_deref(), Some("0,00"));
            assert_eq!(e.delta_h.as_deref(), Some("0,00"));
            assert_eq!(e.comparaison, Comparaison::MemeCondition);
            // Aucun seuil fixé : pas de verdict.
            assert_eq!(e.verdict, VerdictEcart::SeuilNonFixe);
        }
    }
    let en = fiche(&seance, c, Langue::Anglais).spectres[0]
        .ecart
        .clone()
        .unwrap();
    assert_eq!(en.delta_e00.as_deref(), Some("2.34"));
}

#[test]
fn le_verdict_suit_le_seuil_conserve_avec_la_reference() {
    let (_d, biblio, mut seance, [a, b, c]) = trois_gris();
    seance.designer_reference(a, &biblio).unwrap();
    seance.regler_seuil(" 1,4 ", &biblio).unwrap();

    let verdict = |s: &Seance, n| {
        fiche(s, n, Langue::Francais).spectres[1]
            .ecart
            .clone()
            .unwrap()
            .verdict
    };
    assert_eq!(verdict(&seance, b), VerdictEcart::Conforme);
    assert_eq!(verdict(&seance, c), VerdictEcart::HorsTolerance);
    assert_eq!(
        seance
            .reference(Langue::Francais)
            .map(|r| (r.numero, r.seuil)),
        Some((a, Some("1,40".to_string())))
    );
    // Le seuil est conservé avec la référence, dans la bibliothèque.
    assert_eq!(biblio.references().unwrap()[0].seuil, Some(1.4));

    // 0,8 × 0,7 = 0,56 ≤ 0,60 ≤ 0,7 : proche de la limite.
    seance.regler_seuil("0.7", &biblio).unwrap();
    assert_eq!(verdict(&seance, b), VerdictEcart::ProcheDeLaLimite);

    // Seuil effacé : plus de verdict, et rien à la place dans la bibliothèque.
    seance.regler_seuil("", &biblio).unwrap();
    assert_eq!(verdict(&seance, b), VerdictEcart::SeuilNonFixe);
    assert_eq!(biblio.references().unwrap()[0].seuil, None);
}

/// Le verdict porte sur l'écart affiché (deux décimales) : jamais « hors
/// tolérance » pour un écart écrit égal au seuil.
#[test]
fn le_verdict_porte_sur_l_ecart_affiche() {
    let (_d, biblio, c) = bibliotheque();
    let mut seance = Seance::default();
    let a = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.18, confirmees([M0, M1, M2])),
    );
    let n = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.19, confirmees([M0, M1, M2])),
    );
    seance.designer_reference(a, &biblio).unwrap();
    // ΔE00 calculé : 1,1911 ; écrit 1,19.
    seance.regler_seuil("1,19", &biblio).unwrap();
    let e = fiche(&seance, n, Langue::Francais).spectres[0]
        .ecart
        .clone()
        .unwrap();
    assert_eq!(e.delta_e00.as_deref(), Some("1,19"));
    assert_eq!(e.verdict, VerdictEcart::ProcheDeLaLimite);
}

#[test]
fn un_seuil_impossible_est_refuse_sans_rien_changer() {
    let (_d, biblio, mut seance, [a, _, _]) = trois_gris();
    assert_eq!(
        seance.regler_seuil("2", &biblio),
        Err("mesurer.reference.aucune"),
        "pas de seuil sans référence"
    );
    seance.designer_reference(a, &biblio).unwrap();
    seance.regler_seuil("2", &biblio).unwrap();
    for faux in ["0", "-1", "deux", "1,2,3", "inf", "NaN"] {
        assert_eq!(
            seance.regler_seuil(faux, &biblio),
            Err("mesurer.reference.seuil_invalide"),
            "{faux}"
        );
    }
    assert_eq!(
        seance.reference(Langue::Anglais).unwrap().seuil.as_deref(),
        Some("2.00")
    );
    assert_eq!(biblio.references().unwrap()[0].seuil, Some(2.0));
}

#[test]
fn designer_une_autre_reference_retire_la_premiere() {
    let (_d, biblio, mut seance, [a, b, _]) = trois_gris();
    seance.designer_reference(a, &biblio).unwrap();
    seance.regler_seuil("2", &biblio).unwrap();
    // La désigner de nouveau ne perd pas son seuil.
    seance.designer_reference(a, &biblio).unwrap();
    assert_eq!(
        seance.reference(Langue::Francais).unwrap().seuil.as_deref(),
        Some("2,00")
    );
    assert_eq!(biblio.references().unwrap()[0].seuil, Some(2.0));
    seance.designer_reference(b, &biblio).unwrap();

    assert!(!fiche(&seance, a, Langue::Francais).reference);
    assert!(fiche(&seance, b, Langue::Francais).reference);
    // La nouvelle référence part sans seuil : celui de l'autre n'est pas repris.
    assert_eq!(seance.reference(Langue::Francais).unwrap().seuil, None);
    let rangee_b = biblio.arborescence("").unwrap()[0]
        .mesures
        .iter()
        .find(|m| m.nom.as_deref() == Some("Couleur 2"))
        .unwrap()
        .id;
    assert_eq!(
        biblio.references().unwrap(),
        vec![ReferenceCouleur {
            mesure: rangee_b,
            seuil: None
        }]
    );

    seance.retirer_reference(&biblio).unwrap();
    assert_eq!(seance.reference(Langue::Francais), None);
    assert_eq!(biblio.references().unwrap(), vec![]);
    assert!(seance
        .fiches(Langue::Francais)
        .iter()
        .all(|f| !f.reference && f.spectres.iter().all(|s| s.ecart.is_none())));
}

/// Bibliothèque qui refuse tout : la séance ne change pas, l'écran et la
/// bibliothèque disent toujours la même chose.
struct Refus;
impl ReferencesConservees for Refus {
    fn designer(&self, _: IdMesure, _: Option<f64>) -> Result<(), String> {
        Err("base verrouillée".into())
    }
    fn remplacer(&self, _: IdMesure, _: IdMesure) -> Result<(), String> {
        Err("base verrouillée".into())
    }
    fn retirer(&self, _: IdMesure) -> Result<(), String> {
        Err("base verrouillée".into())
    }
    fn reference_conservee(&self) -> Result<Option<ReferenceReprise>, String> {
        Err("base verrouillée".into())
    }
}

#[test]
fn un_refus_de_la_bibliotheque_ne_change_rien() {
    let (_d, biblio, mut seance, [a, _, _]) = trois_gris();
    assert_eq!(
        seance.designer_reference(a, &Refus),
        Err("bibliotheque.erreur.autre")
    );
    assert_eq!(seance.reference(Langue::Francais), None);

    seance.designer_reference(a, &biblio).unwrap();
    assert_eq!(
        seance.regler_seuil("2", &Refus),
        Err("bibliotheque.erreur.autre")
    );
    assert_eq!(seance.reference(Langue::Francais).unwrap().seuil, None);
    assert_eq!(
        seance.retirer_reference(&Refus),
        Err("bibliotheque.erreur.autre")
    );
    assert!(seance.reference(Langue::Francais).is_some());
}

/// Changer de référence se fait d'un bloc : si la bibliothèque refuse,
/// l'ancienne reste la référence, l'écran le dit, et il n'y en a jamais deux.
#[test]
fn un_changement_de_reference_refuse_garde_l_ancienne() {
    let (_d, biblio, mut seance, [a, b, _]) = trois_gris();
    seance.designer_reference(a, &biblio).unwrap();
    seance.regler_seuil("2", &biblio).unwrap();

    assert_eq!(
        seance.designer_reference(b, &Refus),
        Err("bibliotheque.erreur.autre")
    );
    let reference = seance.reference(Langue::Francais).unwrap();
    assert_eq!(
        (reference.numero, reference.seuil.as_deref()),
        (a, Some("2,00"))
    );
    assert_eq!(biblio.references().unwrap().len(), 1);
}

/// La référence est conservée dans la bibliothèque : une mesure qu'elle n'a
/// pas pu ranger ne peut pas le devenir.
#[test]
fn une_mesure_non_rangee_ne_devient_pas_reference() {
    let (_d, biblio, c) = bibliotheque();
    let mut seance = Seance::default();
    let n = seance.ajouter(
        gris(0.18, confirmees([M0, M1, M2])),
        &c,
        |_, _| Err("disque plein".into()),
        Langue::Francais,
    );
    assert_eq!(
        seance.designer_reference(n, &biblio),
        Err("mesurer.reference.non_rangee")
    );
    assert_eq!(
        seance.designer_reference(99, &biblio),
        Err("bibliotheque.erreur.autre")
    );
}

/// L'écart se calcule entre spectres de même condition de mesure, quelle que
/// soit leur place ; sinon l'écran avertit.
#[test]
fn l_ecart_compare_des_conditions_de_mesure_identiques_ou_avertit() {
    let (_d, biblio, c) = bibliotheque();
    let mut seance = Seance::default();
    // Référence : M0 gris 0,18, M1 gris 0,20, M2 gris 0,18.
    let r = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris_spectres([0.18, 0.20, 0.18], confirmees([M0, M1, M2])),
    );
    // Même M1 (0,20), rendu en première place par le pont.
    let permute = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris_spectres([0.20, 0.18, 0.18], confirmees([M1, M0, M2])),
    );
    let supposee = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.18, {
            let Info::Confirmee(d) = confirmees([M0, M1, M2]) else {
                unreachable!()
            };
            Info::Supposee(d)
        }),
    );
    let autre = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.18, confirmees([M2, M2, M2])),
    );
    let inconnue = ajouter(&mut seance, &biblio, &c, gris(0.18, Info::Inconnue));
    seance.designer_reference(r, &biblio).unwrap();

    let ecarts = |n| -> Vec<_> {
        fiche(&seance, n, Langue::Francais)
            .spectres
            .into_iter()
            .map(|s| s.ecart.unwrap())
            .collect()
    };
    let p = ecarts(permute);
    assert!(p
        .iter()
        .all(|e| e.comparaison == Comparaison::MemeCondition));
    assert!(
        p.iter().all(|e| e.delta_e00.as_deref() == Some("0,00")),
        "{p:?}"
    );

    assert!(ecarts(supposee)
        .iter()
        .all(|e| e.comparaison == Comparaison::NonConfirmee));
    // M2 contre M0, M1, M2 : la troisième place seule est comparable.
    let a = ecarts(autre);
    assert_eq!(a[0].comparaison, Comparaison::MemeCondition);
    assert_eq!(a[0].delta_e00.as_deref(), Some("0,00"));
    // Valeurs inconnues (longueurs d'onde non établies) : écart inconnu.
    for e in ecarts(inconnue) {
        assert_eq!(e.comparaison, Comparaison::NonConfirmee);
        assert_eq!(e.delta_e00, None);
        assert_eq!(e.verdict, VerdictEcart::Inconnu);
    }
}

#[test]
fn des_conditions_de_mesure_differentes_sont_signalees() {
    let (_d, biblio, c) = bibliotheque();
    let mut seance = Seance::default();
    let r = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.18, confirmees([M1, M1, M1])),
    );
    let m = ajouter(
        &mut seance,
        &biblio,
        &c,
        gris(0.18, confirmees([M0, M0, M0])),
    );
    seance.designer_reference(r, &biblio).unwrap();
    seance.regler_seuil("2", &biblio).unwrap();
    for s in fiche(&seance, m, Langue::Francais).spectres {
        let e = s.ecart.unwrap();
        assert_eq!(e.comparaison, Comparaison::ConditionDifferente);
        // M0 contre M1 ne se compare pas : l'avertissement, ni écart ni verdict.
        assert_eq!((e.delta_e00, e.delta_c, e.delta_h), (None, None, None));
        assert_eq!(e.verdict, VerdictEcart::NonComparable);
    }
}

/// Le seuil est arrondi à deux décimales à la saisie, comme les écarts
/// affichés : le seuil écrit, conservé et jugé est le même nombre. 1,115
/// n'existe pas exactement en binaire (1,11499…) : il s'écrit 1,11.
#[test]
fn le_seuil_est_arrondi_comme_les_ecarts_affiches() {
    let (_d, biblio, mut seance, [a, _, _]) = trois_gris();
    seance.designer_reference(a, &biblio).unwrap();
    seance.regler_seuil("1,115", &biblio).unwrap();
    assert_eq!(
        seance.reference(Langue::Francais).unwrap().seuil.as_deref(),
        Some("1,11")
    );
    assert_eq!(biblio.references().unwrap()[0].seuil, Some(1.11));
    // Arrondi à zéro : refusé.
    assert_eq!(
        seance.regler_seuil("0,001", &biblio),
        Err("mesurer.reference.seuil_invalide")
    );
}

/// Au lancement suivant, la couleur de référence conservée et son seuil
/// reviennent dans Mesurer ; les nouvelles mesures s'y comparent.
#[test]
fn la_reference_conservee_revient_au_lancement_suivant() {
    let (_d, biblio, mut seance, [_, b, _]) = trois_gris();
    seance
        .renommer(b, "Gris du BAT", |id, nom| {
            biblio.renommer_mesure(id, nom).map_err(|e| e.to_string())
        })
        .unwrap();
    seance.designer_reference(b, &biblio).unwrap();
    seance.regler_seuil("2", &biblio).unwrap();
    drop(seance);

    let mut reprise = Seance::default();
    reprise
        .reprendre_reference(&biblio, Langue::Francais)
        .unwrap();
    let reference = reprise.reference(Langue::Francais).unwrap();
    assert_eq!(
        (reference.nom.as_str(), reference.seuil.as_deref()),
        ("Gris du BAT", Some("2,00"))
    );
    let c = biblio.conditions().unwrap().remove(0);
    let n = ajouter(
        &mut reprise,
        &biblio,
        &c,
        gris(0.20, confirmees([M0, M1, M2])),
    );
    assert_ne!(n, reference.numero);
    let e = fiche(&reprise, n, Langue::Francais).spectres[0]
        .ecart
        .clone()
        .unwrap();
    // (51,84 − 50,10) / S_L, S_L = 1,003 à L* = 50,97 : 1,735, à la main.
    assert_eq!(e.delta_e00.as_deref(), Some("1,73"));
    assert_eq!(e.verdict, VerdictEcart::ProcheDeLaLimite);
    // Rien de neuf dans la bibliothèque : la référence n'est pas rangée deux fois.
    assert_eq!(biblio.arborescence("").unwrap()[0].mesures.len(), 4);

    // Sans référence conservée, la séance reste vide ; un refus est dit.
    let (_d2, vide, _) = bibliotheque();
    let mut neuve = Seance::default();
    neuve.reprendre_reference(&vide, Langue::Francais).unwrap();
    assert!(neuve.fiches(Langue::Francais).is_empty());
    assert_eq!(
        neuve.reprendre_reference(&Refus, Langue::Francais),
        Err("bibliotheque.erreur.autre")
    );
}
