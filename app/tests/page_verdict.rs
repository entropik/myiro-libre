//! La page, lue comme un fichier : verdict « x % de l'image est imprimable
//! sur ce papier avec ce profil », au-dessus de la photo (ticket #80, demande
//! du mainteneur du 30 octobre 2026).

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

/// La part imprimable est celle des pixels à l'écart connu en deçà du seuil
/// (la règle des chiffres : code d'écart < round(10 × seuil)), sur les pixels
/// à l'écart connu ; sans pixel connu, elle est inconnue. Les deux bornes :
/// jamais « 100 » si un seul pixel connu dépasse (« plus de 99,9 »), jamais
/// « 0 » si un seul est bon (« moins de 0,1 »), à 0,1 % près par le
/// formateur. Les pixels à l'écart inconnu sont dits à part.
#[test]
fn la_part_imprimable_suit_la_regle_des_chiffres_et_ses_bornes() {
    let script = lire("interface/verdict-image.js");
    let imprimables = corps(&script, "function imprimables(a, seuil) {");
    for attendu in [
        "const connus = a.pixels_comptes - a.pixels_inconnus;",
        "if (!connus) return null;",
        "a.histogramme.slice(Math.round(seuil * 10))",
        "return { bons: connus - au_dela, connus };",
    ] {
        assert!(imprimables.contains(attendu), "{attendu} :\n{imprimables}");
    }
    let pourcent = corps(&script, "function pourcent(n, total) {");
    for attendu in [
        "const p = (n / total) * 100;",
        "if (n < total && p >= 99.95) return t(\"image.verdict.plus_de\").replace(\"{p}\", () => nombre(99.9, 1));",
        "if (n > 0 && p < 0.05) return t(\"image.verdict.moins_de\").replace(\"{p}\", () => nombre(0.1, 1));",
        "return nombre(p, 1);",
    ] {
        assert!(pourcent.contains(attendu), "{attendu} :\n{pourcent}");
    }
    // 99,95 est la plus petite part que l'arrondi à 0,1 écrirait « 100,0 » ; 0,05 la plus grande
    // qu'il écrirait « 0,0 ».
    assert_eq!(format!("{:.1}", 99.95_f64 + 1e-9), "100.0");
    assert_eq!(format!("{:.1}", 0.05_f64 - 1e-9), "0.0");
    let ecrire = corps(&script, "function ecrire(bloc, a, seuil, nom, compense) {");
    for attendu in [
        "pourcent(n.bons, n.connus)",
        "t(\"image.verdict.inconnu\")",
        "if (a && a.pixels_inconnus) {",
        "pourcent(a.pixels_inconnus, a.pixels_comptes)",
        "t(compense ? \"image.legende.point_noir\" : \"image.legende\")",
    ] {
        assert!(ecrire.contains(attendu), "{attendu} :\n{ecrire}");
    }
    assert!(!script.contains(".toFixed("));
}

/// Le verdict prend la place gardée de la légende, au-dessus de la photo :
/// hauteur fixe de la barre, phrase en grand sur deux lignes au plus, détail
/// en petit avec le carré vert ; il suit le seuil, le profil et la
/// compensation par le calque, sans commande, et reste atténué pendant une
/// analyse relancée sur place.
#[test]
fn le_verdict_est_au_dessus_de_l_image_a_une_place_gardee() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("graphiques.js") < position("verdict-image.js"));
    assert!(position("verdict-image.js") < position("comparer-image.js"));
    let script = lire("interface/verdict-image.js");
    assert!(script.lines().count() < 400);
    let hexa = script.match_indices('#').any(|(i, _)| {
        script[i + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count()
            >= 3
    });
    assert!(!hexa, "couleur en dur");

    let image = lire("interface/comparer-image.js");
    let surimpression = corps(&image, "function surimpression() {");
    assert!(surimpression.contains("legende = window.VerdictImage.creer();"));
    let calque = corps(&image, "function calque() {");
    assert!(calque.contains("window.VerdictImage.ecrire(legende, a, seuil, nom, compense);"));
    let recalculer = corps(&image, "async function recalculer() {");
    for attendu in [
        "window.VerdictImage.perime(legende, true);",
        "window.VerdictImage.perime(legende, false);",
    ] {
        assert!(recalculer.contains(attendu), "{attendu} :\n{recalculer}");
    }
    for entete in [
        "function changerSeuil(valeur) {",
        "function choisirProfil(rang) {",
    ] {
        let f = corps(&image, entete);
        assert!(
            f.contains("calque();") && !f.contains("invoke("),
            "{entete} :\n{f}"
        );
    }

    let css = lire("../design-system/components.css");
    let ligne = |selecteur: &str| {
        css.lines()
            .find(|l| l.starts_with(selecteur))
            .unwrap_or_else(|| panic!("{selecteur} absent"))
            .to_string()
    };
    assert!(ligne(".image-comparee__barre {")
        .contains("height: calc(var(--space-16) + var(--space-6));"));
    let part = ligne(".verdict__part {");
    for attendu in [
        "font-size: var(--text-lg);",
        "font-weight: 600;",
        "-webkit-line-clamp: 2;",
        "overflow: hidden;",
    ] {
        assert!(part.contains(attendu), "{attendu} : {part}");
    }
    assert!(ligne(".verdict--perime {").contains("opacity: .45;"));
    assert!(lire("../design-system/ui-kit.html").contains("class=\"verdict__part\""));
}

/// Les textes du verdict existent en français et en anglais, avec la part
/// `{p}` ; la ligne de détail garde le seuil, le profil et le rendu.
#[test]
fn les_textes_du_verdict_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    for cle in [
        "image.verdict",
        "image.verdict.plus_de",
        "image.verdict.moins_de",
        "image.verdict.inconnus",
        "image.verdict.inconnu",
        "image.legende",
        "image.legende.point_noir",
    ] {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} manque");
        }
    }
    assert!(texte(Langue::Francais, "image.verdict")
        .contains("de l’image est imprimable sur ce papier avec ce profil"));
    assert!(texte(Langue::Anglais, "image.verdict")
        .contains("% of the image is printable on this paper with this profile"));
    for langue in [Langue::Francais, Langue::Anglais] {
        for cle in [
            "image.verdict",
            "image.verdict.plus_de",
            "image.verdict.moins_de",
            "image.verdict.inconnus",
        ] {
            assert!(texte(langue, cle).contains("{p}"), "{cle}");
        }
        for cle in ["image.legende", "image.legende.point_noir"] {
            let t = texte(langue, cle);
            assert!(t.contains("{s}") && t.contains("{profil}"), "{t}");
        }
    }
}
