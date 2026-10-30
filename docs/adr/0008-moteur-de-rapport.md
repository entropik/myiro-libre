# Un seul moteur de rapport, en PDF, libre et indépendant de Windows

Le 30 octobre 2026, le mainteneur a demandé un **rapport client** pour une image comparée (ticket #89) : un PDF sobre, façon cartouche d'architecte, aux coordonnées de l'atelier. Le même moteur doit servir ensuite à la vérification d'un profil (#41), au contrôle et aux mesures d'un profil. Plan : [moteur de rapport](../superpowers/plans/2026-10-30-moteur-de-rapport.md).

## Décision

- **Une crate `rapport`**, sans Windows, sans Tauri, sans pont. Elle décrit un document : un cartouche (atelier, client, objet, papier, profil, date, numéro), des sections, des blocs (verdict, paragraphe, couples, tableau, images, paires de pastilles, graphique SVG, texte gardé tel quel). Elle compile et se teste aussi sous Linux (job `linux` de l'intégration continue).
- **La source Typst est la forme unique du document.** La crate l'écrit : un gabarit fixe (`gabarit.typ`), puis des appels à ses fonctions dont **chaque texte est un littéral de chaîne**, jamais du balisage. Un nom de client comme `") #panic(` reste du texte. Un test d'or fige la source d'un document qui emploie chaque bloc.
- **Le PDF est fait par Typst employé comme bibliothèque** (`typst`, `typst-layout`, `typst-pdf` 0.15.1, Apache-2.0), en mémoire. Le « monde » de Typst est fermé : la source, ses images (fichiers virtuels `/images/…`) et deux polices embarquées. Aucun fichier du disque, aucune police du système, aucun paquet téléchargé.
- **Polices** : Inter, celle de l'application, en deux instances statiques (Regular, SemiBold), tirées de `app/interface/fontes/InterVariable.woff2` par `outils/instances_inter.py`, avec la licence SIL OFL 1.1 copiée à côté (`crates/rapport/fontes/`). Elles doivent porter des noms de police statique (« Inter ») : avec les noms de la police variable, le texte des graphiques SVG disparaissait du PDF (test `les_polices_sont_nommees_inter_sans_variable`).
- **Mise en page** : A4 portrait, marges de 15 mm, marge basse de 56 mm qui porte le cartouche d'architecte **en pied de chaque page** (hauteur fixe de 36 mm, contenu rogné : il ne recouvre jamais le contenu), avec « Page n / total ». Noir et gris, filets fins, ni arrondi ni ombre. Les seules couleurs posées sont celles des pastilles, contrôlées en `#rrggbb`. Verdicts, images, paires, couples et graphiques ne se coupent pas d'une page à l'autre.
- **Graphiques venus de la page** : SVG passés par une liste blanche (`nettoyer_svg` : balises et attributs de dessin seulement ; script, lien, objet étranger, `url(`, entité inconnue refusés). Un graphique refusé est remplacé par une phrase. La vue 3D part en PNG.
- **L'application** construit le document d'une image côté Rust, depuis l'image et l'analyse gardées en mémoire (#80). Elle écrit le PDF à sa place par un temporaire renommé : un PDF ouvert ou un dossier protégé est refusé sans rien laisser derrière.

## Options écartées

| Moteur | Pourquoi écarté |
|---|---|
| HTML imprimé par WebView2 `PrintToPdf` (premier choix du général) | **Lié à Windows et au runtime de Microsoft** : décision du mainteneur, « il faut un moteur de PDF indépendant de Windows, open source ». Fenêtre cachée et attente asynchrone ; impossible à tester sous Linux. Rien n'en avait été livré. |
| `printpdf` (MIT) | Tout se place à la main (texte, coupures de ligne, tableaux) : beaucoup de code, rendu pauvre. |
| Impression manuelle « Microsoft Print to PDF » (choix d'origine de #41) | Trois gestes de plus pour l'imprimeur, marges et en-têtes à régler à chaque fois. |

## Ce que la pointe a confirmé (30 octobre 2026)

- `typst` 0.15.1 compile en bibliothèque avec un monde fermé, en **64 et en 32 bits** ; un PDF A4 de deux pages sort en 20 ms (version optimisée), quelques secondes au plus en version de mise au point.
- Le pied de page se répète sur chaque page ; `counter(page)` donne le numéro et le total.
- **Licences** des dépendances (`cargo tree`) : MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, Zlib, Unicode-3.0, BSL-1.0, 0BSD, CC0 : toutes compatibles avec la GPL-3.0. Aucune n'impose de télécharger quoi que ce soit.
- **Poids** : environ 300 crates de plus, une construction propre d'environ 2 minutes, et l'exécutable grossit d'environ 40 Mo (données de césure, Unicode et syntaxes que Typst embarque). C'est le prix d'un moteur libre et portable.

## Où vit l'atelier

Le réglage « Atelier » (coordonnées, logo, numéro du dernier rapport) est dans `<données de l'application>/atelier` : `atelier.json`, `logo.png`, `rapports.json`, chacun écrit par un temporaire renommé. Il est **hors de la bibliothèque** : aucune migration, et la sauvegarde de la bibliothèque ne l'emporte pas. En démonstration (`--demo`), il va dans le dossier temporaire de la démonstration.

- Le logo est un PNG ou un JPEG reconnu par son contenu, réduit à 600 pixels de côté et réencodé en PNG ; un SVG est refusé.
- Le numéro de rapport suit l'année : `2026-0001`, `2026-0002`…, remis à 1 chaque année. Il n'est **pris qu'une fois le PDF écrit** : un export manqué ne consomme aucun numéro.

## Aucun identifiant d'instrument

Un rapport ne porte ni numéro de série, ni adresse réseau, ni adresse MAC. Le rapport d'une image ne reprend pas la description des profils, où un logiciel peut avoir écrit un numéro de série (test `le_rapport_ne_porte_aucun_identifiant_d_instrument`).

## Branchement de #41

#41 (vérification d'un profil) a déjà son rapport HTML. Remise à niveau sur `dev` après ce ticket : renommer son module en `rapport_verification` pour ne pas cacher la crate, construire un `rapport::Document` depuis ses données (titres en sections, couples, tableaux de plages, verdict avec son mot et son symbole), garder son export HTML, et ajouter l'export PDF par `pdf::fabriquer_pdf` et `pdf::enregistrer_pdf`. Le numéro d'un rapport de vérification devra être **gardé avec elle** (son rapport doit rester identique à l'octet) : à trancher dans #41.
