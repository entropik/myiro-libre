# myiro-libre

Application libre de mesure, de densitométrie et de profilage ICC pour les spectrophotomètres Konica Minolta MYIRO-1 et FD-9.

## Langage

### Mires et mesures

**Mire** :
Définition d'un ensemble de plages à imprimer puis mesurer : valeurs de chaque plage et mise en page. Une mire est indépendante du papier et de l'imprimante.
_Éviter_ : charte, chart, cible, target

**Mire de comparaison** :
Mire servant à comparer deux instruments sur un même tirage (36 plages de 20 mm, lisible en feuille et en bande), produite par `outils/mire_comparaison.py`.
_Éviter_ : mire de test, charte de contrôle

**Tirage** :
Réalisation physique d'une mire, imprimée dans une condition d'impression donnée ; c'est ce que l'instrument mesure.
_Éviter_ : épreuve, impression, feuille (au sens d'objet mesuré)

**Mesure** :
Une passe complète de l'instrument sur un tirage, conservant tous les spectres obtenus ; un même tirage peut être mesuré plusieurs fois.
_Éviter_ : lecture, scan, relevé

**Mesure ponctuelle** :
Mesure d'une seule plage ou couleur, hors mire.
_Éviter_ : spot, mesure spot

**Couleur de référence** :
Couleur nommée, saisie ou mesurée par l'utilisateur, à laquelle on compare une mesure ponctuelle.
_Éviter_ : ton direct, spot color, nuancier

**Feuille** :
Géométrie de lecture du FD-9 : le tirage entier est lu en une passe.

**Bande** :
Géométrie de lecture du MYIRO-1 : une rangée de plages lue en un balayage.
_Éviter_ : strip, rangée

**Instrument** :
Un spectrophotomètre précis (modèle, n° de série, micrologiciel), enregistré avec chaque mesure qu'il produit.
_Éviter_ : appareil, device, spectro

**Étalonnage** :
Mise à zéro de l'instrument sur son blanc de référence avant mesure.
_Éviter_ : calibration (ambigu avec la linéarisation)

**Condition de mesure** :
Condition d'éclairage de l'instrument selon l'ISO 13655 (M0, M1, M2…) sous laquelle un spectre est obtenu.
_Éviter_ : mode de mesure, filtre

### Pont et provenance

**Pont** :
Exécutable séparé qui charge la DLL du fabricant pour un instrument donné et seul la touche ; l'application lui parle par un protocole. Il applique la liste blanche des appels autorisés.
_Éviter_ : bridge, wrapper, driver, pilote (réservé au pilote Windows)

**Palier** :
Étape de la progression imposée sur instrument réel (version du SDK, détection, connexion, étalonnage, mesure ponctuelle, puis bande ou feuille). Le pont refuse tout palier au-delà du plafond autorisé au lancement ; chaque pont déclare les paliers qui existent pour son instrument.
_Éviter_ : phase (réservé au plan d'action), niveau, étape

**Logiciel du fabricant** :
Logiciel Konica Minolta (ou d'un partenaire) installé sur le poste qui contient `FDXSDK.dll` ; l'application l'y trouve seule et confie la DLL au pont de la même architecture. C'est le terme montré à l'opérateur ; « SDK » et « DLL » restent dans le code et le détail technique.
_Éviter_ : SDK, DLL, emplacement du SDK (à l'écran)

**DLL embarquées** :
Copie locale des DLL du fabricant (dossier `SDK/` du dépôt, ignoré par git) incluse dans un installateur construit sur le poste. Un tel installateur ne doit jamais être publié ; les DLL ne sont jamais versionnées.
_Éviter_ : DLL fournies, DLL livrées

**État de l'instrument** :
Ce que la barre du haut dit de l'instrument, en un mot : non détecté, connecté, étalonnage requis ou étalonné. Il est établi par le module `instrument`, jamais par un écran.
_Éviter_ : statut, mode

**Plafond** :
Dernier palier qu'un pont a le droit d'atteindre, fixé à son lancement ; toute demande au-delà est refusée sans toucher à l'instrument.
_Éviter_ : limite, niveau maximal

**Progression** :
Liste des paliers déjà franchis par un pont depuis son lancement. C'est un historique : un palier franchi une fois n'autorise pas à mesurer.
_Éviter_ : niveau atteint, état (au sens d'autorisation)

**État courant** :
Ce que le pont sait de l'instrument à cet instant : non connecté, inexploitable (identité illisible), connecté, étalonné ou perdu (liaison coupée). Lui seul autorise un étalonnage ou une mesure ; un échec d'étalonnage, une perte de liaison ou une nouvelle connexion le font redescendre.
_Éviter_ : statut, palier (réservé à la progression imposée)

**Prise de main** :
Droit exclusif d'un logiciel à piloter un FD-9 en réseau ; un seul l'a à la fois. Le pont l'obtient à la connexion et la rend à la déconnexion. FD-S2w resté connecté la garde.
_Éviter_ : autorité, authority, verrou

**Armement** :
Mise en attente de l'instrument pour une mesure ponctuelle ou une bande : il attend l'appui sur son bouton. Ne règle rien de permanent dans l'instrument.
_Éviter_ : réglage, condition (au sens de `SetMeasureCondition`)

**Remise au repos** :
Désarmement de l'instrument et ce que le pont en sait ensuite (`remise_au_repos` dans le protocole) : au repos (prouvé par l'instrument), repos supposé (rien armé, aucune preuve), ou incertain, avec la raison (repos non signalé, arrêt refusé, liaison perdue). Une mesure déjà lue reste valable quand elle est incertaine, mais la suivante est refusée tant que le repos n'est pas prouvé. Le « retour au repos » est l'événement 0 de l'instrument, qui sert de preuve.
_Éviter_ : nettoyage réussi, arrêt (au sens du résultat)

**Fermeture** :
Fin volontaire de la session avec l'instrument : désarmement, puis déconnexion. Elle est confirmée seulement si les deux sont faits et le repos prouvé ; sinon elle est incertaine (déconnecté, repos supposé ou non prouvé) ou en échec (déconnexion refusée, à redemander).
_Éviter_ : arrêt, sortie, déconnexion (qui n'en est qu'une étape)

**Données brutes** :
Valeurs rendues par l'instrument avant tout calcul de spectre (152 par plage pour le MYIRO-1), conservées avec chaque mesure pour valider le pilote libre (ADR 0006).
_Éviter_ : raw, données capteur

**Jeu de validation** :
Ensemble de paires « données brutes → spectres M0, M1, M2 » calculées par la DLL du fabricant sur des mesures réelles, sans identifiant d'instrument, qui sert d'étalon au pilote libre (ADR 0006). Il se produit en local et n'est jamais versionné. Format : `docs/pilote-libre/jeu-validation.md`.
_Éviter_ : dataset, jeu de test, corpus

**Banc de comparaison** :
Outil qui fait calculer les spectres d'un jeu de validation par un calcul candidat et rend, par condition de mesure, l'écart moyen et maximal avec la DLL, longueur d'onde par longueur d'onde.
_Éviter_ : benchmark (mesure de vitesse), validateur

**Provenance** :
Ce que le pont atteste sur une mesure : instrument (modèle, n° de série, micrologiciel), chaîne logicielle (SDK, pont, empreinte de la DLL), date réelle avec fuseau, géométrie de lecture, condition de mesure relue sur l'instrument, illuminant, observateur. Le reste du contexte (support, encres, séchage, chauffe…) relève de la bibliothèque, pas du pont. Elle est posée par le pont, jamais reconstituée par l'application, et aucune mesure n'existe sans elle.
_Éviter_ : métadonnées, contexte

**Format de mesure** :
Forme écrite d'une mesure, identique dans le protocole et dans la bibliothèque, désignée par un nom versionné (`myiro-libre/mesure/1`). Une version non prise en charge est refusée en clair. Le **format initial** est celui du pont 0.1.0, sans numéro : il se relit sans réécrire les archives, ses faits non démontrables restant inconnus. Description : `docs/formats/mesure.md`.
_Éviter_ : schéma, export (réservé à CGATS)

**Conditions demandées / observées** :
Les conditions de calcul que le pont a demandées à la DLL, et celles relues sur l'instrument par un appel vérifié. Les deux sont conservées séparément ; tant que rien n'est relu, les conditions observées sont inconnues.
_Éviter_ : conditions réelles, paramètres

**Inconnu** :
État d'une donnée que l'instrument ou le SDK n'a pas fournie ou dont le sens n'est pas établi. Une donnée est confirmée, supposée ou inconnue ; une valeur inconnue n'est jamais remplacée par zéro, une chaîne vide ou une valeur par défaut.
_Éviter_ : null, vide, zéro, par défaut

### Couleur

**Écart de couleur** :
Différence chiffrée entre deux couleurs Lab. ΔE00 (CIEDE2000) par défaut ; ΔC (écart de chroma) et ΔH (écart de teinte) sont des critères distincts, signés, qu'on ne confond pas avec ΔE00. On dit toujours quelle formule est employée.
_Éviter_ : delta E sans précision, différence de couleur

**Teinte** :
Angle de la couleur dans le plan a*b*, en degrés (0 à 360). Un gris parfait (chroma nulle) n'a pas de teinte : elle est inconnue, pas nulle.
_Éviter_ : hue, nuance

### Impression

**Condition d'impression** :
Combinaison caractérisée d'une machine, d'un papier, d'encres et de réglages ; elle porte les tirages, les profils et les références de contrôle.
_Éviter_ : setup, configuration, preset

**Linéarisation** :
Courbes par canal ramenant la réponse de chaque encre à une progression visuellement régulière, avant le profilage.
_Éviter_ : calibration (réservé à l'étalonnage de l'instrument), courbes de transfert

**Limite d'encre** :
Quantité maximale utile d'un canal d'encre, au-delà de laquelle la couleur ne progresse plus ; déterminée avec la linéarisation.
_Éviter_ : ink limit, saturation

**Vérification de profil** :
Contrôle d'impression d'un tirage imprimé à travers un profil, dont la référence est calculée à partir de ce profil.
_Éviter_ : validation de profil, test de profil

### Densitométrie et contrôle

**Densité** :
Valeur densitométrique (statut T, E, I…) calculée à partir du spectre d'une plage mesurée.
_Éviter_ : D, valeur densito

**Contrôle d'impression** :
Comparaison d'un tirage mesuré à une référence, avec verdict selon des tolérances, et suivi dans le temps pour une condition d'impression.
_Éviter_ : QC, validation

**Référence** :
Ensemble de valeurs cibles et de tolérances servant au contrôle d'impression, issu d'une norme ou d'un tirage validé mesuré par l'utilisateur.
_Éviter_ : cible, target, standard

### Application

**Tâche** :
L'un des quatre grands travaux proposés dans la barre du haut : Mesurer, Contrôler, Profiler, Bibliothèque. Chaque tâche a sa feuille de travail au centre.
_Éviter_ : onglet, module, mode

**Bibliothèque** :
Base unique sur le poste, pour un utilisateur, où sont rangées les conditions d'impression et leurs mesures, chacune rattachée à l'instrument qui l'a produite (ADR 0001). Elle conserve les mesures telles que le pont les a données, sans rien recalculer. La colonne de gauche de l'application en montre l'arborescence.
_Éviter_ : base de données (au sens technique), dossier de mesures, catalogue

**État vide** :
Ce qu'affiche une tâche qui n'a encore rien à montrer : une phrase et une seule action (« Aucune mesure. Mesurer une couleur »).
_Éviter_ : écran blanc, placeholder
