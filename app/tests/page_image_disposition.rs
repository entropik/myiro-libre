//! La page, lue comme un fichier : disposition de la section « Image » en
//! deux colonnes, image agrandie et case « Compensation du point noir »
//! (ticket #80, essai d'écran du 30 octobre 2026).

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

/// reglages-image.js et image-agrandie.js sont chargés après leurs outils et
/// avant comparer-image.js, sous 400 lignes, sans couleur en dur.
#[test]
fn les_scripts_de_la_section_image_sont_charges_dans_l_ordre() {
    let page = lire("interface/index.html");
    let position = |nom: &str| {
        page.find(&format!("<script src=\"{nom}\" defer>"))
            .unwrap_or_else(|| panic!("{nom} non chargé"))
    };
    for nom in ["reglages-image.js", "image-agrandie.js"] {
        assert!(position("arbre.js") < position(nom), "arbre.js avant {nom}");
        assert!(position("graphiques.js") < position(nom));
        assert!(position(nom) < position("comparer-image.js"), "{nom}");
        let script = lire(&format!("interface/{nom}"));
        let lignes = script.lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
        let hexa = script.match_indices('#').any(|(i, _)| {
            script[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count()
                >= 3
        });
        assert!(!hexa, "{nom} : couleur hexadécimale en dur");
        for interdit in ["rgb(", "rgba(", "hsl(", "style=", ".toFixed("] {
            assert!(!script.contains(interdit), "{nom} : {interdit}");
        }
    }
    for nom in ["comparer-image.js", "pastilles-image.js"] {
        let lignes = lire(&format!("interface/{nom}")).lines().count();
        assert!(lignes < 400, "{nom} : {lignes} lignes");
    }
}

/// Deux colonnes : l'image à gauche, jamais plus haute que 70 % de la
/// fenêtre (cadre aux proportions de l'image, `object-fit: contain`) ; à
/// droite un panneau de 320 à 380 px collé en haut au défilement. Quand la
/// section est étroite, les colonnes s'empilent et l'image passe à 50 %.
#[test]
fn la_section_image_a_deux_colonnes_qui_s_empilent_si_elle_est_etroite() {
    let css = lire("../design-system/components.css");
    let colonnes = ligne_css(&css, ".image-comparee__colonnes {");
    for attendu in [
        "display: grid;",
        "minmax(320px, 1fr)",
        "--hauteur-image: 70vh;",
    ] {
        assert!(colonnes.contains(attendu), "{attendu} : {colonnes}");
    }
    assert!(ligne_css(&css, ".image-comparee {").contains("container-type: inline-size;"));
    let panneau = ligne_css(&css, ".image-comparee__panneau {");
    for attendu in [
        "position: sticky;",
        "top: 0;",
        "overflow-y: auto;",
        "overflow-x: hidden;",
    ] {
        assert!(panneau.contains(attendu), "{attendu} : {panneau}");
    }
    let etroit = css
        .split("@container (max-width: 760px) {")
        .nth(1)
        .and_then(|r| r.split("\n}").next())
        .expect("colonnes empilées quand la section est étroite");
    assert!(etroit.contains("grid-template-columns: minmax(0, 1fr); --hauteur-image: 50vh;"));
    let cadre = ligne_css(&css, ".pipette {");
    assert!(
        cadre.contains("width: min(100%, calc(var(--hauteur-image, 70vh) * var(--ratio, 1.5)));"),
        "{cadre}"
    );
    assert!(ligne_css(&css, ".surimpression {").contains("object-fit: contain;"));
    // Pastilles : autant par rangée que la largeur du panneau en loge.
    let grille = ligne_css(&css, ".pastilles {");
    assert!(
        grille.contains(
            "grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-16) + var(--space-4)), 1fr));"
        ),
        "{grille}"
    );
    let kit = lire("../design-system/ui-kit.html");
    for attendu in [
        "class=\"image-comparee\"",
        "class=\"image-comparee__panneau\"",
        "class=\"agrandie__barre\"",
    ] {
        assert!(kit.contains(attendu), "ui-kit sans {attendu}");
    }
}

/// « Agrandir » ouvre une fenêtre modale sur toute la fenêtre ; le cadre de
/// l'image (surimpression, repère, pipette) y est déplacé tel quel, avec le
/// point cliqué dessous. « Fermer », Échap (événement `close` du dialogue)
/// et un clic hors de l'image ferment ; le cadre revient à sa place et le
/// focus au bouton. Un nouveau dessin de la section ferme l'image agrandie.
#[test]
fn l_image_agrandie_se_ferme_au_bouton_a_echap_et_hors_de_l_image() {
    let script = lire("interface/image-agrandie.js");
    let ouvrir = corps(&script, "function ouvrir(cadre, lecture, bouton) {");
    for attendu in [
        "el(\"dialog\", \"agrandie\")",
        "t(\"image.agrandie.fermer\")",
        "dialogue.append(barre, cadre, lecture());",
        "dialogue.addEventListener(\"close\", rendre);",
        "dialogue.showModal();",
        "!e.target.closest(\".surimpression, .agrandie__barre, .agrandie__lecture\")",
    ] {
        assert!(ouvrir.contains(attendu), "{attendu} :\n{ouvrir}");
    }
    let rendre = corps(&script, "function rendre() {");
    for attendu in [
        "place.replaceWith(cadre);",
        "bouton.focus();",
        "dialogue.remove();",
    ] {
        assert!(rendre.contains(attendu), "{attendu} :\n{rendre}");
    }
    assert!(script.contains("t(\"image.agrandir\")"));
    let image = lire("interface/comparer-image.js");
    let dessiner = corps(&image, "function dessiner() {");
    assert!(
        dessiner.contains("window.ImageAgrandie.fermer();"),
        "{dessiner}"
    );
    let surimpression = corps(&image, "function surimpression() {");
    assert!(
        surimpression.contains("window.ImageAgrandie.bouton(cadre, window.PastillesImage.lecture)")
    );
    let css = lire("../design-system/components.css");
    let agrandie = ligne_css(&css, ".agrandie {");
    for attendu in [
        "width: 100vw;",
        "height: 100vh;",
        "background: var(--fond);",
    ] {
        assert!(agrandie.contains(attendu), "{attendu} : {agrandie}");
    }
}

/// « Compensation du point noir » est cochée par défaut (et le redevient à la
/// fermeture de la comparaison) ; la changer relance l'analyse, et la
/// pipette suit le même réglage. Le réglage est rappelé dans la légende de
/// la surimpression et au-dessus des chiffres. Le champ qui avait le focus
/// le retrouve après le nouveau dessin.
#[test]
fn la_compensation_du_point_noir_se_regle_et_se_rappelle() {
    let image = lire("interface/comparer-image.js");
    assert!(image.contains("let pointNoir = true;"));
    let changer = corps(&image, "function changerPointNoir(coche) {");
    assert!(changer.contains("pointNoir = coche;"), "{changer}");
    assert!(changer.contains("recalculer();"), "{changer}");
    assert!(image.contains("invoke(\"image_analyser\", { sources, compensation: pointNoir })"));
    let fermer = corps(&image, "function fermer() {");
    assert!(fermer.contains("\"les_deux\", true, true]"), "{fermer}");
    let calque = corps(&image, "function calque() {");
    assert!(
        calque.contains("t(compense ? \"image.legende.point_noir\" : \"image.legende\")"),
        "{calque}"
    );
    let surimpression = corps(&image, "function surimpression() {");
    assert!(surimpression.contains("R.pointNoir(pointNoir, changerPointNoir)"));
    assert!(surimpression
        .contains("t(compense ? \"image.rendu.point_noir\" : \"image.rendu.relatif\")"));
    let dessiner = corps(&image, "function dessiner() {");
    assert!(
        dessiner.contains("document.getElementById(focus)?.focus();"),
        "{dessiner}"
    );

    let reglages = lire("interface/reglages-image.js");
    let case = corps(&reglages, "function pointNoir(coche, changer) {");
    for attendu in [
        "type: \"checkbox\", checked: coche, id: \"image-point-noir\"",
        "changer(caseACocher.checked)",
        "t(\"image.point_noir\")",
        "t(\"image.point_noir.aide\")",
    ] {
        assert!(case.contains(attendu), "{attendu} :\n{case}");
    }
    // Le point cliqué dit la couleur visée, ou que c'est celle de l'image.
    let pastilles = lire("interface/pastilles-image.js");
    let visee = corps(&pastilles, "function visee(p) {");
    for attendu in [
        "t(\"pastilles.pipette.visee_identique\")",
        "pastille(p.visee, p.couleur_visee, \"visee\")",
        "t(\"pastilles.pipette.visee\")",
    ] {
        assert!(visee.contains(attendu), "{attendu} :\n{visee}");
    }
}

/// Les textes de la disposition, de l'image agrandie et de la compensation
/// existent en français et en anglais ; la compensation est rappelée dans la
/// légende.
#[test]
fn les_textes_de_l_image_agrandie_et_du_point_noir_existent_en_deux_langues() {
    use app::textes::{texte, Langue};
    for cle in [
        "image.agrandir",
        "image.agrandie.titre",
        "image.agrandie.fermer",
        "image.agrandie.aide",
        "image.point_noir",
        "image.point_noir.aide",
        "image.legende.point_noir",
        "image.rendu.point_noir",
        "image.rendu.relatif",
        "pastilles.pipette.visee",
        "pastilles.pipette.visee_identique",
    ] {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} manque");
        }
    }
    assert_eq!(
        texte(Langue::Francais, "image.point_noir"),
        "Compensation du point noir"
    );
    assert!(
        texte(Langue::Francais, "image.legende.point_noir").contains("compensation du point noir")
    );
    assert!(texte(Langue::Anglais, "image.legende.point_noir").contains("black point compensation"));
    for langue in [Langue::Francais, Langue::Anglais] {
        let visee = texte(langue, "pastilles.pipette.visee");
        assert!(visee.contains("{lab}") && visee.contains("{e}"), "{visee}");
    }
}

