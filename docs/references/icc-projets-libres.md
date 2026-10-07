# Projets libres utiles ou comparables, et pilotes Konica Minolta

Date de recherche : 7 octobre 2026. Les licences et dates viennent de l'interface publique de GitHub (champ « license » et date du dernier envoi) lue ce jour ; « dernier envoi » veut dire dernier dépôt de code sur la branche, pas dernière version publiée.

## Résumé

- Pour le calcul, tout ce qu'il faut existe en libre et entretenu : ArgyllCMS (profils), Little CMS (moteur ICC), colour-science (calculs de référence), iccDEV (outils ICC).
- Pour la lecture/écriture CGATS, la seule caisse Rust trouvée (`cgats`, GPL-3.0) est **archivée** ; un petit lecteur maison est plus prudent.
- Pour le contrôle d'impression (ISO 12647-7), un seul projet libre trouvé : ValiProof (GPL-3.0, 2009, Python), ancien et fondé sur l'ancien ΔE76.
- Pour les instruments Konica Minolta : **aucun pilote libre ni documentation publique du protocole** du MYIRO-1, du FD-9, du FD-5BT ni des CM-25/26/700 n'a été trouvé. Seuls existent de petits projets pour d'autres appareils de la marque (CL-200A, CA-410) et un analyseur de fichiers CSV du CM-26d.
- ArgyllCMS ne gère aucun instrument Konica Minolta ; des utilisateurs du MYIRO-1 passent par l'export CGATS du logiciel du fabricant.
- Un pilote libre complet suppose donc de la rétro-ingénierie (déjà prévue en phase 9, ADR 0006) ; rien n'est présumé ici sur la facilité de l'entreprise.

## 1. Moteurs, bibliothèques et outils

