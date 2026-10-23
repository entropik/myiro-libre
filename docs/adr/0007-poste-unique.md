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

## Cadre livré (22 octobre 2026, ticket #68)

- **Système graphique** : `design-system/` passe au thème bleu nuit du prototype (sombre par défaut, bascule vers le clair), cartes, accent bleu, verdicts couleur + icône + mot. Ni ombre, ni dégradé ; angles droits, sauf la pastille de l'instrument.
- **Cadre** : barre du haut avec cinq tâches dans cet ordre (Bibliothèque, Profiler, Mesurer, Contrôler, Imprimer) et la pastille d'état de l'instrument ; trois zones (bibliothèque 270 px, centre, détails 300 px), seules les zones défilent. L'application s'ouvre sur Bibliothèque. Profiler, Contrôler et Imprimer montrent une carte « Prévu » qui dit ce qu'elles feront et quand.
- **Bibliothèque en arbre** imprimante → papier → résolution, avec l'âge du profil actif (signalé au-delà d'un an), la recherche, l'ajout en ligne et, en bas, les conditions d'avant le parc sous « Non classées » avec leurs mesures.
- **Vue d'une résolution** : profil actif, « Installer dans Windows » (seul bouton principal), exporter, ranger un profil ICC existant, historique, mesures ; à droite, la provenance du profil, où ce qui n'est pas connu (instrument, mire, fiche d'impression d'un profil rangé) est écrit « inconnu ».
- **Installation dans Windows** par le trait `InstallationProfils` (module `app/src/installation.rs`), sur le modèle du pare-feu (ADR 0005) : l'adapter réel passe par `mscms` (`GetColorDirectoryW`, `InstallColorProfileW`), les tests par un dossier simulé. Un autre profil du même nom, au contenu différent, n'est jamais écrasé ; s'il est identique, rien ne change. Si Windows refuse (droits d'administrateur), le message propose d'exporter le profil et de l'installer depuis l'Explorateur.
- Les nouvelles commandes suivent le modèle actuel (`#[tauri::command]` enregistrées dans `lib.rs`) ; le registre des actions (#58, #61) les reprendra.