/// « NE JAMAIS BOUGER l'UI » (mainteneur, 30 octobre 2026) : changer la case
/// du point noir relance l'analyse sur place (`recalculer`), sans la
/// fonction qui reconstruit la section ; les anciennes valeurs restent,
/// atténuées, jusqu'aux nouvelles. Le seuil, le profil et l'affichage ne
/// font que des mises à jour ciblées. Le point cliqué redemandé garde
/// l'ancienne paire en place jusqu'à la nouvelle.
#[test]
fn changer_un_reglage_met_a_jour_sur_place_sans_redessiner_la_section() {
    let image = lire("interface/comparer-image.js");
    let changer = corps(&image, "function changerPointNoir(coche) {");
    assert!(changer.contains("recalculer();"), "{changer}");
    assert!(
        !changer.contains("dessiner(") && !changer.contains("analyser("),
        "{changer}"
    );
    let recalculer = corps(&image, "async function recalculer() {");
    for attendu in [
        "etatCalcul.etat(\"calcul\");",
        "choisirProfil(choisi);",
        "dessinerChiffres();",
        "window.PastillesImage.reprendre();",
    ] {
        assert!(recalculer.contains(attendu), "{attendu} :\n{recalculer}");
    }
    for interdit in ["dessiner();", "replaceChildren", "surimpression("] {
        assert!(!recalculer.contains(interdit), "{interdit} :\n{recalculer}");
    }
    for entete in [
        "function changerSeuil(valeur) {",
        "function choisirProfil(rang) {",
    ] {
        let corps = corps(&image, entete);
        assert!(!corps.contains("dessiner();"), "{entete} :\n{corps}");
    }
    assert!(image.contains("R.affichage(affichage, (v) => { affichage = v; calque(); })"));
    let pastilles = lire("interface/pastilles-image.js");
    let reprendre = corps(&pastilles, "function reprendre() {");
    assert!(!reprendre.contains("pixel = null"), "{reprendre}");
    let css = lire("../design-system/components.css");
    assert!(css.contains(
        ".image-comparee__panneau--recalcul .image-comparee__resultats { opacity: .45; }"
    ));
}

