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

/// L'arbre pose ses noms et ses lignes de détail avec ces classes, et le
/// nom entier au survol.
#[test]
fn l_arbre_utilise_les_lignes_qui_ne_debordent_pas() {
    let script = lire("interface/arbre.js");
    assert!(script.contains("\"noeud__nom\"") && script.contains("\"noeud__meta\""));
    assert!(script.contains(".title = "), "pas de nom au survol");
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
    // Les lignes de l'arbre de la bibliothèque viennent d'arbre.js (ticket #68).
    let script = lire("interface/arbre.js");
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

// ---- Bibliothèque en arbre et vue d'une résolution (ticket #68) ----

/// L'élément entier (balise ouvrante comprise) qui porte `attribut`, en
/// comptant les éléments de même nom imbriqués.
fn element<'a>(page: &'a str, attribut: &str) -> &'a str {
    let Some(position) = page.find(attribut) else {
        panic!("{attribut} absent de la page");
    };
    let debut = page[..position].rfind('<').unwrap_or(0);
    let nom: String = page[debut + 1..]
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let (ouvrante, fermante) = (format!("<{nom}"), format!("</{nom}>"));
    let mut profondeur = 0usize;
    let mut i = debut;
    while i < page.len() {
        let reste = &page[i..];
        if reste.starts_with(&fermante) {
            profondeur -= 1;
            if profondeur == 0 {
                return &page[debut..i + fermante.len()];
            }
            i += fermante.len();
        } else if reste.starts_with(&ouvrante) && reste[ouvrante.len()..].starts_with([' ', '>']) {
            profondeur += 1;
            i += ouvrante.len();
        } else {
            i += reste.chars().next().map_or(1, char::len_utf8);
        }
    }
    panic!("{attribut} : élément non fermé");
}

#[test]
fn parc_js_est_charge_apres_app_js() {
    let page = lire("interface/index.html");
    let app = page.find("<script src=\"app.js\" defer>");
    let parc = page.find("<script src=\"parc.js\" defer>");
    assert!(app.is_some() && parc.is_some(), "scripts absents");
    assert!(app < parc, "parc.js avant app.js");
}

#[test]
fn la_zone_gauche_a_recherche_arbre_et_ajout_d_imprimante() {
    let page = lire("interface/index.html");
    let gauche = element(&page, "data-zone=\"gauche\"");
    for crochet in ["data-recherche", "data-arbre", "data-ajouter-imprimante"] {
        assert!(
            gauche.contains(crochet),
            "{crochet} absent de la zone gauche"
        );
    }
    assert!(gauche.contains("<input") && gauche.contains("type=\"search\""));
    let bouton = gauche
        .split('<')
        .find(|b| b.contains("data-ajouter-imprimante"))
        .unwrap_or_default();
    assert!(bouton.starts_with("button"), "{bouton}");
}

#[test]
fn la_vue_d_une_resolution_a_un_seul_bouton_principal() {
    let page = lire("interface/index.html");
    let vue = element(&page, "data-vue-reglage");
    assert_eq!(vue.matches("btn--primary").count(), 1, "{vue}");
    let principal = vue
        .split('<')
        .find(|b| b.contains("btn--primary"))
        .unwrap_or_default();
    assert!(principal.contains("data-installer"), "{principal}");

    // « Calculer un profil » ouvre la section ; ce n'est pas un bouton principal.
    let ouvrir = vue
        .split('<')
        .find(|b| b.contains("data-calcul-ouvrir"))
        .and_then(|b| b.split('>').next())
        .expect("bouton « Calculer un profil »");
    assert!(ouvrir.starts_with("button class=\"btn\""), "{ouvrir}");
    assert!(ouvrir.contains("data-t=\"profil.titre\""), "{ouvrir}");
    assert!(ouvrir.contains("aria-expanded=\"false\""), "{ouvrir}");
    assert!(vue.contains("data-calcul-profil"), "section absente");
}

// ---- Calcul d'un profil depuis la vue d'une résolution (ticket #40) ----

