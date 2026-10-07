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
- Une perte de liaison (événement 6, même pendant le désarmement ou le retour au repos) bloque tout jusqu'à une nouvelle connexion : toute nouvelle demande est refusée sans appel à la DLL, mais la demande en cours tente encore un désarmement (`FDX_StopMeasurement`, export déjà autorisé) ; la politique de désarmement relève du ticket #24. Seul l'événement 6 mène à l'état perdu : un délai dépassé ou une erreur de la DLL pendant la lecture ne l'invalident pas (supposé, à observer sur l'instrument). Une mesure déjà rendue n'est pas touchée.
- Le protocole distingue trois refus : `etalonnage_requis`, `instrument_perdu` (rendu aussi aux demandes suivantes) et `session_inexploitable`. Dans les trois cas, l'instrument n'est pas armé.

La session n'a pas encore de déconnexion volontaire : seule la perte de liaison (événement 6) mène à l'état perdu ; la fermeture relève du ticket #24 (complément plus bas).

Vérifié contre l'instrument simulé seulement ; le comportement du vrai MYIRO-1 après une perte de liaison reste à observer, de même que la réponse de la DLL à une reconnexion sans déconnexion préalable (supposée acceptée par le simulé, voir la fiche `FDX_Connect`).

### Complément du 7 octobre 2026 : mesures validées et format versionné (ticket #25)

- Les mesures circulent et se conservent dans un seul format, `myiro-libre/mesure/1` ([`docs/formats/mesure.md`](../formats/mesure.md)). Spectres, données brutes, Lab, plages et mesures sont des types de `pont-protocole` qui se vérifient à la construction comme à la relecture : nombres finis, plages cohérentes, une seule plage en ponctuelle, 36 valeurs par spectre et 152 données brutes pour le MYIRO-1 seulement. Une réflectance supérieure à 1 est acceptée. Le pont vérifie chaque mesure avant de l'envoyer : une valeur non finie de la DLL devient une erreur `reponse_inattendue`.
- La provenance est structurée : géométrie de lecture (ponctuelle, bande avec son sens brut, feuille), conditions de calcul **demandées** à la DLL et conditions **observées** sur l'instrument, séparées. Empreinte de la DLL, date d'étalonnage et conditions de calcul sont qualifiées confirmé, supposé ou inconnu. Le pont ne relit aucune condition : `observe` reste inconnu, ce qui précise la phrase « la condition archivée est celle relue sur l'instrument » ci-dessus tant qu'aucun appel vérifié ne la rend.
- Le format initial (pont 0.1.0, sans numéro) se relit en mémoire, sans réécrire les archives : les faits démontrables sont repris, le texte libre du calcul devient un libellé et les conditions restent inconnues. Toute autre version est refusée en clair ; un champ inconnu aussi.
- La conséquence plus bas qui range l'illuminant et la correspondance M0/M1/M2 parmi les champs inconnus est dépassée sur ces deux points : la fiche `docs/abi/FDX_GetMeasureData.md` les confirme sur l'instrument (`Illuminant` 0, 1, 2 donne M0, M1, M2 ; code 2 des Lab : D50). La provenance les déclare donc confirmés ; l'observateur 2° reste supposé, et `DeviceInfo` reste régi par sa propre fiche. Le texte d'origine reste pour l'historique.

### Complément du 7 octobre 2026 : désarmement et fermeture (ticket #24)

Ce complément remplit ce que le complément du ticket #23 renvoyait au ticket #24.

- **Une seule politique, dans la session.** Le désarmement (avant d'armer, après une lecture, à la fermeture) suit toujours la même règle : `FDX_StopMeasurement`, puis attente de l'événement 0 (retour au repos). Après un refus -9986, le pont attend un événement et réessaie : trois essais au plus, 5 s par attente, soit 15 s au plus. L'adapter de la DLL ne garde qu'une ultime tentative à sa destruction (un désarmement sans attente, puis `FDX_Disconnect`), pour le cas où la fermeture normale n'a pas abouti.
- **Le résultat est rapporté.** Le retour au repos vaut `au_repos` (événement 0 reçu après un désarmement accepté), `repos_non_signale`, `arret_refuse` (avec le code de la DLL) ou `liaison_perdue`. Un refus -9986 sans événement ne vaut repos que si rien n'a été armé depuis le dernier repos prouvé : c'est la réponse observée au repos, mais elle ne prouve rien après un armement. Une connexion neuve est supposée trouver l'instrument au repos (à observer).
- **Acquisition et repos séparés.** Une mesure lue reste rendue, avec sa provenance, même si le repos n'est pas prouvé ; la réponse `mesure` porte ce résultat dans `remise_au_repos`. La mesure suivante refait d'abord le désarmement ; sans preuve de repos, elle est refusée (`repos_incertain`) sans armer l'instrument. Une nouvelle connexion rétablit l'état.
- **Fermeture.** `ferme` n'est répondu qu'après la déconnexion faite et le repos prouvé ; `fermeture_incertaine` si la déconnexion est faite sans preuve de repos ; une erreur `deconnexion_echouee` si `FDX_Disconnect` échoue, et le pont reste alors à l'écoute. Dès la demande de fermeture, toute autre demande reçoit `session_fermee` sans appel à la DLL. Un nouveau `fermer` ne refait que ce qui n'est pas terminé (la déconnexion, jamais le désarmement). Sur une liaison perdue, la fermeture déconnecte sans désarmer. La fin de l'entrée ferme la session de la même façon, sans réponse.
- **Contrat, aux conventions du ticket #25.** Ajouts au protocole : champ `remise_au_repos` de la réponse `mesure`, à côté de la mesure et hors du format conservé `myiro-libre/mesure/1`, qualifié comme les autres données (`{"statut": "confirmee", "valeur": {"etat": …}}`) ; une réponse sans ce champ, ou une ligne du format initial, se relit `inconnue`, jamais comme réussie ([`docs/formats/mesure.md`](../formats/mesure.md)). Réponse `fermeture_incertaine`, erreurs `repos_incertain`, `deconnexion_echouee` et `session_fermee {}` (accolades vides, champs inconnus refusés). Aucun champ existant ne change ; `FDX_Disconnect`, déjà autorisé, entre dans le trait `SdkMyiro1`, sans nouvel export ni changement d'ABI.

Vérifié contre l'instrument simulé seulement. Les codes d'échec de `FDX_Disconnect` (pas encore de fiche) et le comportement du vrai MYIRO-1 quand l'événement 0 manque restent à observer.

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
