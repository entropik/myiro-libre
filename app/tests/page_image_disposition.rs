//! La page, lue comme un fichier : disposition de la section « Image » en
//! deux colonnes, image agrandie et case « Compensation du point noir »
//! (ticket #80, essai d'écran du 30 octobre 2026).

use std::path::Path;

/// Fichier de la page, fins de ligne ramenées à `\n`.
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

/// Ligne de `components.css` qui commence par `selecteur`.
fn ligne_css(css: &str, selecteur: &str) -> String {
    css.lines()
        .find(|l| l.trim_start().starts_with(selecteur))
        .unwrap_or_else(|| panic!("{selecteur} absent"))
        .to_string()
}

/// reglages-image.js et image-agrandie.js sont chargés après leurs outils et
/// avant comparer-image.js, sous 400 lignes, sans couleur en dur.
#[test]
fn les_scripts_de_la_section_image_sont_charges_dans_l_ordre() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    for nom in ["reglages-image.js", "image-agrandie.js"] {
        assert!(position("arbre.js") < position(nom), "arbre.js avant {nom}");
        assert!(position("graphiques.js") < position(nom));
        assert!(position(nom) < position("comparer-image.js"), "{nom}");
        let script = lire(&format!("interface/{nom}"));
        let lignes = script.lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
        let hexa = script.match_indices('#').any(|(i, _)| {
            script[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count()
                >= 3
        });
        assert!(!hexa, "{nom} : couleur hexadécimale en dur");
        for interdit in ["rgb(", "rgba(", "hsl(", "style=", ".toFixed("] {
            assert!(!script.contains(interdit), "{nom} : {interdit}");
        }
    }
    for nom in ["comparer-image.js", "pastilles-image.js"] {
        let lignes = lire(&format!("interface/{nom}")).lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
    }
}

/// Deux colonnes : l'image à gauche, jamais plus haute que 70 % de la
/// fenêtre (cadre aux proportions de l'image, `object-fit: contain`) ; à
/// droite un panneau de 320 à 380 px collé en haut au défilement. Quand la
/// section est étroite, les colonnes s'empilent et l'image passe à 50 %.
#[test]
fn la_section_image_a_deux_colonnes_qui_s_empilent_si_elle_est_etroite() {
    let css = lire("../design-system/components.css");
    let colonnes = ligne_css(&css, ".image-comparee__colonnes {");
    for attendu in [
        "display: grid;",
        "clamp(320px, 32%, 380px)",
        "--hauteur-image: 70vh;",
    ] {
        assert!(colonnes.contains(attendu), "{attendu} : {colonnes}");
    }
    assert!(ligne_css(&css, ".image-comparee {").contains("container-type: inline-size;"));
    let panneau = ligne_css(&css, ".image-comparee__panneau {");
    for attendu in [
        "position: sticky;",
        "top: 0;",
        "overflow-y: auto;",
        "overflow-x: hidden;",
    ] {
        assert!(panneau.contains(attendu), "{attendu} : {panneau}");
    }
    let etroit = css
        .split("@container (max-width: 760px) {")
        .nth(1)
        .and_then(|r| r.split("\n}").next())
        .expect("colonnes empilées quand la section est étroite");
    assert!(etroit.contains("grid-template-columns: minmax(0, 1fr); --hauteur-image: 50vh;"));
    let cadre = ligne_css(&css, ".pipette {");
    assert!(
        cadre.contains("width: min(100%, calc(var(--hauteur-image, 70vh) * var(--ratio, 1.5)));"),
        "{cadre}"
    );
    assert!(ligne_css(&css, ".surimpression {").contains("object-fit: contain;"));
    // Pastilles en deux ou trois colonnes dans un panneau de 320 à 380 px.
    let grille = ligne_css(&css, ".pastilles {");
    assert!(
        grille.contains("minmax(calc(var(--space-12) * 2 + var(--rule) * 2), 1fr)"),
        "{grille}"
    );
    let kit = lire("../design-system/ui-kit.html");
    for attendu in [
        "class=\"image-comparee\"",
        "class=\"image-comparee__panneau\"",
        "class=\"agrandie__barre\"",
    ] {
        assert!(kit.contains(attendu), "ui-kit sans {attendu}");
    }
}