/// La section du calcul vit dans calcul-profil.js, chargé après arbre.js et
/// avant reglage.js, qui s'en sert.
#[test]
fn calcul_profil_js_est_charge_entre_arbre_js_et_reglage_js() {
    let page = lire("interface/index.html");
    let position = |nom: &str| page.find(&format!("<script src=\"{nom}\" defer>"));
    let (arbre, calcul, reglage) = (
        position("arbre.js"),
        position("calcul-profil.js"),
        position("reglage.js"),
    );
    assert!(arbre.is_some() && calcul.is_some() && reglage.is_some());
    assert!(arbre < calcul && calcul < reglage, "ordre des scripts");
    assert!(lire("interface/reglage.js").contains("window.CalculProfil"));
}

#[test]
fn la_vue_d_une_resolution_appelle_le_calcul_du_profil() {
    let script = lire("interface/calcul-profil.js");
    for attendu in [
        "invoke(\"profiler_calculer\"",
        "invoke(\"profiler_commandes\"",
        // Tauri attend les paramètres en camelCase.
        "mesureCondition:",
    ] {
        assert!(script.contains(attendu), "{attendu}");
    }
}

#[test]
fn le_texte_de_l_ancien_bouton_prevu_a_disparu() {
    for fichier in [
        "interface/index.html",
        "interface/reglage.js",
        "interface/calcul-profil.js",
        "src/textes.rs",
    ] {
        let texte = lire(fichier);
        assert!(!texte.contains("parc.refaire"), "{fichier}");
        assert!(!texte.contains("data-refaire"), "{fichier}");
    }
}

/// Chaque clé `profil.…` lue par la vue d'une résolution existe en français
/// et en anglais ; les clés complétées à l'écran (`"profil.recette.nom." +
/// recette`) existent pour chaque recette.
#[test]
fn toutes_les_cles_profil_de_la_page_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    let script = lire("interface/reglage.js")
        + &lire("interface/calcul-profil.js")
        + &lire("interface/index.html");
    let mut demandees: Vec<String> = script
        .match_indices("\"profil.")
        .filter_map(|(i, _)| script[i + 1..].split('"').next())
        .map(String::from)
        .collect();
    assert!(demandees.len() > 10, "clés lues : {demandees:?}");
    assert!(demandees.iter().any(|c| c == "profil.recette.nom."));
    let prefixes: Vec<String> = demandees
        .iter()
        .filter(|c| c.ends_with('.'))
        .cloned()
        .collect();
    demandees.retain(|c| !c.ends_with('.'));
    for prefixe in prefixes {
        for recette in ["rapide", "standard", "approfondi"] {
            demandees.push(format!("{prefixe}{recette}"));
        }
    }
    let absentes: Vec<_> = demandees
        .iter()
        .filter(|c| {
            [Langue::Francais, Langue::Anglais]
                .iter()
                .any(|l| texte(*l, c) == c.as_str())
        })
        .collect();
    assert!(absentes.is_empty(), "clés absentes : {absentes:?}");
}

/// Une erreur du calcul arrive en objet `{cle, cause}` : la page dit le texte
/// de la clé, puis la cause d'ArgyllCMS, jamais « [object Object] ». Un objet
/// sans clé ou une exception de la page donne une erreur inattendue ; la
/// cause est insérée telle quelle (`$&` n'est pas interprété).
#[test]
fn une_erreur_du_calcul_dit_sa_cle_puis_sa_cause() {
    let reglage = lire("interface/reglage.js");
    assert!(
        !reglage.contains("t(String(cle))"),
        "objet affiché tel quel"
    );
    assert!(reglage.contains("annoncer(texteErreur(erreur));"));
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("function texteErreur(erreur)"));
    assert!(script.contains("t(\"profil.erreur.cause\").replace(\"{cause}\", () => erreur.cause)"));
    assert!(script.contains("t(\"parc.erreur.inattendue\")"));
    assert!(app::textes::cles().any(|c| c == "parc.erreur.inattendue"));
}

