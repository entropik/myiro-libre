# ArgyllCMS est inclus dans l'installateur et appelé en processus externe

Les profils ICC et les linéarisations sont calculés par ArgyllCMS (`colprof`, `printcal`, `targen`, `profcheck`), appelé en processus externe, dans une version figée livrée avec l'application. Une application dite autonome ne peut pas demander à un imprimeur d'installer un outil en ligne de commande, et écrire un moteur de profilage fiable est hors de portée de la v1.

## Options écartées

- Demander à l'utilisateur d'installer ArgyllCMS : contraire à l'objectif d'autonomie.
- Moteur maison sur LittleCMS : envisageable à long terme, pas pour la v1.

## Conséquences

- ArgyllCMS est sous AGPL-3.0, compatible avec la GPL-3.0 du projet ; chaque version publiée doit fournir (ou pointer précisément) les sources de la version d'ArgyllCMS incluse.
- Une montée de version d'ArgyllCMS est une décision explicite, testée sur des jeux de mesures de référence.

## Livré (23 octobre 2026, ticket #40)

Première partie du ticket : un profil RVB calculé à partir d'une mesure importée d'un export CGATS de FD-S2w.

- **Version figée : ArgyllCMS 3.5.0, Windows 64 bits.** La crate `profilage` lit la version dans l'aide de `colprof` et refuse en clair toute autre version ; un dossier `argyll/` sans `bin/colprof`, `bin/profcheck` et `ref/sRGB.icm` est dit introuvable avant tout calcul. Rien n'est rangé dans ces deux cas.
- **Deux outils seulement pour l'instant** : `colprof` calcule le profil, `profcheck -k` en mesure la précision (écart ΔE00 moyen et maximal entre les mesures et ce que le profil prédit). `printcal` et `targen` viendront avec la linéarisation et les mires produites par myiro-libre (#33). Depuis le 28 octobre 2026 (#71), un troisième outil est lancé : `iccgamut`, pour la limite du gamut d'un profil dans la comparaison des profils (voir plus bas).
- **Processus externe, derrière un trait** `MoteurProfil`, sur le modèle de l'ADR 0005 : `ArgyllCms` pour le vrai calcul, `MoteurSimule` pour tous les tests automatiques. Le calcul se fait dans un dossier de travail temporaire, effacé à la fin, réussi ou non ; la commande montrée est exactement celle lancée.
- **Recettes** : Rapide `-qm`, Standard `-qh` (par défaut), Approfondi `-qu`, toutes avec une table perceptuelle à partir de sRGB (`-S ref/sRGB.icm -cmt -dpp`). Durée mesurée sur le poste le 26 octobre 2026 : Standard, un peu moins de 3 minutes pour 2 520 plages ; Rapide et Approfondi restent à mesurer.
- **Ce qui est gardé avec le profil** : recette, version d'ArgyllCMS, commandes, journal et précision. Le profil est rangé « à vérifier » (état `non_verifie`), affiché « Profil calculé, à vérifier » ; un profil rangé tel quel s'affiche « Profil importé, à vérifier ». La précision du calcul n'est pas une vérification (#41).
- **Installateur** : si le dossier `argyll/` existe à la racine du dépôt (ignoré par git), `outils/preparer_installateur.py` contrôle sa version et embarque `bin/colprof.exe`, `bin/profcheck.exe`, `bin/iccgamut.exe` (ajouté le 28 octobre 2026, #71), `ref/sRGB.icm` et le texte de la licence sous `argyll/`, à côté de l'application. Sans lui, l'installateur se construit sans ArgyllCMS et le script le dit. Le nom du fichier de licence dans la distribution 3.5.0 n'est pas confirmé : le script retient le premier présent d'une courte liste et s'arrête s'il n'en trouve aucun.
- **Obligation de l'AGPL-3.0** : chaque publication d'un installateur qui embarque ArgyllCMS fournit, à côté, les sources d'ArgyllCMS 3.5.0 ou leur lien précis sur https://www.argyllcms.com/. Une publication sans ce lien n'est pas faite.

## Complété (26 octobre 2026, ticket #40)

- **Confirmé avec le vrai ArgyllCMS 3.5.0** : ligne de bilan de `profcheck -k` (`errors(CIEDE2000): max. = …, avg. = …`), `COLOR_REP "iRGB"` écrit par `targen` pour un `.ti1` (le `.ti3` garde `"iRGB_LAB"`). `colprof -D` n'écrit que de l'ASCII : myiro-libre lui passe le nom sans accents, la bibliothèque garde le nom d'origine (voir la [référence des options](../references/icc-argyll-options.md)).
- **Avancement** : les sorties de `colprof -v` et `profcheck` sont lues en direct (un fil par sortie, rien ne bloque) ; les étapes de `colprof` sont repérées à leur première ligne, et seul un pourcentage écrit par ArgyllCMS fait avancer la barre dans une étape. Le journal complet reste gardé avec le profil.
- **Durée estimée** : la durée du dernier calcul réussi de chaque recette, rapportée au nombre de plages, est gardée sur le poste ; l'écran l'annonce comme « estimé », jamais sans calcul précédent.

## Complété (28 octobre 2026, ticket #71)

- **`iccgamut` rejoint les outils lancés** pour la comparaison des profils : `iccgamut -v -ia -pl profil.icc` calcule la limite du gamut d'un profil et son volume (commande confirmée le 26 octobre 2026 avec ArgyllCMS 3.5.0, voir la [référence des options](../references/icc-argyll-options.md)). Il passe par un second trait, `MoteurGamut`, séparé de `MoteurProfil`, avec le même contrôle de version, le même dossier de travail effacé à la fin et sa commande montrée telle quelle.
- **Installateur** : `bin/iccgamut.exe` est embarqué avec `colprof`, `profcheck` et `sRGB.icm` ; un dossier `argyll/` sans lui arrête `outils/preparer_installateur.py`, comme pour les autres fichiers.
- **Sur le poste, un ArgyllCMS sans `iccgamut`** calcule toujours les profils : seul cet outil est dit manquant. La comparaison montre alors la surface brute à la place de la limite du gamut, avec la raison, et calcule tout le reste.