| Projet | Licence | Dernière activité | Pertinence |
|---|---|---|---|
| [ArgyllCMS](https://www.argyllcms.com) | AGPL-3.0 (+ bibliothèques permissives) | version 3.5.0 du 4 février 2026 | moteur de profilage en processus externe (ADR 0002) ; pas d'instrument Konica Minolta |
| [Little CMS](https://github.com/mm2/Little-CMS) | MIT | 5 octobre 2026 | moteur ICC (v2 et v4), lecture du format IT8/CGATS, ΔE00 |
| [`lcms2` (Rust)](https://github.com/kornelski/rust-lcms2) | MIT | 26 août 2026 | interface Rust vers Little CMS |
| [`moxcms`](https://github.com/awxkee/moxcms) | BSD-3-Clause d'après GitHub (le fichier `icc.md` disait « à vérifier ») | 29 septembre 2026 | ICC en Rust pur, CMJN, Lab ; à comparer avec `lcms2` |
| [iccDEV (ex DemoIccMAX)](https://github.com/InternationalColorConsortium/iccDEV) | BSD-3-Clause | 7 octobre 2026 | outils et bibliothèques de l'ICC pour inspecter et manipuler des profils |
| [colour-science](https://github.com/colour-science/colour) | BSD-3-Clause | 6 octobre 2026 | calculs de référence (ΔE00, spectres), pour valider nos calculs ; pas à embarquer |
| [python-colormath](https://github.com/gtaylor/python-colormath) | BSD-3-Clause | archivé le 12 décembre 2023 | tables de densité et ΔE, mais plus entretenu : à lire, pas à employer |
| [`empfindung`](https://github.com/mina86/empfindung) | MIT | version 0.2.6, août 2026 | ΔE en Rust (voir `docs/references/icc-delta-e.md`) |
| [`deltae`](https://gitlab.com/ryanobeirne/deltae) | MIT | version 0.3.2, août 2026 | idem |
| [`cgats` (Rust)](https://github.com/ryanobeirne/cgats) | GPL-3.0 | **archivé** ; dernière version 0.2.0 (novembre 2022) | lecteur CGATS compatible avec notre licence, mais plus entretenu |

## 2. Applications comparables

- [DisplayCAL, fork Python 3](https://github.com/eoyilmaz/displaycal-py3) : licence GPL-3.0, dernier envoi le 29 juillet 2026, pas archivé. Interface libre d'ArgyllCMS pour les écrans : exemple d'application libre au-dessus d'Argyll (écrans seulement, pas l'impression). [établi]
- [ValiProof](https://valiproof.sourceforge.net/documentation.html) : « outil libre de comparaison et de validation de procédés d'épreuve » ; GPL version 3 ou ultérieure ; Python ; lit des fichiers CGATS (`.txt`, `.cgats`, `.ti3`) issus de la mire de 1 617 plages ; critères : moyenne et 95e centile, bord de gamut, blanc du papier, Media Wedge, aplats, gain de point ; version 0.2.0 révisée le 21 octobre 2009 ; l'auteur la dit « en cours de travail ». Les valeurs numériques de tolérance ne sont pas détaillées dans sa documentation. [établi] Sert de comparaison, pas de socle.
- [spectro-rs](https://github.com/Tinnci/spectro-rs) : pilote Rust pour les spectromètres ColorMunki (mesure, XYZ, Lab, ΔE) ; licence non déclarée sur GitHub (« NOASSERTION ») ; 2 étoiles ; dernier envoi le 24 janvier 2026. Exemple d'un pilote libre Rust pour un spectromètre, d'une maturité inconnue. [établi pour les faits GitHub ; qualité : à vérifier]
- [spectackler](https://github.com/octopode/spectackler) : interfaces libres pour spectrophotomètres de laboratoire dont les protocoles ont été retrouvés en écoutant les échanges série. Un exemple de méthode, pas de notre domaine. [établi par une recherche, non vérifié sur le dépôt]
- Documentation de l'ancien projet GNOME Color Manager : spécifications de colorimètres retrouvées par écoute USB (https://wiki.gnome.org/Attic/GnomeColorManager/HardwareSpecs). Montre qu'il est possible de documenter un protocole USB à partir des échanges d'un pilote Windows. [établi]
- Une discussion de 2007 de la liste OpenICC rappelle qu'un fabricant (ColorVision, Spyder) refusait de fournir pilotes et SDK à des projets libres : cas d'école de risque juridique et technique (https://lists.freedesktop.org/archives/openicc/2007q4/001027.html). [établi pour la teneur ; ancien]

## 3. Konica Minolta : pilote libre ou protocole public ?

Recherches faites : recherche web sur les modèles MYIRO-1, FD-9, FD-5BT, CM-25, CM-26, CM-700 ; interrogation de l'interface de recherche de GitHub avec les mots « myiro », « konica minolta spectrophotometer », « cm-700d », « FDXSDK », « FD-9 » ; page des instruments d'ArgyllCMS.

- Dépôts GitHub trouvés (aucun n'est un pilote du MYIRO-1, du FD-9, du FD-5BT ni du CM-700) : [établi]
  - [pykonica](https://github.com/kanazawabruno/pykonica) : MIT, dernier envoi le 21 mars 2026, 10 commits ; contrôle le CL-200A (luxmètre) et le CA-410 (analyseur d'écran) par liaison série ou USB-série ; ne cite aucune documentation officielle de protocole.
  - [Konica-Minolta-CM26dm-parser](https://github.com/nimafo/Konica-Minolta-CM26dm-parser) : MIT, dernier envoi le 20 mai 2026 ; ce n'est **pas un pilote** : il lit des exports CSV du CM-26dG.
  - [cl200a_controller](https://github.com/MasashiSode/cl200a_controller) : MIT, 2023 ; CL-200A. Un projet MATLAB pour le CS-2000 date de 2012 (sans licence).
- ArgyllCMS : sa page des instruments ne mentionne pas Konica Minolta ; un fil du forum de photographes confirme que le MYIRO-1 n'est pas géré et recommande l'export CGATS du logiciel du fabricant. [établi] (https://www.argyllcms.com/doc/instruments.html ; https://photopxl.com/?p=35970)
- Fabricant : des logiciels sont proposés au téléchargement par Konica Minolta (MYIROtools, SpectraMagic NX, FD-S2w pour les propriétaires de FD-5BT) ; aucun SDK public ni spécification de protocole USB trouvés. [établi pour les logiciels ; absence du SDK et du protocole : non prouvée, seulement non trouvée]
- Conclusion factuelle : **non trouvé**. Cela ne prouve pas l'inexistence d'une documentation (forums fermés, dépôts sous d'autres noms). [à vérifier par une recherche plus fine, par exemple sur les forums de profilage]

## 4. Ce qu'on en tire pour myiro-libre

- Pour les instruments : rien de réutilisable en libre ; la voie reste le SDK du fabricant derrière nos ponts (ADR 0005 et 0006), puis, si possible, un pilote libre issu d'écoutes USB (phase 9).
- Pour le calcul : Little CMS (MIT) et Argyll (processus externe) ; `moxcms` à tester ; `colour-science` pour valider.
- Pour CGATS et ΔE : petits modules maison, validés sur les jeux publics (voir les notes `icc-cgats-argyll-formats.md` et `icc-delta-e.md`).
- Pour le contrôle d'épreuve : s'inspirer des critères de ValiProof, mais en ΔE00 (voir `icc-12647-controle.md`).
- Licences : GPL-3.0 (la nôtre) accepte MIT, BSD et la GPL-3.0 ; l'AGPL-3.0 d'Argyll se combine par processus externe (voir `icc-argyll-options.md`).

## 5. Questions ouvertes

- Licence réelle de `moxcms` en double licence ? GitHub dit BSD-3-Clause : vérifier le fichier de la caisse sur crates.io. [à vérifier]
- Licence et fiabilité de `spectro-rs` (aucune licence déclarée) : ne pas copier de code sans l'accord de l'auteur. [à vérifier]
- Existe-t-il, sur des forums de métier ou des archives, un document de protocole du FD-9 ou du MYIRO-1 ? [à vérifier]
- Les CM-25 et CM-26 emploient-ils un protocole proche (séries CM de la marque) du MYIRO-1 ? Aucune source. [à vérifier]
- Dépôts GitHub sous d'autres noms (FDX, FD9) : l'interface de recherche n'a rien donné ; la recherche de code de GitHub n'a pas été faite (elle demande un compte).

## Sources consultées

- https://api.github.com/repos/colour-science/colour
- https://api.github.com/repos/eoyilmaz/displaycal-py3
- https://api.github.com/repos/InternationalColorConsortium/iccDEV
- https://api.github.com/repos/awxkee/moxcms
- https://api.github.com/repos/mm2/Little-CMS
- https://api.github.com/repos/Tinnci/spectro-rs
- https://api.github.com/repos/ryanobeirne/cgats
- https://api.github.com/repos/kornelski/rust-lcms2
- https://api.github.com/search/repositories?q=konica+minolta+spectrophotometer
- https://api.github.com/search/repositories?q=myiro
- https://api.github.com/search/repositories?q=konica+minolta+cm-700d
- https://api.github.com/search/repositories?q=konica+minolta+cs-2000+OR+ca-410+OR+cm-25
- https://github.com/kanazawabruno/pykonica
- https://github.com/nimafo/Konica-Minolta-CM26dm-parser
- https://valiproof.sourceforge.net/documentation.html
- https://www.argyllcms.com/doc/instruments.html
- https://photopxl.com/?p=35970
- https://wiki.gnome.org/Attic/GnomeColorManager/HardwareSpecs
- https://lists.freedesktop.org/archives/openicc/2007q4/001027.html
- https://www.myiro.com/downloads (page des téléchargements du fabricant, vue dans les résultats de recherche)