/// « Calculer le profil » inactif dit pourquoi juste à côté ; pendant le
/// calcul, un seul message « en cours », celui du calcul. Une erreur des
/// commandes (ArgyllCMS introuvable) se lit sous le bouton, sans rien ouvrir.
#[test]
fn le_bouton_calculer_inactif_dit_pourquoi() {
    let script = lire("interface/calcul-profil.js");
    assert!(
        script.contains("corps.append(actions, raisonCalcul);"),
        "raison loin du bouton"
    );
    assert!(script.contains("if (choix.mesure === null) calculer.dataset.inactif = \"\";"));
    assert!(script.contains("calculEnCours ? texteEnCours()"));
    assert!(script.contains(
        ": erreurCalcul || erreurCommandes || (choix.mesure === null ? t(\"profil.calculer.raison\") : \"\")"
    ));
    assert!(script.contains("erreurCommandes = texteErreur(erreur);"));
    assert!(
        !script.contains("texte = texteErreur(erreur)"),
        "erreur dans le <pre> replié"
    );
    assert!(
        script.contains("agir(calculerProfil)"),
        "mécanisme agir() existant"
    );
    let reglage = lire("interface/reglage.js");
    assert!(reglage.contains("occupe.hidden = !enCours || calcul.raisonVisible();"));
}

/// Un calcul fini alors que l'opérateur a changé de résolution n'annonce rien
/// sur l'autre vue ; un rechargement qui échoue après un calcul réussi ne
/// dit pas que le calcul a échoué.
#[test]
fn un_calcul_fini_ne_s_annonce_que_sur_sa_resolution() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("const ici = () => parc.reglage()?.condition.id === condition;"));
    assert!(script.contains("if (ici()) erreurCalcul = texteErreur(erreur);"));
    assert!(script.contains("if (ici()) parc.annoncer("));
    let apres = script
        .split("await parc.recharger();")
        .nth(1)
        .unwrap_or_default();
    assert!(
        script.contains("try {\n      await parc.recharger();")
            && apres.trim_start().starts_with("} catch"),
        "rechargement non protégé"
    );
}

/// La commande et le journal d'un profil calculé restent consultables à
/// droite, repliés sous « Avancé », en lecture seule.
#[test]
fn la_commande_et_le_journal_restent_consultables() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("el(\"pre\", \"\", p.commandes.join(\"\\n\"))"));
    assert!(script.contains("el(\"pre\", \"\", p.journal)"));
    assert!(script.contains("el(\"details\", \"replie\")"));
    assert!(script.contains("el(\"summary\", \"\", t(\"profil.avance.provenance\"))"));
    use app::textes::{texte, Langue};
    assert_eq!(
        texte(Langue::Francais, "profil.avance.provenance"),
        "Avancé\u{202f}: commande et journal"
    );
    assert_eq!(
        texte(Langue::Anglais, "profil.avance.provenance"),
        "Advanced: command and log"
    );
    assert_eq!(
        texte(Langue::Anglais, "profil.precision"),
        "Calculation accuracy"
    );
}

/// Pendant le calcul, le message « en cours » dit l'étape, le temps écoulé
/// (redessiné chaque seconde) et ce qu'ArgyllCMS a écrit en dernier, reçus
/// par l'événement `profil-avancement`. Une seule minuterie, arrêtée à la
/// fin du calcul, réussi ou non.
#[test]
fn pendant_un_calcul_l_avancement_se_voit() {
    use app::textes::{texte, Langue};
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("listen(\"profil-avancement\""));
    assert!(script.contains("calculEnCours ? texteEnCours()"));
    assert!(script.contains("t(\"profil.en_cours\")"));
    assert!(script.contains("setInterval(dessinerRaison, 1000)"));
    assert_eq!(script.matches("setInterval(").count(), 1);
    assert!(script.contains("clearInterval(minuterie)"));
    assert!(
        script.contains("finally {"),
        "minuterie arrêtée même en cas d'erreur"
    );
    for cle in [
        "profil.etape.preparation",
        "profil.etape.profil",
        "profil.etape.controle",
        "profil.ecoule",
        "profil.argyll.ligne",
        "profil.argyll.pourcentage",
    ] {
        assert!(
            script.contains(&format!("\"{cle}\"")),
            "{cle} absente de la page"
        );
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} absente du catalogue");
        }
    }
    // Le pourcentage est celui d'une partie du calcul, pas du tout.
    assert!(texte(Langue::Francais, "profil.argyll.pourcentage").contains("partie"));
    assert!(texte(Langue::Anglais, "profil.argyll.pourcentage").contains("part"));
}

