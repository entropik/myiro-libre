# Poste unique : couleur et impression

Document de conception issu de l'échange du 20 octobre 2026 avec le mainteneur.
Il complète le [cadrage couleur](poste-travail-couleur.md) et en remplace
l'interface. Les décisions du mainteneur sont marquées **(décidé)** ; les choix
de l'assistant, que le mainteneur peut contester, sont marqués **(proposé)**.

## Pourquoi

L'imprimerie travaille aujourd'hui avec quatre logiciels qui ne se parlent pas :
le logiciel de mesure et de profilage de Konica Minolta (MYIROtools, abandonné
par le fabricant), un RIP pour les jet d'encre pigmentaires, un autre RIP pour la
HP Latex, et le flux propre à la presse Konica. Les profils se rangent
« n'importe où », les parcours demandent trop de clics, l'enregistrement d'un
profil oblige à revenir en arrière, les RIP actuels plantent, sont lourds et
traitent mal certains PDF clients en RVB (couleurs ternes, profil d'origine mal
pris en compte).

Le but est **un seul poste**, libre, qui mesure, profile, range et imprime pour
les machines jet d'encre de l'atelier, et qui devienne le socle de l'application
d'impression de l'imprimerie.

## Utilisateur et parc

- Un imprimeur, pas un développeur. Usage quotidien : mesurer des mires au FD-9
  puis faire un profil.
- 4 imprimantes, une vingtaine de papiers en service.
- Machines visées **(décidé)** : Canon imagePROGRAF PRO-4100, Canon iPF5100,
  Epson SC-P5300, HP Latex 700 W. Instruments : FD-9 (quotidien), MYIRO-1.
- Hors périmètre **(décidé)** : la presse Konica Minolta AccurioPress C83hc, qui
  garde son contrôleur Konica et son propre flux.

## Objectifs

1. **Bibliothèque unique** des profils, rangée par imprimante puis par papier.
2. **Parcours de profil court** : mire → impression → séchage → mesure → profil
   → enregistrement, sans retour en arrière.
3. **RIP « light »** pour Canon, Epson et HP : rouleau et feuille, nesting,
   repères de coupe, coupe du rouleau par l'imprimante, file d'attente, relance,
   archive.
4. **Reprise de l'existant** : profils et papiers des logiciels actuels, fichiers
   MYIROtools.
5. **Profils RVB et CMJN (décidé)** : RVB pour les Canon et l'Epson (via leur
   pilote), CMJN pour la HP Latex, profilée elle aussi au FD-9 par le
   mainteneur.

## Non-objectifs (décidé)

- Encre blanche de la HP Latex 700 W : jamais utilisée, non gérée.
- Découpe à la forme et plotter de découpe : non gérés. Seules la coupe du
  rouleau par l'imprimante et les repères de coupe imprimés sont prévus.
- Presse Konica et son contrôleur.
- Étude du cas PDF « couleurs ternes » maintenant : l'application d'abord.
- Licences, clés de protection et activation des logiciels d'origine : rien à
  recréer ni à contourner.

## Anti-objectifs : échec même si « ça marche »

- Retrouver la lourdeur des RIP actuels : dizaines de modules, réglages partout,
  vocabulaire de technicien.
- Un profil ou un travail qu'on ne retrouve plus, ou qui se range « au hasard ».
- Un PDF dont le profil d'origine est ignoré sans que l'écran le dise.
- Plusieurs fenêtres ou applications pour une même tâche.
- Copier le code, les textes, les icônes, les mires ou les modèles des
  logiciels d'origine : on recrée le comportement avec notre propre code et des
  normes publiques.

## Forme du produit

### Une application, plusieurs moteurs (proposé, accepté sur le principe)

- Pour l'utilisateur : une fenêtre, une bibliothèque, une file d'impression.
- À l'intérieur : un cœur commun (calculs couleur, profils, formats, conversion
  d'images) et des modules séparés, un par instrument et un par famille de
  machines, ajoutés étape par étape. Les instruments fonctionnent déjà ainsi
  (ponts `pont-myiro1`, `pont-fd9`).
- Conséquence : l'[ADR 0004](../adr/0004-crates-partagees-rip.md), qui faisait
  du RIP un projet séparé, est à remplacer par un ADR « poste unique ».

### Ce qu'on garde et ce qu'on refait

