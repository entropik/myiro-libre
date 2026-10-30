//! La page, lue comme un fichier : pastilles des couleurs hors gamut et
//! pipette, sous l'image de la section « Image » (ticket #80).

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

/// pastilles-image.js est chargé après ses outils et avant comparer-image.js
/// qui s'en sert, sous 400 lignes, sans couleur en dur : les seules couleurs
/// sont celles des pastilles, contrôlées (`#rrggbb`) et posées par la
/// variable `--c`. Les nombres passent par `Graphiques.nombre` (virgule).
#[test]
fn pastilles_image_js_est_charge_avant_comparer_image_et_sans_couleur_en_dur() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    let pastilles = position("pastilles-image.js");
    for avant in ["arbre.js", "graphiques.js", "comparer-details.js"] {
        assert!(
            position(avant) < pastilles,
            "{avant} avant pastilles-image.js"
        );
    }
    assert!(pastilles < position("comparer-image.js"));

    let script = lire("interface/pastilles-image.js");
    assert!(script.contains("window.PastillesImage = "));
    let lignes = script.lines().count();
    assert!(lignes < 400, "pastilles-image.js : {lignes} lignes");
    let hexa = script.match_indices('#').any(|(i, _)| {
        script[i + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count()
            >= 3
    });
    assert!(!hexa, "couleur hexadécimale en dur");
    for interdit in ["rgb(", "rgba(", "hsl(", "style=", ".toFixed(", "data:"] {
        assert!(!script.contains(interdit), "{interdit}");
    }
    assert!(script.contains("/^#[0-9a-fA-F]{6}$/"), "couleur contrôlée");
    assert!(script.contains("style.setProperty(\"--c\", "));
    assert!(script.contains("window.Graphiques.nombre("));
    let image = lire("interface/comparer-image.js");
    let lignes = image.lines().count();
    assert!(lignes < 400, "comparer-image.js : {lignes} lignes");
}

/// Deux colonnes : à gauche l'image (légende, « Agrandir », surimpression
/// dans le cadre de la pipette) ; à droite le panneau des réglages, du point
/// cliqué, des pastilles et des chiffres. La pipette demande le pixel avec
/// les sources et la compensation de l'analyse affichée.
#[test]
fn les_pastilles_sont_dans_le_panneau_a_cote_de_l_image() {
    let image = lire("interface/comparer-image.js");
    let surimpression = corps(&image, "function surimpression() {");
    for attendu in [
        "window.PastillesImage.cadre(canvas, () => ({ sources, compensation: pointNoir }))",
        "avisInconnu, window.PastillesImage.section());",
        "el(\"div\", \"image-comparee__panneau\")",
        "el(\"div\", \"image-comparee__image\")",
        "colonnes.append(image, panneau);",
        "tableauChiffres = el(\"div\");",
    ] {
        assert!(
            surimpression.contains(attendu),
            "{attendu} :\n{surimpression}"
        );
    }
    let panneau = surimpression
        .find("image-comparee__panneau")
        .unwrap_or(usize::MAX);
    let chiffres = surimpression.find("t(\"image.chiffres\")").unwrap_or(0);
    assert!(panneau < chiffres, "les chiffres sont dans le panneau");
}

/// Au plus 12 paires, celles au-delà du seuil courant (code d'écart du
/// représentant, même règle que les pixels), dans l'ordre de l'analyse (les
/// plus présentes d'abord) ; la part se calcule sur les pixels à l'écart
/// connu. Le seuil et le profil de la surimpression se suivent sans
/// commande. Sans couleur au-delà du seuil, une phrase le dit ; une ligne
/// de précaution dit que les couleurs sont approchées à l'écran.
#[test]
fn les_pastilles_suivent_le_seuil_et_le_profil_sans_commande() {
    let script = lire("interface/pastilles-image.js");
    for attendu in [
        "const AFFICHEES = 12;",
        "const code = (s) => Math.round(s * 10);",
        ".filter((p) => p.code >= code(seuil)).slice(0, AFFICHEES)",
        "analyse.pixels_comptes - analyse.pixels_inconnus",
        "t(\"pastilles.aucune\")",
        "t(\"pastilles.precaution\")",
    ] {
        assert!(script.contains(attendu), "{attendu}");
    }
    let changer = corps(&script, "function changer(");
    assert!(!changer.contains("invoke("), "{changer}");
    assert!(changer.contains("ecrire();"), "{changer}");

    let image = lire("interface/comparer-image.js");
    let nuage = corps(&image, "function nuage() {");
    assert!(
        nuage.contains("window.PastillesImage.changer(analyseDe(choisi), seuil, choisi, nom);"),
        "{nuage}"
    );
    for entete in [
        "function changerSeuil(valeur) {",
        "function choisirProfil(rang) {",
    ] {
        assert!(corps(&image, entete).contains("nuage();"), "{entete}");
    }
    // Le plus petit seuil du curseur est celui des pastilles de `gamut`.
    assert!(lire("interface/reglages-image.js").contains("min: 0.5, max: 10, step: 0.5,"));
    assert_eq!(gamut::SEUIL_PASTILLES, 0.5);
    assert_eq!(gamut::PASTILLES, 48);
}

/// Chaque pastille est une image pour les lecteurs d'écran : son rôle
/// (« image » ou « imprimée ») et ses L*a*b* arrondis.
#[test]
fn chaque_pastille_a_son_texte_de_remplacement() {
    let script = lire("interface/pastilles-image.js");
    let pastille = corps(&script, "function pastille(");
    for attendu in [
        "setAttribute(\"role\", \"img\")",
        "setAttribute(\"aria-label\", ",
        "t(\"pastilles.aria.\" + role)",
        "nombre(lab[0], 0)",
    ] {
        assert!(pastille.contains(attendu), "{attendu} :\n{pastille}");
    }
    assert!(script.contains("pastille(p.demandee, p.couleur_demandee, \"image\")"));
    assert!(script.contains("pastille(p.reproduite, p.couleur_reproduite, \"imprimee\")"));
}

