# Système graphique de myiro-libre

Identité visuelle de l'application : grille suisse de douze colonnes, filets, cartouche d'architecte pour la provenance, grande typographie, contraste fort. Issu de la maquette retenue (voir `docs/adr/0003-interface-tauri.md`, section « Direction visuelle »).

Ouvrir `index.html` dans un navigateur (double-clic, sans compilation).

| Fichier | Rôle |
|---|---|
| `tokens.css` | Source unique : couleurs (clair et sombre), typographie, espacements, filets, grille |
| `components.css` | Composants de l'application, écrits avec les seuls jetons |
| `doc.css` | Mise en page des pages de documentation (pas de l'application) |
| `brand-book.html` | Couleurs, contrastes WCAG calculés, fonds autorisés |
| `typography.html` | Échelle, graisses, libellés, chiffres, règles de composition française |
| `grid.html` | Gabarit de 12 colonnes, dispositions, rythme vertical |
| `ui-kit.html` | Composants avec leur code |
| `patterns.html` | Assemblages : feuille de verdict, lecture d'une bande, étape bloquante |
| `references/` | Voir `references/README.md` |

## Règles

- Aucune couleur, aucun corps de texte, aucun espacement hors de `tokens.css`. Pas de `!important`, pas de style en ligne sauf une valeur dynamique passée par une propriété personnalisée (`--w`, `--c`, `--ref`, `--mes`, `--n`).
- Un composant absent de `ui-kit.html` n'existe pas : on l'ajoute d'abord à la page, puis à `components.css`.
- Un seul bouton principal par écran. Le verdict est un mot en très grand corps doublé d'un carré de couleur : la couleur n'est jamais seule. Le rouge est réservé au danger et au hors tolérance.
- Aucune ombre, aucun dégradé, aucun arrondi, aucun emoji.
- Texte en français : espaces fines insécables avant `: ; ! ?`, guillemets français, apostrophe courbe, virgule décimale, chiffres tabulaires dans les tableaux.
- Les valeurs de contraste de `brand-book.html` sont calculées à partir de `tokens.css` par le script de génération des pages ; les modifier dans `tokens.css` impose de régénérer la page.

## Fonte

Inter (variable, axe de corps optique), chargée depuis Google Fonts dans les pages de documentation. L'application embarquera la fonte localement (pas d'appel réseau, ADR 0003).
