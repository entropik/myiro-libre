//! La page, lue comme un fichier : la séance de mesure d'un tirage FD-9
//! dans la vue d'une résolution (ticket #35). Règles du mainteneur : une
//! action met à jour sur place, sans redessiner la section ni faire bouger
//! la page ; chaque bloc qui montre une valeur a sa place gardée ; tous les
//! textes viennent du catalogue ; aucun calcul de couleur dans la page.

use std::path::Path;

use app::textes::{texte, Langue};

/// Fichier de l'application, fins de ligne ramenées à `\n`.
fn lire(relatif: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relatif))
        .unwrap()
        .replace("\r\n", "\n")
}

const SCRIPTS: [&str; 2] = ["seance-plages.js", "seance.js"];

fn script(nom: &str) -> String {
    lire(&format!("interface/{nom}"))
}

/// Le bloc de la séance dans `index.html`.
fn bloc() -> String {
    let page = lire("interface/index.html");
    let debut = page
        .find("<div id=\"seance-fd9\" data-seance hidden>")
        .unwrap();
    let fin = page[debut..]
        .find("<div class=\"section\" data-feuille-fd9>")
        .unwrap();
    page[debut..debut + fin].to_string()
}

fn present(cle: &str) {
    for langue in [Langue::Francais, Langue::Anglais] {
        let lu = texte(langue, cle);
        assert_ne!(lu, cle, "{cle} manque en {}", langue.code());
        assert!(!lu.trim().is_empty(), "{cle} vide en {}", langue.code());
    }
}

/// Les deux scripts sont chargés après app.js et avant reglage.js, qui s'en
/// sert ; seance-plages.js avant seance.js. Chacun reste sous 400 lignes.
#[test]
fn les_scripts_de_la_seance_sont_charges_avant_reglage_js_et_courts() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    assert!(position("app.js") < position("arbre.js"));
    assert!(position("arbre.js") < position("seance-plages.js"));
    assert!(position("seance-plages.js") < position("seance.js"));
    assert!(position("seance.js") < position("reglage.js"));
    for nom in SCRIPTS.iter().chain(&["reglage.js"]) {
        let lignes = script(nom).lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
    }
    let reglage = script("reglage.js");
    for attendu in ["seance.dessiner(d);", "seance.brancher({"] {
        assert!(reglage.contains(attendu), "{attendu}");
    }
}

