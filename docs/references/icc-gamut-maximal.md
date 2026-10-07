# Profil ICC au gamut maximal : encrage, noir, gris neutre et saturation

Date de recherche : 7 octobre 2026. Méthode : lecture des pages officielles d'ArgyllCMS (colprof, targen, printcal, collink, iccgamut, viewgam, FWA, mppprof, Scenarios), des pages de l'ICC (PRMG, compensation du point noir, correction du support), et des pages publiques de la FOGRA, de l'ISO et de l'association qui porte la méthode G7. Les normes ISO payantes ne sont citées que par leur résumé public. Marques : **[établi]** (lu dans une source primaire), **[probable]** (source secondaire ou lecture partielle), **[à vérifier]**.

Ce fichier complète, sans les répéter : [Options d'ArgyllCMS](icc-argyll-options.md) (tableau complet des options), [Conditions de mesure M0 à M3](icc-conditions-de-mesure.md), [Écart de couleur CIEDE2000](icc-delta-e.md), [Critères de contrôle ISO 12647](icc-12647-controle.md) et la [page d'accueil des références](icc.md).

Mots utilisés :

- **Gamut** : l'ensemble des couleurs qu'une imprimante peut réellement imprimer sur un papier donné. Un « gamut maximal » est un profil qui ne gaspille aucune de ces couleurs.
- **Colorant** : une encre (ou un toner). CMJN = cyan, magenta, jaune, noir ; OVG = orange, vert, violet.
- **TAC** (taux d'encrage total) : la somme des pourcentages d'encre posés au même endroit (400 % au maximum en CMJN).
- **GCR / UCR** : remplacer une partie du gris fait de cyan, magenta et jaune par du noir (GCR : partout ; UCR : seulement dans les ombres neutres).
- **Gamut mapping** (mise en correspondance de gamut) : la façon de faire entrer les couleurs d'une image dans le gamut de l'imprimante.

## L'essentiel en 10 lignes

1. Le gamut réel se gagne **avant** le profil : bonne linéarisation (`printcal`), encrage maximal que le papier supporte, mesure dans la bonne condition (M1 de préférence).
2. Le taux d'encrage total se trouve à l'essai : on imprime des petites mires à plusieurs limites et on garde la plus haute qui ne bave pas, ne colle pas et ne fait pas gondoler le papier.
3. En dessous de 200 % de TAC, on perd des couleurs saturées ; à partir de 250 %, on obtient des noirs denses (ArgyllCMS).
4. Le choix du noir (GCR) ne change presque pas le gamut, mais il change beaucoup la **stabilité du gris** : plus de noir = gris plus stable, mais trame noire plus visible.
5. ArgyllCMS : `-l` (TAC) et `-L` (limite du noir) dans `colprof`, `-k`/`-K` pour la courbe de noir ; la limite par encre se règle dans `printcal -x`.
6. Le gris neutre se juge « par rapport au papier » (le blanc papier devient la référence) : c'est le fonctionnement de l'intention colorimétrique relative.
7. Pour la saturation, le profil doit contenir de bonnes tables perceptuelle et saturation : `colprof -S profil_source` avec des conditions de vision (`-c`, `-d`), et l'intention `s` (saturation accentuée) pour la table saturation.
8. Ne pas prendre un espace géant (ProPhoto) comme source de mise en correspondance : tout serait trop compressé (avertissement d'ArgyllCMS).
9. ArgyllCMS ne sait pas faire de profil ICC pour plus de 4 encres (CMJN+OVG) : il sait générer la mire, pas le profil. La norme ICC, elle, l'autorise (jusqu'à 15 encres).
10. On vérifie avec des chiffres : volume du gamut (`iccgamut`), comparaison de deux gamuts (`viewgam -i`), écarts ΔE00 du profil sur ses propres mesures (`profcheck -k`) et sur des plages de contrôle imprimées à part.

## 1. Mesurer le gamut réel

### Mires : nombre et répartition des plages

- `targen` recommande **3 000 plages ou plus** pour un profil de haute qualité, **500 à 1 000** pour une qualité moyenne, **1 000 à 2 000** pour une presse bien réglée, et **3 000 ou plus** pour les appareils à plus de 4 encres. [établi] (https://www.argyllcms.com/doc/targen.html)
- Pour une imprimante pilotée en RVB : « quelques centaines » de valeurs (400 à 1 000) ; pour un CMJN de qualité, 1 000 à 3 000. [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP2)
- Répartition : par défaut `targen` place les plages par « point le plus éloigné » (OFPS), qui remplit le volume du gamut. Avec un **profil de préconditionnement** (`-c`, par exemple un premier profil grossier de la même machine), les plages sont réparties selon l'écart perçu, donc mieux placées dans les zones saturées et sombres. [établi] (https://www.argyllcms.com/doc/targen.html)
- Plages près du gris : `-N` renforce l'axe neutre (défaut 0,5, effet seulement avec une répartition perceptuelle, donc avec `-c`) ; `-V` renforce les sombres (défaut 1,0). Les dégradés gris (`-g`) et par encre (`-s`) ajoutent des plages ciblées. [établi] (même page)
- Repères pratiques sur papier A4 avec un spectrophotomètre à bande : la table de `targen` donne par exemple 441 plages par A4 pour un i1Pro (1 764 pour 4 A4). Pour le MYIRO-1 la taille de plage est à déterminer avec notre propre mise en page. [établi pour la table ; à vérifier pour le MYIRO-1]
- Réglage de la limite d'encrage de la mire : la mettre **au moins 10 % plus haut** que la limite qu'on donnera ensuite au profil. [établi] (https://www.argyllcms.com/doc/targen.html)

### Linéarisation préalable avec printcal

- `printcal` crée des courbes par encre pour rendre chaque encre « régulière » à l'œil : il mesure l'écart CIE94 le long de chaque dégradé d'encre. [établi] (https://www.argyllcms.com/doc/printcal.html)
- Il **trouve tout seul le maximum utile de chaque encre** : il regarde l'écart au blanc papier et s'arrête quand ajouter de l'encre ne rapporte presque plus (« rendements décroissants »). `-m` garde une marge sous ce maximum pour pouvoir recalibrer plus tard ; `-x` impose un maximum à une encre (par exemple `-xc 80` pour le cyan à 80 %). [établi] (même page ; exemple `-xc` dans https://www.argyllcms.com/doc/Scenarios.html#PC1)
- Dégradés recommandés pour la calibration : 50 pas ou plus par encre. [probable] (https://www.argyllcms.com/doc/Scenarios.html#PC1, lu via un résumé)
- Imprimantes RVB : `printcal` y est « peu utile », car les voies R, V, B ne sont pas de vraies encres et le pilote contrôle déjà leur réponse. [établi] (https://www.argyllcms.com/doc/printcal.html)
- Le fichier `.cal` se transmet ensuite à `targen -C`, `printtarg -K` ou `-I` et `colprof -Y` (voir [Options d'ArgyllCMS](icc-argyll-options.md)).

## 2. Limite d'encrage totale (TAC) et par encre

- Effet sur le gamut : « en général, on veut la plus grande quantité d'encre possible pour maximiser le gamut » ; la plupart des appareils CMJN se situent **entre 200 et 400 %** ; **250 % ou plus** est souhaitable pour des noirs denses ; **sous 200 %**, le gamut saturé commence à être compromis. [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP2)
- `colprof` le confirme : en dessous de 200 %, les couleurs secondaires pleinement saturées (rouge, vert, bleu d'impression) ne sont plus reproduites. [établi] (https://www.argyllcms.com/doc/colprof.html)
- **Comment la trouver** (méthode d'ArgyllCMS) : imprimer quelques petites mires `targen` + `printtarg` à différentes limites, et garder la plus haute qui ne pose pas de problème (bavure, séchage, collage, gondolage). [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP2) Les problèmes physiques se voient à l'œil ; aucune mesure de couleur ne les détecte. [probable]
- Dans `colprof` : `-l` fixe la limite totale (elle doit être **un peu en dessous** de celle de la mire) et `-L` la limite du noir seul, pour éviter que la trame du noir ne « bouche ». [établi] (https://www.argyllcms.com/doc/colprof.html)
- Limite **par encre** (cyan, magenta, jaune) : il n'y a pas d'option dans `targen` ni dans `colprof` ; elle passe par la calibration (`printcal -x` ou maximum automatique). [établi] (https://www.argyllcms.com/doc/targen.html, https://www.argyllcms.com/doc/printcal.html)
- Imprimante pilotée en RVB : le pilote applique déjà ses propres limites ; on ne règle donc pas de TAC dans le profil. [probable] (https://www.argyllcms.com/doc/Scenarios.html#PP2, section sur les imprimantes RVB)

## 3. Séparation noire (GCR, UCR, courbe de noir)

- Dans ArgyllCMS, la courbe de noir s'applique **dans tout le gamut** : c'est du GCR. L'UCR n'est pas géré. [établi] (https://www.argyllcms.com/doc/colprof.html)
- Réglages de `colprof -k` : `z` noir minimum, `h` moitié, `x` noir maximum, `r` rampe linéaire (minimum dans les clairs, maximum dans les ombres) — **`-kr` est le défaut**, équivalent à `-kp 0 0 1 1 1`. [établi] (https://www.argyllcms.com/doc/colprof.html) Cela tranche la question laissée ouverte dans [Options d'ArgyllCMS](icc-argyll-options.md).
- `-kp stle stpo enpo enle forme` : courbe sur mesure. `stle` = niveau de noir dans les clairs, `stpo` = point de départ du noir (en clarté), `enpo` = point où la courbe redevient plate, `enle` = niveau final dans les ombres, `forme` = courbe creuse (0 à 1) ou bombée (1 à 2). [établi] (https://www.argyllcms.com/doc/colprof.html) Le **point de départ du noir** d'un imprimeur correspond à `stpo`.
- `-K` : mêmes réglages, mais en **proportion du noir maximal possible** pour chaque couleur, au lieu d'une valeur absolue. [établi] (même page)
- « Max black » (`-kx`) : d'après ArgyllCMS, c'est actuellement le meilleur choix pour que l'impression soit **la moins sensible à la lumière d'éclairage** ; des niveaux de 0,3 à 0,5 peuvent être préférés pour réduire l'aspect « bruité » du noir en jet d'encre ou les défauts de bandes. [établi] (https://www.argyllcms.com/doc/colprof.html)
- Effet sur le gris : plus de noir rend l'équilibre des gris **plus robuste** (moins de dérive de teinte si une encre couleur varie), mais rend la trame plus visible ; moins de noir donne l'effet inverse. La courbe doit rester **douce** : un changement brusque de noir oblige un changement aussi brusque du CMJ et crée des cassures dans les dégradés. [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP6)
- Méthode conseillée : faire un profil en `-kz` puis en `-kx`, tracer l'axe neutre avec `xicclu`, puis choisir une courbe `-kp` entre les deux. Exemple de la documentation : `-kp 0 0 .93 .87 0.65`. [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP6)
- Effet sur le gamut des ombres : le noir le plus profond est atteint avec le maximum d'encre autorisé ; avec `-L` trop bas ou une TAC trop basse, le point noir remonte. La courbe de noir elle-même change peu le gamut, sauf dans les ombres où elle décide comment on atteint ce point noir. [probable, raisonnement à partir de colprof.html et Scenarios.html#PP6]
- Pour un lien CMJN vers CMJN, `collink` a en plus `-kt` (conserver le noir de la source) et `-ke` (conserver le noir de la table de destination). [établi] (https://www.argyllcms.com/doc/collink.html)
- Imprimante RVB : pas de réglage de noir, le pilote décide. [établi] (https://www.argyllcms.com/doc/colprof.html, la génération du noir ne concerne que le CMJN)

## 4. Gris neutre : par rapport au papier ou absolu

- **Gris relatif au papier** : dans l'intention colorimétrique relative, le blanc du papier devient le blanc de référence et tout le reste est mis à l'échelle de la même façon. L'ICC applique la même logique à la « correction du support » : le blanc du papier de production devient la cible, les autres couleurs suivent. [établi] (https://www.color.org/substratecorrection/)
- **Gris absolu D50** (intention colorimétrique absolue) : on reproduit aussi la teinte du papier ; un gris imprimé sur papier crème restera crème. Utile en épreuve (simuler un autre papier), rarement pour l'impression finale. [probable] (logique des intentions décrite dans https://www.argyllcms.com/doc/collink.html)
- **ISO 12647-2 et correction du support** : la norme de 2013 et l'ISO 13655 décrivent une correction « tristimulus » des valeurs cibles quand le papier réel diffère du papier de référence ; elle sert aussi à calculer un dégradé gris corrigé du papier. [probable] (résumés : https://www.printing.org/publication/t110034, https://iteh.es/catalog/standards/iso/960e390f-2252-4af2-bbd1-167266af6cc0/iso-13655-2017)
- **Méthode G7** : elle règle l'imprimante par des courbes CMJN simples pour suivre une courbe de densité neutre (NPDC) et un équilibre des gris défini ; une échelle de pourcentages CMJ censés paraître gris (héritée du 50/40/40) et des valeurs a*b* cibles. [établi pour l'existence des NPDC et du gris CMJ] (https://www.printing.org/detail/resource/G7-Methodology) Le détail de la cible a*b* (qui part de la teinte du papier dans les clairs et va vers le neutre dans les ombres) n'a pas été lu dans une source primaire. **[à vérifier]**
- **ISO/PAS 15339 et CRPC** : sept conditions de référence (CRPC1 à CRPC7) qui couvrent la gamme des gamuts d'impression, quel que soit le procédé. La PAS de 2015 a été retirée ; une nouvelle version (ISO 15339-2) est en projet, annoncée pour juin 2026. [probable] (https://ccn-scc.ca/standardsdb/standards/8160942, https://www.dinmedia.de/en/draft-standard/iso-dis-15339-2/403652254) Ces conditions reposent sur la méthode G7 pour le gris. **[à vérifier]**
- **Comment le profil garantit le gris** : (1) mesures fiables près de l'axe neutre (plages `-N`, `-g`) ; (2) une linéarisation propre ; (3) une courbe de noir douce ; (4) un contrôle après coup : convertir un dégradé L* de gris neutres (a* = b* = 0) en relatif, l'imprimer, le mesurer et regarder ΔCh (voir [CIEDE2000, ΔH et ΔCh](icc-delta-e.md)). ArgyllCMS ne propose pas d'option « forcer le gris » dans `colprof` : la neutralité vient de la précision du modèle. [probable]
- Équilibrer le gris **par la calibration** (comme G7) plutôt que par le profil rend l'imprimante stable entre deux profils ; `printcal` linéarise chaque encre mais ne vise pas un équilibre de gris de type G7. [probable] (https://www.argyllcms.com/doc/printcal.html)

## 5. Saturation et mise en correspondance de gamut

### Ce que fait chaque intention

- ICC : quatre intentions — perceptuelle, colorimétrique relative, saturation, colorimétrique absolue. (voir la [spécification ICC v4](https://www.color.org/v4spec.xalter))
- ArgyllCMS travaille dans l'espace d'apparence **CIECAM02** et propose des intentions plus fines (`colprof -t` pour la table perceptuelle, `-T` pour la table saturation) [établi] (https://www.argyllcms.com/doc/colprof.html, https://www.argyllcms.com/doc/collink.html) :
  - `p` perceptuelle (préférée) : compression en « genou » en trois dimensions, en gardant teinte et aspect général ;
  - `la` apparence à luminance accordée : compresse ou étire linéairement l'axe des clartés du blanc au noir ;
  - `lp` perceptuelle qui préserve la luminance, au prix de la saturation ;
  - `pa` perceptuelle d'apparence (sans remise à l'échelle du blanc) ;
  - `ms` saturation : compresse et **étire** pour faire coïncider le gamut source avec celui de l'imprimante, en favorisant la saturation plutôt que la teinte ou la clarté ;
  - `s` saturation accentuée (= ICC saturation) : comme la perceptuelle, mais augmente un peu la saturation dans les zones très saturées ;
  - `r` relative, `a`/`aw`/`aa`/`al`/`rl` variantes colorimétriques et absolues.
- Pour un gamut d'impression « exploité à fond » : table saturation en `s` ou `ms`, table perceptuelle en `p`. [probable, choix éditorial à valider par essais]

### Le profil source et les conditions de vision

- `colprof -s source` remplit la table perceptuelle, `-S source` remplit les tables perceptuelle **et** saturation ; sans l'une de ces options, le profil est seulement colorimétrique. On peut aussi donner un pourcentage (compression, et expansion pour la table saturation). [établi] (https://www.argyllcms.com/doc/colprof.html)
- Quel profil source : pour un usage général, prendre un appareil « de l'autre type » (un profil d'écran pour une imprimante) ; pour un usage précis, le vrai profil source ; à défaut, un pourcentage (`-S 20`). [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP5)
- **Ne pas** choisir un très grand espace (ProPhoto) comme source : la compression serait excessive. [établi] (https://www.argyllcms.com/doc/colprof.html)
- `-g fichier.gam` : utiliser en plus le gamut réel d'une image (créé par `tiffgamut`), pour une mise en correspondance optimisée pour cette image. [établi] (https://www.argyllcms.com/doc/colprof.html, https://www.argyllcms.com/doc/tiffgamut.html)
- Conditions de vision `-c` (source) et `-d` (destination) : `pp` impression courante, `pc` impression critique, `pe` évaluation, `mt` écran typique, etc. Exemple de la documentation : `colprof -v -qm -S sRGB.icm -cmt -dpp -kr -l290`. Si l'éclairage réel ne correspond à aucun code, ne pas les mettre. [établi] (https://www.argyllcms.com/doc/colprof.html, https://www.argyllcms.com/doc/Scenarios.html#PP5)
- `-G` n'est **pas** une option de `colprof` : c'est une option de `collink` (lien direct entre deux profils) qui inverse la table A2B à la volée et donne plus de liberté (noir, mise en correspondance). [établi] (https://www.argyllcms.com/doc/collink.html) Pour le gamut le plus fidèle d'une image donnée, un **lien d'appareil** `collink` peut faire mieux que deux profils séparés. [probable]

### Compensation du point noir (BPC)

- La BPC (norme ISO 18619, rédigée avec l'ICC) ajuste une conversion pour garder le détail des ombres et utiliser tout le noir disponible ; elle ne dépend que des deux profils et de l'intention, pas de l'image. Elle s'applique en relative, perceptuelle ou saturation, avec la même intention des deux côtés. [établi] (https://www.color.org/BlackPointCompensation.pdf, version finale ICC du projet de norme)
- Little CMS la propose comme option de transformation. [probable] (https://www.littlecms.com) C'est à notre application (lcms2 ou moxcms) de l'activer pour les aperçus et conversions en colorimétrique relative.

### v2 ou v4, PRMG

- ArgyllCMS produit du **v2** (2.2.0 colorimétrique seul, 2.4.0 avec tables de mise en correspondance). [établi] (https://www.argyllcms.com/doc/colprof.html)
- En v4, l'ICC définit un **support de référence perceptuel** (PRM) : un tirage photo virtuel de contraste 288:1 (blanc à 89 %, noir à 0,30911 %), vu en ISO 3664 P2 (500 lux) ; et un **gamut de référence** (PRMG) que les tables perceptuelles devraient viser, signalé par une balise dédiée. But : que les profils de fabricants différents s'enchaînent mieux en perceptuel. [établi] (https://www.color.org/v4_prmg.xalter)
- ArgyllCMS ne vise pas le PRMG : nos profils v2 restent à utiliser en paires cohérentes. **[à vérifier]**

## 6. Imprimante pilotée en RVB ou en CMJN

- La plupart des imprimantes passant par un pilote simple **se comportent comme des appareils RVB** : le pilote transforme le RVB en encres. On profile alors en « RVB impression » (`targen -d2`). [établi] (https://www.argyllcms.com/doc/Scenarios.html#PP2)
- Conséquences : pas de TAC, pas de courbe de noir (le pilote les décide) ; `printcal` peu utile ; 400 à 1 000 plages suffisent souvent. [établi pour ces trois points] (https://www.argyllcms.com/doc/Scenarios.html, https://www.argyllcms.com/doc/printcal.html, https://www.argyllcms.com/doc/colprof.html)
- Le gamut d'un profil RVB est limité par ce que le pilote veut bien faire : choisir le mode papier et qualité qui donne le plus de densité, et **désactiver toute gestion de couleur du pilote** pour l'impression de la mire, puis garder exactement les mêmes réglages pour la production. [probable] (https://www.argyllcms.com/doc/Scenarios.html#PP2b)
- Un pilote RVB d'imprimante à encres étendues (orange, vert, rouge...) utilise souvent ces encres en interne : le profil RVB capte donc ce gamut étendu sans gérer plus de 4 voies. C'est la voie la plus simple pour une imprimante photo multi-encres. [probable]

## 7. Multi-encres et gamut étendu (CMJN+OVG, FOGRA55)

- **La norme ICC** autorise des espaces à 2 à 15 colorants (signatures `2CLR` à `FCLR`) ; la balise `colorantTable` donne alors le nom et la valeur Lab de chaque encre. [établi] (https://archive.color.org/files/ICC1-2004-10-Errata.doc ; voir aussi la [spécification v4](https://www.color.org/v4spec.xalter))
- **ArgyllCMS** : `targen` sait créer des mires multi-encres (`-d 9` CMJN+orange+vert, `-d 11` CMJN+orange+vert+violet, etc. ; `-D` pour des encres isolées). [établi] (https://www.argyllcms.com/doc/targen.html) Mais `colprof` dit que les profils « N couleurs ne sont pas gérés », et la page des scénarios qu'ArgyllCMS ne sait pas créer de profil ICC pour plus de colorants que le CMJN. [établi] (https://www.argyllcms.com/doc/colprof.html, https://www.argyllcms.com/doc/Scenarios.html)
- `mppprof` crée un **modèle MPP** (format propre à ArgyllCMS, plus compact, moins précis qu'un profil ICC) adapté aux appareils à beaucoup de voies. [établi] (https://www.argyllcms.com/doc/mppprof.html) Il peut servir de profil de préconditionnement pour `targen -c` ; il ne remplace pas un profil ICC utilisable par un RIP. [probable]
- **FOGRA55** : données de caractérisation d'un espace d'échange CMJNOVG « indépendant du procédé », mises au point pour le gamut étendu (ECG) ; la version révisée remplace la bêta de 2018. [établi] (https://fogra.org/en/downloads/work-tools/characterisation-data, https://printingorg.napco.com/node/4849)
- **ISO/TS 21328:2022** : lignes directrices pour créer un jeu de caractérisation CMJNOVG (et CMJN + une partie de O, V, G), avec des recommandations de choix de pigments. [établi par le résumé public] (https://iteh.es/catalog/standards/iso/f0a240c0-0df9-4d8f-af2c-5f1c6c37dc1d/iso-ts-21328-2022)
- Il existe aussi une norme sur la compensation du point noir pour profils N couleurs. **[à vérifier]** (https://knowledge.bsigroup.com/products/image-technology-colour-management-black-point-compensation-for-n-colour-icc-profiles)
- Bibliothèques : `moxcms` annonce jusqu'à 16 encres (voir [icc.md](icc.md)) ; Little CMS gère aussi des espaces multi-encres. **[à vérifier]** pour l'écriture de tels profils.
- Conséquence : un profil CMJN+OVG **imprimable** n'est pas à notre portée avec ArgyllCMS seul ; en revanche, mesurer une mire OVG et **visualiser son gamut** l'est (Lab des plages).

## 8. Mesure : M0, M1, M2 et azurants

- Rappel des conditions et du MYIRO-1 : voir [Conditions de mesure](icc-conditions-de-mesure.md).
- Effet sur le gamut et le neutre : avec un papier azuré, M0 (plus d'UV) donne un blanc plus bleu que M2 (sans UV) ; le blanc sert de référence en relatif, donc **tout l'axe des gris** du profil en dépend. [probable] (même fichier)
- **Compensation des azurants dans ArgyllCMS** (`colprof -f`) : elle demande des mesures **spectrales**, un instrument dont la lumière contient des UV (un instrument « UV cut » ne peut pas exciter les azurants) et la connaissance de l'éclairage de vision, UV compris. [établi] (https://www.argyllcms.com/doc/FWA.html)
- Elle est surtout utile en **épreuve** (faire correspondre deux papiers différents). Pour des profils échangeables, elle risque de les rendre **moins** compatibles, sauf si l'on échange avec des profils faits en M1. Les papiers teintés ou très peu blancs peuvent mal réagir, sauf avec un instrument mesurant sous deux niveaux d'UV. [établi] (https://www.argyllcms.com/doc/FWA.html)
- Le MYIRO-1 mesure lui-même en M1 (selon le fabricant, par deux sources UV) : mesurer en **M1** et ne pas ajouter `-f` est le choix le plus sûr par défaut. [probable] (voir [Conditions de mesure](icc-conditions-de-mesure.md))

## 9. Vérifier le résultat

- **Volume du gamut** : `iccgamut -v -w` écrit un fichier `.gam` et une vue 3D, et donne le volume en « unités L*a*b* cubiques » ; `-k` marque les sommets (rouge, jaune, vert, cyan, bleu, magenta) ; `-i` choisit l'intention, `-f` la table directe (vrai gamut de l'imprimante) ou inverse. [établi] (https://www.argyllcms.com/doc/iccgamut.html)
- **Comparer deux gamuts** : `viewgam -i` calcule le volume commun des deux premiers gamuts, leurs volumes et le pourcentage de recouvrement ; la vue 3D (X3DOM dans le navigateur) permet la transparence pour voir où l'un dépasse l'autre. [établi] (https://www.argyllcms.com/doc/viewgam.html) Comparer, par exemple, le gamut de deux réglages de TAC, ou notre profil contre FOGRA39.
- **Diagnostic de la mise en correspondance** : `colprof -P` écrit des vues 3D des tables perceptuelle et saturation. [établi] (https://www.argyllcms.com/doc/colprof.html)
- **Fidélité du profil à ses mesures** : `profcheck -k` (ΔE00), plage par plage ; ce chiffre mesure le modèle, pas l'impression. [établi] (voir [Options d'ArgyllCMS](icc-argyll-options.md))
- **Contrôle indépendant** : imprimer une mire de contrôle **différente** de la mire de profilage (par exemple une gamme de contrôle de type Media Wedge) au travers du profil, la mesurer et calculer les ΔE00 ; critères et sous-ensembles dans [Critères ISO 12647](icc-12647-controle.md). [probable]
- **Gris** : un dégradé gris converti en relatif, imprimé puis mesuré : regarder a*, b* et ΔCh à chaque pas. [probable]

## 10. Les préréglages RVB du logiciel du fabricant (« Inkjet FineArt » et « Inkjet FineArt Vivid »)

Analyse ajoutée le 7 octobre 2026, à partir du manuel en ligne de MYIROtools et de deux profils RVB que MYIROtools a produits sur le poste en 2021 avec un MYIRO-1. Les profils ont été lus par un petit script local (tables `A2B`/`B2A` évaluées point par point) ; ils ne sont pas versionnés.

**Ce que dit le fabricant** [établi] ([manuel du module Profiler](https://www.myiro.com/support/user-manuals/MYIROtools/myirotools-manual-profiler#icc-rgb), [capture d'écran](https://www.myiro.com/sites/default/files/images/usermanual/myirotools/Profiler_ICC_RGB.png)) :

- En RVB, le mode simple propose seulement deux préréglages : « Inkjet FineArt » (icône « Standard », pour les jets d'encre de qualité) et « Inkjet FineArt Vivid » (« des résultats plus vifs à la conversion »). Aucun réglage n'est montré.
- Les réglages fins (point noir, encrage total, mise en correspondance de gamut) n'existent qu'en mode avancé, pour le CMJN et le multicouleur.
- Le moteur de profilage utilisé n'est pas nommé publiquement **[à vérifier]**.
- Un banc d'essai publié indique des mires de 420, 840 et 2 520 plages, peu de différence entre 840 et 2 520, et une mesure M0/M1/M2 en un seul passage [probable] ([Luminous Landscape](https://luminous-landscape.com/is-there-a-spectrophotometer-in-your-future-myiro-1-or-i1pro3/)).

**Ce que contiennent les profils produits** [établi, sur ces deux fichiers seulement] :

- Profils ICC v2.4, RVB vers Lab, tables directes en grille 17 et tables inverses en grille 33, les trois intentions présentes. Les **mesures spectrales complètes** (380 à 730 nm, M1, D50 2°) sont incluses dans le profil (balise `targ`, au format CGATS.17), ainsi qu'un petit journal de calcul.
- Profil de 840 plages, sain :
  - **Gris** : en colorimétrique relatif, a* et b* restent à ±0,2 du blanc papier de L* 30 à L* 95. En perceptuel, le gris est très légèrement réchauffé (b* +0,5 à +1) et un peu éclairci.
  - **Noir** : L* minimal 15. Le perceptuel ramène doucement les ombres vers ce noir au lieu de les écraser.
  - **Table perceptuelle** : elle compresse tout le gamut, même les couleurs déjà imprimables (C* −1,4 en moyenne, au plus −4 ; L* +3).
  - **Table saturation** : elle **ajoute** de la saturation aux couleurs imprimables (C* +4,7 en moyenne, jusqu'à +11,6), sans déplacer le gris. C'est l'effet « vif ».
  - **Couleurs hors gamut** (primaires Adobe RGB et sRGB) : ramenées au bord du gamut avec peu de dérive de teinte, sauf le bleu (environ −18°).
- Profil de 2 520 plages, **faux** : dans ses mesures incluses, la plage RVB 0 a la clarté d'un gris moyen et la plage RVB 138 est presque noire. Les mesures ne correspondent pas aux plages (mire lue dans le mauvais ordre ou décalée, supposé). Le logiciel a pourtant produit un profil, sans avertissement : un gris demandé sort avec des écarts de 20 à 40 en a* ou b*.

**Ce qu'on suppose de « Vivid »** **[à vérifier]** : la même hausse de saturation que la table saturation ci-dessus, placée aussi dans la table perceptuelle, avec le même gris et le même noir. On ne sait pas quel préréglage a produit le profil sain. Test simple pour trancher : refaire deux profils dans MYIROtools à partir des mêmes mesures, un par préréglage, et comparer leurs tables avec le même script.

## Ce qu'on en tire pour le projet

Réglages par défaut proposés (à valider par essais réels, imprimante par imprimante) :

| Cas | Mire | Réglages `colprof` | Remarques |
|---|---|---|---|
| Imprimante pilotée en RVB | `targen -d2 -f` 900 à 1 300 plages, plus `-g` 20 à 30 gris | `-qh -S` (profil d'écran ou sRGB) `-cmt -dpp -T s` | Pas de TAC ni de noir. Pilote : gestion de couleur coupée, réglages notés. |
| CMJN jet d'encre ou laser | Étape 1 : `printcal -i` (dégradés). Étape 2 : `targen -d4 -l` (TAC + 10 %) ~1 500 à 3 000 plages, `-c` profil préliminaire, `-N` 0,5 à 1 | `-qh -l` TAC `-L` 95 à 98 `-kr` (défaut) ou `-kp` choisi, `-S` source `-cmt -dpp -T s` | Trouver la TAC par mires d'essai. Profil préliminaire de 400 plages pour placer les suivantes. |
| CMJN presse | idem, 1 000 à 2 000 plages | `-l` selon la condition (souvent 300 à 340) **[à vérifier]**, `-L` 95 à 99, `-kp` avec départ du noir vers 20-30 % **[à vérifier]** | Comparer au profil de référence de la condition (FOGRA39, FOGRA51...). |
| CMJN+OVG | mire `targen -d11` pour mesurer et visualiser | pas de profil ICC avec ArgyllCMS | Visualiser le gamut gagné ; profil N couleurs hors de portée pour l'instant. |

Options à montrer dans l'interface (mots simples, la commande exacte dans le journal) :

- « Encre maximale » (TAC, curseur avec assistant d'essai sur mires réduites) et « Noir maximal » (`-L`).
- « Quantité de noir » : peu / équilibré / beaucoup (`-kz`… `-kr` … `-kx`), plus un mode avancé `-kp` avec point de départ du noir.
- « Pour quel usage ? » : photo (perceptuelle `p`), affiche vive (saturation `s` ou `ms`), épreuve (colorimétrique). Source de la mise en correspondance : sRGB / Adobe RGB / écran de l'utilisateur, jamais ProPhoto par défaut.
- « Éclairage de vision » : `pp` par défaut, `pc` pour la cabine normalisée (voir [Éclairage ISO 3664](icc-eclairage-iso3664.md)).
- « Azurants » : condition de mesure affichée (M1 par défaut), compensation `-f` seulement en mode épreuve, avec avertissement.
- Écran de contrôle : volume du gamut, comparaison avec un profil de référence (`viewgam -i`, en pourcentage), ΔE00 moyen et maximal (`profcheck -k`), courbe a*/b* du dégradé gris.

À faire dans le code :

- Garder le `.cal` et la TAC dans la provenance de chaque profil.
- Activer la compensation du point noir dans les conversions lcms2/moxcms en relatif.
- Prévoir la conversion v2 vers v4 seulement si un besoin réel apparaît (voir les questions ouvertes de [Options d'ArgyllCMS](icc-argyll-options.md)).
- Proposer en RVB deux préréglages simples, comme le logiciel du fabricant (section 10) : « Standard » (table perceptuelle, `-S` sRGB ou Adobe RGB) et « Vif » (table saturation renforcée `-T ms`, reprise pour le rendu perceptuel). Le gris et le noir doivent rester identiques entre les deux.
- **Contrôler les mesures avant de calculer un profil** ([ticket 1](https://github.com/entropik/myiro-libre/issues/1)), ce que le logiciel du fabricant ne fait pas (section 10). Le dégradé gris doit aller du foncé au clair, le blanc papier et le noir doivent être aux bonnes plages, et chaque plage doit rester cohérente avec ses voisines. En cas de doute, refuser le calcul et dire pourquoi.
- Banc d'essai : à partir des mesures spectrales incluses dans un profil du fabricant, refaire un profil avec ArgyllCMS et comparer les deux (gris, gamut, ΔE00).

## Points restés à vérifier

- Cible a*b* exacte du gris G7 par rapport à la teinte du papier, et lien entre G7 et les CRPC (texte de la spécification G7 non lu).
- État de l'ISO 15339-2 (projet annoncé pour juin 2026).
- Norme de compensation du point noir pour profils N couleurs : numéro et contenu.
- Écriture de profils N couleurs avec Little CMS ou moxcms.
- Valeurs de TAC et de départ du noir pour une presse : à prendre dans les données de la condition visée (FOGRA, ECI), pas dans ce fichier.
- Taille de plage minimale lisible au MYIRO-1 (nombre de plages par page).
- Comportement d'ArgyllCMS vis-à-vis du PRMG v4 (non visé, d'après l'absence de sortie v4).
- Réglages exacts des préréglages « Inkjet FineArt » et « Inkjet FineArt Vivid », et moteur de profilage du logiciel du fabricant (section 10).

## Sources

- ArgyllCMS : https://www.argyllcms.com/doc/colprof.html, https://www.argyllcms.com/doc/targen.html, https://www.argyllcms.com/doc/printcal.html, https://www.argyllcms.com/doc/collink.html, https://www.argyllcms.com/doc/Scenarios.html, https://www.argyllcms.com/doc/iccgamut.html, https://www.argyllcms.com/doc/viewgam.html, https://www.argyllcms.com/doc/tiffgamut.html, https://www.argyllcms.com/doc/FWA.html, https://www.argyllcms.com/doc/mppprof.html
- ICC : https://www.color.org/v4spec.xalter, https://www.color.org/v4_prmg.xalter, https://www.color.org/BlackPointCompensation.pdf (redirige vers archive.color.org), https://www.color.org/substratecorrection/, https://archive.color.org/files/ICC1-2004-10-Errata.doc
- FOGRA : https://fogra.org/en/downloads/work-tools/characterisation-data ; FOGRA55 (résumé d'article) : https://printingorg.napco.com/node/4849
- G7 : https://www.printing.org/detail/resource/G7-Methodology
- ISO (résumés publics) : https://iteh.es/catalog/standards/iso/f0a240c0-0df9-4d8f-af2c-5f1c6c37dc1d/iso-ts-21328-2022, https://iteh.es/catalog/standards/iso/960e390f-2252-4af2-bbd1-167266af6cc0/iso-13655-2017, https://ccn-scc.ca/standardsdb/standards/8160942, https://www.dinmedia.de/en/draft-standard/iso-dis-15339-2/403652254, https://www.printing.org/publication/t110034
- Little CMS : https://www.littlecms.com
- Logiciel du fabricant (MYIROtools) : https://www.myiro.com/support/user-manuals/MYIROtools/myirotools-manual-profiler ; banc d'essai : https://luminous-landscape.com/is-there-a-spectrophotometer-in-your-future-myiro-1-or-i1pro3/
