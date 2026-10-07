# Le pont SDK a deux seams, une liste blanche par le type et une provenance obligatoire

Les DLL du fabricant (`FDXSDK`, `FD9SDK`) ne sont chargées que dans des exécutables ponts séparés (`pont-myiro1`, `pont-fd9`, éventuellement `pont-i1d3`). L'application ne voit jamais la DLL : elle utilise un module `instrument` qui cache le pont, ses paliers et ses échecs matériels. Quatre conceptions concurrentes ont été comparées (interface minimale, flexible, cas courant, ports et adapters) ; la décision retient leur combinaison.

## Décision

- **Deux seams.** (a) Entre l'application et le processus pont : un trait `Pont` commun, avec `PontProcessus` en production et `PontSimule` en test. (b) À l'intérieur du pont, entre la logique et le FFI : un trait par SDK (`SdkMyiro1`, `SdkFd9`, plus tard `SdkI1d3`), sans trait commun, les ABI n'ayant rien en commun. Chaque trait a un adapter réel et un adapter simulé à échecs injectables.
- **Module `instrument` côté application.** Il expose `ouvrir` puis des mesures (`mesurer_ponctuelle`, plus tard `mesurer_tirage`). L'ordre des paliers, la reconnexion et le redémarrage du pont lui appartiennent. Les gestes humains (poser sur le blanc, passer la bande) passent par un trait dédié.
- **Liste blanche par le type.** Les traits `Sdk*` n'ont aucune méthode `JIG_*` ni `Set*` d'écriture, et la table d'exports de `fdx-sys` aux champs figés est la seule à être résolue. Un test énumère les exports de la DLL pour vérifier qu'aucun export interdit n'est nommé. `SetMeasureCondition` reste exclue tant que son contrat n'est pas établi.
- **Paliers plafonnés au lancement** du pont. L'accord humain entre deux étapes est un paramètre vérifiable, et chaque pont déclare ses propres paliers : l'i1Display3, sans bande, n'est pas forcé dans le moule MYIRO-1.
- **Provenance obligatoire, posée par le pont.** Aucune mesure ne sort sans elle. Elle porte l'instrument (modèle, n° de série, micrologiciel, transport), la chaîne logicielle (version du SDK, version et architecture du pont, empreinte SHA-256 de la DLL chargée), la date avec fuseau, la géométrie de lecture (ponctuelle, bande, feuille), la condition de mesure, l'illuminant et l'observateur. La date est celle du pont ; la condition archivée est celle relue sur l'instrument, et un écart avec la condition demandée est une erreur.
- **Inconnu explicite.** Chaque donnée est confirmée, supposée ou inconnue ; aucune valeur par défaut ni zéro. Le désérialiseur refuse un nombre à la place d'une donnée qualifiée.
- **M1 inexprimable sur une mesure émissive.** La condition `Emissive` est dérivée de la nature de l'instrument, jamais saisie.

### Complément du 7 octobre 2026

Le contrat de `FDX_SetMeasureCondition` est établi (`docs/abi/FDX_SetMeasureCondition.md`) : elle arme la mesure sans écriture persistante. Elle entre dans la liste blanche comme seule exception nommée aux `FDX_Set*`, un test l'empêchant d'en faire entrer d'autres sans fiche ni accord. La phrase ci-dessus qui l'excluait reste pour l'historique. Même jour : la provenance est en place dans le pont, conforme à cette décision, et vérifiée sur l'instrument réel.

### Complément du 7 octobre 2026 : paramètre de connexion du FD-9

Les fiches `docs/abi/FD9_*` établissent ce que `FD9_Connect` lit : la liaison (0 réseau, 1 USB), l'adresse (IP ou nom d'hôte, 23 caractères au plus, car la DLL n'en recopie que 24 octets) et un nom d'application (20 octets recopiés). Le port TCP 49152 est fixé dans la DLL, il n'y a ni délai ni clé de licence. Le paramètre de connexion du FD-9 est donc « une entrée de la détection, ou une adresse saisie », plus le nom `myiro-libre` ; en réseau, l'adresse saisie dispense de la détection. `crates/fd9-sys` en porte les formes (`Appareil::reseau`, `NomApplication`), vérifiées avant tout appel. Sa liste blanche compte six exports et aucune exception : ni `JIG_*` ni `FD9_Set*`, `FD9_TAConnect` (connexion prioritaire) exclu tant que la prise de main n'est pas comprise.

### Complément du 7 octobre 2026 : état courant de l'instrument (ticket #23)

Le pont MYIRO-1 sépare désormais trois choses : le **plafond** (fixé au lancement, jamais relevé), la **progression** des paliers (ce qui a été franchi une fois, gardé comme historique) et l'**état courant** de l'instrument (non connecté, inexploitable, connecté, étalonné, perdu). Seul l'état courant autorise un étalonnage ou une mesure.

- Un nouvel étalonnage rend l'ancien inutilisable dès son début : après un échec, un délai dépassé, un refus ou une perte de liaison, il faut réétalonner.
- La connexion n'est exploitable qu'une fois l'identité lue. Toute nouvelle connexion efface l'identité et la date d'étalonnage de la précédente ; la provenance d'une mesure les prend dans l'état relevé avant l'armement.
- Une perte de liaison (événement 6, même pendant le désarmement ou le retour au repos) bloque tout jusqu'à une nouvelle connexion, sans appel à la DLL. Une mesure déjà rendue n'est pas touchée.
- Le protocole distingue trois refus : `etalonnage_requis`, `instrument_perdu` (rendu aussi aux demandes suivantes) et `session_inexploitable`. Dans les trois cas, l'instrument n'est pas armé.

La session n'a pas encore de déconnexion volontaire : seule la perte de liaison (événement 6) mène à l'état perdu ; la fermeture relève du ticket #24.

Vérifié contre l'instrument simulé seulement ; le comportement du vrai MYIRO-1 après une perte de liaison reste à observer, de même que la réponse de la DLL à une reconnexion sans déconnexion préalable (supposée acceptée par le simulé, voir la fiche `FDX_Connect`).

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
- Le pont n'atteste que ce qu'il observe. Tout le reste de ce que le protocole d'archivage (`Archivage/PROTOCOLE.txt`) demande relève de l'application et de la bibliothèque (ADR 0001) : conditions de session (chauffe, position de sonde, état avant recalibration), support et lot, encres, RIP, linéarisation, séchage, fond de mesure, mire et ordre des plages, formule d'écart de couleur, seuils, statut (en attente, mesuré, validé, non conforme) et profil ICC utilisé.
- Vérifié contre `Archivage/PROTOCOLE.txt` : la provenance ci-dessus couvre toute la part du protocole que l'instrument et le SDK peuvent fournir. La chaîne logicielle y est ajoutée parce que le protocole n'autorise de comparer que des sessions de même chaîne logicielle.
- Le seam (a) et la provenance sont indépendants de Windows ; seuls les ponts y sont liés (ADR 0004 : les crates partagées avec le futur RIP ne dépendent pas des ponts).
