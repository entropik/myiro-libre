# Le pont SDK a deux seams, une liste blanche par le type et une provenance obligatoire

Les DLL du fabricant (`FDXSDK`, `FD9SDK`) ne sont chargées que dans des exécutables ponts séparés (`pont-myiro1`, `pont-fd9`, éventuellement `pont-i1d3`). L'application ne voit jamais la DLL : elle utilise un module `instrument` qui cache le pont, ses paliers et ses échecs matériels. Quatre conceptions concurrentes ont été comparées (interface minimale, flexible, cas courant, ports et adapters) ; la décision retient leur combinaison.

## Décision

- **Deux seams.** (a) Entre l'application et le processus pont : un trait `Pont` commun, avec `PontProcessus` en production et `PontSimule` en test. (b) À l'intérieur du pont, entre la logique et le FFI : un trait par SDK (`SdkMyiro1`, `SdkFd9`, plus tard `SdkI1d3`), sans trait commun, les ABI n'ayant rien en commun. Chaque trait a un adapter réel et un adapter simulé à échecs injectables.
- **Module `instrument` côté application.** Il expose `ouvrir` puis des mesures (`mesurer_ponctuelle`, plus tard `mesurer_tirage`). L'ordre des paliers, la reconnexion et le redémarrage du pont lui appartiennent. Les gestes humains (poser sur le blanc, passer la bande) passent par un trait dédié.
- **Liste blanche par le type.** Les traits `Sdk*` n'ont aucune méthode `JIG_*` ni `Set*` d'écriture, et la table d'exports de `fdx-sys` aux champs figés est la seule à être résolue. Un test énumère les exports de la DLL pour vérifier qu'aucun export interdit n'est nommé. `SetMeasureCondition` reste exclue tant que son contrat n'est pas établi.
- **Paliers plafonnés au lancement** du pont. L'accord humain entre deux étapes est un paramètre vérifiable, et chaque pont déclare ses propres paliers : l'i1Display3, sans bande, n'est pas forcé dans le moule MYIRO-1.
- **Provenance obligatoire, posée par le pont.** Aucune mesure ne sort sans elle. La date est celle du pont, avec fuseau ; la condition archivée est celle relue sur l'instrument, et un écart avec la condition demandée est une erreur.
- **Inconnu explicite.** Chaque donnée est confirmée, supposée ou inconnue ; aucune valeur par défaut ni zéro. Le désérialiseur refuse un nombre à la place d'une donnée qualifiée.
- **M1 inexprimable sur une mesure émissive.** La condition `Emissive` est dérivée de la nature de l'instrument, jamais saisie.

## Options écartées

- **Commande brute et catalogue dynamique** (appeler un export par son nom). Ils ouvrent un chemin vers des exports arbitraires, contraire à la règle de sécurité matérielle. Un banc d'exploration éventuel serait un binaire séparé, hors build de production.
- **Filtre d'exports à l'exécution** comme seule protection : moins sûr qu'un type qui ne peut pas exprimer l'appel.
- **Un trait `Sdk` commun** à tous les instruments : interface large et creuse.
- **Un point d'entrée par étape exposé à l'application** : l'ordre des paliers serait à la charge de l'appelant.

## Conséquences

- Plusieurs champs de provenance resteront inconnus tant que l'ABI n'est pas confirmée (`DeviceInfo`, illuminant, observateur, mapping M0/M1/M2) : l'archivage est complet en forme avant de l'être en contenu, et chaque champ passe à confirmé quand sa fiche `docs/abi/` l'est.
- Les adapters simulés valident la logique, pas l'ABI : seul l'instrument tranche, et un simulé ne doit pas encoder un comportement supposé comme acquis.
- Le FD-9 impose une connexion par famille (réseau, licence, job) : un paramètre de connexion propre à chaque SDK est à prévoir avant la phase 2.
- Une mesure émissive (i1Display3) donne luminance et chromaticité plutôt qu'un spectre réflectif : la mesure devra admettre une variante tristimulus, et l'étalonnage pouvoir être non applicable. L'i1Display3 n'est engagé qu'après les paliers 0 à 3 du MYIRO-1.
- La condition de session (état avant recalibration, chauffe) relève de l'application, pas du pont.
- À relire contre `Archivage/PROTOCOLE.txt` (branche `codex`) avant de figer les types de provenance : les compléments sont fondés sur un résumé de ce protocole.
- Le seam (a) et la provenance sont indépendants de Windows ; seuls les ponts y sont liés (ADR 0004 : les crates partagées avec le futur RIP ne dépendent pas des ponts).
