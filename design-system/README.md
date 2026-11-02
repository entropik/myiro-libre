# Système graphique de myiro-libre

Identité visuelle de l'application : thème bleu nuit (sombre par défaut, bascule vers le clair), cartes, accent bleu, verdicts vert, orange et rouge. Repris du prototype validé du poste unique (voir `docs/adr/0007-poste-unique.md` et `docs/adr/0003-interface-tauri.md`, section « Direction visuelle »). Il remplace, depuis le 22 octobre 2026, la grille suisse ; le cartouche de provenance reste, restylé en carte.

Ouvrir `index.html` dans un navigateur (double-clic, sans compilation).

| Fichier | Rôle |
|---|---|
| `tokens.css` | Source unique : couleurs (sombre et clair), typographie, espacements, filets |
| `components.css` | Composants de l'application, écrits avec les seuls jetons |
| `doc.css` | Mise en page des pages de documentation (pas de l'application) |
| `brand-book.html` | Couleurs, contrastes WCAG calculés |
| `typography.html` | Échelle, graisses, libellés, chiffres, règles de composition française |
| `ui-kit.html` | Composants avec leur code |
| `patterns.html` | Assemblages : le cadre à trois zones et la vue d'une résolution |
| `references/` | Voir `references/README.md` |

## Règles

- Aucune couleur, aucun corps de texte, aucun espacement hors de `tokens.css`. Pas de `!important`, pas de style en ligne sauf une valeur dynamique passée par une propriété personnalisée (`--c`, `--ref`, `--mes`, `--n`, et `--position` de la barre d'avancement). Seules exceptions écrites dans `components.css` : les dimensions du cadre (barre de 64 px, zones de 270 px et 300 px).
- Un composant absent de `ui-kit.html` n'existe pas : on l'ajoute d'abord à la page, puis à `components.css`.
- Cadre : barre du haut, puis trois zones (bibliothèque, centre, détails). Seules les zones défilent, jamais la fenêtre ni en largeur.
- Un seul bouton principal par écran ; tout bouton inactif dit pourquoi, juste à côté. Un verdict est toujours une couleur, une icône et un mot : la couleur n'est jamais seule. Le rouge est réservé au danger et au hors tolérance.
- Aucune ombre, aucun dégradé, aucun emoji ; des angles droits, sauf la pastille de l'instrument (un cercle).
- Les profils d'une comparaison prennent, dans l'ordre, les couleurs de série `--serie-1` à `--serie-4` (bleu, orange, violet, gris) : jamais le vert ni le rouge des verdicts, et toujours avec le nom du profil dans la légende.
- La surimpression d'une image comparée à un profil marque les pixels au-delà du seuil d'alerte en vert fluo `--couleur-hors-gamut-calque`, le même dans les deux thèmes (ni verdict ni série) ; en « alerte seule », les autres pixels prennent `--couleur-hors-gamut-fond`. La légende et les textes prennent `--couleur-hors-gamut`, plus sombre en thème clair pour rester lisible sur la carte, et le vert a toujours sa légende écrite à côté.
- Les ascenseurs sont larges partout : `--ascenseur` (14 px), poignée d'au moins `--ascenseur-longueur-min` (40 px) en `--ascenseur-poignee` sur une piste `--ascenseur-piste`, `--ascenseur-poignee-survol` au survol, angles droits, sans ombre. La fenêtre (Chromium) les dessine par `::-webkit-scrollbar`, qu'elle ignorerait si `scrollbar-width` était posé ; les autres moteurs reçoivent `scrollbar-width` et `scrollbar-color`. Les zones qui défilent gardent la place de l'ascenseur (`scrollbar-gutter: stable`).
- Jamais de texte collé à un ascenseur ni au bord : toute zone qui défile a un retrait intérieur côté ascenseur d'au moins `--gouttiere-ascenseur` (`--space-3`), en plus de la place gardée. Un bloc posé seul dans un panneau de détails (par exemple « Avancé ») prend le même retrait que les autres blocs du panneau, et un texte en `<pre>` (commande, journal) passe à la ligne et a son propre retrait.
- Une zone qui défile dans une zone qui défile déjà prend la classe `defile-gauche` : son ascenseur passe à gauche (`direction: rtl` sur la zone, `ltr` sur son contenu), avec la place gardée et le retrait de ce côté. Jamais deux ascenseurs côte à côte.
- Accordéon : les grandes parties d'une vue se replient (`accordeon.js`). Tout l'en-tête est un bouton (`aria-expanded`, `aria-controls`, chevron à droite), le contenu s'ouvre juste dessous, et seuls les chiffres clés sont ouverts par défaut ; l'état est retenu dans le navigateur, la page marche sans. Une partie lourde ne se dessine qu'ouverte. Ni ombre ni arrondi. Une partie reprise dans le rapport PDF a, à droite de son en-tête et hors du bouton, une coche « Inclure dans le rapport » (`accordeon__rapport`), décochée par défaut et retenue ; les chiffres clés et l'image y sont toujours (coche cochée, inactive, avec sa raison).
- Règle du mainteneur : l'interface ne bouge jamais quand une valeur change. Chaque bloc qui affiche une valeur a une taille réservée (hauteur fixe, emplacements comptés, texte coupé plutôt qu'agrandi), et un état (« Analyse en cours… ») s'affiche à une place gardée, jamais inséré.
- La section Image de la comparaison a deux colonnes : l'image entière à gauche (au plus 70 % de la hauteur de la fenêtre), un panneau à droite qui prend toute la largeur restante (320 px au moins), collé en haut au défilement ; elles s'empilent quand la section a moins de 760 px de large. L'image agrandie est une fenêtre modale à fond plein (`--fond`), sans ombre.
- Les pastilles hors gamut (à côté de l'image comparée) et la couleur d'un point cliqué à la pipette sont, avec les points du nuage, les seules couleurs que la page pose elle-même : des couleurs calculées par la crate `gamut`, contrôlées en `#rrggbb` et passées par `--c`. Une ligne dit toujours qu'elles sont approchées à l'écran. Le repère du point cliqué est un carré à l'encre bordé du fond (`--encre`, `--fond`), lisible sur une photo claire comme sombre.
- Rapport PDF (ticket #89) : la fenêtre d'export (`fenetre-rapport`) n'a qu'un bouton principal et écrit son état à une place gardée ; « Ouvrir le dossier » est caché sans perdre sa place. Les points relevés ont huit emplacements toujours présents. Les parties cochées se redessinent dans `.hors-ecran`, jamais visible. Le rapport lui-même suit son propre gabarit sobre (crate `rapport`, ADR 0008) : noir et gris, filets, ni ombre ni arrondi.
- Seule la barre de calcul peut porter un dégradé : une lumière spectrale animée, signe que le calcul travaille. Partout ailleurs, pas de dégradé. Ses couleurs sont les jetons `--spectre-1` à `--spectre-6` ; son mélange et sa force (`--lumiere-melange`, `--lumiere-force`) suivent le thème.
- Une seule animation : la barre d'avancement d'un calcul (remplissage qui glisse en 300 ms, lumière spectrale qui traverse la piste en 2,6 s). Rien ne bouge et la lumière est cachée si l'utilisateur réduit les animations (`prefers-reduced-motion`).
- Icônes des tâches : tracés SVG propres au projet, aucune icône reprise d'un autre logiciel.
- Texte en français : espaces fines insécables avant `: ; ! ?`, guillemets français, apostrophe courbe, virgule décimale, chiffres tabulaires dans les tableaux.
- Les valeurs de contraste de `brand-book.html` sont calculées à partir de `tokens.css` (formule WCAG 2) ; changer une couleur impose de les recalculer.

Les tests de `app/tests/page.rs` lisent ces fichiers : jetons du prototype, aucune couleur hors des jetons, ni ombre ni arrondi hors de la pastille, ni dégradé hors de la lumière de la barre de calcul, largeurs des zones.

## Fonte

Inter (variable, axe de corps optique), chargée depuis Google Fonts dans les pages de documentation. L'application embarque la fonte localement (`app/interface/fontes/`, pas d'appel réseau, ADR 0003).
