# L'interface est faite avec Tauri 2

L'application de bureau utilise Tauri 2 : cœur Rust, interface web rendue par WebView2 (présent sur Windows 11). Courbes spectrales, tableaux d'écarts, diagrammes de gamut et historiques de contrôle d'impression sont bien mieux servis par l'écosystème web que par les cadres natifs Rust.

## Options écartées

- egui : tout en Rust, mais rendu visuel brut pour une application destinée aux imprimeurs.
- Slint : natif et soigné, mais écosystème de graphiques trop pauvre.

## Conséquences

- L'interface est bilingue français/anglais, avec catalogue de textes dès le départ.
- Le cœur (colorimétrie, mires, bibliothèque, profilage) ne dépend pas de Tauri.

## Direction visuelle (7 octobre 2026)

Retenue après une maquette jetable à deux itérations (branche `prototype/interface-maquette`, `prototypes/interface-maquette/swiss.html`) :

- **Trois zones** : bibliothèque par condition d'impression à gauche, feuille de travail au centre, détails à droite. Barre du haut par tâche (Mesurer, Contrôler, Profiler, Bibliothèque), avec l'instrument et son état toujours visibles.
- **Grille suisse** : 12 colonnes (2, 7 et 3), filets fins, angles droits, pas d'ombres, une seule fonte lineale à corps optique, numérotation des éléments.
- **Cartouche d'architecte** pour la provenance : instrument, étalonnage, condition de mesure, référence, norme et version du jeu de tolérances sont toujours affichés avec le résultat qu'ils concernent.
- **Grande typographie et contraste** : le verdict est un mot en très grand corps, doublé d'une icône et de la couleur (jamais la couleur seule), avec la marge visible et le détail replié.
- **Règles d'usage** : un seul bouton principal par écran, toute action inactive affiche sa raison, rouge réservé au danger et au hors tolérance, une seule langue par fenêtre, section « Avancé » avec la commande ArgyllCMS équivalente.

Le système graphique qui en découle (jetons, composants, règles) est dans `design-system/`.

La maquette n'est pas du code de production : l'interface est réécrite dans l'application. Les valeurs qu'elle affiche (seuils, durées de lecture, réglages de profil) sont des hypothèses à établir.

## Cadre de l'application (7 octobre 2026)

- L'application est la crate `app/` de l'espace de travail Cargo. Elle est la seule à dépendre de Tauri ; un test (`app/tests/independance_coeur.rs`) lit le graphe des dépendances avec `cargo metadata` et échoue si une autre crate tire Tauri.
- La page est écrite en HTML, CSS et JavaScript simples, sans outil de construction web (pas de npm) : `cargo run -p app` suffit pour compiler et ouvrir l'application.
- Les textes de l'interface vivent dans un seul catalogue, côté Rust (`app/src/textes.rs`), en français et en anglais. La page le demande au lancement et n'écrit aucun texte en dur. Un test échoue si une clé manque dans une langue, un autre si le français ne suit pas sa typographie (apostrophe courbe, guillemets français, espace fine avant `: ; ! ?`).
- La langue se choisit dans la barre ou au lancement (`--langue en`) ; toute la fenêtre change de langue, jamais une partie.
- Jetons et composants restent dans `design-system/` : la compilation recopie `tokens.css` et `components.css` dans la page sans les modifier. Les composants du cadre (bandeau, en-tête de feuille, état vide, cadre à trois zones) ont été ajoutés à `ui-kit.html`.
- La fonte Inter (licence SIL Open Font License) est livrée avec l'application, et la politique de sécurité de la page interdit tout chargement extérieur : aucun appel réseau au lancement.

## Textes de l'instrument et choix d'un dossier (7 octobre 2026)

Après un premier essai refusé par l'utilisateur (un champ où coller le chemin du SDK) :

- **Aucun mot technique à l'écran.** SDK, DLL, pont, palier et chemins n'apparaissent que dans le détail technique replié (composant « Détail replié »). La barre dit l'état de l'instrument en une phrase composée par le catalogue (`{modele}, {etat}`), ou « Logiciel du fabricant absent ».
- **Rien à saisir.** L'application trouve seule le logiciel du fabricant (ADR 0005). Seulement si elle échoue, une phrase et un bouton « Choisir le dossier… » ouvrent le sélecteur de dossier de Windows (extension Tauri `dialog`, appelée depuis Rust ; la page n'a aucune permission de plus).
- **Bouton inactif pendant la recherche**, avec sa raison écrite dessous.

