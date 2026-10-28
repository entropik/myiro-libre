# Mires de profilage RVB, TIFF sans gestion couleur et QR code — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dans la vue d'une résolution, l'imprimeur choisit l'instrument (FD-9 ou MYIRO-1). En un clic, il obtient la ou les pages TIFF RVB à imprimer sans gestion couleur, et la mire et son tirage sont rangés dans la bibliothèque.
- **FD-9** : la mire est **RGB-2520, reprise telle quelle** (décision du mainteneur, 26 octobre 2026) : 2 520 plages sur une seule page A3, avec son QR code d'origine, que le FD-9 lit au scan. L'application ne dessine pas de mire FD-9 et ne lui propose ni taille ni format.
- **MYIRO-1** : l'imprimeur choisit la taille (rapide, standard, approfondi) et le format (A4 ou A3) ; l'application dessine des pages en bandes, avec un QR code et un texte lisible.

**Architecture:** une nouvelle crate `mires`, indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS :
- pour le **FD-9**, elle lit le TIFF de la mire RGB-2520, vérifie son empreinte et sa géométrie, et en tire les 2 520 valeurs RVB au centre des plages ;
- pour le **MYIRO-1**, ArgyllCMS (`targen`) choisit les valeurs RVB des plages. Il est appelé par la crate `profilage` du plan 3, derrière le même trait `MoteurProfil`. La crate `mires` lit le `.ti1`, tient le gabarit des bandes tiré du manuel, place les plages en respectant le contraste entre voisines, vérifie la mise en page, dessine les pages (plages, texte, QR code) et les écrit en TIFF.

La bibliothèque reçoit deux tables, `mires` et `tirages` (organisation 6). L'application ajoute le module `app/src/mires.rs`, dont les méthodes de `BibliothequeOuverte` se testent sans Tauri, et une section « Préparer une mire » dans `reglage.js`.