/// Disposition figée : barre de la légende, état de l'analyse, point cliqué,
/// texte de lecture, chiffres et ligne du rendu ont une hauteur fixe ; la
/// grille des pastilles a toujours ses 12 emplacements (3 × 4, rangées de
/// hauteur fixe), vides s'il y a moins de couleurs, la phrase « aucune » ou
/// la raison d'un profil inconnu posée par-dessus. L'état de l'analyse
/// s'affiche à sa place gardée (visibilité seulement), jamais inséré.
#[test]
fn la_mise_en_page_garde_ses_emplacements_reserves() {
    let css = lire("../design-system/components.css");
    let attendus: [(&str, &[&str]); 10] = [
        (
            ".pastilles {",
            &[
                "grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-16) + var(--space-4)), 1fr));",
                "grid-auto-rows: calc(var(--space-12) * 2);",
            ],
        ),
        (
            ".pastilles-pipette {",
            &["height: calc(var(--space-24) * 2.75);", "overflow: hidden;"],
        ),
        (
            ".pastilles-lecture {",
            &["height: calc(3em * var(--leading-normal));"],
        ),
        (
            ".recalcul {",
            &["height: var(--space-12);", "visibility: hidden;"],
        ),
        (".recalcul--actif {", &["visibility: visible;"]),
        (
            ".image-comparee__barre {",
            &["flex-wrap: nowrap;", "height: var(--space-12);"],
        ),
        (
            ".image-comparee__rendu {",
            &["height: calc(2em * var(--leading-normal));"],
        ),
        (".chiffres-image table {", &["table-layout: fixed;"]),
        (
            ".chiffres-image__cellule {",
            &[
                "height: calc(var(--lignes, 1) * 1em * var(--leading-normal));",
                "overflow: hidden;",
            ],
        ),
        (
            ".pastilles__message {",
            &["position: absolute;", "inset: 0;"],
        ),
    ];
    for (selecteur, valeurs) in attendus {
        let ligne = ligne_css(&css, selecteur);
        for attendu in valeurs {
            assert!(ligne.contains(attendu), "{selecteur} : {attendu} : {ligne}");
        }
    }

    let pastilles = lire("interface/pastilles-image.js");
    assert!(pastilles.contains("const AFFICHEES = 12;"));
    let grille = corps(&pastilles, "function grille() {");
    for attendu in [
        "for (let i = 0; i < AFFICHEES; i++) {",
        ": vide());",
        "el(\"p\", \"pastilles__message\")",
        "t(\"pastilles.aucune\")",
    ] {
        assert!(grille.contains(attendu), "{attendu} :\n{grille}");
    }
    let pipette = corps(&pastilles, "function pipette() {");
    let bloc = pipette
        .find("el(\"div\", \"pastilles-pipette\")")
        .unwrap_or(usize::MAX);
    let aide = pipette.find("t(\"pastilles.pipette.aide\")").unwrap_or(0);
    assert!(
        bloc < aide,
        "bloc du point cliqué présent même vide :\n{pipette}"
    );
    let ecrire = corps(&pastilles, "function ecrire() {");
    assert!(
        ecrire.contains("pipette(),") && ecrire.contains("grille(),"),
        "{ecrire}"
    );
    assert!(!ecrire.contains("if (analyse)"), "{ecrire}");

    let image = lire("interface/comparer-image.js");
    assert!(image.contains("const LIGNES = [3, 2, 2, 2, 3, 3, 2, 3];"));
    let chiffres = corps(&image, "function dessinerChiffres() {");
    for attendu in [
        "[...Array(7)]",
        "[...cles, \"image.chiffres.raison\"]",
        "cellule(v, LIGNES[i])",
    ] {
        assert!(chiffres.contains(attendu), "{attendu} :\n{chiffres}");
    }
    let surimpression = corps(&image, "function surimpression() {");
    assert!(
        !surimpression.contains("if (analyses)"),
        "chiffres toujours là :\n{surimpression}"
    );
    let dessiner = corps(&image, "function dessiner() {");
    for attendu in [
        "if (enCours && !avecPanneau)",
        "if (message && !avecPanneau)",
        "etatCalcul.etat(enCours ? \"calcul\" : relancer ? \"relancer\" : \"repos\", message);",
    ] {
        assert!(dessiner.contains(attendu), "{attendu} :\n{dessiner}");
    }
    let reglages = lire("interface/reglages-image.js");
    let etat = corps(
        &reglages,
        "function etatCalcul(panneau, annuler, relancer) {",
    );
    for attendu in [
        "classList.toggle(\"recalcul--actif\"",
        "setAttribute(\"aria-busy\"",
        "t(\"image.en_cours\")",
    ] {
        assert!(etat.contains(attendu), "{attendu} :\n{etat}");
    }
}