/// Barre d'avancement par étapes, dans l'ordre des étapes du moteur. Elle
/// est accessible (progressbar) : sa valeur est le nombre d'étapes faites,
/// une valeur réelle ; seule la case en cours se remplit, et seulement d'un
/// pourcentage écrit par ArgyllCMS. La durée restante n'est qu'une
/// estimation, dite « estimé », et rien sans historique.
#[test]
fn pendant_un_calcul_une_barre_avance_par_etapes() {
    use app::textes::{texte, Langue};
    use profilage::EtapeCalcul;
    let script = lire("interface/calcul-profil.js");
    let etapes: Vec<String> = EtapeCalcul::TOUTES
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    assert!(
        script.contains(&format!("const ETAPES = [{}];", etapes.join(", "))),
        "ordre des étapes du moteur"
    );
    for attendu in [
        "setAttribute(\"role\", \"progressbar\")",
        "setAttribute(\"aria-valuemin\", \"0\")",
        "setAttribute(\"aria-valuemax\", String(ETAPES.length))",
        "setAttribute(\"aria-valuenow\", String(faites))",
        "setAttribute(\"aria-valuetext\"",
        "\"progression__etape\"",
        "\"progression__remplissage\"",
        "avancement.pourcentage !== null",
    ] {
        assert!(script.contains(attendu), "{attendu}");
    }
    // Le remplissage ne vient que du pourcentage écrit par ArgyllCMS.
    assert_eq!(script.matches(".style.width").count(), 1);
    assert!(script.contains(".style.width = `${avancement.pourcentage}%`"));
    // Une étape ne recule jamais.
    assert!(script.contains("ETAPES.indexOf(a.etape) > ETAPES.indexOf(avancement.etape)"));
    // Estimation : seulement si la page en a reçu une.
    assert!(script.contains("estimationCalcul = estimation;"));
    assert!(script.contains("if (estimationCalcul !== null)"));
    for cle in [
        "profil.etape.table",
        "profil.etape.gamut",
        "profil.etape.inverses",
        "profil.etape.limite",
        "profil.progression",
        "profil.estimation.restante",
        "profil.estimation.depassee",
    ] {
        assert!(
            script.contains(&format!("\"{cle}\"")),
            "{cle} absente de la page"
        );
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} absente du catalogue");
        }
    }
    for cle in ["profil.estimation.restante", "profil.estimation.depassee"] {
        assert!(texte(Langue::Francais, cle).contains("estimé"), "{cle}");
        assert!(texte(Langue::Anglais, cle).contains("estimated"), "{cle}");
    }
    // Dessin : jetons du système graphique, angles droits (tests plus haut).
    let css = composants();
    assert!(!regle(&css, ".progression").is_empty());
    assert!(!regle(&css, ".progression__etape").is_empty());
    assert!(!regle(&css, ".progression__remplissage").is_empty());
}

/// Pendant un calcul, le message général ne se cache que si la raison du
/// calcul est vraiment à l'écran (section vidée après un changement de
/// résolution : le message général reste).
#[test]
fn pendant_un_calcul_une_raison_reste_toujours_visible() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("raisonVisible: () => calculEnCours && raisonCalcul !== null,"));
}

/// Le bouton d'import de la section ne casse rien si la feuille n'a pas le
/// sien ; « Recette de calcul » ne se confond pas avec la qualité vérifiée.
#[test]
fn import_protege_et_recette_de_calcul() {
    use app::textes::{texte, Langue};
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("if (bouton) bouton.click();"));
    assert_eq!(
        texte(Langue::Francais, "profil.recette"),
        "Recette de calcul"
    );
    assert_eq!(
        texte(Langue::Anglais, "profil.recette"),
        "Calculation recipe"
    );
}

