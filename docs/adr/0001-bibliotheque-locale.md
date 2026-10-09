# Les mesures vivent dans une bibliothèque locale, pas dans des fichiers

Les mesures, conditions d'impression, références, profils et linéarisations sont conservés dans une bibliothèque locale unique (une base sur le poste), et non comme des fichiers CGATS épars dans des dossiers. Le contrôle d'impression exige un suivi dans le temps et des liens entre condition d'impression, tirages, mesures et profils, que des fichiers isolés ne portent pas.

## Conséquences

- Pour ne jamais enfermer les données : export CGATS.17 de toute mesure, import CGATS, et sauvegarde/restauration complète de la bibliothèque.
- Chaque mesure conserve tous les spectres fournis par l'instrument (toutes les conditions de mesure disponibles) ; le choix M0/M1/M2 se fait au calcul, pas à la mesure.
- Un poste, un utilisateur en v1 : pas de bibliothèque partagée sur le réseau.

## Complément du 7 octobre 2026 : première bibliothèque (ticket #6)

- La crate `crates/bibliotheque` porte la bibliothèque. Elle ne dépend ni de Tauri, ni de Windows, ni des ponts (ADR 0004) : seulement des types de mesure de `pont-protocole`. Elle est contrôlée aussi sur le poste Linux de la CI.
- La base est un fichier SQLite (`bibliotheque.sqlite`, moteur compilé avec la crate, rien à installer), dans un seul dossier par utilisateur : le dossier de données de l'application (sous Windows, `%APPDATA%\org.myiro-libre.app\bibliotheque\`), jamais dans le dépôt.
- La base porte un numéro d'organisation. À l'ouverture, elle est amenée au numéro attendu une étape après l'autre (de 0 à 1, puis de 1 à 2, et ainsi de suite), et tout se fait dans une seule transaction avec l'écriture du nouveau numéro : un arrêt en cours de route (coupure, plantage) n'écrit rien, et la base se rouvre à son ancien numéro pour recommencer. Une base écrite par une version plus récente est refusée sans être touchée. L'étape 0 → 1 tolère les tables déjà présentes, que laissait la toute première version de la crate, qui écrivait les tables puis le numéro en deux temps.
- Une mesure est conservée dans son format versionné (`myiro-libre/mesure/1`, [description](../formats/mesure.md)), tel que le pont l'a produit : spectres M0, M1, M2, données brutes, Lab et provenance se relisent à l'identique, et les données inconnues restent inconnues. La bibliothèque ne recalcule ni ne complète rien.
- Une mesure est rattachée à une condition d'impression (choisie par l'utilisateur) et à un instrument, reconnu par le modèle et le numéro de série de sa provenance ; le micrologiciel reste dans la provenance de chaque mesure, puisqu'il peut changer.
- Les noms de condition d'impression sont uniques sans tenir compte des majuscules (« Offset » et « offset » sont la même condition), espaces de début et de fin retirés. Les mesures se rangent des plus récentes aux plus anciennes selon l'instant réel de la mesure, fuseau compris, et non selon le texte de la date. La recherche ne tient compte ni des majuscules ni des accents ; elle porte sur le nom de la condition, l'instrument et la date de la mesure.
- Pas encore : suppression, export et import CGATS, sauvegarde et restauration, contexte de la condition d'impression (support, encres, RIP…).

## Complément du 9 octobre 2026 : nom d'une mesure (ticket #7)

- Une mesure peut porter un nom, donné par l'utilisateur dans la tâche Mesurer (« Couleur 1 » par défaut) et modifiable. Il est enregistré avec la mesure, à côté de son format versionné, qui ne change pas : le nom n'est pas une donnée du pont. Espaces de début et de fin retirés ; un nom vide est refusé ; deux mesures peuvent porter le même nom.
- Organisation 2 de la base : une colonne `nom` ajoutée à la table des mesures. Rien n'est réécrit ni effacé ; les mesures déjà enregistrées restent sans nom, et une mesure enregistrée sans nom (bande, import) n'en reçoit pas d'office. Un test relit sans perte une base de l'organisation 1.
- La colonne de gauche montre le nom devant la date ; la recherche porte aussi sur le nom de la mesure.