/// Les deux colonnes occupent toute la largeur de la section : la colonne
/// de l'image prend la largeur de la photo (au plus 70 % de la hauteur de la
/// fenêtre × proportions), en paysage comme en portrait ; le panneau prend
/// tout le reste, 320 px au moins, sans largeur maximale fixe.
#[test]
fn la_colonne_de_l_image_prend_la_largeur_de_la_photo() {
    let css = lire("../design-system/components.css");
    let colonnes = ligne_css(&css, ".image-comparee__colonnes {");
    assert!(
        colonnes.contains(
            "grid-template-columns: minmax(0, calc(var(--hauteur-image) * var(--ratio, 1.5))) minmax(320px, 1fr);"
        ),
        "{colonnes}"
    );
    for interdit in ["clamp(", "justify-content: start;", "380px"] {
        assert!(!colonnes.contains(interdit), "{interdit} : {colonnes}");
    }
    let panneau = ligne_css(&css, ".image-comparee__panneau {");
    assert!(!panneau.contains("max-width"), "{panneau}");
    assert!(
        ligne_css(&css, ".pipette > .surimpression {").contains("aspect-ratio: var(--ratio, 1.5);")
    );
    let image = lire("interface/comparer-image.js");
    let surimpression = corps(&image, "function surimpression() {");
    assert!(
        surimpression.contains("colonnes.style.setProperty(\"--ratio\", "),
        "{surimpression}"
    );
}

