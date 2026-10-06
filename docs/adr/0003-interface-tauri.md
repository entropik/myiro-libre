# L'interface est faite avec Tauri 2

L'application de bureau utilise Tauri 2 : cœur Rust, interface web rendue par WebView2 (présent sur Windows 11). Courbes spectrales, tableaux d'écarts, diagrammes de gamut et historiques de contrôle d'impression sont bien mieux servis par l'écosystème web que par les cadres natifs Rust.

## Options écartées

- egui : tout en Rust, mais rendu visuel brut pour une application destinée aux imprimeurs.
- Slint : natif et soigné, mais écosystème de graphiques trop pauvre.

## Conséquences

- L'interface est bilingue français/anglais, avec catalogue de textes dès le départ.
- Le cœur (colorimétrie, mires, bibliothèque, profilage) ne dépend pas de Tauri.