## Complément du 9 octobre 2026 : export, import et sauvegarde (ticket #9)

- Export CGATS.17 de toute mesure, depuis la barre d'actions en haut de la feuille de la tâche Bibliothèque, toujours en vue au-dessus du tableau : « Importer un fichier… », un seul export nommé par son usage (« Exporter pour le profilage… »), « Sauvegarder… », « Restaurer… » ; sélecteur de fichier de Windows. Le détail technique est replié sous « Que font ces boutons ? ». Format et écarts avec l'export du fabricant : [`docs/formats/cgats.md`](../formats/cgats.md).
- Import CGATS : le fichier est lu en **mesure importée**, distincte d'une mesure attestée par un pont ; ce qu'il ne donne pas, spectre compris, reste inconnu. L'import se fait en deux temps : la mesure lue est d'abord montrée en aperçu, rien n'est rangé ; l'opérateur choisit la condition d'impression (celle qu'il avait choisie à gauche est proposée) puis « Ranger dans « … » », ou « Annuler ». Elle est alors rangée à part des mesures du pont : la colonne de gauche et le cartouche disent « Importée » et le nom du fichier d'origine (jamais son dossier). La bibliothèque garde le texte du fichier tel quel et le relit à chaque fois ; elle ne crée aucun instrument d'après ce qu'un fichier déclare.
- Organisation 3 de la base, par-dessus l'organisation 2 du ticket #7 (qui reste telle quelle) : la table des mesures est refaite pour porter l'origine (`pont` ou `importee`), le nom du fichier d'origine, et accepter une mesure sans instrument attesté, date ni géométrie. La mise à niveau recopie toutes les mesures avec leur numéro et leur nom, dans la même transaction que le changement de numéro : une base ou une sauvegarde d'organisation 1 ou 2 (mesures nommées comprises) se relit sans perte (tests). Une mesure importée n'a pas de nom de mesure ; elle est désignée par son fichier.
- Sauvegarde complète : copie de la base en un seul fichier SQLite. La restauration remplace toute la bibliothèque après accord de l'opérateur, et seulement si la sauvegarde a été examinée sans défaut (organisation connue, liens cohérents, chaque mesure relisible).

## Complément du 9 octobre 2026 : couleur de référence (ticket #8)

- Une mesure peut être désignée couleur de référence, avec un écart ΔE00 accepté (son seuil) ou sans seuil. Le seuil absent est enregistré comme absent (`NULL`), jamais comme une valeur sentinelle ; un seuil nul, négatif ou non fini est refusé et rien n'est écrit.
- Organisation 4 de la base, par-dessus l'organisation 3 du ticket #9 (qui reste telle quelle) : une table `references_couleur` (mesure, seuil), dont la clé étrangère pointe vers la table des mesures refaite par l'organisation 3. Rien n'est réécrit ni effacé ; des tests relisent sans perte une base de l'organisation 2 et une de l'organisation 3 (mesure importée comprise) mises à niveau pas à pas, sans lien rompu. Une base d'organisation 4 est refusée sans être touchée par une version qui attend l'organisation 3 au plus. La mesure elle-même ne change pas : retirer la référence laisse la mesure et son nom.
- Seule une mesure d'un pont peut être couleur de référence ; une mesure importée ne le peut pas.
- Dans Mesurer, la séance a une seule couleur de référence à la fois : en désigner une autre remplace la première d'un bloc (`remplacer_reference`, une transaction) ; si la bibliothèque refuse, l'ancienne reste et l'écran le dit, jamais deux références. Seule une mesure rangée peut devenir référence, puisque c'est la bibliothèque qui la conserve.
- Au lancement, la couleur de référence conservée la plus récente revient dans Mesurer, avec son nom et son seuil, sans être rangée une seconde fois.
