# Publication GitHub avec reprise

Consulter ce document avant de publier une spécification, un lot de tickets ou
leurs dépendances, et lorsqu'une écriture GitHub échoue. La validation utilisateur
déjà obtenue reste valable ; les autorisations techniques du poste restent celles
de l'environnement.

## Préparer puis publier

1. Conserver le texte approuvé, le manifeste et l'état dans un dossier durable
   ignoré par git, par exemple `.local/publications/<nom>/` dans le worktree
   autorisé. Les fichiers temporaires servent seulement aux échanges techniques.
2. Valider le manifeste avec `python outils/publier_tickets.py <manifeste>`.
   La commande ne contacte pas GitHub sans `--apply`.
3. Après autorisation de publication, utiliser `--apply`. Ajouter `--native` pour
   les relations GitHub : elles sont posées seulement après vérification des
   dépendances textuelles dans tous les tickets. Les fichiers JSON passent sur
   l'entrée standard de `gh`, avec arguments séparés et sans shell.
4. La publication ne réussit qu'après relecture du corps, des dépendances et de
   l'étiquette. Les numéros sont enregistrés avant de poursuivre. Une réponse
   d'interface optimiste ou une sortie de commande seule n'est pas une preuve.

Un manifeste JSON contient `repo`, un `id` stable, un éventuel `parent`, puis
`tickets`. Chaque ticket porte `key`, `title`, `body` (fichier relatif), et
`blocked_by` (clés des autres tickets du lot ou numéros externes). Pour reprendre
des tickets déjà créés, ajouter leur `number` après vérification. Exemple :

```json
{
  "repo": "owner/repo",
  "id": "session-instrument",
  "parent": 21,
  "tickets": [
    {"key": "etat", "title": "État fiable", "body": "01.md"},
    {"key": "fermeture", "title": "Fermeture vérifiée", "body": "02.md", "blocked_by": ["etat", 3]}
  ]
}
```

L'état `etat.json` et le verrou empêchent les créations répétées et les exécutions
concurrentes. Après une création incertaine, la reprise recherche le marqueur
unique directement dans les tickets ; sans correspondance unique, elle exige une
réconciliation et n'en crée pas un autre. Un verrou restant après interruption
se retire seulement après vérification que le processus n'existe plus.

Pour remplacer explicitement le texte d'un ticket existant, fournir `number`,
`replace_body: true` et `expected_body_sha256` (empreinte UTF-8 du corps relu).
Le fichier `body` devient la version attendue. Si le texte distant a changé,
la publication s'arrête pour réconciliation ; une reprise sur le texte déjà
conforme ne le réécrit pas. Sans cette option, le corps distant est préservé.

Pour retirer une ancienne dépendance native, indiquer son numéro dans
`remove_blocked_by` et l'omettre de `blocked_by`, puis utiliser `--apply --native`.
Seuls les retraits explicitement listés sont autorisés ; leur état est enregistré
et leur disparition relue. Décrire et vérifier l'ordre des changements lorsque
des dépendances sont inversées, pour ne pas créer de cycle intermédiaire.

## Échec : arrêter les écritures, conserver les preuves

- **Syntaxe locale** : corriger la construction de la commande, avant tout accès
  réseau. Utiliser l'outil plutôt qu'un assemblage de texte PowerShell.
- **Réseau ou bac à sable** : demander l'exécution autorisée conformément à
  l'environnement. Réutiliser une autorisation applicable ; ne pas changer les
  protections, les identifiants ou `safe.directory` global pour dépanner.
- **Droits (401/403)** : une lecture ciblée des droits, puis bilan précis.
- **Serveur (5xx), délai ou réponse vide** : conserver statut HTTP et identifiant
  de requête, puis effectuer au plus une relecture du résultat attendu. Arrêter
  les mutations du lot à ce stade. Une nouvelle voie (REST, GraphQL, navigateur,
  commentaire) n'est pas un nouvel essai indépendant : elle appartient au même
  incident. Une reprise ultérieure exige un signal nouveau ou une demande de
  l'utilisateur, puis commence par une réconciliation en lecture seule.

Le helper n'effectue aucune nouvelle tentative automatique. Son état persistant
permet une reprise explicite. Les journaux stockent les statuts et identifiants
utiles, jamais les jetons ni les en-têtes d'authentification.

## Communication

Préparer un lot concret avant toute demande d'autorisation technique. Donner une
mise à jour lorsque le diagnostic change, plutôt que raconter chaque commande.
Rapporter séparément les éléments sauvegardés, ceux restés locaux et le blocage
observé. Une erreur HTTP 500 atteste une erreur du serveur ; elle ne révèle pas
sa cause interne et ne prouve pas une panne générale de GitHub.

## Vérification

`python -m unittest discover -s outils -p "test_*.py"` couvre la publication et le
contrôle du contexte sans réseau. Le workflow `publication.yml` exécute ces
contrôles sur Windows et Linux. Les tests Rust et documentaires existants restent
dans leur workflow ; ce contrôle ne les remplace pas.
