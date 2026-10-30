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

**Mire RGB-2520** :
La mire du FD-9 : 2 520 plages RVB sur une seule page A3, avec son QR code d'origine, reprise telle quelle (décision du mainteneur, 26 octobre 2026). myiro-libre en vérifie l'empreinte et la copie octet pour octet ; il ne la redessine pas.
_Éviter_ : mire FD-9 dessinée, charte RGB

**Taille de mire** :
Nombre de plages d'une mire MYIRO-1 : rapide (232), standard (464) ou approfondi (928). Point de départ validé par le mainteneur le 23 octobre 2026, à corriger après essais.
_Éviter_ : résolution de mire, densité de mire

**Gabarit** :
Mise en page des bandes du MYIRO-1 pour un format (A4 ou A3) et un côté de plage : nombre de plages par bande, de bandes par page, bandeau du texte et du QR code.
_Éviter_ : modèle, template

**Plan de lecture** :
Ordre et place de chaque plage d'une mire, page par page, bande par bande. Il est rangé avec la mire, avec son empreinte, et sert à relire les mesures dans le bon ordre.
_Éviter_ : layout, disposition

**Tirage** :
Réalisation physique d'une mire, imprimée dans une condition d'impression donnée ; c'est ce que l'instrument mesure. Le tirage rangé dans la bibliothèque à la préparation d'une mire en est la trace (date, mire, nombre de pages), avant même l'impression.
_Éviter_ : épreuve, impression, feuille (au sens d'objet mesuré)

**Mesure** :
Une passe complète de l'instrument sur un tirage, conservant tous les spectres obtenus ; un même tirage peut être mesuré plusieurs fois.
_Éviter_ : lecture, scan, relevé

**Mesure ponctuelle** :
Mesure d'une seule plage ou couleur, hors mire.
_Éviter_ : spot, mesure spot

**Couleur de référence** :
Couleur nommée, saisie ou mesurée par l'utilisateur, à laquelle on compare une mesure ponctuelle. Dans Mesurer, une mesure rangée de la séance est désignée couleur de référence ; elle est conservée dans la bibliothèque avec son écart accepté.
_Éviter_ : ton direct, spot color, nuancier

**Feuille** :
Géométrie de lecture du FD-9 : le tirage entier est lu en une passe.

**Bande** :
Géométrie de lecture du MYIRO-1 : une rangée de plages lue en un balayage.
_Éviter_ : strip, rangée

**Instrument** :
Un spectrophotomètre précis (modèle, n° de série, micrologiciel), enregistré avec chaque mesure qu'il produit. À l'écran, seul son modèle est montré : aucune vue de la page (vue d'une résolution, feuille des mesures, Mesurer) ne montre son n° de série, qui reste dans la provenance des mesures et dans l'export CGATS.
_Éviter_ : appareil, device, spectro

**Étalonnage** :
Mise à zéro de l'instrument sur son blanc de référence avant mesure.
_Éviter_ : calibration (ambigu avec la linéarisation)

**Capuchon** :
Pièce qui se fixe sous le MYIRO-1 et contient son blanc de référence, une tuile appairée à l'instrument par son n° de série. Pour l'étalonnage, l'instrument est posé à plat dessus.
_Éviter_ : socle, support, tuile (la tuile n'en est qu'une partie)

**Geste** :
Action que seul l'opérateur peut faire, demandée par le module `instrument` à travers l'écran : poser le MYIRO-1 sur son capuchon, le poser sur la couleur à mesurer (plus tard, passer la bande). L'opérateur le fait ou y renonce ; s'il renonce, rien n'est envoyé à l'instrument.
_Éviter_ : action manuelle, interaction

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
Logiciel Konica Minolta (ou d'un partenaire) installé sur le poste qui contient `FDXSDK.dll` (MYIRO-1) ou `FD9SDK.dll` (FD-9, par exemple FD-S2w) ; l'application l'y trouve seule et confie la DLL au pont de la même architecture. C'est le terme montré à l'opérateur ; « SDK » et « DLL » restent dans le code et le détail technique.
_Éviter_ : SDK, DLL, emplacement du SDK (à l'écran)

**DLL embarquées** :
Copie locale des DLL du fabricant (dossier `SDK/` du dépôt, ignoré par git) incluse dans un installateur construit sur le poste. Un tel installateur ne doit jamais être publié ; les DLL ne sont jamais versionnées.
_Éviter_ : DLL fournies, DLL livrées

**État de l'instrument** :
Ce que la barre du haut dit de l'instrument, en un mot : non détecté, détecté (vu par la détection mais pas connecté, comme le FD-9 tant que son pont s'arrête à ce palier), connecté, étalonnage requis ou étalonné. Il est établi par le module `instrument`, jamais par un écran.
_Éviter_ : statut, mode

**Instrument actif** :
L'instrument du poste dont le pont est ouvert, montré dans la barre ; un seul à la fois. Le choisir dans la liste des instruments du poste ferme d'abord le pont de l'autre. Le dernier choix est retenu d'un lancement à l'autre.
_Éviter_ : instrument sélectionné, instrument par défaut, appareil courant

**Mesure automatique** :
Mesure ponctuelle du MYIRO-1 que l'application fait partir elle-même : l'opérateur pose l'instrument sur la couleur puis clique sur « Mesurer », sans appuyer sur son bouton. Le pont arme l'instrument puis le déclenche par `FDX_StartMeasurement`. Que le MYIRO-1 accepte ce déclenchement est supposé jusqu'à l'essai réel. Son contraire est la **mesure manuelle**, qui part à l'appui sur le bouton. Choix « Mesure : automatique / manuelle » de la feuille Mesurer, automatique par défaut. Le FD-9 ne mesure pas encore.
_Éviter_ : mode déclenché, trigger, mesure logicielle (à l'écran)

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

**Adresse réseau** :
Paramètre de connexion propre au FD-9 : son adresse IP (par exemple 192.0.2.40) ou son nom sur le réseau, 23 caractères au plus. Saisie, elle dispense de la détection.
_Éviter_ : IP (à l'écran), hôte, port

**Autorisation du pare-feu** :
Règle du pare-feu de Windows que myiro-libre ajoute pour laisser passer la réponse du FD-9 à la détection : entrante, UDP, port local 49152, réseau local, pour le seul programme pont du FD-9. Nom fixe « myiro-libre - FD-9 - detection (UDP 49152) ». Demandée une fois, par la fenêtre de contrôle de compte de Windows, à la première détection qui ne trouve rien.
_Éviter_ : ouvrir les ports, exception, désactiver le pare-feu

**Armement** :
Mise en attente de l'instrument pour une mesure ponctuelle ou une bande : il attend l'appui sur son bouton. Ne règle rien de permanent dans l'instrument.
_Éviter_ : réglage, condition (au sens de `SetMeasureCondition`)

**Remise au repos** :
Désarmement de l'instrument et ce que le pont en sait ensuite (`remise_au_repos` dans le protocole) : au repos (prouvé par l'instrument), repos supposé (rien armé, aucune preuve), ou incertain, avec la raison (repos non signalé, arrêt refusé, liaison perdue). Une mesure déjà lue reste valable quand elle est incertaine, mais la suivante est refusée tant que le repos n'est pas prouvé. Le « retour au repos » est l'événement 0 de l'instrument, qui sert de preuve.
_Éviter_ : nettoyage réussi, arrêt (au sens du résultat)

**Fermeture** :
Fin volontaire de la session avec l'instrument : désarmement, puis déconnexion. Elle est confirmée seulement si les deux sont faits et le repos prouvé ; sinon elle est incertaine (déconnecté, repos supposé ou non prouvé) ou en échec (déconnexion refusée, à redemander).
_Éviter_ : arrêt, sortie, déconnexion (qui n'en est qu'une étape)

**Annulation** :
Demande de l'opérateur d'interrompre la mesure en cours tant qu'elle attend (le bouton, ou en automatique le déclenchement). Elle ne vise que cette mesure, jamais la suivante. Le pont désarme l'instrument et rapporte la remise au repos ; une mesure déjà partie ou terminée est gardée. L'issue d'une mesure est unique : acquise, annulée, abandonnée (rien envoyé), refusée ou échouée.
_Éviter_ : arrêt, abandon (pour une mesure déjà envoyée), interruption forcée

**Interruption forcée** :
Arrêt du programme pont par l'application, quand il ne répond plus dans le délai. L'état de l'instrument devient incertain et une récupération est demandée (débrancher, rebrancher, réessayer) : un pont arrêté ne prouve jamais que l'instrument est au repos.
_Éviter_ : annulation, fermeture

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

**Mesure importée** :
Mesure lue dans un fichier CGATS.17 (écrit par myiro-libre ou par un autre logiciel), et non produite par un pont : ce qu'elle dit vient du fichier, pas d'une provenance attestée. Elle est rangée dans une condition d'impression, marquée « Importée » avec le nom de son fichier. Ce que le fichier ne donne pas, spectre compris, reste inconnu. Description : `docs/formats/cgats.md`.
_Éviter_ : mesure externe, mesure chargée

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

**Écart accepté** :
Écart ΔE00 au-delà duquel une mesure est hors tolérance par rapport à une couleur de référence ; choisi par l'utilisateur et conservé avec la référence. Tant qu'il n'est pas fixé, il n'y a pas de verdict (« seuil non fixé ») et aucune valeur par défaut ne le remplace.
_Éviter_ : tolérance par défaut, seuil 0

**Verdict d'écart** :
Jugement d'un écart au regard de l'écart accepté, à trois niveaux : conforme (sous 80 % de l'écart accepté), proche de la limite (de 80 % à l'écart accepté inclus), hors tolérance (au-delà). Les 80 % sont un choix du projet, pas une valeur de norme. Entre deux mesures de conditions de mesure différentes, il n'y a ni écart ni verdict. Il porte sur l'écart tel qu'il est écrit, à deux décimales. Toujours un mot et un carré, jamais la couleur seule ; le rouge est réservé au hors tolérance.
_Éviter_ : OK/KO, bon/mauvais, pass/fail

**Teinte** :
Angle de la couleur dans le plan a*b*, en degrés (0 à 360). Un gris parfait (chroma nulle) n'a pas de teinte : elle est inconnue, pas nulle.
_Éviter_ : hue, nuance

**Chroma** :
Distance d'une couleur à l'axe des gris dans le plan a\*b\* (C\*) : plus elle est grande, plus la couleur est vive. Un gris a une chroma nulle. La chroma ne dépend pas de la clarté, au contraire de la saturation au sens courant.
_Éviter_ : saturation, pureté

### Impression

**Condition d'impression** :
Combinaison caractérisée d'une machine, d'un papier, d'encres et de réglages ; elle porte les tirages, les profils et les références de contrôle. Dans la bibliothèque, c'est la feuille de l'arbre imprimante → papier → résolution d'impression ; une condition créée avant ce rangement, sans papier, est dite « non classée ».
_Éviter_ : setup, configuration, preset

**Imprimante** :
Une machine du parc de l'atelier, premier niveau de la bibliothèque ; chaque imprimante a ses propres papiers.
_Éviter_ : périphérique, device, traceur

**Papier** :
Un support utilisé sur une imprimante donnée ; le même papier sur deux imprimantes fait deux papiers, car il s'imprime différemment.
_Éviter_ : média, media, substrat

**Résolution d'impression** :
Réglage de qualité du pilote pour un papier (par exemple « 1200 ppp ») ; chaque résolution d'un papier est une condition d'impression et peut avoir son propre profil.
_Éviter_ : DPI, qualité, mode

**Profil actif** :
Le profil ICC en service pour une condition d'impression : un seul à la fois. Un nouveau profil le remplace, et l'ancien reste dans l'historique avec son fichier intact. Un profil actif de plus d'un an est signalé comme ancien.
_Éviter_ : profil courant, profil par défaut

**Linéarisation** :
Courbes par canal ramenant la réponse de chaque encre à une progression visuellement régulière, avant le profilage.
_Éviter_ : calibration (réservé à l'étalonnage de l'instrument), courbes de transfert

**Limite d'encre** :
Quantité maximale utile d'un canal d'encre, au-delà de laquelle la couleur ne progresse plus ; déterminée avec la linéarisation.
_Éviter_ : ink limit, saturation

**Vérification de profil** :
Évaluation d'un tirage d'une seconde mire imprimée à travers un profil : elle distingue la fidélité aux couleurs demandées de l'accord entre les mesures et la prédiction du profil. Les deux résultats conservent leurs références et leurs tolérances.
_Éviter_ : validation de profil, test de profil

**Fidélité aux couleurs demandées** :
Écart ΔE00 entre la couleur demandée (les RVB de la seconde mire lus comme du sRVB, en colorimétrique relatif) et la mesure ramenée au blanc du support. Elle se calcule seulement pour une mire imprimée à travers le profil, et seulement sur les couleurs dans le gamut.
_Éviter_ : justesse, exactitude

**Accord avec la prédiction** :
Écart ΔE00 entre ce que le profil prédit pour la valeur après transformation et la mesure telle quelle. Il dit si le profil décrit bien l'imprimante, même pour une couleur qu'elle ne peut pas atteindre.
_Éviter_ : précision (réservé à la précision du calcul)

**Valeur après transformation** :
Les RVB envoyés à l'imprimante une fois la couleur demandée convertie par le profil. myiro-libre ne sait pas ce que le logiciel d'impression a envoyé : ArgyllCMS refait la conversion avec le même profil, la valeur est donc calculée, approchée, et dite comme telle.
_Éviter_ : valeur imprimée, valeur réelle

**Seuil de gamut** :
Écart ΔE00 entre la couleur demandée et ce que le profil prédit pouvoir en rendre, au-delà duquel la couleur est hors gamut. Ces couleurs restent listées, avec leurs propres statistiques ; les critères de fidélité portent sur les couleurs dans le gamut. Il fait partie du jeu de tolérances.
_Éviter_ : limite de gamut (réservé à l'enveloppe du gamut d'un profil)

**Jeu de tolérances** :
Les seuils choisis par l'imprimeur pour accepter un profil : moyenne, 95 % des plages et maximum, pour la fidélité et pour la prédiction (chacun peut rester vide), et le seuil de gamut. Aucun n'est imposé : la page propose des valeurs de départ, à confirmer.
_Éviter_ : norme, seuils par défaut

**Version de tolérances** :
Numéro d'un jeu de tolérances, augmenté de un à chaque modification enregistrée. Une version n'est jamais modifiée ; une vérification garde la version avec laquelle elle a été faite, et son rapport se régénère à l'identique. Un profil accepté l'est « selon Atelier v2 ».
_Éviter_ : révision, mise à jour du jeu

**Vérification incomplète** :
État d'un profil dont un critère demandé n'a pas pu être calculé (mire imprimée sans gestion des couleurs alors qu'un seuil de fidélité est demandé, ou aucune couleur dans le gamut), sans qu'aucun seuil ne soit dépassé. Elle n'accepte jamais le profil.
_Éviter_ : acceptation partielle, vérification à zéro

**Recalcul** :
Nouvelle conclusion d'une vérification avec un autre jeu de tolérances, à partir des écarts gardés plage par plage, sans nouvelle mesure ni ArgyllCMS. Il s'ajoute à l'historique ; la vérification d'origine reste telle quelle.
_Éviter_ : remesure, correction de la vérification

**Mire de vérification FD-9** :
Mire de 210 couleurs créée une seule fois par myiro-libre, différentes de celles de la RGB-2520, rangée dans la bibliothèque avec les mires de profilage (mais sans tirage) et écrite pour FD-S2w, qui l'ouvre et en tire le fichier à imprimer.
_Éviter_ : seconde RGB-2520, mire de contrôle

**Recette de calcul** :
Qualité demandée au calcul d'un profil : rapide (pour un essai), standard (par défaut, en production) ou approfondie (en cours de réglage depuis le 28 octobre 2026, pas encore disponible). Elle est conservée avec le profil, avec la version d'ArgyllCMS et la commande lancée.
_Éviter_ : preset, mode, qualité seule

**Précision du calcul** :
Écart ΔE00 moyen et maximal entre les mesures qui ont servi au calcul et ce que le profil prédit pour elles. Elle dit si le calcul a bien suivi les mesures, pas si le profil est bon : un profil calculé reste « à vérifier » (« Profil calculé, à vérifier ») jusqu'à la vérification de profil.
_Éviter_ : qualité du profil, vérification

**Contrôle de cohérence** :
Vérification, avant un calcul de profil, que les mesures correspondent aux plages de la mire : dégradé gris du foncé au clair, blanc papier et noir aux bonnes plages, plages répétées mesurées pareil, chaque plage proche de ses voisines. S'il signale une plage, le calcul est refusé, avec les plages en cause et, si un décalage de lecture l'explique, ce décalage ; rien n'est corrigé. Son résultat est gardé dans la provenance du profil. Les seuils sont des choix, à vérifier sur des mesures réelles.
_Éviter_ : validation des mesures, correction des mesures

### Comparaison de profils

**Gamut** :
Ensemble des couleurs qu'une condition d'impression peut produire, tel que son profil ICC le décrit. Il se lit dans la table colorimétrique du profil, en colorimétrie absolue : le blanc du papier est compris.
_Éviter_ : espace couleur, gamme

**Surface brute** :
Forme du gamut obtenue en passant les six faces du cube RVB par le profil. Elle est calculée sans ArgyllCMS, mais se replie là où la couleur fait demi-tour (bleus et violets sombres).
_Éviter_ : enveloppe, coque

**Limite du gamut** :
Enveloppe extérieure du gamut, calculée par ArgyllCMS (`iccgamut`) : une forme propre, sans replis. C'est la forme montrée par défaut ; sans ArgyllCMS ou sans `iccgamut`, la surface brute la remplace, avec la raison.
_Éviter_ : frontière, contour

**Volume du gamut** :
Taille du gamut en unités L\*a\*b\* au cube. Il dit l'étendue des couleurs d'un profil, pas sa qualité : un grand volume n'est pas un meilleur profil.
_Éviter_ : score, note

**Coupe à clarté constante** :
Tranche du gamut à une clarté L\* donnée, vue dans le plan a\*b\* : la chroma atteinte dans chaque teinte à cette clarté. C'est la section du gamut par cette clarté, lue tous les 2° : une valeur approchée.
_Éviter_ : section, slice

**Chroma par teinte** :
Chroma la plus forte atteinte par un profil dans chaque teinte, toutes clartés confondues. Valeur approchée, par tranches de 2°.
_Éviter_ : saturation par teinte

**Couleurs de base** :
Les huit coins du cube RVB passés par le profil : blanc, noir, rouge, vert, bleu, cyan, magenta et jaune.
_Éviter_ : primaires seules, coins

**Table inverse** :
Table B2A1 d'un profil de sortie : d'une couleur demandée (L\*a\*b\*), elle donne les valeurs RVB à envoyer à l'imprimante, en colorimétrique relatif. Une couleur hors du gamut y est ramenée sur sa limite. Un profil sans table inverse lisible se compare quand même, mais ne dit pas ce qu'il ferait d'une image.
_Éviter_ : table B2A, LUT inverse

**Couleur demandée** :
Couleur qu'un pixel d'une photo demande, lue par le profil de l'image (sRVB si la photo n'en a pas), en L\*a\*b\* relatif D50 : le blanc de l'image vaut L\* = 100. C'est elle que l'imprimante doit rendre ; la couleur reproduite dit ce qu'elle rendra.
Dans le code : `ProfilSource::demande`.
_Éviter_ : couleur d'origine, couleur source

**Couleur reproduite** :
Couleur que l'imprimante rend d'une couleur demandée : la couleur passée par la table inverse, puis par la table colorimétrique (aller-retour B2A1 puis A2B1, en colorimétrique relatif). L'écart ΔE00 entre les deux dit si la couleur est dans le gamut.
_Éviter_ : couleur simulée, épreuve

**Profil de l'image** :
Profil ICC embarqué dans une photo, qui dit quelle couleur chaque pixel demande (la couleur demandée, en L\*a\*b\* relatif). Une photo sans profil est lue en sRVB. Un profil embarqué illisible ou en CMJN est refusé avec sa raison, jamais remplacé en silence.
Dans le code : `ProfilSource` (le « profil source » de l'ICC).
_Éviter_ : profil d'entrée

**Nuage de couleurs** :
Les couleurs d'une photo placées dans l'espace L\*a\*b\*, regroupées par petits cubes de 4 unités : au plus 4 000 points, les plus peuplés d'abord. Chaque point est placé sur le blanc du papier du profil choisi et garde le plus grand écart de ses pixels.
_Éviter_ : nuage de points, histogramme 3D

**Pastille hors gamut** :
Sous la photo comparée, deux carrés côte à côte : à gauche une couleur de la photo (couleur demandée), à droite la couleur que l'imprimante en rendra (couleur reproduite) avec le profil choisi, puis leur écart en ΔE00 et la part de la photo. Chaque pastille représente un petit cube du nuage par sa couleur la plus fréquente ; l'écran en montre au plus 12, celles au-delà du seuil d'alerte, les plus présentes d'abord. Couleurs approchées à l'écran.
Dans le code : `Pastille`, `Paire`.
_Éviter_ : échantillon, swatch, vignette

**Compensation du point noir** :
Réglage d'impression qui ramène le noir de la photo sur le noir le plus profond que le papier peut rendre, en éclaircissant un peu toutes les ombres, sans toucher au blanc. Sans elle, un papier dont le noir est moins profond que celui de la photo « boucherait » les ombres, et la comparaison signalerait à tort toutes les ombres hors gamut. Elle est cochée par défaut, comme dans un pilote d'impression en colorimétrique relatif. La couleur obtenue est la couleur visée : c'est elle que l'imprimante doit rendre.
Dans le code : `Rendu::PointNoir`, `PointNoir`.
_Éviter_ : BPC, compensation du noir

**Pipette** :
Un clic sur la photo comparée montre la couleur du point touché et sa couleur imprimée, en pastilles, avec leurs L\*a\*b\*, l'écart et s'il dépasse le seuil d'alerte. Le point est marqué sur la photo. Calcul fait à la demande sur l'image d'analyse.
Dans le code : `paire_couleur`, commande `image_pixel`.
_Éviter_ : compte-gouttes, sélecteur de couleur

**Image d'analyse** :
La photo ramenée à 2 048 pixels de côté au plus, en prenant un pixel sur N dans chaque sens, sans mélanger les voisins : c'est sur elle que se calculent les écarts, et l'écran dit « 1 pixel sur N » quand elle est réduite. Elle n'existe qu'en mémoire, le temps de la séance ; la photo n'est jamais rangée ni copiée.
_Éviter_ : vignette, aperçu, image réduite

**Seuil d'alerte** :
Écart ΔE00 entre couleur demandée et couleur reproduite au-delà duquel un pixel est signalé en vert fluo sur la photo : 2,0 au départ, réglable de 0,5 à 10 par pas de 0,5, sans relancer le calcul. Quel que soit le seuil, les écarts se classent aussi en léger (2 à 5), moyen (5 à 10) et fort (plus de 10). Un pixel dont l'écart ne se calcule pas est dit « inconnu », jamais compté comme bon. À ne pas confondre avec l'écart accepté d'une mesure, qui donne un verdict.
_Éviter_ : tolérance, seuil hors gamut

**Famille de teinte** :
Pour les teintes touchées d'une photo : rouges, jaunes, verts, cyans, bleus ou magentas, chaque famille centrée sur la teinte de la couleur sRVB pure du même nom et bornée à mi-chemin des voisines ; une couleur de chroma inférieure à 5 est un gris. À ne pas confondre avec la famille d'une zone du cube RVB dans les écarts entre profils.
Dans le code : `familles_teinte`.
_Éviter_ : secteur de teinte

**Profil de référence** :
Dans une comparaison, le premier profil choisi : les écarts des autres profils se calculent par rapport à lui. Dans un avant / après, c'est le profil actif. À ne pas confondre avec la référence d'un contrôle d'impression.
_Éviter_ : profil maître, profil de base

**Fidélité d'un profil** :
Écart ΔE00 entre ce que le profil prédit et les mesures qu'il embarque, plage par plage, recalculé par myiro-libre pour tout profil, d'où qu'il vienne. Elle se mesure sur les plages qui ont servi au calcul : elle avantage le profil qui les suit au plus près, et ne dit pas lequel prédit le mieux un nouveau tirage (c'est la vérification de profil). Sans mesures embarquées lisibles, elle est inconnue, avec la raison.
_Éviter_ : précision du calcul (donnée par ArgyllCMS au calcul), qualité

**Répétabilité** :
Écart ΔE00 entre les plages répétées d'une même mire (mêmes valeurs RVB imprimées et mesurées plusieurs fois). Elle dit le bruit de la mesure et de la feuille, pas le profil. Sans plage répétée, elle est inconnue.
_Éviter_ : reproductibilité, dispersion seule

**Mêmes mesures** :
Deux profils embarquent les mêmes mesures quand ils ont le même nombre de plages, les mêmes valeurs RVB une fois triées et au plus 0,1 ΔE00 d'écart plage par plage. La comparaison le dit : seul le logiciel de calcul change.
_Éviter_ : même mire, même fichier

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
L'un des cinq grands travaux proposés dans la barre du haut, dans cet ordre : Bibliothèque, Profiler, Mesurer, Contrôler, Imprimer. Chaque tâche a sa vue au centre ; une tâche pas encore livrée montre une carte « Prévu ».
_Éviter_ : onglet, module, mode

**Bibliothèque** :
Base unique sur le poste, pour un utilisateur, où sont rangées les conditions d'impression et leurs mesures, chacune rattachée à l'instrument qui l'a produite (ADR 0001). Elle conserve les mesures telles que le pont les a données, sans rien recalculer. La zone gauche de l'application en montre l'arbre imprimante → papier → résolution ; les conditions d'avant le parc y sont « non classées ».
_Éviter_ : base de données (au sens technique), dossier de mesures, catalogue

**Pastille de l'instrument** :
Petit rond de couleur dans la barre du haut, devant le nom et l'état de l'instrument actif : non détecté, connecté, étalonnage requis, étalonné, en mesure. Le texte dit toujours l'état : la couleur n'est jamais seule. Un clic ouvre la liste des instruments du poste.
_Éviter_ : voyant, LED, indicateur

**Barre d'avancement** :
Barre qui montre où en est le calcul d'un profil : chaque étape (préparation, parties du calcul de colprof, contrôle de précision) en occupe une part égale, et elle ne recule jamais. Dans une étape, elle n'avance que d'un pourcentage écrit par ArgyllCMS ; une lumière spectrale qui la traverse dit que le calcul travaille. Dessous : l'étape, le temps écoulé et, s'il y a eu un calcul avec la même recette, le temps restant « estimé ».
_Éviter_ : jauge, pourcentage global, temps restant (sans « estimé »)

**À vérifier** :
État d'un profil qui n'a pas encore passé la vérification de profil : « Profil calculé, à vérifier » pour un profil calculé par myiro-libre, « Profil importé, à vérifier » pour un profil rangé tel quel. L'état enregistré reste `non_verifie`.
_Éviter_ : non vérifié, non validé

**Ranger un profil existant** :
Mettre dans une résolution de la bibliothèque un profil ICC RVB ou CMJN déjà fait ailleurs (fichier `.icc` ou `.icm`). Il devient le profil actif, sous le nom proposé (imprimante, papier, résolution, date) ; sa provenance dit « profil existant », et l'instrument et la mire qui l'ont produit restent « inconnus ».
_Éviter_ : importer un profil (« importer » est réservé aux mesures CGATS), ajouter un profil

**Installer dans Windows** :
Copier le profil actif d'une résolution dans le dossier des couleurs de Windows, pour que le pilote de l'imprimante et les logiciels le trouvent. Un autre profil du même nom, au contenu différent, n'est jamais écrasé ; si Windows refuse, on exporte le profil et on l'installe depuis l'Explorateur.
_Éviter_ : déployer, enregistrer le profil

**Sauvegarde** :
Copie complète de la bibliothèque en un seul fichier, à garder en lieu sûr ou à restaurer sur un autre poste. La restaurer remplace toute la bibliothèque, après accord de l'opérateur.
_Éviter_ : backup, archive (réservé à `Archivage/`), export (réservé à CGATS)

**Bibliothèque d'essai** :
Copie de la bibliothèque, ouverte au lancement par `--bibliotheque <dossier>` à la place de celle du poste, pour essayer une version de l'application sans toucher à la vraie ; le titre de la fenêtre le dit. Sans dossier, avec l'option répétée ou mal écrite, ou avec le dossier de la vraie bibliothèque, l'application ne démarre pas. `--demo` l'emporte sur elle. Les réglages du poste et le dossier des mires restent les vrais.
_Éviter_ : bibliothèque de test, bac à sable, bibliothèque de démonstration (réservé à `--demo` et ses données fictives)

**État vide** :
Ce qu'affiche une tâche qui n'a encore rien à montrer : une phrase et une seule action (« Aucune mesure. Mesurer une couleur »).
_Éviter_ : écran blanc, placeholder
