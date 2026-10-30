# Comparer une image et un profil — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dans la vue « Comparer », l'imprimeur ouvre une photo de client (JPEG, TIFF ou PNG, en RVB). Pour chaque profil affiché, il voit quels pixels l'imprimante ne pourra pas reproduire : surimpression sur l'image avec un seuil réglable, part de l'image hors gamut, classes d'écart, teintes touchées, et nuage de couleurs de l'image dans la vue 3D et dans la coupe.

**Architecture:** Le calcul vit dans la crate `gamut` de #71, qui reste sans dépendance externe. Elle gagne trois briques :
- la table inverse B2A1 d'un profil de sortie et l'aller-retour colorimétrique relatif (`ProfilIcc::reproduire`) ;
- le profil de l'image (`ProfilSource` : matrice et courbes, gris, ou table A2B0 ; sRVB par défaut) ;
- l'analyse des pixels (`analyser_pixels`).

Une nouvelle crate `photo` décode les fichiers par la crate `image`. Elle lit le profil ICC embarqué et l'orientation EXIF, refuse le CMJN et ramène l'image à une **image d'analyse** d'au plus 2 048 pixels de côté. L'application ajoute `app/src/comparer/image.rs` : une image ouverte pour la séance, jamais rangée, analysée hors du verrou et annulable. La page ajoute `app/interface/comparer-image.js` (image, surimpression, chiffres) et un calque de points à `vue3d.js` et `Graphiques.coupe`.

**Tech Stack:** Rust 2021 ; crate `image` 0.25 (`default-features = false`, `jpeg`, `png`, `tiff`), seule dépendance externe nouvelle. Crates `colorimetrie`, `gamut`, `serde`. Tauri 2, avec réponse binaire (`tauri::ipc::Response`) pour les pixels. HTML, CSS et JavaScript simples, Canvas 2D (ADR 0003).

**Spec:** ticket #80 et ses choix validés par le mainteneur le 28 octobre 2026. Plan 5 (`docs/superpowers/plans/2026-10-26-comparer-des-profils.md`), dont ce plan est la suite. Seuil hors gamut du plan 7 (`docs/superpowers/plans/2026-10-28-verification-profil.md`, « Choix de l'assistant ») : même méthode, aller-retour B2A1 puis A2B1 en colorimétrique relatif, ΔE00 entre la couleur demandée et la couleur reproduite.

**Dépend de :** #71 fusionné dans `dev` (crate `gamut`, `app/src/comparer.rs`, `FichiersOuverts`, `vue3d.js`, `graphiques.js`, `comparer.js`). Les noms cités ci-dessous sont ceux de la branche `ticket/71-comparer-profils` au commit `3dcbb34`. Les corrections d'après l'essai d'écran ont pu les renommer : relire avant chaque tâche.

## Feuille de route

Ce plan est une nouvelle fonction demandée pendant l'essai de #71. Il passe après #71 et ne retarde pas la première livraison (plans 4, 6 et 7). **Le CMJN (PDF, TIFF CMJN) est une seconde étape**, hors de ce plan, qui sera un ticket à part.

## Global Constraints

