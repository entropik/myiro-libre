//! La page, lue comme un fichier : nuage de couleurs de l'image dans la vue
//! 3D et la coupe de la vue « Comparer » (ticket #80).

use std::path::Path;

/// Fichier de la page, fins de ligne ramenées à `\n`.
fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif))
        .unwrap()
        .replace("\r\n", "\n")
}

/// Corps d'une fonction `entete` de `script`, jusqu'à sa fin indentée de
/// `retrait` espaces.
fn corps<'a>(script: &'a str, entete: &str, retrait: usize) -> &'a str {
    let corps = script
        .split(entete)
        .nth(1)
        .unwrap_or_else(|| panic!("{entete} absent"));
    let fin = format!("\n{}}}", " ".repeat(retrait));
    &corps[..corps.find(&fin).unwrap_or(corps.len())]
}

fn sans_couleur_en_dur(nom: &str, script: &str) {
    let hexa = script.match_indices('#').any(|(i, _)| {
        script[i + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count()
            >= 3
    });
    assert!(!hexa, "{nom} : couleur hexadécimale en dur");
    for interdit in ["rgba(", "hsl(", "style="] {
        assert!(!script.contains(interdit), "{nom} : {interdit}");
    }
}

/// nuage-image.js porte le nuage du profil de la surimpression : chargé
/// avant vue3d.js, comparer-image.js et comparer.js qui s'en servent, sous
/// 400 lignes, sans couleur en dur. La couleur d'alerte est le jeton
/// `--couleur-hors-gamut`, lu par la vue 3D avec ses autres jetons.
#[test]
fn nuage_image_js_est_charge_avant_ses_utilisateurs_et_sans_couleur_en_dur() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    let nuage = position("nuage-image.js");
    for apres in ["vue3d.js", "comparer-image.js", "comparer.js"] {
        assert!(nuage < position(apres), "nuage-image.js avant {apres}");
    }
    let script = lire("interface/nuage-image.js");
    assert!(script.contains("window.NuageImage = "));
    sans_couleur_en_dur("nuage-image.js", &script);
    assert!(!script.contains("rgb("), "nuage-image.js : rgb(");
    sans_couleur_en_dur("graphiques.js", &lire("interface/graphiques.js"));
    let vue = lire("interface/vue3d.js");
    assert!(vue.contains("alerte: jeton(\"--couleur-hors-gamut\"),"));
    for nom in [
        "nuage-image.js",
        "vue3d.js",
        "graphiques.js",
        "comparer.js",
        "comparer-image.js",
    ] {
        let lignes = lire(&format!("interface/{nom}")).lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
    }
}

/// Dans la vue 3D, les points de l'image sont peints après les triangles
/// (au-dessus de la surface), triés du plus loin au plus proche, en carrés
/// de 3 px × devicePixelRatio. Chacun a sa couleur réelle, ou la couleur
/// d'alerte au-delà du seuil, et un contour à l'encre du thème. Le texte de
/// remplacement compte les points et ceux en alerte.
#[test]
fn la_vue_3d_peint_le_nuage_au_dessus_des_triangles() {
    let vue = lire("interface/vue3d.js");
    let dessiner = corps(&vue, "function dessiner() {", 4);
    assert!(
        dessiner
            .contains("dessinerTriangles(ctx, k);\n      window.NuageImage.peindre(ctx, jetons, "),
        "{dessiner}"
    );
    assert!(vue.contains("points(liste) {"), "Vue3d sans points(liste)");
    let decrire = corps(&vue, "function decrire() {", 4);
    assert!(
        decrire.contains("window.NuageImage.decrire(nuage)"),
        "{decrire}"
    );

    let script = lire("interface/nuage-image.js");
    let peindre = corps(
        &script,
        "function peindre(ctx, jetons, projeter, points) {",
        2,
    );
    for attendu in [
        ".sort((u, v) => v.e[2] - u.e[2])",
        "3 * ratio",
        "p.alerte ? jetons.alerte : ",
        "ctx.strokeStyle = jetons.encre;",
        "ctx.fillRect(",
        "ctx.strokeRect(",
    ] {
        assert!(peindre.contains(attendu), "{attendu} :\n{peindre}");
    }
    assert!(
        script.contains("/^#[0-9a-fA-F]{6}$/"),
        "couleur du point contrôlée"
    );
}

