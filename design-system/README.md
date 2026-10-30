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
- Seule la barre de calcul peut porter un dégradé : une lumière spectrale animée, signe que le calcul travaille. Partout ailleurs, pas de dégradé. Ses couleurs sont les jetons `--spectre-1` à `--spectre-6` ; son mélange et sa force (`--lumiere-melange`, `--lumiere-force`) suivent le thème.
- Une seule animation : la barre d'avancement d'un calcul (remplissage qui glisse en 300 ms, lumière spectrale qui traverse la piste en 2,6 s). Rien ne bouge et la lumière est cachée si l'utilisateur réduit les animations (`prefers-reduced-motion`).
- Icônes des tâches : tracés SVG propres au projet, aucune icône reprise d'un autre logiciel.
- Texte en français : espaces fines insécables avant `: ; ! ?`, guillemets français, apostrophe courbe, virgule décimale, chiffres tabulaires dans les tableaux.
- Les valeurs de contraste de `brand-book.html` sont calculées à partir de `tokens.css` (formule WCAG 2) ; changer une couleur impose de les recalculer.

Les tests de `app/tests/page.rs` lisent ces fichiers : jetons du prototype, aucune couleur hors des jetons, ni ombre ni arrondi hors de la pastille, ni dégradé hors de la lumière de la barre de calcul, largeurs des zones.

## Fonte

Inter (variable, axe de corps optique), chargée depuis Google Fonts dans les pages de documentation. L'application embarque la fonte localement (`app/interface/fontes/`, pas d'appel réseau, ADR 0003).