/// Les jetons des ascenseurs : épaisseur et longueur minimale de la poignée,
/// communes ; piste, poignée et survol dans chacun des deux thèmes. Le
/// système graphique les montre et les explique.
#[test]
fn les_jetons_d_ascenseur_existent_dans_les_deux_themes() {
    let jetons = lire("../design-system/tokens.css");
    let (sombre, reste) = jetons
        .split_once(":root[data-theme=\"light\"]")
        .expect("thème clair");
    let clair = &reste[..reste.find('}').unwrap_or(reste.len())];
    for bloc in [sombre, clair] {
        for jeton in [
            "--ascenseur-piste:",
            "--ascenseur-poignee:",
            "--ascenseur-poignee-survol:",
        ] {
            assert!(bloc.contains(jeton), "{jeton} absent d'un thème");
        }
    }
    assert!(jetons.contains("--ascenseur: 14px;"));
    assert!(jetons.contains("--ascenseur-longueur-min: 40px;"));
    assert!(lire("../design-system/ui-kit.html").contains("--ascenseur"));
    assert!(lire("../design-system/README.md").contains("--ascenseur"));
}

/// Jamais de texte collé à un ascenseur ni au bord : chaque zone qui défile
/// garde la place de l'ascenseur et a un retrait intérieur côté ascenseur
/// d'au moins `--gouttiere-ascenseur` (la zone du centre a déjà
/// `--space-8`). Le jeton est commun aux deux thèmes.
#[test]
fn les_zones_qui_defilent_ont_leur_retrait_cote_ascenseur() {
    let jetons = lire("../design-system/tokens.css");
    assert!(jetons.contains("--gouttiere-ascenseur: var(--space-3);"));
    let css = lire("../design-system/components.css");
    for selecteur in [
        ".zone-gauche {",
        ".zone-droite {",
        ".defile {",
        ".feuille__corps:not([hidden]) {",
        ".image-comparee__panneau {",
    ] {
        let ligne = ligne_css(&css, selecteur);
        assert!(
            ligne.contains("padding-right: var(--gouttiere-ascenseur);"),
            "{selecteur} : {ligne}"
        );
    }
    for selecteur in [
        ".zone-gauche, .zone-centre, .zone-droite {",
        ".defile {",
        ".feuille__corps:not([hidden]) {",
        ".image-comparee__panneau {",
        ".agrandie {",
    ] {
        let ligne = ligne_css(&css, selecteur);
        assert!(
            ligne.contains("scrollbar-gutter: stable;"),
            "{selecteur} : {ligne}"
        );
    }
    assert!(ligne_css(&css, ".zone-centre {").contains("padding: var(--space-6) var(--space-8);"));
    assert!(ligne_css(&css, ".agrandie {").contains("padding: var(--space-6);"));
    // Toute règle qui fait défiler a sa place gardée.
    for ligne in css
        .lines()
        .filter(|l| l.contains("overflow-y: auto") || l.contains("overflow: auto"))
    {
        assert!(ligne.contains("scrollbar-gutter: stable;"), "{ligne}");
    }
    let readme = lire("../design-system/README.md");
    assert!(readme.contains("--gouttiere-ascenseur"));
}

