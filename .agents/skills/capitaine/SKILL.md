---
name: capitaine
description: Mener un ticket MYIRO avec $capitaine #N, ou les tickets prêts avec $capitaine, jusqu'aux PR fusionnées vers dev, avec des ouvriers TDD en worktrees isolés.
---

# Capitaine pour Codex

Le capitaine organise, les ouvriers implémentent, deux relecteurs spécialisés vérifient.
Une demande d'installation, d'explication ou de modification du skill ne lance
aucun ticket. Une mission explicitement lancée couvre code, tests, PR et fusion
vers `dev`, dans les limites demandées par l'utilisateur et des permissions.
Les essais matériels et les validations humaines restent soumis à son accord.

Parler français, avec des mots simples. Nommer les tickets par leur titre puis
leur numéro. Annoncer « Capitaine — titre (#N) » ou « Capitaine — tickets prêts »
au démarrage ; ne pas promettre de renommer automatiquement la session.

## Préparer la mission

1. Lire `AGENTS.md`, `CLAUDE.md` et `PLAN-ACTION.md` dans
   `C:/Code/MYIRO-fork`. Vérifier dossier, branche `dev`, état Git et exécuter :
   `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev`.
   Préserver tout travail préexistant. Ne pas changer de branche dans ce dossier.
2. Lire `docs/agents/issue-tracker.md` et `docs/agents/triage-labels.md`.
   Avant toute écriture GitHub, lire `docs/agents/publication-github.md`.
   Les modifications de spécifications, corps de tickets ou relations suivent
   le helper et son état durable ; une mission ne permet pas de les réécrire librement.
3. Vérifier les outils de collaboration disponibles et les agents déjà présents.
   Ce skill demande explicitement la délégation pendant une mission, pas pendant
   son installation. Sans outil de sous-agents, expliquer la limite et demander
   si l'utilisateur veut une exécution séquentielle par un seul agent.

## Choisir et reprendre

- `$capitaine #N` : traiter seulement ce ticket. Lire corps, commentaires,
  étiquettes, assignés, PR existantes et dépendances. Un bloqueur ouvert ou
  `ready-for-human` suspend ce ticket, avec la raison et l'action attendue.
- `$capitaine` : traiter les tickets ouverts `ready-for-agent`, sans assigné
  ni bloqueur ouvert, dans le dépôt MYIRO. Respecter un périmètre de parent ou
  de module si l'utilisateur l'a donné. Recalculer les tickets prêts après
  chaque fusion ; des travaux dépendants ne démarrent pas avant leur déblocage.
- Vérifier les dépendances natives et les mentions textuelles de bloqueurs.
  Une dépendance illisible ou un désaccord entre sources n'est pas un feu vert :
  clarifier avant de prendre le ticket.
- Avant réservation, relire assignation, commentaires de prise en charge,
  `git worktree list --porcelain`, branches et PR. Un assigné identique au compte
  courant ne prouve pas que le chantier appartient à cette session. Ne pas
  reprendre le travail d'un autre agent sans coordination.
- Pour une reprise autorisée, garder branche, commits et worktree existants.
  Une PR ouverte reprend à la relecture ; une branche sans PR reprend avec un
  ouvrier. Ne pas créer une seconde PR pour la même branche.

Conserver un état local sous `.local/capitaine/` : ticket, propriétaire de session,
chemin du worktree, branche, SHA de base, agent, dernier SHA relu/testé, PR et
validations humaines. Relire Git et GitHub à chaque reprise : cet état est un
aide-mémoire, pas une preuve distante. Ne jamais y copier de secrets.

## Exécuter un ticket

### 1. Réserver et cadrer

Après vérification de disponibilité, assigner le ticket au compte courant avec
`gh issue edit N --add-assignee @me`, puis commenter la prise en charge par le
capitaine Codex et le chantier choisi. Relire pour confirmer la réservation.
En cas d'erreur distante ambiguë, réconcilier une fois avant de réessayer ; si
l'état reste ambigu, arrêter les écritures pour ce ticket et signaler le problème.

Lire `GLOSSARY.md`, les ADR cités et le code concerné. Fixer les interfaces sous
test (« seams ») avec les critères du ticket. Pour une interface nouvelle,
utiliser `codebase-design`. Un choix qui modifie le comportement attendu ou le
périmètre revient à l'utilisateur ; les choix internes restent au capitaine.

### 2. Isoler puis déléguer

Pour un nouveau chantier, actualiser `origin/dev` et enregistrer son SHA comme
base fixe. Créer une branche `ticket/N-slug` dans un nouveau worktree explicitement
nommé, par exemple `C:/Code/MYIRO-ticket-N`, depuis `origin/dev`. Vérifier d'abord
que ni le chemin ni la branche ne sont déjà utilisés. Contrôler le contexte de
ce worktree avec sa branche et `--reference dev` ; traiter toute divergence.
Ne jamais prendre une archive privée comme base. Si ce chemin dépasse les droits
d'écriture de la session, demander l'escalade prévue par l'outil avant sa création.

Lire [la feuille de mission](ouvrier.md) avant la première délégation.
Utiliser `collaboration.spawn_agent` avec `task_name` et un message contenant :
chemin absolu du worktree, branche, SHA de base, ticket, interfaces convenues et
chemin absolu de cette feuille. Garder le modèle hérité. Les sous-agents partagent
le système de fichiers : l'outil ne crée pas de worktree et ne change pas leur
dossier courant. Exiger `workdir` explicite pour les commandes et des chemins
absolus pour les modifications. Une seule personne écrit dans chaque worktree.

Avec quatre places disponibles, garder au plus un ouvrier et réserver deux places
aux relecteurs du skill `code-review`, le capitaine occupant la quatrième.
Compter aussi les agents déjà présents avant tout lancement. Si les places
manquent, coordonner leur disponibilité ; ne pas annoncer une revue indépendante
qui n'a pas pu être exécutée. Avec davantage de places, des ouvriers supplémentaires
peuvent traiter les tickets indépendants, chacun dans son worktree. Réutiliser
uniquement les agents inactifs de cette mission via `collaboration.followup_task`
après une passation explicite de leur nouveau périmètre ; ne pas supposer qu'un
agent terminé libère une place.
Utiliser `collaboration.send_message` pour un agent actif, `followup_task` pour
reprendre un agent inactif, et les états/messages pour vérifier les passations.
Les ouvriers ne délèguent pas à leur tour.

### 3. Relire et corriger

Quand l'ouvrier a terminé et ne modifie plus le worktree, enregistrer son SHA.
Le capitaine lit et applique lui-même `code-review` contre le SHA de base enregistré,
avec le ticket comme spécification et les preuves des tests. Il confie directement
les deux axes du skill aux deux agents réservés : standards et spécification.
Il n'ajoute pas de relecteur coordinateur intermédiaire. Les relecteurs travaillent
en lecture seule, ne délèguent pas et n'ont pas implémenté le ticket examiné.
Le capitaine agrège leurs rapports séparément selon le skill, en distinguant
constatations fermes et préférences. Les contrôles sans instrument peuvent être
relancés dans le worktree, sans exécutions concurrentes qui se gênent.

Une constatation ferme repart à l'ouvrier pour correction en TDD, puis à la
relecture. Un avis de goût est noté sans boucle de correction obligatoire. Après
deux tours infructueux, expliciter la cause et l'arbitrage ; un défaut réel ou un
critère manquant reste bloquant, quel que soit le nombre de tours.

Le capitaine tient le journal du chantier dans le même worktree, seulement quand
l'ouvrier est inactif. Toute modification après relecture invalide les contrôles
concernés : faire revoir et retester le nouveau SHA avant publication/fusion.

### 4. Obtenir les validations humaines

Pour un écran nécessitant une validation humaine, proposer le lancement depuis
le worktree et les critères à regarder ; attendre une réponse effective avant
de les déclarer validés. Pour un appareil réel, annoncer chaque palier, son effet
et le geste attendu ; n'exécuter ce palier qu'après accord explicite. Les ouvriers
restent sur les simulateurs. Les règles matérielles de `CLAUDE.md` s'appliquent.

Consigner les résultats sans identifiants matériels réels et les rattacher au
SHA testé. Un changement du comportement testé demande une nouvelle validation.
Les autres tickets indépendants peuvent continuer pendant l'attente. En l'absence
de réponse, garder le chantier en attente, sans fusion ni clôture anticipée.

### 5. Publier et surveiller

Vérifier l'état propre du chantier, son contenu public et les tests attendus.
Pousser seulement sa branche `ticket/*`, sans forcer. Utiliser `pr` pour le corps
en français : `Closes #N`, interfaces testées, résultats, limites et validations
humaines. Respecter l'attribution Codex éventuellement exigée par les hooks,
sans inventer une identité ni attribuer le travail à Claude Code.

Créer la PR avec `--base dev --head ticket/N-slug`, puis relire la PR distante
(base, tête, corps et SHA). Ne pas ouvrir de PR vers la branche de livraison.
Suivre les contrôles du SHA courant, dont `windows` et `linux` et tout contrôle
supplémentaire requis par le dépôt. Utiliser des attentes courtes pour rester
disponible aux messages. Un échec repart à l'ouvrier avec les journaux utiles.
L'absence de contrôle n'équivaut pas à une CI réussie.

### 6. Fusionner et vérifier

Juste avant fusion, vérifier de nouveau dépendances, périmètre, validations,
SHA relu et testé, CI et absence de constatation ferme. Si `dev` a avancé et exige
une mise à jour, intégrer `origin/dev` dans le worktree sans réécrire l'historique,
puis refaire relecture et contrôles pertinents. Sérialiser les fusions.

Fusionner la PR vers `dev` avec la stratégie autorisée, sans contourner les
protections ni utiliser `--admin`. Protéger la fusion par le SHA de tête validé
(`gh pr merge PR --merge --match-head-commit SHA`). Si l'outil ne permet pas une
fusion conditionnelle au SHA, suspendre la fusion et signaler la limite : une
simple relecture juste avant l'action ne protège pas contre un commit concurrent.
Un refus de permission reste un blocage à expliquer, pas à contourner.

Relire la PR : état fusionné, commit et branche cible. Vérifier la clôture du
ticket ; une PR vers `dev` peut ne pas le fermer automatiquement. Le fermer
explicitement avec un lien vers la PR seulement si tous ses critères sont remplis.

Nettoyer uniquement le worktree créé par cette mission, une fois tous ses agents
inactifs et ses processus arrêtés. Vérifier chemin absolu, contenu non suivi ou
ignoré à conserver, état propre et présence des commits dans `origin/dev`.
Utiliser `git worktree remove` sans force. Au moindre doute, conserver le dossier
et le signaler ; ne pas effacer les fichiers pour faire réussir le nettoyage.
Une branche ne se supprime qu'après ces vérifications, sans suppression forcée.

## Bilan

Distinguer tickets fusionnés et fermés, validations en attente, blocages et
nettoyages restant à faire. Donner les liens des PR et ce que l'utilisateur peut
tester. Ne pas déclarer une attente « terminée ». Aucun push de `dev`, passage
vers la branche de livraison ou publication de version n'est inclus dans la mission.
