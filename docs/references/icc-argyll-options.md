# ArgyllCMS : options des outils de profilage d'impression, et licence

Date de recherche : 7 octobre 2026. Source principale : la documentation officielle (https://www.argyllcms.com/doc/ArgyllDoc.html). Version courante annoncée sur le site : 3.5.0 du 4 février 2026. Les options sont lues dans les pages de la version courante ; sur une version plus ancienne installée, `outil -?` fait foi.

## Résumé

- Chaîne complète pour une imprimante CMJN : `targen` (liste de plages) puis `printtarg` (mise en page) puis mesure (`chartread`, remplacé chez nous par nos ponts qui écrivent le `.ti3`) puis `colprof` (profil) puis `profcheck` (contrôle). `printcal` sert à linéariser avant de profiler.
- Correction importante par rapport à une idée reçue : dans `colprof`, `-V` n'est **pas** la version ICC mais l'accentuation des zones sombres. La documentation ne propose aucune option pour choisir la version ICC : le profil est en version 2.2.0 (colorimétrique seul) ou 2.4.0 si on demande une mise en correspondance de gamut (`-s` ou `-S`). Pas de ICC v4 en sortie d'après la documentation.
- Aucun instrument Konica Minolta n'est géré par Argyll : `chartread` ne pilote pas MYIRO-1 ni FD-9. Nous écrirons nous-mêmes le `.ti3`.
- Licence : AGPL-3.0 (avec quelques bibliothèques sous licences plus permissives). Livrer l'outil avec notre application, en processus externe, est possible si nous fournissons son code source et ses mentions de licence ; notre application étant en GPL-3.0, il n'y a pas de conflit de principe (analyse juridique à confirmer, voir section 4).

## 1. Tableau commande vers effet

### targen (https://www.argyllcms.com/doc/targen.html) [établi]

| Option | Effet |
|---|---|
| `-d n` | espace de couleur de l'appareil ; 4 = CMJN (défaut), autres valeurs pour gris, RVB, CMJ, N couleurs |
| `-f n` | nombre de plages « réparties » ; défaut 836 ; répartition par défaut : échantillonnage du point le plus éloigné (OFPS) |
| `-r` / `-R` / `-q` / `-Q` / `-i` / `-I` / `-t` | autres répartitions : aléatoire (appareil ou perceptuel), quasi-aléatoire, cubique centré, point éloigné incrémental |
| `-m n`, `-M n` | grille régulière (n pas par axe) : tout le cube / sa surface seulement |
| `-s n`, `-g n`, `-n n` | dégradés par encre (`-s`), dégradés gris nominaux (`-g`), pas sur l'axe neutre (`-n`, avec profil de préconditionnement) |
| `-e n`, `-B n` | nombre de plages blanches (défaut 4) et noires (défaut 4 pour gris et RVB, 0 sinon) |
| `-l n` | limite d'encre totale en % (p. ex. 260, 300) ; sans valeur, estimée du profil donné par `-c` |
| `-c profil` | profil de préconditionnement pour répartir les plages selon la perception |
| `-N x`, `-V x` | concentration de plages sur l'axe neutre (défaut 0,5) / sur les sombres |
| `-A x`, `-G` | degré d'adaptation de l'OFPS ; version « bonne » plutôt que rapide |
| `-w` | fichier de visualisation 3D du plan Lab |
| Remarque de la page | selon le type d'appareil (jet d'encre, épreuve chimique, presse), de 300 à plus de 3 000 plages |

Exemple de la documentation : `targen -v -d4 -l260 -f1053 NomImprimante`.

### printtarg (https://www.argyllcms.com/doc/printtarg.html) [établi]

| Option | Effet |
|---|---|
| sortie | PostScript (défaut), EPS (`-e`), TIFF 8 bits (`-t`) ou 16 bits (`-T`), plus un `.ti2` (valeurs et positions) et un `.cht` (reconnaissance) avec `-s`/`-S` |
| `-p taille` | format de page : A4, A3, Letter, Legal, 4x6, 11x17... ou dimensions `LxH` en mm |
| `-m`, `-M` | marge (6 mm par défaut) ; `-M` la compte dans l'image TIFF |
| `-a x`, `-A x` | échelle des plages et des espaceurs / des espaceurs seulement |
| `-i code` | instrument visé : `i1` (défaut), `3p`, `CM`, `20`, `22`, `41`, `51`, `SS` (**aucun code Konica Minolta**) |
| `-r` | pas d'ordre aléatoire des plages (garde l'ordre du `.ti1`) |
| `-c`, `-b`, `-n` | espaceurs en couleur (défaut), en noir et blanc, ou aucun |
| `-K`, `-I` | applique un fichier de calibration `.cal` aux valeurs / l'inclut sans l'appliquer |
| `-R n` | graine aléatoire pour retrouver la même disposition |
| `-w`, `-k`, `-o` | encodage du blanc, du noir et du CMJ (PostScript) |
| `-x`, `-y` | motifs d'étiquettes de bandes et de plages |

