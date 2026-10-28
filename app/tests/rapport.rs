//! Rapport de vérification en HTML : régénéré depuis les données rangées,
//! dans une seule langue, sans script ni lien externe, exporté seul ou à
//! côté du profil.

mod banc_verification;

use app::rapport::rapport_html;
use app::textes::{cles, texte, Langue};
use banc_verification::*;
use bibliotheque::IdVerification;
use profilage::{Impression, MoteurVerificationSimule};

/// Vérifie le profil du banc sur une seconde mire mesurée avec `decalage`.
fn verifier(b: &Banc, impression: Impression, decalage: f64) -> IdVerification {
    let seconde = b.importer("seconde_M1.txt", &b.seconde(decalage));
    b.ouverte
        .verifier_profil(
            &b.demande(seconde, impression),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap()
        .verification
}

fn rapport(b: &Banc, v: IdVerification, langue: Langue) -> String {
    rapport_html(&b.ouverte.donnees_rapport(v).unwrap(), langue)
}

#[test]
fn le_rapport_porte_les_deux_familles_et_la_version_des_tolerances() {
    let b = banc();
    let v = verifier(&b, Impression::ATraversLeProfil, 0.0);
    let html = rapport(&b, v, Langue::Francais);
    for attendu in [
        "Atelier",
        "v1",
        texte(Langue::Francais, "verif.fidelite"),
        texte(Langue::Francais, "verif.prediction"),
        texte(Langue::Francais, "verif.hors_gamut"),
        "Epson SC-P5300",
        "Semigloss 250",
        "1440 dpi",
        "seconde_M1.txt",
        "FD-9",
        "Premium Semigloss",
        "xicclu -v0 -fb -ir -pl profil.icc",
        "✓",
    ] {
        assert!(html.contains(attendu), "{attendu} absent du rapport");
    }
    assert!(html.starts_with("<!DOCTYPE html>"), "{}", &html[..40]);
    assert!(html.contains("lang=\"fr\""));
}

/// Jeu de tolérances modifié après usage : la vérification faite avec la
/// version 1 redonne le même rapport, à l'octet.
#[test]
fn le_rapport_d_une_ancienne_verification_ne_change_pas() {
    let b = banc();
    let v = verifier(&b, Impression::ATraversLeProfil, -4.0);
    let avant = rapport(&b, v, Langue::Francais);
    let mut criteres = atelier();
    criteres.prediction.max = Some(9.0);
    let v2 = b
        .ouverte
        .enregistrer_tolerances("Atelier", &criteres, a(30))
        .unwrap();
    b.ouverte.recalculer_verification(v, v2, a(31)).unwrap();
    assert_eq!(rapport(&b, v, Langue::Francais), avant);
}

#[test]
fn un_rapport_est_dans_une_seule_langue() {
    let b = banc();
    let v = verifier(&b, Impression::ATraversLeProfil, -4.0);
    let html = rapport(&b, v, Langue::Anglais);
    assert!(html.contains("lang=\"en\""));
    let francais: Vec<&str> = cles()
        .filter(|c| c.starts_with("verif.") || c.starts_with("rapport."))
        // Le nom proposé d'un jeu devient le nom choisi par l'imprimeur : une donnée.
        .filter(|c| *c != "verif.tolerances.nom_propose")
        .map(|c| texte(Langue::Francais, c))
        .filter(|t| t.chars().count() > 6)
        .filter(|t| !cles().any(|c| texte(Langue::Anglais, c) == *t))
        .filter(|t| html.contains(*t))
        .collect();
    assert!(francais.is_empty(), "textes français : {francais:?}");
    assert!(html.contains(texte(Langue::Anglais, "verif.fidelite")));
    // Nombres à l'anglaise : point décimal.
    assert!(!html.contains("0,00"), "virgule décimale en anglais");
    let fr = rapport(&b, v, Langue::Francais);
    assert!(fr.contains(','), "virgule décimale en français");
}

#[test]
fn les_noms_sont_echappes() {
    let b = banc_papier("Canson <Baryta> & co \"blanc\" l'été");
    let v = verifier(&b, Impression::ATraversLeProfil, 0.0);
    let html = rapport(&b, v, Langue::Francais);
    assert!(html.contains("Canson &lt;Baryta&gt; &amp; co &quot;blanc&quot; l&#39;été"));
    assert!(!html.contains("<Baryta>"));
}

/// Une ligne `INSTRUMENTATION` avec numéro de série : ni le rapport, ni le
/// profil exporté, ni rien de rangé à côté ne le transporte.
#[test]
fn le_numero_de_serie_ne_sort_jamais() {
    let b = banc();
    let mesure = export_lignes(
        &b.lignes_mire(|c| lab_fidele(c, 0.0)),
        INSTRUMENT_AVEC_SERIE,
    );
    let seconde = b.importer("seconde_M1.txt", &mesure);
    let v = b
        .ouverte
        .verifier_profil(
            &b.demande(seconde, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap()
        .verification;
    for langue in [Langue::Francais, Langue::Anglais] {
        let html = rapport(&b, v, langue);
        assert!(html.contains("FD-9"));
        assert!(!html.contains("Serial") && !html.contains("12345678"));
    }
    let fichier = b.dossier.path().join("x.icc");
    b.ouverte
        .exporter_profil_et_rapport(v, Langue::Francais, &fichier)
        .unwrap();
    for f in [fichier.clone(), fichier.with_extension("html")] {
        let octets = std::fs::read(&f).unwrap();
        let texte = String::from_utf8_lossy(&octets);
        assert!(!texte.contains("12345678"), "{}", f.display());
    }
}

#[test]
fn le_rapport_n_a_ni_script_ni_lien_externe() {
    let b = banc();
    let v = verifier(&b, Impression::ATraversLeProfil, 0.0);
    let html = rapport(&b, v, Langue::Anglais).to_lowercase();
    for interdit in [
        "<script", "http:", "https:", "<link", "src=", "@import", "url(",
    ] {
        assert!(!html.contains(interdit), "{interdit} dans le rapport");
    }
}

#[test]
fn un_inconnu_est_ecrit_en_toutes_lettres() {
    let b = banc();
    // Mesure décalée : aucun écart réel n'est nul.
    let v = verifier(&b, Impression::SansGestion, -1.0);
    let html = rapport(&b, v, Langue::Francais);
    assert!(html.contains(texte(Langue::Francais, "verif.inconnu.sans_demande")));
    let verdict = format!(
        "class=\"verdict\">? {}",
        texte(Langue::Francais, "verif.etat.incomplet")
    );
    assert!(
        html.contains(&verdict),
        "verdict incomplet en mot et symbole"
    );
    // Chaque contrôle de fidélité dit « inconnu », aucun ne vaut 0,00.
    assert!(!html.contains(">0,00<"), "un inconnu écrit 0,00");
    let inconnu = format!(">{}<", texte(Langue::Francais, "verif.inconnu"));
    assert!(html.matches(&inconnu).count() >= 6, "fidélité inconnue");
}

#[test]
fn le_profil_et_son_rapport_s_exportent_cote_a_cote() {
    let b = banc();
    let v = verifier(&b, Impression::ATraversLeProfil, -8.0);
    let fichier = b.dossier.path().join("export").join("x.icc");
    std::fs::create_dir_all(fichier.parent().unwrap()).unwrap();
    b.ouverte
        .exporter_profil_et_rapport(v, Langue::Francais, &fichier)
        .unwrap();
    assert_eq!(std::fs::read(&fichier).unwrap(), icc());
    let html = std::fs::read_to_string(fichier.with_extension("html")).unwrap();
    assert_eq!(html, rapport(&b, v, Langue::Francais));
    assert!(html.contains("✗"), "hors tolérance");

    let seul = b.dossier.path().join("rapport.html");
    b.ouverte
        .exporter_rapport(v, Langue::Anglais, &seul)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(&seul).unwrap(),
        rapport(&b, v, Langue::Anglais)
    );
}

#[test]
fn les_memes_couleurs_que_la_mire_du_profil_sont_dites_dans_le_rapport() {
    let b = banc();
    let meme = b.importer("reimpression_M1.txt", &export(&MIRE_PROFIL, 0.0));
    let v = b
        .ouverte
        .verifier_profil(
            &b.demande(meme, Impression::ATraversLeProfil),
            &MoteurVerificationSimule::fidele(),
            a(10),
        )
        .unwrap()
        .verification;
    let html = rapport(&b, v, Langue::Francais);
    assert!(html.contains(texte(Langue::Francais, "verif.memes_couleurs")));
}
