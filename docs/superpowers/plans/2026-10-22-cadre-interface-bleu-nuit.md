# Nouveau cadre d'interface bleu nuit et bibliothèque en arbre — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'application prend le système graphique bleu nuit du prototype validé, un cadre à trois zones avec cinq tâches, et une bibliothèque en arbre imprimante → papier → résolution où l'on range un profil ICC existant, l'exporte et l'installe dans Windows.

**Architecture:** Le système graphique est refait dans `design-system/` (source unique, recopiée par `app/build.rs`). La page garde ses crochets `data-*` et ses scripts actuels (instrument, étalonnage, mesure, import et export CGATS) : on change le cadre et l'habillage, pas leur comportement. Le parc (plan 1, crate `bibliotheque`) est branché par un nouveau module Rust `app/src/parc.rs` et un nouveau script `app/interface/parc.js`. L'installation dans Windows passe par un trait `InstallationProfils` à deux adapters, sur le modèle de `PareFeu` (ADR 0005).

**Tech Stack:** Rust 2021, Tauri 2, `tauri-plugin-dialog`, HTML, CSS et JavaScript simples sans outil de construction (ADR 0003), API Windows `mscms` (`InstallColorProfileW`, `GetColorDirectoryW`) par lien direct, sans nouvelle dépendance.

