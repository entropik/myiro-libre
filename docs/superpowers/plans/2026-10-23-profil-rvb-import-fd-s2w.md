# Calcul d'un profil RVB à partir d'une mesure importée (FD-S2w) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dans la vue d'une résolution, l'imprimeur choisit une mesure importée d'un export CGATS de FD-S2w et obtient en un clic un profil ICC RVB calculé par ArgyllCMS. Le profil devient le profil actuel, « non vérifié », avec sa précision, sa commande et son journal conservés.

**Architecture:** La crate `cgats` lit en plus les valeurs RVB des plages. Une nouvelle crate `profilage`, indépendante de Tauri, de Windows et des ponts, prépare et contrôle les plages, écrit le `.ti3` et pilote ArgyllCMS (`colprof` puis `profcheck`) derrière un trait `MoteurProfil` à deux adapters : `ArgyllCms` (réel) et `MoteurSimule` (tests), sur le modèle de l'ADR 0005. L'application ajoute le module `app/src/profiler.rs` : les méthodes de `BibliothequeOuverte` se testent sans Tauri, et le calcul se fait hors du verrou de la bibliothèque. La page reçoit une section « Calculer un profil » dans `app/interface/reglage.js`. Le profil est rangé par `Bibliotheque::enregistrer_profil`, sans migration de base.

**Tech Stack:** Rust 2021, Tauri 2, `std::process` (aucune nouvelle dépendance), ArgyllCMS 3.5.0 pour Windows 64 bits (version figée, ADR 0002), HTML, CSS et JavaScript simples (ADR 0003).

**Spec:** `docs/specifications/2026-10-20-poste-unique-design.md` (étape 4 « Profil » du parcours, « Transparence des calculs », « Le spectre d'abord, provenance visible ») ; `docs/specifications/poste-travail-couleur.md` (première livraison, règles transversales) ; ticket #40 (critères d'acceptation) ; références `docs/references/icc-argyll-options.md` et `docs/references/icc-cgats-argyll-formats.md`.

## Feuille de route mise à jour (23 octobre 2026)

Décision du mainteneur du 23 octobre 2026 : le calcul de profil passe avant les mires. Le FD-9 est mesuré chaque jour avec FD-S2w, dont l'export CGATS donne valeurs RVB, Lab et spectres. Calculer le profil à partir de cet export rend « Refaire le profil » utile tout de suite, sans attendre les mires (#33) ni la mesure de feuille au FD-9 (#34). La feuille de route du plan 1 devient :

