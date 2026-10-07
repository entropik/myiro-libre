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

**Plafond** :
Dernier palier qu'un pont a le droit d'atteindre, fixé à son lancement ; toute demande au-delà est refusée sans toucher à l'instrument.
_Éviter_ : limite, niveau maximal

**Prise de main** :
Droit exclusif d'un logiciel à piloter un FD-9 en réseau ; un seul l'a à la fois. Le pont l'obtient à la connexion et la rend à la déconnexion. FD-S2w resté connecté la garde.
_Éviter_ : autorité, authority, verrou

**Armement** :
Mise en attente de l'instrument pour une mesure ponctuelle ou une bande : il attend l'appui sur son bouton. Ne règle rien de permanent dans l'instrument.
_Éviter_ : réglage, condition (au sens de `SetMeasureCondition`)

**Données brutes** :
Valeurs rendues par l'instrument avant tout calcul de spectre (152 par plage pour le MYIRO-1), conservées avec chaque mesure pour valider le pilote libre (ADR 0006).
_Éviter_ : raw, données capteur

**Provenance** :
Ce que le pont atteste sur une mesure : instrument (modèle, n° de série, micrologiciel), chaîne logicielle (SDK, pont, empreinte de la DLL), date réelle avec fuseau, géométrie de lecture, condition de mesure relue sur l'instrument, illuminant, observateur. Le reste du contexte (support, encres, séchage, chauffe…) relève de la bibliothèque, pas du pont. Elle est posée par le pont, jamais reconstituée par l'application, et aucune mesure n'existe sans elle.
_Éviter_ : métadonnées, contexte

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
