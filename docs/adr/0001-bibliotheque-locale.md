# Les mesures vivent dans une bibliothèque locale, pas dans des fichiers

Les mesures, conditions d'impression, références, profils et linéarisations sont conservés dans une bibliothèque locale unique (une base sur le poste), et non comme des fichiers CGATS épars dans des dossiers. Le contrôle d'impression exige un suivi dans le temps et des liens entre condition d'impression, tirages, mesures et profils, que des fichiers isolés ne portent pas.

## Conséquences

- Pour ne jamais enfermer les données : export CGATS.17 de toute mesure, import CGATS, et sauvegarde/restauration complète de la bibliothèque.
- Chaque mesure conserve tous les spectres fournis par l'instrument (toutes les conditions de mesure disponibles) ; le choix M0/M1/M2 se fait au calcul, pas à la mesure.
- Un poste, un utilisateur en v1 : pas de bibliothèque partagée sur le réseau.
