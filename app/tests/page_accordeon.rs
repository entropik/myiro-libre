//! La page, lue comme un fichier : accordéon des grandes parties de la vue
//! « Comparer », ascenseur à gauche des zones imbriquées et bloc compact du
//! point cliqué (contrôle d'écran du mainteneur, 30 octobre 2026).

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

/// Une zone qui défile dans une zone qui défile déjà met son ascenseur à
/// gauche : jamais deux ascenseurs côte à côte. Le panneau de l'image et le
/// tableau d'une feuille (dans le corps de la feuille, qui défile) le font.
#[test]
fn une_zone_imbriquee_a_son_ascenseur_a_gauche() {
    let css = lire("../design-system/components.css");
    let regle = ligne_css(&css, ".defile-gauche {");
    for attendu in [
        "direction: rtl;",
        "padding-right: 0;",
        "padding-left: var(--gouttiere-ascenseur);",
    ] {
        assert!(regle.contains(attendu), "{attendu} : {regle}");
    }
    assert!(ligne_css(&css, ".defile-gauche > * {").contains("direction: ltr;"));
    // La règle vient après celles des zones qu'elle corrige.
    let position = |s: &str| css.find(s).unwrap_or_else(|| panic!("{s} absent"));
    assert!(position(".defile {") < position(".defile-gauche {"));
    assert!(position(".image-comparee__panneau {") < position(".defile-gauche {"));
    let image = lire("interface/comparer-image.js");
    assert!(image.contains("el(\"div\", \"image-comparee__panneau defile-gauche\")"));
    let bibliotheque = lire("interface/bibliotheque.js");
    assert!(bibliotheque.contains("el(\"div\", \"defile defile-gauche\")"));
    assert!(lire("../design-system/README.md").contains("defile-gauche"));
    assert!(lire("../design-system/ui-kit.html").contains("defile-gauche"));
}

/// Chaque grande partie de la vue « Comparer » est une partie d'accordéon :
/// un bouton d'en-tête (Entrée, Espace), `aria-expanded`, `aria-controls`,
/// chevron décoratif, corps en région. Par défaut, seuls les chiffres clés
/// sont ouverts ; l'état est retenu dans le navigateur, dans un `try` (la page
/// marche sans). Une partie lourde ne se remplit qu'ouverte et dans la page,
/// et se vide à la fermeture.
#[test]
fn les_grandes_parties_sont_repliables() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("arbre.js") < position("accordeon.js"));
    assert!(position("accordeon.js") < position("comparer-ecarts.js"));
    let script = lire("interface/accordeon.js");
    assert!(script.lines().count() < 400);
    let partie = corps(
        &script,
        "function partie(cle, contenu = [], remplir = null, vider = null) {",
    );
    for attendu in [
        "el(\"button\", \"accordeon__bouton\")",
        "type: \"button\"",
        "setAttribute(\"aria-controls\", id)",
        "setAttribute(\"aria-expanded\", String(ouvrir))",
        "setAttribute(\"role\", \"region\")",
        "setAttribute(\"aria-labelledby\", bouton.id)",
        "lastChild.setAttribute(\"aria-hidden\", \"true\")",
        "corps.hidden = !ouvrir;",
        "if (ouvrir && !rempli && corps.isConnected) {",
        "if (vider) vider();",
    ] {
        assert!(partie.contains(attendu), "{attendu} :\n{partie}");
    }
    assert!(script.contains("const OUVERTES_PAR_DEFAUT = new Set([\"comparer.chiffres\"]);"));
    for entete in ["function lire() {", "function garder() {"] {
        let f = corps(&script, entete);
        assert!(
            f.contains("try {") && f.contains("} catch {"),
            "{entete} :\n{f}"
        );
    }

    // Toutes les grandes parties passent par l'accordéon ; les lourdes se remplissent à l'ouverture.
    let details = lire("interface/comparer-details.js");
    let section = corps(&details, "function section(cle, ...contenu) {");
    assert!(section.contains("window.Accordeon.partie(cle, [], contenu[0], contenu[1])"));
    assert!(section.contains("window.Accordeon.partie(cle, contenu)"));
    let comparer = lire("interface/comparer.js");
    let volume = corps(&comparer, "function volume3d(liste) {");
    assert!(volume
        .contains("section(\"comparer.volume\", (zone) => remplirVolume(zone, liste), () => {"));
    assert!(volume.contains("vue3d.detruire(); vue3d = null;"));
    let graphiques = corps(&comparer, "function graphiques(liste) {");
    for attendu in [
        "section(\"comparer.teintes\", (zone) => {",
        "section(\"comparer.coupe\", (zone) => {",
        "}, () => { coupeNuage = () => {}; });",
        "section(\"comparer.chemin\", (zone) => {",
    ] {
        assert!(graphiques.contains(attendu), "{attendu} :\n{graphiques}");
    }
    assert!(comparer.contains("section(\"comparer.primaires\", "));
    let ecarts = lire("interface/comparer-ecarts.js");
    assert!(ecarts.contains("window.Accordeon.partie(\"comparer.ecarts\", [section])"));
    // Ouvrir une image ouvre la partie « Image ».
    let image = lire("interface/comparer-image.js");
    let ouvrir = corps(&image, "async function ouvrir() {");
    assert!(
        ouvrir.contains("window.Accordeon.ouvrir(\"image.titre\");"),
        "{ouvrir}"
    );
    for fichier in ["comparer.js", "comparer-image.js"] {
        assert!(
            lire(&format!("interface/{fichier}")).lines().count() < 400,
            "{fichier}"
        );
    }

    let css = lire("../design-system/components.css");
    let bouton = ligne_css(&css, ".accordeon__bouton {");
    for attendu in ["width: 100%;", "cursor: pointer;", "text-align: left;"] {
        assert!(bouton.contains(attendu), "{attendu} : {bouton}");
    }
    assert!(ligne_css(
        &css,
        ".accordeon__bouton[aria-expanded=\"true\"] .accordeon__chevron {"
    )
    .contains("transform: rotate(45deg);"));
    assert!(ligne_css(&css, ".accordeon__bouton:focus-visible {").contains("outline:"));
    assert!(lire("../design-system/ui-kit.html").contains("class=\"accordeon__bouton\""));
    assert!(lire("../design-system/README.md").contains("Accordéon"));
}

