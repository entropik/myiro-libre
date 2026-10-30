//! La page, lue comme un fichier : coches « Inclure dans le rapport »,
//! points relevés et fenêtre d'export du rapport PDF (ticket #89).

use std::path::Path;

fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif))
        .unwrap()
        .replace("\r\n", "\n")
}

/// Corps d'une fonction `entete` de `script`, jusqu'à sa fin indentée de
/// deux espaces.
fn corps<'a>(script: &'a str, entete: &str) -> &'a str {
    let corps = script
        .split(entete)
        .nth(1)
        .unwrap_or_else(|| panic!("{entete} absent"));
    &corps[..corps.find("\n  }").unwrap_or(corps.len())]
}

/// Règle CSS d'un sélecteur, sur une ligne.
fn ligne_css(css: &str, selecteur: &str) -> String {
    let debut = css
        .find(&format!("\n{selecteur}"))
        .unwrap_or_else(|| panic!("{selecteur} absent"));
    css[debut + 1..].lines().next().unwrap().to_string()
}

#[test]
fn chaque_partie_a_sa_coche_hors_du_bouton() {
    let script = lire("interface/accordeon.js");
    let partie = corps(
        &script,
        "function partie(cle, contenu = [], remplir = null, vider = null) {",
    );
    // La coche est posée dans le titre, à côté du bouton, jamais dedans.
    assert!(partie.contains("titre.append(bouton);"), "{partie}");
    assert!(partie.contains("titre.append(coche(cle));"), "{partie}");
    assert!(!partie.contains("bouton.append(coche"), "{partie}");
    let coche = corps(&script, "function coche(cle) {");
    for attendu in [
        "el(\"label\", \"accordeon__rapport\")",
        "c.type = \"checkbox\"",
        "t(\"rapport.inclure\")",
        "c.disabled = true",
        "t(\"rapport.toujours\")",
    ] {
        assert!(coche.contains(attendu), "{attendu} :\n{coche}");
    }
    for cle in [
        "comparer.volume",
        "comparer.teintes",
        "comparer.coupe",
        "comparer.chemin",
        "comparer.primaires",
        "comparer.ecarts",
    ] {
        assert!(script.contains(&format!("\"{cle}\"")), "{cle}");
    }
    assert!(script
        .contains("const RAPPORT_TOUJOURS = new Set([\"comparer.chiffres\", \"image.titre\"]);"));
    assert!(script.contains("cochees"));
    assert!(script.lines().count() < 400);
    let css = lire("../design-system/components.css");
    assert!(ligne_css(&css, ".accordeon__titre {").contains("display: flex;"));
    assert!(ligne_css(&css, ".accordeon__rapport {").contains("white-space: nowrap;"));
    assert!(lire("../design-system/ui-kit.html").contains("accordeon__rapport"));
}

#[test]
fn les_coches_sont_retenues_sans_stockage_obligatoire() {
    let script = lire("interface/accordeon.js");
    assert!(script.contains("const CLE_RAPPORT = \"myiro-libre.rapport\";"));
    for entete in ["function lireCoches() {", "function garderCoches() {"] {
        let f = corps(&script, entete);
        assert!(
            f.contains("try {") && f.contains("} catch {"),
            "{entete} :\n{f}"
        );
    }
}

#[test]
fn garder_un_point_va_jusqu_a_huit() {
    let script = lire("interface/pastilles-image.js");
    assert!(script.contains("const POINTS_MAX = 8;"));
    let releves = corps(&script, "function releves() {");
    for attendu in [
        "t(\"rapport.points.garder\")",
        "garder.disabled = ",
        "gardes.length >= POINTS_MAX",
        "t(\"rapport.points.raison.plein\")",
        "t(\"rapport.points.raison.aucun\")",
        "t(\"rapport.points.retirer\")",
    ] {
        assert!(releves.contains(attendu), "{attendu} :\n{releves}");
    }
    assert!(script.contains("pointsReleves"));
    // Une autre image : les points gardés sont oubliés.
    assert!(corps(&script, "function oublier() {").contains("gardes = [];"));
    assert!(script.lines().count() < 400);
}

#[test]
fn la_liste_des_points_a_sa_place_reservee() {
    let script = lire("interface/pastilles-image.js");
    let releves = corps(&script, "function releves() {");
    // Les huit emplacements sont toujours là, vides ou remplis.
    assert!(
        releves.contains("for (let i = 0; i < POINTS_MAX; i++)"),
        "{releves}"
    );
    assert!(
        releves.contains("\"points-releves__point points-releves__point--vide\""),
        "{releves}"
    );
    let css = lire("../design-system/components.css");
    let liste = ligne_css(&css, ".points-releves__liste {");
    assert!(liste.contains("grid-template-rows: repeat(4,"), "{liste}");
    let point = ligne_css(&css, ".points-releves__point {");
    assert!(point.contains("height:"), "{point}");
    assert!(lire("../design-system/ui-kit.html").contains("points-releves"));
}

