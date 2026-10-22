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

/// L'application occupe toute la fenêtre : la barre du haut (64 px), puis
/// les trois zones ; la fenêtre elle-même ne défile jamais.
#[test]
fn l_application_occupe_toute_la_largeur_de_la_fenetre() {
    let css = composants();
    let app = regle(&css, ".app:not([hidden])").join(";");
    for attendu in ["height:100vh", "overflow:hidden", "grid-template-rows:64px"] {
        assert!(app.contains(attendu), "{attendu} : {app}");
    }
    assert!(!app.contains("max-width"), "{app}");
}

/// Aucune zone ne défile en largeur ; une zone trop longue défile seule en
/// hauteur.
#[test]
fn aucune_zone_ne_defile_en_largeur() {
    let css = composants();
    for zone in [".zone-gauche", ".zone-centre", ".zone-droite"] {
        let corps = regle(&css, zone).join(";");
        for attendu in ["overflow-y:auto", "overflow-x:hidden", "min-width:0"] {
            assert!(corps.contains(attendu), "{zone} : {attendu} : {corps}");
        }
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

/// Les trois zones : bibliothèque à gauche (270 px), centre souple, détails
/// à droite (300 px).
#[test]
fn les_trois_zones_ont_leurs_largeurs() {
    let css = composants();
    let corps = regle(&css, ".zones").join(";");
    let colonnes = corps
        .split(';')
        .find_map(|d| d.strip_prefix("grid-template-columns:"))
        .unwrap_or_default();
    assert!(colonnes.starts_with("270px"), "{corps}");
    assert!(colonnes.ends_with("300px"), "{corps}");
}

/// Les jetons du thème sombre, par défaut, et du thème clair sont ceux du
/// prototype validé.
#[test]
fn les_jetons_sombres_sont_ceux_du_prototype() {
    let jetons = lire("../design-system/tokens.css");
    for valeur in [
        "#141a26", "#1b2333", "#1e2738", "#222c3f", "#2e3a50", "#e8ecf3", "#8d99ad", "#3d8bff",
        "#2fbf71", "#e8a23a", "#ff5d64", "#173b6b",
    ] {
        assert!(jetons.contains(valeur), "{valeur}");
    }
    for valeur in [
        "#eef1f6", "#ffffff", "#f6f8fb", "#d6dce6", "#5d6879", "#1f6fe5", "#13864b", "#a8650a",
        "#cc1f29", "#dbe8fb",
    ] {
        assert!(jetons.contains(valeur), "clair : {valeur}");
    }
    assert!(jetons.contains(":root[data-theme=\"light\"]"));
}

/// Aucune couleur écrite en dur dans les composants : tout passe par les
/// jetons.
#[test]
fn aucune_couleur_hors_des_jetons() {
    let css = composants();
    let fautes: Vec<_> = css
        .iter()
        .filter(|(_, c)| {
            c.contains("rgb(")
                || c.contains("rgba(")
                || c.contains("hsl(")
                || c.match_indices('#').any(|(i, _)| {
                    c[i + 1..]
                        .chars()
                        .take_while(char::is_ascii_hexdigit)
                        .count()
                        >= 3
                })
        })
        .collect();
    assert!(fautes.is_empty(), "{fautes:?}");
}

/// Ni ombre, ni dégradé ; des angles droits, sauf la pastille de
/// l'instrument (un cercle).
#[test]
fn aucune_ombre_ni_degrade_ni_arrondi_hors_pastille() {
    let css = composants();
    for (selecteurs, corps) in &css {
        assert!(!corps.contains("box-shadow"), "{selecteurs}");
        assert!(!corps.contains("gradient("), "{selecteurs}");
        if corps.contains("border-radius") {
            assert!(
                selecteurs
                    .split(',')
                    .all(|s| s.trim().starts_with(".pastille")),
                "arrondi hors pastille : {selecteurs}"
            );
        }
    }
}

/// Un nom long de l'arbre (« Hahnemühle Photo Rag 308 ») finit par des
/// points sans déborder ; le nom entier se lit au survol.
#[test]
fn un_nom_long_de_l_arbre_se_lit_au_survol() {
    let css = composants();
    let corps = regle(&css, ".noeud__nom").join(";");
    for attendu in [
        "overflow:hidden",
        "text-overflow:ellipsis",
        "white-space:nowrap",
        "min-width:0",
    ] {
        assert!(corps.contains(attendu), "{attendu} : {corps}");
    }
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

/// Un nœud de l'arbre de la bibliothèque tient sur deux lignes au plus (nom,
/// puis date et lecture), sans couper les mots.
#[test]
fn une_entree_de_la_bibliotheque_ne_coupe_pas_les_mots() {
    let css = composants();
    assert!(
        css.iter()
            .filter(|(s, _)| s.contains(".noeud"))
            .all(|(_, c)| !c.contains("overflow-wrap:anywhere")
                && !c.contains("word-break:break-all"))
    );
    for ligne in [".noeud__nom", ".noeud__meta"] {
        let corps = regle(&css, ligne).join(";");
        assert!(corps.contains("display:block"), "{ligne} : {corps}");
        assert!(corps.contains("white-space:nowrap"), "{ligne} : {corps}");
    }
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

/// Le centre garde son espace sous la barre du haut et sur les côtés : rien
/// ne le remet à zéro.
#[test]
fn le_haut_des_zones_garde_son_espace_sous_le_bandeau() {
    let css = composants();
    let centre = regle(&css, ".zone-centre");
    assert!(
        centre.iter().any(|c| c.contains("padding:var(--space-6)")),
        "{centre:?}"
    );
    assert!(centre.iter().all(|c| !c.contains("padding-top:0")));
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

// ---- Cadre à trois zones et cinq tâches (ticket #68) ----

/// Valeur d'un attribut dans une balise (`<button … data-tache="x" …>`).
fn attribut<'a>(balise: &'a str, nom: &str) -> Option<&'a str> {
    let motif = format!(" {nom}=\"");
    let (_, reste) = balise.split_once(&motif)?;
    reste.split('"').next()
}

/// Balises ouvrantes de la page, dans l'ordre, avec leur position.
fn balises(page: &str) -> Vec<(usize, &str)> {
    page.match_indices('<')
        .map(|(i, _)| (i, page[i..].split('>').next().unwrap_or_default()))
        .collect()
}

/// La barre du haut a les cinq tâches, dans l'ordre du poste unique, chacune
/// avec son icône et son libellé du catalogue ; l'application s'ouvre sur la
/// Bibliothèque. Trois zones sous la barre.
#[test]
fn la_barre_a_cinq_taches_dans_l_ordre_avec_une_icone() {
    let page = lire("interface/index.html");
    let taches: Vec<(usize, &str)> = balises(&page)
        .into_iter()
        .filter(|(_, b)| b.starts_with("<button") && attribut(b, "data-tache").is_some())
        .collect();
    let noms: Vec<&str> = taches
        .iter()
        .filter_map(|(_, b)| attribut(b, "data-tache"))
        .collect();
    assert_eq!(
        noms,
        [
            "bibliotheque",
            "profiler",
            "mesurer",
            "controler",
            "imprimer"
        ]
    );
    for (debut, balise) in &taches {
        let nom = attribut(balise, "data-tache").unwrap_or_default();
        let bouton = page[*debut..].split("</button>").next().unwrap_or_default();
        assert!(bouton.contains("<svg"), "{nom} sans icône");
        assert!(
            bouton.contains(&format!("data-t=\"tache.{nom}\"")),
            "{nom} sans libellé"
        );
    }
    assert!(
        taches[0].1.contains("aria-current=\"page\""),
        "ouverture sur Bibliothèque"
    );
    for zone in ["gauche", "centre", "droite"] {
        assert!(
            page.contains(&format!("data-zone=\"{zone}\"")),
            "zone {zone}"
        );
    }
    for tache in noms {
        assert!(
            page.contains(&format!("data-vue=\"{tache}\"")),
            "vue {tache}"
        );
    }
    assert!(page.contains("data-ariane"));
}

/// Tout bouton principal inactif dans la page est suivi, juste après et
/// avant tout autre bouton, d'une raison (`<p class="why" data-t=…>`).
#[test]
fn chaque_bouton_principal_inactif_dit_pourquoi() {
    let page = lire("interface/index.html");
    let mut vus = 0;
    for (debut, balise) in balises(&page) {
        let principal = balise.starts_with("<button")
            && attribut(balise, "class").is_some_and(|c| c.split(' ').any(|c| c == "btn--primary"));
        if !principal || !balise.split_whitespace().any(|a| a == "disabled") {
            continue;
        }
        vus += 1;
        let apres = &page[debut + balise.len()..];
        let Some(position) = apres.find("<p class=\"why\"") else {
            panic!("sans raison : {balise}");
        };
        let entre = &apres[..position];
        assert!(
            !entre.contains("<button") && !entre.contains("<section"),
            "raison trop loin : {balise}"
        );
        let raison = apres[position..].split('>').next().unwrap_or_default();
        assert!(
            raison.contains(" data-t=\""),
            "raison sans texte : {raison}"
        );
    }
    assert!(vus >= 4, "boutons principaux inactifs vus : {vus}");
}

/// La pastille de l'instrument a une règle pour chacun de ses cinq états.
#[test]
fn la_pastille_connait_les_cinq_etats() {
    let css = composants();
    for etat in [
        "non-detecte",
        "connecte",
        "etalonnage-requis",
        "etalonne",
        "en-mesure",
    ] {
        let selecteur = format!(".pastille--{etat}");
        assert!(
            regle(&css, &selecteur)
                .iter()
                .any(|c| c.contains("background:")),
            "{selecteur}"
        );
    }
    let script = lire("interface/app.js");
    assert!(script.contains("pastille--"), "app.js ne pose pas l'état");
}