/// Chaque texte demandé par l'arbre et la vue d'une résolution (`t("…")`
/// dans parc.js, arbre.js et reglage.js) existe au catalogue.
#[test]
fn le_parc_ne_demande_que_des_cles_du_catalogue() {
    let script = [
        "interface/parc.js",
        "interface/arbre.js",
        "interface/calcul-profil.js",
        "interface/reglage.js",
    ]
    .map(lire)
    .join("\n");
    let demandees: Vec<&str> = script
        .match_indices("t(\"")
        .filter(|(i, _)| {
            !script[..*i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
        })
        .filter_map(|(i, _)| script[i + 3..].split('"').next())
        .collect();
    assert!(demandees.len() > 10, "clés lues : {demandees:?}");
    // `t("parc.etat." + p.etat)` : un début de clé, complété à l'écran.
    let inconnues: Vec<_> = demandees
        .iter()
        .filter(|c| {
            !app::textes::cles().any(|k| {
                if c.ends_with('.') {
                    k.starts_with(**c)
                } else {
                    k == **c
                }
            })
        })
        .collect();
    assert!(inconnues.is_empty(), "clés absentes : {inconnues:?}");
}

// ---- Relecture du ticket #68 ----

/// Pendant une action sur le profil (ranger, exporter, installer), les trois
/// boutons sont inactifs et une raison « en cours » est dite juste à côté.
#[test]
fn pendant_une_action_sur_le_profil_les_boutons_disent_pourquoi() {
    let page = lire("interface/index.html");
    let vue = element(&page, "data-vue-reglage");
    let raison = vue
        .split('<')
        .find(|b| b.contains("data-reglage-occupe"))
        .and_then(|b| b.split('>').next())
        .expect("raison « en cours » absente");
    assert!(raison.starts_with("p class=\"why\""), "{raison}");
    assert!(raison.contains("data-t=\"parc.en_cours\""), "{raison}");
    assert!(raison.split_whitespace().any(|a| a == "hidden"), "{raison}");

    let script = lire("interface/reglage.js");
    assert!(
        script.contains(
            "const BOUTONS = \"[data-installer], [data-exporter-profil], [data-ranger-profil], [data-calcul-ouvrir], [data-calcul-profil] button, [data-calcul-profil] input\";"
        ),
        "les boutons de la vue"
    );
    assert!(script.contains("occupe.hidden = !enCours || calcul.raisonVisible();"));
    assert!(app::textes::cles().any(|c| c == "parc.en_cours"));
}

/// La pastille dit « en mesure » pendant une mesure seulement, pas pendant
/// l'étalonnage.
#[test]
fn la_pastille_n_est_en_mesure_que_pendant_une_mesure() {
    let script = lire("interface/app.js");
    assert!(
        script.contains("if (operationEnCours === \"mesure\") return \"en-mesure\";"),
        "pastille"
    );
    assert!(!script.contains("if (operationEnCours) return \"en-mesure\";"));
}

/// Un état de vérification inconnu de la page ne casse pas la vue : elle le
/// montre comme « Profil non vérifié ».
#[test]
fn un_etat_de_verification_inconnu_est_montre_non_verifie() {
    let script = lire("interface/reglage.js");
    assert!(
        script.contains("VERDICTS[actif.etat] || VERDICTS.non_verifie"),
        "repli absent"
    );
}

// ---- Relecture finale du ticket #40 ----

/// La commande arrive même sans ArgyllCMS ; la raison qui empêchera le
/// calcul est dite sous le bouton avant le clic.
#[test]
fn sans_argyll_la_commande_se_montre_et_la_raison_aussi() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains("commandes = reponse.commandes.join(\"\\n\");"));
    assert!(
        script.contains("erreurCommandes = reponse.moteur ? texteErreur(reponse.moteur) : \"\";")
    );
}

/// La version refusée prend place dans le texte de la clé, pas sous
/// « Message d'ArgyllCMS ».
#[test]
fn la_version_refusee_se_lit_dans_la_phrase() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains(".replace(\"{version}\", () => erreur.version)"));
}

/// Une échelle RVB supposée est dite à droite, dans la provenance.
#[test]
fn l_echelle_rvb_supposee_est_dite() {
    let script = lire("interface/calcul-profil.js");
    assert!(script.contains(
        "if (p.rvb_supposes) lignes.push(cleValeur(\"profil.rvb\", t(\"profil.rvb.supposee\")));"
    ));
}

/// Le commentaire de la page nomme le script qui construit la section.
#[test]
fn la_section_du_calcul_est_construite_par_calcul_profil_js() {
    let page = lire("interface/index.html");
    let section = page.find("data-calcul-profil hidden").expect("section");
    let commentaire = &page[page[..section].rfind("<!--").unwrap_or(0)..section];
    assert!(commentaire.contains("calcul-profil.js"), "{commentaire}");
}
