# Vérifier un profil ICC sur une seconde mire — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dans la vue d'une résolution, l'imprimeur choisit un profil, un jeu de tolérances et la mesure importée d'une seconde mire imprimée à travers ce profil. Il obtient deux familles de résultats, la fidélité aux couleurs demandées et l'accord avec la prédiction du profil. Le profil passe à l'état « accepté selon <tolérances v N> », « hors tolérance » ou « vérification incomplète », et un rapport en HTML peut être exporté, seul ou avec le profil.

**Architecture:** Une nouvelle crate `verification` (dépend de `colorimetrie` et de `serde` seulement, indépendante de Tauri, de Windows, des ponts et d'ArgyllCMS) fait tout le calcul métier : écarts ΔE00 plage par plage, couleurs hors gamut, statistiques (moyenne, 95e centile, maximum, pires plages), tolérances et conclusion. La crate `profilage` reçoit un trait `MoteurVerification` qui interroge le profil par `xicclu` d'ArgyllCMS : couleur demandée, valeur envoyée à l'imprimante, prédiction. Il a deux adapters, `ArgyllCms` et `MoteurVerificationSimule` (ADR 0005). La crate `bibliotheque` range les jeux de tolérances et les vérifications, immuables, en JSON opaque. L'application ajoute `app/src/verifier.rs` (méthodes de `BibliothequeOuverte` testées sans Tauri, calcul hors du verrou) et `app/src/rapport.rs` (rapport HTML). La page ajoute `app/interface/verification.js` à la vue d'une résolution.

**Tech Stack:** Rust 2021, crates `colorimetrie`, `serde` et `serde_json` (déjà dans l'espace de travail, aucune nouvelle dépendance externe), ArgyllCMS 3.5.0 (`xicclu`, version figée, ADR 0002), Tauri 2, HTML, CSS et JavaScript simples (ADR 0003).

**Spec:** ticket #41 (critères d'acceptation) ; `docs/specifications/poste-travail-couleur.md` (première livraison, règles transversales) ; `docs/specifications/2026-10-20-poste-unique-design.md` (« Verdict en clair, détail à la demande », « Transparence des calculs », « Le spectre d'abord, provenance visible ») ; plan 3 `docs/superpowers/plans/2026-10-23-profil-rvb-import-fd-s2w.md` (crate `profilage`, `preparer_plages`, `Moteur`) ; référence `docs/references/icc-argyll-options.md`.

## Feuille de route (28 octobre 2026)

Ce plan est le **plan 7**, première partie : la vérification à partir d'une **mesure importée** (export CGATS de FD-S2w), comme le plan 3 l'a fait pour le calcul. Il ne dépend ni du plan 4 (mires, #33) ni du plan 5 (comparaison, #71), qui avancent en parallèle.

1. à 3. Bibliothèque, cadre, calcul d'un profil RVB importé (faits).
4. Mires de profilage (#33, plan 4, en cours).
5. Comparer des profils (#71, plan 5, en cours).
6. Mesure d'une feuille au FD-9 (#34, plan 6, en étude).
7. **Vérification du profil sur une seconde mire** (ce plan, #41). Seconde partie, après les plans 4 et 6 : mire de vérification produite par myiro-libre, parcours en 4 étapes, mesure directe au MYIRO-1 et au FD-9. Le noyau de ce plan les sert tels quels : il prend des RVB et des Lab, d'où qu'ils viennent.
8. Séchage et reprise ; 9. reprise de l'existant.

Hors de ce plan : capture d'écran jointe à la fiche, espace source autre que sRVB, rendu perceptif, compensation du point noir, moyenne de plusieurs exemplaires (#35), CMJN.

## Global Constraints

- Travail dans un worktree `ticket/41-verification` issu de `dev` (`.claude/worktrees/ticket-41`), PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/41-verification --reference dev`.
- La crate `verification` ne dépend que de `colorimetrie`, `serde` et `serde_json` ; elle rejoint la liste `CRATES` du job linux de la CI et `app/tests/independance_coeur.rs`. Aucune nouvelle dépendance externe dans tout le plan.
- ArgyllCMS : version figée `3.5.0`, en processus externe, sans fenêtre de console, dossier de travail effacé après usage (`DossierTravail`). Sans `xicclu`, l'outil est dit manquant (`ErreurMoteur::OutilManquant`) et rien n'est rangé.
- « Demande », « valeur après transformation » et « prédiction » ne sont jamais confondues : chaque plage les garde séparément, avec la mesure (#41).
- Aucun seuil universel inventé (#41) : un jeu de tolérances n'existe que si l'imprimeur l'a enregistré. La page propose des valeurs de départ, marquées « proposition », qu'il doit confirmer.
- Jeux de tolérances et vérifications sont **immuables** : modifier un jeu enregistre une nouvelle version ; un recalcul ajoute une vérification et garde l'originale (#41, règles transversales).
- Un profil hors tolérance reste actif, installable et exportable ; rien n'est effacé par un échec (#41).
- Ce qui n'est pas calculable reste « inconnu », avec sa raison, jamais zéro. Un critère requis non calculable donne « vérification incomplète », jamais « accepté ».
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57). Fichiers source nouveaux sous 400 lignes. `lib.rs` de l'application ne reçoit que l'enregistrement des commandes et de l'état.
- Aucun texte en dur dans la page ni dans le rapport : tout passe par `app/src/textes.rs`, en français et en anglais ; un rapport est dans une seule langue. Thèmes clair et sombre. Rouge réservé au hors tolérance ; un verdict est toujours couleur + icône + mot.
- Données de test fictives : jamais de numéro de série, d'adresse réseau ou de nom de client réels. Aucun fichier de `tests/` ne contient « install » dans son nom.
- Ne jamais écrire le nom du logiciel concurrent. Ne pas citer `docs/sources/`.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p verification -p profilage -p bibliotheque -p app`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Recouvrements avec les chantiers en cours

- **Plan 5 (#71)** rend publics dans `profilage` `ArgyllCms::outil`, `ArgyllCms::lancer`, `commande`, `ligne`, `DossierTravail` (et ses méthodes `creer`, `chemin`), et ajoute `ErreurMoteur::OutilManquant { outil: String }` (clé `profil.erreur.outil_manquant`). S'il est fusionné avant la tâche 3, s'en servir ; sinon faire **exactement** les mêmes changements, avec les mêmes noms, pour que la seconde fusion soit sans conflit de sens.
- **Plan 4 (#33)** ajoute l'organisation 6 de la bibliothèque (mires et tirages). La tâche 4 prend la **première organisation libre** au moment où elle s'exécute (6, ou 7 si le plan 4 est déjà fusionné) ; celui qui fusionne en second renumérote et rejoue `cargo test -p bibliotheque`.
- **Plan 4, tâche 8 (#1)** : le contrôle de cohérence des mesures. La tâche 9 de ce plan le branche avant la vérification, seulement quand il est dans `dev`.

## Choix de l'assistant (à signaler dans la PR)

- **Ce que veut dire « imprimée à travers le profil ».** Les valeurs RVB de la mire sont des couleurs **sRVB** (`argyll/ref/sRGB.icm`). Le logiciel d'impression les convertit vers l'imprimante avec le profil, en **colorimétrique relatif, sans compensation du point noir**. C'est le seul cas de cette première partie ; la fiche le dit. Une mire imprimée **sans gestion des couleurs** est aussi acceptée : seule la prédiction se calcule alors.
- **Valeur après transformation : calculée, donc approchée.** On ne sait pas ce que le logiciel d'impression a envoyé à l'imprimante. ArgyllCMS refait la conversion avec le même profil (`xicclu`, table B2A1). La page et le rapport disent « calculée par ArgyllCMS ».
- **Fidélité** : ΔE00 entre la couleur demandée (Lab de sRVB, colorimétrique relatif) et la mesure, ramenée au blanc du support du profil. La mesure est passée en relatif par le rapport composante par composante de XYZ au blanc du support (règle ICC). Le blanc du support est celui que prédit le profil pour le blanc de l'appareil (1, 1, 1), en absolu.
- **Prédiction** : ΔE00 entre ce que le profil prédit pour la valeur après transformation (colorimétrie absolue, table A2B) et la mesure telle quelle. C'est le même sens que `profcheck -k`.
- **Hors gamut** : une couleur demandée est hors gamut quand le profil prédit qu'il ne peut pas l'atteindre : ΔE00 entre la demande et la prédiction relative supérieur au **seuil de gamut** du jeu de tolérances (proposé : 1,0). Ces plages restent listées, avec leurs propres statistiques ; les critères de fidélité portent sur les plages dans le gamut.
- **95e centile** : rang le plus proche (écarts triés en ordre croissant, rang `ceil(0,95 × n)`, compté à partir de 1).
- **Pires plages** : les 10 plus grands écarts de chaque famille, du plus grand au plus petit.
- **Règle d'acceptation** : un critère est un seuil renseigné (moyenne, 95e centile ou maximum d'une famille). Un critère dépassé → hors tolérance ; sinon un critère non calculable → incomplet ; sinon → accepté. Une famille sans aucun seuil n'est pas requise.
- **Recalcul** : avec un autre jeu de tolérances (ou une nouvelle version), à partir des écarts conservés plage par plage, sans ArgyllCMS ni nouvelle mesure. Il ajoute une vérification liée à l'originale.
- **État du profil** : celui de sa vérification la plus récente.
- **Rapport en HTML autonome** (un seul fichier, styles intégrés, sans script ni lien externe), plutôt qu'en PDF. Raisons : aucune dépendance à ajouter, il s'ouvre partout, et Windows l'imprime en PDF (« Microsoft Print to PDF »). Il est régénéré à chaque export depuis les données rangées, jamais stocké. Il est en noir et gris, avec le verdict en mot et en symbole (✓, ✗, ?), pour rester lisible une fois photocopié.
- **Mire distincte** : la mesure qui a servi à calculer le profil est refusée pour le vérifier. Une mire dont les couleurs sont les mêmes que celles du profil est acceptée, mais le rapport le signale (« mêmes couleurs que la mire du profil »).

## Review Focus

1. **La mesure du profil choisie pour le vérifier** (même import, ou le seul import de la résolution) : refusée avant tout calcul, avec la raison à côté de la mesure, inactive dans la liste. → test `la_mesure_du_profil_ne_peut_pas_le_verifier` (tâche 5).
2. **Mire très saturée sur papier mat, presque tout hors gamut** : la fidélité dans le gamut n'a aucune plage ; elle est « inconnue », la vérification est incomplète, jamais acceptée ; les plages hors gamut sont listées. → test `sans_plage_dans_le_gamut_une_fidelite_requise_rend_incomplet` (tâche 2).
3. **Jeu de tolérances modifié après usage** : une nouvelle version est enregistrée, l'ancienne vérification garde l'ancienne version, et son rapport se régénère à l'identique. → tests `modifier_un_jeu_cree_une_version_sans_toucher_l_ancienne` (tâche 4) et `le_rapport_d_une_ancienne_verification_ne_change_pas` (tâche 6).
4. **Vérifier un profil de l'historique** (un profil plus récent est actif) : seul l'état du profil vérifié change, le profil actif reste actif et garde son état. → test `verifier_un_ancien_profil_ne_touche_pas_le_profil_actif` (tâche 5).
5. **ArgyllCMS du plan 3 sans `xicclu`** (dossier `argyll/` copié à moitié) : message « outil manquant : xicclu » avec l'action, rien n'est rangé, le calcul de profil reste possible. → test `sans_xicclu_rien_n_est_range` (tâche 5).

---

### Tâche 0 : Préparer le chantier

**Files:** aucun.

- [ ] **Step 1 : Contexte**

Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev` puis `git status --short`
Expected: contexte conforme. Les changements préexistants ne relèvent pas de ce plan.

- [ ] **Step 2 : Worktree**

Run: `git worktree add .claude/worktrees/ticket-41 -b ticket/41-verification dev`

- [ ] **Step 3 : ArgyllCMS dans le worktree**

Copier `C:\Code\MYIRO-fork\argyll\` dans `.claude/worktrees/ticket-41/argyll/` (ignoré par git). Vérifier `argyll/bin/xicclu.exe` et `argyll/ref/sRGB.icm`.

- [ ] **Step 4 : Annoncer la prise en charge sur #41** (`gh issue comment 41`), avec le lien de ce plan.

---

### Tâche 1 : Crate `verification` — écarts plage par plage et statistiques

**Files:**
- Modify: `crates/colorimetrie/src/lib.rs` (`lab_vers_xyz`), `Cargo.toml` (membre), `.github/workflows/ci.yml` (`-p verification` dans `CRATES`), `app/tests/independance_coeur.rs` (crate attendue sans Tauri, si la liste y est explicite)
- Create: `crates/verification/Cargo.toml`, `crates/verification/src/lib.rs`, `crates/verification/src/ecarts.rs`
- Test: `crates/colorimetrie/tests/` (fichier existant des conversions), `crates/verification/tests/ecarts.rs`

**Interfaces:**
- Consumes: `colorimetrie::{Lab, Xyz, Inconnu, delta_e00, xyz_vers_lab, blanc_d50}`.
- Produces:
  ```rust
  // colorimetrie
  pub fn lab_vers_xyz(lab: Lab, blanc: Xyz) -> Result<Xyz, Inconnu>; // inverse exact de xyz_vers_lab
  // verification
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct PlageAVerifier { pub id: String, pub rvb: [f64; 3] /* 0 à 1 */, pub mesure: Lab /* absolu */ }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct ValeursProfil {
      pub demande: Option<Vec<Lab>>,   // Lab relatif des RVB en sRVB ; None : imprimée sans gestion des couleurs
      pub appareil: Vec<[f64; 3]>,     // valeur après transformation, 0 à 1 (= rvb sans gestion)
      pub prediction_rel: Vec<Lab>,
      pub prediction_abs: Vec<Lab>,
      pub blanc_support: Lab,          // absolu
  }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct ResultatPlage {
      pub id: String, pub rvb: [f64; 3], pub demande: Option<Lab>, pub appareil: [f64; 3],
      pub prediction: Lab, pub mesure: Lab, pub mesure_rel: Lab,
      pub ecart_fidelite: Option<f64>, pub ecart_prediction: f64, pub ecart_gamut: Option<f64>,
  }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct EcartPlage { pub id: String, pub de00: f64 }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Statistiques { pub plages: usize, pub moyenne: f64, pub p95: f64, pub max: f64, pub pires: Vec<EcartPlage> }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Resultats {
      pub plages: Vec<ResultatPlage>,
      pub seuil_gamut: f64,
      pub fidelite: Option<Statistiques>,   // plages dans le gamut ; None sans demande ou sans plage
      pub hors_gamut: Option<Statistiques>, // None sans plage hors gamut
      pub prediction: Option<Statistiques>,
  }
  pub enum RefusVerification { Vide, TaillesDifferentes, ValeurNonFinie { plage: String } }
  impl RefusVerification { pub fn cle(&self) -> &'static str; } // verif.refus.vide, .tailles, .valeur
  pub fn vers_relatif(mesure: Lab, blanc_support: Lab) -> Result<Lab, Inconnu>;
  pub fn comparer(plages: &[PlageAVerifier], valeurs: &ValeursProfil, seuil_gamut: f64) -> Result<Resultats, RefusVerification>;
  pub fn reclasser(resultats: &Resultats, seuil_gamut: f64) -> Resultats; // nouveau seuil, mêmes écarts
  pub fn statistiques(ecarts: &[EcartPlage]) -> Option<Statistiques>;    // None pour une liste vide
  ```
- `vers_relatif` : `lab_vers_xyz(mesure, D50)`, puis chaque composante × `D50 / blanc_support` (blanc en XYZ), puis `xyz_vers_lab(…, D50)`. `ecart_gamut` = ΔE00(demande, prédiction relative) ; plage hors gamut si `ecart_gamut > seuil_gamut`.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn lab_vers_xyz_est_l_inverse_de_xyz_vers_lab() {}        // colorimetrie, 5 couleurs, à 1e-9
#[test] fn le_blanc_du_support_devient_le_blanc_relatif() {}      // vers_relatif(blanc, blanc) == Lab { l: 100, a: 0, b: 0 } à 1e-6
#[test]
fn le_95e_centile_est_au_rang_le_plus_proche() {
    let e: Vec<_> = (1..=20).map(|i| EcartPlage { id: i.to_string(), de00: i as f64 }).collect();
    let s = statistiques(&e).unwrap();
    assert_eq!((s.moyenne, s.p95, s.max, s.plages), (10.5, 19.0, 20.0, 20));
    assert_eq!(s.pires.len(), 10);
    assert_eq!(s.pires[0].id, "20");
}
#[test] fn une_liste_vide_n_a_pas_de_statistiques() {}
#[test] fn un_profil_parfait_donne_des_ecarts_nuls() {}               // demande = prédiction = mesure_rel
#[test] fn une_plage_aberrante_est_la_pire_et_seule_au_dessus() {}   // 1 plage à 8 ΔE00, les autres à 0,5
#[test] fn une_derive_globale_monte_moyenne_et_centile() {}          // toutes les mesures décalées de L − 3
#[test] fn fidelite_et_prediction_peuvent_diverger() {}
    // prédiction bonne (mesure = prédiction_abs) mais demande loin (hors gamut à 4 ΔE00) : prediction.moyenne < 0,01, hors_gamut.plages == n
#[test] fn sans_demande_seule_la_prediction_est_calculee() {}       // fidelite == None, hors_gamut == None
#[test] fn reclasser_avec_un_autre_seuil_garde_les_ecarts() {}       // ecart_* identiques, partage gamut / hors gamut changé
#[test] fn des_tailles_differentes_sont_refusees() {}
#[test] fn une_valeur_non_finie_nomme_la_plage() {}
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p colorimetrie -p verification` → échec de compilation.

- [ ] **Step 3 : Implémenter** `lab_vers_xyz` dans `colorimetrie`, puis `ecarts.rs`. Les écarts passent par `colorimetrie::delta_e00` ; une `Inconnu` devient `ValeurNonFinie { plage }`.

- [ ] **Step 4 : Vérifier** — Run: `cargo test -p colorimetrie -p verification && cargo clippy -p verification --all-targets -- -D warnings` → PASS.

- [ ] **Step 5 : Commit**

```bash
git add Cargo.toml .github/workflows/ci.yml app/tests/independance_coeur.rs crates/colorimetrie crates/verification
git commit -m "Vérification : écarts de fidélité et de prédiction plage par plage, statistiques"
```

---

### Tâche 2 : Tolérances et conclusion

**Files:**
- Create: `crates/verification/src/tolerances.rs`
- Test: `crates/verification/tests/tolerances.rs`

**Interfaces:**
- Consumes: `Resultats`, `Statistiques` (tâche 1).
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
  pub struct Seuils { pub moyenne: Option<f64>, pub p95: Option<f64>, pub max: Option<f64> }
  #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Criteres { pub fidelite: Seuils, pub prediction: Seuils, pub seuil_gamut: f64 }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Famille { Fidelite, Prediction }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Critere { Moyenne, P95, Max }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Controle { pub famille: Famille, pub critere: Critere, pub limite: f64, pub valeur: Option<f64>, pub respecte: Option<bool> }
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Conclusion { Accepte, HorsTolerance, Incomplet }
  pub enum RefusCriteres { AucunSeuil, SeuilInvalide /* ≤ 0 ou non fini */ }
  impl RefusCriteres { pub fn cle(&self) -> &'static str; } // verif.tolerances.aucun_seuil, .seuil_invalide
  pub fn valider_criteres(c: &Criteres) -> Result<(), RefusCriteres>;
  pub fn conclure(resultats: &Resultats, criteres: &Criteres) -> (Conclusion, Vec<Controle>);
  ```
- `conclure` : un `Controle` par seuil renseigné, dans l'ordre fidélité puis prédiction, moyenne, 95e centile, maximum. `valeur: None` et `respecte: None` quand la famille n'a pas de statistiques. Règle : voir « Choix de l'assistant ». `seuil_gamut` doit valoir celui de `resultats` : l'appelant passe par `reclasser` avant.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn tous_les_seuils_respectes_donnent_accepte() {}
#[test] fn un_maximum_depasse_donne_hors_tolerance_et_nomme_le_critere() {}  // Controle { Prediction, Max, respecte: Some(false) }
#[test] fn sans_demande_une_fidelite_requise_rend_incomplet() {}              // critère requis absent
#[test] fn sans_plage_dans_le_gamut_une_fidelite_requise_rend_incomplet() {}
#[test] fn un_depassement_l_emporte_sur_un_critere_incalculable() {}         // HorsTolerance
#[test] fn une_famille_sans_seuil_n_est_pas_requise() {}                      // prédiction seule, sans demande → Accepte
#[test] fn un_jeu_sans_aucun_seuil_est_refuse() {}
#[test] fn un_seuil_nul_ou_negatif_est_refuse() {}
#[test] fn les_criteres_se_relisent_a_l_identique_en_json() {}
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p verification --test tolerances` → échec de compilation.
- [ ] **Step 3 : Implémenter `tolerances.rs`** et l'exporter depuis `lib.rs`.
- [ ] **Step 4 : Vérifier** — Run: `cargo test -p verification` → PASS.
- [ ] **Step 5 : Commit** — `git commit -am "Vérification : tolérances, contrôles et conclusion, critère absent dit incomplet"`

---

### Tâche 3 : `profilage` — interroger le profil par `xicclu`

**Files:**
- Create: `crates/profilage/src/evaluation.rs`
- Modify: `crates/profilage/src/lib.rs`, `crates/profilage/src/moteur.rs` (`Etape::Verification`, `OutilManquant` s'il n'y est pas), `crates/profilage/src/argyll.rs` (visibilités du plan 5 si absentes), `docs/references/icc-argyll-options.md` (section `xicclu`)
- Test: `crates/profilage/tests/evaluation.rs`, `crates/profilage/tests/argyll.rs` (tests `#[ignore]`)

**Interfaces:**
- Consumes: `ArgyllCms::{outil, lancer, version}`, `DossierTravail`, `controler_version`, `ligne` (plan 3 et plan 5).
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Impression { SansGestion, ATraversLeProfil } // à travers : sRVB, colorimétrique relatif, sans compensation du point noir
  pub struct DemandeEvaluation<'a> { pub icc: &'a [u8], pub rvb: &'a [[f64; 3]] /* 0 à 1 */, pub impression: Impression }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Evaluation {
      pub demande: Option<Vec<[f64; 3]>>,  // Lab ; None sans gestion
      pub appareil: Vec<[f64; 3]>,
      pub prediction_rel: Vec<[f64; 3]>,
      pub prediction_abs: Vec<[f64; 3]>,
      pub blanc_support: [f64; 3],         // Lab absolu prédit pour l'appareil (1, 1, 1)
      pub version: String, pub commandes: Vec<String>, pub journal: String,
  }
  pub trait MoteurVerification: Send + Sync {
      fn commandes_verification(&self, impression: Impression) -> Vec<String>;
      fn evaluer(&self, demande: &DemandeEvaluation) -> Result<Evaluation, ErreurMoteur>;
  }
  impl MoteurVerification for ArgyllCms {}
  pub struct MoteurVerificationSimule { /* … */ }
  impl MoteurVerificationSimule {
      pub fn fidele() -> Self;                       // appareil = rvb ; Lab = fonction simple des RVB (celle du test synthétique du plan 3)
      pub fn rend(evaluation: Evaluation) -> Self;
      pub fn echoue(erreur: ErreurMoteur) -> Self;
  }
  pub fn arguments_xicclu(fonction: &str /* "f" | "b" */, intention: &str /* "r" | "a" */, profil: &Path) -> Vec<OsString>;
  pub fn lire_sortie_xicclu(sortie: &str, attendues: usize) -> Result<Vec<[f64; 3]>, String>;
  ```
- Déroulé de `evaluer` (supposé, confirmé au step 5) : écrire `profil.icc` dans le dossier de travail ; avec gestion, `xicclu -v0 -ff -ir -pl sRGB.icm` (RVB → demande), puis `xicclu -v0 -fb -ir -pl profil.icc` (demande → appareil) ; sans gestion, appareil = rvb ; puis `xicclu -v0 -ff -ir -pl profil.icc` et `-ia` sur les valeurs d'appareil suivies de `1 1 1` (prédictions et blanc du support). Chaque appel lit ses valeurs sur l'entrée standard, une ligne par plage. Version contrôlée d'abord ; `xicclu` absent → `OutilManquant { outil: "xicclu" }` ; nombre de lignes lues différent du nombre attendu → `Echec { etape: Verification, cause }`. `commandes` : les lignes de commande réduites aux noms de fichier, comme au plan 3.
- `ErreurMoteur::cle` : `Echec { etape: Verification, .. }` → `verif.erreur.calcul`.

- [ ] **Step 1 : Tests en échec (`tests/evaluation.rs`)**

```rust
#[test]
fn xicclu_est_appele_en_relatif_vers_lab() {
    let a: Vec<_> = arguments_xicclu("b", "r", Path::new("profil.icc")).iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert_eq!(a, ["-v0", "-fb", "-ir", "-pl", "profil.icc"]);
}
#[test] fn la_sortie_de_xicclu_se_lit_ligne_par_ligne() {}           // "52.1 -0.3 1.2\n…" → [[52.1, -0.3, 1.2], …]
#[test] fn une_sortie_trop_courte_est_refusee() {}
#[test] fn sans_gestion_les_commandes_ne_demandent_pas_srgb() {}    // aucune ligne ne cite sRGB.icm
#[test] fn le_moteur_simule_fidele_rend_l_appareil_egal_aux_rvb() {}
#[test] fn sans_xicclu_l_outil_est_dit_manquant() {}                // dossier argyll factice avec colprof, profcheck, sRGB.icm seulement
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p profilage --test evaluation` → échec de compilation.
- [ ] **Step 3 : Implémenter `evaluation.rs`** (sous 400 lignes, `MoteurVerificationSimule` compris).
- [ ] **Step 4 : Vérifier** — Run: `cargo test -p profilage` → PASS.

- [ ] **Step 5 : Confirmer `xicclu` avec ArgyllCMS 3.5.0 (tests `#[ignore]` dans `tests/argyll.rs`)**

```rust
#[test] #[ignore] fn xicclu_evalue_un_profil_calcule() {}
    // profil de la mire synthétique du plan 3 ; 20 RVB : sorties de bonne taille, blanc du support L > 90
#[test] #[ignore] fn notre_passage_en_relatif_concorde_avec_argyll() {}
    // verification::vers_relatif(prediction_abs, blanc_support) contre prediction_rel : ΔE00 ≤ 0,2 pour chaque plage
```
Run: `cargo test -p profilage --test argyll -- --ignored --nocapture`
Expected: PASS. Sinon, relever la vraie forme de sortie de `xicclu -v0` (séparateurs, échelle des valeurs d'appareil) et corriger `lire_sortie_xicclu`, ou passer `vers_relatif` en adaptation de Bradford si l'écart dépasse 0,2. Noter les constats (« confirmé le … avec 3.5.0 ») dans `docs/references/icc-argyll-options.md`.

- [ ] **Step 6 : Commit** — `git commit -am "Profilage : interroger un profil par xicclu derrière le trait MoteurVerification"`

---

### Tâche 4 : Bibliothèque — jeux de tolérances et vérifications immuables

**Files:**
- Create: `crates/bibliotheque/src/verifications.rs`
- Modify: `crates/bibliotheque/src/lib.rs` (`VERSION_BASE`, appel de l'organisation dans `migrer`, `pub use`), `crates/bibliotheque/src/profils.rs` (`mesures_profil`)
- Test: `crates/bibliotheque/tests/verifications.rs`

**Interfaces:**
- Consumes: `IdProfil`, `IdMesure`, `EtatVerification`, `verifier_condition`, `instant` (crate `bibliotheque`).
- Produces (la crate ne dépend pas de `verification` : critères, fiche et calcul y sont du JSON opaque) :
  ```rust
  pub struct IdTolerances(pub i64);
  pub struct IdVerification(pub i64);
  #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
  pub struct JeuTolerances { pub id: IdTolerances, pub nom: String, pub version: u32, pub horodatage: String, pub criteres: String, pub utilise: bool }
  pub struct NouvelleVerification<'a> {
      pub profil: IdProfil, pub mesure: IdMesure, pub tolerances: IdTolerances, pub horodatage: &'a str,
      pub fiche: &'a str, pub calcul: &'a str, pub etat: EtatVerification, pub recalcul_de: Option<IdVerification>,
  }
  #[derive(Clone, Debug, PartialEq, Serialize)]
  pub struct VerificationRangee {
      pub id: IdVerification, pub profil: IdProfil, pub mesure: IdMesure, pub tolerances: JeuTolerances,
      pub horodatage: String, pub fiche: String, pub calcul: String, pub etat: EtatVerification, pub recalcul_de: Option<IdVerification>,
  }
  impl Bibliotheque {
      pub fn enregistrer_tolerances(&self, nom: &str, criteres: &str, horodatage: &str) -> Resultat<IdTolerances>; // version = dernière du nom + 1
      pub fn jeux_tolerances(&self) -> Resultat<Vec<JeuTolerances>>;  // par nom, puis version décroissante
      pub fn jeu_tolerances(&self, id: IdTolerances) -> Resultat<JeuTolerances>;
      pub fn enregistrer_verification(&self, v: &NouvelleVerification) -> Resultat<IdVerification>; // une transaction : insertion + état du profil
      pub fn verifications(&self, profil: IdProfil) -> Resultat<Vec<VerificationRangee>>;          // la plus récente d'abord
      pub fn verification(&self, id: IdVerification) -> Resultat<VerificationRangee>;
      pub fn mesures_profil(&self, profil: IdProfil) -> Resultat<Vec<IdMesure>>;
      pub fn condition_profil(&self, profil: IdProfil) -> Resultat<IdCondition>;
  }
  ```
- Nouvelles erreurs : `ErreurBibliotheque::ToleranceInconnue(IdTolerances)`, `VerificationInconnue(IdVerification)`, `EtatVerificationInvalide` (`NonVerifie` passé à `enregistrer_verification`). Un nom de jeu vide → `NomVide`.
- Organisation (numéro : voir « Recouvrements ») :
  ```sql
  CREATE TABLE tolerances (
      id INTEGER PRIMARY KEY, nom TEXT NOT NULL, version INTEGER NOT NULL CHECK (version > 0),
      horodatage TEXT NOT NULL, criteres TEXT NOT NULL, UNIQUE (nom, version)
  );
  CREATE TABLE verifications (
      id INTEGER PRIMARY KEY, profil INTEGER NOT NULL REFERENCES profils (id),
      mesure INTEGER NOT NULL REFERENCES mesures (id), tolerances INTEGER NOT NULL REFERENCES tolerances (id),
      horodatage TEXT NOT NULL, fiche TEXT NOT NULL, calcul TEXT NOT NULL,
      etat TEXT NOT NULL CHECK (etat IN ('accepte', 'hors_tolerance', 'incomplet')),
      recalcul_de INTEGER REFERENCES verifications (id)
  );
  CREATE INDEX verifications_par_profil ON verifications (profil);
  CREATE TRIGGER tolerances_immuables BEFORE UPDATE ON tolerances BEGIN SELECT RAISE(ABORT, 'jeu de tolérances immuable'); END;
  CREATE TRIGGER tolerances_gardees BEFORE DELETE ON tolerances BEGIN SELECT RAISE(ABORT, 'jeu de tolérances immuable'); END;
  CREATE TRIGGER verifications_immuables BEFORE UPDATE ON verifications BEGIN SELECT RAISE(ABORT, 'vérification immuable'); END;
  CREATE TRIGGER verifications_gardees BEFORE DELETE ON verifications BEGIN SELECT RAISE(ABORT, 'vérification immuable'); END;
  ```

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn un_jeu_se_range_en_version_1_et_se_relit() {}
#[test] fn modifier_un_jeu_cree_une_version_sans_toucher_l_ancienne() {}  // même nom → version 2 ; version 1 relue à l'identique
#[test] fn un_jeu_utilise_le_dit() {}                                     // utilise == true après une vérification
#[test] fn une_verification_donne_son_etat_au_profil() {}                 // HorsTolerance → profil HorsTolerance, toujours actif
#[test] fn un_recalcul_s_ajoute_et_garde_l_original() {}                  // deux vérifications, recalcul_de == Some(première), état du profil = la plus récente
#[test] fn la_base_refuse_de_modifier_une_verification() {}              // UPDATE direct dans la base → erreur
#[test] fn non_verifie_n_est_pas_un_resultat() {}                         // EtatVerificationInvalide, rien n'est rangé
#[test] fn une_base_de_l_organisation_precedente_passe_sans_rien_perdre() {}
#[test] fn les_mesures_d_un_profil_calcule_se_relisent() {}
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p bibliotheque --test verifications` → échec de compilation.
- [ ] **Step 3 : Implémenter `verifications.rs`** et l'organisation, dans le style des précédentes (commentaire en tête, rien n'est réécrit ni effacé).
- [ ] **Step 4 : Vérifier** — Run: `cargo test -p bibliotheque` → PASS, tests existants compris.
- [ ] **Step 5 : Commit** — `git add crates/bibliotheque && git commit -m "Bibliothèque : tolérances versionnées et vérifications immuables"`

---

### Tâche 5 : Vérifier un profil dans l'application

**Files:**
- Create: `app/src/verifier.rs`
- Modify: `app/Cargo.toml` (dépendance `verification`), `app/src/lib.rs` (module, commandes), `app/src/profiler.rs` (`Moteur::verification`), `app/src/parc.rs` (`DetailReglage::verification`), `app/src/textes.rs`
- Test: `app/tests/verifier.rs`

**Interfaces:**
- Consumes: `preparer_plages`, `conditions_utilisables`, `MoteurVerification`, `Impression`, `Evaluation` (`profilage`) ; `comparer`, `reclasser`, `conclure`, `valider_criteres`, `Criteres` (`verification`) ; tâche 4 ; `ErreurProfil`, `SourceProfil`, `sources_profil`, `instant_utc`.
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct FicheTirage {
      pub impression: Impression, pub logiciel: String, pub support_pilote: String,
      pub qualite: String, pub encres: String, pub remarques: String, pub confirmee: bool,
  }
  #[derive(Clone, Debug, Deserialize)]
  pub struct DemandeVerification { pub profil: IdProfil, pub mesure: IdMesure, pub mesure_condition: ConditionMesure, pub tolerances: IdTolerances, pub fiche: FicheTirage }
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct CalculVerification {   // le JSON rangé dans `verifications.calcul`
      pub fichier: String, pub instrument: Option<String>, pub condition: ConditionMesure, pub condition_supposee: bool,
      pub memes_couleurs_que_le_profil: bool, pub evaluation: Evaluation, pub resultats: Resultats, pub controles: Vec<Controle>,
  }
  #[derive(Serialize)]
  pub struct ResultatVerification { pub verification: IdVerification, pub etat: EtatVerification, pub calcul: CalculVerification }
  #[derive(Serialize)]
  pub struct ResumeVerification { pub verification: IdVerification, pub horodatage: String, pub etat: EtatVerification, pub tolerances: String /* « Atelier v2 » */ }
  impl BibliothequeOuverte {
      pub fn sources_verification(&self, profil: IdProfil) -> Result<Vec<SourceProfil>, ErreurProfil>;
      pub fn derniere_fiche(&self, condition: IdCondition) -> Result<Option<FicheTirage>, ErreurProfil>; // confirmee remise à false
      pub fn enregistrer_tolerances(&self, nom: &str, criteres: &Criteres, maintenant: SystemTime) -> Result<IdTolerances, ErreurProfil>;
      pub fn jeux_tolerances(&self) -> Result<Vec<JeuTolerances>, ErreurProfil>;
      pub fn verifier_profil(&self, demande: &DemandeVerification, moteur: &dyn MoteurVerification, maintenant: SystemTime) -> Result<ResultatVerification, ErreurProfil>;
      pub fn recalculer_verification(&self, verification: IdVerification, tolerances: IdTolerances, maintenant: SystemTime) -> Result<ResultatVerification, ErreurProfil>;
      pub fn historique_verifications(&self, profil: IdProfil) -> Result<Vec<ResumeVerification>, ErreurProfil>;
  }
  // DetailReglage::verification: Option<ResumeVerification> — la plus récente du profil actif.
  // Commandes : verifier_sources, verifier_commandes, verifier_lancer (async), verifier_recalculer,
  //             tolerances_lister, tolerances_enregistrer, verifier_historique.
  ```
- Refus, avant tout calcul (`ErreurProfil::sans_cause`) : fiche non confirmée → `verif.refus.fiche`; mesure parmi `mesures_profil` → `verif.refus.mesure_du_profil` (aussi `raison` de la source) ; mesure d'une autre résolution que le profil → `profil.erreur.mesure_ailleurs` ; profil CMJN → `verif.refus.cmjn` ; refus de `preparer_plages` et de `comparer` → leur clé. `Moteur::verification()` : `None` → `profil.erreur.argyll_introuvable`.
- `verifier_profil` lit sous le verrou, relâche, appelle `moteur.evaluer`, puis `comparer` (avec `criteres.seuil_gamut`) et `conclure`, et range par `enregistrer_verification`. `Conclusion` → `EtatVerification` (`Accepte`, `HorsTolerance`, `Incomplet`). `recalculer_verification` relit le calcul rangé, applique `reclasser` puis `conclure` avec le nouveau jeu, et range une vérification `recalcul_de` l'originale, même mesure, même fiche.

- [ ] **Step 1 : Tests en échec (`app/tests/verifier.rs`, bibliothèque temporaire, `MoteurSimule` pour calculer le profil, `MoteurVerificationSimule`)**

```rust
#[test] fn un_profil_fidele_est_accepte_selon_la_version_du_jeu() {}       // état Accepte ; ResumeVerification.tolerances == "Atelier v1"
#[test] fn un_profil_hors_tolerance_reste_actif_et_exportable() {}
#[test] fn la_mesure_du_profil_ne_peut_pas_le_verifier() {}
#[test] fn une_fiche_non_confirmee_est_refusee() {}
#[test] fn sans_gestion_avec_un_critere_de_fidelite_la_verification_est_incomplete() {}
#[test] fn verifier_un_ancien_profil_ne_touche_pas_le_profil_actif() {}
#[test] fn sans_xicclu_rien_n_est_range() {}                                  // MoteurVerificationSimule::echoue(OutilManquant) → Err("profil.erreur.outil_manquant")
#[test] fn un_recalcul_avec_une_nouvelle_version_ajoute_une_verification() {} // historique : 2 entrées, l'originale intacte
#[test] fn la_derniere_fiche_est_reprise_non_confirmee() {}
#[test] fn les_memes_couleurs_que_la_mire_du_profil_sont_signalees() {}
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p app --test verifier` → échec de compilation.
- [ ] **Step 3 : Implémenter `verifier.rs`** (sous 400 lignes ; commandes `#[tauri::command(async)]` pour `verifier_lancer`).
- [ ] **Step 4 : Textes FR et EN dans `textes.rs`** : `verif.titre` « Vérifier le profil », `verif.mesure` « Mesure de la seconde mire », `verif.tolerances` « Tolérances », `verif.fiche` « Fiche d'impression », `verif.fiche.impression.a_travers_le_profil` « À travers ce profil, depuis des couleurs sRVB, en colorimétrique relatif », `verif.fiche.impression.sans_gestion` « Sans gestion des couleurs », `verif.fiche.confirmer` « Je confirme ces réglages », `verif.lancer` « Vérifier », `verif.en_cours`, `verif.fidelite` « Fidélité aux couleurs demandées », `verif.prediction` « Accord avec la prédiction du profil », `verif.hors_gamut` « Couleurs hors gamut », `verif.inconnu.*` (une raison par cas), `verif.etat.*`, `verif.critere.*`, `verif.calculee` « calculée par ArgyllCMS », `verif.memes_couleurs`, toutes les clés `verif.refus.*`, `verif.erreur.*`, `verif.tolerances.*`. Chaque refus dit la cause puis l'action. Si `profil.erreur.outil_manquant` n'existe pas encore (plan 5 non fusionné), l'ajouter avec le texte du plan 5 : « ArgyllCMS est incomplet : il manque {outil}. Replacez le dossier argyll complet. »
- [ ] **Step 5 : Vérifier** — Run: `cargo test -p app && cargo clippy --all-targets -- -D warnings` → PASS (tests de typographie de `textes.rs` compris).
- [ ] **Step 6 : Commit** — `git commit -am "Application : vérifier un profil sur une seconde mire importée, recalcul sans remesure"`

---

### Tâche 6 : Le rapport, et l'export du profil avec son rapport

**Files:**
- Create: `app/src/rapport.rs`
- Modify: `app/src/verifier.rs` (commandes d'export), `app/src/lib.rs` (enregistrement), `app/src/textes.rs` (`rapport.*`)
- Test: `app/tests/rapport.rs`

**Interfaces:**
- Consumes: `VerificationRangee`, `CalculVerification`, `FicheTirage`, `DetailReglage`, `ResumeProfil`, `provenance_profil`, `textes::{texte, Langue}`.
- Produces:
  ```rust
  pub struct DonneesRapport { /* imprimante, papier, résolution, profil (nom, date, empreinte, provenance),
                                 vérification rangée, jeu de tolérances, calcul, fiche */ }
  impl BibliothequeOuverte {
      pub fn donnees_rapport(&self, verification: IdVerification) -> Result<DonneesRapport, ErreurProfil>;
      pub fn exporter_rapport(&self, verification: IdVerification, langue: Langue, fichier: &Path) -> Result<(), String>;
      pub fn exporter_profil_et_rapport(&self, verification: IdVerification, langue: Langue, fichier_icc: &Path) -> Result<(), String>;
      // écrit le .icc puis, à côté, le même nom en .html
  }
  pub fn rapport_html(d: &DonneesRapport, langue: Langue) -> String;
  // Commandes : verifier_exporter_rapport(verification, langue, filtre), verifier_exporter_profil(verification, langue, filtre)
  ```
- Contenu, dans cet ordre : verdict (mot + symbole) et règle d'acceptation ; jeu de tolérances (nom, version, date, seuils) ; fidélité, hors gamut, prédiction, chacune avec moyenne, 95e centile, maximum, nombre de plages et pires plages, ou « inconnu » avec sa raison ; contrôles un par un ; profil (nom, date, empreinte SHA-256, recette, version d'ArgyllCMS) ; source (fichier, instrument déclaré ou « inconnu », condition de mesure, « supposée ») ; fiche d'impression ; mention « mêmes couleurs que la mire du profil » s'il y a lieu ; commandes ArgyllCMS ; tableau de toutes les plages (id, RVB, demande, valeur après transformation, prédiction, mesure, trois écarts). Tout texte variable est échappé (`&`, `<`, `>`, `"`, `'`). Aucune balise `<script>`, aucun lien externe.

- [ ] **Step 1 : Tests en échec**

```rust
#[test] fn le_rapport_porte_les_deux_familles_et_la_version_des_tolerances() {}  // contient « Atelier », « v1 », les deux titres de famille
#[test] fn le_rapport_d_une_ancienne_verification_ne_change_pas() {}               // jeu passé en v2 : rapport de la vérification v1 identique à l'octet
#[test] fn un_rapport_est_dans_une_seule_langue() {}                                // anglais : aucune clé française du catalogue
#[test] fn les_noms_sont_echappes() {}                                              // papier « Canson <Baryta> & co » → &lt;Baryta&gt; &amp;
#[test] fn le_rapport_n_a_ni_script_ni_lien_externe() {}
#[test] fn un_inconnu_est_ecrit_en_toutes_lettres() {}                              // sans gestion : « inconnu » et sa raison, jamais « 0,00 »
#[test] fn le_profil_et_son_rapport_s_exportent_cote_a_cote() {}                   // dossier temporaire : x.icc identique au profil rangé, x.html présent
```

- [ ] **Step 2 : Lancer, constater l'échec** — Run: `cargo test -p app --test rapport` → échec de compilation.
- [ ] **Step 3 : Implémenter `rapport.rs`** (sous 400 lignes ; styles intégrés en noir et gris, mise en page A4 pour l'impression ; nombres au format de la langue, virgule en français).
- [ ] **Step 4 : Vérifier** — Run: `cargo test -p app` → PASS.
- [ ] **Step 5 : Commit** — `git commit -am "Rapport de vérification en HTML, exporté seul ou avec le profil"`

---

### Tâche 7 : La section « Vérifier le profil » dans la vue d'une résolution

**Files:**
- Create: `app/interface/verification.js` (chargé par `index.html` avant `reglage.js`, sur le modèle de `calcul-profil.js`)
- Modify: `app/interface/index.html`, `app/interface/reglage.js` (bouton d'ouverture et provenance), `design-system/components.css` (seulement si une classe manque ; jetons existants uniquement)
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes: commandes de la tâche 5 et de la tâche 6 ; `agir()`, `annoncer()` et la résolution affichée, prêtés par `reglage.js` comme pour `window.CalculProfil`.

Comportement :
- Un bouton « Vérifier le profil » ouvre une section repliable, qui résume ses choix quand elle est fermée. Il est inactif sans profil actif RVB, et dit pourquoi.
- La section propose : le profil (actif présélectionné, ceux de l'historique ensuite) ; la mesure (les mesures importées de la résolution, la plus récente d'abord ; la mesure du profil reste visible, inactive, avec sa raison) ; la condition de mesure (M1 si présente) ; le jeu de tolérances (dernière version de chaque nom) ; la fiche d'impression, reprise de la dernière vérification et à confirmer par une case ; puis « Avancé » replié, avec les commandes `xicclu` en lecture seule.
- « Tolérances » ouvre un petit éditeur : nom, trois seuils par famille (vides permis), seuil de gamut. Sans aucun jeu enregistré, il est prérempli avec les valeurs de départ, marquées « proposition, à confirmer ». « Enregistrer » range une nouvelle version et le dit (« Atelier v2 enregistré ; les vérifications faites avec v1 ne changent pas »).
- Seul bouton principal : « Vérifier ». Pendant le calcul, les boutons de la vue attendent (`agir()`).
- Résultat : le grand verdict (couleur + icône + mot, rouge seulement pour hors tolérance), puis trois cartes, fidélité, hors gamut et prédiction (moyenne, 95e centile, maximum, nombre de plages), les contrôles un par un, et les pires plages dans un tableau. Les valeurs demandées, calculées, prédites et mesurées ont chacune leur colonne et leur libellé. « Recalculer avec d'autres tolérances » et « Exporter le rapport » à côté ; « Exporter le profil avec son rapport » dans la vue.
- À droite, dans la provenance du profil : la dernière vérification (date, état, jeu et version), et l'historique de ses vérifications.

- [ ] **Step 1 : Tests en échec dans `app/tests/page.rs`**

```rust
#[test] fn la_vue_d_une_resolution_appelle_la_verification() {}        // verification.js contient "verifier_lancer", "tolerances_enregistrer", "verifier_exporter_rapport"
#[test] fn verification_js_est_charge_avant_reglage_js() {}
#[test] fn toutes_les_cles_verif_de_la_page_existent_en_deux_langues() {}
#[test] fn verification_js_n_a_pas_de_couleur_en_dur() {}               // aucun « # » suivi de 3 ou 6 chiffres hexadécimaux, aucun rgb(
```

- [ ] **Step 2 : Implémenter** (fichier sous 400 lignes ; au-delà, sortir l'éditeur dans `app/interface/tolerances.js`).
- [ ] **Step 3 : Vérifier** — Run: `cargo test -p app --test page` → PASS.
- [ ] **Step 4 : Commit** — `git commit -am "Interface : vérifier un profil depuis la vue d'une résolution"`

---

### Tâche 8 : Installateur, documentation, essai d'écran et PR

**Files:**
- Modify: `outils/preparer_installateur.py` et `outils/test_preparer_installateur.py` (`bin/xicclu.exe` dans `ARGYLL_FICHIERS`, s'il n'y est pas), `CLAUDE.md` (crate `verification`, module `verifier`, `CRATES`), `docs/adr/0002-argyllcms-inclus.md` (`xicclu` embarqué), `docs/adr/0004-crates-partagees-rip.md` (contrat de la crate `verification`), `GLOSSARY.md`, `docs/blog/<jour>.md`

- [ ] **Step 1 : Installateur** — ajouter `bin/xicclu.exe` et son test. Run: `python outils/test_preparer_installateur.py` → OK.
- [ ] **Step 2 : Documentation** — ADR 0004 : entrées, règle du relatif, seuil de gamut, centile, règle d'acceptation (changer l'une d'elles change les verdicts : rupture de contrat). Glossaire : fidélité aux couleurs demandées, accord avec la prédiction, valeur après transformation, seuil de gamut, jeu de tolérances, version de tolérances, vérification incomplète, recalcul. Journal du jour en mots simples.
- [ ] **Step 3 : Contrôles** — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/verifier_docs.py` → tout vert.
- [ ] **Step 4 : Essai d'écran avec le mainteneur (geste humain, sur Epson pigmentaire / semigloss si possible)**
  1. Profil calculé (plan 3) depuis une mesure FD-S2w de la mire de profilage.
  2. Imprimer la seconde mire choisie (voir « Décisions du mainteneur » : RGB-2520 réimprimée) depuis le logiciel de l'atelier, à travers ce profil, en colorimétrique relatif ; la mesurer avec FD-S2w et importer l'export dans la résolution.
  3. Enregistrer le jeu de tolérances, remplir et confirmer la fiche, vérifier.
  4. Noter les trois statistiques de chaque famille, le nombre de plages hors gamut et le verdict ; ouvrir le rapport exporté dans un navigateur et l'imprimer en PDF.
  5. Modifier le jeu (v2), recalculer : deux vérifications dans l'historique, la première intacte.
  6. Reporter les paramètres de l'essai (imprimante, papier, pilote, logiciel) dans le journal.
- [ ] **Step 5 : PR vers `dev`** avec les choix de l'assistant, le Review Focus et les résultats de l'essai. Commenter #41 : livré pour l'import FD-S2w ; restent la mire de vérification produite par myiro-libre et la mesure directe (seconde partie du plan 7).

---

### Tâche 9 : Brancher le contrôle de cohérence (#1), quand il est dans `dev`

**Condition :** `profilage` exporte le contrôle de la tâche 8 du plan 4. Sinon, cette tâche attend : la noter dans la PR et sur #1.

**Files:**
- Modify: `app/src/verifier.rs`
- Test: `app/tests/verifier.rs`

- [ ] **Step 1 : Test en échec** — `#[test] fn des_mesures_incoherentes_bloquent_la_verification() {}` : une mesure dont les plages sont décalées d'un rang est refusée avec la clé du contrôle et les plages en cause ; rien n'est rangé.
- [ ] **Step 2 : Appeler le contrôle** juste après `preparer_plages`, avec les mêmes seuils que pour le calcul de profil.
- [ ] **Step 3 : Vérifier et commiter** — Run: `cargo test -p app --test verifier` → PASS. `git commit -am "Vérification : mesures incohérentes refusées avant tout calcul (#1)"`

---

## Décisions du mainteneur (28 octobre 2026)

Réponses données au général le 28 octobre 2026 : « 1 oui, 2b, 3 ok, 4 oui, 5 HTML ».

1. **Seconde mire imprimée** depuis le logiciel habituel, couleurs lues comme du sRVB, converties par le profil en **colorimétrique relatif, sans compensation du point noir** : validé.
2. **Seconde mire pour l'essai au FD-9** : **RGB-2520 réimprimée à travers le profil** (mêmes couleurs que la mire du profil : contrôle moins exigeant, signalé dans le rapport).
3. **Valeurs de départ du jeu « Atelier »**, en ΔE00, validées :
   - prédiction : moyenne ≤ 1,5 · 95 % des plages ≤ 3,0 · maximum ≤ 5,0 ;
   - fidélité, couleurs dans le gamut : moyenne ≤ 2,0 · 95 % ≤ 4,0 · maximum ≤ 6,0.
4. **Seuil « hors gamut »** : **1,0 ΔE00** prédit par le profil, validé.
5. **Rapport en HTML** (ouvert dans le navigateur, imprimé en PDF par Windows), en noir et gris : validé.
