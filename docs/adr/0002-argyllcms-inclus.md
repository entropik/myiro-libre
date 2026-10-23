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
- **Deux outils seulement pour l'instant** : `colprof` calcule le profil, `profcheck -k` en mesure la précision (écart ΔE00 moyen et maximal entre les mesures et ce que le profil prédit). `printcal` et `targen` viendront avec la linéarisation et les mires produites par myiro-libre (#33).
- **Processus externe, derrière un trait** `MoteurProfil`, sur le modèle de l'ADR 0005 : `ArgyllCms` pour le vrai calcul, `MoteurSimule` pour tous les tests automatiques. Le calcul se fait dans un dossier de travail temporaire, effacé à la fin, réussi ou non ; la commande montrée est exactement celle lancée.
- **Recettes** : Rapide `-qm`, Standard `-qh` (par défaut), Approfondi `-qu`, toutes avec une table perceptuelle à partir de sRGB (`-S ref/sRGB.icm -cmt -dpp`). Les durées de chaque recette restent à mesurer sur le poste.
- **Ce qui est gardé avec le profil** : recette, version d'ArgyllCMS, commandes, journal et précision. Le profil est rangé « non vérifié » : la précision du calcul n'est pas une vérification (#41).
- **Installateur** : si le dossier `argyll/` existe à la racine du dépôt (ignoré par git), `outils/preparer_installateur.py` contrôle sa version et embarque `bin/colprof.exe`, `bin/profcheck.exe`, `ref/sRGB.icm` et le texte de la licence sous `argyll/`, à côté de l'application. Sans lui, l'installateur se construit sans ArgyllCMS et le script le dit. Le nom du fichier de licence dans la distribution 3.5.0 n'est pas confirmé : le script retient le premier présent d'une courte liste et s'arrête s'il n'en trouve aucun.
- **Obligation de l'AGPL-3.0** : chaque publication d'un installateur qui embarque ArgyllCMS fournit, à côté, les sources d'ArgyllCMS 3.5.0 ou leur lien précis sur https://www.argyllcms.com/. Une publication sans ce lien n'est pas faite.
