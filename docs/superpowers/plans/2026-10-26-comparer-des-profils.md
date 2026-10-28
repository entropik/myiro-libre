# Comparer des profils ICC — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'imprimeur choisit de un à quatre profils RVB, rangés dans la bibliothèque ou ouverts depuis un fichier `.icc`. Il les compare dans l'application : gamut en volume, saturation par teinte, coupe à clarté constante, courbes le long d'un chemin du cube RVB, couleurs de base, chiffres clés, fidélité à leurs mesures et écarts entre profils.

**Architecture:** Une nouvelle crate `gamut`, indépendante de Tauri, de Windows et des ponts (ADR 0004, ADR 0007), fait tout le calcul. Elle lit la table A2B1 d'un profil, la passe en colorimétrie absolue et en tire la surface du cube RVB, le volume, la table de chroma, les couleurs de base, les écarts ΔE00, la fidélité, la répétabilité et la lecture d'un fichier `.gam`. La crate `profilage` reçoit un second trait, `MoteurGamut`, qui calcule la limite du gamut par `iccgamut`. Il a deux adapters, `ArgyllCms` et `MoteurGamutSimule`, sur le modèle de l'ADR 0005. L'application ajoute `app/src/comparer.rs` : ses méthodes de `BibliothequeOuverte` se testent sans Tauri, et le calcul se fait hors du verrou. La page ajoute une vue « Comparer » à la zone centrale. La vue 3D est **maison** : du Canvas 2D aux triangles triés par profondeur, sans bibliothèque externe.

**Tech Stack:** Rust 2021, crates `colorimetrie` et `serde` (aucune nouvelle dépendance externe), ArgyllCMS 3.5.0 (`iccgamut`, version figée, ADR 0002), Tauri 2, HTML, CSS, JavaScript simples, Canvas 2D et SVG (ADR 0003).

**Spec:** ticket #71 et son commentaire du 26 octobre 2026 (leçons de l'essai) ; `docs/specifications/2026-10-20-poste-unique-design.md` (« Graphiques lisibles », « Le spectre d'abord, provenance visible », « Transparence des calculs ») ; prototype validé, page « Gamuts HFA Matt 200 » (artifact du 26 octobre 2026) et ses scripts de calcul, références de méthode : `icc.py` (lecture A2B1, interpolation tétraédrique, colorimétrie absolue, ΔE00), `calcul.py` (surface, volume, table de chroma, couleurs de base, écarts et zones), `plis2.py` (lecture `.gam`), `cretes.py` et `repet.py` (chemins, répétabilité). Ces scripts restent hors du dépôt ; ce plan reprend leurs règles.

## Feuille de route (26 octobre 2026)