- Travail dans un worktree `ticket/80-image-profil` issu de `dev` après la fusion de #71 (`.claude/worktrees/ticket-80`), PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/80-image-profil --reference dev`.
- La crate `gamut` ne dépend toujours que de `colorimetrie` et de `serde`. La crate `photo` dépend de `image` et de `serde`, et de rien d'autre. Les deux restent indépendantes de Tauri, de Windows, des ponts et d'ArgyllCMS, et `photo` rejoint la liste `CRATES` du job linux de la CI.
- Une seule dépendance externe nouvelle : `image = { version = "0.25", default-features = false, features = ["jpeg", "png", "tiff"] }`. Vérifier par `cargo tree -p photo -e normal` que chaque crate tirée est sous MIT, Apache-2.0, BSD, Zlib ou équivalent, donc compatible GPL-3.0. Vérifier aussi qu'elle compile en `i686-pc-windows-msvc`.
- **L'image n'est jamais modifiée, copiée ni écrite sur le disque**, pas même dans un dossier de travail. Elle ne vit qu'en mémoire, le temps de la séance, comme un `.icc` ouvert pour la comparaison (#71). Aucun appel à ArgyllCMS ne reçoit ses pixels.
- Rendu **colorimétrique relatif** : couleur demandée = Lab relatif D50 de la source ; couleur reproduite = A2B1(B2A1(demandée)) du profil de sortie. Écart = ΔE00 (`colorimetrie::delta_e00`) entre les deux, la même règle que le seuil hors gamut du plan 7.
- **Seuil d'alerte par défaut : 2,0 ΔE00** (ticket #80). Il se règle de 0,5 à 10 par pas de 0,5 sans relancer le calcul.
- **Profil de l'image** : celui qu'elle embarque. Sans profil, sRVB. Un profil embarqué illisible n'est **jamais** remplacé en silence : il est refusé avec sa raison, et l'opérateur peut choisir « Traiter comme sRVB », un choix que la page affiche alors comme « supposé ».
- Ce qui est **approché** est dit : l'image d'analyse prise sur un pixel sur N, et les valeurs de 16 bits ramenées à 8 bits. Ce qui est **inconnu** reste inconnu, avec sa raison, jamais zéro : un profil sans table B2A1, ou un pixel transparent.
- Interface : aucune couleur en dur hors de `design-system/tokens.css`. La couleur d'alerte est un nouveau jeton `--couleur-hors-gamut`, lisible dans les deux thèmes. Aucun texte en dur : tout passe par `app/src/textes.rs`, en français et en anglais. CSP inchangée (`img-src 'self'`) : les pixels arrivent en binaire et sont dessinés par `putImageData`, jamais par une URL `data:`.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57). Fichiers source nouveaux sous 400 lignes. `lib.rs` de l'application ne reçoit que l'enregistrement des commandes et de l'état.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p gamut -p photo -p app`, `python outils/verifier_docs.py`. Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- **Décodage par la crate `image`, pas par ArgyllCMS.** Cela vaut aussi pour les décodeurs séparés. `tiffgamut` et `cctiff` ne lisent ni le PNG ni les profils de tous les JPEG. Ils obligeraient à écrire la photo du client dans un dossier de travail, ce que la règle « jamais sur le disque » interdit. Ils ne donnent pas non plus d'écart par pixel. Les décodeurs séparés (`zune-jpeg`, `png`, `tiff`) sont ceux que `image` emploie déjà. La passer par `image` donne en une seule interface :
  - les types de pixels (gris, palette, alpha, 16 bits) ;
  - `ImageDecoder::icc_profile()` et `ImageDecoder::orientation()` ;
  - des limites de mémoire (`Limits`).
  Toutes ces crates sont en Rust pur, donc sans souci en 32 bits, et sous licence MIT/Apache. `tiff` existe déjà dans l'espace de travail (crate `mires`, 0.11) : une seconde version tirée par `image` est acceptée, et notée dans la PR.
- **Aller-retour en Rust, pas par `xicclu`.** Le plan 7 interroge le profil par `xicclu`, en processus externe, pour une centaine de plages. Une photo donne des centaines de milliers de couleurs. Les passer par un programme externe obligerait à écrire leurs valeurs sur le disque, et serait lent pour quatre profils. La crate `gamut` lit déjà les tables `mft2` et interpole A2B1 en tétraédrique. Elle lit B2A1 de la même façon, mais l'interpole en trilinéaire, comme ArgyllCMS. Sur les deux profils de l'atelier, l'écart à `xicclu` mesuré est de 0,0001 ΔE00 en trilinéaire, contre 0,58 en tétraédrique. La concordance avec `xicclu` est tenue par un test `#[ignore]` (tâche 1, écart ≤ 0,1 ΔE00).
- **Image d'analyse d'au plus 2 048 pixels de côté, en sautant des pixels sans les mélanger.** Une moyenne de pixels voisins inventerait des couleurs moins saturées que la photo. Toutes les mesures partent de cette image d'analyse : le pourcentage, les classes, les teintes, le nuage et la surimpression. Une photo de 24 Mpx (6 000 × 4 000) donne une image d'analyse de 2 048 × 1 365, un pixel sur trois dans chaque sens. La page affiche « calculé sur 1 pixel sur 9 ».
- **Calcul par couleur distincte.** Les pixels de l'image d'analyse sont regroupés par valeur RVB 8 bits. L'aller-retour se fait une fois par couleur distincte, puis l'écart est reporté sur chaque pixel. Une photo courante a de 100 000 à 500 000 couleurs distinctes.
- **Teintes touchées** : teinte h de la couleur demandée (Lab relatif D50) rangée en six familles ancrées sur les couleurs sRVB pures (rouge, jaune, vert, cyan, bleu, magenta), plus « gris » (C\* < 5). L'ancrage de chaque famille est la teinte de sa couleur sRVB pure, calculée par `ProfilSource::srvb` : 40,9°, 99,6°, 134,4°, 196,5°, 301,4° et 327,1°. Les bornes sont à mi-chemin des ancrages voisins, et le magenta rejoint le rouge en passant par 0°. `gamut` expose l'ordre des familles (`FAMILLES_TEINTE`) et leurs bornes (`familles_teinte`), pour que la page les nomme sans recalcul. **Écart au plan, décidé par le général le 28 octobre 2026** : le plan prévoyait douze secteurs de 30°, ceux du tableau « Chroma par teinte » de #71. Mais un imprimeur doit lire « bleus » pour le bleu d'une photo, et le bleu sRVB tombe vers 301°, à cheval entre deux secteurs de 30° qui ne portent pas de nom. Les zones « Rouges saturés »… de #71 ne changent pas : elles classent des valeurs RVB d'appareil, pas les couleurs d'une photo.
- **Nuage de couleurs** : au plus 4 000 points, rangés par cellules de 4 unités Lab, les plus peuplées d'abord. Chaque point est placé en colorimétrie absolue sur le blanc du papier du profil choisi : en colorimétrique relatif, le blanc de l'image devient le blanc du papier.
- **Pixels transparents** (alpha = 0) : ni comptés ni affichés en alerte. Un alpha partiel compte comme opaque.

## Review Focus