Le plan ajoute aussi le **contrôle de cohérence des mesures** (#1, rattaché à #33 par le mainteneur le 26 octobre 2026) : avant tout calcul de profil, la crate `profilage` vérifie que les mesures correspondent aux plages (dégradé gris, blanc et noir, doublons, voisines, décalage). Si ce n'est pas le cas, le calcul est refusé en mots simples, avec les plages en cause. Le résultat du contrôle est gardé dans la provenance du profil. Le contrôle vaut pour une mesure importée (FD-S2w, MYIROtools) comme pour les futures mesures des mires de ce plan.

**Tech Stack:** Rust 2021, Tauri 2, ArgyllCMS 3.5.0 (`targen`, version figée, ADR 0002), crates `tiff` (lecture de la mire RGB-2520, compressée en LZW), `qrcode` (codage du QR, sans image) et `font8x8` (police bitmap du domaine public). Les pages MYIRO-1 sont écrites à la main en PackBits, une compression du TIFF de base. En test seulement : `rqrr` (décodage du QR).

**Spec:** `docs/specifications/2026-10-20-poste-unique-design.md` (étape 1 « Mire » et étape 2 « Impression » du parcours, « Séchage et QR code ») ; ticket #33 (critères d'acceptation) ; ticket #1 (contrôle de cohérence) et `docs/references/icc-gamut-maximal.md`, section 10 (le cas qui l'a fait ouvrir) ; `docs/references/icc-argyll-options.md` (`targen`) ; manuel de FD-S2w, « Conditions de la charte » (p. 35 à 37) et « Mesure des données de charte définie » (p. 38 à 42) ; contraintes du MYIRO-1 relevées dans `outils/mire_comparaison.py`.

**Dépend de :** plan 3 (`docs/superpowers/plans/2026-10-23-profil-rvb-import-fd-s2w.md`) fusionné dans `dev`. Ce plan reprend sa crate `profilage`, `VERSION_ARGYLL`, `chercher_argyll`, la garde `DossierTravail`, `CREATE_NO_WINDOW` et `argyll/` ignoré par git.

## Feuille de route

Ce plan est le **plan 4** de la feuille de route du 23 octobre 2026. Depuis le 26 octobre, il s'exécute après le plan 5 (comparer des profils, #71), et les plans suivants sont décalés d'un numéro. Le plan 3 calcule un profil à partir d'une mesure importée. Le plan 6 mesure une feuille au FD-9 (#34) : le FD-9 scanne d'abord la feuille, lit le QR code de la mire RGB-2520 et place ses points de mesure d'après la géométrie que ce plan établit. Le plan 7 assemble le parcours en 4 étapes (#41). Le plan 8 gère le séchage, avec la file « en attente de mesure ».

Révision du 26 octobre 2026, demandée par le mainteneur : la première version de ce plan dessinait aussi les mires FD-9 (480, 960 ou 1 920 plages de 10 mm, sur 1 à 4 pages). Elle ignorait la capacité d'une page donnée par le manuel du FD-9 (2 928 plages en A3). Les mires FD-9 dessinées sont retirées : le FD-9 utilise la mire RGB-2520, telle quelle, sur une seule page.

Reportés, avec l'accord du mainteneur (23 octobre 2026) :
- la **fiche d'impression** (réglages du pilote) va au plan 7, avec le parcours ;
- l'**import des mires existantes** (CGATS, XML de FD-S2w) va au plan 9, avec la reprise de l'existant ;
- les **mires CMJN** vont avec le profilage CMJN. Le modèle de ce plan les prévoit sans les produire ;
- le PDF ne vient pas maintenant : le ticket donne la priorité au TIFF.

## Global Constraints

- Worktree `ticket/33-mires-rvb` issu de `dev`, PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/33-mires-rvb --reference dev`.
- La crate `mires` reste indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS (ADR 0004, ADR 0007). Ses seules dépendances : `colorimetrie`, `serde`, `serde_json`, `sha2`, `tiff`, `qrcode` (`default-features = false`) et `font8x8`.
- **Mire FD-9 = RGB-2520, telle quelle** : le fichier `RGB-2520 A3 - 1p.tif` n'est ni redessiné, ni recadré, ni recompressé, ni complété (pas de QR code ni de texte ajoutés). Il est copié octet pour octet dans le dossier des mires à imprimer. Comme `argyll/`, il est posé à la main dans un dossier `mire-fd9/` à la racine du dépôt, ignoré par git, et l'installateur l'embarque s'il est présent.
- Le contrôle de cohérence vit dans la crate `profilage` (`coherence.rs`), qui reçoit la dépendance `colorimetrie` pour le ΔE00. Il ne corrige jamais les mesures : il ne remet aucune plage en ordre, n'en écarte aucune et n'applique aucun décalage ; il refuse ou il laisse passer, et il le dit.
- ArgyllCMS : uniquement par la crate `profilage` et son trait `MoteurProfil`, version `VERSION_ARGYLL` = `3.5.0`. Aucune seconde recherche ni seconde garde de dossier.
- Contraintes des manuels, reprises telles quelles et jamais resserrées par invention :
  - **FD-9** (manuel de FD-S2w, p. 35 à 37) : plages de 6 à 30 mm, de même taille et à intervalles réguliers dans une zone ; espace entre deux plages inférieur à 17 % de leur largeur ; marges d'au moins 23 mm en tête, 33 mm en pied et 4 mm sur les côtés ; zone de plages de 300 mm de large au plus ; 8 mm de blanc entre zones, 50 zones par page au plus ; aucun texte à moins de 2 mm d'une zone ; écart d'au moins 40/256 de densité entre voisines ; **jusqu'à 2 928 plages par page A3** (1 394 en A4) ; papier de 90 à 320 mm de large avec le chargeur, de 45 à 330 mm à la main, donc l'A3 passe.
  - **MYIRO-1 en bande** : plages d'au moins 7 mm ; ΔE*ab supérieur à 10 entre voisines ; 10 mm de blanc papier aux deux bouts ; bande de 257 mm au plus.
- **Géométrie de la mire RGB-2520**, mesurée sur le TIFF (26 octobre 2026) et tenue en constantes dans la crate `mires` :
  - image de 3 300 × 4 961 pixels à 300 ppp (279,4 × 420 mm), RVB 8 bits, compression LZW, sans profil ICC intégré ;
  - empreinte SHA-256 `7550f4a38725ad9260982152bb6bcbe077922289adb48dd954f0bd7d2c7fc713` ;
  - une seule zone de **42 colonnes × 60 rangées**, plages jointives de 71 pixels (6,01 mm), coin haut gauche au pixel (159, 285), soit 13,46 mm du bord gauche et 24,13 mm du haut ;
  - ordre des plages : rangée par rangée, de gauche à droite, de haut en bas. La plage 1 (85, 255, 127) est en haut à gauche, comme dans l'export MYIROtools du 26 octobre ;
  - QR code d'origine en bas à droite, contenu `FD-9:011M:RGB-2520 - 1P MY9*1+`. Il porte le nom de la mire, `RGB-2520 - 1P MY9`. Les champs `011M` et `*1+` sont **à vérifier** : FD-S2w part du modèle `FD-9:0000`, et le manuel dit que le QR donne le nom, le nombre de pages, de plages, de rangées et de colonnes ;
  - valeurs RVB lues au centre de chaque plage : identiques à celles de l'export MYIROtools pour 2 512 plages, à 1/255 près pour les 8 autres. **Les valeurs de référence sont celles du TIFF**, parce que c'est lui qui est imprimé.
- **MYIRO-1** : tailles validées par le mainteneur le 23 octobre 2026, comme point de départ à corriger après essais : 232 / 464 / 928 plages. Formats A4 (210 × 297 mm, par défaut) et A3 (297 × 420 mm). TIFF RVB 8 bits, 300 ppp, **sans profil ICC intégré** (pas d'étiquette 34675), une page par fichier.
- Ce qui n'est pas établi reste écrit « à vérifier » dans la documentation, jamais présenté comme sûr.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57). Fichiers source nouveaux sous 400 lignes. `lib.rs` et `colonne.rs` de l'application ne reçoivent que l'enregistrement des commandes.
- Aucun texte en dur dans la page : tout passe par `app/src/textes.rs`, en français et en anglais.
- Données de test fictives : jamais de numéro de série, d'adresse réseau ni de nom de client réels. Aucun fichier de `tests/` ne contient « install » dans son nom.
- Ne jamais écrire le nom du logiciel concurrent, ni citer `docs/sources/`. En dehors de la mire RGB-2520 retenue par le mainteneur, aucune mire de tiers (`docs/mires/`) n'est lue, copiée ni citée.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p mires -p profilage -p bibliotheque -p app`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- **Mire FD-9 cherchée** dans `mire-fd9/` à côté de l'exécutable, puis deux niveaux plus haut (racine du dépôt depuis `target/debug/`), comme `argyll/`. Absente, ou d'une autre empreinte : la préparation FD-9 est refusée en clair (« mire RGB-2520 introuvable » ou « ce fichier n'est pas la mire RGB-2520 »), et le MYIRO-1 reste utilisable.
- **Valeurs de la mire FD-9** : lues dans le TIFF à chaque préparation, au centre de chaque plage, et non recopiées dans le code. Le code ne garde que la géométrie et l'empreinte.
- **Impression de la mire FD-9** : l'image fait 279,4 mm de large, et une page A3 297 mm. Imprimée centrée, la zone de plages est décalée de 8,8 mm environ par rapport au fichier. Le manuel tolère 21 mm de décalage, et le scan du FD-9 retrouve les plages : c'est **à vérifier** à l'essai (tâche 9).
- **Côté de plage MYIRO-1 par défaut** : 8 mm, choisi à l'intérieur des limites du manuel. La section « Avancé » laisse régler le côté dans ces limites. Seule la plage de 20 mm a déjà été mesurée (mire de comparaison, 7 octobre 2026) ; les autres côtés sont « à vérifier » à l'essai.
- **Orientation MYIRO-1** (revue le 28 octobre 2026, demande du mainteneur : « trop de vide, trop de blanc ») : page en **portrait** ; les bandes traversent la largeur, se lisent de gauche à droite et sont centrées ; le texte et le QR code sont dans un bandeau de 22 mm en haut.
- **Écart entre deux bandes MYIRO-1** : 3 mm (15 mm avant le 28 octobre 2026). Le manuel ne fixe aucun écart entre bandes : à confirmer à l'impression avec la règle MY-A02 ; repli à 5 mm.
- **Marge de page MYIRO-1** : 5 mm sur les quatre bords, marge technique d'impression en feuille donnée par le mainteneur.
- **Capacité MYIRO-1 qui en découle** : en plages de 8 mm, 24 bandes de 22 plages = 528 plages par page A4, et 35 bandes de 29 plages = 1 015 par page A3. Les tailles validées tiennent donc sur 1, 1 et 2 pages A4, et sur une seule page A3.
- **Contraste entre voisines (MYIRO-1)** : ΔE*ab > 10 avec la voisine de gauche. Les plages au bout d'une bande doivent aussi trancher sur le blanc papier. Le Lab est **estimé** à partir des valeurs RVB, lues comme du sRGB : c'est une estimation pour la mise en page, jamais une mesure.
- **Génération MYIRO-1** : `targen -v -d2 -G -f<n> mire`, avec les plages blanches et noires par défaut d'ArgyllCMS. L'option `-G` est plus lente : le temps est mesuré à l'essai, et l'option sera retirée si c'est trop long.
- **Plages blanches de `targen`** (28 octobre 2026, décision du général) : les 4 plages blanches (RVB 100, papier nu) sont gardées telles quelles. Placées en milieu de bande, chacune laisse un blanc de la taille d'une plage ; la lecture d'une telle bande au MYIRO-1 est à vérifier à l'essai. Si la bande est coupée, repli sur `targen -e0`, avec un blanc papier mesuré à part.
- **Pas de doublon** (28 octobre 2026, demande du mainteneur à l'essai d'écran, arbitrage du général) : préparer une mire dans une résolution qui a déjà un tirage non mesuré de la même mire ne crée ni tirage ni fichier ; le tirage existant est repris avec ses fichiers et la date de sa première préparation. Même mire : même empreinte pour la FD-9 ; mêmes réglages pour le MYIRO-1 (taille, nombre de plages, format et côté), cherchés avant `targen`. Un fichier effacé à la main est réécrit sous le même numéro ; un fichier présent n'est jamais écrasé. Un nouveau tirage viendra quand le précédent aura été mesuré.
- **Aucun tirage sans ses fichiers** (28 octobre 2026, arbitrage du général) : les pages sont écrites dans un dossier de travail, mises à leur place, et la transaction de la bibliothèque (`enregistrer_mire_et_tirage_si`) n'est validée qu'ensuite. En cas d'échec, ni tirage, ni mire nouvelle, ni fichier ne restent.
- **QR code des pages MYIRO-1** : contenu `MYIRO-LIBRE:T<tirage>:P<page>/<pages>:<8 premiers caractères de l'empreinte>`, en mode alphanumérique, correction M, modules de 0,6 mm. Le MYIRO-1 ne le lit pas : il sert à reconnaître la page ailleurs, et il ne coûte rien. La mire FD-9 garde son QR d'origine.
- **Dossier des fichiers à imprimer** : `Documents\myiro-libre\Mires\`, créé par l'application. L'utilisateur ne choisit pas de dossier, et un bouton « Ouvrir le dossier » le montre dans l'Explorateur.
- **Nombre libre MYIRO-1 en mode avancé** : de 24 à 3 000 plages. Ce sont des bornes de saisie, pas un avis sur la qualité.
- **Seuils du contrôle de cohérence** (tâche 8), acceptés par le mainteneur le 28 octobre 2026. Ce ne sont pas des valeurs de norme : aucune norme ne fixe de seuil pour ce contrôle. Ils restent des choix à vérifier sur des mesures réelles, et le code les commente comme tels :
  - dégradé gris : un retour en arrière de plus de 3 L* entre deux gris successifs ;
  - blanc et noir : le blanc RVB 100 à moins de 2 L* de la plage la plus claire, le noir RVB 0 à moins de 5 L* de la plus foncée ;
  - doublons : plus de 3 ΔE00 entre deux plages de même RVB (entre MYIRO-1 et FD-9, les écarts mesurés le 7 octobre restaient sous 0,6 ΔE00) ;
  - voisines : plus de 20 ΔE00 entre une plage et la moyenne de ses 6 plus proches voisines en RVB, pondérée par l'inverse de la distance ;
  - le calcul est refusé dès qu'un contrôle signale une plage. Un contrôle qui ne peut pas se faire (moins de 3 gris, pas de blanc ou de noir dans la mire) est dit « non fait » dans le résultat, sans bloquer les autres.
- **Décalage** : seulement une piste donnée à l'imprimeur. Si le contrôle refuse, on essaie de décaler les mesures de 1 à 60 plages, dans les deux sens. Si un décalage ne laisse plus aucune plage suspecte, le message le dit (« les mesures semblent décalées de 3 plages »), mais rien n'est recalculé avec ce décalage.

## Review Focus

1. **Mire dont les plages se ressemblent toutes** (nombre libre très petit, ou valeurs proches du blanc) : la préparation est refusée en clair (« plages trop proches pour être lues »), sans fichier à moitié écrit ni mire rangée. → tests `des_plages_toutes_claires_sont_refusees` (tâche 3) et `un_refus_de_mise_en_page_ne_range_rien` (tâche 6).
2. **Noms d'imprimante ou de papier longs, accentués ou avec des signes interdits dans un nom de fichier** (« Hahnemühle Photo Rag 308 », « Canson « Baryta » / 2 ») : le texte lisible garde les accents et est coupé par « … » s'il déborde, et le nom de fichier est nettoyé. → tests `une_ligne_trop_longue_est_coupee_par_des_points` (tâche 4) et `le_nom_de_fichier_est_nettoye` (tâche 6).
3. **Côté de plage MYIRO-1 hors des limites du manuel** (6 mm) : refusé à la saisie, avec la limite dite à côté du champ, et aucune génération lancée. → test `un_cote_hors_limites_est_refuse` (tâche 1).
3 bis. **Mire FD-9 absente, modifiée ou remplacée par un autre fichier** (même nom, autre empreinte ; mire réenregistrée par un logiciel) : la préparation FD-9 est refusée en clair, rien n'est copié ni rangé, et la copie déposée est identique octet pour octet à l'original. → tests `une_mire_fd9_d_une_autre_empreinte_est_refusee` (tâche 1 bis) et `la_mire_fd9_est_copiee_telle_quelle` (tâche 6).
4. **ArgyllCMS absent, ou `targen` qui rend moins de plages que demandé** : message clair (étape, cause, action), rien n'est rangé, et la vue reste utilisable. → tests `argyll_introuvable_ne_prepare_rien` (tâche 6) et `un_ti1_au_nombre_inattendu_est_refuse` (tâche 2).
5. **Pages mélangées** : deux mises en page de la même mire avec deux graines différentes n'ont pas la même empreinte. Le QR code d'une page dit son numéro et se relit à l'identique. → tests `deux_graines_donnent_deux_empreintes` (tâche 3) et `le_qr_d_une_page_se_relit` (tâche 4).
6. **Mesures mélangées ou décalées** (cas du ticket #1) : le calcul est refusé avant ArgyllCMS, les plages en cause sont nommées, rien n'est rangé et le profil actuel ne change pas. → tests `des_mesures_decalees_d_une_plage_sont_refusees_et_le_decalage_est_dit` (tâche 8), `des_mesures_melangees_sont_refusees_avec_les_plages_en_cause` (tâche 8) et `des_mesures_incoherentes_ne_lancent_pas_argyll` (tâche 8).
7. **Fausse alerte sur une mesure saine** : une mire saine de 840 plages, et la vraie mesure de 2 520 plages du 26 octobre, passent sans alerte. → tests `une_mesure_saine_de_840_plages_passe_sans_alerte` et `la_mesure_reelle_du_26_octobre_passe` (`#[ignore]`, tâche 8).

---

### Tâche 0 : Préparer le chantier

**Files:** aucun.

- [ ] **Step 1 : Contexte et dépendance**

Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev`, puis `git log --oneline -5 dev`
Expected: contexte conforme ; la fusion du plan 3 (`ticket/40-profil-rvb-import`) est dans `dev`, et `crates/profilage` existe. Sinon, s'arrêter : ce plan en dépend.

- [ ] **Step 2 : Worktree**

Run: `git worktree add .claude/worktrees/ticket-33 -b ticket/33-mires-rvb dev`

- [ ] **Step 3 : ArgyllCMS présent**

Run: `ls C:/Code/MYIRO-fork/argyll/bin/targen.exe`
Expected: le fichier existe (posé au plan 3, tâche 0). Seuls les tests `--ignored` et l'essai d'écran en ont besoin.

- [ ] **Step 4 : Mire RGB-2520 présente**

Copier `A:\ATELIER OOBLIK\LOGICIELS\MYIRO tools\RGB-2520 A3 - 1p.tif` dans `C:/Code/MYIRO-fork/mire-fd9/`, sans l'ouvrir ni le réenregistrer dans un autre logiciel, puis :
Run: `sha256sum "C:/Code/MYIRO-fork/mire-fd9/RGB-2520 A3 - 1p.tif"`
Expected: `7550f4a38725ad9260982152bb6bcbe077922289adb48dd954f0bd7d2c7fc713`. Seuls les tests `--ignored` de la tâche 1 bis et l'essai en ont besoin : les autres tests travaillent sur un petit TIFF fictif.

---

### Tâche 1 : Crate `mires`, modèle et gabarit du MYIRO-1

**Files:**
- Create: `crates/mires/Cargo.toml`, `crates/mires/src/lib.rs` (modèle, erreurs, `pub use`), `crates/mires/src/gabarit.rs`
- Modify: `Cargo.toml` (membre `crates/mires`), `.github/workflows/ci.yml` (`mires` ajoutée à la liste `CRATES` du job `linux`)
- Test: `crates/mires/tests/gabarit.rs`

**Interfaces:**
- Produces :
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Instrument { Fd9, Myiro1 }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Format { A4, A3 }                 // dimensions(self) -> (f64, f64) en mm, portrait : (210, 297), (297, 420)
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum TailleMire { Rapide, Standard, Approfondi }
  pub fn nombre_plages_myiro1(taille: TailleMire) -> usize;   // 232, 464, 928
  /// Gabarit des pages en bandes du MYIRO-1. Le FD-9 n'a pas de gabarit : sa mire est RGB-2520 (tâche 1 bis).
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Gabarit { pub format: Format, pub cote_mm: f64 }
  impl Gabarit {
      pub fn par_defaut(format: Format) -> Gabarit;                                   // 8 mm
      pub fn nouveau(format: Format, cote_mm: f64) -> Result<Gabarit, ErreurMire>;
      pub fn limites_cote() -> (f64, Option<f64>);                                    // (7, None)
      pub fn page_mm(&self) -> (f64, f64);       // largeur, hauteur : portrait
      pub fn colonnes(&self) -> usize;           // plages par bande
      pub fn rangees(&self) -> usize;            // bandes par page
      pub fn plages_par_page(&self) -> usize;
      pub fn pages(&self, nombre: usize) -> usize;
      pub fn origine_plage(&self, rangee: usize, colonne: usize) -> (f64, f64); // coin haut gauche, en mm
      pub fn zone_texte(&self) -> Rectangle;     // bandeau du haut, à 2 mm au moins des plages
  }
  #[derive(Clone, Copy, Debug, PartialEq)] pub struct Rectangle { pub x: f64, pub y: f64, pub largeur: f64, pub hauteur: f64 }
  #[derive(Debug, PartialEq)]
  pub enum ErreurMire {
      CoteHorsLimites { min: f64, max: Option<f64> },
      Ti1(String),
      NombreInattendu { attendu: usize, trouve: usize },
      Contraste { defauts: usize },
      Ecriture(String),
      MireFd9Introuvable,
      MireFd9Autre { empreinte: String },        // fichier lu, mais ce n'est pas la mire RGB-2520 retenue
      MireFd9Illisible(String),                   // TIFF illisible, ou géométrie différente de celle attendue
  }
  impl ErreurMire { pub fn cle(&self) -> &'static str; }  // mire.erreur.cote, mire.erreur.ti1, mire.erreur.nombre, mire.erreur.contraste, mire.erreur.ecriture,
                                                          // mire.erreur.fd9_introuvable, mire.erreur.fd9_autre, mire.erreur.fd9_illisible
  ```
- Constantes publiques, chacune commentée avec sa source (« manuel MYIRO-1 », relevé dans `outils/mire_comparaison.py`, ou « choix, à vérifier ») :
  - texte : `DISTANCE_TEXTE_MM = 2.0` ;
  - MYIRO-1 : `MYIRO1_BLANC_BOUT_MM = 10.0`, `MYIRO1_BANDE_MAX_MM = 257.0`, `MYIRO1_ECART_BANDES_MM = 3.0` (à confirmer à l'impression, repli à 5 mm), `MYIRO1_MARGE_PAGE_MM = 5.0` (marge technique donnée par le mainteneur), `MYIRO1_BANDEAU_MM = 22.0` (choix) ;
  - voisines : `CONTRASTE_MIN_DE76 = 10.0`.
- Calcul de la capacité MYIRO-1 :
  - plages par bande : ⌊(257 − 2 × 10) / côté⌋, borné par ⌊(largeur − 2 × 5 − 2 × 10) / côté⌋ ;
  - bandes par page : le plus grand n tel que n × côté + (n − 1) × 3 ≤ hauteur − 2 × 5 − 22 (page en portrait) ;
  - chaque bande, blancs de bout compris, est centrée dans la largeur.

- [ ] **Step 1 : Tests en échec**

```rust
#[test]
fn les_tailles_validees_sont_celles_du_mainteneur() {
    assert_eq!([Rapide, Standard, Approfondi].map(nombre_plages_myiro1), [232, 464, 928]);
}
#[test]
fn la_capacite_par_defaut_tient_les_tailles_en_1_1_2_pages_a4() {
    let my = Gabarit::par_defaut(Format::A4);
    assert_eq!((my.colonnes(), my.rangees(), my.plages_par_page()), (22, 24, 528));
    assert_eq!([232, 464, 928].map(|n| my.pages(n)), [1, 1, 2]);
}
#[test] fn la_capacite_a3_est_1015() {}
#[test]
fn un_cote_hors_limites_est_refuse() {
    assert_eq!(Gabarit::nouveau(Format::A4, 6.0), Err(ErreurMire::CoteHorsLimites { min: 7.0, max: None }));
    assert!(Gabarit::nouveau(Format::A4, 7.0).is_ok());
}
#[test] fn une_bande_myiro1_ne_depasse_jamais_257_mm_blancs_compris() {}   // pour les côtés de 7 à 30 mm, A4 et A3
#[test] fn la_zone_de_texte_est_a_2_mm_au_moins_des_plages() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p mires`
Expected: échec de compilation (crate absente).

- [ ] **Step 3 : Implémenter le modèle et `gabarit.rs`** selon les interfaces et les formules ci-dessus.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p mires && cargo clippy -p mires --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add Cargo.toml .github/workflows/ci.yml crates/mires
git commit -m "Mires : gabarit MYIRO-1 tiré du manuel, tailles validées"
```

---

### Tâche 1 bis : Mire FD-9 RGB-2520, reprise telle quelle

**Files:**
- Create: `crates/mires/src/rgb2520.rs`
- Modify: `crates/mires/Cargo.toml` (`tiff`), `crates/mires/src/lib.rs` (`pub use`), `.gitignore` (`mire-fd9/`)
- Test: `crates/mires/tests/rgb2520.rs`

**Interfaces:**
- Produces :
  ```rust
  pub const RGB2520_FICHIER: &str = "RGB-2520 A3 - 1p.tif";
  pub const RGB2520_EMPREINTE: &str = "7550f4a38725ad9260982152bb6bcbe077922289adb48dd954f0bd7d2c7fc713";
  pub const RGB2520_NOM: &str = "RGB-2520 - 1P MY9";        // nom porté par son QR code
  /// Géométrie mesurée sur le TIFF le 26 octobre 2026, en pixels à 300 ppp.
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct GeometrieFd9 { pub largeur_px: u32, pub hauteur_px: u32, pub origine_px: (u32, u32), pub pas_px: u32,
                            pub colonnes: usize, pub rangees: usize }
  pub const RGB2520_GEOMETRIE: GeometrieFd9 = GeometrieFd9 { largeur_px: 3300, hauteur_px: 4961, origine_px: (159, 285),
                                                            pas_px: 71, colonnes: 42, rangees: 60 };
  #[derive(Clone, Debug, PartialEq)]
  pub struct MireFd9 { pub octets: Vec<u8>, pub empreinte: String, pub geometrie: GeometrieFd9, pub valeurs: Vec<[f64; 3]> /* 0–100, ordre des plages */ }
  /// Lit et vérifie la mire : empreinte d'abord, puis TIFF RVB 8 bits, dimensions et 2 520 plages.
  pub fn lire_mire_fd9(octets: Vec<u8>) -> Result<MireFd9, ErreurMire>;
  /// Même lecture, sans contrôle d'empreinte : pour les tests sur un TIFF fictif de même forme.
  pub fn lire_mire_fd9_sans_empreinte(octets: Vec<u8>, geometrie: &GeometrieFd9) -> Result<MireFd9, ErreurMire>;
  ```
- Lecture des valeurs : pixel au centre de chaque plage (origine + pas × indice + pas / 2), octet vers 0–100 par `v / 2,55`. Le pas de 71 pixels vaut 6,011 mm : le décalage cumulé reste sous 1,2 pixel sur 60 rangées, loin du bord d'une plage.
- Contrôles de forme : TIFF lisible par la crate `tiff`, RVB 8 bits, dimensions égales à la géométrie, blanc (255, 255, 255) à 3 pixels autour de la zone, plages au nombre de `colonnes × rangees`. Tout écart donne `MireFd9Illisible`, avec la cause.

- [ ] **Step 1 : Tests en échec**

```rust
fn tiff_fictif() -> (Vec<u8>, GeometrieFd9) { /* 6 × 4 plages de 5 px, origine (3, 3), couleurs connues, écrit avec la crate tiff */ }
#[test] fn les_valeurs_d_une_mire_fictive_se_lisent_dans_l_ordre_des_plages() {}
#[test] fn une_mire_fd9_d_une_autre_empreinte_est_refusee() {}       // lire_mire_fd9(tiff_fictif) → MireFd9Autre { empreinte }
#[test] fn des_dimensions_differentes_sont_refusees() {}              // MireFd9Illisible
#[test] fn un_fichier_qui_n_est_pas_un_tiff_est_refuse() {}
#[test] #[ignore] fn la_vraie_mire_rgb2520_se_lit() {}
// variable MIRE_FD9 ou mire-fd9/ du dépôt : 2 520 valeurs ; plage 1 = (85, 255, 127) / 2,55 ;
// et, si MESURE_REELLE est posée, les RVB de l'export du 26 octobre diffèrent d'au plus 1/255 sur 8 plages au plus
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p mires --test rgb2520`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `rgb2520.rs`**, et ajouter `mire-fd9/` à `.gitignore`.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p mires && cargo test -p mires --test rgb2520 -- --ignored --nocapture`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add .gitignore crates/mires
git commit -m "Mires : mire FD-9 RGB-2520 reprise telle quelle, empreinte et géométrie contrôlées"
```

---

### Tâche 2 : Générer les plages MYIRO-1 avec `targen` et lire le `.ti1`

**Files:**
- Modify: `crates/profilage/src/moteur.rs`, `crates/profilage/src/argyll.rs`
- Create: `crates/mires/src/ti1.rs`
- Test: `crates/profilage/tests/moteur.rs`, `crates/profilage/tests/argyll.rs` (`#[ignore]`), `crates/mires/tests/ti1.rs`

**Interfaces:**
- Consumes : `MoteurProfil`, `ArgyllCms`, `MoteurSimule`, `ErreurMoteur`, `Etape`, `DossierTravail` (plan 3, tâche 3).
- Produces, dans `profilage` :
  ```rust
  pub struct PlagesGenerees { pub ti1: String, pub version: String, pub commande: String, pub journal: String }
  pub fn arguments_targen(nombre: usize, base: &Path) -> Vec<OsString>;   // -v -d2 -G -f<nombre> <base>
  // ajoutés au trait MoteurProfil :
  fn commande_plages(&self, nombre: usize) -> String;                     // « targen -v -d2 -G -f232 mire », sans rien lancer
  fn generer_plages(&self, nombre: usize) -> Result<PlagesGenerees, ErreurMoteur>;
  // Etape reçoit la variante Plages ; ErreurMoteur::cle rend « mire.erreur.generation » pour Echec { etape: Plages, .. }
  impl MoteurSimule { pub fn avec_plages(self, ti1: String) -> Self; }   // generer_plages rend ce ti1 ; sans lui, Echec { etape: Plages }
  ```
- Produces, dans `mires` :
  ```rust
  /// Valeurs RVB du `.ti1`, de 0 à 100, dans l'ordre des SAMPLE_ID.
  pub fn lire_ti1(texte: &str, attendu: usize) -> Result<Vec<[f64; 3]>, ErreurMire>;
  ```
  `lire_ti1` exige l'en-tête `CTI1` et les colonnes `RGB_R`, `RGB_G`, `RGB_B`. Il ignore les autres colonnes (`XYZ_*`), refuse une valeur hors de 0 à 100 ou illisible (`Ti1`) et un nombre de lignes différent de `attendu` (`NombreInattendu`).

- [ ] **Step 1 : Tests en échec**

```rust
// profilage/tests/moteur.rs
#[test]
fn targen_reçoit_le_nombre_et_le_rvb() {
    let a: Vec<_> = arguments_targen(232, Path::new("mire")).iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert_eq!(a, ["-v", "-d2", "-G", "-f232", "mire"]);
}
#[test] fn la_commande_des_plages_se_montre_sans_rien_lancer() {}   // MoteurSimule : commande_plages(464) contient "-f464", aucune demande enregistrée
#[test] fn le_moteur_simule_rend_son_ti1() {}
// profilage/tests/argyll.rs
#[test] #[ignore] fn targen_rend_le_nombre_de_plages_demande() {}  // vrai ArgyllCMS : 232 demandées → lire_ti1(…, 232) réussit, version "3.5.0"
// mires/tests/ti1.rs (fichiers .ti1 fictifs écrits dans le test)
#[test] fn un_ti1_rvb_se_lit_dans_l_ordre() {}
#[test] fn un_ti1_au_nombre_inattendu_est_refuse() {}               // 3 lignes, 4 attendues → NombreInattendu { attendu: 4, trouve: 3 }
#[test] fn une_valeur_hors_echelle_est_refusee() {}                 // RGB_R 100.5 → Ti1
#[test] fn un_ti1_sans_colonnes_rvb_est_refuse() {}                 // colonnes CMYK_* seules → Ti1
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p profilage -p mires`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter**

`ArgyllCms::generer_plages` suit le même déroulé que `calculer` :
1. contrôler la version ;
2. préparer le dossier de travail (garde `DossierTravail`) ;
3. lancer `bin/targen` avec `arguments_targen(nombre, "mire")` et `CREATE_NO_WINDOW` ;
4. lire `mire.ti1`.

La commande affichée réduit les chemins à leur nom.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p profilage -p mires`, puis `cargo test -p profilage --test argyll -- --ignored targen_rend_le_nombre_de_plages_demande`
Expected: PASS. Le test réel sert aussi à confirmer que `-f` compte les plages blanches et noires. Si ce n'est pas le cas, corriger l'argument et le noter dans la PR.

- [ ] **Step 5 : Commit**

```bash
git add crates/profilage crates/mires
git commit -m "Mires : plages RVB choisies par targen, lecture du .ti1"
```

---

### Tâche 3 : Placer les plages MYIRO-1 et vérifier la mise en page

**Files:**
- Create: `crates/mires/src/mise_en_page.rs`
- Test: `crates/mires/tests/mise_en_page.rs`

**Interfaces:**
- Consumes : `Gabarit` (tâche 1) ; `colorimetrie::xyz_vers_lab`, `colorimetrie::Xyz`, `colorimetrie::Lab`.
- Produces :
  ```rust
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct PlagePlacee { pub index: usize /* rang dans les valeurs */, pub rangee: usize, pub colonne: usize }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Page { pub numero: usize /* à partir de 1 */, pub plages: Vec<PlagePlacee> }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct MiseEnPage { pub gabarit: Gabarit, pub graine: u64, pub pages: Vec<Page> }
  #[derive(Debug, PartialEq)]
  pub enum Defaut {
      ContrasteVoisine { page: usize, rangee: usize, colonne: usize },
      ContrasteBlanc { page: usize, rangee: usize, colonne: usize },
      Nombre { attendu: usize, trouve: usize },
      Doublon { index: usize },
      HorsGabarit { page: usize, rangee: usize, colonne: usize },
  }
  pub fn estimer_lab(rvb: [f64; 3]) -> Lab;   // RVB 0–100 lu comme sRGB, Lab sous D65 : estimation pour la mise en page seulement
  pub fn mettre_en_page(valeurs: &[[f64; 3]], gabarit: &Gabarit, graine: u64) -> Result<MiseEnPage, ErreurMire>;
  pub fn verifier(mise: &MiseEnPage, valeurs: &[[f64; 3]]) -> Vec<Defaut>;
  impl MiseEnPage { pub fn empreinte(&self) -> String; }   // SHA-256 hexadécimal en minuscules du JSON de la mise en page
  ```
- Règles de contraste, en ΔE76 sur `estimer_lab`, strictement supérieur à `CONTRASTE_MIN_DE76` :
  - avec la voisine de gauche dans la bande ; pas de règle avec la bande du dessus : les bandes MYIRO-1 sont lues une à une ;
  - avec le blanc papier (RVB 100, 100, 100), pour la première et la dernière plage de chaque bande.
- Algorithme de `mettre_en_page` :
  1. Mélanger les rangs avec un générateur déterministe. Ce générateur est écrit dans la crate (splitmix64) et amorcé par `graine`, sans dépendance `rand`.
  2. Remplir les pages dans l'ordre de lecture : rangée par rangée, puis page par page.
  3. Réparer par échanges. Tant qu'une plage est en défaut, l'échanger avec une autre position tirée par le générateur, à condition que l'échange réduise le nombre total de défauts. On s'arrête quand il n'y a plus de défaut, ou après 50 × n tentatives.
  4. S'il reste des défauts, rendre `Contraste { defauts }`.

  La dernière rangée de la dernière page peut rester incomplète.

- [ ] **Step 1 : Tests en échec**

```rust
fn grille(pas: usize) -> Vec<[f64; 3]> { /* cube RVB régulier de pas³ valeurs, de 0 à 100 */ }
#[test]
fn une_mire_myiro1_approfondie_se_place_sans_defaut() {
    let v = grille(9)[..928].to_vec();                    // 928 plages
    let g = Gabarit::par_defaut(Format::A4);
    let m = mettre_en_page(&v, &g, 7).unwrap();
    assert_eq!(m.pages.len(), 4);
    assert!(verifier(&m, &v).is_empty());
}
#[test] fn une_mire_myiro1_rapide_se_place_sans_defaut() {}   // grille(6) = 216 plages, A4, 1 page
#[test] fn chaque_valeur_est_placee_une_seule_fois() {}
#[test] fn la_meme_graine_donne_la_meme_mise_en_page() {}
#[test] fn deux_graines_donnent_deux_empreintes() {}
#[test] fn des_plages_toutes_claires_sont_refusees() {}    // 30 valeurs entre 95 et 100 → Err(Contraste { .. })
#[test] fn verifier_signale_deux_voisines_identiques() {}  // mise en page fabriquée à la main
#[test] fn verifier_signale_une_plage_claire_au_bout_d_une_bande() {}
#[test] fn la_mise_en_page_se_relit_a_l_identique() {}     // JSON aller-retour, même empreinte
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p mires --test mise_en_page`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `mise_en_page.rs`** selon les règles et l'algorithme ci-dessus.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p mires`
Expected: PASS. La mise en page de 928 plages prend moins de 2 secondes en mode debug ; sinon, revoir la réparation.

- [ ] **Step 5 : Commit**

```bash
git add crates/mires
git commit -m "Mires : placement des plages avec contraste entre voisines, vérification et empreinte"
```

---

### Tâche 4 : Dessiner une page MYIRO-1, QR code, texte et TIFF

**Files:**
- Create: `crates/mires/src/rendu.rs` (image, texte, QR), `crates/mires/src/tiff.rs` (écriture PackBits)
- Modify: `crates/mires/Cargo.toml` (`qrcode`, `font8x8` ; en développement `rqrr`)
- Test: `crates/mires/tests/rendu.rs`

**Interfaces:**
- Consumes : `MiseEnPage`, `Gabarit::origine_plage`, `Gabarit::zone_texte` (tâches 1 et 3).
- Produces :
  ```rust
  pub const RESOLUTION_PPP: u32 = 300;
  pub struct ImageRvb { pub largeur: u32, pub hauteur: u32, pub pixels: Vec<u8> }   // RVB entrelacé, fond blanc 255
  pub struct Etiquette { pub lignes: Vec<String>, pub code: String }
  #[derive(Debug, PartialEq)] pub struct CodeTirage { pub tirage: i64, pub page: usize, pub pages: usize, pub empreinte: String }
  pub fn contenu_qr(code: &CodeTirage) -> String;      // "MYIRO-LIBRE:T42:P1/2:1A2B3C4D" (empreinte : 8 premiers caractères, en majuscules)
  pub fn lire_qr(texte: &str) -> Option<CodeTirage>;  // inverse de contenu_qr
  pub fn rendre_page(mise: &MiseEnPage, valeurs: &[[f64; 3]], page: usize, etiquette: &Etiquette) -> Result<ImageRvb, ErreurMire>;
  pub fn ecrire_tiff(image: &ImageRvb, sortie: &mut impl std::io::Write) -> Result<(), ErreurMire>;
  ```
- Conversions : mm vers pixels par `round(mm × 300 / 25,4)`, appliqué à chaque bord pour que deux plages voisines se touchent sans trou. Valeur 0–100 vers octet par `round(v × 2,55)`.
- Texte : `font8x8` (table latine pour les accents), agrandi 4 fois, soit environ 2,7 mm par ligne, en noir. Une ligne trop longue pour la zone de texte est coupée et finit par « … ».
- QR code : modules de 0,6 mm (7 px), marge blanche de 4 modules, placé à droite de la zone de texte, et le texte à sa gauche.
- TIFF : petit-boutiste, un seul IFD, compression PackBits (32773), photométrie RVB (2), 8 bits par canal, `XResolution` et `YResolution` à 300/1, unité en pouces, `Software` = `myiro-libre`. Ni étiquette ICC (34675), ni autre étiquette de couleur.

- [ ] **Step 1 : Tests en échec**

```rust
#[test]
fn une_page_myiro1_a4_portrait_fait_2480_par_3508_pixels() {}
#[test]
fn une_plage_a_la_bonne_couleur_au_bon_endroit() {}       // centre de la plage (rangée 0, colonne 0) = octets de sa valeur
#[test]
fn le_tiff_se_relit_et_n_a_pas_de_profil() {
    // relu avec la crate `tiff` : mêmes dimensions et mêmes pixels, résolution 300, aucune étiquette 34675
}
#[test]
fn le_qr_d_une_page_se_relit() {
    // rqrr sur l'image rendue → texte décodé ; lire_qr(texte) == Some(CodeTirage { tirage: 42, page: 2, pages: 4, empreinte: "1A2B3C4D".into() })
}
#[test] fn contenu_qr_et_lire_qr_font_l_aller_retour() {}
#[test] fn un_texte_qui_n_est_pas_a_nous_n_est_pas_lu() {}  // lire_qr("https://exemple.org") == None
#[test] fn une_ligne_trop_longue_est_coupee_par_des_points() {}
#[test] fn le_texte_reste_dans_sa_zone() {}               // aucun pixel non blanc hors des plages, de la zone de texte et du QR
#[test] fn les_accents_sont_dessines() {}                 // « é » et « e » ne donnent pas les mêmes pixels
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p mires --test rendu`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `rendu.rs` et `tiff.rs`.** Le codage PackBits suit la spécification TIFF 6.0, section 9, ligne par ligne.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p mires && cargo clippy -p mires --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add crates/mires
git commit -m "Mires : pages TIFF RVB sans profil, texte lisible et QR code"
```

---

### Tâche 5 : Ranger les mires et leurs tirages dans la bibliothèque

**Files:**
- Create: `crates/bibliotheque/src/mires.rs`
- Modify: `crates/bibliotheque/src/lib.rs` (`VERSION_BASE` = 6, `ORGANISATION_6`, `pub use`)
- Test: `crates/bibliotheque/tests/mires.rs`

**Interfaces:**
- Consumes : `IdCondition`, `ErreurBibliotheque`, `verifier_condition` (crate `bibliotheque`).
- Produces (la crate `bibliotheque` ne dépend pas de `mires` : valeurs et mise en page y sont du JSON opaque) :
  ```rust
  pub struct IdMire(pub i64);
  pub struct IdTirage(pub i64);
  pub struct NouvelleMire<'a> {
      pub nom: &'a str, pub instrument: &'a str /* "fd9" | "myiro1" */, pub plages: usize,
      pub generation: &'a str /* JSON : commande, version, journal (MYIRO-1) ; fichier repris tel quel (FD-9) */,
      pub valeurs: &'a str, pub mise_en_page: &'a str /* MiseEnPage (MYIRO-1) ou GeometrieFd9 (FD-9) */,
      pub empreinte: &'a str /* empreinte de la mise en page (MYIRO-1) ou SHA-256 du TIFF (FD-9) */, pub horodatage: &'a str,
  }
  pub struct ResumeTirage { pub id: IdTirage, pub mire: IdMire, pub nom: String, pub instrument: String, pub plages: usize, pub pages: usize, pub horodatage: String }
  pub struct MireRangee { pub id: IdMire, pub nom: String, pub instrument: String, pub plages: usize, pub generation: String, pub valeurs: String, pub mise_en_page: String, pub empreinte: String }
  impl Bibliotheque {
      pub fn enregistrer_mire_et_tirage(&self, condition: IdCondition, mire: &NouvelleMire, pages: usize) -> Resultat<(IdMire, IdTirage)>; // une transaction ;
      // une mire de même empreinte déjà rangée est reprise, pas dupliquée : chaque préparation FD-9 ajoute un tirage de la même mire RGB-2520
      pub fn tirages(&self, condition: IdCondition) -> Resultat<Vec<ResumeTirage>>;   // du plus récent au plus ancien
      pub fn mire_du_tirage(&self, tirage: IdTirage) -> Resultat<(IdCondition, MireRangee)>;
  }
  ```
- `ORGANISATION_6` :
  ```sql
  CREATE TABLE mires (
      id INTEGER PRIMARY KEY, nom TEXT NOT NULL,
      instrument TEXT NOT NULL CHECK (instrument IN ('fd9', 'myiro1')),
      plages INTEGER NOT NULL CHECK (plages > 0), generation TEXT NOT NULL,
      valeurs TEXT NOT NULL, mise_en_page TEXT NOT NULL, empreinte TEXT NOT NULL UNIQUE, horodatage TEXT NOT NULL
  );
  CREATE TABLE tirages (
      id INTEGER PRIMARY KEY, condition INTEGER NOT NULL REFERENCES conditions (id),
      mire INTEGER NOT NULL REFERENCES mires (id), pages INTEGER NOT NULL CHECK (pages > 0),
      horodatage TEXT NOT NULL, etat TEXT NOT NULL DEFAULT 'prepare' CHECK (etat IN ('prepare'))
  );
  CREATE INDEX tirages_par_condition ON tirages (condition);
  ```
  L'état `prepare` est le seul pour l'instant. Le plan 8 ajoutera « en attente de mesure » par une nouvelle organisation.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn une_mire_et_son_tirage_se_rangent_et_se_relisent() {}
#[test] fn deux_tirages_de_la_meme_mire_partagent_une_seule_mire() {}     // même empreinte → même IdMire, deux IdTirage
#[test] fn une_base_en_organisation_5_passe_en_6_sans_rien_perdre() {}   // parc, profils et mesures intacts
#[test] fn un_tirage_sur_une_condition_inconnue_est_refuse_sans_ranger_la_mire() {}
#[test] fn les_tirages_d_une_resolution_vont_du_plus_recent_au_plus_ancien() {}
#[test] fn un_instrument_inconnu_est_refuse() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p bibliotheque --test mires`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `mires.rs` et `ORGANISATION_6`**, en suivant le style des organisations précédentes (commentaire en tête, rien n'est réécrit ni effacé).

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p bibliotheque`
Expected: PASS, tests existants compris.

- [ ] **Step 5 : Commit**

```bash
git add crates/bibliotheque
git commit -m "Bibliothèque : mires et tirages rangés avec leur résolution (organisation 6)"
```

---

### Tâche 6 : Préparer une mire dans l'application

**Files:**
- Create: `app/src/mires.rs`
- Modify: `app/src/lib.rs` (module et commandes seulement), `app/Cargo.toml` (`mires`), `app/src/textes.rs`
- Test: `app/tests/mires.rs`

**Interfaces:**
- Consumes : tâches 1 à 5 ; `MoteurProfil`, `MoteurSimule`, `ArgyllCms`, `chercher_argyll` (`profilage`) ; `BibliothequeOuverte` et `detail_reglage` (`app/src/parc.rs`) ; le nettoyage de nom de `bibliotheque::nom_profil_propose`.
- Produces :
  ```rust
  #[derive(Deserialize)]
  pub struct DemandeMire { pub instrument: Instrument, pub taille: Option<TailleMire>, pub nombre: Option<usize> /* avancé, 24 à 3000 */,
                           pub format: Option<Format>, pub cote_mm: Option<f64> /* avancé */ }
  // FD-9 : taille, nombre, format et côté sont refusés s'ils sont donnés (« mire.erreur.fd9_fixe ») : la mire est RGB-2520.
  /// Cherche `mire-fd9/RGB-2520 A3 - 1p.tif` à côté de l'exécutable, puis deux niveaux plus haut, comme `chercher_argyll`.
  pub fn chercher_mire_fd9(exe: &Path) -> Option<PathBuf>;
  #[derive(Serialize)]
  pub struct ApercuMire { pub plages: usize, pub pages: usize, pub cote_mm: f64, pub limites_cote: (f64, Option<f64>), pub commande: String }
  #[derive(Serialize)]
  pub struct MirePreparee { pub tirage: i64, pub fichiers: Vec<PathBuf>, pub dossier: PathBuf, pub plages: usize, pub pages: usize, pub commande: String }
  impl BibliothequeOuverte {
      pub fn apercu_mire(&self, demande: &DemandeMire, moteur: &dyn MoteurProfil) -> Result<ApercuMire, String>;          // ne lance rien
      pub fn preparer_mire(&self, condition: IdCondition, demande: &DemandeMire, moteur: &dyn MoteurProfil,
                           mire_fd9: Option<&Path>, dossier: &Path, maintenant: SystemTime) -> Result<MirePreparee, String>; // Err = clé de texte
  }
  pub fn nom_fichier_page(imprimante: &str, papier: &str, resolution: &str, date: &str, tirage: i64, page: usize, pages: usize) -> String;
  // « Canon PRO-4100 - Baryta - 600 dpi - 2026-10-23 - T42 - page 1 sur 2.tif »
  ```
- Commandes Tauri : `mire_apercu`, `mire_preparer` et `mire_ouvrir_dossier`. Le dossier est `Documents\myiro-libre\Mires` (`app.path().document_dir()`), et l'Explorateur est lancé avec `CREATE_NO_WINDOW`.
- Déroulé de `preparer_mire` pour le **FD-9** (`apercu_mire` rend 2 520 plages, 1 page, aucune commande) :
  1. `mire_fd9` absent : `mire.erreur.fd9_introuvable`, avec le dossier attendu dans le message.
  2. Lire le fichier, puis `lire_mire_fd9` : empreinte, forme et 2 520 valeurs.
  3. `enregistrer_mire_et_tirage` (nom `RGB-2520`, instrument `fd9`, 1 page, valeurs, géométrie, empreinte du TIFF).
  4. Copier les octets lus, sans aucune modification, dans `dossier` sous `nom_fichier_page(…, 1, 1)`, d'abord sous un nom `.part`, renommé une fois écrit. Aucun texte ni QR n'est ajouté : la mire garde les siens.
- Déroulé de `preparer_mire` pour le **MYIRO-1**, sans tenir le verrou de la bibliothèque pendant `targen` :
  1. Contrôler le gabarit et le nombre. Le côté et le nombre sont refusés avant tout lancement.
  2. `generer_plages`, puis `lire_ti1`.
  3. `mettre_en_page`, avec une graine tirée de l'horloge, puis `verifier` : il ne doit rester aucun défaut, et le nombre de plages doit être exact.
  4. `enregistrer_mire_et_tirage`.
  5. Rendre et écrire chaque page dans `dossier`, d'abord sous un nom `.part`, renommé une fois écrit. Si une écriture échoue, les fichiers de ce tirage sont effacés et la cause est rendue. La mire reste rangée : elle peut être redessinée à l'identique.
  6. Les lignes du texte lisible :
     - « <imprimante> · <papier> · <résolution> » ;
     - « Mire <taille ou nombre> · <n> plages · <instrument> · page k sur n · <date> » ;
     - « Imprimer à 100 %, sans gestion des couleurs ».
- Clés de texte à créer (français et anglais) : `mire.*` pour les libellés de la section, la carte FD-9 (« RGB-2520 · 2 520 plages · 1 page A3 »), les tailles, les formats, l'aide de l'avancé, le rappel d'impression, et les erreurs de `ErreurMire::cle`, `mire.erreur.generation` et `mire.erreur.fd9_fixe`.

- [ ] **Step 1 : Tests en échec (`app/tests/mires.rs`, avec `MoteurSimule::avec_plages` sur un `.ti1` fictif)**

```rust
#[test] fn une_mire_preparee_ecrit_ses_pages_et_range_son_tirage() {}   // 232 plages MYIRO-1 A4 → 1 fichier, tirages(condition).len() == 1
#[test] fn une_mire_de_deux_pages_ecrit_deux_fichiers_nommes() {}        // 464 plages MYIRO-1 A4 : « page 1 sur 2 », « page 2 sur 2 »
#[test] fn la_mire_fd9_est_copiee_telle_quelle() {}                       // TIFF fictif via lire_mire_fd9_sans_empreinte : fichier déposé == octets d'origine
#[test] fn une_mire_fd9_absente_ne_prepare_rien() {}                      // Err("mire.erreur.fd9_introuvable"), aucun fichier, aucun tirage
#[test] fn une_mire_fd9_n_accepte_ni_taille_ni_format() {}                // Err("mire.erreur.fd9_fixe")
#[test] fn deux_preparations_fd9_donnent_deux_tirages_d_une_meme_mire() {}
#[test] fn le_nom_de_fichier_est_nettoye() {}                            // « Canson « Baryta » / 2 » sans « « » / »
#[test] fn l_apercu_dit_plages_pages_et_commande_sans_rien_lancer() {}
#[test] fn un_cote_hors_limites_ne_lance_pas_targen() {}                 // MoteurSimule : aucune demande enregistrée
#[test] fn argyll_introuvable_ne_prepare_rien() {}                       // Err("profil.erreur.argyll_introuvable"), aucun fichier, aucun tirage
#[test] fn un_refus_de_mise_en_page_ne_range_rien() {}                   // ti1 de plages toutes claires → Err("mire.erreur.contraste")
#[test] fn toutes_les_cles_mire_existent_en_deux_langues() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test mires`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `app/src/mires.rs`** et les trois commandes.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p app && cargo clippy --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add app
git commit -m "Application : préparer une mire, écrire ses pages TIFF et ranger son tirage"
```

---

### Tâche 7 : Section « Préparer une mire » dans la vue d'une résolution

**Files:**
- Modify: `app/interface/reglage.js`, `app/interface/index.html` (si un gabarit HTML est nécessaire), `app/src/textes.rs`
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes : commandes `mire_apercu`, `mire_preparer`, `mire_ouvrir_dossier` (tâche 6) ; `agir()` et `annoncer()` de `reglage.js` ; cartes et section repliable du `design-system/`.

Contenu de la section, dans l'ordre :
1. **Instrument** : deux cartes, FD-9 (par défaut) et MYIRO-1.
   - FD-9 choisi : la carte dit « Mire RGB-2520 · 2 520 plages · 1 page A3 », et les points 2 à 4 ne s'affichent pas.
2. **Taille** (MYIRO-1) : trois cartes, Rapide, Standard et Approfondi, avec pour chacune le nombre de plages et de pages.
3. **Format** (MYIRO-1) : A4 (par défaut) et A3.
4. **Avancé** (MYIRO-1), replié. Fermé, il résume ses valeurs (« 8 mm, 232 plages »). Ouvert, il montre :
   - le côté de plage, avec ses limites écrites à côté ;
   - le nombre libre ;
   - la commande `targen` équivalente, en lecture seule.
5. Bouton principal **« Préparer la mire »**. Pendant le travail, « Action en cours » s'affiche à côté, comme ailleurs.
6. Après la préparation :
   - la liste des fichiers ;
   - le bouton « Ouvrir le dossier » ;
   - le rappel « Imprimez à 100 %, sans gestion des couleurs (dans le pilote et dans le logiciel d'impression) ».
7. **Mires préparées** : les tirages de la résolution (date, instrument, plages, pages).

Les choix de la section sont gardés par résolution pendant la séance (pas de stockage). Chaque changement de choix redemande `mire_apercu`.

- [ ] **Step 1 : Tests en échec (`app/tests/page.rs`)**

```rust
#[test] fn la_vue_d_une_resolution_prepare_une_mire() {}                 // reglage.js contient "mire_preparer", "mire_apercu", "mire_ouvrir_dossier"
#[test] fn toutes_les_cles_mire_de_la_page_existent_en_deux_langues() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test page`
Expected: FAIL.

- [ ] **Step 3 : Implémenter la section** dans `reglage.js`, sans texte en dur.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p app`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add app
git commit -m "Interface : section « Préparer une mire » dans la vue d'une résolution"
```

---

### Tâche 8 : Contrôler la cohérence des mesures avant un calcul (#1)

Le logiciel du fabricant a calculé un profil sur une mesure de 2 520 plages dont les mesures ne correspondaient pas aux plages, sans avertissement (`docs/references/icc-gamut-maximal.md`, section 10). myiro-libre refuse ce cas avant d'appeler ArgyllCMS.

**Files:**
- Create: `crates/profilage/src/coherence.rs`
- Modify: `crates/profilage/Cargo.toml` (`colorimetrie`), `crates/profilage/src/lib.rs` (`pub use`), `app/src/profiler.rs`, `app/src/textes.rs`, `app/interface/calcul-profil.js`
- Test: `crates/profilage/tests/coherence.rs`, `app/tests/profiler.rs`, `app/tests/page.rs`

**Interfaces:**
- Consumes : `PlagesProfil`, `PlageProfil` (plan 3) ; `colorimetrie::delta_e00`, `colorimetrie::Lab`.
- Produces, dans `profilage` :
  ```rust
  // Seuils : « choix, à vérifier » (voir « Choix de l'assistant »), chacun commenté.
  pub const RETOUR_GRIS_MAX_L: f64 = 3.0;
  pub const ECART_BLANC_MAX_L: f64 = 2.0;
  pub const ECART_NOIR_MAX_L: f64 = 5.0;
  pub const ECART_DOUBLONS_MAX_DE00: f64 = 3.0;
  pub const ECART_VOISINES_MAX_DE00: f64 = 20.0;
  pub const VOISINES: usize = 6;
  pub const DECALAGE_MAX: usize = 60;
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Critere { DegradeGris, BlancNoir, Doublons, Voisines }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct PlageSuspecte { pub rang: usize /* à partir de 1, dans l'ordre du fichier */, pub critere: Critere, pub ecart: f64 }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct ControleCoherence {
      pub faits: Vec<Critere>,
      pub non_faits: Vec<Critere>,            // contrôle impossible : moins de 3 gris, pas de blanc ou de noir, aucun doublon
      pub suspectes: Vec<PlageSuspecte>,      // triées par rang
      pub decalage: Option<i64>,              // piste seulement : comparer le RVB de la plage i au Lab de la plage i + s ne laisse aucune suspecte
  }
  impl ControleCoherence { pub fn accepte(&self) -> bool; }   // aucune plage suspecte
  pub fn controler_coherence(plages: &PlagesProfil) -> ControleCoherence;
  ```
- Règles, sur les RVB de 0 à 100 et les Lab des plages :
  - **Gris** : plages dont R, G et B diffèrent de moins de 0,5. Triées par RVB croissant, leur L* ne recule pas de plus de `RETOUR_GRIS_MAX_L` d'un gris au suivant ; les deux plages d'un recul sont suspectes. Moins de 3 gris : critère non fait.
  - **Blanc et noir** : la plage RVB (100, 100, 100) a un L* à moins de `ECART_BLANC_MAX_L` du plus grand L* de la mesure ; la plage RVB (0, 0, 0) a un L* à moins de `ECART_NOIR_MAX_L` du plus petit. Plage absente : critère non fait pour elle.
  - **Doublons** : plages de même RVB (au centième près) ; plus de `ECART_DOUBLONS_MAX_DE00` entre deux d'entre elles les rend suspectes. Aucun doublon : critère non fait.
  - **Voisines** : pour chaque plage, ses `VOISINES` plus proches voisines en RVB (distance euclidienne, sans elle-même ni ses doublons) ; leur Lab moyen, pondéré par l'inverse de la distance, est comparé au sien en ΔE00. Au-delà de `ECART_VOISINES_MAX_DE00`, la plage est suspecte.
  - **Décalage**, seulement si des plages sont suspectes : pour s de −`DECALAGE_MAX` à +`DECALAGE_MAX` (sauf 0), refaire les quatre contrôles en associant le RVB de la plage i au Lab de la plage i + s, sur les plages communes. Le premier s, par valeur absolue croissante, qui ne laisse aucune suspecte est rendu. Les voisines en RVB ne dépendent pas de s : elles sont calculées une seule fois, pour que 2 520 plages restent sous 2 secondes en mode debug.
- Application (`app/src/profiler.rs`) :
  - `preparer_profil` appelle `controler_coherence` juste après `preparer_plages`. Si le contrôle refuse : `ErreurProfil` de clé `profil.refus.incoherent`, avec un nouveau champ `coherence: Option<RefusCoherence>`. Les commandes, l'estimation et le calcul refusent donc tous les trois, et la page montre le refus dès le choix de la mesure, avant le bouton « Calculer ».
  - `RefusCoherence { plages: Vec<String> /* identifiants du fichier, 10 au plus */, autres: usize, criteres: Vec<Critere>, decalage: Option<i64> }` : les rangs sont traduits en identifiants de plage du fichier (`SAMPLE_ID`).
  - Si le contrôle accepte, `"coherence"` est ajouté aux paramètres du profil rangé. `ParametresCalcul` le lit avec `#[serde(default)]`, et `ProvenanceProfil` reçoit `coherence: Option<ControleCoherence>`. Un profil calculé avant ce plan n'a pas ce champ : il est dit « mesures non contrôlées ».
- Textes (français et anglais), en mots simples :
  - `profil.refus.incoherent` : « Les mesures ne correspondent pas aux plages de la mire : le profil serait faux. Plages en cause : {plages}. Relisez la mire, vérifiez le sens de la feuille et l'ordre de lecture, et que ce fichier est bien celui de cette mire. » ;
  - `profil.coherence.decalage` : « Les mesures semblent décalées de {n} plages. » ;
  - `profil.coherence.critere.*` (une clé par critère), `profil.coherence.accepte`, `profil.coherence.non_fait` et `profil.coherence.non_controle` pour la provenance.
- Page (`calcul-profil.js`) : le refus remplace le bouton « Calculer » par son message, comme les autres refus de plages. La provenance d'un profil calculé dit « Mesures contrôlées : cohérentes », puis la liste des critères non faits s'il y en a.

- [ ] **Step 1 : Tests en échec**

```rust
// profilage/tests/coherence.rs
// Mesure synthétique de 840 plages :
//  - grille RVB 9 × 9 × 9 (729 plages, dont 9 gris, le blanc et le noir) ;
//  - 100 plages tirées par splitmix64 (graine fixe) ;
//  - 11 doublons de plages de la grille.
// L'ordre du fichier est mélangé (graine fixe), comme une mire de targen.
// Lab : RVB lu comme du sRGB, puis papier simulé (L* = 15 + 0,8 × L*, a* et b* × 0,85), plus un bruit fixe de ±0,3.
fn mesure_saine() -> PlagesProfil { /* … */ }
#[test] fn une_mesure_saine_de_840_plages_passe_sans_alerte() {}            // accepte(), faits == les 4 critères, decalage == None
#[test] fn des_mesures_decalees_d_une_plage_sont_refusees_et_le_decalage_est_dit() {}  // Lab de la plage i donné à la plage i + 1 → refus, decalage == Some(1)
#[test] fn des_mesures_melangees_sont_refusees_avec_les_plages_en_cause() {} // Lab mélangés → refus, decalage == None, suspectes non vide
#[test] fn un_gris_qui_recule_est_signale() {}                              // L* du gris 50 % mis à celui du gris 25 %
#[test] fn un_blanc_papier_trop_sombre_est_signale() {}                     // cas du ticket : RVB 0 au L* d'un gris moyen
#[test] fn des_doublons_trop_differents_sont_signales() {}
#[test] fn une_plage_isolee_est_signalee_par_son_rang() {}                  // une seule plage faussée de 30 ΔE00 → une suspecte, de ce rang
#[test] fn sans_gris_le_critere_est_dit_non_fait() {}                       // mesure sans gris : DegradeGris dans non_faits, pas de refus pour autant
#[test] fn le_controle_de_2520_plages_tient_en_2_secondes() {}
#[test] #[ignore] fn la_mesure_reelle_du_26_octobre_passe() {}             // variable MESURE_REELLE : export CGATS local, non versionné ; cgats::lire → preparer_plages (M1) → accepte()
// app/tests/profiler.rs (MoteurSimule)
#[test] fn des_mesures_incoherentes_ne_lancent_pas_argyll() {}             // aucune demande au moteur, Err(cle "profil.refus.incoherent"), aucun profil rangé, profil actuel inchangé
#[test] fn le_refus_nomme_les_plages_du_fichier() {}                       // identifiants SAMPLE_ID, 10 au plus, « autres » compte le reste
#[test] fn un_profil_calcule_garde_son_controle_dans_sa_provenance() {}
#[test] fn un_profil_calcule_avant_le_controle_se_lit_encore() {}          // paramètres sans "coherence" → coherence == None
// app/tests/page.rs
#[test] fn toutes_les_cles_de_coherence_existent_en_deux_langues() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p profilage --test coherence && cargo test -p app --test profiler`
Expected: échec de compilation.

- [ ] **Step 3 : Implémenter `coherence.rs`**, puis le branchement dans `app/src/profiler.rs`, les textes et la page.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p profilage -p app && cargo clippy --all-targets -- -D warnings`, puis, avec la mesure réelle posée sur le poste, `cargo test -p profilage --test coherence -- --ignored --nocapture`
Expected: PASS. Si la mesure réelle déclenche une alerte, ne pas relever un seuil au hasard : noter la plage et l'écart dans la PR et en parler au mainteneur.

- [ ] **Step 5 : Commit**

```bash
git add crates/profilage app
git commit -m "Profil : mesures contrôlées avant le calcul, refus en clair si elles ne correspondent pas aux plages (#1)"
```

---

### Tâche 9 : Installateur, documentation et essai avec le mainteneur

**Files:**
- Modify: `outils/preparer_installateur.py` (ajouter `argyll/bin/targen.exe` à la liste des ressources du plan 3, et embarquer `mire-fd9/RGB-2520 A3 - 1p.tif` s'il est présent, après contrôle de son empreinte), `outils/test_preparer_installateur.py`, `CLAUDE.md` (crate `mires`, module `mires` de l'application, contrôle de cohérence de `profilage`), `GLOSSARY.md`, `docs/blog/<jour>.md`, `docs/references/icc-gamut-maximal.md` (section « Ce qu'on en tire » : contrôle fait, renvoi au code)

- [ ] **Step 1 : Installateur**

Ajouter `targen.exe` aux ressources ArgyllCMS du plan 3. Si `mire-fd9/` existe, embarquer la mire RGB-2520 sous `mire-fd9/` ; une autre empreinte arrête le script ; sans le dossier, le dire en une ligne et continuer, comme pour `argyll/`.

Run: `python outils/test_preparer_installateur.py && python outils/test_verifier_docs.py && python outils/verifier_docs.py`
Expected: OK.

- [ ] **Step 2 : Glossaire**

Ajouter les termes suivants, sans collision avec « Feuille » ni « Tirage » :
- **Taille de mire** : rapide, standard, approfondi (MYIRO-1) ;
- **Mire RGB-2520** : la mire du FD-9, 2 520 plages sur une page A3, reprise telle quelle ;
- **Gabarit** : mise en page des bandes du MYIRO-1 pour un format ;
- **Plan de lecture** : ordre et place de chaque plage, page par page ;
- **Contrôle de cohérence** : vérification, avant un calcul de profil, que les mesures correspondent aux plages.

« Tirage » reste l'objet imprimé. Le tirage rangé à la préparation est sa trace.

- [ ] **Step 3 : Essai avec le mainteneur (geste humain, écran et papier)**

1. Préparer la mire FD-9 (RGB-2520) et une mire MYIRO-1 rapide A4 sur une résolution choisie par le mainteneur. Le papier exact est noté par le mainteneur, jamais supposé (#33). Noter la durée de `targen`.
2. Vérifier que le fichier FD-9 déposé a l'empreinte `7550f4a3…c7fc713`, donc qu'il est identique à l'original.
3. Imprimer la mire FD-9 sur une page A3 et la page MYIRO-1, à 100 %, sans gestion des couleurs.
4. À la règle : côté d'une plage FD-9 (6 mm environ) et MYIRO-1 (8 mm), longueur d'une bande MYIRO-1 avec ses blancs (257 mm au plus), marges de la mire FD-9 imprimée (au moins 23 mm en tête, 33 mm en pied, 4 mm sur les côtés).
5. Lire les QR codes avec un téléphone : `FD-9:011M:RGB-2520 - 1P MY9*1+` pour la mire FD-9, un texte qui commence par `MYIRO-LIBRE:T` pour la page MYIRO-1.
6. Vérifier que le texte lisible de la page MYIRO-1 est net et que les accents sont justes.
6 bis. Lire une bande MYIRO-1 qui contient une plage blanche en milieu de bande ; si la bande est coupée, repli sur `targen -e0` avec un blanc papier mesuré à part.
7. Passer une bande au MYIRO-1 dans un logiciel officiel, si le mainteneur le souhaite. La mesure au FD-9 par l'application vient au plan 6 ; d'ici là, la mire FD-9 imprimée se mesure comme aujourd'hui, et son export s'importe (plan 3).
8. Contrôle de cohérence : relancer le calcul sur la mesure du 26 octobre (2 520 plages, HFA Mat 200g) ; aucune alerte attendue, et la provenance dit « Mesures contrôlées : cohérentes ». Si le mainteneur a encore les mesures faussées de 2021, les importer : le calcul doit être refusé, avec les plages en cause.

Consigner les résultats dans le ticket #33, sans identifiant matériel.

- [ ] **Step 4 : Journal, CLAUDE.md, commit**

```bash
git commit -am "Documentation : mire FD-9 RGB-2520, mires MYIRO-1 et essai d'impression"
```

- [ ] **Step 5 : PR vers `dev`**

PR `ticket/33-mires-rvb` → `dev`, avec les « Choix de l'assistant » et les résultats de l'essai. La PR ferme #1 (« Closes #1 ») si ses quatre critères sont couverts. Le ticket #33 reste ouvert : il manque encore l'import des mires existantes, la fiche d'impression, le PDF, le CMJN et la lecture des pages au FD-9 (plan 6).

## Questions ouvertes (ne bloquent pas le plan)

- **Mesure au FD-9 par l'application (plan 6)** : le FD-9 commence toujours par scanner la feuille (« prénumérisation », manuel p. 38 à 42). Il cherche le QR code dès ce scan : le SDK contient un détecteur de QR, `PatchPosExtractor::DetectQRCode`. Avec une mire définie, il compare les plages trouvées à la définition et rend les points de mesure (`FD9_MatchingChart`, sortie `outMeasurePoint`). Sans définition, il reconnaît seul les plages (`FD9_StartPatchRecognition`, `FD9_GetPatchRecognitionResult2`). Le manuel conseille une mire définie quand la reconnaissance libre se trompe. Le plan 6 passera donc la géométrie RGB-2520 de ce plan au SDK. La forme exacte des structures est **à établir** par des fiches d'ABI, palier par palier.
- **Définition de mire de FD-S2w** : FD-S2w range ses mires définies en XML (`Identifier="ChartDefinition"` : feuilles, zones, nombre de rangées et de colonnes, taille et pas des plages, couleur de chaque plage). Écrire la définition de RGB-2520 dans ce format permettrait de la mesurer dans FD-S2w avec sélection automatique par le QR code, dès maintenant. L'unité des dimensions (centièmes de millimètre ?) est **à vérifier** sur une définition connue avant d'en écrire une.
- **Champs du QR code** `011M` et `*1+` : sens à établir (plan 6).
- **Mire imprimée centrée sur A3** : décalage d'environ 8,8 mm par rapport au fichier, toléré par le manuel (21 mm au plus) ; à confirmer à l'essai.