/// Pipette : un clic sur l'image (quel que soit l'affichage) demande la
/// couleur du pixel touché au module Rust, en coordonnées de l'image
/// affichée ; la dernière demande l'emporte, un refus n'affiche rien. Le
/// résultat suit le profil et le seuil sans nouveau clic. Le point cliqué
/// est marqué sur l'image ; une nouvelle analyse (autres profils,
/// compensation changée) le redemande, une autre image l'oublie. Le cadre
/// garde les proportions de l'image.
#[test]
fn un_clic_sur_l_image_montre_la_couleur_du_point() {
    let script = lire("interface/pastilles-image.js");
    let cliquer = corps(&script, "function cliquer(e, canvas) {");
    for attendu in ["e.offsetX", "canvas.clientWidth", "demanderPixel("] {
        assert!(cliquer.contains(attendu), "{attendu} :\n{cliquer}");
    }
    let demander = corps(&script, "async function demanderPixel(clic) {");
    for attendu in [
        "invoke(\"image_pixel\", { ...demandeur(), point: clic.point })",
        "if (n !== demande) return;",
    ] {
        assert!(demander.contains(attendu), "{attendu} :\n{demander}");
    }
    let reprendre = corps(&script, "function reprendre() {");
    assert!(reprendre.contains("demanderPixel(dernier);"), "{reprendre}");
    let cadre = corps(&script, "function cadre(canvas, demandeurPixel) {");
    assert!(cadre.contains("addEventListener(\"click\""), "{cadre}");
    assert!(cadre.contains("pipette__repere"), "{cadre}");
    assert!(cadre.contains("setProperty(\"--ratio\", "), "{cadre}");
    let resultat = corps(&script, "function pipette() {");
    for attendu in [
        "pixel.profils[rang]",
        "t(\"pastilles.pipette.titre\")",
        "\"pastilles.pipette.au_dela\" : \"pastilles.pipette.en_deca\"",
        // Les L*a*b* arrondis des deux couleurs, écrits sous la paire.
        "lab(v)",
    ] {
        assert!(resultat.contains(attendu), "{attendu} :\n{resultat}");
    }
    assert!(script.contains("t(\"pastilles.lab\")"));
    let image = lire("interface/comparer-image.js");
    let analyser = corps(&image, "async function analyser() {");
    assert!(
        analyser.contains("window.PastillesImage.reprendre();"),
        "{analyser}"
    );
    let ouvrir = corps(&image, "async function ouvrir() {");
    assert!(
        ouvrir.contains("window.PastillesImage.oublier();"),
        "{ouvrir}"
    );

    let css = lire("../design-system/components.css");
    for attendu in [
        ".pastilles {",
        ".pastille-couleur {",
        ".pipette {",
        ".pipette__repere {",
        "cursor: crosshair;",
    ] {
        assert!(css.contains(attendu), "{attendu}");
    }
    let repere = css
        .lines()
        .find(|l| l.starts_with(".pipette__repere {"))
        .unwrap_or_default();
    assert!(
        repere.contains("var(--encre)") && repere.contains("var(--fond)"),
        "repère lisible sur fond clair et sombre : {repere}"
    );
    let kit = lire("../design-system/ui-kit.html");
    for attendu in ["class=\"pastilles\"", "class=\"pipette__repere\""] {
        assert!(kit.contains(attendu), "ui-kit sans {attendu}");
    }
}

/// Chaque texte des pastilles (`t("pastilles.…")`, et le début de clé
/// `pastilles.aria.` complété par le rôle) existe en français et en
/// anglais ; le texte de remplacement de l'image dit qu'un clic montre la
/// couleur.
#[test]
fn les_textes_des_pastilles_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    let script = lire("interface/pastilles-image.js");
    let mut demandees: Vec<String> = script
        .match_indices("\"pastilles.")
        .filter_map(|(i, _)| script[i + 1..].split('"').next())
        .map(String::from)
        .collect();
    assert!(demandees.len() >= 12, "clés lues : {demandees:?}");
    let avant = demandees.len();
    demandees.retain(|c| c != "pastilles.aria.");
    assert!(demandees.len() < avant, "début de clé des rôles absent");
    demandees.extend(
        [
            "pastilles.aria.image",
            "pastilles.aria.imprimee",
            "pastilles.aria.visee",
        ]
        .map(String::from),
    );
    for cle in &demandees {
        assert!(!cle.ends_with('.'), "début de clé inattendu : {cle}");
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle.as_str(), "{cle} manque");
        }
    }
    for langue in [Langue::Francais, Langue::Anglais] {
        for cle in [
            "pastilles.aria.image",
            "pastilles.aria.imprimee",
            "pastilles.aria.visee",
            "pastilles.lab",
        ] {
            let t = texte(langue, cle);
            assert!(
                t.contains("{l}") && t.contains("{a}") && t.contains("{b}"),
                "{t}"
            );
        }
        assert!(texte(langue, "pastilles.aucune").contains("{s}"));
        assert!(texte(langue, "pastilles.lecture").contains("{profil}"));
    }
    assert!(texte(Langue::Francais, "image.canvas.aria").contains("Cliquez sur l’image"));
    assert!(texte(Langue::Anglais, "image.canvas.aria").contains("Click the image"));
    assert!(texte(Langue::Francais, "pastilles.precaution").contains("approchées"));
}
