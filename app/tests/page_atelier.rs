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
    let formulaire = corps(&script, "function formulaire({");
    assert_eq!(
        formulaire.matches("btn--primary").count(),
        1,
        "{formulaire}"
    );
    for attendu in [
        "\"atelier.enregistrer\")",
        "\"atelier.choisir_logo\")",
        "\"atelier.retirer_logo\")",
        "\"atelier.premiere\")",
        "el(\"textarea\", \"input\")",
    ] {
        assert!(formulaire.contains(attendu), "{attendu} :\n{formulaire}");
    }
    // En fenêtre (depuis le rapport) comme en page (vue « Réglages »), le même formulaire.
    let ouvrir = corps(&script, "function ouvrir(apres, premiere = false) {");
    assert!(ouvrir.contains("formulaire({") && ouvrir.contains("showModal()"));
    assert!(script.contains("formulaire({ prefixe: \"reglages-atelier-\" })"));
    for champ in ["nom", "adresse", "courriel", "telephone", "site"] {
        assert!(script.contains(&format!("\"{champ}\"")), "{champ}");
    }
    // Retirer le logo, inactif sans logo, dit pourquoi.
    assert!(script.contains("\"atelier.retirer_logo.raison\""));
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
    // Rien de rogné ni de caché (mainteneur, 2 novembre 2026) : les cases
    // prennent la hauteur de leur texte et coupent les mots longs ; traits
    // haut et bas épais, comme dans le PDF.
    let cellule = ligne_css(&css, ".cartouche-rapport__cellule {");
    assert!(!cellule.contains("overflow: hidden") && !cellule.contains(" height:"));
    assert!(cellule.contains("overflow-wrap: anywhere;"), "{cellule}");
    assert!(cartouche.contains("border-block-width: calc(var(--rule-strong) * 2);"));
}

/// Le logo est peint dans un canvas : une image chargée par une adresse
/// `blob:` ou `data:` est refusée par la CSP (`img-src 'self'`, ADR 0003),
/// et l'aperçu montrait une image cassée. Même règle dans la fenêtre du
/// rapport.
#[test]
fn le_logo_est_peint_dans_un_canvas_sans_adresse_blob() {
    for fichier in ["interface/atelier.js", "interface/rapport-image.js"] {
        let script = lire(fichier);
        for interdit in ["createObjectURL", "el(\"img\"", "data:image"] {
            assert!(!script.contains(interdit), "{fichier} : {interdit}");
        }
    }
    let script = lire("interface/atelier.js");
    let peindre = corps(&script, "function peindreLogo(image, classe) {");
    assert!(peindre.contains("el(\"canvas\", classe)") && peindre.contains("drawImage(image"));
    assert!(peindre.contains("\"role\", \"img\"") && peindre.contains("t(\"rapport.logo\")"));
    assert!(script.contains("createImageBitmap("));
    assert!(lire("interface/rapport-image.js")
        .contains("window.Atelier.peindreLogo(image, \"fenetre-rapport__logo\")"));
    let conf: serde_json::Value = serde_json::from_str(&lire("tauri.conf.json")).unwrap();
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    assert!(csp.contains("img-src 'self';"), "{csp}");
}

/// « Réglages » est le dernier onglet de la barre du haut, même style que
/// les tâches ; sa vue porte le formulaire « Atelier » en page. Les textes
/// de page passent par `data-t`, repris sur place au changement de langue.
#[test]
fn la_vue_reglages_porte_l_atelier_en_page() {
    let page = lire("interface/index.html");
    let nav = page
        .split("<nav class=\"taches\"")
        .nth(1)
        .and_then(|n| n.split("</nav>").next())
        .unwrap();
    let dernier = nav.rsplit("<button").next().unwrap();
    assert!(
        dernier.contains("class=\"tache\" data-tache=\"reglages\""),
        "{dernier}"
    );
    assert!(dernier.contains("<svg") && dernier.contains("data-t=\"tache.reglages\""));
    let vue = page
        .split("<section data-vue=\"reglages\"")
        .nth(1)
        .and_then(|v| v.split("</section>").next())
        .expect("vue reglages");
    for attendu in [
        "data-t=\"tache.reglages\"",
        "data-t=\"reglages.atelier\"",
        "data-t=\"reglages.atelier.aide\"",
        "data-reglages-atelier",
    ] {
        assert!(vue.contains(attendu), "{attendu}");
    }
    assert!(
        lire("interface/app.js").contains("\"reglages\"]"),
        "Réglages sans instrument"
    );
    let script = lire("interface/atelier.js");
    assert!(script.contains("e.detail !== \"reglages\""));
    // Posé une fois, puis relu sur place : jamais redessiné à chaque visite.
    assert!(script.contains("if (!enPage) {") && script.contains("enPage.relire()"));
    let formulaire = corps(&script, "function formulaire({");
    assert!(formulaire.contains("e.dataset.t = cle") || script.contains("e.dataset.t = cle"));
    use app::textes::{texte, Langue};
    for cle in [
        "tache.reglages",
        "reglages.atelier",
        "reglages.atelier.aide",
    ] {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle}");
        }
    }
}

#[test]
fn une_erreur_est_dite_sous_son_champ() {
    let script = lire("interface/atelier.js");
    let erreur = corps(&script, "function montrerErreur(erreurs, refus) {");
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
    // Toute clé écrite entre guillemets : `t("…")`, `txt(…, "…")` ou `dataset.t = "…"`.
    for prefixe in ["\"atelier.", "\"rapport.", "\"cartouche_rapport."] {
        for morceau in script.split(prefixe).skip(1) {
            let fin = morceau.find('"').unwrap();
            let suite = &morceau[..fin];
            if !suite.ends_with('.') {
                cles.push(format!("{}{suite}", &prefixe[1..]));
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
