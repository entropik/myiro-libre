//! La page de l'application, lue comme un fichier : ce que le navigateur
//! ferait de ses attributs et du système graphique.

use std::collections::BTreeSet;
use std::path::Path;

fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif)).unwrap()
}

/// Classes du système graphique dont une règle à elles seules fixe `display`.
fn classes_qui_fixent_display(css: &str) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    for regle in css.split('}') {
        let Some((selecteurs, corps)) = regle.split_once('{') else {
            continue;
        };
        if !corps.contains("display:") {
            continue;
        }
        // Retire les commentaires qui précèdent la règle.
        let selecteurs = selecteurs.rsplit("*/").next().unwrap_or(selecteurs);
        for selecteur in selecteurs.split(',') {
            let s = selecteur.trim();
            if let Some(nom) = s.strip_prefix('.') {
                if nom
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    classes.insert(nom.to_string());
                }
            }
        }
    }
    classes
}

/// Un élément caché par l'attribut `hidden` doit vraiment l'être : une classe
/// qui fixe `display` (`.actions`, `.seg`…) l'emporte sur `hidden` dans le
/// navigateur, et l'élément resterait visible. On cache alors un enveloppant
/// sans classe.
#[test]
fn un_element_cache_ne_porte_pas_de_classe_qui_fixe_display() {
    let classes = classes_qui_fixent_display(&lire("../design-system/components.css"));
    assert!(classes.contains("actions"), "{classes:?}");
    let page = lire("interface/index.html");

    let mut fautes = Vec::new();
    for balise in page.split('<').skip(1) {
        let balise = balise.split('>').next().unwrap_or_default();
        let cachee = balise
            .split_whitespace()
            .any(|attribut| attribut == "hidden" || attribut == "hidden/");
        if !cachee {
            continue;
        }
        let Some((_, reste)) = balise.split_once("class=\"") else {
            continue;
        };
        let valeur = reste.split('"').next().unwrap_or_default();
        if valeur.split_whitespace().any(|c| classes.contains(c)) {
            fautes.push(balise.to_string());
        }
    }
    assert!(fautes.is_empty(), "visibles malgré hidden : {fautes:?}");
}

// ---- Écran refusé le 9 octobre 2026 (« illisible ») : plein cadre lisible ----

/// Règles du système graphique : (sélecteurs, corps sans espaces),
/// commentaires retirés.
fn regles(css: &str) -> Vec<(String, String)> {
    let mut sans_commentaires = String::new();
    let mut reste = css;
    while let Some((avant, apres)) = reste.split_once("/*") {
        sans_commentaires.push_str(avant);
        reste = apres.split_once("*/").map_or("", |(_, r)| r);
    }
    sans_commentaires.push_str(reste);
    sans_commentaires
        .split('}')
        .filter_map(|r| r.split_once('{'))
        .map(|(s, c)| {
            let s = s.rsplit('{').next().unwrap_or(s);
            (s.trim().to_string(), c.replace(' ', ""))
        })
        .collect()
}

/// Corps des règles dont un sélecteur est exactement `selecteur`.
fn regle<'a>(css: &'a [(String, String)], selecteur: &str) -> Vec<&'a str> {
    css.iter()
        .filter(|(s, _)| s.split(',').any(|x| x.trim() == selecteur))
        .map(|(_, c)| c.as_str())
        .collect()
}

fn composants() -> Vec<(String, String)> {
    regles(&lire("../design-system/components.css"))
}

/// Dans l'application, la page prend toute la largeur de la fenêtre : pas
/// de grandes marges vides autour de colonnes serrées.
#[test]
fn l_application_occupe_toute_la_largeur_de_la_fenetre() {
    let css = composants();
    assert!(
        regle(&css, ".app .page")
            .iter()
            .any(|c| c.contains("max-width:none")),
        "{:?}",
        regle(&css, ".app .page")
    );
}

/// Aucune zone ne défile en largeur ; une zone trop longue défile seule en
/// hauteur.
#[test]
fn aucune_zone_ne_defile_en_largeur() {
    let css = composants();
    let zone = regle(&css, ".app > .frame > .grid > *").join(";");
    for attendu in ["overflow-y:auto", "overflow-x:hidden", "min-width:0"] {
        assert!(zone.contains(attendu), "{attendu} : {zone}");
    }
    // Les cases du cartouche se partagent la largeur, même avec un long texte.
    assert!(
        regle(&css, ".cells")
            .iter()
            .any(|c| c.contains("minmax(0,1fr)")),
        "{:?}",
        regle(&css, ".cells")
    );
}

/// Toutes les barres de défilement sont fines, partout.
#[test]
fn toutes_les_barres_de_defilement_sont_fines() {
    let css = composants();
    assert!(
        regle(&css, "*")
            .iter()
            .any(|c| c.contains("scrollbar-width:thin")),
        "pas de règle universelle"
    );
    assert!(css.iter().all(|(_, c)| !c.contains("scrollbar-width:auto")));
}