/// La coupe reçoit les points en quatrième paramètre, facultatif : sans
/// eux, elle ne change pas. Elle ne garde que ceux dont la clarté est à
/// ± une demi-tranche (2,5 L*) de la clarté choisie, en petits cercles
/// au-dessus des contours, légendés et comptés dans « Valeurs ».
#[test]
fn la_coupe_montre_les_couleurs_de_l_image_proches_de_la_clarte() {
    let graphiques = lire("interface/graphiques.js");
    assert!(graphiques.contains("const DEMI_TRANCHE = 2.5;"));
    let coupe = corps(
        &graphiques,
        "function coupe(svg, series, l, points = []) {",
        2,
    );
    for attendu in [
        "Math.abs(p.lab[0] - l) <= DEMI_TRANCHE",
        "noeud(\"circle\"",
        "graphique__nuage",
        "t(\"nuage.legende\")",
        "t(\"nuage.valeurs\")",
        "t(\"nuage.valeurs.alerte\")",
        "t(\"nuage.aria\")",
    ] {
        assert!(coupe.contains(attendu), "{attendu} :\n{coupe}");
    }
    let traces = coupe
        .find("trace(svg, points, s, true)")
        .unwrap_or(usize::MAX);
    let cercles = coupe.find("noeud(\"circle\"").unwrap_or(0);
    assert!(
        traces < cercles,
        "les points passent au-dessus des contours"
    );

    let css = lire("../design-system/components.css");
    assert!(
        css.contains(".graphique__nuage {"),
        "composant du nuage absent"
    );
    assert!(css.contains(".graphique__nuage--alerte { fill: var(--couleur-hors-gamut);"));

    let comparer = lire("interface/comparer.js");
    assert!(
        comparer.contains("reglages.l, window.NuageImage.points());"),
        "la coupe reçoit le nuage"
    );
}

/// Le nuage affiché est celui du profil de la surimpression. Il suit le
/// seuil sans nouveau calcul : un point est en alerte si son plus grand
/// écart, en code d'écart (dixièmes arrondis vers le bas, bornés à 254),
/// atteint `round(10 × seuil)`, la règle des pixels de la surimpression.
/// La bascule « Couleurs de l'image » est cochée à l'ouverture d'une image ;
/// sans nuage, elle est inactive et dit pourquoi.
#[test]
fn le_nuage_suit_le_seuil_et_le_profil_de_la_surimpression() {
    let image = lire("interface/comparer-image.js");
    let changer = corps(&image, "function changerSeuil(valeur) {", 2);
    assert!(changer.contains("nuage();"), "{changer}");
    assert!(!changer.contains("invoke("), "{changer}");
    // Un autre profil de surimpression, ou un nouveau dessin de la section, refait le nuage.
    let choisir = corps(&image, "function choisirProfil(rang) {", 2);
    assert!(choisir.contains("nuage();"), "{choisir}");
    let dessiner = corps(&image, "function dessiner() {", 2);
    assert!(dessiner.contains("choisirProfil(choisi);"), "{dessiner}");
    assert!(image.contains("window.NuageImage.changer(analyseDe(choisi), seuil, "));

    let script = lire("interface/nuage-image.js");
    assert!(
        script.contains("Math.min(Math.floor(p.ecart_max * 10), 254) >= Math.round(seuil * 10)")
    );
    let changer = corps(&script, "function changer(", 2);
    assert!(
        changer.contains("montre = true;"),
        "cochée à l'ouverture :\n{changer}"
    );
    let champ = corps(&script, "function champ() {", 2);
    for attendu in ["type = \"checkbox\"", "t(\"nuage.bascule\")"] {
        assert!(champ.contains(attendu), "{attendu} :\n{champ}");
    }
    assert!(script.contains("t(\"nuage.indisponible\")"));
    assert!(script.contains(".disabled = "));

    let comparer = lire("interface/comparer.js");
    assert!(comparer.contains("window.NuageImage.suivre("));
    assert!(comparer.contains("vue3d.points(window.NuageImage.points());"));
    let volume = corps(&comparer, "function volume3d(liste) {", 2);
    assert!(volume.contains("window.NuageImage.champ()"), "{volume}");
}

/// Chaque texte du nuage (`t("nuage.…")`) existe en français et en anglais.
#[test]
fn les_textes_du_nuage_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    let script = lire("interface/nuage-image.js") + &lire("interface/graphiques.js");
    let demandees: Vec<&str> = script
        .match_indices("t(\"nuage.")
        .filter_map(|(i, _)| script[i + 3..].split('"').next())
        .collect();
    assert!(demandees.len() >= 7, "clés lues : {demandees:?}");
    let absentes: Vec<_> = demandees
        .iter()
        .filter(|c| {
            [Langue::Francais, Langue::Anglais]
                .iter()
                .any(|l| texte(*l, c) == **c)
        })
        .collect();
    assert!(absentes.is_empty(), "clés absentes : {absentes:?}");
    for langue in [Langue::Francais, Langue::Anglais] {
        assert!(texte(langue, "nuage.aide").contains("{profil}"));
        assert!(texte(langue, "nuage.legende").contains("{l}"));
    }
}