#[test]
fn exporter_dessine_hors_ecran_les_parties_cochees_repliees() {
    let script = lire("interface/rapport-image.js");
    let captures = corps(&script, "function captures(catalogue, langue) {");
    for attendu in [
        "el(\"div\", \"hors-ecran\")",
        "document.body.append(zone);",
        "zone.remove();",
        "textes = catalogue;",
        "racine.dataset.theme = \"light\";",
        "racine.dataset.theme = theme;",
        "window.Accordeon.cochees()",
    ] {
        assert!(captures.contains(attendu), "{attendu} :\n{captures}");
    }
    // Tout est rétabli même si une capture échoue.
    assert!(captures.contains("} finally {"), "{captures}");
    let css = lire("../design-system/components.css");
    let hors = ligne_css(&css, ".hors-ecran {");
    for attendu in ["position: fixed;", "left: -10000px;"] {
        assert!(hors.contains(attendu), "{attendu} : {hors}");
    }
    // Les graphiques SVG ne gardent que la liste blanche, styles figés en attributs.
    let svg = corps(&script, "function serialiser(noeud) {");
    assert!(svg.contains("BALISES.has("), "{svg}");
    assert!(script.contains("getComputedStyle("));
    // La vue 3D est rendue en PNG dans un canvas de 1 200 × 900.
    assert!(script.contains("canvas.style.width = \"1200px\";"));
    assert!(lire("interface/vue3d.js").contains("capturer() {"));
    assert!(script.lines().count() < 400);
}

#[test]
fn le_bouton_rapport_est_secondaire() {
    let page = lire("interface/index.html");
    assert!(page.contains(
        "<button class=\"btn\" data-comparer-rapport data-t=\"rapport.bouton\" aria-haspopup=\"dialog\" disabled></button>"
    ));
    assert!(page.contains(
        "<p class=\"why\" data-comparer-rapport-raison data-t=\"rapport.bouton.raison\"></p>"
    ));
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("comparer-image.js") < position("rapport-image.js"));
    assert!(position("rapport-image.js") < position("comparer.js"));
    let script = lire("interface/rapport-image.js");
    let etat = corps(&script, "function etat() {");
    for attendu in [
        "bouton.disabled = !pret;",
        "t(pret ? \"rapport.bouton.aide\" : \"rapport.bouton.raison\")",
    ] {
        assert!(etat.contains(attendu), "{attendu} :\n{etat}");
    }
    assert!(lire("interface/comparer-image.js").contains("image-dessinee"));
    let comparer = lire("interface/comparer.js");
    assert!(comparer.contains("window.RapportImage.brancher("));
    assert!(comparer.lines().count() < 400);
    assert!(lire("interface/comparer-image.js").lines().count() < 400);
}

#[test]
fn la_fenetre_d_export_a_un_seul_bouton_principal() {
    let script = lire("interface/rapport-image.js");
    let fenetre = corps(&script, "function ouvrir() {");
    assert_eq!(fenetre.matches("btn--primary").count(), 1, "{fenetre}");
    for attendu in [
        "el(\"dialog\", \"fenetre-rapport\")",
        "t(\"rapport.exporter\")",
        "t(\"rapport.champ.client\")",
        "t(\"rapport.champ.objet\")",
        "t(\"rapport.champ.langue\")",
        "t(\"rapport.modifier_atelier\")",
        "showModal()",
    ] {
        assert!(fenetre.contains(attendu), "{attendu} :\n{fenetre}");
    }
    // Échap ferme (événement « close ») et le focus revient au bouton.
    let fermer = corps(&script, "function rendre() {");
    assert!(fermer.contains("bouton.focus()"), "{fermer}");
    // L'état « Rapport en cours… » s'écrit à une place gardée.
    assert!(script.contains("statut.textContent = "));
    assert!(script.contains("invoke(\"image_rapport_exporter\""));
    assert!(script.contains("invoke(\"rapport_ouvrir_dossier\""));
    let css = lire("../design-system/components.css");
    assert!(ligne_css(&css, ".fenetre-rapport__statut {").contains("min-height:"));
}

#[test]
fn les_textes_du_rapport_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    let mut cles = Vec::new();
    for fichier in ["rapport-image.js", "accordeon.js", "pastilles-image.js"] {
        let script = lire(&format!("interface/{fichier}"));
        for morceau in script.split("t(\"rapport.").skip(1) {
            let fin = morceau.find('"').unwrap();
            // Une clé composée (« rapport.langue. » + code) est listée plus bas.
            if !morceau[..fin].ends_with('.') {
                cles.push(format!("rapport.{}", &morceau[..fin]));
            }
        }
    }
    cles.extend(["rapport.langue.fr", "rapport.langue.en"].map(String::from));
    cles.push("rapport.bouton".into());
    cles.push("rapport.bouton.raison".into());
    assert!(cles.len() > 15, "{cles:?}");
    for cle in &cles {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} sans texte");
        }
    }
}