/// Une entrée de la bibliothèque tient sur deux lignes (nom, puis date et
/// lecture), sans couper les mots.
#[test]
fn une_entree_de_la_bibliotheque_ne_coupe_pas_les_mots() {
    let css = composants();
    assert!(
        css.iter()
            .filter(|(s, _)| s.contains(".index"))
            .all(|(_, c)| !c.contains("overflow-wrap:anywhere")
                && !c.contains("word-break:break-all"))
    );
    for ligne in [".index__nom", ".index__meta"] {
        let corps = regle(&css, ligne).join(";");
        assert!(corps.contains("display:block"), "{ligne} : {corps}");
        assert!(corps.contains("white-space:nowrap"), "{ligne} : {corps}");
    }
    let script = lire("interface/bibliotheque.js");
    assert!(script.contains("\"index__nom\"") && script.contains("\"index__meta\""));
}

/// Deux champs qui se suivent (condition d'impression, mesure automatique
/// ou manuelle) ne se touchent pas.
#[test]
fn deux_champs_qui_se_suivent_sont_espaces() {
    let css = composants();
    assert!(
        regle(&css, ".field + .field")
            .iter()
            .any(|c| c.contains("margin-top:")),
        "{:?}",
        regle(&css, ".field + .field")
    );
}

/// Les boutons de condition de mesure portent seulement « M0 », « M1 »,
/// « M2 » ; « à confirmer » est dit une fois, à côté.
#[test]
fn les_boutons_de_condition_de_mesure_sont_courts() {
    let script = lire("interface/mesurer.js");
    assert!(
        script.contains("el(\"button\", \"\", libelleCourt(f, i))"),
        "boutons longs"
    );
    assert!(script.contains("mesurer.conditions.a_confirmer"));
}

/// Les détails gardent la couleur, les valeurs et la provenance en haut :
/// l'écart, compact, les précède ; les actions sur la référence viennent
/// après la provenance.
#[test]
fn les_details_de_mesurer_gardent_l_essentiel_en_haut() {
    let script = lire("interface/mesurer.js");
    assert!(
        script
            .contains("detail.replaceChildren(...ecart, sectionValeurs, espace, blocReference(f))"),
        "ordre des détails"
    );
}

/// La colonne de la bibliothèque, commune à toutes les tâches, a trois
/// colonnes de la grille sur douze (et non deux) : ses entrées tiennent
/// sur deux lignes même à 1280 pixels. La feuille en prend six, les
/// détails trois. Les marges latérales de l'application sont réduites.
#[test]
fn la_colonne_de_la_bibliotheque_est_assez_large() {
    let page = lire("interface/index.html");
    assert!(page.contains("<aside class=\"c-1-3 index\" data-bibliotheque>"));
    assert!(page.contains("<div class=\"c-4-9\">"));
    assert!(page.contains("<aside class=\"c-10-12\">"));
    let css = composants();
    assert!(regle(&css, ".c-1-3")
        .iter()
        .any(|c| c.contains("grid-column:1/span3")));
    assert!(regle(&css, ".c-4-9")
        .iter()
        .any(|c| c.contains("grid-column:4/span6")));
    let page_app = regle(&css, ".app .page").join(";");
    assert!(
        page_app.contains("padding-left:var(--space-4)")
            && page_app.contains("padding-right:var(--space-4)"),
        "marges : {page_app}"
    );
}

/// Le haut des trois zones garde l'espace sous le bandeau : la marge
/// latérale de l'application ne remet pas à zéro l'espace du haut du cadre
/// (`.page frame`).
#[test]
fn le_haut_des_zones_garde_son_espace_sous_le_bandeau() {
    let css = composants();
    assert!(regle(&css, ".frame")
        .iter()
        .any(|c| c.contains("padding-top:var(--space-8)")));
    for selecteur in [".app .page", ".app > .frame"] {
        for corps in regle(&css, selecteur) {
            assert!(
                !corps.contains("padding:") && !corps.contains("padding-top:"),
                "{selecteur} écrase l'espace du haut : {corps}"
            );
        }
    }
}

/// Un nom trop long dans la liste des mesures finit par des points, et le
/// nom entier se lit au survol.
#[test]
fn un_nom_de_mesure_trop_long_se_lit_au_survol() {
    let css = composants();
    assert!(regle(&css, ".input")
        .iter()
        .any(|c| c.contains("text-overflow:ellipsis")));
    let script = lire("interface/mesurer.js");
    assert!(
        script.contains("nom.title = f.nom;"),
        "pas de nom au survol"
    );
}

/// Une mesure sans nom, à gauche : la lecture en ligne 1, la date courte
/// seule en ligne 2 (jamais la date deux fois).
#[test]
fn une_mesure_sans_nom_ne_montre_pas_sa_date_deux_fois() {
    let script = lire("interface/bibliotheque.js");
    assert!(script.contains("const nom = m.nom || lecture(m.geometrie, m.plages);"));
    assert!(script.contains(
        "const detail = m.nom ? `${dateCourte(m.horodatage)} · ${lecture(m.geometrie, m.plages)}` : dateCourte(m.horodatage);"
    ));
}