## Plein cadre et rôle des zones (7 octobre 2026, ticket #6)

Demandé par le mainteneur à la première revue de la bibliothèque à l'écran :

- L'application remplit exactement la fenêtre : le bandeau reste en place, la page ne défile jamais, et seule une zone trop longue défile à l'intérieur d'elle-même (classe `app` du système graphique). Sous 64 rem de large, les zones s'empilent et la page défile de nouveau.
- La feuille du centre montre ce qui est choisi : pour une mesure, son en-tête (date, lecture, condition d'impression) puis le tableau des valeurs Lab par plage avec la bascule de condition de mesure, en grand ; l'emplacement de la courbe de spectre est réservé. Sans choix, un état vide juste (« Choisissez une mesure dans la bibliothèque. »), ou l'invitation à ajouter une première condition d'impression, avec une action active, jamais un bouton inactif en double.
- Les détails, à droite, ne gardent que le cartouche de provenance.

## Bandeau et étalonnage (7 octobre 2026, ticket #4)

- Les tâches occupent cinq colonnes du bandeau (`c-3-7`) et l'instrument avec les réglages les cinq suivantes (`c-8-12`) : le bouton « Étalonner » tient à côté de l'état sans faire grandir le bandeau. Il n'apparaît que lorsque l'état est « étalonnage requis ».
- L'étalonnage guidé occupe la feuille du centre, sans fenêtre par-dessus : trois étapes (poser sur le blanc, étalonnage, mesure), un schéma au trait maison (composant `schema`), une seule action principale à la fois. Après un échec, l'avis remplace l'explication et la consigne, pour que tout tienne sans défilement.
- Le passage aux zones empilées se fait strictement sous 64 rem : à 1024 px, largeur minimale de la fenêtre, l'application garde le plein cadre à trois zones. Vérifié par captures sans affichage à 1024×768, 1280×800 et 1440×900.

## Couleur de référence et écart dans Mesurer (9 octobre 2026, ticket #8)

- La zone de détails montre, pour la mesure choisie : sur la couleur de référence, l'écart accepté (saisi à la main, virgule ou point, vide pour ne pas juger) et « Retirer la référence » ; sur une autre mesure, le verdict en grand (corps `verdict__word--m`, pour tenir dans trois colonnes) doublé d'un carré à symbole, un carré partagé (référence en haut, mesure en bas), puis ΔE00 (CIEDE2000), ΔC (saturation) et ΔH (teinte) avec la formule écrite à côté ; le détail du calcul est replié.
- La liste ajoute ΔE00 et un petit verdict (carré et mot) à chaque mesure ; la référence porte le libellé « Référence ». Sans écart accepté : « Seuil non fixé », carré vide, aucun verdict.
- L'écart compare les spectres de même condition de mesure (M0 avec M0…), quelle que soit leur place ; si elles ne sont pas confirmées, un avis le dit au-dessus des chiffres ; si elles diffèrent, l'avis remplace l'écart et le verdict.
- `cargo run -p app -- --demo` garnit aussi la séance de Mesurer de mesures fictives, avec une couleur de référence et ses quatre cas (trois verdicts, un avis).

## Écran lisible en plein cadre (9 octobre 2026, refus de l'écran Mesurer)

Après le refus de l'écran (« illisible », fenêtre d'environ 1900 × 1080) :

- L'application prend toute la largeur de la fenêtre (`.app .page`, sans largeur maximale, marges et gouttières de 1 rem) ; les pages de documentation du système graphique gardent leur mesure.
- La colonne de la bibliothèque, commune à toutes les tâches, passe de deux à trois colonnes de la grille ; la feuille en prend six, les détails trois (au lieu de 2 + 7 + 3). Une entrée de mesure tient sur deux lignes : le nom, puis la date courte et la lecture ; aucun mot n'est coupé, et une fin trop longue est remplacée par des points, le texte entier restant au survol.
- Aucune zone ne défile en largeur ; toutes les barres de défilement sont fines ; deux champs qui se suivent sont espacés.
- Dans Mesurer, les boutons de spectre portent « M0 », « M1 », « M2 » et « à confirmer » est dit une seule fois dessous. Les détails montrent d'abord l'écart, compact et titré, puis la couleur, les valeurs et la provenance ; les actions sur la couleur de référence viennent en dernier.
