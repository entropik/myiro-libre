# ArgyllCMS est inclus dans l'installateur et appelé en processus externe

Les profils ICC et les linéarisations sont calculés par ArgyllCMS (`colprof`, `printcal`, `targen`, `profcheck`), appelé en processus externe, dans une version figée livrée avec l'application. Une application dite autonome ne peut pas demander à un imprimeur d'installer un outil en ligne de commande, et écrire un moteur de profilage fiable est hors de portée de la v1.

## Options écartées

- Demander à l'utilisateur d'installer ArgyllCMS : contraire à l'objectif d'autonomie.
- Moteur maison sur LittleCMS : envisageable à long terme, pas pour la v1.

## Conséquences

- ArgyllCMS est sous AGPL-3.0, compatible avec la GPL-3.0 du projet ; chaque version publiée doit fournir (ou pointer précisément) les sources de la version d'ArgyllCMS incluse.
- Une montée de version d'ArgyllCMS est une décision explicite, testée sur des jeux de mesures de référence.