- **Gardé (proposé)** : Rust et Tauri, les ponts FD-9 et MYIRO-1, la crate
  `colorimetrie`, ArgyllCMS pour le calcul des profils
  ([ADR 0002](../adr/0002-argyllcms-inclus.md)), les règles de sécurité
  matérielle. Qt n'est pas retenu : l'interface Tauri existe déjà.
- **Refait (décidé)** : toute l'interface, en partant des besoins ci-dessous,
  de l'organisation fonctionnelle de MYIROtools et, pour la présentation, d'un
  logiciel concurrent basé sur ArgyllCMS, inspiration principale du mainteneur
  (voir ci-dessous) ; pas de l'interface actuelle du dépôt.

### Inspiration d'interface (décidé)

Un logiciel commercial concurrent, basé sur ArgyllCMS, est la référence
d'interface la plus forte du mainteneur. Son analyse et un guide d'interface
détaillé existent en local (documents de tiers, non versionnés ; le
constructeur les demande au mainteneur). On en reprend des **principes
d'usage**, jamais la présentation exacte, les icônes, les couleurs de marque,
les textes ni les captures.

Principes retenus :

- **Cadre permanent à trois zones** : bibliothèque à gauche, zone de travail au
  centre, détails de la sélection à droite, identique d'une tâche à l'autre.
- **Barre du haut par tâche** : Bibliothèque, Profiler, Mesurer, Contrôler,
  Imprimer ; à droite, l'instrument avec une pastille d'état (non détecté,
  connecté, étalonnage requis, étalonné, en mesure).
- **Assistants à étapes numérotées** avec fil d'Ariane cliquable résumant les
  choix déjà faits ; revenir en arrière est possible, jamais obligatoire.
- **Verdict en clair, détail à la demande** : conforme / proche de la limite /
  hors tolérance, toujours couleur + icône + mot, marge affichée.
- **Le spectre d'abord, provenance visible** : chaque valeur dit d'où elle
  vient ; « inconnu » écrit en toutes lettres, jamais zéro.
- **Transparence des calculs** : pour chaque appel ArgyllCMS, une section
  « Avancé » montre la commande équivalente en lecture seule.
- **Un seul bouton principal par écran**, libellé par un verbe précis ; tout
  bouton inactif dit pourquoi ; rouge réservé au danger et au hors tolérance.
- **Sections repliables qui résument leurs valeurs** quand elles sont fermées ;
  cartes illustrées pour les choix importants (RVB ou CMJN, choix de mire).
- **Graphiques lisibles** : axes, unités, légende, lecture au survol ; gamut
  avec coupe 2D en plus de la vue 3D.
- **Atelier d'abord** : thème clair et sombre, grandes cibles, hors ligne, sans
  compte ni télémétrie, français et anglais sans mélange.

Un prototype cliquable jetable (thème bleu nuit avec bascule clair/sombre,
cadre à trois zones, cinq tâches, parcours de profil, écran Impression) a été
validé comme base visuelle le 20 octobre 2026, à affiner ensuite. Il reste
local (`.superpowers/`, hors dépôt). **(décidé)** Ce prototype devient la
référence graphique : le `design-system/` du dépôt (jetons papier, filets,
grille suisse) est refait à partir de lui (thème bleu nuit avec bascule
clair/sombre, cartes, accent bleu, verdicts vert / orange / rouge).

Ce que la conception ajoute à cette inspiration : étapes d'impression et de
séchage dans le parcours, vérification systématique du profil, bibliothèque
par imprimante × papier × résolution, écran Impression et file d'attente.

## Partie A : couleur

### Bibliothèque (décidé : présentation en liste)

