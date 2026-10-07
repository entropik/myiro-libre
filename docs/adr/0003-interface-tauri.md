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

La maquette n'est pas du code de production : l'interface est réécrite dans l'application. Les valeurs qu'elle affiche (seuils, durées de lecture, réglages de profil) sont des hypothèses à établir.
