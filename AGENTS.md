# AGENTS.md

Consignes communes à Claude Code et Codex. Lire aussi `CLAUDE.md` pour le contexte
et la sécurité matérielle, et `PLAN-ACTION.md` pour le plan.

## Isolement des agents

- Le dossier de travail courant est `C:\Code\MYIRO-fork`, sur la branche `dev`.
  Claude Code et Codex y travaillent directement, **un seul agent à la fois**.
  Avant toute écriture, vérifier le dossier, la branche et `git status --short` ;
  conserver les changements préexistants et se coordonner en cas de recouvrement.
- `dev` est la branche publique de développement et d'intégration. `main` reçoit
  les versions validées par PR `dev` → `main` ; ne pas changer de branche dans le
  dossier principal pour préparer une livraison.
- Pour un chantier parallèle ou une PR demandée, créer un worktree séparé et une
  branche `ticket/*` depuis `dev`, puis intégrer vers `dev`. Aucun second agent
  ne modifie simultanément le même dossier de travail.
- L'ancien worktree `C:\Code\MYIRO-fork-codex` reste une archive locale, sous
  `archive/codex-initial-2026-10-09`. `prive`, l'ancienne branche `codex` et
  `archive/*` contiennent de la rétro-analyse privée : ne jamais les pousser ni
  fusionner leur historique dans une branche publique. Reprendre seulement les
  fichiers publics utiles dans un commit propre issu de `dev`.
- Le hook `pre-push` autorise `main`, `dev` et `ticket/*` et refuse l'historique
  privé. Sa source versionnée est `.githooks/pre-push` ; conserver les autres
  hooks documentaires lors de son installation.
- Ne pas utiliser `git reset --hard`, `git push --force`, `git clean -f`, ni
  réécrire l'historique partagé. Préserver les archives et worktrees existants.

## Contrôle avant travail

Dans le dossier principal :

```text
python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev
```

Dans un worktree de chantier, fournir son chemin et sa branche `ticket/*`, avec
`--reference dev`. Une divergence des règles impose leur comparaison ; elle
n'autorise ni fusion automatique ni changement de branche dans le dossier principal.

## Publications GitHub

Avant de publier ou reprendre une spécification, un lot de tickets ou leurs
relations, lire [la procédure](docs/agents/publication-github.md). Utiliser le
helper, son état durable sous `.local/publications/`, et annoncer la réussite
après relecture des résultats. Ces archives locales restent ignorées par Git.

## Archives et contenu public

- `Audit-MYIRO/collecte/`, `Audit-ColorGPS/collecte/` et `SDK/` restent dans
  `C:\Code\MYIRO-fork`, en lecture seule pour les agents : aucun installateur exécuté.
- Public : code Rust, documentation rédigée, plans, scripts d'audit et synthèse
  `Audit-MYIRO/LIRE-MOI-AUDIT.txt`.
- Local uniquement : `Audit-ColorGPS/`, `Audit-MYIRO/analyse/`,
  `Audit-MYIRO/retroanalyse/`, DLL, manuels et données propriétaires. Aucun numéro
  de série, adresse MAC ou IP d'instrument dans le dépôt public.
- Les désassemblages `**/code.asm.txt` et ressources `*.resources/` restent
  ignorés ; ils se régénèrent avec les outils d'audit.

## Coordination et langue

Les dossiers ne sont plus réservés à un agent nommé. Le chantier demandé fixe
le périmètre ; avant de toucher un fichier déjà modifié, identifier le travail
en cours et préserver ce qui ne relève pas du chantier.

Documentation, rapports et messages de commit en français.