- Arbre : imprimante → papiers → résolution d'impression **(décidé : un profil
  par résolution est possible pour un même papier)**. Pour chaque résolution :
  profil actuel avec sa date et son état de vérification, historique des
  profils précédents, profils anciens signalés (plus d'un an). Un papier qui
  n'a qu'une résolution s'affiche sans ce niveau.
- À l'impression, le profil est choisi par machine × support × résolution.
- Détail à droite : instrument, mire, condition de mesure, vérification ;
  actions « Installer dans Windows », « Refaire le profil », « Exporter ».
- Recherche par texte (« semigloss », « P900 »).
- Chaque profil est rangé avec tout ce qui l'a produit : mire, mesures,
  instrument, fiche d'impression (réglages du pilote), date.
- Nom fabriqué automatiquement (imprimante, papier, date), modifiable.
- Rangement géré par l'application : l'utilisateur ne choisit jamais de dossier.
- Une section « En attente de mesure » liste les mires imprimées qui sèchent.

### Parcours « nouveau profil » en 4 étapes (décidé)

1. **Mire** : imprimante, papier, instrument et taille de mire pré-remplis avec
   les derniers choix du couple ; préréglages rapide, standard, approfondi ;
   mode avancé replié.
2. **Impression** : fichier de mire prêt (sans gestion couleur) et fiche
   d'impression reprise du dernier tirage ; rappel des réglages du pilote.
3. **Mesure** : M1 par défaut (modifiable) ; plages colorées au fur et à mesure ;
   mesures gardées automatiquement.
4. **Profil** : calcul, affichage de la précision, puis nom et enregistrement en
   un clic, installation dans Windows cochée par défaut ; proposition d'imprimer
   la mire de vérification.

Dans MYIROtools, le même parcours traverse une douzaine d'écrans et demande le
nom du profil avant le calcul : c'est ce qu'il faut éviter.

### Séchage et QR code (décidé)

- Après l'impression, le travail passe « en attente de mesure » avec une
  durée de séchage réglable par papier (30 min et plus pour le jet d'encre),
  retenue pour la fois suivante. Notification Windows quand la mire est prête.
  Fermer l'application ne perd rien.
- Plusieurs mires peuvent être imprimées d'un coup puis mesurées à la suite.
- Chaque mire porte un QR code et un texte lisible (imprimante, papier, mire,
  date). Glissée dans le FD-9, elle rouvre directement le bon travail à l'étape
  Mesure. Sans QR (MYIRO-1, ancienne mire) : choix dans la liste.
- **À vérifier** : comment le FD-9 lit un QR code. Le SDK sait capturer l'image
  et reconnaître un QR ; reste à établir si un QR à notre format est accepté,
  ou s'il faut décoder nous-mêmes l'image capturée. Étude sans appel d'écriture
  à l'instrument, selon les paliers habituels.

### Reprise de l'existant (proposé)

- Fichiers de réglages et de mesures MYIROtools (format XML masqué, lisible).
- Papiers personnalisés du RIP des pigmentaires (dossiers lisibles : un XML et
  les profils ICC).
- Profils éparpillés : ramassés, classés par imprimante et papier, confirmation
  demandée en cas de doute.

## Partie B : impression

### Écran « Impression » (décidé sur le contenu, proposé sur la présentation)

Besoins exprimés à partir de la fenêtre d'impression du RIP actuel : choisir
l'imprimante, ajouter un ou plusieurs fichiers, choisir le papier, la taille de
l'image, la marge, la rotation, la position, optimiser (nesting), repères
d'impression, gestion de la couleur.

- En haut : machine, support, profil choisi automatiquement par la
  bibliothèque, modèle de réglages.
- À gauche : fichiers (glisser-déposer, nombre d'exemplaires, réglages communs
  ou par fichier), taille (taille du fichier, ajuster, remplir, perso), mise en
  place (nesting auto, centré, manuel ; écart entre images ; rotation auto si
  elle économise du support), coupe et repères ; sections rares repliées
  (marges, fond perdu, étiquette, couleur).
- Au centre : aperçu du support réel (rouleau avec ligne de coupe, ou feuille),
  longueur consommée, pourcentage de perte.
- À droite : file de la machine et archive avec bouton « Relancer ».
- « Enregistrer comme modèle » mémorise tous les réglages d'un type de travail.

### Supports

- Rouleau : largeur, travaux mis à la suite, coupe par l'imprimante après le
  travail ou à la fin.
- Feuille : formats standard (A, US) et formats perso enregistrés.

### File, relance, archive (décidé)

- Une file par machine : pause, réordonner, annuler.
- Relancer un travail archivé avec exactement les mêmes réglages.
- Archive : fichiers, réglages, date ; recherche par client, date, machine.

### Couleur des fichiers (décidé sur l'exigence)

- Le profil intégré d'un fichier (PDF, TIFF, JPEG) est toujours respecté.
- L'écran affiche le profil d'origine utilisé pour chaque fichier ; un fichier
  sans profil est signalé avec l'hypothèse appliquée.
- Le profil de sortie vient de la bibliothèque (machine × papier).

### Sorties par machine (proposé)

| Machine | Première sortie | Plus tard |
|---|---|---|
| Canon PRO-4100, iPF5100 | pilote Windows, gestion couleur du pilote désactivée, profil appliqué par l'application | sortie directe si elle apporte linéarisation ou limites d'encre |
| Epson SC-P5300 | idem | idem |
| HP Latex 700 W | à établir : aucun pilote Windows utilisé à l'atelier ; le RIP actuel la pilote en direct. Solution d'attente possible : déposer le travail préparé dans un dossier surveillé du RIP actuel | pilote direct, après étude d'interopérabilité |

## Ordre de livraison (proposé)

1. **Couleur** : bibliothèque, parcours en 4 étapes, QR et séchage, FD-9 puis
   MYIRO-1, reprise de l'existant. Cela recouvre la première livraison du
   cadrage couleur (#33, #34, #35, #40, #41) avec la nouvelle interface.
2. **CMJN et passerelles** : profilage CMJN (mires CMJN, réglages ArgyllCMS de
   génération du noir et de limite d'encre avec valeurs par défaut sensées) pour
   la HP Latex, dont les mires sont imprimées sans gestion couleur par le RIP
   actuel ; installation des profils de la bibliothèque dans les RIP actuels,
   pour ne plus mesurer et ranger qu'à un seul endroit pendant la transition.
   **(proposé : le CMJN vient juste après le RVB, pas plus tard)**
3. **Impression** : écran Impression, file, archive, modèles, sorties Canon et
   Epson par pilote Windows.
4. **HP Latex** : sortie directe après étude ; sorties directes Canon et Epson
   si utiles.

Chaque étape est utile seule.

## Contraintes

- Sécurité matérielle (`CLAUDE.md`) : jamais d'appel `FDX_JIG_*` ni d'écriture
  dans un instrument dont le contrat n'est pas établi. MYIROtools contient une
  fonction qui écrit dans la mémoire d'usine du MYIRO-1 : elle n'est pas
  recréée.
- Aucun contenu protégé dans le dépôt : DLL, mires, modèles, textes, icônes des
  logiciels d'origine. Les audits restent locaux (`Audit-MYIRO/analyse/`).
- Aucun identifiant matériel réel (numéro de série, adresse réseau) ni nom de
  client dans les fichiers versionnés.
- Documentation en français, mots simples. Licence GPL-3.0 visée.
- Vocabulaire : le [glossaire](../../GLOSSARY.md) réserve « Feuille » à la
  géométrie de lecture du FD-9 et « Tirage » à l'objet mesuré. Les termes
  d'impression (« support en feuille », « travail ») sont à y ajouter sans
  collision.

## Choix de l'assistant après relecture (proposé)

- **Notification de séchage** : envoyée tant que l'application est ouverte ;
  à la réouverture, les mires devenues prêtes sont mises en avant. Pas de
  processus résident en v1.
- **QR code non bloquant** : si la lecture par le FD-9 n'aboutit pas, la
  première livraison sort avec le choix du travail dans la liste ; le QR vient
  ensuite.
- **Base existante** : la bibliothèque SQLite actuelle ne contient qu'un essai
  (1 condition, 1 instrument, 6 mesures) ; elle est reprise par une conversion
  simple, sans contraindre le nouveau modèle.
- **Passerelles, étape 2** : pour le RIP des pigmentaires, écrire un nouveau
  dossier de papier (format lisible, exemples locaux), après sauvegarde, sans
  jamais modifier les papiers existants ; pour le RIP de la Latex, format à
  étudier dans l'installation locale avant tout dimensionnement.
- **Lecture des fichiers MYIROtools** : le masquage de leurs fichiers est levé
  uniquement pour relire les fichiers de l'utilisateur (interopérabilité) ;
  la méthode n'est pas publiée dans le dépôt tant que le mainteneur ne l'a pas
  décidé.

## Laissé au constructeur

- Découpage en crates et en processus, format de stockage de la bibliothèque et
  de l'archive, bibliothèques de lecture PDF et de conversion couleur, algorithme
  de nesting, présentation graphique fine (le `design-system/` du dépôt
  s'applique).

## Questions ouvertes

- Lecture des QR code par le FD-9 (voir plus haut).
- Protocole de sortie de la HP Latex 700 W : à établir par l'audit du RIP
  actuel (interopérabilité), sans recopier de code.
- Format de dépôt dans le dossier surveillé du RIP actuel de la Latex, pour la
  solution d'attente.
