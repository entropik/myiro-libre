---
description: Mettre à jour la documentation du projet en fin de journée (journal, glossaire, ADR, vérifications)
---

Fais la mise à jour de fin de journée de la documentation de myiro-libre. Réponds en français, avec des mots simples : l'utilisateur est imprimeur, pas développeur.

1. **Voir ce qui a changé aujourd'hui** : `git log --since=midnight --stat` et `git status`. Ne te fie pas à ta seule mémoire de la conversation.
2. **Journal** : crée ou complète `docs/blog/AAAA-MM-JJ.md` (date du jour). Sections : ce qui a été fait, décisions, questions ouvertes, prochaines étapes. Court et factuel.
3. **Décisions** : si une décision structurante a été prise, vérifie qu'elle est dans un ADR de `docs/adr/` (nouvel ADR numéroté ou complément). Ne modifie pas un ADR ancien sans le dire.
4. **Vocabulaire** : relis `GLOSSARY.md` ; propose les mots nouveaux ou ambigus apparus dans la journée (sans les ajouter sans accord).
5. **Cohérence** : si `PLAN-ACTION.md`, `README.md` ou `CLAUDE.md` sont devenus faux, signale-le et propose la correction.
6. **Vérifier** : lance `python outils/verifier_docs.py`. Corrige tout problème bloquant.
7. **Proposer le commit** : résume les fichiers de documentation modifiés et demande à l'utilisateur s'il veut un commit et un push. N'engage rien sans son accord ; si un push est demandé, ne publie que les commits de documentation (voir la mémoire sur le dépôt public).

Règle absolue : aucune mention du logiciel concurrent dont le manuel a été analysé dans un fichier versionné. Écris « un logiciel concurrent ». Le script de l'étape 6 le contrôle.
