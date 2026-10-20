# Une seule application pour la couleur et l'impression

Le 20 octobre 2026, le mainteneur a fixé le but du projet : **un seul poste**, libre, qui mesure, profile, range les profils et imprime pour les machines jet d'encre de l'atelier ([conception du poste unique](../specifications/2026-10-20-poste-unique-design.md)). L'imprimerie travaille aujourd'hui avec plusieurs logiciels qui ne se parlent pas, et les profils se rangent « n'importe où ».

Cet ADR remplace l'[ADR 0004](0004-crates-partagees-rip.md) sur un point : le RIP n'est plus un projet séparé. Il devient une partie de la même application.

## Décision

- **Une fenêtre pour l'utilisateur** : une bibliothèque, une file d'impression, les mêmes instruments.
- **Plusieurs moteurs à l'intérieur** : un cœur commun (calculs couleur, profils, formats, bibliothèque) et des modules séparés, un par instrument (les ponts `pont-myiro1` et `pont-fd9`) et un par famille de machines, ajoutés étape par étape.
- **Le cœur reste indépendant** de Tauri, de Windows et des ponts : les crates `colorimetrie`, `cgats`, `bibliotheque` et les formats d'échange gardent leur API publique comme un contrat, comme le demandait l'ADR 0004. Elles restent vérifiées aussi sur le poste Linux de l'intégration continue quand elles y figurent.
- **La bibliothèque range ce que l'imprimeur cherche** : imprimante, puis papier, puis résolution d'impression. Chaque résolution est une condition d'impression qui garde son profil actif et l'historique des précédents (ticket #66).

## Conséquences

- Le contrat de la crate `colorimetrie` (ADR 0004) reste valable tel quel.
- La bibliothèque passe à l'organisation 5 (imprimantes, papiers, résolutions, profils). Une base existante garde toutes ses données ; ses conditions d'impression d'avant apparaissent « non classées » jusqu'à ce qu'on les range.
- Un profil n'est accepté que s'il est un fichier ICC RVB ou CMJN : RVB pour les imprimantes pilotées par leur pilote Windows, CMJN pour la HP Latex.
- L'interface est refaite à partir du prototype validé ; les sorties vers chaque famille de machines sont étudiées une par une (ordre de livraison dans la conception).