Le mainteneur passe la comparaison avant les mires : elle sert tout de suite à juger les profils calculés par myiro-libre (plan 3, fusionné le 26 octobre, PR #72) face à ceux d'un autre logiciel.

1. Bibliothèque : parc et profils (fait, #66).
2. Nouveau cadre d'interface (fait, #68).
3. Calcul d'un profil RVB à partir d'une mesure importée (fait, #40 première partie, PR #72).
4. **Comparer des profils** (ce plan, #71).
5. Mires de profilage (#33) : plan 4, `2026-10-23-mires-de-profilage.md`, écrit, en attente de relecture.
6. Mesure d'une feuille au FD-9 (#34), puis guidage, reprise et moyenne (#35).
7. Vérification du profil sur une seconde mire et parcours en 4 étapes (#41).
8. Séchage et reprise ; reprise de l'existant.

Ce plan couvre une partie de #42 (« Visualiser et comparer les gamuts ») : coupe, volume, chiffres et méthode. #42 reste ouvert pour la suite : profils ICC v4 (`mAB`), mesures et couleurs choisies superposées dans la coupe, rendu filaire, part hors gamut.

## Global Constraints

- Travail dans un worktree `ticket/71-comparer-profils` issu de `dev` (`.claude/worktrees/ticket-71`), PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/71-comparer-profils --reference dev`.
- La crate `gamut` ne dépend que de `colorimetrie` et de `serde` ; elle reste indépendante de Tauri, de Windows, des ponts et de `cgats`. Elle rejoint la liste `CRATES` du job linux de la CI. Aucune nouvelle dépendance externe dans tout le plan.
- Les valeurs sont lues dans la table **A2B1** (colorimétrique relative) des profils, puis passées en **colorimétrie absolue** avec le blanc du support (`wtpt`), blanc papier compris (#71).
- ArgyllCMS : version figée `3.5.0`, appelée en processus externe, sans fenêtre de console. Commande confirmée le 26 octobre 2026 : `iccgamut -v -ia -pl profil.icc` écrit `profil.gam` et affiche `Total volume of gamut is 467316.128099 cubic colorspace units`.
- Ce qui n'est pas connu reste « inconnu », avec sa raison, jamais zéro : profil sans mesures embarquées, ArgyllCMS absent, teinte d'un gris.
- La page dit ce qui est **calculé** et ce qui est **approché** (#71). Le volume n'est jamais présenté comme une note de qualité (#42).
- Un fichier `.icc` ouvert pour la comparaison n'est jamais rangé dans la bibliothèque ni écrit sur le disque. Il ne vit que le temps de la session (#71).
- La page n'a pas de CDN : la CSP de `app/tauri.conf.json` (`script-src 'self'; style-src 'self'`) ne change pas. Aucune bibliothèque JavaScript n'est ajoutée.
- Aucune couleur en dur hors de `design-system/tokens.css` : les quatre couleurs de série y sont ajoutées comme jetons. Les « couleurs réelles » des formes sont des données calculées par `colorimetrie::lab_vers_srgb`.
- Aucun texte en dur dans la page : tout passe par `app/src/textes.rs`, en français et en anglais. Thèmes clair et sombre. Vue 3D et graphiques utilisables au clavier, avec un texte de remplacement.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57). Fichiers source nouveaux sous 400 lignes. `lib.rs` de l'application ne reçoit que l'enregistrement des commandes et de l'état.
- Données de test fictives ou synthétiques : les profils d'autres logiciels et les mesures de l'atelier restent hors du dépôt (tests `#[ignore]` sur une variable d'environnement). Un fichier de `tests/` ne contient pas « install » dans son nom.
- Ne jamais écrire le nom du logiciel concurrent. Ne pas citer `docs/sources/`.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p gamut -p profilage -p app`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- **Vue 3D maison, en Canvas 2D.** Les bibliothèques 3D courantes ne passent pas la CSP sans changement (styles injectés), et elles demanderaient une licence à suivre et un fichier lourd dans l'installateur. Avec au plus quatre profils, on dessine environ 12 000 triangles : un Canvas 2D les trie par profondeur à chaque image (algorithme du peintre). C'est ce tri qui manquait au prototype : la transparence y dessinait de fausses facettes. Rendu **opaque** par défaut, ombrage plat ; transparence en option.
- **Structures ICC lues** : table A2B1 de type `mft2` (lut16), PCS Lab ou XYZ, profils v2 et v4. C'est ce qu'écrivent ArgyllCMS et les profils de l'essai. Les types `mft1` (lut8) et `mAB` (lutAtoB, courant en v4) sont **refusés en le disant**, avec le nom de la structure. Ils restent pour #42.
- **Profils RVB seulement.** Un profil CMJN (HP Latex) est refusé avec « comparaison CMJN pas encore prévue ».
- **Jusqu'à quatre profils**, le premier choisi sert de référence aux écarts. Les modes du ticket en découlent : un profil seul, deux imprimantes, avant / après (deux profils d'une même résolution) et « même mesure, deux logiciels ». Ce dernier mode est **détecté** quand deux profils embarquent les mêmes mesures.
- **Forme par défaut** : « Limite du gamut » (iccgamut) quand ArgyllCMS est là, sinon « Surface brute ». Dans les bleus sombres, la surface brute se replie (commentaire du ticket).
- **Mesures d'un profil** : celles qu'il embarque (balise `targ`, écrite par ArgyllCMS comme par le logiciel du fabricant). Elles sont lues par `cgats::lire`, qui suit ses règles d'échelle RVB. Le Lab est pris dans le fichier ; à défaut, il est calculé depuis le spectre par `colorimetrie` (D50, 2°) et dit « calculé par myiro-libre ».
- **« Mêmes mesures »** : même nombre de plages, mêmes RVB à 0,01 % près une fois triées, et un écart de 0,1 ΔE00 au plus plage par plage. Le prototype retrouvait un export à 0,02 près depuis les spectres.
- **Couleurs de série** (jetons `--serie-1` à `--serie-4`) : bleu, orange, violet, gris. Ni vert ni rouge, qui sont réservés aux verdicts. Contraste mesuré face à `--carte`, au-dessus de 4,3:1 dans les deux thèmes.

## Review Focus

1. **Profil d'une structure non lue** (v4 `mAB`, lut8, profil d'écran à matrice sans A2B1, profil CMJN) dans une comparaison de trois profils : ce profil est refusé avec la structure nommée, et les deux autres s'affichent normalement. → tests `une_table_mab_est_refusee_en_la_nommant` (tâche 1) et `un_profil_refuse_n_empeche_pas_les_autres` (tâche 5).
2. **Profil sans mesures embarquées**, ou avec des mesures dont l'échelle RVB est inconnue : la fidélité et la répétabilité sont « inconnues », avec la raison ; jamais 0,00. → test `sans_mesures_embarquees_la_fidelite_est_inconnue` (tâche 5).
3. **ArgyllCMS absent ou d'une autre version** : le bouton « Limite du gamut » est inactif et dit pourquoi, la surface brute s'affiche, et tout le reste est calculé. → tests `sans_argyll_la_limite_est_indisponible_et_le_reste_est_calcule` (tâche 5) et `une_autre_version_refuse_la_limite` (tâche 4).
4. **Le même profil choisi deux fois** (ou un avant / après sans changement) : écarts nuls partout, aucune division par zéro, les zones vides ne sont pas listées, et la page dit « profils identiques ». → test `deux_profils_identiques_donnent_des_ecarts_nuls` (tâche 3).
5. **Profil sans blanc du support (`wtpt`)** : la colorimétrie absolue est impossible, donc le profil est refusé. On ne suppose jamais un blanc D50. → test `un_profil_sans_blanc_du_support_est_refuse` (tâche 1).

---

### Tâche 0 : Préparer le chantier

**Files:** aucun.

- [ ] **Step 1 : Contexte**

Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev` puis `git status --short`
Expected: contexte conforme. Les changements préexistants (journal du 9 octobre, `.agents/`, plan 4 non commité) ne relèvent pas de ce plan.

- [ ] **Step 2 : Worktree**

Run: `git worktree add .claude/worktrees/ticket-71 -b ticket/71-comparer-profils dev`

- [ ] **Step 3 : ArgyllCMS dans le worktree**

Copier `C:\Code\MYIRO-fork\argyll\` dans `.claude/worktrees/ticket-71/argyll/` (ignoré par git). C'est le même rangement que pour le plan 3 : `bin\`, `ref\` directement sous `argyll\`. Vérifier `argyll/bin/iccgamut.exe`.

- [ ] **Step 4 : Annoncer la prise en charge sur #71** (`gh issue comment 71`), avec le lien de ce plan.

---

### Tâche 1 : Crate `gamut` — lire un profil et passer en colorimétrie absolue

**Files:**
- Create: `crates/gamut/Cargo.toml`, `crates/gamut/src/lib.rs`, `crates/gamut/src/icc.rs`, `crates/gamut/src/essai.rs`
- Modify: `Cargo.toml` (membres de l'espace de travail), `.github/workflows/ci.yml` (`-p gamut` dans `CRATES`)
- Test: `crates/gamut/tests/icc.rs`

**Interfaces:**
- Consumes: `colorimetrie::{Lab, Xyz, xyz_vers_lab}`.
- Produces:
  - `pub struct ProfilIcc` avec `pub fn lire(octets: &[u8]) -> Result<ProfilIcc, ErreurIcc>`, `pub fn version(&self) -> String` (« 2.4 »), `pub fn description(&self) -> Option<String>`, `pub fn grille(&self) -> u8`, `pub fn lab_absolu(&self, rvb: [f64; 3]) -> [f64; 3]` (RVB de 0 à 1, borné ; L\*, a\*, b\*), `pub fn blanc_support(&self) -> [f64; 3]` et `pub fn mesures_embarquees(&self) -> Option<&[u8]>`.
  - `pub enum ErreurIcc { EnTete(String), PasRvb { espace: String }, SansTable, StructureNonPriseEnCharge { structure: String }, Tronque { balise: String }, SansBlanc }`, avec `pub fn cle(&self) -> &'static str` qui rend les clés `comparer.refus.en_tete`, `comparer.refus.pas_rvb`, `comparer.refus.sans_table`, `comparer.refus.structure`, `comparer.refus.tronque` et `comparer.refus.sans_blanc`.
  - `pub struct ProfilEssai { pub grille: u8, pub blanc: [f64; 3], pub cible: fn([f64; 3]) -> [f64; 3], pub description: String, pub mesures: Option<String> }` et `pub fn profil_essai(p: &ProfilEssai) -> Vec<u8>`. C'est un profil ICC v2 synthétique de sortie RVB : PCS Lab, A2B1 `mft2` aux courbes identité dont la grille vaut `cible` (RVB 0–1 vers Lab relatif), A2B0 copie d'A2B1, `wtpt` = XYZ de `blanc`, `desc`, `cprt`, et `targ` si `mesures` est donné. `bibliotheque::espace_icc` doit l'accepter (test en dev-dependency). Si `iccgamut` le refuse en tâche 4, compléter les balises qu'il exige et le noter. Il sert aux tests de cette crate et de l'application.

Règles de lecture (reprises de `icc.py`, à suivre exactement) :
- En-tête : au moins 128 octets, `acsp` aux octets 36 à 39, espace de données (16–19) `RGB `, PCS (20–23) `Lab ` ou `XYZ `. L'illuminant du PCS est lu aux octets 68 à 79 (trois s15Fixed16).
- Table des balises à l'octet 128 : nombre (u32), puis signature, position et taille (12 octets chacune). Une balise qui sort du fichier donne `Tronque`.
- `wtpt` (type `XYZ `) : trois s15Fixed16 après 8 octets. S'il manque, c'est `SansBlanc`.
- `A2B1` absente : `SansTable`. Un type autre que `mft2` donne `StructureNonPriseEnCharge` avec le type (`mft1`, `mAB `…). `mft2` : 3 entrées et 3 sorties exigées, grille ≥ 2 ; la matrice est ignorée (entrée RVB) ; courbes d'entrée (u16), table (u16, première entrée la plus lente), courbes de sortie (u16).
- Évaluation : courbe d'entrée par interpolation linéaire, **interpolation tétraédrique** dans la table (les six cas et l'ordre des égalités de `tetra()` dans `icc.py`), puis courbe de sortie.
- PCS Lab 16 bits v2 (`w` = valeur × 65535) : L\* = w / 652,80, a\* = w / 256 − 128, b\* = w / 256 − 128. PCS XYZ : X = w / 32768.
- Colorimétrie absolue : XYZ relatif (blanc = illuminant du PCS) × `wtpt` / illuminant, composante par composante, puis Lab rapporté à l'illuminant du PCS (`colorimetrie::xyz_vers_lab`, échelle Y = 100).
- `desc` : texte ASCII du type `desc`, ou premier enregistrement d'un `mluc` ; sinon `None`. `targ` : octets du type `text` après 8 octets, sans les NUL de fin.

- [ ] **Step 1 : Écrire les tests qui échouent**

`crates/gamut/tests/icc.rs`, avec un profil d'essai linéaire `L = 25 + 75·(r+g+b)/3`, `a = 60·(r−g)`, `b = 60·(g−b)` :
- `un_profil_d_essai_se_relit` : `version() == "2.4"` (ou celle choisie par `profil_essai`, notée dans le test), `grille() == 9`, `description()` rendue.
- `aux_noeuds_la_table_est_rendue_exactement` : pour chaque nœud de la grille 9, `lab_absolu` = `cible` à 0,01 près (blanc = D50).
- `entre_les_noeuds_une_fonction_lineaire_est_reproduite` : en (0,3 ; 0,55 ; 0,8), écart ≤ 0,01 (une fonction linéaire est exacte en tétraédrique).
- `le_blanc_absolu_est_le_blanc_du_papier` : profil dont `cible(1,1,1)` = (100, 0, 0) et `blanc` = (95, −0,5, 2,5) ; `lab_absolu([1.,1.,1.])` = (95, −0,5, 2,5) à 0,02 près, et `blanc_support()` aussi.
- `une_table_mab_est_refusee_en_la_nommant` : on remplace le type `mft2` par `mAB ` dans les octets ; `Err(StructureNonPriseEnCharge { structure: "mAB" })` et la clé `comparer.refus.structure`.
- `un_profil_sans_blanc_du_support_est_refuse` (balise `wtpt` renommée) → `SansBlanc`.
- `un_profil_cmjn_est_refuse` (octets 16–19 = `CMYK`) → `PasRvb { espace: "CMYK" }`.
- `une_balise_qui_sort_du_fichier_est_refusee` (fichier coupé au milieu de A2B1) → `Tronque { balise: "A2B1" }`.
- `les_mesures_embarquees_sont_rendues_telles_quelles` : `targ` = « CTI3… » se relit à l'octet près.

- [ ] **Step 2 : Lancer les tests**

Run: `cargo test -p gamut --test icc`
Expected: FAIL (crate absente).

- [ ] **Step 3 : Implémenter `icc.rs`, `essai.rs` et `lib.rs`** selon les règles ci-dessus. `lib.rs` donne la doc de la crate : ce qu'elle calcule, d'où viennent les valeurs (A2B1, absolue), et le renvoi à l'ADR 0004.

- [ ] **Step 4 : Lancer les tests**

Run: `cargo test -p gamut` puis `cargo clippy -p gamut --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5 : Commit** — « Gamut : lecture de la table A2B1 d'un profil, colorimétrie absolue ».

---

### Tâche 2 : `gamut` — surface, volume, chroma par teinte, couleurs de base, chemins

**Files:**
- Create: `crates/gamut/src/surface.rs`, `crates/gamut/src/analyse.rs`
- Test: `crates/gamut/tests/analyse.rs`

**Interfaces:**
- Consumes: `ProfilIcc` (tâche 1), `colorimetrie::{lab_vers_lch, lab_vers_srgb, Lab}`.
- Produces:
  - `pub struct Maillage { pub sommets: Vec<[f64; 3]>, pub triangles: Vec<[u32; 3]> }` (sommets en L\*, a\*, b\*).
  - `pub fn surface_cube(n: usize) -> (Vec<[f64; 3]>, Vec<[u32; 3]>)` : les six faces du cube RVB, `n` points par arête, sommets répétés d'une face à l'autre (6·n² sommets, 12·(n−1)² triangles), triangles orientés vers l'extérieur du cube (`faces()` de `calcul.py`).
  - `pub fn volume(m: &Maillage) -> f64` : valeur absolue de Σ (p0 · (p1 × p2)) / 6, coordonnées prises dans l'ordre (a\*, b\*, L\*).
  - `pub struct Primaire { pub nom: &'static str, pub rvb: [u8; 3], pub lab: [f64; 3], pub c: f64, pub h: Option<f64>, pub ecran: String, pub ramenee: bool }`. Les noms valent `blanc`, `noir`, `rouge`, `vert`, `bleu`, `cyan`, `magenta` et `jaune`, dans cet ordre. `ecran` est le `#rrggbb` de `lab_vers_srgb`, et `ramenee` dit qu'une couleur hors de l'écran a été bornée.
  - `pub struct MaillageAffiche { pub sommets: Vec<[f64; 3]>, pub couleurs: Vec<String>, pub triangles: Vec<[u32; 3]> }`.
  - `pub struct Analyse { pub version: String, pub description: Option<String>, pub grille: u8, pub blanc: [f64; 3], pub noir: [f64; 3], pub dmax_approche: f64, pub volume_surface: f64, pub primaires: Vec<Primaire>, pub chroma_par_teinte: Vec<f64>, pub coupes: Vec<Vec<f64>>, pub affichage: MaillageAffiche }`, sérialisable.
  - `pub fn analyser(p: &ProfilIcc) -> Analyse`.
  - `pub fn chemin(p: &ProfilIcc, depart: [u8; 3], arrivee: [u8; 3]) -> Vec<[f64; 2]>` : 256 points (L\*, C\*), t = i / 255.
  - `pub fn maillage_affiche(m: &Maillage) -> MaillageAffiche` (couleurs écran des sommets), pour la limite du gamut (tâche 5).

Valeurs fixées (prototype) :
- `blanc` et `noir` : `lab_absolu` de (1,1,1) et (0,0,0). `dmax_approche` = −log₁₀(Y du noir absolu, blanc Y = 1). C'est **approché** : une densité tirée de Y, pas une densité ISO status T.
- `volume_surface` : surface du cube à **33** points par arête, passée par `lab_absolu`.
- Table de chroma (**approchée**) : grille RVB 33×33×33 ; case (L, h) avec L = ⌊L\*⌋ borné de 0 à 99 (100 lignes) et h = ⌊teinte / 2°⌋ (180 colonnes). Chaque case garde le C\* maximal. Une case vide entre deux cases non vides de la même ligne prend le plus petit des deux, avec la teinte qui boucle (`gbd()` de `calcul.py`). `coupes[L]` est la ligne L ; `chroma_par_teinte[h]` est le maximum de la colonne h. Les valeurs sont arrondies à 0,1.
- `affichage` : surface du cube à **17** points par arête, soit 1 734 sommets et 3 072 triangles.

- [ ] **Step 1 : Écrire les tests qui échouent** (profil d'essai linéaire de la tâche 1, blanc D50) :
- `la_surface_du_cube_a_ses_sommets_et_triangles` : `surface_cube(17)` donne 1 734 sommets et 3 072 triangles, toutes normales sortantes.
- `le_volume_d_une_transformation_lineaire_est_son_determinant` : le volume de la surface 33 vaut 270 000 à 0,1 % près (|det| de la matrice ((25,25,25), (60,−60,0), (0,60,−60))).
- `les_couleurs_de_base_sont_dans_l_ordre_avec_leur_teinte` : huit primaires dans l'ordre ; `rouge.lab` = (50, 60, 0) à 0,01 près, `h = Some(0.0)` ; `blanc.h == None` (gris, C\* = 0).
- `la_densite_du_noir_vient_de_y` : noir L\* 25 donne `dmax_approche` = −log₁₀(((25 + 16) / 116)³) = 1,36 à 0,01 près.
- `la_coupe_et_la_saturation_par_teinte_ont_leurs_dimensions` : 100 coupes de 180 valeurs, 180 valeurs par teinte, et `chroma_par_teinte[0]` ≥ toutes les valeurs de `coupes[l][0]`.
- `une_case_vide_entre_deux_pleines_est_comblee` : test unitaire de la règle de comblement sur une ligne fabriquée (dans `analyse.rs`, `#[cfg(test)]`).
- `un_chemin_a_256_points_du_depart_a_l_arrivee` : `chemin(bleu → cyan)` a 256 points ; le premier = L\*, C\* de `lab_absolu([0,0,1])`, le dernier celui de `[0,1,1]`.

- [ ] **Step 2 : Lancer les tests** — `cargo test -p gamut --test analyse` → FAIL.
- [ ] **Step 3 : Implémenter `surface.rs` et `analyse.rs`.**
- [ ] **Step 4 : Lancer les tests** — `cargo test -p gamut` → PASS ; clippy propre.
- [ ] **Step 5 : Commit** — « Gamut : surface du cube RVB, volume, chroma par teinte et coupes, couleurs de base, chemins ».

---

### Tâche 3 : `gamut` — écarts entre profils, fidélité, répétabilité, limite iccgamut

**Files:**
- Create: `crates/gamut/src/comparaison.rs`, `crates/gamut/src/mesures.rs`, `crates/gamut/src/limite.rs`
- Test: `crates/gamut/tests/comparaison.rs`, `crates/gamut/tests/reference.rs` (`#[ignore]`)

**Interfaces:**
- Consumes: `ProfilIcc`, `Maillage`, `colorimetrie::delta_e00`.
- Produces:
  - `pub struct Statistiques { pub nombre: usize, pub moyenne: f64, pub mediane: f64, pub p95: f64, pub max: f64, pub part_sup_1: f64, pub part_sup_2: f64 }` (parts en %), et `pub fn statistiques(ecarts: &[f64]) -> Option<Statistiques>` (`None` si vide). Les centiles sont interpolés linéairement au rang p/100·(n−1), comme `numpy.percentile`.
  - `pub enum Famille { Rouge, Jaune, Vert, Cyan, Bleu, Magenta }` et `pub enum Zone { Gris, Ombres, Clairs, Saturee(Famille), Moyenne(Famille) }`, avec `pub fn cle(&self) -> String` (`comparer.zone.gris`, `comparer.zone.saturee.rouge`…).
  - `pub struct EcartZone { pub zone: Zone, pub nombre: usize, pub moyenne: f64, pub p95: f64, pub max: f64 }`, `pub struct PointEcart { pub rvb: [u8; 3], pub reference: [f64; 3], pub compare: [f64; 3], pub ecart: f64 }`, `pub struct Ecarts { pub grille: Statistiques, pub zones: Vec<EcartZone>, pub pires: Vec<PointEcart>, pub primaires: Vec<f64>, pub identiques: bool }`.
  - `pub fn comparer(reference: &ProfilIcc, compare: &ProfilIcc) -> Ecarts`.
  - `pub struct Mesure { pub rvb: [f64; 3], pub lab: [f64; 3] }` (RVB de 0 à 1).
  - `pub fn fidelite(p: &ProfilIcc, mesures: &[Mesure]) -> Option<Statistiques>`.
  - `pub struct Repetabilite { pub couleurs: usize, pub paires: usize, pub moyenne: f64, pub max: f64 }` et `pub fn repetabilite(mesures: &[Mesure]) -> Option<Repetabilite>`.
  - `pub fn memes_mesures(a: &[Mesure], b: &[Mesure]) -> bool`.
  - `pub fn points_du_chemin(mesures: &[Mesure], depart: [u8; 3], arrivee: [u8; 3]) -> Vec<[f64; 3]>` : (position 0–255, L\*, C\*) des plages à 0,5/255 au plus du segment.
  - `pub fn lire_gam(texte: &str) -> Result<Maillage, String>` : première table `VERTEX_NO LAB_L LAB_A LAB_B`, seconde `VERTEX_0 VERTEX_1 VERTEX_2`. Un indice hors des sommets est une erreur.

Règles fixées (prototype, `calcul.py`) :
- Écarts : ΔE00 entre `reference` et `compare` sur la grille RVB **33×33×33** (35 937 valeurs), puis sur les huit couleurs de base. `pires` : les 10 plus grands, du plus fort au plus faible. `identiques` : écart maximal < 0,005.
- Zones, sur les RVB de 0 à 1 de la grille, avec mx = max, mn = min et ch = mx − mn. Elles peuvent se recouvrir, et une zone vide n'est pas listée. **Gris** : ch ≤ 0,1. **Ombres** : mx ≤ 0,35. **Clairs** : mn ≥ 0,75. La famille vient de la teinte TSV : hh en degrés, famille = ⌊((hh + 30) mod 360) / 60⌋ dans l'ordre rouge, jaune, vert, cyan, bleu, magenta. Un gris parfait (ch = 0) n'a pas de famille, sans calcul qui donne NaN. **Saturée** : ch > 0,6. **Moyenne** : 0,1 < ch ≤ 0,6 et mx > 0,35. Les zones sont triées par écart moyen décroissant.
- Fidélité : ΔE00 entre `lab_absolu(rvb)` et le Lab mesuré, plage par plage.
- Répétabilité : plages aux RVB strictement égaux ; ΔE00 de toutes leurs paires. `None` sans plage répétée.
- Mêmes mesures : voir « Choix de l'assistant ».

- [ ] **Step 1 : Écrire les tests qui échouent** (`tests/comparaison.rs`, profils d'essai) :
- `deux_profils_identiques_donnent_des_ecarts_nuls` : `comparer(p, p)` → moyenne, p95 et max à 0, `identiques`, aucune valeur NaN, zones présentes à 0.
- `un_decalage_de_clarte_se_retrouve_partout` : profil B = A avec L\* − 1 (L\* + 1 sortirait de l'échelle du PCS au blanc). Pour un pur ΔL\* = 1, ΔE00 = 1 / S_L, qui va de 0,57 (L\* 100) à 1 (L\* 50) : `max` ≤ 1,0, `moyenne` ≥ 0,55 et `part_sup_1 == 0`.
- `les_zones_suivent_les_seuils_du_prototype` : pour (1, 0, 0) → Saturee(Rouge) ; (0,5, 0,5, 0,5) → Gris ; (0,2, 0,1, 0,1) → Ombres et Gris ; (0,9, 0,8, 0,8) → Clairs et Gris ; (0,8, 0,5, 0,3) → Moyenne(Rouge) (fonction privée testée dans `comparaison.rs`).
- `la_mediane_et_le_p95_suivent_numpy` : `statistiques(&[0.,1.,2.,3.,4.])` → médiane 2, p95 3,8.
- `la_fidelite_d_un_profil_a_ses_propres_noeuds_est_nulle`, et `la_repetabilite_compte_les_paires` (trois plages blanches identiques et deux noires → 2 couleurs, 4 paires).
- `les_memes_mesures_dans_un_autre_ordre_sont_reconnues`, et `un_ecart_de_0_2_n_est_plus_la_meme_mesure`.
- `un_fichier_gam_se_lit_et_un_indice_faux_est_refuse` : texte `.gam` fabriqué (4 sommets, 4 triangles d'un tétraèdre) → volume connu ; un indice 9 → erreur.

- [ ] **Step 2 : Écrire le test de référence** (`tests/reference.rs`, `#[ignore]`, variable `PROFILS_COMPARAISON` = dossier local contenant `mt.icc` et `libre.icc` de l'essai du 26 octobre, jamais versionnés). Valeurs du prototype :
  - `libre.icc` : `volume_surface` 467 693 ± 2 ; rouge (55,35 ; 71,20 ; 53,12) ± 0,01 ; blanc (97,91 ; −0,33 ; 2,18) ± 0,01 ; `dmax_approche` 1,59 ± 0,01.
  - `mt.icc` : `volume_surface` 468 785 ± 2 ; rouge (55,38 ; 71,48 ; 53,28) ± 0,01.
  - `comparer(mt, libre)` : grille moyenne 0,31, p95 0,65, max 5,70 (± 0,01), médiane 0,26, `part_sup_1` 1,6 %, `part_sup_2` 0,3 % (± 0,1).
  - `lire_gam(libre.gam)` : 722 sommets, 1 440 triangles, volume 467 316 ± 1 % (méthode différente d'iccgamut).

Run: `PROFILS_COMPARAISON=<dossier> cargo test -p gamut --test reference -- --ignored` (fait par le mainteneur ou avec son accord : les profils sont à lui).

- [ ] **Step 3 : Implémenter** `comparaison.rs`, `mesures.rs` et `limite.rs`.
- [ ] **Step 4 : Lancer les tests** — `cargo test -p gamut` → PASS ; le test de référence passe avec les profils de l'atelier.
- [ ] **Step 5 : Commit** — « Gamut : écarts entre profils par zone, fidélité, répétabilité, lecture de la limite iccgamut ».

---

### Tâche 4 : `profilage` — limite du gamut par iccgamut

**Files:**
- Create: `crates/profilage/src/limite.rs`
- Modify: `crates/profilage/src/lib.rs`, `crates/profilage/src/moteur.rs` (`Etape::Gamut`), `crates/profilage/src/argyll.rs` (rendre `pub(crate)` `ArgyllCms::outil`, `commande`, `DossierTravail` et la méthode qui lance un outil), `outils/preparer_installateur.py` (`bin/iccgamut.exe` dans `ARGYLL_FICHIERS`), `outils/test_preparer_installateur.py`
- Test: `crates/profilage/tests/limite.rs`

**Interfaces:**
- Consumes: `ArgyllCms`, `ErreurMoteur`, `controler_version`, `DossierTravail` (plan 3).
- Produces:
  - `pub struct LimiteCalculee { pub gam: String, pub volume: Option<f64>, pub version: String }`.
  - `pub trait MoteurGamut: Send + Sync { fn commande_limite(&self) -> String; fn limite(&self, icc: &[u8]) -> Result<LimiteCalculee, ErreurMoteur>; }`.
  - `impl MoteurGamut for ArgyllCms` : version contrôlée (3.5.0) ; dossier de travail ; `profil.icc` écrit ; `iccgamut -v -ia -pl profil.icc` ; `profil.gam` lu ; volume lu dans la sortie ; dossier effacé, réussite ou échec.
  - `pub fn arguments_iccgamut() -> Vec<OsString>` = `["-v", "-ia", "-pl", "profil.icc"]`, et `pub fn lire_volume_iccgamut(sortie: &str) -> Option<f64>` (ligne `Total volume of gamut is <x> cubic colorspace units`).
  - `pub struct MoteurGamutSimule` avec `reussit(gam: String)` et `echoue(ErreurMoteur)`.
  - `Etape::Gamut`, dont la cause s'affiche sous la clé `comparer.erreur.limite`.

Le plan 4 (mires) ajoute aussi une variante à `Etape` (`Plages`) et des méthodes à `MoteurProfil`. Le trait `MoteurGamut` est séparé exprès : le plan exécuté en second n'a qu'à ajouter sa variante à `Etape`.

- [ ] **Step 1 : Écrire les tests qui échouent** :
- `le_volume_se_lit_dans_la_sortie_d_iccgamut` : la ligne réelle ci-dessus → `Some(467316.128099)` ; une sortie sans cette ligne → `None`.
- `les_arguments_d_iccgamut_sont_fixes`.
- `le_moteur_simule_rend_sa_limite` et `le_moteur_simule_echoue_comme_demande`.
- `une_autre_version_refuse_la_limite` : un faux dossier ArgyllCMS dont `colprof` donne 3.4.0 → `VersionNonValidee`. Suivre le modèle du test de version du plan 3 ; s'il passe par `controler_version` seul, le tester ainsi.
- `limite_reelle_d_un_profil_d_essai` (`#[ignore]`, variable `ARGYLL` comme `tests/argyll.rs`) : profil d'essai linéaire de `gamut` (dev-dependency) → `gam` lisible par `gamut::lire_gam`, volume de 270 000 ± 2 %.

- [ ] **Step 2 : Lancer** — `cargo test -p profilage --test limite` → FAIL.
- [ ] **Step 3 : Implémenter** `limite.rs`, puis l'installateur : ajouter `bin/iccgamut.exe` à `ARGYLL_FICHIERS` et son test.
- [ ] **Step 4 : Lancer** — `cargo test -p profilage`, `python outils/test_preparer_installateur.py`, puis le test réel avec `ARGYLL` → PASS. Noter dans `docs/references/icc-argyll-options.md` la commande et la ligne de volume, « confirmées le 26 octobre 2026 avec ArgyllCMS 3.5.0 ».
- [ ] **Step 5 : Commit** — « Profilage : limite du gamut par iccgamut derrière le trait MoteurGamut ».

---

### Tâche 5 : Application — sources, mesures embarquées, analyse et chemins

**Files:**
- Create: `app/src/comparer.rs`, `app/src/comparer/embarquees.rs`
- Modify: `app/Cargo.toml` (dépendance `gamut`), `app/src/lib.rs` (module, état `FichiersOuverts`, commandes), `app/src/textes.rs` (clés `comparer.*`, FR et EN)
- Test: `app/tests/comparer.rs`

**Interfaces:**
- Consumes: `gamut::*` (tâches 1 à 3), `profilage::{MoteurGamut, ArgyllCms}`, l'état `profiler::Moteur` (même `ArgyllCms`), `cgats::{decoder, lire}`, `colorimetrie::spectre_vers_lab`, `Bibliotheque::{fichier_profil, historique_profils, parc}`.
- Produces:
  - `#[serde(tag = "type")] pub enum SourceComparaison { Bibliotheque { profil: i64 }, Fichier { jeton: u32 } }`.
  - `pub struct FichiersOuverts(Mutex<Vec<FichierOuvert>>)` avec `pub fn ouvrir(&self, chemin: &Path) -> Result<FichierOuvertResume, String>`. `FichierOuvert { jeton, nom, icc }` reste en mémoire, jamais écrit. `FichierOuvertResume { jeton: u32, nom: String }`.
  - `pub struct ProfilComparable { pub profil: IdProfil, pub imprimante: String, pub papier: String, pub resolution: String, pub nom: String, pub horodatage: String, pub actif: bool }`.
  - `pub fn mesures_embarquees(targ: &[u8]) -> Result<MesuresProfil, &'static str>` (dans `embarquees.rs`), avec `MesuresProfil { mesures: Vec<gamut::Mesure>, condition: ConditionMesure, condition_supposee: bool, rvb_supposes: bool, lab_calcule: bool }`. Les erreurs sont des clés (`comparer.fidelite.sans_mesures`, `comparer.fidelite.rvb_inconnus`, `comparer.fidelite.lab_inconnu`…). La condition est la seule présente, ou M1 si le fichier en donne plusieurs.
  - `pub enum EtatLimite { Calculee { maillage: gamut::MaillageAffiche, volume: Option<f64> }, Indisponible { cle: &'static str, cause: Option<String> } }`.
  - `pub enum EtatFidelite { Connue { statistiques: gamut::Statistiques, plages: usize, condition: ConditionMesure, condition_supposee: bool, rvb_supposes: bool, lab_calcule: bool }, Inconnue { cle: &'static str } }`.
  - `pub enum EtatProfil { Analyse { analyse: gamut::Analyse, limite: EtatLimite, fidelite: EtatFidelite, repetabilite: Option<gamut::Repetabilite> }, Refuse { cle: &'static str, detail: Option<String> } }`.
  - `pub struct ProfilCompare { pub source: SourceComparaison, pub nom: String, pub origine: String, pub etat: EtatProfil }`.
  - `pub struct Comparaison { pub profils: Vec<ProfilCompare>, pub ecarts: Vec<Option<gamut::Ecarts>>, pub memes_mesures: Vec<bool>, pub commande_limite: Option<String> }` : `ecarts[i]` et `memes_mesures[i]` comparent le profil i au profil 0 (`None` et `false` pour 0, et pour un profil refusé).
  - `pub struct CheminProfil { pub points: Vec<[f64; 2]>, pub mesures: Vec<[f64; 3]> }`.
  - Méthodes de `BibliothequeOuverte` : `profils_comparables(&self) -> Result<Vec<ProfilComparable>, String>`, `comparer(&self, sources: &[SourceComparaison], fichiers: &FichiersOuverts, moteur: Option<&dyn MoteurGamut>) -> Result<Comparaison, String>` et `chemin_comparaison(&self, sources: &[SourceComparaison], fichiers: &FichiersOuverts, depart: [u8; 3], arrivee: [u8; 3]) -> Result<Vec<CheminProfil>, String>`. Les octets se lisent sous le verrou, et tout le calcul se fait après. Au-delà de quatre sources : `Err("comparer.refus.trop_de_profils")`.
  - Commandes `#[tauri::command(async)]` : `comparer_profils_comparables`, `comparer_ouvrir_fichier(filtre: String) -> Result<Option<FichierOuvertResume>, String>` (fenêtre de Windows, `.icc` et `.icm`, comme `parc_ranger_profil`), `comparer_analyser(sources)` et `comparer_chemin(sources, depart, arrivee)`.

- [ ] **Step 1 : Écrire les tests qui échouent** (`app/tests/comparer.rs`, bibliothèque temporaire, profils d'essai de `gamut`, `MoteurGamutSimule`) :
- `les_profils_comparables_listent_actifs_et_historique` : deux résolutions, trois profils ; chemin imprimante, papier, résolution et `actif` justes.
- `un_profil_de_la_bibliotheque_et_un_fichier_se_comparent` : `ecarts[1]` présent, `memes_mesures[1] == false`.
- `un_fichier_ouvert_n_est_jamais_range` : après `ouvrir` et `comparer`, `profils_comparables()` est inchangé.
- `un_profil_refuse_n_empeche_pas_les_autres` : trois sources, dont un fichier `mAB` ; `Refuse { cle: "comparer.refus.structure" }`, les deux autres `Analyse` ; `ecarts[2]` calculé.
- `sans_mesures_embarquees_la_fidelite_est_inconnue` : profil d'essai sans `targ` → `Inconnue { cle: "comparer.fidelite.sans_mesures" }`, `repetabilite == None`.
- `des_mesures_spectrales_seules_donnent_un_lab_calcule` : `targ` CGATS d'en-tête `ORIGINATOR "MYIROtools"` avec RVB 0–255 et spectres seuls → `Connue`, `lab_calcule`, `rvb_supposes`, condition M1 confirmée.
- `les_memes_mesures_sont_reconnues` : deux profils d'essai (grilles différentes), même `targ` → `memes_mesures[1]`.
- `sans_argyll_la_limite_est_indisponible_et_le_reste_est_calcule` : `moteur = None` → `Indisponible { cle: "comparer.limite.sans_argyll" }`, `analyse` complète.
- `un_echec_d_iccgamut_est_dit_avec_sa_cause` (`MoteurGamutSimule::echoue`).
- `plus_de_quatre_profils_est_refuse`.
- `le_chemin_rend_les_points_et_les_mesures_du_chemin`.
- Les clés de tous les `cle()` (`gamut::ErreurIcc`, zones, états) existent dans le catalogue, en français et en anglais. C'est le modèle du test des clés du plan 3.

- [ ] **Step 2 : Lancer** — `cargo test -p app --test comparer` → FAIL.
- [ ] **Step 3 : Implémenter** `comparer.rs`, `embarquees.rs` et les textes. Origine affichée : « Bibliothèque : Imprimante › Papier › Résolution, profil du JJ/MM/AAAA » ou « Fichier : nom.icc (non rangé) ».
- [ ] **Step 4 : Lancer** — `cargo test -p app`, clippy → PASS.
- [ ] **Step 5 : Commit** — « Application : comparer des profils de la bibliothèque et des fichiers, mesures embarquées, limite du gamut ».

---

### Tâche 6 : Page — couleurs de série, vue 3D et graphiques

**Files:**
- Modify: `design-system/tokens.css` (jetons de série), `design-system/README.md` (une ligne)
- Create: `app/interface/vue3d.js`, `app/interface/graphiques.js`
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes: `Comparaison`, `CheminProfil` (tâche 5), en JSON.
- Produces (JavaScript, objets globaux comme les autres scripts de la page) :
  - `Vue3d.creer(canvas, options) -> { afficher(formes), regler(options), detruire() }`. `formes` est une liste de `{ sommets, triangles, couleurs?, serie }`, et `options` vaut `{ couleur: "serie"|"reelle", rendu: "opaque"|"transparent" }`.
  - `Graphiques.teintes(svg, series)`, `Graphiques.coupe(svg, series, l)`, `Graphiques.chemin(svgL, svgC, series)`. Chaque `series` est une liste de `{ valeurs, serie, nom }`.

Jetons (valeurs fixées, contraste ≥ 4,3:1 face à `--carte`) :

| Jeton | Sombre | Clair |
|---|---|---|
| `--serie-1` (bleu) | `#4c9aff` | `#1f6fe5` |
| `--serie-2` (orange) | `#f0773a` | `#c4501a` |
| `--serie-3` (violet) | `#b48cff` | `#7c4dd6` |
| `--serie-4` (gris) | `#c9d1dd` | `#5d6879` |

Vue 3D :
- Projection orthographique de (a\*, b\*, L\*) centré sur L\* 50. Le glisser à la souris fait tourner la vue, la molette zoome. **Au clavier** (canvas `tabindex="0"`) : flèches pour tourner, `+` et `−` pour zoomer, `0` pour revenir à la vue de départ.
- À chaque image, les triangles de toutes les formes sont **triés par profondeur** et peints du plus loin au plus proche. Ombrage plat : teinte × (0,7 + 0,3·|n·lumière|). En transparent, l'opacité vaut 0,35 ; le tri garde les faces dans le bon ordre.
- Axes L\*, a\*, b\* gradués tous les 20, en couleur `--attenue`. Au survol, une bulle donne le sommet le plus proche (L\*, a\*, b\*). Les jetons de série sont lus par `getComputedStyle` et relus au changement de thème.
- Le canvas porte `role="img"` et un `aria-label` qui résume les profils affichés et leurs volumes.

Graphiques (SVG) : axes, unités et légende ; teinte de 0 à 360° en abscisse ; coupe en plan a\*b\* de −100 à +100 ; chemin en abscisse de 0 à 255. Chaque graphique a un `aria-label`, et ses valeurs se lisent dans un tableau replié « Valeurs », une ligne tous les 30°.

- [ ] **Step 1 : Écrire les tests de page qui échouent** (`app/tests/page.rs`) :
- `les_couleurs_de_serie_sont_des_jetons_des_deux_themes` : `--serie-1` à `--serie-4` sont définis dans les deux blocs de thème, aux valeurs du tableau.
- `aucun_script_de_la_page_ne_vient_d_ailleurs` : aucun `http` ni `//` en `src` dans `index.html`, et aucun `import(` distant dans `app/interface/*.js`.
- `la_csp_ne_change_pas` : la chaîne `csp` de `tauri.conf.json` vaut exactement celle d'aujourd'hui.
- `la_vue_3d_se_pilote_au_clavier` : `vue3d.js` traite `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`, `+`, `-` et `0`.

- [ ] **Step 2 : Lancer** — `cargo test -p app --test page` → FAIL.
- [ ] **Step 3 : Implémenter** les jetons, `vue3d.js` et `graphiques.js` (chacun sous 400 lignes).
- [ ] **Step 4 : Lancer** — `cargo test -p app --test page` → PASS.
- [ ] **Step 5 : Commit** — « Interface : couleurs de série, vue 3D triée par profondeur, graphiques de teinte, de coupe et de chemin ».

---

### Tâche 7 : Page — vue « Comparer »

**Files:**
- Create: `app/interface/comparer.js`
- Modify: `app/interface/index.html` (section de la vue, scripts `vue3d.js`, `graphiques.js` et `comparer.js` après `app.js` et avant `reglage.js`), `app/interface/reglage.js` (entrées), `app/src/textes.rs`
- Test: `app/tests/page.rs`

Contenu de la vue, dans la zone centrale, à la place de la vue d'une résolution, la tâche « Bibliothèque » restant active :
1. **Profils comparés** : une puce par profil avec sa couleur de série, son nom et un bouton « Retirer ». Le **bouton principal** est « Ajouter un profil ». Il ouvre une liste des profils de la bibliothèque (imprimante › papier › résolution, date, « actuel ») et une action « Ouvrir un fichier .icc ». À quatre profils, il est inactif et dit pourquoi. Un profil refusé garde sa puce, avec la raison écrite à côté.
2. **Chiffres clés** (une carte par profil) : blanc papier (L\*a\*b\*), noir (L\* et densité « ≈ »), volume (surface et limite, en unités L\*a\*b\* au cube), fidélité (moyenne, p95, max, nombre de plages ; « inconnue » avec la raison), répétabilité.
3. **Gamut en volume** : profils affichés (cases à cocher), couleur (par profil / couleurs réelles), forme (**Limite du gamut** par défaut / Surface brute), rendu (**Opaque** par défaut / Transparent). Sans ArgyllCMS, « Limite du gamut » est inactif, avec sa raison à côté.
4. **Saturation par teinte** et **Coupe à clarté constante** (curseur L\* de 5 à 95, valeur affichée).
5. **Courbes le long d'un chemin du cube RVB** : départ et arrivée parmi les huit couleurs de base, bleu → cyan par défaut (la face B = 255 du prototype). L\* et C\* de chaque profil, avec les mesures embarquées du chemin en points.
6. **Couleurs de base** : tableau des huit couleurs, avec L\*a\*b\*, C\*, h (« — » pour un gris) et la pastille. Une pastille ramenée est marquée « approchée ».
7. **Écarts au profil de référence** (à partir de deux profils) : moyenne, médiane, p95, max, parts au-delà de 1 et de 2 ; zones triées ; les 10 points les plus forts ; écart de chaque couleur de base. « Profils identiques » quand `identiques`. Quand `memes_mesures` : « Mêmes mesures : seul le logiciel de calcul change », et les deux fidélités côte à côte.
8. **Méthode** (section repliable qui résume quand elle est fermée). Elle sépare **calculé** (valeurs lues dans les tables A2B1 en colorimétrie absolue, volume de la surface du cube à 33 points par arête, limite et volume par iccgamut, écarts ΔE00 sur 35 937 valeurs RVB) et **approché** (saturation par teinte et coupes, par tranches de 1 L\* et de 2° ; densité du noir tirée de Y ; pastilles d'écran). Elle dit aussi la limite de la fidélité, en ces mots : « La fidélité se mesure sur les plages qui ont servi au calcul : elle avantage le profil qui les suit au plus près. Pour savoir lequel prédit le mieux un nouveau tirage, il faut mesurer une mire de contrôle (vérification, prévue). »
9. **Zone de droite** : provenance de chaque profil (origine, version ICC, description, grille A2B1, mesures embarquées : nombre, condition, « supposée » le cas échéant), puis, sous « Avancé », la commande `iccgamut` en lecture seule.

Entrées : dans la vue d'une résolution, un bouton secondaire « Comparer » ouvre la vue avec le profil actuel. Dans l'historique, « Comparer à l'actuel » ouvre l'avant / après, avec l'actuel en référence.

- [ ] **Step 1 : Écrire les tests de page qui échouent** :
- `comparer_js_est_charge_apres_ses_outils` : ordre `app.js` < `vue3d.js`, `graphiques.js` < `comparer.js` < `reglage.js`.
- `la_vue_comparer_a_un_seul_bouton_principal`, sur le modèle de `la_vue_d_une_resolution_a_un_seul_bouton_principal`.
- `la_vue_d_une_resolution_ouvre_la_comparaison` (`reglage.js` appelle l'ouverture de la vue).
- `les_textes_de_la_vue_comparer_existent_dans_les_deux_langues` : toutes les clés `comparer.` citées par `comparer.js` et `index.html` existent en FR et en EN.
- `chaque_bouton_principal_inactif_dit_pourquoi` couvre la nouvelle vue : vérifier que le test existant la voit.

- [ ] **Step 2 : Lancer** → FAIL.
- [ ] **Step 3 : Implémenter** `comparer.js`. Si le fichier dépasse 400 lignes, en sortir le tableau des écarts dans `comparer-ecarts.js`.
- [ ] **Step 4 : Lancer** — `cargo test -p app` → PASS ; `cargo run -p app` : contrôle visuel rapide en clair et en sombre, en FR et en EN.
- [ ] **Step 5 : Commit** — « Interface : vue Comparer, chiffres clés, volume, teintes, coupe, chemins, couleurs de base, écarts ».

---

### Tâche 8 : Documentation, essai à l'écran, PR

**Files:**
- Modify: `docs/adr/0004-crates-partagees-rip.md` (section « Contrat de la crate `gamut` (date) »), `GLOSSARY.md`, `docs/blog/<jour>.md`, `CLAUDE.md` (crate `gamut` dans la liste des crates et dans `CRATES` de la CI), `docs/references/icc-argyll-options.md` (fait en tâche 4)

- [ ] **Step 1 : Contrat de `gamut` dans l'ADR 0004** : entrée (profil RVB, A2B1 `mft2`), colorimétrie absolue, grilles 33 et 17, règles des zones et des centiles, ce qui est approché, et refus nommés. Changer une de ces règles change les chiffres, c'est donc une rupture de contrat.
- [ ] **Step 2 : Glossaire** : gamut, surface brute, limite du gamut, coupe à clarté constante, fidélité d'un profil, répétabilité, profil de référence (d'une comparaison).
- [ ] **Step 3 : Journal du jour** : ce qui est livré, les choix de l'assistant et ce qui reste pour #42.
- [ ] **Step 4 : Contrôles** — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/verifier_docs.py` → tout vert.
- [ ] **Step 5 : Essai à l'écran avec le mainteneur** (étape humaine) : comparer le profil calculé par myiro-libre et celui du logiciel du fabricant sur la même mesure HFA Mat 200g. Contrôler que la vue reconnaît « mêmes mesures », que les chiffres concordent avec le prototype (volumes, rouge, écarts) et que la vue 3D opaque n'a pas de fausses facettes dans les bleus sombres. Puis comparer deux imprimantes sur un même papier. Noter les remarques ; les corrections vont dans cette PR.
- [ ] **Step 6 : PR vers `dev`** avec les choix de l'assistant et le Review Focus. Commenter #71 (livré) et #42 (partie couverte, reste à faire).
