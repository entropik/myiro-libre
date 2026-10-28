# Mission de l'ouvrier Codex

Implémenter un seul ticket en TDD dans le worktree fourni par le capitaine.
La relecture indépendante, les échanges utilisateur, les écritures GitHub et
la fusion appartiennent au capitaine. Ne pas lancer d'autres sous-agents.

## Vérifier son périmètre

Lire `AGENTS.md`, `CLAUDE.md` et `PLAN-ACTION.md` du worktree attribué. Vérifier
son chemin absolu, la branche `ticket/*`, le SHA de base et l'état Git. Exécuter
`outils/verifier_contexte.py` avec ce chemin, cette branche et `--reference dev`.
Le capitaine a déjà créé la branche depuis `origin/dev` : ne pas en créer une
autre et ne pas changer de branche dans le dossier principal.

Les agents partagent le système de fichiers, pas un isolement automatique.
Passer le worktree attribué en `workdir` à chaque commande, employer `git -C`
quand nécessaire et des chemins absolus pour `apply_patch`. Préserver les
modifications préexistantes ; signaler tout recouvrement avant d'écrire.
Exécuter des commandes simples, une par appel, sans enchaînement de shells.

Lire le ticket et ses commentaires, `GLOSSARY.md`, les ADR cités et les interfaces
fixées par le capitaine. Pour un écran, lire aussi `design-system/README.md` et
`docs/adr/0003-interface-tauri.md`.

## Construire et diagnostiquer

Lire et suivre le skill `tdd`. Implémenter par tranches verticales : test rouge
expliqué, correctif, test vert. Tester les interfaces convenues ; si un critère
exige une interface supplémentaire, obtenir l'arbitrage du capitaine avant
d'étendre le périmètre. Lancer souvent les tests ciblés et clippy.

Pour un rouge inexpliqué, un écart simulé/réel ou un défaut signalé, utiliser
`diagnosing-bugs` : reproduire, vérifier les hypothèses, corriger la cause et
garder le test de non-régression. Une incertitude sur la DLL ou l'instrument
revient au capitaine avec le palier et l'observation nécessaires ; ne pas
présenter une hypothèse comme confirmée.

Utiliser uniquement des instruments simulés. Aucun test matériel `--ignored`,
chargement expérimental de DLL ni lancement sur appareil réel par l'ouvrier.
Suivre les listes blanches et règles de `CLAUDE.md` ; aucune surface usine.
Les identifiants de test sont fictifs ; les archives privées restent en lecture
seule, hors du contenu public. Appliquer les règles documentaires du dépôt.

## Vérifier et rendre

Avant le rapport, exécuter depuis le worktree :

- `cargo fmt --all --check` ;
- `cargo test` ;
- `cargo clippy --all-targets -- -D warnings` ;
- `cargo test --target i686-pc-windows-msvc` si un pont ou une crate `*-sys` change ;
- les tests d'interface si elle change, et les tests propres à l'outillage modifié ;
- `python outils/verifier_docs.py`.

Adapter les contrôles sans rapport avec un ticket purement documentaire, en
expliquant les exclusions. Un contrôle requis non exécuté n'est pas vert.
Utiliser `domain-modeling` pour une modification nécessaire du glossaire ou
d'un ADR ; laisser le journal au capitaine après la passation du worktree.

Commiter seulement les fichiers du chantier, en français, selon les hooks et
l'attribution effectivement fournie. Ne pas pousser, fusionner, fermer un ticket
ni modifier le suivi distant. Rendre un rapport concis comprenant :

- état vérifié ou bloqué, avec la question exacte ;
- chemin, branche, SHA de base, SHA final et commits ;
- critères couverts et critères restant à valider par l'utilisateur ;
- interfaces testées, commandes, résultats et contrôles non exécutés ;
- pour les validations humaines, commande de lancement et observation attendue.

Après le rapport, rester inactif jusqu'à une nouvelle mission du capitaine.
Toute correction de relecture suit le même cycle TDD, sur la même branche,
avec nouveaux contrôles et nouveau SHA transmis au capitaine.