1. **Photo en CMJN** (JPEG CMJN d'une agence, TIFF CMJN) : refus clair « image CMJN : prévue dans une seconde étape », sans calcul faux sur trois canaux sur quatre. → test `une_image_cmjn_est_refusee` (tâche 4).
2. **Photo de téléphone en Display P3** (profil matrice et courbes `para`, type 3) : lue comme telle, et non comme sRVB. → test `un_profil_display_p3_a_courbes_para_est_lu` (tâche 2).
3. **Photo en portrait avec orientation EXIF** : affichée droite, et la surimpression tombe sur les bons pixels. → test `l_orientation_exif_est_appliquee` (tâche 4).
4. **Profil de sortie sans table B2A1, ou avec un B2A1 `mAB`** (ICC v4) : ce profil est « inconnu : pas de table inverse lisible », et les autres profils sont calculés quand même. → test `un_profil_sans_b2a1_est_inconnu_sans_bloquer_les_autres` (tâche 5).
5. **PNG avec transparence** : les pixels transparents ne pèsent pas dans le pourcentage et ne sont pas marqués. → test `les_pixels_transparents_ne_comptent_pas` (tâche 3).

---

### Tâche 0 : Préparer le chantier

- [ ] **Step 1 : Contexte** — #71 fusionné dans `dev` (`crates/gamut` présent), worktree `.claude/worktrees/ticket-80` sur `ticket/80-image-profil`. Lancer `verifier_contexte.py`. Réserver #80 (`gh issue edit 80 --add-assignee @me`).
- [ ] **Step 2 : ArgyllCMS local** — copier `C:\Code\MYIRO-fork\argyll\` dans le worktree (ignoré par git), pour les seuls tests `#[ignore]` (concordance `xicclu`, sRVB d'ArgyllCMS).
- [ ] **Step 3 : Relire les noms** de `gamut` (`ProfilIcc`, `Lut`, `lire_mft2`, `profil_essai`), de `app/src/comparer.rs` et de `app/src/comparer/sources.rs` (`FichiersOuverts`, `SourceComparaison`, `lire_sources`), et de la page (`Vue3d.creer`, `Graphiques.coupe`). Noter dans la PR les noms qui diffèrent de ce plan.

---

### Tâche 1 : `gamut` — table inverse B2A1 et aller-retour relatif

**Files:**
- Modify: `crates/gamut/src/table.rs` (`lire_mft2(d, balise)` prend le nom de la balise pour ses refus ; entrée Lab ou XYZ), `crates/gamut/src/icc.rs`, `crates/gamut/src/essai.rs` (`ProfilEssai::inverse`), `crates/gamut/src/lib.rs`
- Test: `crates/gamut/tests/inverse.rs`

**Interfaces:**
- Produces:
  ```rust
  impl ProfilIcc {
      /// L*a*b* relatifs (D50 du PCS) de `rvb` par A2B1, sans le blanc du support.
      pub fn lab_relatif(&self, rvb: [f64; 3]) -> [f64; 3];
      /// Lab relatif → Lab absolu (même règle que `lab_absolu` : × wtpt / D50 en XYZ).
      pub fn relatif_vers_absolu(&self, lab: [f64; 3]) -> [f64; 3];
      /// Aller-retour : A2B1(B2A1(lab)), en Lab relatif. Err si B2A1 absent ou non lu.
      pub fn reproduire(&self, lab: [f64; 3]) -> Result<[f64; 3], ErreurIcc>;
      /// La table inverse a-t-elle été lue (sinon pourquoi).
      pub fn inverse(&self) -> Result<(), ErreurIcc>;
  }
  // ErreurIcc gagne : SansTableInverse  → cle "comparer.refus.sans_table_inverse"
  // ProfilEssai gagne : pub inverse: Option<fn([f64; 3]) -> [f64; 3]> (Lab relatif → RVB 0–1)
  ```
- `ProfilIcc::lire` ne refuse **pas** un profil sans B2A1 : la comparaison de #71 reste possible, et seule `reproduire` échoue. Le résultat de la lecture de B2A1 est gardé dans le profil (`Result<Lut, ErreurIcc>`).
- Encodage de l'entrée de B2A1 `mft2` : Lab « hérité » 16 bits, c'est-à-dire L\* × 652,80 / 65 535 et (a\* + 128) × 256 / 65 535 ; XYZ × 32 768 / 65 535. C'est l'inverse exact du décodage déjà fait par `lab_absolu`.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn l_aller_retour_d_une_couleur_du_gamut_rend_la_meme_couleur() {}
// profil d'essai dont A2B1 et B2A1 sont exactement inverses (cible linéaire) :
// delta_e00(lab, reproduire(lab)) < 0,05 pour 50 couleurs tirées dans le gamut
#[test] fn une_couleur_hors_gamut_revient_sur_la_limite() {}
// B2A1 d'essai borne le chroma à 40 : lab = [50, 70, 0] → reproduite ≈ [50, 40, 0], écart > 5 (8,6 mesuré : ΔE00 resserre les écarts de chroma)
#[test] fn un_profil_sans_b2a1_se_lit_mais_ne_reproduit_pas() {}
// lire() Ok ; reproduire() == Err(ErreurIcc::SansTableInverse) ; inverse().is_err()
#[test] fn un_b2a1_mab_est_dit_non_pris_en_charge() {}   // Err(StructureNonPriseEnCharge { structure: "mAB" }), sans espace, comme le refus d'A2B1
#[test] fn lab_relatif_et_absolu_valent_les_valeurs_calculees_a_la_main() {} // papier L* = 90 : valeurs attendues calculées à la main, PCS Lab et XYZ
#[test] fn au_pcs_xyz_l_aller_retour_passe_par_la_matrice_puis_les_courbes() {} // profil_essai_xyz : matrice diagonale non identité, puis courbes
#[test] #[ignore] fn l_aller_retour_concorde_avec_xicclu() {}
// profil calculé par ArgyllCMS (argyll/ du worktree, MoteurProfil du plan 3 sur la mesure d'essai) :
// 100 couleurs Lab, xicclu -fb -ir puis -ff -ir ; écart ≤ 0,1 ΔE00 avec reproduire()
```

- [ ] **Step 2 : Lancer** — `cargo test -p gamut --test inverse` → FAIL (`reproduire` n'existe pas).
- [ ] **Step 3 : Implémenter** dans `icc.rs` (lecture de B2A1, encodage d'entrée, `reproduire`, `lab_relatif`, `relatif_vers_absolu`) et dans `table.rs` (paramètre `balise`). Les nouvelles fonctions de `icc.rs` vont dans un module `inverse.rs` si `icc.rs` dépasse 400 lignes.
- [ ] **Step 4 : Lancer** — `cargo test -p gamut` → PASS, anciens tests compris. Puis `cargo test -p gamut --test inverse -- --ignored --nocapture` → PASS, en notant l'écart maximal trouvé.
- [ ] **Step 5 : Commit** — `Gamut : table inverse B2A1 et aller-retour colorimétrique relatif`

---

### Tâche 2 : `gamut` — le profil de l'image (`ProfilSource`)

**Files:**
- Create: `crates/gamut/src/source.rs`
- Modify: `crates/gamut/src/lib.rs`, `crates/gamut/src/icc.rs` (lecture des balises partagée : `Balises` en `pub(crate)`)
- Test: `crates/gamut/tests/source.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum ProfilSource { /* matrice et courbes RVB, gris (kTRC), table A2B0 mft2 RVB */ }
  impl ProfilSource {
      /// sRVB (IEC 61966-2-1), primaires adaptées D50 telles que les écrit un profil ICC sRVB :
      /// rXYZ 0,4361 0,2225 0,0139 · gXYZ 0,3851 0,7169 0,0971 · bXYZ 0,1431 0,0606 0,7141 ;
      /// courbe para type 3 : γ 2,4, a 1/1,055, b 0,055/1,055, c 1/12,92, d 0,04045.
      pub fn srvb() -> ProfilSource;
      /// Profil embarqué : RVB matrice et courbes (rXYZ…, rTRC… en curv ou para types 0 à 4),
      /// gris (kTRC), ou RVB à table A2B0 mft2. Autre chose : Err avec sa structure.
      pub fn lire(octets: &[u8]) -> Result<ProfilSource, ErreurIcc>;
      /// Nom lu dans `desc`, ou « sRVB » pour srvb().
      pub fn nom(&self) -> String;
      /// Couleur demandée : Lab relatif D50 de `rvb` (0–1 ; un gris passe r = v = b).
      pub fn demande(&self, rvb: [f64; 3]) -> [f64; 3];
  }
  ```
- `curv` : 0 point, identité ; 1 point, gamma u8Fixed8 ; n points, table interpolée. `para` : types 0 à 4 de l'ICC (g ; g a b ; g a b c ; g a b c d ; g a b c d e f).
- Un profil embarqué CMJN est refusé par `ErreurIcc::PasRvb` (clé existante). Un profil Lab ou d'un autre espace de données donne `EnTete`.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn le_blanc_srvb_est_le_blanc_d50() {}          // demande([1,1,1]) ≈ [100, 0, 0] à 0,01
#[test] fn srvb_concorde_avec_colorimetrie() {}         // pour 1 000 rvb : lab_vers_srgb(demande(rvb)) ≈ rvb à 0,5/255
#[test] fn un_profil_display_p3_a_courbes_para_est_lu() {}
// profil construit en test (matrice P3 adaptée D50, para type 3) : demande([1,0,0]) a un chroma
// supérieur de plus de 10 à celui de srvb().demande([1,0,0])
#[test] fn un_profil_gris_ktrc_est_lu() {}               // demande([0.5;3]) a a* et b* ≈ 0
#[test] fn un_profil_a2b0_mft2_est_lu() {}               // profil_essai : demande(x) ≈ lab_relatif(x) du même profil
#[test] fn un_profil_cmjn_embarque_est_refuse() {}      // Err(ErreurIcc::PasRvb { espace: "CMYK" })
#[test] #[ignore] fn srvb_concorde_avec_le_srgb_d_argyll() {} // ProfilSource::lire(argyll/ref/sRGB.icm) ≈ srvb() à 0,05 ΔE00
```

- [ ] **Step 2 : Lancer** — `cargo test -p gamut --test source` → FAIL.
- [ ] **Step 3 : Implémenter** `source.rs`, en moins de 400 lignes. Matrice : XYZ = M · (courbe(r), courbe(v), courbe(b)), puis `xyz_vers_lab` sur D50. Les profils v2 et v4 matrice/TRC donnent déjà du relatif D50.
- [ ] **Step 4 : Lancer** — `cargo test -p gamut` → PASS ; `-- --ignored` pour le test sRVB d'ArgyllCMS → PASS.
- [ ] **Step 5 : Commit** — `Gamut : profil de l'image, matrice et courbes, gris, A2B0, sRVB par défaut`

---

### Tâche 3 : `gamut` — analyser les pixels d'une image

**Files:**
- Create: `crates/gamut/src/image.rs`
- Modify: `crates/gamut/src/lib.rs`
- Test: `crates/gamut/tests/image.rs`

**Interfaces:**
- Consumes: `ProfilSource::demande` (tâche 2), `ProfilIcc::reproduire` et `relatif_vers_absolu` (tâche 1), `colorimetrie::{delta_e00, lab_vers_lch, lab_vers_srgb}`.
- Produces:
  ```rust
  /// Écart d'un pixel, en dixièmes de ΔE00, borné à 254 ; 255 = transparent ou inconnu.
  pub const ECART_INCONNU: u8 = 255;
  pub const PAS_HISTOGRAMME: f64 = 0.1;   // 255 cases de 0,1 ΔE00 ; la dernière reçoit tout ≥ 25,4
  pub const SECTEURS_TEINTE: usize = 7;    // 6 familles ancrées sur les couleurs sRVB pures, puis « gris » (C* < 5)
  pub const POINTS_NUAGE: usize = 4_000;
  #[derive(Serialize)] pub struct PointNuage { pub lab: [f64; 3] /* absolu */, pub pixels: u32, pub ecart_max: f64, pub couleur: String /* #rrggbb, lab_vers_srgb */ }
  #[derive(Serialize)] pub struct AnalyseImage {
      pub pixels_comptes: u64,                 // hors transparents
      pub couleurs_distinctes: u32,
      pub histogramme: Vec<u64>,               // 255 cases, en pixels
      pub par_teinte: Vec<Vec<u64>>,           // SECTEURS_TEINTE × 255
      pub nuage: Vec<PointNuage>,
      #[serde(skip)] pub ecarts: Vec<u8>,      // un octet par pixel, ordre des lignes
  }
  pub struct Annulee;
  pub fn analyser_pixels(
      pixels: &[[u8; 4]],                      // RVBA de l'image d'analyse
      source: &ProfilSource,
      sortie: &ProfilIcc,
      annuler: &dyn Fn() -> bool,              // regardé toutes les 65 536 couleurs distinctes
  ) -> Result<Result<AnalyseImage, ErreurIcc>, Annulee>;
  ```
- Déroulé :
  1. Regrouper les pixels opaques par RVB : `HashMap<u32, u32>`.
  2. Pour chaque couleur distincte : demandée = `source.demande`, reproduite = `sortie.reproduire`, écart = `delta_e00` ; un écart non calculable vaut `ECART_INCONNU`.
  3. Remplir `ecarts`, `histogramme` et `par_teinte` (teinte de la demandée), pondérés par le nombre de pixels.
  4. Nuage : cellules de 4 unités Lab sur la demandée, les `POINTS_NUAGE` cellules les plus peuplées ; position = moyenne de la cellule, passée par `relatif_vers_absolu`.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn une_image_dans_le_gamut_n_a_aucun_pixel_au_dela_du_seuil() {}
// profil d'essai à gamut large, image de gris moyens : histogramme au-delà de 2,0 (case 20 et plus) = 0
#[test] fn une_image_saturee_sur_un_gamut_etroit_est_comptee_hors_gamut() {}
// B2A1 qui borne C* à 40, pixels rouges sRVB purs : 100 % au-delà de 10 ΔE00 ; famille « rouge » en tête
#[test] fn les_pixels_transparents_ne_comptent_pas() {}
// 10 pixels dont 4 à alpha 0 : pixels_comptes == 6 ; leurs ecarts == ECART_INCONNU
#[test] fn deux_pixels_de_meme_couleur_ne_sont_calcules_qu_une_fois() {} // couleurs_distinctes == 1 pour 1 000 pixels identiques
#[test] fn le_nuage_ne_depasse_pas_4000_points_et_garde_les_plus_peuples() {}
#[test] fn sans_b2a1_l_analyse_est_refusee_avec_sa_raison() {}           // Ok(Err(ErreurIcc::SansTableInverse))
#[test] fn l_analyse_s_arrete_quand_on_annule() {}                       // annuler renvoie vrai → Err(Annulee)
#[test] fn une_image_d_un_million_de_couleurs_tient_en_dix_secondes() {} // #[ignore] (chronomètre, lancé en release)
```

- [ ] **Step 2 : Lancer** — `cargo test -p gamut --test image` → FAIL.
- [ ] **Step 3 : Implémenter** `image.rs` (moins de 400 lignes).
- [ ] **Step 4 : Lancer** — `cargo test -p gamut` → PASS. Puis `cargo test -p gamut --release --test image -- --ignored` → PASS, en notant la durée.
- [ ] **Step 5 : Commit** — `Gamut : écarts d'une image à un profil, par couleur distincte, histogramme, teintes et nuage`

---

### Tâche 4 : Crate `photo` — ouvrir une photo et en tirer l'image d'analyse

**Files:**
- Create: `crates/photo/Cargo.toml`, `crates/photo/src/lib.rs`
- Modify: `Cargo.toml` (membre `crates/photo`), `.github/workflows/ci.yml` (`photo` dans `CRATES`)
- Test: `crates/photo/tests/photo.rs` (fichiers d'essai construits par le test lui-même ; aucune photo dans le dépôt)

**Interfaces:**
- Produces:
  ```rust
  pub const COTE_MAX_ANALYSE: u32 = 2_048;
  pub const PIXELS_MAX_ORIGINE: u64 = 120_000_000;   // au-delà : refus « image trop grande »
  pub const OCTETS_MAX_DECODAGE: u64 = 1 << 30;      // Limits::max_alloc
  #[derive(Debug, PartialEq)] pub enum RefusPhoto { Illisible(String), FormatNonPrisEnCharge, Cmjn, TropGrande { pixels: u64 } }
  impl RefusPhoto { pub fn cle(&self) -> &'static str; } // "image.refus.illisible" | ".format" | ".cmjn" | ".trop_grande"
  pub struct Photo {
      pub largeur_origine: u32, pub hauteur_origine: u32,
      pub largeur: u32, pub hauteur: u32,            // image d'analyse, orientation appliquée
      pub pas: u32,                                   // 1 pixel sur `pas` dans chaque sens
      pub seize_bits: bool,                           // valeurs ramenées à 8 bits (approché)
      pub gris: bool,
      pub icc: Option<Vec<u8>>,                       // profil embarqué, tel quel
      pub rvba: Vec<[u8; 4]>,                         // largeur × hauteur, ligne par ligne
  }
  pub fn ouvrir(octets: &[u8]) -> Result<Photo, RefusPhoto>;
  ```
- Le format est reconnu par son contenu (`ImageReader::with_guessed_format`), pas par l'extension. Seuls JPEG, PNG et TIFF sont acceptés : un autre format est refusé par `FormatNonPrisEnCharge`.
- Le CMJN est reconnu au type de pixels du décodeur et à l'en-tête du profil embarqué (espace `CMYK`). Il est refusé avant tout calcul.
- Le profil embarqué est lu par `icc_profile()` (JPEG : segments APP2 réassemblés). Si `image` ne le rend pas pour le TIFF (balise 34675, **supposé** : à confirmer par le test), le lire par la crate `tiff` déjà tirée par `image`.
- `pas` = plus petit entier tel que les deux côtés divisés par `pas` restent ≤ `COTE_MAX_ANALYSE`. On prend un pixel tous les `pas`, sans moyenne.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn un_jpeg_rvb_avec_profil_embarque_rend_son_profil() {}     // segment APP2 « ICC_PROFILE » ajouté à la main
#[test] fn un_png_avec_iccp_rend_son_profil() {}
#[test] fn un_tiff_avec_balise_34675_rend_son_profil() {}            // écrit par la crate tiff
#[test] fn une_image_sans_profil_rend_none() {}
#[test] fn une_image_cmjn_est_refusee() {}                            // JPEG CMJN (encodé en test) → RefusPhoto::Cmjn
#[test] fn l_orientation_exif_est_appliquee() {}                      // JPEG 4 × 2 avec Orientation = 6 → largeur 2, hauteur 4, pixel (0,0) attendu
#[test] fn une_grande_image_est_echantillonnee_sans_melange() {}      // 5 000 × 100 → pas 3, chaque pixel rendu existe dans l'original
#[test] fn un_tiff_16_bits_est_ramene_a_8_bits() {}                   // seize_bits == true, 65535 → 255
#[test] fn un_fichier_qui_n_est_pas_une_image_est_illisible() {}
#[test] fn un_format_gif_est_refuse() {}                              // FormatNonPrisEnCharge
```

- [ ] **Step 2 : Lancer** — `cargo test -p photo` → FAIL.
- [ ] **Step 3 : Dépendance** — `cargo add -p photo image@0.25 --no-default-features -F jpeg,png,tiff`, puis `cargo add -p photo serde -F derive`. Lancer `cargo tree -p photo -e normal` et relever les licences dans la PR. Puis `cargo build -p photo --target i686-pc-windows-msvc` → OK.
- [ ] **Step 4 : Implémenter** `lib.rs` (moins de 400 lignes), sans `unwrap` : chaque erreur du décodeur devient `Illisible(cause)`.
- [ ] **Step 5 : Lancer** — `cargo test -p photo` puis `cargo test -p photo --target i686-pc-windows-msvc` → PASS.
- [ ] **Step 6 : Commit** — `Photo : ouvrir JPEG, PNG et TIFF RVB, profil embarqué, orientation, image d'analyse`

---

### Tâche 5 : Application — ouvrir une image et l'analyser, hors du verrou et annulable

**Files:**
- Create: `app/src/comparer/image.rs`
- Modify: `app/src/comparer.rs` (`pub mod image;`), `app/src/lib.rs` (commandes et état), `app/Cargo.toml` (`photo`), `app/src/textes.rs`
- Test: `app/tests/comparer_image.rs`

**Interfaces:**
- Consumes: `photo::ouvrir`, `ProfilSource`, `analyser_pixels`, `AnalyseImage` ; `BibliothequeOuverte::lire_sources`, `FichiersOuverts`, `SourceComparaison` (#71).
- Produces:
  ```rust
  /// L'image ouverte pour la séance (une à la fois), en mémoire seulement.
  #[derive(Default)] pub struct ImageOuverte(Mutex<Option<ImageEnMemoire>>);
  #[derive(Default)] pub struct AnalyseEnCours(AtomicBool);           // drapeau d'annulation
  #[derive(Serialize)] pub struct ImageResume {
      pub nom: String, pub largeur: u32, pub hauteur: u32, pub largeur_origine: u32, pub hauteur_origine: u32,
      pub pas: u32, pub seize_bits: bool,
      pub profil: EtatProfilImage,       // Embarque { nom } | Srvb { suppose: bool } | Refuse { cle, detail }
  }
  #[derive(Serialize)] pub struct AnalyseProfil {
      pub source: SourceComparaison,
      pub etat: EtatAnalyse,              // Calculee { analyse: AnalyseImage } | Inconnue { cle, detail }
  }
  impl ImageOuverte {
      pub fn ouvrir(&self, chemin: &Path) -> Result<ImageResume, String>;   // lit le fichier, photo::ouvrir, garde en mémoire
      pub fn traiter_comme_srvb(&self) -> Result<ImageResume, String>;      // profil refusé → Srvb { suppose: true }
      pub fn fermer(&self);
      pub fn apercu(&self) -> Result<Vec<u8>, String>;                     // RVBA de l'image d'analyse
  }
  impl BibliothequeOuverte {
      pub fn analyser_image(&self, image: &ImageOuverte, sources: &[SourceComparaison],
          fichiers: &FichiersOuverts, annuler: &AnalyseEnCours) -> Result<Vec<AnalyseProfil>, String>;
  }
  // Commandes : image_ouvrir (fenêtre de Windows, filtres jpg jpeg png tif tiff), image_traiter_comme_srvb,
  // image_fermer, image_apercu -> tauri::ipc::Response, image_analyser -> Vec<AnalyseProfil>,
  // image_ecarts(index) -> tauri::ipc::Response (octets d'écart du profil `index` de la dernière analyse),
  // image_annuler.
  ```
- `analyser_image` copie sous les verrous les octets des profils (`lire_sources`) et un `Arc` des pixels, puis relâche les verrous. Le calcul se fait ensuite pour chaque profil, en parallèle par `std::thread::scope` (quatre au plus). Les octets d'écart de la dernière analyse sont gardés avec l'image, pour `image_ecarts`.
- Un profil refusé par `gamut` (lecture, ou `SansTableInverse`) donne `Inconnue` avec sa clé, sans empêcher les autres.
- Ouvrir une nouvelle image remplace la précédente, et « Fermer la comparaison » appelle `image_fermer`.
- Taille lue au plus : 1 Gio. Au-delà, refus `image.refus.trop_grande`, sans rien lire de plus.

- [ ] **Step 1 : Tests en échec (sans Tauri ; bibliothèque temporaire, profils d'essai rangés, PNG construit en test)**

```rust
#[test] fn une_image_ouverte_n_est_ni_rangee_ni_ecrite() {}
// la bibliothèque et son dossier ont les mêmes fichiers avant et après ouvrir() + analyser_image()
#[test] fn chaque_profil_affiche_a_son_analyse() {}                 // 2 sources → 2 AnalyseProfil dans le même ordre
#[test] fn un_profil_sans_b2a1_est_inconnu_sans_bloquer_les_autres() {}
// profil_essai sans inverse + profil avec : [Inconnue { cle: "comparer.refus.sans_table_inverse" }, Calculee]
#[test] fn un_profil_embarque_illisible_n_est_jamais_remplace_en_silence() {}
// profil embarqué tronqué → profil: Refuse ; analyser_image → Err("image.refus.profil") ; après traiter_comme_srvb → Srvb { suppose: true }
#[test] fn annuler_arrete_l_analyse_et_le_dit() {}                  // drapeau posé → Err("image.annulee")
#[test] fn l_apercu_a_largeur_fois_hauteur_fois_4_octets() {}
#[test] fn sans_image_ouverte_l_analyse_est_refusee() {}             // Err("image.refus.aucune")
```

- [ ] **Step 2 : Lancer** — `cargo test -p app --test comparer_image` → FAIL.
- [ ] **Step 3 : Implémenter** `image.rs` (moins de 400 lignes) et enregistrer les commandes et les états dans `lib.rs`.
- [ ] **Step 4 : Textes FR et EN** (`textes.rs`), chaque refus disant la cause puis l'action :
  - `image.ouvrir` « Ajouter une image » ;
  - `image.refus.cmjn` « Cette image est en CMJN. La comparaison des images CMJN viendra dans une seconde étape. Ouvrez une version RVB de la photo. » ;
  - `image.refus.profil` « Le profil intégré à l'image ne se lit pas ({detail}). Choisissez « Traiter comme sRVB » si la photo vient d'un appareil ou d'un site en sRVB. » ;
  - `image.refus.trop_grande`, `image.refus.illisible`, `image.refus.format`, `image.refus.aucune`, `image.annulee` ;
  - `image.profil.embarque` « Profil de l'image : {nom} », `image.profil.srvb` « Profil de l'image : sRVB (aucun profil intégré) », `image.profil.suppose` « sRVB, supposé : choisi par vous » ;
  - `image.approche` « Calculé sur 1 pixel sur {n} » ;
  - `image.seize_bits` « Valeurs 16 bits ramenées à 8 bits ».
- [ ] **Step 5 : Lancer** — `cargo test -p app` → PASS.
- [ ] **Step 6 : Commit** — `Comparer : ouvrir une image pour la séance, l'analyser pour chaque profil, hors du verrou et annulable`

---

### Tâche 6 : Page — l'image, la surimpression et les chiffres

**Files:**
- Create: `app/interface/comparer-image.js`
- Modify: `app/interface/comparer.js` (bouton « Ajouter une image », section, fermeture), `app/interface/index.html` (script), `design-system/tokens.css` (`--couleur-hors-gamut`, `--couleur-hors-gamut-fond`, dans les deux thèmes), `app/src/textes.rs`, `app/tests/page.rs` (si la page y est contrôlée : nouveau script référencé, aucune couleur en dur)

**Interfaces:**
- Consumes: commandes de la tâche 5 ; `Graphiques.nombre`.
- Produces: `window.ComparerImage = { creer(conteneur, { resume, analyses, profils }) → { regler({ seuil, profil }), detruire() } }`.

- La section « Image » se place sous les chiffres clés, et contient dans l'ordre :
  - le nom de l'image et l'état de son profil (embarqué, sRVB ou supposé), puis les mentions « approché » et « 16 bits » s'il y a lieu ;
  - le choix du profil pour la surimpression (les profils affichés, la référence d'abord) ;
  - le curseur « Seuil d'alerte », de 0,5 à 10, pas 0,5, valeur 2,0 ;
  - la bascule « Afficher : image / image et alerte / alerte seule » ;
  - un `canvas` à l'échelle de la colonne : l'image par `putImageData`, puis un calque où chaque pixel d'écart > seuil prend `--couleur-hors-gamut`.
- Le changement de seuil ne relance aucune commande. Il recalcule le calque à partir des octets d'écart, et les chiffres à partir de `histogramme` et `par_teinte`.
- Les chiffres forment un tableau, une colonne par profil (comme les chiffres clés validés au point 4 de l'essai de #71) :
  - part de l'image au-delà du seuil (%, sur `pixels_comptes`) ;
  - classes léger, moyen et fort (bornes : « Décisions à valider ») ;
  - les trois teintes les plus touchées (nom du secteur et %) ;
  - « inconnu » et sa raison pour un profil `Inconnue`.
- Pendant le calcul, « Analyse en cours… » et un bouton « Annuler » (`image_annuler`). Les autres vues restent utilisables.
- Clavier : curseur et sélecteurs accessibles. Le `canvas` porte un texte de remplacement (« {p} % de l'image au-delà de {s} ΔE00 avec {profil} »).

- [ ] **Step 1 : Test en échec (`app/tests/page.rs`, s'il contrôle la page)** — `comparer_image_js_est_charge_et_sans_couleur_en_dur`.
- [ ] **Step 2 : Implémenter** `comparer-image.js` (moins de 400 lignes), les jetons et les textes.
- [ ] **Step 3 : Lancer** — `cargo test -p app` → PASS ; `cargo run -p app` avec un JPEG sRVB de test : l'image s'affiche, et le curseur fait bouger le calque et les chiffres sans délai visible.
- [ ] **Step 4 : Commit** — `Comparer : section Image, surimpression hors gamut, seuil réglable, chiffres par profil`

---

### Tâche 7 : Le nuage de couleurs de l'image dans la vue 3D et la coupe

**Files:**
- Modify: `app/interface/vue3d.js` (calque de points), `app/interface/graphiques.js` (`coupe(svg, series, l, points)`), `app/interface/comparer.js` (bascule « Couleurs de l'image »), `app/src/textes.rs`

**Interfaces:**
- Produces:
  - `Vue3d.creer(...)` renvoie aussi `points(liste)`, où `liste` = `[{ lab: [L, a, b], couleur: "#rrggbb", alerte: bool }]`. Les points sont dessinés après les triangles, triés par profondeur, en petits carrés de 3 px × `devicePixelRatio`. Un point en alerte prend `--couleur-hors-gamut`, un autre sa propre couleur.
  - `Graphiques.coupe(svg, series, l, points)` : les points dont |L\* − l| ≤ 2,5, en petits cercles. Le paramètre est facultatif : sans lui, le graphique ne change pas.
- Le nuage affiché est celui du profil choisi pour la surimpression (`PointNuage`), recalculé dans la page quand le seuil change (`alerte = ecart_max > seuil`).

- [ ] **Step 1 : Implémenter** les deux calques et la bascule « Couleurs de l'image » (cochée quand une image est ouverte).
- [ ] **Step 2 : Lancer** — `cargo test -p app` → PASS ; `cargo run -p app` : avec un JPEG très saturé et un profil mat, les points rouges vifs sortent du volume et sont en alerte. Dans la coupe à L\* 50, les points tombent dans ou hors du contour.
- [ ] **Step 3 : Commit** — `Comparer : nuage de couleurs de l'image dans la vue 3D et la coupe`

---

### Tâche 8 : Documentation, essai d'écran et PR

**Files:**
- Modify: `CLAUDE.md` (crate `photo`, module `comparer/image`, liste `CRATES`), `docs/adr/0004-crates-partagees-rip.md` (ajouts au contrat de `gamut` : `reproduire`, `lab_relatif`, `relatif_vers_absolu`, `ProfilSource`, `analyser_pixels`, `ErreurIcc::SansTableInverse`), `docs/adr/0003-interface-tauri.md` (réponse binaire pour les pixels ; la CSP ne change pas), `GLOSSARY.md` (couleur demandée, couleur reproduite, image d'analyse, seuil d'alerte), `docs/blog/<jour>.md`

- [ ] **Step 1 : Documentation** — dont, dans la PR et l'ADR 0004, les points « confirmé » et « supposé » (TIFF 34675, concordance `xicclu`).
- [ ] **Step 2 : Contrôles** — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/test_preparer_installateur.py`, `python outils/verifier_docs.py` → tout vert.
- [ ] **Step 3 : Essai d'écran avec le mainteneur** (geste humain, par le général) :
  1. Ouvrir la comparaison de la résolution HFA Mat 200g (notre profil et celui de MYIROtools), puis « Ajouter une image » avec une photo de client en sRVB.
  2. L'état du profil de l'image s'affiche (« aucun profil intégré », ou son nom).
  3. Surimpression : les zones signalées correspondent à ce qu'il attend d'un papier mat (bleus et verts saturés, rouges vifs). Le curseur de seuil répond tout de suite.
  4. Chiffres par profil, classes et teintes touchées : lisibles et utiles pour choisir un papier.
  5. Nuage dans la vue 3D et dans la coupe.
  6. Une photo Adobe RGB, puis une photo de téléphone (Display P3) : le profil intégré est reconnu.
  7. Une image CMJN : refus clair.
  8. Une grande photo (24 Mpx) : « Analyse en cours… », la page reste utilisable, « Annuler » fonctionne, et la mention « 1 pixel sur N » apparaît.
  9. Anglais et thème clair.
- [ ] **Step 4 : PR vers `dev`** — y mettre les choix de l'assistant, le Review Focus, les licences relevées par `cargo tree`, les noms qui ont changé depuis #71 et les résultats de l'essai. Commenter #80, et ouvrir le ticket de la seconde étape (CMJN) s'il n'existe pas.

---

## Décisions du mainteneur (28 octobre 2026)

Réponses données au général le 28 octobre 2026 : « 1 ok, 2 réduite, 3 vert, 4 une, 5 ok ».

1. **Classes d'écart** au-delà du seuil de 2 ΔE00 : léger de 2 à 5, moyen de 5 à 10, fort au-delà de 10.
2. **Calcul sur l'image réduite** à 2 048 px de côté au plus, un pixel sur N sans mélange, avec la mention à l'écran.
3. **Couleur d'alerte vive : vert fluo** (jeton de `design-system/tokens.css`).
4. **Une seule image à la fois**, face à 1 à 4 profils.
5. **Profil intégré illisible** : la page propose « Traiter comme sRVB », jamais de façon silencieuse, puis l'affiche comme supposé.
