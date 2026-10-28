# Bibliothèque par imprimante, papier et résolution, avec profils — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** La crate `bibliotheque` range les conditions d'impression sous un arbre imprimante → papier → résolution et conserve les profils ICC de chaque condition (profil actif, historique, état de vérification, provenance).

**Architecture:** Ajout pur dans la crate `bibliotheque` (aucune modification de l'application ni de l'API existante) : deux nouveaux modules `parc.rs` et `profils.rs`, une organisation de base n° 5 (migration SQLite dans la transaction existante), deux nouveaux fichiers de tests. Une condition d'impression existante sans papier reste « non classée ».

**Tech Stack:** Rust 2021, rusqlite 0.37 (bundled), serde, sha2 (empreinte des profils), tempfile (tests).

**Spec:** `docs/specifications/2026-10-20-poste-unique-design.md` (Partie A, « Bibliothèque ») ; glossaire `GLOSSARY.md` (Condition d'impression).

## Feuille de route de l'étape 1 (un plan par sous-projet)

Ce plan est le **plan 1**. Les suivants seront écrits un par un, chacun livrant un logiciel qui fonctionne :

1. **Bibliothèque : parc et profils** (ce plan, crate seule).
2. **Nouveau cadre d'interface** : système graphique refait d'après le prototype validé (bleu nuit, bascule clair/sombre, cartes, accent bleu), cadre à trois zones, cinq tâches, bibliothèque en arbre branchée sur le plan 1, installation d'un profil dans Windows. S'appuie sur le registre des actions (tickets #54 à #58) s'il est livré, sinon le prévoit.
3. **Calcul d'un profil RVB à partir d'une mesure importée** (export CGATS de FD-S2w), par ArgyllCMS (#40, première partie) : [plan 3](2026-10-23-profil-rvb-import-fd-s2w.md), fusionné le 26 octobre 2026 (PR #72).
4. **Comparer des profils** : volume, saturation par teinte, coupe, chemins du cube RVB, couleurs de base, écarts entre profils (#71) : [plan 5](2026-10-26-comparer-des-profils.md).
5. **Mires de profilage** : génération RVB et CMJN par ArgyllCMS, TIFF sans gestion couleur, QR code et texte lisible (#33) : plan 4, `2026-10-23-mires-de-profilage.md`.
6. **Mesure d'une feuille au FD-9** (#34), selon les paliers de sécurité ; lecture du QR à étudier ; guidage, reprise et moyenne (#35).
7. **Vérification du profil** et parcours en 4 étapes, enregistrement en un clic (#41).
8. **Séchage et reprise** : file « en attente de mesure », durée par papier, mise en avant à la réouverture.
9. **Reprise de l'existant** : profils épars, papiers du RIP des pigmentaires, fichiers MYIROtools.

Mise à jour du 23 octobre 2026 : le calcul de profil passe avant les mires, à la demande du mainteneur. Il part des exports FD-S2w faits chaque jour, sans attendre #33 ni #34.

Mise à jour du 26 octobre 2026 : la comparaison de profils (#71, plan 5) passe avant les mires (plan 4, écrit avant elle), à la demande du mainteneur. Elle sert tout de suite à juger les profils calculés par myiro-libre face à ceux d'un autre logiciel.

## Global Constraints

- Branche de travail `dev` dans `C:\Code\MYIRO-fork`, un agent à la fois (`AGENTS.md`) ; lancer `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev` avant d'écrire.
- La crate `bibliotheque` reste indépendante de Tauri, de Windows et des ponts (ADR 0004 et ticket #54) : aucune dépendance nouvelle sauf `sha2`.
- Identifiants, messages d'erreur, commentaires et documentation en français ; mots simples.
- Aucun `unwrap`, `expect` ni `panic!` dans le code hors tests (ticket #57).
- Fichiers source nouveaux sous 400 lignes ; `lib.rs` (déjà 1 020 lignes) ne reçoit que les déclarations de modules, la migration et les nouvelles variantes d'erreur.
- Une migration ne réécrit ni n'efface aucune donnée existante ; tout se fait dans la transaction de `migrer`.
- Données de test fictives : jamais de numéro de série, d'adresse réseau ou de nom de client réels.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p bibliotheque`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Bibliothèque existante du poste** (organisation 4 : 1 condition, 6 mesures, une couleur de référence possible) : après migration, tout se relit à l'identique et la condition apparaît dans `non_classees`. → test `une_base_d_organisation_4_garde_tout_et_reste_non_classee` (tâche 2).
2. **Noms réels avec accents, guillemets, apostrophes et caractères interdits par Windows** (« Hahnemühle Photo Rag 308 », « Canson « Baryta » », « Mat/Satin ») dans le nom de profil proposé : nom de fichier valide, lisible. → test `le_nom_propose_est_un_nom_de_fichier_windows_valide` (tâche 3).
3. **Fichier qui n'est pas un profil ICC** (PDF, ICC tronqué, espace Lab ou gris) : refus, rien n'est écrit, le profil actif ne change pas. → test `un_fichier_qui_n_est_pas_un_profil_rvb_ou_cmjn_est_refuse_sans_rien_changer` (tâche 3).
4. **Nouveau profil sur une condition qui en a déjà un** : un seul actif, l'ancien reste dans l'historique avec son fichier intact. → test `un_nouveau_profil_devient_actif_et_l_ancien_passe_a_l_historique` (tâche 3).
5. **Recherche sans accents ni majuscules** (« hahnemuhle », « LUSTER ») : trouve imprimante, papier ou résolution. → test `la_recherche_ignore_accents_et_majuscules` (tâche 2).

---

### Tâche 0 : Mettre `dev` à jour avec `origin/main`

`origin/main` contient 21 commits absents de `dev` (mesure ponctuelle, mesure automatique, export CGATS, annulation, couleur de référence ; la crate `bibliotheque` y est en organisation 4). Le plan s'écrit contre cet état.

**Files:** aucun fichier écrit à la main.

- [ ] **Step 1 : Vérifier le contexte et l'arbre de travail**

Run: `python outils/verifier_contexte.py --worktree C:/Code/MYIRO-fork --branch dev --reference dev` puis `git status --short`
Expected: contexte conforme ; noter les changements préexistants (ils ne relèvent pas de ce plan et restent intacts).

- [ ] **Step 2 : Fusionner**

Run: `git fetch origin && git merge --no-ff origin/main -m "Intègre main dans dev (mesure ponctuelle, CGATS, annulation, couleur de référence)"`
Expected: fusion sans conflit. En cas de conflit, s'arrêter et demander au mainteneur ; ne jamais utiliser `reset --hard` ni `push --force`.

- [ ] **Step 3 : Vérifier que tout passe**

Run: `cargo test` puis `cargo clippy --all-targets -- -D warnings`
Expected: PASS, aucun avertissement. `grep -n "VERSION_BASE: i32" crates/bibliotheque/src/lib.rs` affiche `4`.

---

### Tâche 1 : Organisation 5 de la base (imprimantes, papiers, résolution, profils)

**Files:**
- Modify: `crates/bibliotheque/src/lib.rs` (constante `VERSION_BASE`, `migrer`, nouvelle constante `ORGANISATION_5`, variantes d'erreur)
- Modify: `crates/bibliotheque/Cargo.toml` (dépendance `sha2 = "0.10"`)
- Test: `crates/bibliotheque/tests/parc.rs` (créé ici, complété en tâche 2)

**Interfaces:**
- Produces : tables `imprimantes(id, nom UNIQUE)`, `papiers(id, imprimante → imprimantes, nom, UNIQUE(imprimante, nom))`, colonnes `conditions.papier` (NULL = non classée) et `conditions.resolution` (NULL si non classée), index unique partiel `(papier, resolution) WHERE papier IS NOT NULL`, table `profils(id, condition → conditions, nom, horodatage, espace CHECK IN ('rvb','cmjn'), etat CHECK IN ('non_verifie','accepte','hors_tolerance','incomplet') DEFAULT 'non_verifie', actif INTEGER CHECK IN (0,1), empreinte TEXT, parametres TEXT, mesures TEXT, icc BLOB NOT NULL)` avec index unique partiel `(condition) WHERE actif = 1`.
- Produces : variantes `ErreurBibliotheque::ImprimanteInconnue(IdImprimante)`, `PapierInconnu(IdPapier)`, `ProfilInconnu(IdProfil)`, `ProfilIllisible(String)` et leurs messages `Display` en français (« imprimante n° … inconnue », « papier n° … inconnu », « profil n° … inconnu », « ce fichier n'est pas un profil ICC RVB ou CMJN utilisable : … »). Les types `IdImprimante`, `IdPapier`, `IdProfil` sont déclarés en tâches 2 et 3 ; dans cette tâche, déclarer les trois newtypes `pub struct IdX(pub i64)` (mêmes dérivés que `IdCondition`) dans les modules `parc` et `profils` encore vides, réexportés par `lib.rs`.

- [ ] **Step 0 : Figer une base d'organisation 4 (avant tout changement de version)**

Déplacer les fonctions d'aide de `tests/bibliotheque.rs` (`plage`, `provenance`, mesure fictive) dans `tests/commun/mod.rs` et les importer depuis `tests/bibliotheque.rs` (`mod commun;`). Ajouter dans `tests/parc.rs` un test `#[ignore] fn produire_fixture_organisation_4()` qui ouvre une bibliothèque dans un `TempDir`, crée la condition « Essai », enregistre une mesure fictive, la désigne couleur de référence avec un seuil de 1,5, puis copie `bibliotheque.sqlite` vers `crates/bibliotheque/tests/donnees/organisation-4.sqlite`.
Run: `cargo test -p bibliotheque --test parc -- --ignored produire_fixture_organisation_4` (une seule fois, sur le code encore en organisation 4) puis vérifier `PRAGMA user_version` = 4 sur le fichier produit. Versionner la fixture ; ne plus relancer ce test ensuite (il porte un commentaire le disant).

- [ ] **Step 1 : Écrire le test qui échoue**

Dans `tests/parc.rs`, test `une_base_neuve_est_en_organisation_5` : ouvrir une bibliothèque dans un `TempDir`, relire `PRAGMA user_version` du fichier `bibliotheque.sqlite` avec une connexion rusqlite séparée (ajouter `rusqlite` en dev-dependency avec la même version) ; `assert_eq!(version, 5)` ; vérifier que `SELECT papier, resolution FROM conditions` se prépare sans erreur.

- [ ] **Step 2 : Vérifier qu'il échoue**

Run: `cargo test -p bibliotheque --test parc`
Expected: FAIL (`left: 4, right: 5`).

- [ ] **Step 3 : Implémenter**

`VERSION_BASE = 5`, branche `4 => transaction.execute_batch(ORGANISATION_5)?` dans `migrer`, `ORGANISATION_5` documentée comme les précédentes (« par-dessus l'organisation 4 ; rien n'est réécrit ni effacé »). Ajouter les variantes d'erreur.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p bibliotheque`
Expected: PASS (nouveaux et anciens tests).

- [ ] **Step 5 : Commit**

```bash
git add crates/bibliotheque Cargo.lock
git commit -m "Bibliothèque : organisation 5, imprimantes, papiers, résolutions et profils"
```

---

### Tâche 2 : Parc — imprimantes, papiers, résolutions et arbre

**Files:**
- Create: `crates/bibliotheque/src/parc.rs`
- Modify: `crates/bibliotheque/src/lib.rs` (`mod parc; pub use parc::*;` ; `replier` et `instant` passent en `pub(crate)`)
- Test: `crates/bibliotheque/tests/parc.rs`

**Interfaces:**
- Consumes : `Bibliotheque` (champ `base`), `ConditionImpression`, `IdCondition`, `replier`, `ErreurBibliotheque`.
- Produces (dans `parc.rs`, tout `Serialize`) :
  - `pub struct Imprimante { pub id: IdImprimante, pub nom: String }`
  - `pub struct Papier { pub id: IdPapier, pub imprimante: IdImprimante, pub nom: String }`
  - `pub struct Reglage { pub condition: ConditionImpression, pub resolution: String, pub profil: Option<ResumeProfil>, pub profil_ancien: bool }`
  - `pub struct BranchePapier { pub papier: Papier, pub reglages: Vec<Reglage> }`
  - `pub struct BrancheImprimante { pub imprimante: Imprimante, pub papiers: Vec<BranchePapier> }`
  - `pub struct Parc { pub imprimantes: Vec<BrancheImprimante>, pub non_classees: Vec<ConditionImpression> }`
  - `impl Bibliotheque` : `creer_imprimante(&self, nom: &str) -> Resultat<Imprimante>` ; `imprimantes(&self) -> Resultat<Vec<Imprimante>>` ; `creer_papier(&self, imprimante: IdImprimante, nom: &str) -> Resultat<Papier>` ; `creer_reglage(&self, papier: IdPapier, resolution: &str) -> Resultat<ConditionImpression>` ; `parc(&self, recherche: &str, maintenant: std::time::SystemTime) -> Resultat<Parc>`.
- Règles : noms débarrassés des espaces de bord, jamais vides (`NomVide`) ; imprimante unique (`NomDejaPris`) ; papier unique par imprimante (même nom permis sur deux imprimantes) ; résolution unique par papier (`NomDejaPris`). `creer_reglage` crée la condition d'impression nommée `"{imprimante} · {papier} · {résolution}"`. Tri alphabétique selon `replier` à chaque niveau. `recherche` vide : tout ; sinon une imprimante, un papier ou une résolution correspondant (sous-chaîne, après `replier`) garde sa branche et ses ancêtres. `profil` et `profil_ancien` restent `None`/`false` jusqu'à la tâche 3.

- [ ] **Step 1 : Écrire les tests qui échouent** (dans `tests/parc.rs`)

```rust
#[test]
fn l_arbre_range_imprimante_papier_resolution() {
    let (_d, b) = bibliotheque_temporaire();
    let canon = b.creer_imprimante(" Canon PRO-4100 ").unwrap();
    let luster = b.creer_papier(canon.id, "HMPhoto Luster 290").unwrap();
    b.creer_reglage(luster.id, "1200 ppp").unwrap();
    let c600 = b.creer_reglage(luster.id, "600 ppp").unwrap();
    assert_eq!(c600.nom, "Canon PRO-4100 · HMPhoto Luster 290 · 600 ppp");
    let parc = b.parc("", maintenant()).unwrap();
    assert_eq!(parc.imprimantes.len(), 1);
    let reglages: Vec<_> = parc.imprimantes[0].papiers[0].reglages.iter().map(|r| r.resolution.as_str()).collect();
    assert_eq!(reglages, ["1200 ppp", "600 ppp"]);
}

#[test]
fn les_doublons_sont_refuses_a_chaque_niveau() {
    // même imprimante deux fois → NomDejaPris ; même papier sur la même imprimante → NomDejaPris ;
    // même papier sur une autre imprimante → Ok ; même résolution deux fois → NomDejaPris ;
    // nom "  " → NomVide ; papier sur IdImprimante(999) → ImprimanteInconnue(IdImprimante(999))
}

#[test]
fn la_recherche_ignore_accents_et_majuscules() {
    // imprimantes « Canon PRO-4100 » (papier « Hahnemühle Photo Rag 308 ») et « Epson SC-P5300 » (papier « Luster »)
    // parc("hahnemuhle") → seule Canon, seul ce papier ; parc("LUSTER") → seule Epson ; parc("zzz") → aucune
}

#[test]
fn une_base_d_organisation_4_garde_tout_et_reste_non_classee() {
    // copier tests/donnees/organisation-4.sqlite (tâche 1, Step 0) dans un TempDir, puis ouvrir :
    // conditions() == [« Essai »] ; la mesure se relit égale à la mesure fictive d'origine ;
    // reference(id) == Some(seuil 1,5) ; parc("").non_classees == [« Essai »] ; parc("").imprimantes vide
}
```

- [ ] **Step 2 : Vérifier qu'ils échouent**

Run: `cargo test -p bibliotheque --test parc`
Expected: FAIL (méthodes absentes).

- [ ] **Step 3 : Implémenter `parc.rs`** selon les Interfaces. Lecture de l'arbre en une requête jointe `imprimantes ⟕ papiers ⟕ conditions`, filtrage et tri en Rust avec `replier`.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p bibliotheque`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add crates/bibliotheque
git commit -m "Bibliothèque : parc des imprimantes, papiers et résolutions, recherche sans accents"
```

---

### Tâche 3 : Profils — enregistrement, actif, historique, vérification, nom proposé

**Files:**
- Create: `crates/bibliotheque/src/profils.rs`
- Modify: `crates/bibliotheque/src/parc.rs` (remplir `profil` et `profil_ancien`)
- Test: `crates/bibliotheque/tests/profils.rs`

**Interfaces:**
- Consumes : `Bibliotheque`, `IdCondition`, `IdMesure`, `instant`, `ErreurBibliotheque`.
- Produces (dans `profils.rs`, tout `Serialize`) :
  - `pub enum EspaceProfil { Rvb, Cmjn }` ; `pub enum EtatVerification { NonVerifie, Accepte, HorsTolerance, Incomplet }` (valeurs en base : `rvb`, `cmjn`, `non_verifie`, `accepte`, `hors_tolerance`, `incomplet`).
  - `pub struct NouveauProfil<'a> { pub condition: IdCondition, pub nom: &'a str, pub icc: &'a [u8], pub horodatage: &'a str, pub parametres: &'a str, pub mesures: &'a [IdMesure] }` — `horodatage` en RFC 3339 UTC ; `parametres` : texte libre (commande et réglages ArgyllCMS) conservé tel quel ; `mesures` : mesures sources, stockées en JSON.
  - `pub struct ResumeProfil { pub id: IdProfil, pub nom: String, pub horodatage: String, pub espace: EspaceProfil, pub etat: EtatVerification, pub empreinte: String, pub actif: bool }`
  - `impl Bibliotheque` : `enregistrer_profil(&self, p: &NouveauProfil) -> Resultat<IdProfil>` ; `profil_actif(&self, c: IdCondition) -> Resultat<Option<ResumeProfil>>` ; `historique_profils(&self, c: IdCondition) -> Resultat<Vec<ResumeProfil>>` (plus récent d'abord, actif compris) ; `fichier_profil(&self, id: IdProfil) -> Resultat<Vec<u8>>` ; `marquer_verification(&self, id: IdProfil, etat: EtatVerification) -> Resultat<()>`.
  - Fonctions libres : `pub fn espace_icc(icc: &[u8]) -> Result<EspaceProfil, String>` ; `pub fn nom_profil_propose(imprimante: &str, papier: &str, resolution: &str, date: &str) -> String` ; `pub fn profil_ancien(horodatage: &str, maintenant: std::time::SystemTime) -> bool`.
- Règles :
  - `espace_icc` : au moins 132 octets ; taille annoncée (u32 gros-boutiste à l'octet 0) égale à la longueur ; signature `acsp` aux octets 36..40 ; espace de données (octets 16..20) `RGB ` → `Rvb`, `CMYK` → `Cmjn`, tout autre → erreur. Le message d'erreur dit ce qui manque (« taille annoncée 2000, fichier de 132 octets », « signature acsp absente », « espace de données Lab non pris en charge »).
  - `enregistrer_profil` : condition existante (`ConditionInconnue`), nom non vide (`NomVide`), `espace_icc` valide (`ProfilIllisible`) ; empreinte = SHA-256 hexadécimal minuscule du fichier ; dans **une** transaction : l'actif de la condition passe à `actif = 0`, le nouveau est inséré `actif = 1`, état `non_verifie`.
  - `nom_profil_propose` : chaque partie débarrassée des caractères `< > : " / \ | ? *` et des guillemets « » ainsi que des caractères de contrôle, espaces internes remplacés par `-`, tirets répétés réduits à un, accents conservés ; parties jointes par `_`, suffixe `.icc`. Exemple fixé : `("Canon PRO-4100", "HMPhoto Luster 290", "1200 ppp", "2026-10-20")` → `"Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-20.icc"`.
  - `profil_ancien` : vrai si l'horodatage précède `maintenant` de plus de 365 jours (seuil « plus d'un an » de la spec).
  - Dans `parc` : `profil` = `profil_actif` de la condition ; `profil_ancien` = `profil_ancien(horodatage, maintenant)`.

- [ ] **Step 1 : Écrire les tests qui échouent** (dans `tests/profils.rs`, avec une aide `fn icc(espace: &[u8; 4]) -> Vec<u8>` qui fabrique 132 octets : taille 132 à l'octet 0, `espace` à 16, `acsp` à 36, nombre d'étiquettes 0 à 128)

```rust
#[test]
fn un_nouveau_profil_devient_actif_et_l_ancien_passe_a_l_historique() {
    // condition via creer_imprimante/creer_papier/creer_reglage
    // p1 = enregistrer_profil(nom "v1", icc(b"RGB "), "2026-03-03T10:00:00Z") ; p2 idem "v2", "2026-10-20T09:00:00Z"
    // profil_actif == Some(p2, actif true, espace Rvb, etat NonVerifie)
    // historique_profils : [p2, p1], p1.actif == false
    // fichier_profil(p1) == icc(b"RGB ") octet pour octet
    // empreinte de p1 : 64 caractères hexadécimaux minuscules
}

#[test]
fn un_fichier_qui_n_est_pas_un_profil_rvb_ou_cmjn_est_refuse_sans_rien_changer() {
    // après un p1 valide : b"%PDF-1.7..." ; icc tronqué (taille annoncée 2000) ; icc(b"Lab ") ; icc(b"GRAY")
    // chaque appel → Err(ProfilIllisible(_)) ; profil_actif reste p1 ; historique_profils.len() == 1
}

#[test]
fn le_cmjn_est_reconnu() { /* icc(b"CMYK") → espace Cmjn */ }

#[test]
fn la_verification_change_l_etat() { /* marquer_verification(p, Accepte) → profil_actif.etat == Accepte ; IdProfil(999) → ProfilInconnu */ }

#[test]
fn le_nom_propose_est_un_nom_de_fichier_windows_valide() {
    assert_eq!(nom_profil_propose("Canon PRO-4100", "HMPhoto Luster 290", "1200 ppp", "2026-10-20"),
               "Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-20.icc");
    assert_eq!(nom_profil_propose("Epson SC-P5300", "Canson « Baryta » Mat/Satin", "720 ppp", "2026-10-20"),
               "Epson-SC-P5300_Canson-Baryta-MatSatin_720-ppp_2026-10-20.icc");
    let n = nom_profil_propose("Canon iPF5100", "Hahnemühle Photo Rag 308", "600 ppp", "2026-10-20");
    assert!(n.contains("Hahnemühle"));
    assert!(!n.chars().any(|c| "<>:\"/\\|?*".contains(c) || c.is_control()));
}

#[test]
fn un_profil_de_plus_d_un_an_est_ancien() {
    // maintenant = 2026-10-20T00:00:00Z ; "2025-10-19T23:59:59Z" → true ; "2025-10-21T00:00:00Z" → false
    // et dans parc(): profil_ancien suit cette règle pour le profil actif
}
```

- [ ] **Step 2 : Vérifier qu'ils échouent**

Run: `cargo test -p bibliotheque --test profils`
Expected: FAIL (module absent).

- [ ] **Step 3 : Implémenter `profils.rs`** puis brancher `parc`.

- [ ] **Step 4 : Vérifier**

Run: `cargo test -p bibliotheque && cargo clippy --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS, aucun avertissement.

- [ ] **Step 5 : Commit**

```bash
git add crates/bibliotheque
git commit -m "Bibliothèque : profils ICC par condition, actif et historique, nom proposé"
```

---

### Tâche 4 : Documentation (glossaire, ADR, format, journal)

**Files:**
- Modify: `GLOSSARY.md` (entrées **Imprimante**, **Papier**, **Résolution d'impression**, **Profil actif** ; « Condition d'impression » précisée : feuille de l'arbre imprimante → papier → résolution ; mention de « non classée »)
- Create: `docs/adr/0007-poste-unique.md` (« Une seule application pour la couleur et l'impression » : remplace l'ADR 0004 sur le RIP séparé ; les crates du cœur restent indépendantes de Tauri, de Windows et des ponts ; lien vers la spec du 20 octobre)
- Modify: `docs/adr/0004-crates-partagees-rip.md` (ligne d'en-tête : « Remplacé par l'ADR 0007 pour le RIP séparé ; le contrat de `colorimetrie` reste valable »)
- Modify: `docs/blog/2026-10-20.md` (section « Bibliothèque : imprimantes, papiers, profils »)

- [ ] **Step 1 : Rédiger** en mots simples, sans nommer les logiciels concurrents.

- [ ] **Step 2 : Vérifier**

Run: `python outils/test_verifier_docs.py && python outils/verifier_docs.py`
Expected: `Vérifications documentaires : OK`.

- [ ] **Step 3 : Commit**

```bash
git add .gitignore GLOSSARY.md docs/adr docs/blog/2026-10-20.md docs/specifications/2026-10-20-poste-unique-design.md docs/superpowers/plans
git commit -m "Poste unique : ADR 0007, glossaire du parc et des profils, journal"
```
