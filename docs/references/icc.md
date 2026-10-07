# Références ouvertes : ICC, couleur et logiciels libres

Base de connaissance du projet : des **liens** vers des sources ouvertes ou officielles, avec ce qu'on y trouve et ce qu'on en tire. Rien n'est recopié. Recherche faite le 7 octobre 2026 par recherche web ; les numéros de version et les affirmations marquées **[à vérifier]** sont à confirmer sur la source avant de s'appuyer dessus.

Pour la mettre à jour : relancer une recherche (outil de recherche web de Claude Code, skill `research`) et corriger ce fichier ; la commande `/fin-de-journee` le rappelle.

## 1. Profils ICC et normes

| Source | Contenu | Usage pour le projet |
|---|---|---|
| [Spécification ICC v4](https://www.color.org/v4spec.xalter) | Version courante ICC.1:2022 (profil 4.4.0.0), publiée en mai 2022, co-publiée comme ISO 15076-1. Téléchargement gratuit sur color.org. | Référence pour lire et écrire les profils. Le norme ISO elle-même est payante : on cite la spécification ICC gratuite. |
| [Spécifications ICC](https://www.color.org/icc_specs2a/) et [anciennes versions](https://www.color.org/past_icc_specs/) | Versions v2 et v4. | ArgyllCMS produit surtout du v2 par défaut : savoir lire les deux. |
| [iccMAX](https://color.org/iccmax/) et [outils libres de l'ICC](https://www.color.org/opensource.xalter) | Nouvelle génération de profils, et un projet de bibliothèques et d'outils libres (publié sur GitHub, renommé iccDEV en octobre 2025 **[à vérifier]**). | Veille ; utile pour inspecter des profils. |
| [Registre des données de caractérisation](https://color.org/registry2.xalter) et [FOGRA39](https://www.color.org/chardata/FOGRA39.xalter) | Données de référence des conditions d'impression normalisées (FOGRA39 : 1 485 plages, FOGRA39L : 1 617). | Valeurs de référence Lab pour le contrôle d'impression. |
| [Données de caractérisation FOGRA](https://fogra.org/en/downloads/work-tools/characterisation-data) | Ensemble des données FOGRA39 à FOGRA60 en téléchargement. | Idem. Vérifier les droits d'usage avant de les redistribuer. |
| [ECI](https://www.eci.org) | Profils (ISOcoated v2) et mires (ECI2002) de référence. | Mires et profils de comparaison. Vérifier la licence. |

## 2. Moteur de profilage : ArgyllCMS

- Site et documentation : [argyllcms.com](https://www.argyllcms.com), [index de la documentation](https://argyllcms.com/doc/ArgyllDoc.html). Version 3.5.0 du 4 février 2026 **[à vérifier]**, licence **AGPL-3.0** (avec des parties sous licences plus permissives).
- Outils utiles : `targen` (générer les plages), `printtarg` (mettre la mire en page), `chartread` (lire), `colprof` (créer le profil), `printcal` (calibration d'impression), `profcheck` (vérifier un profil), `iccgamut` (gamut), `icclu` et `xicclu` (interroger un profil), `iccdump` (afficher un profil).
- **Instruments** : [page des instruments](https://www.argyllcms.com/doc/instruments.html). D'après cette page, **aucun instrument Konica Minolta n'est géré** (marques listées : JETI, Image Engineering, Klein, X-Rite et Calibrite, Gretag-Macbeth, DataColor, quelques autres). Elle précise que les résultats d'autres logiciels peuvent être importés. Conséquence : le pilotage du MYIRO-1 et du FD-9 passe par notre pont SDK (ADR 0002 et 0005), ArgyllCMS ne sert qu'au calcul.
- Une discussion de forum ([photopxl](https://photopxl.com/?p=35970)) confirme le contournement par export CGATS d'un logiciel du fabricant, puis import dans ArgyllCMS. C'est le plan B si le pilote libre échoue.

## 3. Bibliothèques libres utilisables (Rust)

| Projet | Licence | Rôle possible |
|---|---|---|
| [Little CMS](https://github.com/mm2/Little-CMS) | MIT | Moteur ICC de référence (profils v2 et v4, transformations). Implémentation complète de la spécification 4.4. Son [site](https://www.littlecms.com). |
| [crate `lcms2`](https://lib.rs/lcms2) | MIT | Interface Rust vers Little CMS, utilisée en production. |
| [`moxcms`](https://lib.rs/crates/moxcms) | à vérifier | Gestion ICC en Rust pur : CMJN, RVB, Lab, jusqu'à 16 encres. Plus rapide que `lcms2` sur certaines conversions d'après ses mesures. |
| [crate `cgats`](https://lib.rs/crates/cgats) | GPL-3.0 | Lecture, écriture et moyenne de fichiers CGATS.17, calcul de ΔE. Compatible avec notre licence GPL-3.0. |
| [`empfindung`](https://rust-digger.code-maven.com/crates/empfindung) | à vérifier | Écarts de couleur (ΔE) en Rust. |

Question à trancher pour l'architecture : la conversion Lab vers RVB ou CMJN et le contrôle de profil passent-ils par `lcms2` (mature) ou `moxcms` (Rust pur) ? ArgyllCMS (AGPL-3.0, processus externe) reste séparé.

## 4. Autres projets libres à connaître

- [colour-science](https://github.com/colour-science/colour) (Python) : calculs colorimétriques de référence (CIEDE2000, spectral). Sert à **valider** nos calculs en les comparant, pas à être embarqué.
- [DisplayCAL (fork Python 3)](https://github.com/eoyilmaz/displaycal-py3) : interface libre d'ArgyllCMS pour les écrans, un exemple d'application libre au-dessus d'ArgyllCMS.
- [DemoIccMAX / iccDEV](https://www.color.org/opensource.xalter) : outils de l'ICC pour examiner un profil.

## 5. Ce qu'il reste à chercher

- Définition exacte des critères de contrôle ISO 12647-7 et -8 (sous-ensembles de plages, limites) : normes payantes, voir si des synthèses officielles ou FOGRA existent.
- Conditions de mesure M0, M1, M2 et ISO 13655 : une source gratuite fiable.
- Densité (statuts E et T, ISO 5-3) : formules et tables de pondération libres.
- Existence d'un pilote libre pour les spectromètres Konica Minolta (aucun trouvé pour l'instant).

## Outils de recherche

- Recherche web de Claude Code (`WebSearch`, `WebFetch`) : disponible, utilisée pour ce fichier.
- Context7 (documentation à jour de bibliothèques de code) : pour Tauri 2 et les crates Rust, pas pour les normes couleur ; non connecté dans la dernière session.
- DeepWiki (documentation générée pour des dépôts GitHub) : pour comprendre un dépôt comme Little CMS **[à vérifier]**.