### chartread (https://www.argyllcms.com/doc/chartread.html) [établi par résumé]

- Lit la mire avec un instrument géré et écrit le `.ti3`. `-F` filtre (M0 aucun, M1 D50, M2 UV cut, M3 polarisé) ; `-H` haute résolution spectrale ; `-Q` observateur ; `-p` lecture plage par plage ; `-r` reprise d'une lecture partielle ; `-x` saisie manuelle de valeurs XYZ ou Lab ; `-T` tolérance de cohérence des plages ; `-N` pas d'étalonnage initial. Inutilisable avec nos instruments, mais le `.ti3` produit sert de modèle.

### colprof (https://www.argyllcms.com/doc/colprof.html) [établi]

Dans les tableaux, la barre oblique « / » sépare les lettres possibles d'une option (on tape une seule lettre).

| Option | Effet |
|---|---|
| `-q l/m/h/u` | qualité (finesse des tables) : basse, moyenne (défaut), haute, ultra |
| `-b l/m/h/u/n` | qualité de la table inverse B2A seule ; `-bn` la supprime |
| `-k z/h/x/r` | génération du noir CMJN : minimum (`z`), 0,5 (`h`), maximum (`x`), rampe linéaire (`r`) (le défaut diffère selon la lecture de la page : `h` ou `r`, à vérifier avec `colprof -?`) ; `-k p ...` courbe personnalisée ; `-K` : mêmes réglages en proportion du maximum |
| `-l n` | limite d'encre totale en % (0 à 400, défaut pris dans le `.ti3` ; 220 à 300 usuel) |
| `-L n` | limite du canal noir en % (0 à 100 ; 95 à 99 usuel pour une presse ; peut ralentir) |
| `-r x` | écart moyen appareil plus instrument, en % (0,5 par défaut) : plus c'est grand, plus les données sont lissées |
| `-V x` | accentuation des zones sombres (1,0 à 3,0, défaut 1,0) |
| `-a l/x/X/g/s/m/G/S` | structure du profil : table Lab (`l`, défaut), table XYZ (`x`), matrice (écrans)... |
| `-s src/%`, `-S src/%` | mise en correspondance de gamut vers les tables perceptuelle (`-s`) ou perceptuelle et saturation (`-S`), à partir d'un profil source ou d'un pourcentage ; sans cela le profil est colorimétrique seul |
| `-c`, `-d` | conditions de vision source et destination pour cette mise en correspondance (par exemple `pp`, `mt`) |
| `-t`, `-T` | intention de la mise en correspondance (perceptuelle / saturation) |
| `-Z p/r/s/a` | intention par défaut du profil : perceptuelle, relative, saturation, absolue ; `-Z tmnb` règle les attributs (transparence, mat, négatif, noir et blanc) |
| `-f [illum]`, `-i illum`, `-o obs` | compensation des azurants (spectral seulement) ; illuminant du calcul (D50 par défaut) ; observateur (2° 1931 par défaut) |
| `-u`, `-ua`, `-uc` | pour profils d'entrée : mise à l'échelle du blanc, absolu, écrêtage |
| `-nc` | ne garde pas les données de test dans le profil (empêche de recalculer la limite d'encre plus tard) |
| `-Y c:fichier.cal` | ajoute ou remplace la calibration pour le calcul de la limite d'encre |
| `-A`, `-M`, `-D`, `-C`, `-O` | fabricant, modèle, description, copyright ; nom du fichier de sortie |
| `-P` | graphiques 3D de diagnostic des mises en correspondance |
| `-v` | bavard |

Exemple de la documentation : `colprof -v -D"Printer B" -qm -S sRGB.icm -cmt -dpp -kr -l290 PrinterB`.

### printcal (https://www.argyllcms.com/doc/printcal.html) [établi]

| Option | Effet |
|---|---|
| `-i` | calibration initiale : fixe les cibles et crée le premier `.cal` |
| `-r` | recalibration : ramène l'appareil à l'état de la calibration d'origine |
| `-e` | vérification : contrôle que la mire imprimée suit la calibration attendue |
| `-I` | mode imitation : le comportement réel de l'appareil devient la cible absolue |
| `-s x`, `-z n` | lissage des courbes (défaut 1,0) ; résolution des courbes (défaut 256) |
| `-t`, `-n`, `-m`, `-x` | forme de la cible de linéarisation à 50 %, ΔE minimal d'émulation, marge de maximum automatique, maximum imposé |
| `-p`, `-w`, `-v` | graphiques, vues 3D, bavardage |
| `-a` | produit aussi un fichier de courbes `.AMP` pour Photoshop |
| `-d` | essai sans écrire de fichier |

Déroulement type d'après la page : `-i` au départ, `-r` périodiquement, `-e` pour vérifier.

### profcheck (https://www.argyllcms.com/doc/profcheck.html) [établi]

- Compare ce que le profil prédit aux valeurs mesurées du `.ti3`. `-k` : écarts en CIEDE2000 ; `-c` : CIE94 ; sinon CIE76. `-v 2` : écart de chaque plage ; `-s` : tri décroissant ; `-h` : histogramme ; `-w`, `-x`, `-e`, `-m` : vue 3D ; `-P` : écrit un `.ti3` « élagué » sans les plages à trop forte erreur. La page ne décrit pas de moyenne ou de maximum globaux dans son texte : à vérifier à l'exécution.

### Autres outils utiles [établi] (https://www.argyllcms.com/doc/ArgyllDoc.html)

`scanin` (image TIFF de mire vers `.ti3`), `txt2ti3` (import d'autres formats), `spec2cie` (spectres vers XYZ et Lab, compensation des azurants), `iccgamut` (gamut d'un profil), `xicclu` (interroger un profil), `iccdump` (vider un profil en texte), `revfix` (recalculer la table B2A), `fakeread` (lecture simulée à partir d'un profil : utile pour tester sans appareil), `average`, `collink`, `cctiff`.

## 2. Ce qu'on en tire pour myiro-libre

- Offrir deux ou trois **recettes** à l'utilisateur (« jet d'encre CMJN normal », « presse offset ») qui fixent `-l`, `-L`, `-k`, `-q` ; dans l'interface, des mots simples ; l'option exacte est montrée dans un journal.
- Tester sans appareil : `fakeread` simule la lecture d'un profil ; on compare avec le `.ti3` écrit par nos ponts.
- Ordre de travail : `printcal` avant `targen` si la calibration est utilisée (le `.cal` sert à `targen -C`, `printtarg -K` et `colprof -Y`).
- ICC v4 : à ne pas promettre (voir `docs/references/icc.md` pour les bibliothèques capables de lire et écrire de la v4).
- Qualité de profil : `-q h` en production, `-q m` pour les essais.

## 3. Questions ouvertes

- Peut-on convertir un profil v2 d'Argyll en v4 avec l'outil de l'ICC (iccDEV) ou Little CMS ? [à vérifier]
- La mise en page de `printtarg -i i1` convient-elle à une lecture à la bande au MYIRO-1 ? Notre propre mire (voir le commit de la mire de comparaison) évite la question. [à vérifier]
- Les options d'une version installée plus ancienne peuvent différer : se fier à `colprof -?`.

## 4. Licence et livraison en processus externe

- ArgyllCMS est publié sous licence Affero GPL version 3 (AGPL-3.0) ; « presque tout le code source et les exécutables » sont couverts par cette licence, avec des bibliothèques intégrées sous licences permissives (TIFF, JPEG et autres). [établi] (https://www.argyllcms.com/doc/ArgyllDoc.html)
- L'auteur écrit que si l'on veut employer son code dans un produit commercial ou non compatible GPL, il faut négocier une licence commerciale ; il considère les interfaces graphiques comme des œuvres dérivées qui doivent respecter une licence compatible GPL. [établi, même page] Le texte de la licence est lié depuis cette page (`License.txt`) ; l'adresse directe donnait une erreur 404 lors de la recherche. [à vérifier]
- Notre application est en GPL-3.0 : compatible avec l'AGPL-3.0 (chaque licence autorise la combinaison avec l'autre, l'article 13 s'appliquant à l'ensemble). [probable] La FAQ officielle du projet GNU sur la GPL distingue les programmes « simplement regroupés » des programmes « combinés » : tuyaux, sockets et arguments de ligne de commande sont les moyens normaux de communication entre deux programmes séparés, sauf échange de structures de données internes très intimes. [probable] (lu via un résumé de recherche, le site gnu.org ne répondait pas : https://www.gnu.org/licenses/gpl-faq.html)
- Conséquences concrètes si nous livrons Argyll avec l'application : fournir le code source correspondant (ou une offre écrite) et le texte de la licence ; conserver les mentions des bibliothèques intégrées ; ne pas retirer les messages de copyright ; indiquer clairement qu'Argyll est un programme à part. Si un jour notre licence devenait non libre, il faudrait une licence commerciale d'Argyll ou ne plus l'embarquer (ADR 0002 sur l'inclusion d'ArgyllCMS). [probable ; point à faire valider par un juriste ou la communauté libre avant la première livraison]
- Le clause réseau de l'AGPL (article 13) vise les services accessibles à distance : une application de bureau n'est pas concernée. [probable]

## Sources consultées

- https://www.argyllcms.com/doc/ArgyllDoc.html
- https://www.argyllcms.com/doc/colprof.html
- https://www.argyllcms.com/doc/targen.html
- https://www.argyllcms.com/doc/printtarg.html
- https://www.argyllcms.com/doc/printcal.html
- https://www.argyllcms.com/doc/profcheck.html
- https://www.argyllcms.com/doc/chartread.html
- https://www.argyllcms.com/doc/Scenarios.html
- https://www.argyllcms.com/doc/instruments.html
- https://www.argyllcms.com/ (version 3.5.0)
- https://www.gnu.org/licenses/gpl-faq.html (non joignable ; contenu connu par résumé)
