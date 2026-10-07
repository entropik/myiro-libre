# La DLL d'abord, puis un pilote libre validé contre elle

Les SDK Konica Minolta (`FDXSDK`, `FD9SDK`) ne sont plus maintenus et MYIRO tools ferme fin 2026. Une DLL figée finira par ne plus fonctionner (nouvelle version de Windows, DLL MYIRO-1 de MY-CT1 en 32 bits seulement, pas de version Linux). L'analyse des logiciels récupérés montre qu'un pilote libre est atteignable : les instruments se présentent comme un port série USB (ou TCP en réseau), les commandes du protocole et leur somme de contrôle sont identifiées, et le calcul des spectres se fait dans le SDK, pas dans l'instrument.

## Décision

1. **La DLL d'abord.** La v1 mesure par la DLL du fabricant, à travers les ponts de l'ADR 0005. C'est la voie la plus sûre pour l'instrument et la plus rapide vers des mesures réelles.
2. **Puis un pilote libre**, écrit de zéro en Rust, qui parle directement à l'instrument et refait les calculs du SDK. Il est construit **commande par commande**, en commençant par la lecture seule. Une fonction ne passe au pilote libre que lorsque ses résultats sont **identiques** à ceux de la DLL sur les mêmes mesures.
3. **La DLL sert d'étalon.** Elle fournit, pour une même mesure, les données brutes de l'instrument (`GetRAWData`) et le spectre qu'elle en calcule (`GetMeasureData`). Dès la v1, chaque mesure faite par la DLL conserve donc ses données brutes avec ses spectres : elles forment le jeu de validation du pilote libre.
4. **Le pilote libre est un adapter de plus au seam des ponts** (ADR 0005) : l'application ne change pas, et l'on peut toujours revenir à la DLL.

## Ce que le pilote libre doit refaire

- **Le dialogue** avec l'instrument : trames, somme de contrôle, commandes, événements, en USB (port série virtuel) et en réseau.
- **Le calcul** : données brutes (152 valeurs) et données d'étalonnage d'usine lues dans l'instrument, vers les spectres M0, M1, M2.
- **La lecture de bande** (MYIRO-1) : reconnaissance des plages et du sens de passage. **La reconnaissance de mire** (FD-9) : capture et repérage des plages.

## Sécurité

Avec un pilote libre, c'est notre code qui envoie les octets à l'appareil. La liste blanche s'applique alors aux **codes de commande du protocole**, avec la même règle que pour les exports : lecture seule d'abord, aucune commande de mise à jour du micrologiciel, d'écriture en mémoire d'usine ou de références d'étalonnage. Certaines commandes servent à la fois à lire et à écrire selon un sous-code : le filtre porte sur le couple commande et sous-code. Toute trame non comprise est refusée, jamais rejouée.

## Conséquences

- Le pont DLL garde systématiquement les données brutes et la provenance complète de chaque mesure.
- Le calcul des spectres devient un module testable sans instrument, validé contre des paires « brut → spectre » produites par la DLL.
- Une fois validé, le pilote libre rend possibles Mac et Linux, et l'application ne dépend plus d'aucun composant fermé.
- L'observation du trafic entre un logiciel officiel et l'instrument (port série, réseau) reste passive : on écoute, on ne rejoue rien.
- L'analyse menée pour l'interopérabilité ne copie aucun code du fabricant : seuls des faits (formats, codes, comportements) passent dans le dépôt.

### Complément du 7 octobre 2026

Le jeu de validation et son banc vivent dans une crate à part, `jeu-validation`, indépendante de Windows et des ponts (comme les crates partagées de l'ADR 0004) ; elle ne dépend que de `pont-protocole` pour relire les sorties JSON du pont. Le calcul candidat y est un trait, `CalculSpectres`, dont le pilote libre sera un adapter. Le jeu remplace le numéro de série par un pseudonyme et se produit hors git ; seul son format est documenté (`docs/pilote-libre/jeu-validation.md`).

## Options écartées

- **Pilote libre tout de suite** : plus long avant la première mesure, et sans étalon pour vérifier les calculs.
- **Rester indéfiniment sur la DLL** : dépendance à un composant fermé et non maintenu, appelé à cesser de fonctionner.