1. Bibliothèque : parc et profils (fait, #66).
2. Nouveau cadre d'interface (fait, #68).
3. **Calcul d'un profil RVB à partir d'une mesure importée** (ce plan, première partie de #40).
4. Mires de profilage RVB, TIFF sans gestion couleur, QR code (#33).
5. Mesure d'une feuille au FD-9 (#34), puis guidage de la mesure, reprise et moyenne (#35).
6. Vérification du profil sur une seconde mire, tolérances et rapport (#41) ; parcours complet en 4 étapes.
7. Séchage et reprise : « en attente de mesure ».
8. Reprise de l'existant : profils épars, papiers du RIP, fichiers MYIROtools.

Hors de ce plan, et laissés à #35 et #41 : moyenne de plusieurs exemplaires, mire produite par myiro-libre, vérification et tolérances, réglages avancés (azurants, limites), CMJN.

## Global Constraints

- Travail dans un worktree `ticket/40-profil-rvb-import` issu de `dev`, PR vers `dev` (`AGENTS.md`). Avant d'écrire : `python outils/verifier_contexte.py --worktree <worktree> --branch ticket/40-profil-rvb-import --reference dev`.
- Les crates `cgats` et `profilage` restent indépendantes de Tauri, de Windows et des ponts (ADR 0004, ADR 0007). Aucune nouvelle dépendance externe.
- ArgyllCMS : version figée `3.5.0`, appelée en processus externe, jamais versionnée dans git (dossier `argyll/` ignoré). Une autre version est refusée en clair (ADR 0002 : une montée de version est une décision explicite).
- Ce qu'un fichier ne dit pas reste inconnu : aucune échelle RVB devinée, aucun Lab inventé, aucune valeur par défaut (`docs/formats/cgats.md`).
- Un profil calculé est enregistré `non_verifie` ; l'écran distingue « calcul réussi » et « qualité vérifiée » (#40).
- La commande équivalente est montrée en lecture seule avant le calcul et conservée après, avec la version d'ArgyllCMS, le journal et la précision (#40).
- Erreur : étape, cause, action ; aucun profil partiel n'est rangé.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (#57) ; fichiers source nouveaux sous 400 lignes ; `lib.rs` et `colonne.rs` de l'application ne reçoivent que l'enregistrement des commandes.
- Aucun texte en dur dans la page : tout passe par `app/src/textes.rs`, en français et en anglais.
- Données de test fictives : jamais de numéro de série, d'adresse réseau ou de nom de client réels. Un fichier de `tests/` ne contient pas « install » dans son nom (droits d'administrateur sous Windows en 32 bits).
- Ne jamais écrire le nom du logiciel concurrent. Ne pas citer `docs/sources/`.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p cgats -p profilage -p bibliotheque -p app`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- **Échelle des valeurs RVB.** Dans un export FD-S2w (`FILE_DESCRIPTOR "FD-S2w"`), l'échelle est 0 à 255, **supposée**. C'est observé sur les exports réels du poste du 7 octobre 2026 (valeurs jusqu'à 251,16), mais aucun document du fabricant ne l'affirme. Dans un fichier `CTI3` d'ArgyllCMS, l'échelle est 0 à 100, confirmée par la description du format. Pour tout autre fichier, l'échelle est inconnue et le calcul est refusé.
- **Recettes de calcul** (qualité de `colprof`) : Rapide `-qm`, **Standard `-qh` (par défaut)**, Approfondi `-qu`. Les trois ajoutent une table perceptuelle à partir de sRGB (`-S <argyll>/ref/sRGB.icm -cmt -dpp`), parce qu'on imprime surtout des fichiers RVB par le pilote. La référence notée le 7 octobre 2026 conseille `-qh` en production et `-qm` pour les essais.
- **Précision affichée** : la moyenne et le maximum des écarts ΔE00 entre les mesures et ce que le profil prédit (`profcheck -k`). C'est une précision de calcul, pas une vérification : l'état reste « non vérifié » jusqu'à #41.
- **Condition de mesure** : M1 par défaut (conception, étape « Mesure »). La page ne propose que les conditions présentes dans la mesure. Une condition seulement supposée (source `D50` du fabricant) est acceptée et dite « supposée » dans le détail.
- **Pas de seuil de plages minimal** inventé : si `colprof` refuse un jeu trop petit, son message est rendu comme cause.
- Après le calcul, « Installer dans Windows » redevient le bouton principal de la vue. Il n'est pas coché d'office : cela viendra avec le parcours en 4 étapes (plan 6).

## Review Focus

1. **Export FD-S2w réel au format Windows-1252, une condition par fichier (M1 seul)** : l'import le range ; le calcul propose M1 et réussit. Un export M0 seul propose M0, sans rien supposer d'autre. → tests `un_export_fd_s2w_donne_des_rvb_supposes_sur_255` (tâche 1) et `seules_les_conditions_presentes_sont_proposees` (tâche 4).
2. **Mesure importée sans colonnes RVB** (mesure ponctuelle, export Lab seul) : le bouton « Calculer le profil » est inactif et dit pourquoi, juste à côté. → test `une_mesure_sans_rvb_ne_peut_pas_servir_de_source` (tâche 4).
3. **ArgyllCMS absent ou d'une autre version** : message clair avec l'action (« Installez ArgyllCMS 3.5.0 dans … »), rien n'est rangé, la vue reste utilisable. → tests `argyll_introuvable_ne_range_rien` (tâche 4) et `une_autre_version_d_argyll_est_refusee` (tâche 3).
4. **Échec de `colprof` en cours de calcul** (jeu de plages dégénéré, disque plein) : l'étape et la dernière ligne d'erreur s'affichent, l'ancien profil reste actif, et aucun fichier temporaire ne reste. → tests `un_echec_du_moteur_laisse_le_profil_actif_inchange` (tâche 4) et `le_dossier_de_travail_est_efface_apres_un_echec` (tâche 3).
5. **Noms d'imprimante ou de papier avec accents, guillemets ou espaces** (« Hahnemühle Photo Rag 308 », « Canson « Baryta » ») dans la description du profil et les chemins passés à ArgyllCMS : le profil se calcule, la description garde les accents, et aucun argument n'est coupé. → test `une_description_accentuee_avec_guillemets_reste_un_seul_argument` (tâche 3).

---

### Tâche 0 : Préparer le poste et le chantier

**Files:**
- Modify: `.gitignore`

- [x] **Step 1 : Contexte**

Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev` puis `git status --short`
Expected: contexte conforme ; noter les changements préexistants (journal du 9 octobre, `.agents/`, `docs/mires/`). Ils ne relèvent pas de ce plan.

- [x] **Step 2 : Créer le worktree du ticket**

Run: `git worktree add .claude/worktrees/ticket-40 -b ticket/40-profil-rvb-import dev`

- [x] **Step 3 : Ignorer ArgyllCMS et les mires de tiers**

Ajouter à `.gitignore`, avec un commentaire en français :
```
# ArgyllCMS (AGPL-3.0), posé à la main pour le développement et embarqué par l'installateur : jamais versionné
argyll/
# Mires de tiers sans droit de redistribution, copie locale de travail
docs/mires/
```

- [ ] **Step 4 : Geste humain, à demander au mainteneur**

Télécharger ArgyllCMS 3.5.0 pour Windows 64 bits sur https://www.argyllcms.com/ et le décompresser à la racine du dépôt principal, sous `argyll/`, de sorte qu'existent `argyll/bin/colprof.exe`, `argyll/bin/profcheck.exe` et `argyll/ref/sRGB.icm`. Les tâches 1 à 5 n'en ont pas besoin : seuls les tests `--ignored` de la tâche 3 et l'essai d'écran de la tâche 6 l'utilisent.

- [x] **Step 5 : Commit**

```bash
git add .gitignore
git commit -m "Ignore ArgyllCMS posé à la main et les mires de tiers"
```

---

### Tâche 1 : Lire les valeurs RVB d'un fichier CGATS

**Files:**
- Modify: `crates/cgats/src/lib.rs` (champ `rvb` de `PlageImportee`), `crates/cgats/src/lecture.rs`
- Modify: `docs/formats/cgats.md` (section « Ce que myiro-libre lit »)
- Test: `crates/cgats/tests/cgats.rs`

**Interfaces:**
- Produces : `PlageImportee::rvb: Info<[f32; 3]>`, valeurs ramenées de 0 à 1 (1 = 100 % de la composante). `Confirmee` pour un fichier `CTI3` (échelle 0 à 100), `Supposee` pour un export FD-S2w (échelle 0 à 255), `Inconnue` sans colonnes RVB ou quand l'échelle n'est pas établie.
- Règles : `RGB_R`, `RGB_G` et `RGB_B` présentes toutes trois ; une ou deux seulement → refus `"tableau {rang} : colonnes RVB incomplètes"`. L'identifiant de la première ligne du fichier (`CTI3`, `CGATS.17`…) est gardé par `tableaux` s'il ne l'est pas déjà. Plusieurs tableaux portent les mêmes RVB, sinon refus `"tableau {n} : pas les mêmes valeurs RVB que le premier tableau"`. Une valeur RVB non numérique est refusée comme un Lab illisible. Une valeur hors échelle est gardée telle quelle : c'est la tâche 2 qui la refuse pour le calcul.

- [x] **Step 1 : Tests en échec**

```rust
#[test]
fn un_export_fd_s2w_donne_des_rvb_supposes_sur_255() {
    // En-tête FD-S2w fictif (FILE_DESCRIPTOR "FD-S2w", MEASUREMENT_SOURCE "D50"),
    // colonnes SAMPLE_ID RGB_R RGB_G RGB_B LAB_L LAB_A LAB_B, deux plages :
    // 255 255 255 / 95 0 2 et 0 117.61 199.05 / 42.09 -28 -60.57.
    let m = lire(&fichier_fd_s2w_rvb()).unwrap();
    assert_eq!(m.plages[0].rvb, Info::Supposee([1.0, 1.0, 1.0]));
    let Info::Supposee(v) = m.plages[1].rvb else { panic!() };
    assert!((v[1] - 117.61 / 255.0).abs() < 1e-6);
}

#[test]
fn un_ti3_d_argyll_donne_des_rvb_confirmes_sur_100() {
    // Première ligne CTI3, RGB_R 50 → 0,5.
}

#[test]
fn sans_colonnes_rvb_les_rvb_sont_inconnus() {}

#[test]
fn un_autre_logiciel_avec_rvb_donne_une_echelle_inconnue() {
    // CGATS.17, ORIGINATOR "autre", RGB_R… présents : rvb == Info::Inconnue.
}

#[test]
fn deux_colonnes_rvb_sur_trois_sont_refusees() {}

#[test]
fn deux_tableaux_aux_rvb_differents_sont_refuses() {}
```

- [x] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p cgats`
Expected: échec de compilation (`rvb` inconnu).

- [x] **Step 3 : Implémenter dans `lecture.rs`**

Dans `lire_tableau`, chercher les trois colonnes avec la même fonction `colonne` (sans tenir compte de la casse). L'échelle se décide une fois par fichier dans `lire`, d'après l'identifiant de la première ligne et le `FILE_DESCRIPTOR` du premier tableau, puis se passe à `lire_tableau`.

- [x] **Step 4 : Documenter**

Dans `docs/formats/cgats.md`, remplacer « Les autres colonnes (densités, RVB, …) sont ignorées » par une phrase sur les RVB, avec un petit tableau « qui écrit / échelle / qualification » sur le modèle de celui des spectres.

- [x] **Step 5 : Vérifier et commiter**

Run: `cargo test -p cgats && cargo test -p bibliotheque`
Expected: PASS (la bibliothèque relit les imports existants à l'identique).

```bash
git commit -am "CGATS : lire les valeurs RVB des plages, échelle jamais devinée"
```

---

### Tâche 2 : Crate `profilage`, préparer les plages et écrire le `.ti3`

**Files:**
- Create: `crates/profilage/Cargo.toml` (dépendances : `cgats`, `pont-protocole`), `crates/profilage/src/lib.rs`, `crates/profilage/src/plages.rs`, `crates/profilage/src/ti3.rs`
- Modify: `Cargo.toml` (membre du workspace), `.github/workflows/ci.yml` (ajouter `profilage` à la liste `CRATES` du job linux)
- Test: `crates/profilage/tests/plages.rs`, `crates/profilage/tests/ti3.rs`

**Interfaces:**
- Produces :
  ```rust
  pub struct PlageProfil { pub rvb: [f64; 3] /* 0 à 100 */, pub lab: [f64; 3], pub spectre: Option<Vec<f64>> /* % */ }
  pub struct PlagesProfil {
      pub plages: Vec<PlageProfil>,
      pub condition: ConditionMesure,
      pub condition_supposee: bool,
      pub rvb_supposes: bool,
      pub longueurs_onde: Option<Echantillonnage>, // Some seulement si toutes les plages ont un spectre
  }
  pub enum RefusPlages {
      ConditionAbsente(ConditionMesure),
      RvbInconnus,                           // aucune colonne RVB, ou échelle inconnue
      LabInconnu { plage: String },          // identifiant de la plage
      RvbHorsEchelle { plage: String },
      Vide,
  }
  pub fn preparer_plages(mesure: &MesureImportee, condition: ConditionMesure) -> Result<PlagesProfil, RefusPlages>;
  pub fn conditions_utilisables(mesure: &MesureImportee) -> Vec<ConditionMesure>; // conditions dont toutes les plages ont un Lab, et RVB connus
  pub fn ecrire_ti3(plages: &PlagesProfil, description: &str, cree: &str) -> String;
  ```
- `RefusPlages` implémente `Display` (phrase française simple) et donne une clé de texte par `pub fn cle(&self) -> &'static str` : `profil.refus.condition_absente`, `profil.refus.rvb_inconnus`, `profil.refus.lab_inconnu`, `profil.refus.rvb_hors_echelle`, `profil.refus.vide`.

- [x] **Step 1 : Tests en échec (`tests/plages.rs`)**

```rust
#[test]
fn une_mesure_fd_s2w_m1_se_prepare() {
    // MesureImportee fictive : 3 plages, rvb Supposee, lab[M1] connus, condition M1 Supposee.
    let p = preparer_plages(&mesure, ConditionMesure::M1).unwrap();
    assert_eq!(p.plages[1].rvb, [0.0, 117.61 / 255.0 * 100.0, 199.05 / 255.0 * 100.0]); // à 1e-4 près
    assert!(p.rvb_supposes && p.condition_supposee);
}
#[test] fn une_condition_absente_est_refusee() {}           // M0 demandé, lab[M0] tous inconnus
#[test] fn un_lab_manquant_nomme_la_plage() {}              // RefusPlages::LabInconnu { plage: "17" }
#[test] fn des_rvb_inconnus_sont_refuses() {}
#[test] fn un_rvb_au_dela_de_100_pour_cent_est_refuse() {}  // 255,01 sur 255 → RvbHorsEchelle
#[test] fn seules_les_conditions_completes_sont_utilisables() {}
#[test] fn sans_spectre_complet_les_longueurs_d_onde_sont_absentes() {}
```

- [x] **Step 2 : Tests en échec (`tests/ti3.rs`)**

```rust
#[test]
fn le_ti3_a_l_en_tete_attendu_par_argyll() {
    let t = ecrire_ti3(&plages_fictives(), "PRO-4100 Rag 2026-10-23", "2026-10-23T09:00:00Z");
    assert!(t.starts_with("CTI3\r\n"));
    for l in ["DEVICE_CLASS \"OUTPUT\"", "COLOR_REP \"iRGB_LAB\"", "ORIGINATOR \"myiro-libre\"",
              "NUMBER_OF_SETS 3", "SAMPLE_ID RGB_R RGB_G RGB_B LAB_L LAB_A LAB_B"] {
        assert!(t.contains(l), "{l}");
    }
}
#[test] fn les_identifiants_du_ti3_sont_les_rangs() {}         // 1, 2, 3 quel que soit SAMPLE_ID d'origine
#[test] fn avec_spectres_les_bandes_sont_declarees() {}         // SPECTRAL_BANDS 36, START 380, END 730, colonnes SPEC_380…SPEC_730, en %
#[test] fn une_description_avec_guillemets_est_doublee() {}      // « Canson "Baryta" » → ""Baryta""
#[test] fn les_nombres_se_relisent_a_l_identique() {}
```

- [x] **Step 3 : Implémenter**

`plages.rs` : lecture de `mesure.plages[i].lab[condition as usize]` et de `rvb` ; conversion 0–1 → 0–100 en f64 ; spectres pris seulement si toutes les plages en ont un pour cette condition et que `mesure.longueurs_onde` est connue. `ti3.rs` : mise en forme sur le modèle de `crates/cgats/src/ecriture.rs` (tabulations, `\r\n`, `KEYWORD` avant tout mot-clé hors CGATS, nombres avec `{}`). `INSTRUMENT_TYPE_SPECTRAL "YES"` avec spectres, `"NO"` sinon.

`COLOR_REP "iRGB_LAB"` reprend l'exemple établi de la description du `.ti3` pour une imprimante RVB. Il est **confirmé à la tâche 3, step 6**, contre `targen -d2` : si ArgyllCMS 3.5.0 écrit autre chose pour « Print RGB », on prend sa valeur et on corrige ce test.

- [x] **Step 4 : Vérifier et commiter**

Run: `cargo test -p profilage && cargo clippy -p profilage --all-targets -- -D warnings`
Expected: PASS.

```bash
git add Cargo.toml .github/workflows/ci.yml crates/profilage
git commit -m "Profilage : plages contrôlées et fichier .ti3 pour ArgyllCMS"
```

---

### Tâche 3 : Piloter ArgyllCMS derrière un trait

**Files:**
- Create: `crates/profilage/src/moteur.rs` (trait, types, `MoteurSimule`), `crates/profilage/src/argyll.rs` (adapter réel, recherche, version, précision)
- Test: `crates/profilage/tests/moteur.rs`, `crates/profilage/tests/argyll.rs` (tests `#[ignore]` avec le vrai ArgyllCMS)

**Interfaces:**
- Consumes : `ecrire_ti3` (tâche 2).
- Produces :
  ```rust
  pub const VERSION_ARGYLL: &str = "3.5.0";
  #[derive(Clone, Copy, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum Recette { Rapide, Standard, Approfondi }
  pub struct DemandeProfil<'a> { pub ti3: &'a str, pub description: &'a str, pub recette: Recette }
  pub struct Precision { pub moyenne_de00: f64, pub max_de00: f64 }
  pub struct ProfilCalcule { pub icc: Vec<u8>, pub version: String, pub commandes: Vec<String>, pub journal: String, pub precision: Precision }
  pub enum Etape { Version, Profil, Controle }
  pub enum ErreurMoteur {
      Introuvable,
      VersionNonValidee { trouvee: String },
      Echec { etape: Etape, cause: String },   // cause : dernière ligne non vide de la sortie d'erreur
      Fichier(String),
  }
  pub trait MoteurProfil: Send + Sync {
      /// Commandes équivalentes, sans rien lancer (section « Avancé »).
      fn commandes(&self, demande: &DemandeProfil) -> Vec<String>;
      fn calculer(&self, demande: &DemandeProfil) -> Result<ProfilCalcule, ErreurMoteur>;
  }
  pub struct ArgyllCms { /* racine contenant bin/ et ref/ */ }
  impl ArgyllCms { pub fn new(racine: PathBuf) -> Self; pub fn version(&self) -> Result<String, ErreurMoteur>; }
  pub fn chercher_argyll(dossier_exe: &Path) -> Option<PathBuf>;
  pub fn arguments_colprof(recette: Recette, description: &str, srgb: &Path, sortie: &Path, base: &Path) -> Vec<OsString>;
  pub fn lire_precision(sortie_profcheck: &str) -> Option<Precision>;
  pub fn lire_version(sortie_colprof: &str) -> Option<String>;       // « Version 3.5.0 » → "3.5.0"
  pub fn controler_version(trouvee: &str) -> Result<(), ErreurMoteur>; // VersionNonValidee si ≠ VERSION_ARGYLL
  pub struct MoteurSimule { /* résultat à rendre et demandes reçues */ }
  impl MoteurSimule { pub fn reussit(icc: Vec<u8>) -> Self; pub fn echoue(e: ErreurMoteur) -> Self; pub fn demandes(&self) -> Vec<(String, Recette)>; }
  ```
- `ErreurMoteur::cle(&self) -> &'static str` : `profil.erreur.argyll_introuvable`, `profil.erreur.argyll_version`, `profil.erreur.calcul`, `profil.erreur.controle`, `profil.erreur.fichier`.

- [x] **Step 1 : Tests en échec (`tests/moteur.rs`)**

```rust
#[test]
fn la_recette_standard_demande_la_qualite_haute() {
    let a = arguments_colprof(Recette::Standard, "Essai", Path::new("ref/sRGB.icm"), Path::new("p.icc"), Path::new("mesure"));
    let a: Vec<_> = a.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert_eq!(a, ["-v", "-qh", "-S", "ref/sRGB.icm", "-cmt", "-dpp", "-D", "Essai", "-O", "p.icc", "mesure"]);
}
#[test] fn rapide_et_approfondi_changent_seulement_la_qualite() {}   // -qm, -qu
#[test]
fn une_description_accentuee_avec_guillemets_reste_un_seul_argument() {
    // « Hahnemühle "Rag" 308 » est un seul élément de la liste, intact.
}
#[test]
fn la_precision_se_lit_dans_la_sortie_de_profcheck() {
    let p = lire_precision("Profile check complete, peak err = 2.512836, avg err = 0.448123, RMS = 0.6\n").unwrap();
    assert_eq!((p.max_de00, p.moyenne_de00), (2.512836, 0.448123));
}
#[test] fn une_sortie_sans_precision_donne_rien() {}
#[test] fn argyll_est_cherche_a_cote_de_l_executable_puis_a_la_racine_du_depot() {}
    // dossier temporaire : <exe>/argyll/bin/colprof(.exe) trouvé d'abord ; sinon <exe>/../../argyll ; sinon None
#[test] fn une_autre_version_d_argyll_est_refusee() {}
    // lire_version("Create ICC profile, Version 3.4.1\n…") == Some("3.4.1"),
    // puis controler_version("3.4.1") == Err(VersionNonValidee { trouvee: "3.4.1" })
#[test] fn le_moteur_simule_rend_son_resultat_et_garde_les_demandes() {}
```

Et un test unitaire dans `argyll.rs` (`#[cfg(test)]`), sur la garde privée `DossierTravail` qui crée le dossier de travail et l'efface quand elle sort de portée :

```rust
#[test]
fn le_dossier_de_travail_est_efface_apres_un_echec() {
    let chemin = {
        let d = DossierTravail::creer().unwrap();
        std::fs::write(d.chemin().join("mesure.ti3"), "x").unwrap();
        let _: Result<(), ()> = Err(()); // un échec du calcul fait sortir la garde de portée
        d.chemin().to_path_buf()
    };
    assert!(!chemin.exists());
}
```

- [x] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p profilage`
Expected: échec de compilation.

- [x] **Step 3 : Implémenter `ArgyllCms::calculer`**

Déroulé, sans `unwrap` :
1. `version()` : lance `bin/colprof` sans argument, lit `Version (\S+)` dans stdout+stderr ; si elle diffère de `VERSION_ARGYLL` → `VersionNonValidee`.
2. Dossier de travail unique sous `std::env::temp_dir()` (`myiro-libre-profil-<pid>-<nanos>`), tenu par la garde `DossierTravail` : il est effacé en fin de calcul, réussi ou non.
3. Écrire `mesure.ti3`, lancer `colprof` avec `arguments_colprof` (sortie `profil.icc`, base `mesure`), puis `profcheck -k mesure.ti3 profil.icc`.
4. Sous Windows, `creation_flags(CREATE_NO_WINDOW)` comme dans `app/src/pont.rs:210`.
5. `journal` = stdout et stderr des deux outils ; `commandes` = les deux lignes de commande, chemins réduits à leur nom (`colprof -v -qh … mesure`). Elles servent à l'affichage et à l'archive.
6. Précision illisible → `Echec { etape: Controle, cause }` : un profil sans précision n'est pas présenté comme terminé.

- [x] **Step 4 : Vérifier**

Run: `cargo test -p profilage`
Expected: PASS.

- [x] **Step 5 : Tests réels `#[ignore]` (`tests/argyll.rs`)**

```rust
#[test] #[ignore] fn argyll_du_depot_est_en_version_3_5_0() {}
#[test] #[ignore] fn un_profil_se_calcule_sur_une_mire_synthetique() {}
    // Mire synthétique : grille RVB 5×5×5 (125 plages), Lab calculés par une
    // fonction simple du test (L = 20 + 0,75 × moyenne RVB, a et b proportionnels
    // à R−G et G−B). Profil ICC valide (`bibliotheque::espace_icc` → Rvb),
    // précision moyenne < 1 ΔE00.
```
Racine d'ArgyllCMS : variable `ARGYLL` ou, à défaut, `<racine du dépôt>/argyll`.

- [ ] **Step 6 : Confirmer `COLOR_REP` et la sortie de `profcheck`**

Run (dans un dossier temporaire) : `<argyll>/bin/targen -d2 -f20 essai`, puis lire `COLOR_REP` dans `essai.ti1`. Lancer ensuite le test ignoré de la tâche 3.
Expected: `COLOR_REP "iRGB"`. Sinon, reprendre la valeur d'ArgyllCMS dans `ti3.rs` et dans son test. La ligne de `profcheck` doit correspondre à `lire_precision` ; sinon, corriger le motif et le test avec la ligne réelle. Noter les deux constats (« confirmé le … avec 3.5.0 ») dans `docs/references/icc-argyll-options.md`.

Run: `cargo test -p profilage --test argyll -- --ignored --nocapture`
Expected: PASS.

- [x] **Step 7 : Commit**

```bash
git commit -am "Profilage : ArgyllCMS derrière le trait MoteurProfil, version figée 3.5.0"
```

---

### Tâche 4 : Calculer et ranger le profil dans l'application

**Files:**
- Create: `app/src/profiler.rs`
- Modify: `app/Cargo.toml` (dépendance `profilage`), `app/src/lib.rs` (déclaration du module, état `Moteur`, enregistrement des commandes), `app/src/parc.rs` (`DetailReglage::sources`), `app/src/textes.rs`
- Test: `app/tests/profiler.rs`

**Interfaces:**
- Consumes : `preparer_plages`, `conditions_utilisables`, `ecrire_ti3` (tâche 2) ; `MoteurProfil`, `Recette`, `ArgyllCms`, `chercher_argyll`, `MoteurSimule` (tâche 3) ; `nom_profil_propose`, `NouveauProfil`, `enregistrer_profil` (crate `bibliotheque`).
- Produces :
  ```rust
  #[derive(Serialize)]
  pub struct SourceProfil { pub mesure: IdMesure, pub fichier: String, pub conditions: Vec<ConditionMesure>, pub raison: Option<&'static str> }
  // raison : clé de texte quand la mesure ne peut pas servir (RefusPlages::Vide, RvbInconnus, AucuneCondition)
  #[derive(Serialize)]
  pub struct ProvenanceProfil { pub mesure: IdMesure, pub fichier: String, pub condition: ConditionMesure, pub condition_supposee: bool,
                                pub recette: Recette, pub version: String, pub precision_moyenne: f64, pub precision_max: f64 }
  pub struct DetailReglage { /* … */
      pub sources: Vec<SourceProfil>,            // une par mesure importée de la résolution
      pub provenance: Option<ProvenanceProfil>,  // lue dans les paramètres du profil actif ; None s'il n'a pas été calculé
  }
  #[derive(Serialize)]
  pub struct ErreurProfil { pub cle: &'static str, pub cause: Option<String> }
  // cause : la cause d'ErreurMoteur::Echec (dernière ligne d'erreur), la version trouvée pour VersionNonValidee, None ailleurs.
  // La page affiche le texte de `cle`, puis `profil.erreur.cause` (« Message d'ArgyllCMS : {cause} ») si `cause` est là.
  impl BibliothequeOuverte {
      pub fn commandes_profil(&self, condition: IdCondition, mesure: IdMesure, mesure_condition: ConditionMesure,
                              recette: Recette, moteur: &dyn MoteurProfil) -> Result<Vec<String>, ErreurProfil>;
      pub fn calculer_profil(&self, condition: IdCondition, mesure: IdMesure, mesure_condition: ConditionMesure,
                             recette: Recette, moteur: &dyn MoteurProfil, maintenant: SystemTime) -> Result<ResultatProfil, ErreurProfil>;
  }
  #[derive(Serialize)]
  pub struct ResultatProfil { pub profil: IdProfil, pub precision_moyenne: f64, pub precision_max: f64 }
  // Commandes (async, hors du fil de la fenêtre) :
  // profiler_commandes(condition: i64, mesure: i64, mesure_condition: ConditionMesure, recette: Recette) -> Result<Vec<String>, ErreurProfil>
  // profiler_calculer(condition: i64, mesure: i64, mesure_condition: ConditionMesure, recette: Recette) -> Result<ResultatProfil, ErreurProfil>
  ```
- Paramètres conservés (`NouveauProfil::parametres`), un JSON :
  `{"origine":"calcul","moteur":"ArgyllCMS","version":"3.5.0","recette":"standard","condition":"M1","condition_supposee":true,"rvb_supposes":true,"mesure":12,"fichier":"mesure00001_M1.txt","commandes":[…],"precision":{"moyenne_de00":…,"max_de00":…},"ti3":"…","journal":"…"}`.
  `mesures` = `[mesure]`. Nom : `nom_profil_propose(imprimante, papier, résolution, date du jour)`. `DetailReglage::origine` donne déjà `calcul` pour tout ce qui n'est pas un profil existant.
- État Tauri `Moteur(Option<ArgyllCms>)`, rempli au lancement par `chercher_argyll(dossier de l'exécutable)`. `None` → `ErreurProfil { cle: "profil.erreur.argyll_introuvable", cause: None }`.

- [x] **Step 1 : Tests en échec (`app/tests/profiler.rs`, bibliothèque temporaire, `MoteurSimule`)**

```rust
#[test] fn un_profil_calcule_devient_actif_et_non_verifie() {}
    // parc PRO-4100 / Rag / 720 dpi, import d'un CGATS fictif FD-S2w avec RVB (3 plages, M1),
    // calcul → profil actif, etat NonVerifie, origine "calcul", mesures == [id de la mesure]
#[test] fn les_parametres_gardent_commande_version_recette_et_precision() {}
#[test] fn le_nom_du_profil_est_celui_propose() {}               // nom_profil_propose(…, "2026-10-23")
#[test] fn un_echec_du_moteur_laisse_le_profil_actif_inchange() {}
    // profil existant rangé, MoteurSimule::echoue(Echec{Profil,..}) → Err("profil.erreur.calcul"), actif inchangé
#[test] fn argyll_introuvable_ne_range_rien() {}
#[test] fn une_mesure_sans_rvb_ne_peut_pas_servir_de_source() {}  // sources[0].raison == Some("profil.refus.rvb_inconnus")
#[test] fn seules_les_conditions_presentes_sont_proposees() {}    // export M0 seul → conditions == [M0]
#[test] fn une_mesure_d_une_autre_resolution_est_refusee() {}     // Err("profil.erreur.mesure_ailleurs")
#[test] fn les_commandes_se_montrent_sans_rien_calculer() {}      // commandes_profil n'appelle pas calculer
```

- [x] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test profiler`
Expected: échec de compilation.

- [x] **Step 3 : Implémenter `app/src/profiler.rs`**

`calculer_profil` lit la mesure importée et les noms sous le verrou (`self.avec`), relâche le verrou, lance `moteur.calculer`, puis reprend le verrou pour `enregistrer_profil`. Toute erreur devient une `ErreurProfil` : sa clé de texte (`RefusPlages::cle`, `ErreurMoteur::cle`, `profil.erreur.mesure_ailleurs`, ou la clé d'une erreur de la bibliothèque) et, pour un échec d'ArgyllCMS, sa cause, rendue à la page : la dernière ligne d'erreur de l'outil, ou la version trouvée. L'erreur complète est aussi écrite dans le journal du terminal (`eprintln!`), comme dans `parc.rs`. Les commandes Tauri sont `#[tauri::command(async)]`, comme `ouvrir_instrument` (`app/src/lib.rs:203`).

- [x] **Step 4 : Textes FR et EN dans `textes.rs`**

Clés : `profil.titre` « Calculer un profil », `profil.source` « Mesure », `profil.condition` « Condition de mesure », `profil.recette` « Qualité du calcul », `profil.recette.rapide` / `.standard` / `.approfondi` avec une phrase chacune (« pour un essai, en une minute environ », etc. ; durées à confirmer à la tâche 6), `profil.avance` « Avancé : commande équivalente », `profil.calculer` « Calculer le profil », `profil.en_cours` « Calcul du profil en cours. Cela peut prendre plusieurs minutes. », `profil.fait` « Profil calculé. Écart moyen {moyenne} ΔE00, maximum {max}. Il n'est pas encore vérifié. », `profil.supposee` « supposée », toutes les clés `profil.refus.*` et `profil.erreur.*` des tâches 2 à 4. Chaque erreur dit la cause, puis l'action ; `profil.erreur.argyll_introuvable` : « ArgyllCMS 3.5.0 est introuvable. Placez-le dans le dossier argyll à côté de l'application. » Retirer `parc.refaire.raison`.

- [x] **Step 5 : Vérifier et commiter**

Run: `cargo test -p app && cargo clippy --all-targets -- -D warnings`
Expected: PASS (les tests de typographie française de `textes.rs` compris).

```bash
git commit -am "Application : calculer un profil RVB depuis une mesure importée et le ranger"
```

---

### Tâche 5 : La section « Calculer un profil » dans la vue d'une résolution

**Files:**
- Modify: `app/interface/reglage.js`, `design-system/components.css` (seulement si une classe manque ; jetons existants uniquement)
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes : `DetailReglage::sources`, commandes `profiler_commandes` et `profiler_calculer` (tâche 4), `agir()` et `annoncer()` de `reglage.js`.

Comportement :
- Le bouton « Refaire le profil » devient « Calculer un profil » et ouvre une section repliable dans la vue de la résolution.
- La section contient : choix de la mesure (les mesures importées de la résolution, la plus récente d'abord), condition (M1 présélectionnée si elle est présente), recette (Standard présélectionnée), puis « Avancé » replié, qui montre les commandes en lecture seule (`profiler_commandes`, rechargées à chaque changement de choix).
- Le seul bouton principal est « Calculer le profil ».
- Une mesure qui ne peut pas servir reste dans la liste, inactive, avec sa raison juste à côté. Sans aucune mesure importée, la section dit : « Importez d'abord un export CGATS de FD-S2w dans cette résolution », avec le bouton d'import existant.
- Pendant le calcul, tous les boutons de la vue attendent et `profil.en_cours` s'affiche à côté (mécanisme `agir()` existant).
- Après le calcul, la vue se recharge : le nouveau profil est actif et l'avis `profil.fait` donne la précision. « Installer dans Windows » redevient le bouton principal.
- Une erreur s'affiche à côté du bouton, en texte et non en rouge plein : le rouge reste réservé au danger et au hors tolérance. Elle donne la cause, puis l'action.
- À droite, dans la provenance d'un profil calculé : mesure source (nom du fichier), condition (avec « supposée » s'il y a lieu), recette, version d'ArgyllCMS et précision. L'instrument est celui que déclare le fichier ; sinon « inconnu ».

- [x] **Step 1 : Tests en échec dans `app/tests/page.rs`**

```rust
#[test] fn la_vue_d_une_resolution_appelle_le_calcul_du_profil() {}  // reglage.js contient "profiler_calculer" et "profiler_commandes"
#[test] fn le_texte_de_l_ancien_bouton_prevu_a_disparu() {}           // aucune occurrence de "parc.refaire.raison"
#[test] fn toutes_les_cles_profil_de_la_page_existent_en_deux_langues() {}
```

- [x] **Step 2 : Implémenter dans `reglage.js`**

Le fichier fait 164 lignes. Si la section l'amène au-delà de 400 lignes, créer `app/interface/calcul-profil.js`, chargé par `index.html` avant `reglage.js`, sur le modèle d'`arbre.js`.

- [x] **Step 3 : Vérifier et commiter**

Run: `cargo test -p app --test page`
Expected: PASS.

```bash
git commit -am "Interface : calculer un profil depuis la vue d'une résolution"
```

---

### Tâche 6 : Installateur, documentation et essai d'écran

**Files:**
- Modify: `outils/preparer_installateur.py`, `CLAUDE.md` (commandes et crate `profilage`), `docs/adr/0002-argyllcms-inclus.md` (section « Livré »), `GLOSSARY.md` (Recette de calcul, Précision du profil), `docs/blog/<jour>.md`

- [ ] **Step 1 : Installateur**

Si `argyll/` existe à la racine, `preparer_installateur.py` vérifie la version (`argyll/bin/colprof` → `Version 3.5.0`) et ajoute à `app/tauri.installateur.json` les ressources `argyll/bin/colprof.exe`, `argyll/bin/profcheck.exe`, `argyll/ref/sRGB.icm` et le texte de licence d'ArgyllCMS, sous `argyll/`. Sans `argyll/`, il prévient en une ligne et continue. Le journal et l'ADR 0002 rappellent l'obligation de l'AGPL : chaque publication fournit les sources de la version incluse ou en donne le lien précis.

Run: `python outils/test_verifier_docs.py && python outils/verifier_docs.py`
Expected: OK.

- [ ] **Step 2 : Essai d'écran avec le mainteneur (geste humain)**

1. Compiler, puis lancer avec `cargo run -p app`.
2. Dans « Canon PRO-4100 / <papier> / <résolution> », importer un export FD-S2w réel (M1), puis calculer avec la recette Standard.
3. Noter la durée de chaque recette, la précision obtenue et le nom du profil.
4. Installer le profil dans Windows et l'ouvrir dans un logiciel de l'atelier.
5. Recommencer sans `argyll/` pour voir le message.
6. Reporter les durées dans les textes des recettes (tâche 4, step 4).

- [ ] **Step 3 : Journal, glossaire, ADR, commit**

Écrire le journal du jour en mots simples (ce qui marche, ce qui reste « Prévu »), puis :

```bash
git commit -am "Documentation : calcul de profil RVB, ArgyllCMS 3.5.0 dans l'installateur"
```

- [ ] **Step 4 : PR vers `dev`**

PR `ticket/40-profil-rvb-import` → `dev`, avec les « Choix de l'assistant » et les résultats de l'essai d'écran. #40 reste ouvert : mire produite par myiro-libre, moyenne (#35) et CMJN restent à faire.
