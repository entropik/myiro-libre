# Conditions de mesure : M0, M1, M2, M3 (ISO 13655), azurants, illuminant, observateur

Date de recherche : 7 octobre 2026. Norme citée : ISO 13655:2009, « Graphic technology — Spectral measurement and colorimetric computation for graphic arts images » (payante ; on n'en reprend que ce que son extrait gratuit et des pages publiques en disent).

## Résumé

- Presque tous les papiers contiennent des azurants optiques (ils transforment les ultraviolets en lumière bleue). Deux instruments qui n'éclairent pas avec les mêmes ultraviolets mesurent donc deux blancs différents : d'où les conditions M0 à M3.
- M0 : éclairage proche de l'illuminant A, sans règle sur les ultraviolets. M1 : éclairage proche de D50, avec ultraviolets réalistes. M2 : sans ultraviolets (« UV cut »). M3 : comme M2 avec deux polariseurs croisés.
- Pour échanger des mesures et des profils, M1 est la condition recommandée par ArgyllCMS ; les données FOGRA récentes (FOGRA51, 52) sont en M1.
- Le MYIRO-1 est un instrument à géométrie 45°/0° annulaire ; selon le fabricant il propose M0, M1, M2 et un illuminant défini par l'utilisateur, mais pas M3.
- ArgyllCMS sait compenser les azurants à partir d'un spectre complet (option `-f` de colprof, outil spec2cie), mais pas avec un instrument qui coupe les ultraviolets.
- Dans myiro-libre : toujours enregistrer la condition de mesure relue sur l'instrument (déjà prévu dans la provenance) et ne jamais mélanger des mesures de conditions différentes sans le dire.

## 1. Ce que disent les sources

- Les quatre conditions de l'ISO 13655:2009 [établi] (introduction de l'extrait gratuit : https://cdn.standards.iteh.ai/samples/39877/fc7f9f9e2f954e279c475d6c603fd8c1/ISO-13655-2009.pdf) :
  - M0 : l'éclairage ressemble à l'illuminant A ; cela assure la continuité avec les instruments existants et avec l'ISO 5-3 (densité).
  - M1 : l'éclairage a la même couleur que l'illuminant D50 (donc des ultraviolets comparables à la lumière du jour).
  - M2 : on exige seulement le spectre de l'éclairage entre 420 nm et au moins 700 nm, et pas de puissance notable sous 400 nm (« UV cut »).
  - M3 : mêmes exigences d'éclairage que M2, plus un polariseur à l'émission et un à la réception, orientés en croix.
- Le texte explique pourquoi la norme existe : en 2009, aucun instrument vendu pour les arts graphiques n'avait un éclairage vraiment proche de D50 ; la variation d'ultraviolets entre instruments pouvait donner jusqu'à environ 5 unités de b* d'écart sur du papier azuré. [établi, même source]
- La norme contient aussi (d'après son sommaire) : un fond de mesure normalisé (annexe A, normative), une annexe sur la géométrie, une sur les échantillons fluorescents, une sur l'usage de la polarisation (annexe G) et une méthode d'essai de la coupure UV (annexe H, normative). Le contenu des annexes n'est pas public. [établi pour le sommaire seulement]
- Article du fabricant sur l'ISO 13655:2009 : M0 vient de lampes à incandescence aux ultraviolets instables ; M1 est la condition « préférée » ; M2 vise des lieux sans UV (musée) ; M3 sert l'offset pour prévoir la densité d'une feuille sèche à partir d'une feuille humide. Seules quatre géométries sont admises, dont 45° annulaire / 0° et 45° circonférentiel / 0°. Le fabricant obtient M1 par un procédé qu'il appelle « Virtual Fluorescence Standard » : deux sources d'énergies UV différentes, pour séparer la fluorescence de la réflexion. [établi] (https://sensing.konicaminolta.us/us/?p=10979)
- Résumé d'un fournisseur d'instruments (M0 à M3, ordre de grandeur des températures, polarisation pour la densité sèche/humide) : https://techkon.datacolor.com/what-are-m0-m1-m2-m3-measuring-mode-settings/ [lu via un résumé de recherche : à vérifier]

## 2. Ce que ArgyllCMS expose

- `colprof -f [illum]` : active la compensation des azurants à partir de données spectrales ; l'illuminant de l'instrument peut être M0, M1, M2, A, C, D50, D65, F5, F8, F10 ou un fichier `.sp`. `colprof -i illum` choisit l'illuminant du calcul (D50 par défaut) et `colprof -o observer` l'observateur (1931 2° par défaut ; 1964 10°, 2015 2°, 2015 10°, « shaw » ou un fichier `.cmf`). [établi] (https://www.argyllcms.com/doc/colprof.html)
- `chartread -F` choisit la configuration de filtre des instruments qui en ont (aucun = M0, D50 = M1, UV cut = M2, polarisé = M3) ; `-Q observer` fixe l'observateur ; `-H` demande la haute résolution spectrale si l'instrument la propose. [établi par un résumé de la page ; à relire dans le texte] (https://www.argyllcms.com/doc/chartread.html)
- `spotread` : `-i` illuminant de réflexion, `-I` illuminant simulé de l'instrument pour la compensation des azurants, `-Q` observateur, `-w` pour exprimer le Lab avec le blanc de l'illuminant choisi au lieu de D50, `-T` pour température de couleur, CRI et indices d'une lumière ambiante. [établi] (https://www.argyllcms.com/doc/spotread.html)
- Page sur la compensation des azurants : elle exige une mesure spectrale, le spectre de l'illuminant (avec sa part d'UV) et un instrument qui voit les UV (ni filtré, ni sans UV). Elle simule M1 (recommandé pour des profils interchangeables, D50, observateur 2°) et M2, et estime le spectre sans fluorescence par une méthode approchée qui peut se tromper sur des papiers colorés. Elle précise qu'un filtre UV n'élimine pas le problème : il empêche d'en détecter la cause. [établi] (https://www.argyllcms.com/doc/FWA.html)
- Dans le fichier de mesures `.ti3`, le mot-clé facultatif `INSTRUMENT_FILTER` prend les valeurs `POLARIZED`, `D65` ou `UVCUT` ; `TARGET_INSTRUMENT` aide le logiciel à retrouver le spectre de l'éclairage de l'instrument. [établi] (https://www.argyllcms.com/doc/ti3_format.html)
- Argyll n'a pas de pilote pour les instruments Konica Minolta ; sa page des instruments ne cite que l'Eye-One Pro comme capable de M0, M1 et M2 par calcul. [établi] (https://www.argyllcms.com/doc/instruments.html)

## 3. Conséquences pour un instrument 45°/0° comme le MYIRO-1

- Géométrie : 45°a:0° (annulaire) est une des géométries retenues par l'ISO 13655:2009. [établi] (article du fabricant ci-dessus)
- Fiche publique du MYIRO-1 : lampe LED, plage 380 à 730 nm par pas de 10 nm (36 bandes, ce qui correspond à l'exemple `SPECTRAL_BANDS 36`, 380 à 730 du format `.ti3`), tache de mesure de 3,5 mm, conditions M0, M1, M2 et illuminant utilisateur ; l'éclairage sous 400 nm est unidirectionnel. [établi par un résumé de la fiche d'un revendeur : https://chromachecker.com/manuals/en/show/konica-minolta_myiro-1_ ; à confirmer sur la fiche du fabricant]
- Comme l'instrument est à 45°/0°, il est peu sensible à la brillance du papier et n'a pas de polariseur : M3 n'existe donc pas pour lui. Densité « sèche » prévue à partir d'une mesure humide : impossible directement. [probable ; M3 absent d'après la fiche]
- M1 est obtenu par le procédé à deux niveaux d'UV du fabricant (si le MYIRO-1 l'applique comme les autres instruments de la marque : à vérifier), donc par calcul, pas par une vraie lampe D50.
- Le blanc d'étalonnage et le fond de mesure comptent autant que la condition : l'ISO 13655:2009 a un fond normalisé (annexe A). Fond noir ou blanc : à tenir dans la provenance. [établi pour l'existence de l'annexe]

## 4. Illuminant et observateur

- D50 et observateur 2° sont les réglages par défaut d'ArgyllCMS et les valeurs de référence de la chaîne ICC (l'espace de connexion de profils est en D50). [établi pour Argyll ; probable pour l'ICC, voir `docs/references/icc.md`]
- D65 sert aux écrans et à certains usages (impression textile, emballage). Les observateurs 10° et 2015 sont proposés mais Argyll prévient que les observateurs non standard réduisent l'interchangeabilité. [établi] (https://www.argyllcms.com/doc/colprof.html)
- Si l'instrument fournit des spectres, tout le reste (XYZ, Lab, illuminant, observateur, compensation) peut se recalculer ; il faut donc **toujours garder les spectres**, ce que le glossaire (« Mesure ») impose déjà.

## 5. Ce qu'on en tire pour myiro-libre

- Enregistrer avec chaque mesure : condition lue sur l'instrument (M0, M1 ou M2), fond de mesure, illuminant et observateur du calcul, et inscrire `INSTRUMENT_FILTER` / `TARGET_INSTRUMENT` dans le `.ti3` exporté quand c'est exact.
- Refuser de comparer (contrôle ISO, ΔE) deux mesures de conditions différentes, ou le signaler en rouge.
- Pour profiler avec ArgyllCMS : essayer `colprof -f` (compensation) sur des spectres M0 ou M1 et comparer au profil sans compensation ; tester d'abord sur du papier azuré et sur du papier non azuré.
- Prévoir dans l'interface un choix simple (« mesure comme la lumière du jour (M1) » / « sans UV (M2) »), en cachant les détails ISO.

## 6. Questions ouvertes

- Comment le MYIRO-1 produit-il M1 exactement (deux niveaux d'UV ? un calcul interne ?) et le FD-9 ? [à vérifier]
- Quelles conditions le FD-9 propose-t-il, et a-t-il une polarisation ? [à vérifier]
- Les spectres livrés par le SDK sont-ils déjà corrigés pour la condition affichée, ou bruts ? (à voir sur appareil réel, palier 5)
- Le fond de mesure normalisé de l'annexe A (noir, blanc ?) : seule la norme payante le dit. [à vérifier]
- Quelle précision gagne-t-on avec `colprof -f` sur un MYIRO-1 ? À mesurer.

## Sources consultées

- https://cdn.standards.iteh.ai/samples/39877/fc7f9f9e2f954e279c475d6c603fd8c1/ISO-13655-2009.pdf (extrait gratuit : préface, introduction, sommaire)
- https://sensing.konicaminolta.us/us/?p=10979
- https://techkon.datacolor.com/what-are-m0-m1-m2-m3-measuring-mode-settings/
- https://chromachecker.com/manuals/en/show/konica-minolta_myiro-1_
- https://www.argyllcms.com/doc/colprof.html
- https://www.argyllcms.com/doc/chartread.html
- https://www.argyllcms.com/doc/spotread.html
- https://www.argyllcms.com/doc/FWA.html
- https://www.argyllcms.com/doc/ti3_format.html
- https://www.argyllcms.com/doc/instruments.html
- https://fogra.org/en/certification/prepress-technology/contract-proof-creation
- https://registry.color.org/cmyk-registry/fogra51