**Spec:** `docs/specifications/2026-10-20-poste-unique-design.md` (« Inspiration d'interface », « Bibliothèque ») ; prototype de référence : `.superpowers/brainstorm/1085-1792485206/content/proto-poste-unique.html` (local, hors dépôt : le constructeur le demande au capitaine s'il n'est pas dans son worktree) ; plan 1 : `docs/superpowers/plans/2026-10-20-bibliotheque-parc-profils.md`.

## Global Constraints

- Branche de travail : celle du ticket, partie de `dev` ; un agent à la fois dans le dossier principal (`AGENTS.md`).
- `design-system/` reste la source unique : aucune couleur, aucun corps, aucun espacement hors de `tokens.css` ; pas de `!important` ; pas de style en ligne sauf une valeur passée par propriété personnalisée.
- Jetons de couleur repris du prototype, à l'identique. Sombre (par défaut) : fond `#141a26`, barre `#1b2333`, panneau `#1e2738`, carte `#222c3f`, filet `#2e3a50`, encre `#e8ecf3`, atténué `#8d99ad`, accent `#3d8bff`, conforme `#2fbf71`, limite `#e8a23a`, hors tolérance `#ff5d64`, sélection `#173b6b`. Clair : `#eef1f6`, `#ffffff`, `#ffffff`, `#f6f8fb`, `#d6dce6`, `#141a26`, `#5d6879`, `#1f6fe5`, `#13864b`, `#a8650a`, `#cc1f29`, `#dbe8fb`.
- Cadre : barre du haut de 64 px ; zones gauche 270 px, centre souple, droite 300 px ; seules les zones défilent, jamais la page ni en largeur.
- Aucune ombre, aucun dégradé, aucun emoji ; angles droits, sauf la pastille de l'instrument (cercle).
- Rouge réservé au danger et au hors tolérance ; verdict toujours couleur + icône + mot ; un seul bouton principal par écran ; tout bouton inactif dit pourquoi, juste à côté.
- Aucun texte en dur dans la page : tout passe par `app/src/textes.rs`, en français et en anglais, typographie française vérifiée par les tests existants.
- Ne jamais écrire le nom du logiciel concurrent : « un logiciel concurrent ». Ne pas citer `docs/sources/`. Le prototype reste hors dépôt ; ne pas le copier tel quel (on en reprend les valeurs et la disposition).
- Icônes des tâches : nos propres tracés SVG en ligne (ceux du prototype conviennent), aucune icône reprise d'un autre logiciel.
- Aucun `unwrap`, `expect` ni `panic!` hors tests (ticket #57) ; fichiers source nouveaux sous 400 lignes ; `colonne.rs` (1 013 lignes) et `lib.rs` ne reçoivent que l'enregistrement des nouvelles commandes.
- Les tests n'installent jamais de profil dans le vrai Windows : ils passent par `InstallationSimulee`.
- Données de test fictives : jamais de numéro de série, d'adresse réseau ou de nom de client réels.
- Contrôles avant chaque commit : `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p app -p bibliotheque`, `python outils/verifier_docs.py`.
- Messages de commit en français, terminés par `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Choix de l'assistant (à signaler dans la PR)

- L'application s'ouvre sur **Bibliothèque**, comme le prototype validé (aujourd'hui : Mesurer).
- Profiler, Contrôler et Imprimer sont présents dans la barre et ouvrent une carte « Prévu » qui dit ce que la tâche fera et dans quelle étape ; leur bouton principal est inactif avec sa raison.
- Ranger un profil ICC existant dans une résolution est livré ici : c'est le seul moyen de remplir la bibliothèque avant le calcul de profil (plan 5), et cela sert tout de suite à rassembler les profils de l'atelier.
- Les conditions d'avant le parc restent visibles dans un groupe « Non classées » en bas de l'arbre, avec leurs mesures, sans action de classement (elle viendra avec la reprise de l'existant, plan 7).
- Le registre des actions (#58, #61) n'est pas livré : les nouvelles commandes suivent le modèle actuel (`#[tauri::command]` enregistrées dans `lib.rs`), à reprendre par #58.

## Review Focus

1. **Bibliothèque vide au premier lancement** (aucune imprimante) : la zone gauche dit quoi faire et propose « Ajouter une imprimante » ; le centre ne montre pas de carte vide sans explication. → test `un_parc_vide_propose_d_ajouter_une_imprimante` (tâche 3) et critère d'écran (tâche 5).
2. **Profil du même nom déjà présent dans le dossier des couleurs de Windows, avec un contenu différent** : l'installation refuse de l'écraser et le dit ; avec le même contenu, elle réussit sans rien changer. → tests `un_profil_different_du_meme_nom_n_est_pas_ecrase` et `un_profil_deja_installe_a_l_identique_est_accepte` (tâche 4).
3. **Fichier choisi qui n'est pas un profil RVB ou CMJN** (PDF, profil Lab ou gris) : refus avec la raison en clair, profil actif inchangé. → test `ranger_un_fichier_qui_n_est_pas_un_profil_donne_une_cle_de_texte` (tâche 3).
4. **Noms longs, accentués, avec guillemets** (« Hahnemühle Photo Rag 308 », « Canson « Baryta » ») dans l'arbre et le fil d'Ariane : rien ne déborde en largeur, le nom entier se lit au survol. → test `un_nom_long_de_l_arbre_se_lit_au_survol` (tâche 1).
5. **Annulation de la fenêtre de choix de fichier** (ranger ou exporter un profil) : rien ne change, aucun message d'erreur. → test `annuler_le_choix_du_fichier_ne_change_rien` (tâche 3).

---

### Tâche 1 : Système graphique bleu nuit

**Files:**
- Modify: `design-system/tokens.css`, `design-system/components.css`, `design-system/README.md`, `design-system/ui-kit.html`, `design-system/brand-book.html`, `design-system/index.html`
- Test: `app/tests/page.rs`

**Interfaces:**
- Produces (classes CSS utilisées par les tâches 2 et 5) : `.app` (grille barre / zones), `.barre-haut`, `.tache` (+ `aria-current="page"`), `.pastille` avec modificateurs `--non-detecte`, `--connecte`, `--etalonnage-requis`, `--etalonne`, `--en-mesure`, `.zones`, `.zone-gauche`, `.zone-centre`, `.zone-droite`, `.ariane`, `.carte`, `.carte--verdict`, `.verdict__mot`, `.verdict--conforme|--limite|--hors`, `.criteres`, `.arbre`, `.noeud` (+ `.noeud--imprimante|--papier|--resolution`, `aria-selected`), `.age--recent|--ancien|--aucun`, `.etapes`, `.etape` (+ `--faite`, `aria-current="step"`), `.puces`, `.section-repliable`, `.cle-valeur`, `.btn`, `.btn--primary`, `.why`. Les classes existantes encore appelées par les scripts (`.notice`, `.notice--warn`, `.seg`, `.replie`, `.prose`, `.field`, `.input`, `.actions`, `.steps`, `.schema*`, `.empty*`, `.section*`, `.label`, `.num`, `table`) sont gardées et rhabillées aux nouveaux jetons.
- Jetons (noms) : `--fond`, `--barre`, `--panneau`, `--carte`, `--filet`, `--encre`, `--attenue`, `--accent`, `--conforme`, `--limite`, `--hors`, `--selection`, plus les jetons d'espacement et de corps déjà présents.

- [ ] **Step 1 : Adapter les tests de `app/tests/page.rs` au nouveau cadre**

Garder l'intention de chaque test existant (refus du 9 octobre « illisible ») et changer seulement les sélecteurs :
- `la_colonne_de_la_bibliotheque_est_assez_large` devient `les_trois_zones_ont_leurs_largeurs` : la règle `.zones` contient `grid-template-columns:270px` et se termine par `300px`.
- `l_application_occupe_toute_la_largeur_de_la_fenetre`, `aucune_zone_ne_defile_en_largeur`, `toutes_les_barres_de_defilement_sont_fines` : mêmes assertions sur `.app`, `.zone-gauche`, `.zone-centre`, `.zone-droite`.
- `une_entree_de_la_bibliotheque_ne_coupe_pas_les_mots` porte sur `.noeud`.

Ajouter :

```rust
#[test]
fn les_jetons_sombres_sont_ceux_du_prototype() {
    let jetons = lire("../design-system/tokens.css");
    for valeur in ["#141a26", "#1b2333", "#1e2738", "#222c3f", "#2e3a50", "#e8ecf3",
                   "#8d99ad", "#3d8bff", "#2fbf71", "#e8a23a", "#ff5d64", "#173b6b"] {
        assert!(jetons.contains(valeur), "{valeur}");
    }
}

#[test]
fn aucune_couleur_hors_des_jetons() {
    // Aucune valeur `#xxx`, `#xxxxxx`, `rgb(` ni `hsl(` dans components.css.
}

#[test]
fn aucune_ombre_ni_degrade_ni_arrondi_hors_pastille() {
    // components.css : pas de `box-shadow`, `gradient(` ; `border-radius`
    // seulement dans les règles de `.pastille`.
}

#[test]
fn un_nom_long_de_l_arbre_se_lit_au_survol() {
    // `.noeud__nom` : `overflow:hidden`, `text-overflow:ellipsis`, `white-space:nowrap`.
}
```

- [ ] **Step 2 : Lancer les tests, constater l'échec**

Run: `cargo test -p app --test page`
Expected: FAIL (jetons et classes absents).

- [ ] **Step 3 : Réécrire `tokens.css` et `components.css`**

Thème sombre par défaut sur `:root` et `:root[data-theme="dark"]`, thème clair sur `:root[data-theme="light"]` (le sélecteur `data-theme` est déjà posé par `app.js`). Disposition et proportions reprises du prototype (barre, tâches à icône au-dessus du libellé, pastille, arbre à trois niveaux avec retraits 0 / 22 / 40 px, cartes, grille de quatre critères, étapes, puces). Mettre à jour les règles de `README.md` (thème bleu nuit, cartes, accent bleu ; la grille suisse et le cartouche d'architecte sont retirés) et les pages `ui-kit.html`, `brand-book.html` (contrastes recalculés), `index.html` du système graphique ; supprimer `grid.html` et son lien si la grille de 12 colonnes n'est plus utilisée.

- [ ] **Step 4 : Lancer les tests**

Run: `cargo test -p app --test page`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add design-system app/tests/page.rs
git commit -m "Système graphique : thème bleu nuit du prototype, cartes, accent bleu"
```

### Tâche 2 : Cadre à trois zones et cinq tâches

**Files:**
- Modify: `app/interface/index.html`, `app/interface/app.js`, `app/src/textes.rs`
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes : classes de la tâche 1 ; états du module instrument déjà rendus à la page (`non_detecte`, `detecte`, `connecte`, `etalonnage_requis`, `etalonne`) et l'état « mesure en cours » de `mesurer.js`.
- Produces : `data-tache` = `bibliotheque | profiler | mesurer | controler | imprimer`, dans cet ordre ; zones `[data-zone="gauche"]`, `[data-zone="centre"]`, `[data-zone="droite"]` ; vues `section[data-vue="<tache>"]` ; détails `[data-details-tache="<tache>"]` ; fil d'Ariane `[data-ariane]`.

- [ ] **Step 1 : Écrire les tests**

```rust
#[test]
fn la_barre_a_cinq_taches_dans_l_ordre_avec_une_icone() {
    // index.html : les `data-tache` lus dans l'ordre valent
    // ["bibliotheque", "profiler", "mesurer", "controler", "imprimer"] ;
    // chaque bouton contient un `<svg` et un libellé `data-t="tache.<nom>"`.
}

#[test]
fn chaque_bouton_principal_inactif_dit_pourquoi() {
    // Tout `<button class="btn btn--primary" … disabled` est suivi, dans le même
    // parent, d'un `<p class="why"` portant un `data-t`.
}

#[test]
fn la_pastille_connait_les_cinq_etats() {
    // components.css a une règle pour chacun des cinq modificateurs `.pastille--*`.
}
```

Dans `textes.rs`, ajouter `tache.imprimer` (« Imprimer » / « Print ») et les textes des cartes « Prévu » de Profiler, Contrôler et Imprimer ; le test existant des clés vérifie les deux langues.

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app`
Expected: FAIL sur les trois nouveaux tests.

- [ ] **Step 3 : Restructurer `index.html` et `app.js`**

Barre : marque, cinq tâches à icône, espace, bouton instrument devenu pastille + texte (le clic ouvre toujours la liste des instruments, ticket #49), bascules thème et langue. Zones : gauche = bibliothèque (vide en attendant la tâche 5, sauf le groupe existant), centre = vues, droite = détails. Les sections de l'instrument, de l'étalonnage et de Mesurer gardent tous leurs crochets `data-*`. Tâche par défaut : `bibliotheque`. Thème par défaut quand rien n'est retenu : `dark`. La pastille prend le modificateur qui correspond à l'état reçu.

- [ ] **Step 4 : Lancer les tests**

Run: `cargo test -p app`
Expected: PASS, y compris les tests existants de Mesurer, de l'instrument et de l'annulation.

- [ ] **Step 5 : Commit**

```bash
git add app/interface app/src/textes.rs app/tests/page.rs
git commit -m "Interface : cadre à trois zones, cinq tâches, pastille de l'instrument"
```

### Tâche 3 : Commandes du parc et des profils

**Files:**
- Create: `app/src/parc.rs`
- Modify: `app/src/lib.rs` (déclaration du module et enregistrement des commandes seulement), `app/src/colonne.rs` (accès en lecture à la bibliothèque ouverte si besoin, rien d'autre), `app/src/textes.rs`
- Test: `app/tests/parc.rs`

**Interfaces:**
- Consumes (crate `bibliotheque`, plan 1) : `creer_imprimante`, `creer_papier`, `creer_reglage`, `parc(recherche, maintenant) -> Parc`, `enregistrer_profil(&NouveauProfil) -> IdProfil`, `profil_actif`, `historique_profils`, `fichier_profil`, `espace_icc`, `nom_profil_propose`, et `arborescence("") -> Vec<Branche>` (existante) pour les mesures et mesures importées d'une condition.
- Produces, méthodes de `BibliothequeOuverte` testables sans Tauri, erreurs rendues en clés du catalogue :
  - `parc(&self, recherche: &str) -> Result<Parc, String>`
  - `ajouter_imprimante(&self, nom: &str) -> Result<Imprimante, String>`
  - `ajouter_papier(&self, imprimante: IdImprimante, nom: &str, resolution: &str) -> Result<ConditionImpression, String>` (crée le papier et sa première résolution en un geste)
  - `ajouter_resolution(&self, papier: IdPapier, resolution: &str) -> Result<ConditionImpression, String>`
  - `detail_reglage(&self, condition: IdCondition) -> Result<DetailReglage, String>` avec `DetailReglage { condition: ConditionImpression, imprimante: String, papier: String, resolution: String, actif: Option<ResumeProfil>, historique: Vec<ResumeProfil>, ancien: bool, mesures: Vec<ResumeMesure>, importees: Vec<ResumeImportee> }` (types existants de la crate, tirés de la `Branche` de la condition)
  - `ranger_profil(&self, condition: IdCondition, fichier: &Path, aujourd_hui: &str) -> Result<IdProfil, String>` : lit le fichier, nom proposé par `nom_profil_propose`, paramètres `{"origine":"profil existant"}`, aucune mesure.
  - `exporter_profil(&self, profil: IdProfil, fichier: &Path) -> Result<(), String>`
  - commandes Tauri `parc`, `parc_ajouter_imprimante`, `parc_ajouter_papier`, `parc_ajouter_resolution`, `parc_detail_reglage`, `parc_ranger_profil` (fenêtre de choix, filtre `.icc` / `.icm`), `parc_exporter_profil` (fenêtre d'enregistrement, nom par défaut = nom du profil) ; une fenêtre annulée renvoie `Ok(None)`.

- [ ] **Step 1 : Écrire les tests dans `app/tests/parc.rs`**

```rust
#[test]
fn un_parc_vide_propose_d_ajouter_une_imprimante() {
    // Bibliothèque neuve : parc("") a 0 imprimante et 0 non classée.
}

#[test]
fn ajouter_un_papier_cree_sa_premiere_resolution() {
    // ajouter_imprimante("Canon PRO-4100"), ajouter_papier(id, "HMPhoto Luster 290", "1200 ppp")
    // → parc("") : 1 imprimante, 1 papier, 1 réglage « 1200 ppp » sans profil.
}

#[test]
fn ranger_un_profil_existant_le_rend_actif_avec_le_nom_propose() {
    // Profil RVB fictif valide (en-tête ICC 'RGB ' et signature 'acsp'),
    // ranger_profil(..., "2026-10-22") → detail_reglage : actif.nom ==
    // "Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-22.icc", historique de 1.
}

#[test]
fn ranger_un_fichier_qui_n_est_pas_un_profil_donne_une_cle_de_texte() {
    // Un PDF fictif : Err(clé) où la clé existe dans le catalogue ;
    // detail_reglage : actif inchangé (None).
}

#[test]
fn exporter_un_profil_ecrit_le_fichier_a_l_identique() {}

#[test]
fn annuler_le_choix_du_fichier_ne_change_rien() {
    // La fonction appelée après une fenêtre annulée (aucun chemin) ne touche
    // pas la bibliothèque : parc identique avant et après.
}

#[test]
fn un_nom_de_papier_en_double_donne_une_cle_de_texte() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test parc`
Expected: FAIL (module `parc` absent).

- [ ] **Step 3 : Écrire `app/src/parc.rs`**

Une clé du catalogue par erreur de la bibliothèque utile ici (`ImprimanteInconnue`, `PapierInconnu`, `ProfilInconnu`, `ProfilIllisible`, nom vide, nom pris), ajoutée à `cle_erreur` et à `textes.rs`.

- [ ] **Step 4 : Lancer les tests**

Run: `cargo test -p app`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add app/src/parc.rs app/src/lib.rs app/src/colonne.rs app/src/textes.rs app/tests/parc.rs
git commit -m "Application : commandes du parc, ranger et exporter un profil existant"
```

### Tâche 4 : Installer un profil dans Windows

**Files:**
- Create: `app/src/installation.rs`
- Modify: `app/src/parc.rs`, `app/src/lib.rs` (enregistrement), `app/src/textes.rs`
- Test: `app/tests/installation.rs`

**Interfaces:**
- Produces :
  - `pub trait InstallationProfils { fn dossier(&self) -> Result<PathBuf, ErreurInstallation>; fn installer(&self, fichier: &Path) -> Result<(), ErreurInstallation>; }`
  - `pub enum ErreurInstallation { DejaPresentDifferent, Refusee, Systeme(String) }`
  - `pub fn installer_profil(systeme: &dyn InstallationProfils, nom: &str, icc: &[u8], temporaire: &Path) -> Result<EtatInstallation, ErreurInstallation>` avec `EtatInstallation { Installe, DejaInstalle }` : si `dossier()/nom` existe avec le même contenu → `DejaInstalle` sans appel ; avec un autre contenu → `DejaPresentDifferent` sans appel ; sinon écrit `temporaire/nom` puis `installer`.
  - `pub fn est_installe(systeme: &dyn InstallationProfils, nom: &str, icc: &[u8]) -> bool`
  - Adapters : `InstallationWindows` (`#[cfg(windows)]`, `#[link(name = "mscms")] extern "system"` pour `GetColorDirectoryW` et `InstallColorProfileW`, chemins en UTF-16 terminés par zéro ; accès refusé → `Refusee`) et `InstallationSimulee { dossier: PathBuf, refuse: bool }` qui copie dans son dossier.
  - `DetailReglage` gagne `installe: bool` ; commande Tauri `parc_installer_profil(profil: IdProfil)`.

- [ ] **Step 1 : Écrire les tests (adapter simulé, dossier temporaire)**

```rust
#[test] fn un_profil_neuf_est_copie_dans_le_dossier_des_couleurs() {}
#[test] fn un_profil_deja_installe_a_l_identique_est_accepte() {}       // DejaInstalle, aucun appel
#[test] fn un_profil_different_du_meme_nom_n_est_pas_ecrase() {}        // DejaPresentDifferent, fichier d'origine intact
#[test] fn un_refus_de_windows_est_rendu_tel_quel() {}                  // refuse: true → Refusee
#[test] fn le_detail_dit_si_le_profil_actif_est_installe() {}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test installation`
Expected: FAIL.

- [ ] **Step 3 : Écrire `app/src/installation.rs`**

Textes : « Installé dans Windows », « Un autre profil du même nom est déjà installé dans Windows : renommez ce profil ou retirez l'autre. », « Windows a refusé l'installation : il demande peut-être des droits d'administrateur. Exportez le profil et installez-le depuis l'Explorateur (clic droit, Installer le profil). »

- [ ] **Step 4 : Lancer les tests**

Run: `cargo test -p app` puis `cargo test -p app --target i686-pc-windows-msvc`
Expected: PASS.

- [ ] **Step 5 : Commit**

```bash
git add app/src/installation.rs app/src/parc.rs app/src/lib.rs app/src/textes.rs app/tests/installation.rs
git commit -m "Application : installer le profil actif dans Windows sans écraser un autre profil"
```

### Tâche 5 : Bibliothèque en arbre et vue d'une résolution

**Files:**
- Create: `app/interface/parc.js`
- Modify: `app/interface/index.html`, `app/interface/bibliotheque.js` (le rendu de l'arbre passe à `parc.js` ; détail de mesure, import, export et sauvegarde restent), `app/src/textes.rs`
- Test: `app/tests/page.rs`

**Interfaces:**
- Consumes : commandes de la tâche 3 et `parc_installer_profil` (tâche 4) ; classes de la tâche 1.
- Produces : `parc.js` expose `window.Parc = { recharger(), choisirReglage(idCondition) }` ; `bibliotheque.js` expose `window.Bibliotheque.afficherMesure(id)` que `parc.js` appelle pour une mesure.

- [ ] **Step 1 : Écrire les tests de la page**

```rust
#[test]
fn parc_js_est_charge_apres_app_js() {}

#[test]
fn la_zone_gauche_a_recherche_arbre_et_ajout_d_imprimante() {
    // index.html : dans [data-zone="gauche"], un input [data-recherche], un [data-arbre],
    // un bouton [data-ajouter-imprimante].
}

#[test]
fn la_vue_d_une_resolution_a_un_seul_bouton_principal() {
    // Dans [data-vue-reglage] : exactement un `btn--primary` ([data-installer]) ;
    // [data-refaire] est inactif et suivi de son `why`.
}
```

- [ ] **Step 2 : Lancer, constater l'échec**

Run: `cargo test -p app --test page`
Expected: FAIL.

- [ ] **Step 3 : Écrire `parc.js` et la vue**

Arbre : imprimante (repliable, nombre de papiers quand repliée) → papier → résolution ; un papier à une seule résolution s'affiche sans ce niveau. À droite de chaque papier ou résolution, l'âge du profil actif : mois et année (`.age--recent`), signalé s'il a plus d'un an (`.age--ancien`), « aucun » sinon. Recherche à chaque frappe. En bas, groupe « Non classées » avec les conditions d'avant et leurs mesures. Ajout en ligne, sans fenêtre : « Ajouter une imprimante » en bas de l'arbre, « Ajouter un papier » (nom + résolution, résolution pré-remplie avec la dernière de cette imprimante) et « Ajouter une résolution » sur le nœud choisi.

Centre d'une résolution : fil d'Ariane `Bibliothèque / imprimante / papier`, titre `papier · résolution`, carte verdict (« Profil vérifié », « Profil non vérifié », « Aucun profil » ; date, espace RVB ou CMJN), boutons « Installer dans Windows » (principal ; inactif sans profil, avec sa raison), « Exporter », « Ranger un profil existant », « Refaire le profil » (inactif : « Le calcul des profils arrive dans une prochaine étape. »), historique (date, actuel ou remplacé, état), mesures de la condition. Droite : provenance du profil (origine, empreinte courte, « inconnu » écrit en toutes lettres pour l'instrument et la mire d'un profil rangé), fiche d'impression « inconnue », état d'installation.

- [ ] **Step 4 : Lancer les tests et contrôles**

Run: `cargo test -p app` puis `cargo clippy --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5 : Test d'écran par le mainteneur (capitaine, étape 5)**

Lancer `cargo run -p app`. Critères, un par un :
1. L'application s'ouvre en bleu nuit sur Bibliothèque ; la bascule passe en clair et revient.
2. Ajouter « Canon PRO-4100 », un papier avec sa résolution, puis une seconde résolution.
3. Ranger un vrai profil ICC de l'atelier : il apparaît avec sa date, le nom proposé est lisible.
4. « Installer dans Windows » : le profil apparaît dans la Gestion des couleurs de Windows (ou le message de refus est clair).
5. Mesurer fonctionne comme avant (étalonnage, mesure, liste) dans le nouveau cadre.
6. Les anciennes mesures d'essai sont visibles sous « Non classées ».

- [ ] **Step 6 : Commit**

```bash
git add app/interface app/src/textes.rs app/tests/page.rs
git commit -m "Bibliothèque : arbre imprimante, papier, résolution et vue du profil"
```

### Tâche 6 : Documentation

**Files:**
- Modify: `docs/adr/0003-interface-tauri.md` (section « Direction visuelle » : remplacée le 22 octobre 2026 par le thème bleu nuit, renvoi à l'ADR 0007), `docs/adr/0007-poste-unique.md` (cadre livré), `GLOSSARY.md` (section « Application » : Pastille de l'instrument, Ranger un profil existant, Installer dans Windows), `docs/blog/<date du jour>.md`, `CLAUDE.md` (ligne `app` : nouveaux modules `parc` et `installation`)

- [ ] **Step 1 : Rédiger, en mots simples**
- [ ] **Step 2 : Contrôler**

Run: `python outils/verifier_docs.py`
Expected: vert.

- [ ] **Step 3 : Commit**

```bash
git add docs GLOSSARY.md CLAUDE.md
git commit -m "Documentation : cadre bleu nuit, parc et installation des profils"
```

## Questions ouvertes (ne bloquent pas le plan)

- `InstallColorProfileW` demande-t-il des droits d'administrateur sur le poste ? Le critère d'écran 4 le dira ; en cas de refus, le message propose l'export, et une installation par la fenêtre de contrôle de compte (comme le pare-feu) fera l'objet d'un ticket.