/// Le bloc du point cliqué a toujours la hauteur d'une paire remplie : une
/// ligne de titre puis cinq lignes (Image, Imprimé, écart, seuil, visée) à
/// côté des deux pastilles ; vide, il garde la même grille avec deux carrés
/// vides et la phrase. L'état de l'analyse tient sur une ligne.
#[test]
fn le_point_clique_et_l_etat_de_l_analyse_sont_compacts() {
    let css = lire("../design-system/components.css");
    let bloc = ligne_css(&css, ".pastilles-pipette {");
    assert!(bloc.contains(
        "grid-template-rows: calc(1em * var(--leading-normal)) calc(5em * var(--leading-normal));"
    ));
    assert!(!bloc.contains("var(--space-24)"), "{bloc}");
    assert!(ligne_css(&css, ".paire__lignes {")
        .contains("grid-template-rows: repeat(5, calc(1em * var(--leading-normal)));"));
    assert!(ligne_css(&css, ".paire__lignes > * {").contains("white-space: nowrap;"));
    let recalcul = ligne_css(&css, ".recalcul {");
    assert!(recalcul.contains("height: calc(1em * var(--leading-normal) + var(--space-1) * 2);"));
    assert!(ligne_css(&css, ".recalcul > p {").contains("white-space: nowrap;"));

    let pastilles = lire("interface/pastilles-image.js");
    let pipette = corps(&pastilles, "function pipette() {");
    for attendu in [
        "el(\"div\", \"paire paire--pipette\")",
        "el(\"div\", \"paire__lignes\")",
        "ligne.append(vide().firstChild, lignes);",
        "ligne.append(couleurs, lignes);",
    ] {
        assert!(pipette.contains(attendu), "{attendu} :\n{pipette}");
    }
    let visee = corps(&pastilles, "function visee(p) {");
    assert!(visee.contains("ligne.title = texte;"), "{visee}");
}
