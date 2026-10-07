# Les mesures vivent dans une bibliothèque locale, pas dans des fichiers

Les mesures, conditions d'impression, références, profils et linéarisations sont conservés dans une bibliothèque locale unique (une base sur le poste), et non comme des fichiers CGATS épars dans des dossiers. Le contrôle d'impression exige un suivi dans le temps et des liens entre condition d'impression, tirages, mesures et profils, que des fichiers isolés ne portent pas.

## Conséquences

- Pour ne jamais enfermer les données : export CGATS.17 de toute mesure, import CGATS, et sauvegarde/restauration complète de la bibliothèque.
- Chaque mesure conserve tous les spectres fournis par l'instrument (toutes les conditions de mesure disponibles) ; le choix M0/M1/M2 se fait au calcul, pas à la mesure.
- Un poste, un utilisateur en v1 : pas de bibliothèque partagée sur le réseau.

## Complément du 7 octobre 2026 : première bibliothèque (ticket #6)

- La crate `crates/bibliotheque` porte la bibliothèque. Elle ne dépend ni de Tauri, ni de Windows, ni des ponts (ADR 0004) : seulement des types de mesure de `pont-protocole`. Elle est contrôlée aussi sur le poste Linux de la CI.
- La base est un fichier SQLite (`bibliotheque.sqlite`, moteur compilé avec la crate, rien à installer), dans un seul dossier par utilisateur : le dossier de données de l'application (sous Windows, `%APPDATA%\org.myiro-libre.app\bibliotheque\`), jamais dans le dépôt. Un numéro d'organisation est écrit dans la base ; une base écrite par une version plus récente est refusée plutôt que modifiée.
- Une mesure est conservée dans son format versionné (`myiro-libre/mesure/1`, [description](../formats/mesure.md)), tel que le pont l'a produit : spectres M0, M1, M2, données brutes, Lab et provenance se relisent à l'identique, et les données inconnues restent inconnues. La bibliothèque ne recalcule ni ne complète rien.
- Une mesure est rattachée à une condition d'impression (choisie par l'utilisateur) et à un instrument, reconnu par le modèle et le numéro de série de sa provenance ; le micrologiciel reste dans la provenance de chaque mesure, puisqu'il peut changer.
- Les noms de condition d'impression sont uniques (espaces de début et de fin retirés). La recherche ne tient compte ni des majuscules ni des accents ; elle porte sur le nom de la condition, l'instrument et la date de la mesure.
- Pas encore : suppression, export et import CGATS, sauvegarde et restauration, contexte de la condition d'impression (support, encres, RIP…).
