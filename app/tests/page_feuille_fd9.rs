//! La page, lue comme un fichier : l'écran « Mesurer une feuille au FD-9 »
//! dans la vue d'une résolution (ticket #34, tâche 9). Règles du
//! mainteneur : l'interface ne bouge jamais (mises à jour sur place, places
//! gardées), jetons du système graphique, clavier.

use std::path::Path;

/// Fichier de la page, fins de ligne ramenées à `\n`.
fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif))
        .unwrap()
        .replace("\r\n", "\n")
}

/// Corps des règles CSS dont un sélecteur est exactement `selecteur`, sans
/// espaces.
fn regle(css: &str, selecteur: &str) -> String {
    css.split('}')
        .filter_map(|r| r.split_once('{'))
        .filter(|(s, _)| {
            let s = s.rsplit("*/").next().unwrap_or(s);
            s.split(',').any(|x| x.trim() == selecteur)
        })
        .map(|(_, c)| c.replace(' ', ""))
        .collect::<Vec<_>>()
        .join(";")
}

/// Morceau de `index.html` du bloc de la feuille.
fn bloc() -> String {
    let page = lire("interface/index.html");
    let debut = page
        .find("<div class=\"section\" data-feuille-fd9>")
        .unwrap();
    let fin = page[debut..]
        .find("<div class=\"section\">\n        <div class=\"section__head\"><span class=\"label label--ink\" data-t=\"parc.historique\">")
        .unwrap();
    page[debut..debut + fin].to_string()
}

/// Les deux scripts sont chargés après app.js et avant reglage.js, qui s'en
/// sert ; chacun, et reglage.js, reste sous 400 lignes.
#[test]
fn les_scripts_de_la_feuille_sont_charges_avant_reglage_js_et_courts() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("app.js") < position("feuille-vue.js"));
    assert!(position("feuille-vue.js") < position("feuille-fd9.js"));
    assert!(position("feuille-fd9.js") < position("reglage.js"));
    for nom in ["feuille-vue.js", "feuille-fd9.js", "reglage.js"] {
        let lignes = lire(&format!("interface/{nom}")).lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
    }
    let reglage = lire("interface/reglage.js");
    assert!(reglage.contains("feuilleFd9.dessiner(d);"));
}

