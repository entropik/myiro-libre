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

const SCRIPTS: [&str; 3] = ["seance-plages.js", "seance-apercu.js", "seance.js"];

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
    assert!(position("seance-plages.js") < position("seance-apercu.js"));
    assert!(position("seance-apercu.js") < position("seance.js"));
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
    assert_eq!(b.matches("<div class=\"feuille-fd9__etat\"").count(), 5);
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

/// Le corps d'une fonction d'un script : de `function nom(` à la prochaine
/// fonction de même niveau.
fn corps(script: &str, nom: &str) -> String {
    let debut = script
        .find(&format!("function {nom}("))
        .unwrap_or_else(|| panic!("function {nom} absente"));
    let fin = script[debut + 1..]
        .find("\n  function ")
        .or_else(|| script[debut + 1..].find("\n  // ----"))
        .map_or(script.len(), |f| debut + 1 + f);
    script[debut..fin].to_string()
}

/// Relecture de l'écran : la fiche repart vide, case décochée, à chaque
/// autre résolution, à chaque tirage sans séance et après la fin d'une
/// séance ; jamais une confirmation qui suit d'un tirage à l'autre.
#[test]
fn la_fiche_repart_vide_et_non_confirmee() {
    let s = script("seance.js");
    let vider = corps(&s, "viderFiche");
    for attendu in ["champ.value = \"\";", "confirmee.checked = false;"] {
        assert!(vider.contains(attendu), "{attendu} :\n{vider}");
    }
    assert!(s.matches("viderFiche();").count() >= 3, "{s}");
    let dessiner = corps(&s, "dessiner");
    assert!(dessiner.contains("viderFiche();"), "{dessiner}");
}

/// Après « Terminer » : la séance et la fiche sont remises à vide, un
/// tirage restant est choisi et peut se commencer sans changer de
/// résolution ; un message gardé dit la fin et son résultat.
#[test]
fn apres_la_fin_un_tirage_restant_peut_se_commencer() {
    let s = script("seance.js");
    let fin = &s[s.find("terminer.addEventListener(\"click\"").unwrap()..];
    let fin = &fin[..fin.find("}));").unwrap()];
    for attendu in [
        "seance = null;",
        "etat = null;",
        "viderFiche();",
        "seance.terminee.resultat",
        "await parc.recharger();",
    ] {
        assert!(fin.contains(attendu), "{attendu} :\n{fin}");
    }
    present("seance.terminee.resultat");
    present("seance.incomplete.resultat");
}

/// Les écarts signalés se montrent : plage, feuille et ΔE00, dans un bloc
/// de taille gardée qui défile ; la grille d'inspection les borde aussi.
#[test]
fn les_ecarts_signales_se_montrent() {
    let b = bloc();
    assert!(b.contains(
        "<div class=\"feuille-fd9__echec defile-gauche\" role=\"list\" data-t-aria=\"seance.ecarts\" data-seance-ecarts>"
    ));
    assert!(b.contains("<template data-seance-ecart-modele>"));
    let apercu = script("seance-apercu.js");
    for attendu in [
        "e.signale",
        "seance.ecarts.ligne",
        "seance.ecarts.valeur",
        "e.de00",
    ] {
        assert!(apercu.contains(attendu), "{attendu}");
    }
    let plages = script("seance-plages.js");
    assert!(plages.contains("jeton(\"--limite\")"));
}

/// Chaque bouton inactif dit pourquoi, juste à côté, dans une ligne gardée :
/// sous la rangée des boutons de la séance, et sous « Terminer la séance ».
#[test]
fn chaque_bouton_inactif_de_la_seance_dit_pourquoi() {
    let b = bloc();
    let terminer = b.find("data-seance-terminer").unwrap();
    let apres = &b[terminer..];
    let raison = apres.find("<p class=\"why").unwrap();
    assert!(apres[raison..].starts_with(
        "<p class=\"why feuille-fd9__raison\" role=\"status\" data-seance-raison-fin>"
    ));
    assert!(!apres[..raison].contains("<button"));
    let s = script("seance.js");
    for attendu in [
        "seance.raison.sans_seance",
        "seance.raison.aucune_feuille",
        "seance.raison.deja_commencee",
        "seance.raison.chargement",
        "raisonFin.textContent",
    ] {
        assert!(s.contains(attendu), "{attendu}");
    }
}

/// Les anciennes valeurs restent jusqu'aux nouvelles : une autre séance ne
/// vide pas l'écran avant son état ; une inspection n'écrit le nom de la
/// feuille qu'une fois ses plages reçues.
#[test]
fn les_anciennes_valeurs_restent_jusqu_aux_nouvelles() {
    let s = script("seance.js");
    let montrer = corps(&s, "montrerSeance");
    assert!(montrer.contains("chargement = true;"), "{montrer}");
    assert!(!montrer.contains("    etat = null;\n"), "{montrer}");
    let p = script("seance-plages.js");
    let ouvrir = corps(&p, "ouvrir");
    let recu = ouvrir.find("plages = r;").unwrap();
    let nom = ouvrir.find("nomFichier.textContent = fichier;").unwrap();
    assert!(recu < nom, "{ouvrir}");
}

/// Fermer l'inspection rend le focus au bouton « Inspecter » ; l'ouvrir
/// amène la grille dans la vue. Le fichier d'origine se lit sur demande.
#[test]
fn l_inspection_garde_le_focus_et_montre_le_fichier_d_origine() {
    let p = script("seance-plages.js");
    for attendu in [
        "retour.focus()",
        "scrollIntoView({ block: \"nearest\" })",
        "invoke(\"seance_original\"",
    ] {
        assert!(p.contains(attendu), "{attendu}");
    }
    let b = bloc();
    assert!(b.contains(
        "<button class=\"btn\" data-seance-plages-original data-t=\"seance.plages.original\" aria-expanded=\"false\" aria-controls=\"seance-original\"></button>"
    ));
    assert!(b.contains("<pre data-seance-original-texte></pre>"));
}

/// La mire du plan est nommée ; un refus de moyennage nomme la feuille en
/// cause ; aucun texte en dur pour les nombres et les conditions.
#[test]
fn la_mire_et_la_feuille_en_cause_sont_nommees_sans_texte_en_dur() {
    assert!(bloc().contains("data-seance-mire"));
    let apercu = script("seance-apercu.js");
    for attendu in [
        "seance.mire",
        "refus.incompatible.source",
        "seance.refus.feuille",
        "seance.apercu.plages.valeur",
        "seance.apercu.manquantes.valeur",
        "seance.emplacement.",
    ] {
        assert!(apercu.contains(attendu), "{attendu}");
    }
    for nom in SCRIPTS {
        let s = script(nom);
        for en_dur in ["} / ${", "\"M0\"", "\"M1\"", "\"M2\""] {
            assert!(!s.contains(en_dur), "{nom} : {en_dur}");
        }
    }
    for cle in [
        "seance.emplacement.m0",
        "seance.emplacement.m1",
        "seance.emplacement.m2",
    ] {
        present(cle);
    }
}
