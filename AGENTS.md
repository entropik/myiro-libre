# AGENTS.md

Consignes communes aux agents (Claude Code, Codex) qui travaillent sur ce projet. Lire aussi `CLAUDE.md` (contexte, règles de sécurité matérielle) et `PLAN-ACTION.md` (plan).

## Isolement des agents

Un seul dépôt git, un dossier de travail et une branche par agent :

| Agent | Dossier de travail | Branche |
|---|---|---|
| Claude Code | `C:\Code\MYIRO-fork` | `main` |
| Codex | `C:\Code\MYIRO-fork-codex` (worktree git) | `codex` |

- Chaque agent ne modifie que son propre dossier de travail.
- `main` est la seule branche publiée (dépôt public `entropik/myiro-libre`). Son historique ne contient aucun travail de rétro-analyse.
- `prive` (historique complet d'origine) et `codex` contiennent les dossiers privés : **ne jamais les pousser**, ne jamais les fusionner dans `main` (`git merge codex` ferait entrer l'historique privé). Un hook `pre-push` refuse toute autre branche que `main`.
- Pour faire entrer un livrable public de Codex (fiches d'ABI rédigées dans `docs/abi/`, etc.) dans `main` : `git checkout codex -- <chemins publics>` puis commit, chemin par chemin.
- Ne jamais faire `git checkout prive` ou `git checkout codex` dans `C:\Code\MYIRO-fork` : quitter ensuite ces branches supprimerait du disque les dossiers privés.
- Ne jamais faire `git reset --hard`, `git push --force`, `git clean -f` ni réécrire l'historique d'une branche partagée.

## Archive commune hors git

`Audit-MYIRO/collecte/`, `Audit-ColorGPS/collecte/` et `SDK/` ne sont pas versionnés (binaires et manuels sous licence Konica Minolta / Ergosoft). Ils n'existent que dans `C:\Code\MYIRO-fork`. Les deux agents les lisent **en lecture seule** à ce chemin absolu ; personne ne les modifie ni n'exécute les installateurs.

Les désassemblages `**/code.asm.txt` et les ressources extraites `*.resources/` ne sont pas versionnés non plus : ils se régénèrent avec les scripts de `outils/`.

## Ce qui est public, ce qui reste local

Public (branche `main`) : plan, consignes, scripts d'audit `Audit-MYIRO/outils/*.py`, synthèse `Audit-MYIRO/LIRE-MOI-AUDIT.txt`, documentation rédigée (`docs/`), code Rust.

Local uniquement : `Audit-ColorGPS/` (Ergosoft, logiciel commercial, et fichiers d'environnement d'impression), `Audit-MYIRO/analyse/` et `Audit-MYIRO/retroanalyse/` (chaînes et désassemblages tirés des DLL Konica Minolta). Ne publier aucun numéro de série, adresse MAC ou IP d'instrument.

## Répartition des dossiers

| Dossier | Responsable par défaut |
|---|---|
| `Audit-ColorGPS/` | Codex |
| `Audit-MYIRO/retroanalyse/`, `Audit-MYIRO/outils/desassemble.py` | Codex |
| `crates/`, `app/`, `docs/`, `Cargo.toml` (application Rust) | Claude Code |
| `PLAN-ACTION.md`, `CLAUDE.md`, `AGENTS.md` | Claude Code ; Codex propose ses changements dans un commit séparé sur `codex` |

Toucher au dossier d'un autre agent seulement par un commit sur sa propre branche, signalé dans le message de commit.

## Langue

Documentation, rapports et messages de commit en français.