/// Rien ne bouge : les scripts écrivent dans les blocs de la page sans
/// jamais les reconstruire ; une ligne nouvelle vient d'un modèle et reste
/// ensuite. Ni couleur en dur, ni style en ligne, ni fenêtre du navigateur.
#[test]
fn la_seance_s_ecrit_sur_place_sans_couleur_en_dur() {
    for nom in SCRIPTS {
        let s = script(nom);
        let hexa = s.match_indices('#').any(|(i, _)| {
            s[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count()
                >= 3
        });
        assert!(!hexa, "{nom} : couleur hexadécimale en dur");
        for interdit in [
            "replaceChildren",
            "innerHTML",
            "createElement(\"div\"",
            "rgb(",
            "rgba(",
            "hsl(",
            "style=",
            "data:",
            "blob:",
            "confirm(",
            "alert(",
            "prompt(",
            "aria-pressed",
        ] {
            assert!(!s.contains(interdit), "{nom} : {interdit}");
        }
        assert!(
            s.contains("modele.content.firstElementChild.cloneNode(true)"),
            "{nom}"
        );
    }
    // La seule couleur posée par la page : la couleur demandée d'une plage,
    // contrôlée en #rrggbb et passée par --c.
    let plages = script("seance-plages.js");
    for attendu in [
        "/^#[0-9a-fA-F]{6}$/",
        "consigne.style.setProperty(\"--c\", c)",
        "jeton(\"--fond\")",
        "jeton(\"--hors\")",
        "jeton(\"--attenue\")",
        "jeton(\"--encre\")",
    ] {
        assert!(plages.contains(attendu), "{attendu}");
    }
}

/// Chaque bloc qui montre une valeur a sa place gardée, avec les classes
/// déjà fixées par le système graphique : raison et messages sur deux lignes,
/// listes et aperçu à hauteur fixe qui défilent avec l'ascenseur à gauche,
/// toile de taille fixe, ligne de survol gardée. La séance en cours se dit
/// sur une ligne gardée de la vue, jamais insérée.
#[test]
fn chaque_valeur_a_sa_place_gardee() {
    let b = bloc();
    for attendu in [
        "<p class=\"why feuille-fd9__raison\" role=\"status\" data-seance-etat>",
        "<p class=\"why feuille-fd9__raison\" role=\"status\" data-seance-raison>",
        "<p class=\"why feuille-fd9__message\" role=\"status\" aria-live=\"polite\" data-seance-message>",
        "<div class=\"feuille-fd9__echec defile-gauche\" role=\"list\" data-t-aria=\"seance.sources\" data-seance-sources>",
        "<template data-seance-modele>",
        "<canvas class=\"feuille-vue__toile\" tabindex=\"0\" role=\"img\"",
        "<div class=\"feuille-vue__survol\">",
        "<p class=\"feuille-vue__legende\" data-t=\"seance.plages.legende\">",
        "<template data-seance-complement-modele>",
    ] {
        assert!(b.contains(attendu), "{attendu}");
    }
    assert_eq!(b.matches("<div class=\"feuille-fd9__etat\"").count(), 3);
    let page = lire("interface/index.html");
    assert!(page.contains(
        "<p class=\"why chiffres-image__cellule\" role=\"status\" data-seance-reprendre></p>"
    ));
    // Une ligne cachée l'est par un enveloppant sans classe.
    for modele in ["data-seance-modele>", "data-seance-complement-modele>"] {
        let apres = &b[b.find(modele).unwrap() + modele.len()..];
        assert!(
            apres.starts_with("<div role=\"listitem\"><div class=\"cle-valeur\">"),
            "{modele}"
        );
    }
    // Un texte coupé se lit en entier au survol.
    let seance = script("seance.js");
    for attendu in [
        "raison.title = raison.textContent;",
        "libelle.title = texte;",
    ] {
        assert!(seance.contains(attendu), "{attendu}");
    }
}

/// « Mesurer le tirage » ouvre la section : c'est le seul bouton de la
/// séance qui porte aria-expanded, car il ouvre vraiment quelque chose.
#[test]
fn la_section_s_ouvre_depuis_la_vue_d_une_resolution() {
    let page = lire("interface/index.html");
    assert!(page.contains(
        "<button class=\"btn\" data-seance-ouvrir data-t=\"seance.ouvrir\" aria-expanded=\"false\" aria-controls=\"seance-fd9\"></button>"
    ));
    let seance = script("seance.js");
    assert_eq!(seance.matches("aria-expanded").count(), 1, "{seance}");
    assert!(
        seance.contains("ouvrirBouton.setAttribute(\"aria-expanded\", String(!racine.hidden));")
    );
}

/// La mesure au FD-9 depuis la séance attend son câblage dans
/// feuille-fd9.js : le bouton reste inactif, sa raison est dite, et jamais
/// l'ancienne mesure d'une feuille de résolution n'est appelée à sa place.
#[test]
fn la_mesure_au_fd9_de_la_seance_reste_inactive_avec_sa_raison() {
    let b = bloc();
    assert!(b.contains(
        "<button class=\"btn\" data-seance-mesurer data-t=\"seance.mesurer\" disabled></button>"
    ));
    let seance = script("seance.js");
    assert!(seance.contains("mesurer.disabled = true;"));
    assert!(seance.contains("raisons.push(\"seance.raison.mesure_a_venir\");"));
    for nom in SCRIPTS {
        let s = script(nom);
        assert!(!s.contains("mesurer_feuille_fd9"), "{nom}");
    }
}

/// Chaque commande appelée par la page est enregistrée côté Rust.
#[test]
fn chaque_commande_de_la_seance_est_enregistree() {
    let lib = lire("src/lib.rs");
    let mut vues = 0;
    for nom in SCRIPTS {
        let s = script(nom);
        for (i, _) in s.match_indices("invoke(\"") {
            let commande = s[i + 8..].split('"').next().unwrap();
            vues += 1;
            assert!(
                lib.contains(&format!("::{commande},")) || lib.contains(&format!("::{commande}\n")),
                "{commande} non enregistrée"
            );
        }
    }
    assert!(vues >= 7, "commandes vues : {vues}");
}

/// Tous les textes viennent du catalogue, dans les deux langues : clés des
/// scripts, du bloc, et celles que les scripts complètent.
#[test]
fn toutes_les_cles_de_la_seance_existent_en_deux_langues() {
    let mut cles: Vec<String> = Vec::new();
    for nom in SCRIPTS {
        let s = script(nom);
        for (i, _) in s.match_indices("t(\"") {
            if s[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            {
                continue;
            }
            cles.push(s[i + 3..].split('"').next().unwrap().to_string());
        }
    }
    let b = bloc();
    for attribut in ["data-t=\"", "data-t-aria=\""] {
        for (i, _) in b.match_indices(attribut) {
            cles.push(
                b[i + attribut.len()..]
                    .split('"')
                    .next()
                    .unwrap()
                    .to_string(),
            );
        }
    }
    cles.push("seance.ouvrir".into());
    for complete in [
        "seance.etat.en_cours",
        "seance.etat.terminee",
        "seance.etat.incomplete",
        "seance.plages.etat.associee",
        "seance.plages.etat.manquante",
        "seance.plages.etat.douteuse",
        "seance.source.acquisition",
        "seance.source.import",
    ] {
        cles.push(complete.into());
    }
    assert!(cles.len() > 50, "clés lues : {}", cles.len());
    for cle in &cles {
        present(cle);
    }
}

/// L'aperçu écran n'est jamais présenté comme une preuve colorimétrique.
#[test]
fn l_apercu_ecran_n_est_pas_une_preuve_colorimetrique() {
    assert!(bloc().contains("data-t=\"seance.plages.avertissement\""));
    assert!(texte(Langue::Francais, "seance.plages.avertissement")
        .contains("n’est pas une preuve colorimétrique"));
    assert!(
        texte(Langue::Anglais, "seance.plages.avertissement").contains("not colorimetric proof")
    );
}