/// « Agrandir » ouvre une fenêtre modale sur toute la fenêtre ; le cadre de
/// l'image (surimpression, repère, pipette) y est déplacé tel quel, avec le
/// point cliqué dessous. « Fermer », Échap (événement `close` du dialogue)
/// et un clic hors de l'image ferment ; le cadre revient à sa place et le
/// focus au bouton. Un nouveau dessin de la section ferme l'image agrandie.
#[test]
fn l_image_agrandie_se_ferme_au_bouton_a_echap_et_hors_de_l_image() {
    let script = lire("interface/image-agrandie.js");
    let ouvrir = corps(&script, "function ouvrir(cadre, lecture, bouton) {");
    for attendu in [
        "el(\"dialog\", \"agrandie\")",
        "t(\"image.agrandie.fermer\")",
        "dialogue.append(barre, cadre, lecture());",
        "dialogue.addEventListener(\"close\", rendre);",
        "dialogue.showModal();",
        "!e.target.closest(\".surimpression, .agrandie__barre, .agrandie__lecture\")",
    ] {
        assert!(ouvrir.contains(attendu), "{attendu} :\n{ouvrir}");
    }
    let rendre = corps(&script, "function rendre() {");
    for attendu in [
        "place.replaceWith(cadre);",
        "bouton.focus();",
        "dialogue.remove();",
    ] {
        assert!(rendre.contains(attendu), "{attendu} :\n{rendre}");
    }
    assert!(script.contains("t(\"image.agrandir\")"));
    let image = lire("interface/comparer-image.js");
    let dessiner = corps(&image, "function dessiner() {");
    assert!(
        dessiner.contains("window.ImageAgrandie.fermer();"),
        "{dessiner}"
    );
    let surimpression = corps(&image, "function surimpression() {");
    assert!(
        surimpression.contains("window.ImageAgrandie.bouton(cadre, window.PastillesImage.lecture)")
    );
    let css = lire("../design-system/components.css");
    let agrandie = ligne_css(&css, ".agrandie {");
    for attendu in [
        "width: 100vw;",
        "height: 100vh;",
        "background: var(--fond);",
    ] {
        assert!(agrandie.contains(attendu), "{attendu} : {agrandie}");
    }
}

/// « Compensation du point noir » est cochée par défaut (et le redevient à la
/// fermeture de la comparaison) ; la changer relance l'analyse, et la
/// pipette suit le même réglage. Le réglage est rappelé dans la légende de
/// la surimpression et au-dessus des chiffres. Le champ qui avait le focus
/// le retrouve après le nouveau dessin.
#[test]
fn la_compensation_du_point_noir_se_regle_et_se_rappelle() {
    let image = lire("interface/comparer-image.js");
    assert!(image.contains("let pointNoir = true;"));
    let changer = corps(&image, "function changerPointNoir(coche) {");
    assert!(changer.contains("pointNoir = coche;"), "{changer}");
    assert!(changer.contains("analyser();"), "{changer}");
    assert!(image.contains("invoke(\"image_analyser\", { sources, compensation: pointNoir })"));
    let fermer = corps(&image, "function fermer() {");
    assert!(fermer.contains("\"les_deux\", true]"), "{fermer}");
    let calque = corps(&image, "function calque() {");
    assert!(
        calque.contains("t(pointNoir ? \"image.legende.point_noir\" : \"image.legende\")"),
        "{calque}"
    );
    let surimpression = corps(&image, "function surimpression() {");
    assert!(surimpression.contains("R.pointNoir(pointNoir, changerPointNoir)"));
    assert!(surimpression
        .contains("t(pointNoir ? \"image.rendu.point_noir\" : \"image.rendu.relatif\")"));
    let dessiner = corps(&image, "function dessiner() {");
    assert!(
        dessiner.contains("document.getElementById(focus)?.focus();"),
        "{dessiner}"
    );

    let reglages = lire("interface/reglages-image.js");
    let case = corps(&reglages, "function pointNoir(coche, changer) {");
    for attendu in [
        "type: \"checkbox\", checked: coche, id: \"image-point-noir\"",
        "changer(caseACocher.checked)",
        "t(\"image.point_noir\")",
        "t(\"image.point_noir.aide\")",
    ] {
        assert!(case.contains(attendu), "{attendu} :\n{case}");
    }
    // Le point cliqué dit la couleur visée, ou que c'est celle de l'image.
    let pastilles = lire("interface/pastilles-image.js");
    let visee = corps(&pastilles, "function visee(p) {");
    for attendu in [
        "t(\"pastilles.pipette.visee_identique\")",
        "pastille(p.visee, p.couleur_visee, \"visee\")",
        "t(\"pastilles.pipette.visee\")",
    ] {
        assert!(visee.contains(attendu), "{attendu} :\n{visee}");
    }
}

/// Les textes de la disposition, de l'image agrandie et de la compensation
/// existent en français et en anglais ; la compensation est rappelée dans la
/// légende.
#[test]
fn les_textes_de_l_image_agrandie_et_du_point_noir_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    for cle in [
        "image.agrandir",
        "image.agrandie.titre",
        "image.agrandie.fermer",
        "image.agrandie.aide",
        "image.point_noir",
        "image.point_noir.aide",
        "image.legende.point_noir",
        "image.rendu.point_noir",
        "image.rendu.relatif",
        "pastilles.pipette.visee",
        "pastilles.pipette.visee_identique",
    ] {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} manque");
        }
    }
    assert_eq!(
        texte(Langue::Francais, "image.point_noir"),
        "Compensation du point noir"
    );
    assert!(
        texte(Langue::Francais, "image.legende.point_noir").contains("compensation du point noir")
    );
    assert!(texte(Langue::Anglais, "image.legende.point_noir").contains("black point compensation"));
    for langue in [Langue::Francais, Langue::Anglais] {
        let visee = texte(langue, "pastilles.pipette.visee");
        assert!(visee.contains("{lab}") && visee.contains("{e}"), "{visee}");
    }
}