/// « Avancé : commande et journal d'iccgamut », posé seul dans le panneau
/// des détails de la comparaison, a le même retrait que les autres blocs du
/// panneau ; la commande et le journal, en `<pre>`, passent à la ligne et ont
/// leur propre retrait. Le même bloc de la provenance d'un profil calculé est
/// déjà dans un corps de section à retrait : la règle du `<pre>` vaut pour
/// les deux.
#[test]
fn le_bloc_avance_est_aligne_sur_le_panneau() {
    let details = lire("interface/comparer-details.js");
    let avance = corps(&details, "function avance(commande, liste) {");
    for attendu in [
        "el(\"div\", \"details-avance\")",
        "bloc.append(d);",
        "return bloc;",
    ] {
        assert!(avance.contains(attendu), "{attendu} :\n{avance}");
    }
    let css = lire("../design-system/components.css");
    let bloc = ligne_css(&css, ".details-avance {");
    assert!(
        bloc.contains("padding: 0 var(--space-4) var(--space-4);"),
        "{bloc}"
    );
    assert!(ligne_css(&css, ".section-repliable > summary {").contains("var(--space-4)"));
    let pre = ligne_css(&css, ".replie pre {");
    for attendu in [
        "padding: var(--space-2) var(--space-3);",
        "white-space: pre-wrap;",
        "overflow-wrap: anywhere;",
    ] {
        assert!(pre.contains(attendu), "{attendu} : {pre}");
    }
    let calcul = lire("interface/calcul-profil.js");
    assert!(calcul
        .contains("el(\"span\", \"label\", t(\"profil.journal\")), el(\"pre\", \"\", p.journal)"));
    let reglage = lire("interface/reglage.js");
    let section = corps(&reglage, "function section(titre, lignes) {");
    assert!(
        section.contains("el(\"div\", \"section-repliable__corps\")"),
        "{section}"
    );
}
