//! La page, lue comme un fichier : formulaire « Atelier » (ticket #89),
//! rempli une fois, avec un aperçu du cartouche.

use std::path::Path;

fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif))
        .unwrap()
        .replace("\r\n", "\n")
}

fn corps<'a>(script: &'a str, entete: &str) -> &'a str {
    let corps = script
        .split(entete)
        .nth(1)
        .unwrap_or_else(|| panic!("{entete} absent"));
    &corps[..corps.find("\n  }").unwrap_or(corps.len())]
}

fn ligne_css(css: &str, selecteur: &str) -> String {
    let debut = css
        .find(&format!("\n{selecteur}"))
        .unwrap_or_else(|| panic!("{selecteur} absent"));
    css[debut + 1..].lines().next().unwrap().to_string()
}

#[test]
fn le_formulaire_a_ses_champs_et_un_seul_bouton_principal() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("rapport-image.js") < position("atelier.js"));
    let script = lire("interface/atelier.js");
    assert!(script.contains("window.Atelier = "));
    assert!(script.lines().count() < 400);
    let ouvrir = corps(&script, "function ouvrir(apres, premiere = false) {");
    assert_eq!(ouvrir.matches("btn--primary").count(), 1, "{ouvrir}");
    for attendu in [
        "t(\"atelier.enregistrer\")",
        "t(\"atelier.choisir_logo\")",
        "t(\"atelier.retirer_logo\")",
        "t(\"atelier.premiere\")",
        "el(\"textarea\", \"input\")",
        "showModal()",
    ] {
        assert!(ouvrir.contains(attendu), "{attendu} :\n{ouvrir}");
    }
    for champ in ["nom", "adresse", "courriel", "telephone", "site"] {
        assert!(script.contains(&format!("\"{champ}\"")), "{champ}");
    }
    // Retirer le logo, inactif sans logo, dit pourquoi.
    assert!(script.contains("t(\"atelier.retirer_logo.raison\")"));
    for commande in [
        "atelier_lire",
        "atelier_logo",
        "atelier_enregistrer",
        "atelier_choisir_logo",
        "atelier_retirer_logo",
    ] {
        assert!(
            script.contains(&format!("invoke(\"{commande}\"")),
            "{commande}"
        );
    }
}

#[test]
fn l_apercu_du_cartouche_suit_la_frappe() {
    let script = lire("interface/atelier.js");
    let apercu = corps(&script, "function apercu(zone, valeurs, logo) {");
    for attendu in [
        "el(\"div\", \"cartouche-rapport\")",
        "\"cartouche-rapport__etiquette\"",
    ] {
        assert!(apercu.contains(attendu), "{attendu} :\n{apercu}");
    }
    assert!(script.contains("addEventListener(\"input\", redessiner)"));
    let css = lire("../design-system/components.css");
    let cartouche = ligne_css(&css, ".cartouche-rapport {");
    for attendu in ["display: grid;", "border: var(--rule) solid var(--encre);"] {
        assert!(cartouche.contains(attendu), "{attendu} : {cartouche}");
    }
    assert!(lire("../design-system/ui-kit.html").contains("class=\"cartouche-rapport\""));
}

#[test]
fn une_erreur_est_dite_sous_son_champ() {
    let script = lire("interface/atelier.js");
    let erreur = corps(&script, "function montrerErreur(refus) {");
    assert!(erreur.contains("erreurs[refus.champ]"), "{erreur}");
    // Chaque champ a sa ligne d'erreur, à une place gardée.
    assert!(script.contains("el(\"p\", \"why fenetre-atelier__erreur\")"));
    let css = lire("../design-system/components.css");
    assert!(ligne_css(&css, ".fenetre-atelier__erreur {").contains("min-height:"));
}

#[test]
fn les_textes_de_l_atelier_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    let script = lire("interface/atelier.js");
    let mut cles = Vec::new();
    for prefixe in ["t(\"atelier.", "t(\"rapport."] {
        for morceau in script.split(prefixe).skip(1) {
            let fin = morceau.find('"').unwrap();
            let suite = &morceau[..fin];
            if !suite.ends_with('.') {
                cles.push(format!("{}{suite}", &prefixe[3..]));
            }
        }
    }
    for champ in ["nom", "adresse", "courriel", "telephone", "site"] {
        cles.push(format!("atelier.champ.{champ}"));
    }
    assert!(cles.len() > 12, "{cles:?}");
    for cle in &cles {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} sans texte");
        }
    }
}