/// Aucune couleur en dur : les points et le repère viennent des jetons, la
/// couleur de consigne d'une plage passe par `--c`, contrôlée.
#[test]
fn aucune_couleur_en_dur_dans_la_feuille() {
    for nom in ["feuille-vue.js", "feuille-fd9.js"] {
        let script = lire(&format!("interface/{nom}"));
        let hexa = script.match_indices('#').any(|(i, _)| {
            script[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count()
                >= 3
        });
        assert!(!hexa, "{nom} : couleur hexadécimale en dur");
        for interdit in [
            "rgb(",
            "rgba(",
            "hsl(",
            "style=",
            "innerHTML",
            "data:",
            "blob:",
        ] {
            assert!(!script.contains(interdit), "{nom} : {interdit}");
        }
    }
    let vue = lire("interface/feuille-vue.js");
    for attendu in [
        "jeton(\"--point-vise\")",
        "jeton(\"--point-vise-bord\")",
        "jeton(\"--encre\")",
        "jeton(\"--fond\")",
        "/^#[0-9a-fA-F]{6}$/",
        "consigne.style.setProperty(\"--c\", valeur)",
    ] {
        assert!(vue.contains(attendu), "{attendu}");
    }
    let jetons = lire("../design-system/tokens.css");
    for jeton in ["--point-vise:", "--point-vise-bord:"] {
        assert_eq!(
            jetons.matches(jeton).count(),
            2,
            "{jeton} dans les deux thèmes"
        );
    }
}

/// Rien ne bouge : les deux scripts mettent à jour les blocs de la page sur
/// place, sans jamais les reconstruire ; chaque bloc qui montre une valeur
/// a sa hauteur gardée ; « Annuler » garde sa place, invisible au repos.
#[test]
fn la_feuille_se_met_a_jour_sur_place_dans_des_places_gardees() {
    for nom in ["feuille-vue.js", "feuille-fd9.js"] {
        let script = lire(&format!("interface/{nom}"));
        for interdit in [
            "replaceChildren",
            "createElement(\"div\"",
            "appendChild",
            ".append(",
        ] {
            assert!(!script.contains(interdit), "{nom} : {interdit}");
        }
    }
    let css = lire("../design-system/components.css");
    // Hauteurs fixes, jamais minimales : un texte long (bourrage, raison sur
    // deux lignes) est coupé proprement ou défile dans sa place, avec retrait.
    for selecteur in [
        ".feuille-fd9__raison",
        ".feuille-fd9__etat",
        ".feuille-fd9__message",
        ".feuille-fd9__echec",
    ] {
        let corps = regle(&css, selecteur);
        assert!(!corps.contains("min-height"), "{selecteur} : {corps}");
        assert!(
            corps.contains(";height:")
                || corps.starts_with("height:")
                || corps.contains("{height:"),
            "{selecteur} : hauteur fixe : {corps}"
        );
    }
    for selecteur in [".feuille-fd9__raison", ".feuille-fd9__message"] {
        let corps = regle(&css, selecteur);
        for attendu in ["-webkit-line-clamp:2", "overflow:hidden"] {
            assert!(corps.contains(attendu), "{selecteur} : {attendu} : {corps}");
        }
    }
    let echec = regle(&css, ".feuille-fd9__echec");
    for attendu in ["overflow-y:auto", "scrollbar-gutter:stable"] {
        assert!(echec.contains(attendu), "{attendu} : {echec}");
    }
    // Défilement dans une zone qui défile : ascenseur à gauche, avec retrait.
    assert!(bloc().contains("class=\"feuille-fd9__echec defile-gauche\" data-feuille-echec hidden"));
    // Le texte coupé se lit en entier au survol.
    let fd9 = lire("interface/feuille-fd9.js");
    for attendu in [
        "raison.title = raison.textContent;",
        "message.title = message.textContent;",
    ] {
        assert!(fd9.contains(attendu), "{attendu}");
    }
    for (selecteur, attendu) in [
        (".garde-place[data-vide]", "visibility:hidden"),
        (".feuille-vue__toile", "height:min(65vh"),
        (".feuille-vue__survol", "height:calc("),
        (".feuille-vue__legende", "overflow:hidden"),
        (".feuille-vue__survol-texte", "text-overflow:ellipsis"),
    ] {
        let corps = regle(&css, selecteur);
        assert!(corps.contains(attendu), "{selecteur} : {attendu} : {corps}");
    }
    let bloc = bloc();
    assert!(bloc.contains(
        "<button class=\"btn garde-place\" data-feuille-annuler data-t=\"feuille_fd9.annuler\" data-vide disabled>"
    ));
    let fd9 = lire("interface/feuille-fd9.js");
    assert!(fd9.contains("annuler.toggleAttribute(\"data-vide\", !enCours());"));
    // Le zoom ne change que ce que la toile dessine : pas d'ascenseur.
    let toile = regle(&css, ".feuille-vue__toile");
    assert!(!toile.contains("overflow"), "{toile}");
}

/// Le bouton est actif seulement quand le module Rust le dit, et sa raison
/// est écrite juste dessous ; « Annuler » ne vaut que pendant l'attente et
/// la lecture.
#[test]
fn le_bouton_et_annuler_suivent_l_etat_de_la_feuille() {
    let fd9 = lire("interface/feuille-fd9.js");
    for attendu in [
        "mesurer.disabled = enCours() || !etat || !etat.disponible || condition === null;",
        "etat.raisons.map((c) => t(c)).join(\" \")",
        // L'attente et la lecture, calage prêt compris (montré « Lecture »).
        "const ANNULABLES = [\"connexion\", \"glisser_feuille\", \"feuille_engagee\", \"lecture\", \"calage_pret\"];",
        "mesure_lancee: \"mesure\"",
        "annuler.disabled = !enCours() || !ANNULABLES.includes(feuille.etape);",
        "invoke(\"annuler_feuille_fd9\")",
        "invoke(\"mesurer_feuille_fd9\", { condition: feuille.condition })",
        "invoke(\"feuille_fd9_etat\")",
    ] {
        assert!(fd9.contains(attendu), "{attendu}");
    }
    let bloc = bloc();
    let mesurer = bloc.find("data-feuille-mesurer").unwrap();
    let raison = bloc.find("data-feuille-raison").unwrap();
    assert!(mesurer < raison, "la raison est juste sous le bouton");
}

/// L'état en clair : glissez la feuille, lecture, « Mesure : n / 2 520 »
/// avec les nombres du pont, rangée, ou l'erreur avec sa cause et son
/// action (texte général pour une clé inconnue), détail replié.
#[test]
fn l_etat_de_la_feuille_est_dit_en_clair() {
    let fd9 = lire("interface/feuille-fd9.js");
    for attendu in [
        "t(`feuille_fd9.etape.${feuille.etape}`)",
        ".replace(\"{n}\", () => nombre(feuille.mesurees))",
        ".replace(\"{total}\", () => nombre(feuille.plages))",
        "const cle = textes[`${echec.cle}.cause`] ? echec.cle : \"feuille_fd9.autre\";",
        "t(`${cle}.action`)",
        "window.__TAURI__.event.listen(\"feuille_fd9\"",
    ] {
        assert!(fd9.contains(attendu), "{attendu}");
    }
    let bloc = bloc();
    for attendu in [
        "role=\"progressbar\"",
        "data-feuille-cause",
        "data-feuille-action",
        "data-t=\"feuille_fd9.detail\"",
        "aria-live=\"polite\"",
    ] {
        assert!(bloc.contains(attendu), "{attendu}");
    }
    assert_eq!(bloc.matches("class=\"etape\" role=\"listitem\"").count(), 4);
}

/// Deux onglets comme le logiciel du fabricant : « Mire » et « Points
/// mesurés », reliés au panneau, au clavier ; les points sont montrés dès
/// le calage prêt, avec l'image demandée en binaire par son numéro.
#[test]
fn deux_onglets_mire_et_points_mesures() {
    let bloc = bloc();
    for attendu in [
        "role=\"tablist\"",
        "id=\"feuille-onglet-mire\" aria-controls=\"feuille-fd9-panneau\" aria-selected=\"true\"",
        "id=\"feuille-onglet-points\" aria-controls=\"feuille-fd9-panneau\" aria-selected=\"false\" tabindex=\"-1\"",
        "role=\"tabpanel\" id=\"feuille-fd9-panneau\"",
        "<canvas class=\"feuille-vue__toile\" tabindex=\"0\" role=\"img\"",
    ] {
        assert!(bloc.contains(attendu), "{attendu}");
    }
    let vue = lire("interface/feuille-vue.js");
    for attendu in [
        "{ ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: onglets.length - 1 }",
        "b.tabIndex = actif ? 0 : -1;",
        "{ ArrowRight: [0, 1], ArrowLeft: [0, -1], ArrowDown: [1, 0], ArrowUp: [-1, 0] }",
        "if (e.key === \"+\" || e.key === \"=\") return zoomer(zoom + 1);",
        // N° de plage comme le logiciel du fabricant : page, colonne en
        // lettres, rangée (« 1A1 », « 1AP60 »).
        "`${p.zone + 1}${lettres(p.colonne)}${p.rangee + 1}`",
        "new ResizeObserver(() => dessiner()).observe(toile);",
        "attributeFilter: [\"data-theme\"]",
        // Un glisser interrompu (fenêtre perdue, geste annulé) s'arrête.
        "toile.addEventListener(\"pointercancel\", lacher);",
    ] {
        assert!(vue.contains(attendu), "{attendu}");
    }
    // Les mesures de la feuille viennent de la définition, jamais en dur.
    for en_dur in ["27940", "41660", "450", "60 *"] {
        assert!(!vue.contains(en_dur), "{en_dur} en dur dans feuille-vue.js");
    }
    let fd9 = lire("interface/feuille-fd9.js");
    let ecoute = fd9
        .split("window.__TAURI__.event.listen(\"feuille_fd9\"")
        .nth(1)
        .unwrap();
    let ecoute = &ecoute[..ecoute.find("\n  });").unwrap()];
    for attendu in [
        "if (e.etape === \"calage_pret\") {",
        "vueFeuille.montrer(\"points\");",
        "montrerCalage(e.numero, e.calage, false);",
    ] {
        assert!(ecoute.contains(attendu), "{attendu} :\n{ecoute}");
    }
    assert!(fd9.contains("invoke(\"feuille_fd9_image\", { numero })"));
    assert!(fd9.contains("invoke(\"feuille_fd9_dernier_calage\", { condition: id })"));
}

/// Le système graphique décrit les onglets et la feuille au FD-9 avant
/// l'application.
#[test]
fn le_systeme_graphique_a_les_onglets_et_la_feuille() {
    let kit = lire("../design-system/ui-kit.html");
    for attendu in [
        "<h2>Onglets</h2>",
        "<h2>Feuille au FD-9</h2>",
        "class=\"onglets\"",
        "feuille-vue__toile",
    ] {
        assert!(kit.contains(attendu), "{attendu}");
    }
    // Même ordre que la page : Mesurer, Mire et points mesurés, Annuler.
    let exemple = &kit[kit.find("<h2>Feuille au FD-9</h2>").unwrap()..];
    let position = |texte: &str| exemple.find(texte).unwrap();
    assert!(
        position(">Mesurer une feuille au FD-9</button>")
            < position(">Mire et points mesurés</button>")
    );
    assert!(position(">Mire et points mesurés</button>") < position(">Annuler</button>"));
    let page = bloc();
    assert!(page.find("data-feuille-voir").unwrap() < page.find("data-feuille-annuler").unwrap());
    let css = lire("../design-system/components.css");
    let choisi = regle(&css, ".onglets [role=\"tab\"][aria-selected=\"true\"]");
    assert!(choisi.contains("background:var(--carte)"), "{choisi}");
}
