# Moteur de rapport et rapport client d'une image (#89) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Depuis la vue « Comparer » où une photo est ouverte (#80), l'imprimeur exporte en un clic un **rapport PDF** à remettre à son client. Le rapport est sobre, en A4 portrait, avec un **cartouche d'architecte** aux coordonnées de l'atelier. Il contient la phrase « x % de l'image est imprimable… », la photo et sa surimpression, les pastilles Image / Imprimé, les points relevés à la pipette et les parties cochées de l'accordéon. Le même moteur servira ensuite à la vérification (#41), au contrôle et aux mesures.

**Architecture:** Une nouvelle crate `rapport`, indépendante de Tauri, de Windows et des ponts, décrit un document : cartouche, sections et blocs. Elle l'écrit en **HTML autonome** : un seul fichier, styles intégrés, images en `data:`, aucun script. L'application garde le réglage « Atelier » dans un fichier JSON à côté de ses données. Elle transforme ce HTML en **PDF** par un trait `ImprimantePdf` : `ImprimantePdfWebView2` charge le fichier dans une fenêtre cachée et appelle `PrintToPdf` de WebView2 ; `ImprimantePdfSimulee` sert aux tests (ADR 0005). Le rapport d'une image est construit côté Rust depuis l'image et les analyses déjà en mémoire (#80) ; la page n'envoie que les choix et les graphiques des parties cochées.

**Tech Stack:** Rust 2021 ; Tauri 2.12 (`WebviewWindowBuilder`, `with_webview`) ; `webview2-com` 0.39 et `windows` 0.62, déjà tirées par `wry` (licences MIT / Apache-2.0) ; `image` 0.25, déjà dans `photo` ; HTML, CSS et JavaScript simples.

**Spec :** ticket #89 et son commentaire de décisions du 30 octobre 2026 (`gh issue view 89 --comments`). Ce plan en tient lieu : il n'y a pas de document de spécification séparé.

## Décisions à faire valider par le mainteneur

Ces questions sont posées avant la tâche 1. La recommandation est appliquée si le mainteneur ne dit rien d'autre.

**Réponse du mainteneur (30 octobre 2026) : « 1a 2a 3a 4a 5a »** — les cinq recommandations sont retenues telles quelles.

1. **Où va le cartouche ?** *Recommandé* : en **pied de chaque page**, en bas à droite, comme sur un plan (atelier, logo, client, objet, papier, profil, date, n° de rapport, page x / y). *Autre choix* : en haut de la première page seulement, et un simple pied de page ensuite.
2. **Numéro de rapport.** *Recommandé* : l'année puis un numéro qui se suit, remis à 1 chaque année (`2026-0001`, `2026-0002`…). *Autre choix* : la date et l'heure (`20261030-1542`).
3. **Points de la pipette.** *Recommandé* : un bouton « Garder ce point » dans le bloc du point cliqué. Les points gardés, **8 au plus**, forment une liste « Points relevés » (chacun avec « Retirer »), et tous partent dans le rapport. *Autre choix* : seul le dernier point cliqué part dans le rapport.
4. **Ce qu'on demande au moment d'exporter.** *Recommandé* : **Client** et **Objet / référence** (facultatifs), puis la langue du rapport (celle de l'app par défaut). Rien d'autre.
5. **Parties cochées par défaut.** *Recommandé* : la phrase de verdict, l'image (photo, surimpression, pastilles, points relevés) et les chiffres par profil sont toujours dans le rapport. Les autres parties de l'accordéon (gamut en volume, chroma par teinte, coupe, chemins, couleurs de base, écarts) sont **décochées par défaut**, et l'app retient les coches.

## Choix du moteur PDF (décision du général, à relire)

| Moteur | Pour | Contre |
|---|---|---|
| **HTML imprimé par WebView2 `PrintToPdf`** (retenu) | Même moteur que l'écran : le système graphique, les graphiques SVG et la vue 3D se reprennent tels quels. Aucune nouvelle crate : `webview2-com` et `windows` sont déjà dans `Cargo.lock` par `wry` (MIT / Apache-2.0). Aucun réseau. Le HTML autonome reste exportable seul (décision de #41). | Windows seulement, comme l'app. Il faut une fenêtre cachée et une attente asynchrone. La pagination suit les règles d'impression de Chromium (`@page`, `break-inside`). |
| `typst` (crate, Apache-2.0) | Mise en page fine, sans navigateur. | Très grosse dépendance, polices à embarquer, deuxième langage de mise en page à maintenir, graphiques à redessiner. |
| `printpdf` (MIT) | Petit, pur Rust. | Tout se place à la main (texte, coupures de ligne, tableaux) : beaucoup de code, rendu pauvre. |
| Impression manuelle « Microsoft Print to PDF » (choix d'origine de #41) | Rien à coder. | Trois gestes de plus pour l'imprimeur, marges et en-têtes du navigateur à régler à chaque fois. |

**Confirmé :** `webview2-com` 0.39.1, `windows` 0.62.2 et `wry` 0.57 sont dans `Cargo.lock`. **Supposé, vérifié par la pointe de la tâche 3 :** une fenêtre Tauri invisible peut charger un fichier `file:///` local et le passer à `ICoreWebView2_7::PrintToPdf`, sans en-tête ni pied du navigateur, et les éléments en `position: fixed` se répètent sur chaque page imprimée. Si la pointe échoue, on s'arrête et on revient au général avant toute autre tâche.

## Global Constraints

- Français dans le code, les commentaires et la documentation ; textes de l'interface et du rapport dans `app/src/textes.rs`, en français et en anglais. Un rapport est dans **une seule langue** (règle de #41).
- **Aucun identifiant d'instrument** dans un rapport (ni numéro de série, ni adresse réseau, ni MAC), ni dans un fichier versionné (#84, mémoire du projet).
- Rapport sobre, en noir et gris, sans ombre ni arrondi. Le vert fluo `#39ff14` (jeton `--couleur-hors-gamut-calque`) ne sert qu'à la surimpression. Un verdict n'est jamais dit par la couleur seule.
- HTML du rapport : un seul fichier, styles intégrés, images en `data:`, **aucun script**, aucun lien externe, et un `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">`. Tout texte variable est échappé, et tout SVG venu de la page passe par une liste blanche.
- A4 portrait (`@page { size: A4; margin: 15mm 15mm 32mm }` : le bas garde la place du cartouche).
- Règles d'interface du mainteneur : **ne jamais faire bouger l'UI** (mises à jour sur place, emplacements réservés), toujours du padding, ascenseurs larges (`--ascenseur`), ascenseur à gauche des zones imbriquées (`.defile-gauche`). Un seul bouton principal par écran, et toute action inactive dit sa raison.
- Fichiers de moins de 400 lignes. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` en 64 et 32 bits. La crate `rapport` entre dans la liste `CRATES` du job `linux` de la CI.
- Aucun test n'ouvre de vraie fenêtre ni n'écrit hors d'un dossier temporaire. Le test qui produit un vrai PDF est `#[ignore]`.

## Review Focus

1. **Logo trop grand ou piégé** (photo de 40 Mo, SVG avec script) : seuls PNG et JPEG sont acceptés, réduits à 600 px au plus et réencodés en PNG. Le SVG est refusé avec sa raison. → test `un_logo_svg_est_refuse_et_un_grand_logo_reduit` (tâche 2).
2. **Texte saisi qui casse le HTML** (`<script>`, `&`, guillemets dans le nom du client ou de l'atelier) : il ressort échappé, et le PDF ne contient aucun script. → test `les_textes_saisis_sont_echappes` (tâche 1).
3. **PDF de destination ouvert dans un lecteur ou dossier protégé** : refus nommé « le fichier est ouvert ou protégé », aucun fichier à moitié écrit, fichier temporaire effacé. → test `un_pdf_non_ecrit_laisse_rien_derriere` (tâche 3).
4. **Partie cochée mais repliée** (graphique jamais dessiné) : la page la dessine hors écran le temps de l'export, sans rien déplacer dans la vue. → test `exporter_dessine_hors_ecran_les_parties_cochees_repliees` (tâche 5).
5. **Rapport long** (4 profils, 8 points, toutes les parties cochées) : aucune coupure au milieu d'un tableau de pastilles ou d'une image (`break-inside: avoid`), et le cartouche ne recouvre jamais le contenu. → test `les_blocs_ne_se_coupent_pas_et_le_pied_garde_sa_place` (tâche 1), vérifié à l'essai d'écran (tâche 7).

---

## Fichiers

- Create `crates/rapport/` : `Cargo.toml` (dépend de `serde` seulement), `src/lib.rs` (types publics), `src/html.rs` (écriture), `src/style.rs` (CSS d'impression), `src/svg.rs` (liste blanche), `tests/html.rs`, `tests/svg.rs`.
- Create `app/src/atelier.rs` (réglage Atelier, logo, numéro de rapport), `app/tests/atelier.rs`.
- Create `app/src/pdf.rs` (trait `ImprimantePdf`, adapters WebView2 et simulé), `app/tests/pdf.rs`.
- Create `app/src/comparer/image/rapport.rs` (document d'une image), `app/src/comparer/image/vignette.rs` (vignettes), `app/tests/rapport_image.rs`.
- Create `app/interface/rapport-image.js` (fenêtre d'export, captures des graphiques) et `app/interface/atelier.js` (formulaire Atelier).
- Modify `app/interface/accordeon.js` (coche « Inclure dans le rapport »), `app/interface/pastilles-image.js` (points relevés), `app/interface/comparer.js` (bouton « Rapport PDF… »), `app/interface/index.html`, `app/src/textes.rs`, `app/src/lib.rs`, `app/Cargo.toml`, `Cargo.toml`, `.github/workflows/ci.yml`, `design-system/components.css`, `design-system/ui-kit.html`, `design-system/README.md`.
- Docs : `docs/adr/0008-moteur-de-rapport.md`, `GLOSSARY.md`, `CLAUDE.md`, `docs/blog/<jour>.md`.

---

### Tâche 1 : Crate `rapport` — document, HTML autonome, cartouche

**Files:**
- Create: `crates/rapport/Cargo.toml`, `crates/rapport/src/{lib.rs,html.rs,style.rs,svg.rs}`
- Modify: `Cargo.toml` (membre), `.github/workflows/ci.yml` (`-p rapport` dans `CRATES`)
- Test: `crates/rapport/tests/html.rs`, `crates/rapport/tests/svg.rs`

**Interfaces:**
- Produces :
  ```rust
  pub struct Atelier { pub nom: String, pub adresse: String, pub courriel: String,
      pub telephone: String, pub site: String, pub logo_png: Option<Vec<u8>> }
  pub struct Cartouche { pub atelier: Atelier, pub client: String, pub objet: String,
      pub lignes: Vec<(String, String)> /* papier, profil, date… déjà traduits */,
      pub numero: String, pub titre: String }
  pub struct Document { pub langue: String /* "fr" | "en" */, pub cartouche: Cartouche,
      pub sections: Vec<Section>, pub libelles: Libelles }
  pub struct Libelles { pub page: String /* "Page {n} / {total}" */, pub atelier: String,
      pub client: String, pub objet: String, pub numero: String }
  pub struct Section { pub titre: String, pub blocs: Vec<Bloc> }
  pub enum Bloc {
      Verdict { texte: String, detail: String },
      Paragraphe(String),
      Couples(Vec<(String, String)>),
      Tableau { entetes: Vec<String>, lignes: Vec<Vec<String>>, nombres: Vec<bool> },
      Images(Vec<Figure>),                 // côte à côte
      Paires(Vec<PaireCouleurs>),          // pastilles Image / Imprimé
      Svg { svg: String, legende: String },// déjà passé par la liste blanche
      Pre(String),
  }
  pub struct Figure { pub png: Vec<u8>, pub legende: String, pub largeur_mm: f32 }
  pub struct PaireCouleurs { pub image: String /* #rrggbb */, pub imprime: String,
      pub alt: String, pub lignes: Vec<String> }
  pub fn ecrire_html(d: &Document) -> String;
  pub fn echapper(t: &str) -> String;
  pub fn nettoyer_svg(svg: &str) -> Result<String, RefusSvg>;
  ```
- Consumes : rien (seulement `serde`).

- [ ] **Step 1 : Écrire les tests qui échouent** (`tests/html.rs`) :
  ```rust
  #[test] fn le_html_est_autonome_et_sans_script() {}          // contient la meta CSP exacte, aucun "<script", aucun "http", aucun "src=\"" hors "data:"
  #[test] fn les_textes_saisis_sont_echappes() {}               // client "<script>&\"'" → "&lt;script&gt;&amp;&quot;&#39;"
  #[test] fn le_cartouche_porte_atelier_client_numero() {}      // nom, adresse (sauts de ligne en <br>), courriel, numéro, titre, lignes
  #[test] fn le_logo_est_une_image_data_png() {}                // "data:image/png;base64," ; sans logo, aucune balise <img> dans le cartouche
  #[test] fn le_cartouche_est_en_pied_de_chaque_page() {}       // décision 1 : classe "cartouche" en position:fixed; bottom:0 dans le style ; @page margin bas 32mm
  #[test] fn les_blocs_ne_se_coupent_pas_et_le_pied_garde_sa_place() {} // style : "break-inside:avoid" sur .bloc, figure, table, .paires
  #[test] fn le_style_est_sobre() {}                            // aucun "box-shadow", aucun "border-radius", couleurs du style ⊂ {noir, gris, blanc}
  #[test] fn une_paire_garde_ses_deux_couleurs_et_son_texte() {}// deux carrés aux #rrggbb donnés, role="img", aria-label = alt
  ```
  Et `tests/svg.rs` :
  ```rust
  #[test] fn un_svg_de_graphique_passe_tel_quel() {}            // svg, g, path, line, polyline, polygon, rect, circle, text, tspan, title ; attributs de dessin
  #[test] fn un_script_ou_un_lien_dans_un_svg_est_refuse() {}  // <script>, on*=, href, xlink:href, <foreignObject>, url( → RefusSvg
  ```
- [ ] **Step 2 : Lancer, constater l'échec** — `cargo test -p rapport` : la crate n'existe pas.
- [ ] **Step 3 : Implémenter.** Le style vient de `style.rs`. Corps en Inter si présente sur le poste, sinon sans-serif. Titres en capitales espacées comme les étiquettes de l'app, filets de 0,5 pt, cartouche en tableau à filets de 0,75 pt (logo, atelier | client, objet | lignes | n°, page). Le numéro de page s'écrit avec `counter(page)` dans une boîte de marge `@page @bottom-right`. Si WebView2 ne les prend pas en charge (constaté à la tâche 3), le pied montre seulement le n° de rapport, et l'écart est noté dans l'ADR. `nettoyer_svg` est un petit analyseur de balises : il garde la liste blanche, retire les attributs `on*` et `style` contenant `url(`, et refuse le reste. Pas de crate XML : le SVG vient de notre propre page.
- [ ] **Step 4 : Tests verts** — `cargo test -p rapport`, puis `cargo clippy -p rapport --all-targets -- -D warnings`.
- [ ] **Step 5 : Commit** — `Rapport : crate rapport, document en HTML autonome avec cartouche d'architecte`.

### Tâche 2 : Réglage « Atelier », logo et numéro de rapport

**Files:**
- Create: `app/src/atelier.rs`
- Modify: `app/src/lib.rs` (état, commandes), `app/src/textes.rs` (`atelier.*`)
- Test: `app/tests/atelier.rs`

**Interfaces:**
- Consumes : `rapport::Atelier`.
- Produces :
  ```rust
  pub struct ReglageAtelier { dossier: PathBuf }               // app_data_dir()/atelier
  impl ReglageAtelier {
      pub fn new(dossier: PathBuf) -> Self;
      pub fn lire(&self) -> Result<rapport::Atelier, ErreurAtelier>;       // absent → Atelier vide
      pub fn enregistrer(&self, a: &AtelierSaisi) -> Result<(), ErreurAtelier>;  // atelier.json, écriture atomique
      pub fn poser_logo(&self, fichier: &Path) -> Result<(), ErreurAtelier>;     // logo.png
      pub fn retirer_logo(&self) -> Result<(), ErreurAtelier>;
      pub fn prochain_numero(&self, annee: i32) -> Result<String, ErreurAtelier>; // "2026-0001", rapports.json
  }
  pub struct AtelierSaisi { nom, adresse, courriel, telephone, site: String }  // serde, champs bornés
  pub enum ErreurAtelier { LogoFormat, LogoIllisible, Ecriture(String), Lecture(String) } // + fn cle()
  // Commandes : atelier_lire, atelier_enregistrer(saisi), atelier_choisir_logo(filtre) (sélecteur Tauri), atelier_retirer_logo
  ```
- [ ] **Step 1 : Tests qui échouent** :
  ```rust
  #[test] fn sans_fichier_l_atelier_est_vide() {}
  #[test] fn l_atelier_enregistre_se_relit_a_l_identique() {}
  #[test] fn un_champ_trop_long_est_refuse() {}                 // nom ≤ 120, adresse ≤ 400, autres ≤ 200 caractères
  #[test] fn un_logo_svg_est_refuse_et_un_grand_logo_reduit() {}// .svg → LogoFormat ; PNG 4000×1000 → logo.png de 600×150
  #[test] fn le_numero_se_suit_et_repart_a_1_chaque_annee() {}  // 2026-0001, 2026-0002, puis 2027-0001
  #[test] fn une_ecriture_interrompue_garde_l_ancien_atelier() {}// fichier temporaire puis renommage
  ```
- [ ] **Step 2 : Échec constaté** — `cargo test -p app --test atelier`.
- [ ] **Step 3 : Implémenter.** Le dossier est `app.path().app_data_dir()/atelier`, à côté des données de l'app. Ce n'est pas la bibliothèque : il n'y a aucune migration, et la sauvegarde de la bibliothèque ne l'inclut pas (écrit dans l'ADR). Le logo est décodé par `image`, réduit au plus à 600 px de côté, et réencodé en PNG. Le numéro de rapport est réservé au moment de l'export réussi seulement (tâche 4).
- [ ] **Step 4 : Tests verts.**
- [ ] **Step 5 : Commit** — `Atelier : coordonnées, logo et numéro de rapport, gardés à côté des données de l'application`.

### Tâche 3 : Imprimer un HTML en PDF (pointe d'abord)

**Files:**
- Create: `app/src/pdf.rs`
- Modify: `app/Cargo.toml` (`[target.'cfg(windows)'.dependencies] webview2-com = "0.39"`, `windows = { version = "0.62", features = [...] }`, mêmes versions que `Cargo.lock`)
- Test: `app/tests/pdf.rs`

**Interfaces:**
- Produces :
  ```rust
  pub trait ImprimantePdf: Send + Sync {
      fn imprimer(&self, html: &str, sortie: &Path) -> Result<(), ErreurPdf>;
  }
  pub struct ImprimantePdfSimulee { pub recus: Mutex<Vec<String>>, pub echec: Option<ErreurPdf> } // écrit "%PDF-1.7 simulé\n" + garde le HTML
  pub struct ImprimantePdfWebView2 { app: tauri::AppHandle }                                      // Windows
  pub enum ErreurPdf { Destination(String), Moteur(String), Delai } // + fn cle()
  pub const DELAI_PDF: Duration = Duration::from_secs(30);
  ```
- [ ] **Step 1 : Pointe, sans commit** (dans un test `#[ignore]` `un_vrai_pdf_sort_de_webview2`). Fenêtre `WebviewWindowBuilder` invisible, URL `WebviewUrl::External(file:///…/rapport.html)`, attente de `PageLoadEvent::Finished`. Ensuite `with_webview(|w| w.controller())` → `CoreWebView2` → `cast::<ICoreWebView2_7>()`, réglages par `ICoreWebView2Environment6::CreatePrintSettings` (portrait, 8,27 × 11,69 pouces, marges 0, `ShouldPrintBackgrounds` vrai, `ShouldPrintHeaderAndFooter` faux), puis `PrintToPdf` avec un `PrintToPdfCompletedHandler`. Noter dans le compte rendu ce qui est **confirmé** : la fenêtre cachée, `file:///`, le pied répété, `counter(page)`. **Si un point bloque, s'arrêter et le dire au général.**
- [ ] **Step 2 : Tests qui échouent** (sans vraie fenêtre) :
  ```rust
  #[test] fn le_simule_ecrit_un_pdf_et_garde_le_html() {}
  #[test] fn un_pdf_non_ecrit_laisse_rien_derriere() {}        // destination dans un dossier inexistant → Destination ; aucun .tmp restant
  #[test] fn le_pdf_est_ecrit_a_cote_puis_renomme() {}         // écrit dans <sortie>.tmp puis renommé ; un PDF existant n'est remplacé qu'après succès
  ```
- [ ] **Step 3 : Implémenter** `ImprimantePdfWebView2` : HTML dans un dossier temporaire propre à l'export, effacé à la fin même en cas d'échec ; fenêtre fermée dans tous les cas ; délai de 30 s → `Delai`. Le travail se fait hors du fil de l'interface. La fenêtre est créée depuis le fil principal par `run_on_main_thread`, et le résultat revient par un canal.
- [ ] **Step 4 : Tests verts** (64 et 32 bits), et `cargo test -p app --test pdf -- --ignored` lancé une fois à la main sur le poste. Le PDF est ouvert et regardé.
- [ ] **Step 5 : Commit** — `PDF : un rapport HTML imprimé en PDF par WebView2, dans une fenêtre cachée`.

### Tâche 4 : Le rapport d'une image, côté Rust

**Files:**
- Create: `app/src/comparer/image/rapport.rs`, `app/src/comparer/image/vignette.rs`
- Modify: `app/src/comparer/image.rs` (accès aux analyses gardées), `app/src/comparer/image/commandes.rs` (commande), `app/src/textes.rs` (`rapport.image.*`)
- Test: `app/tests/rapport_image.rs`

**Interfaces:**
- Consumes : `ImageOuverte` (#80 : image d'analyse, analyses par profil, `Rendu`), `gamut::{Pastille, paire_couleur}`, `ReglageAtelier`, `ImprimantePdf`, `rapport::*`.
- Produces :
  ```rust
  #[derive(Deserialize)] pub struct DemandeRapportImage {
      pub langue: Langue, pub client: String, pub objet: String,
      pub analyse: u64, pub profil_surimpression: usize, pub seuil: f64, pub point_noir: bool,
      pub points: Vec<(u32, u32)>,              // pixels de l'image d'analyse, 8 au plus (décision 3)
      pub parties: Vec<String>,                 // clés d'accordéon cochées (décision 5)
      pub graphiques: Vec<GraphiquePartie> }    // SVG ou PNG des parties cochées, venus de la page
  #[derive(Deserialize)] pub struct GraphiquePartie { pub partie: String, pub svg: Option<String>, pub png_base64: Option<String>, pub legende: String }
  pub struct PartImprimable { pub pour_mille_bons: Option<u32>, pub inconnus_pour_mille: u32 } // même règle que verdict-image.js
  pub fn part_imprimable(a: &AnalyseImage, seuil: f64) -> PartImprimable;
  pub fn phrase_imprimable(p: &PartImprimable, langue: Langue) -> String; // « plus de 99,9 % », « moins de 0,1 % »
  impl ImageOuverte { pub fn document_rapport(&self, d: &DemandeRapportImage, atelier: rapport::Atelier, numero: String, contexte: &ContexteComparaison) -> Result<rapport::Document, String>; }
  pub fn vignette(pixels: &[[u8; 4]], largeur: u32, cote_max: u32) -> Vec<u8>;                         // PNG
  pub fn vignette_surimpression(pixels: &[[u8; 4]], ecarts: &[u8], largeur: u32, seuil: f64, cote_max: u32) -> Vec<u8>; // vert #39ff14 si code ≥ round(10 × seuil) et ≠ 255
  // Commande : image_rapport_exporter(demande, filtre) → sélecteur « Enregistrer » (Documents\myiro-libre\Rapports, nom « Rapport <image> <numéro>.pdf ») → PDF → Some(nom) | None si annulé
  ```
- [ ] **Step 1 : Tests qui échouent** (avec `ImprimantePdfSimulee` et le profil d'essai de `gamut`) :
  ```rust
  #[test] fn la_part_imprimable_suit_la_regle_de_la_page() {}  // 972/28 → 97,2 % ; 10 000/1 → « plus de 99,9 % » ; 1/10 000 → « moins de 0,1 % » ; inconnus cités
  #[test] fn le_rapport_porte_verdict_vignettes_et_pastilles() {}// HTML simulé : phrase, deux <img data:>, 12 paires au plus, au-delà du seuil
  #[test] fn la_surimpression_suit_le_seuil_choisi() {}         // seuil 2,0 vs 5,0 : nombre de pixels verts différent, égal au compte de l'analyse
  #[test] fn les_points_releves_partent_avec_leur_paire() {}    // 2 points → 2 paires avec Lab, écart, « au-delà/en deçà »
  #[test] fn une_partie_decochee_n_apparait_pas() {}            // parties sans "comparer.coupe" → aucun titre de coupe
  #[test] fn le_rapport_ne_porte_aucun_identifiant_d_instrument() {} // aucun numéro de série ni « Serial » même si le profil en porte un dans sa description
  #[test] fn un_svg_refuse_n_arrete_pas_le_rapport() {}        // partie remplacée par une phrase « graphique non repris »
  #[test] fn le_numero_n_est_pris_qu_apres_un_pdf_ecrit() {}   // échec simulé → numéro non consommé
  #[test] fn un_rapport_anglais_n_a_aucun_texte_francais() {}
  ```
- [ ] **Step 2 : Échec constaté.**
- [ ] **Step 3 : Implémenter.** Les vignettes font au plus 1 600 px de côté, en PNG, à 80 mm de large chacune, côte à côte. Les pastilles reprennent la sélection de la page (au-delà du seuil, les plus présentes d'abord, 12 au plus), sous la forme d'une fonction partagée dans `pastilles` si elle n'existe que côté JavaScript. Les chiffres par profil forment un tableau avec une colonne par profil. Le cartouche contient : papier, imprimante et résolution du profil de surimpression s'il vient de la bibliothèque (sinon le nom du fichier), le profil, l'image (nom, taille), la date du jour, le seuil et le rendu. **Garder en tête de `verdict-image.js` un commentaire qui renvoie à `part_imprimable`** : les deux doivent rester identiques.
- [ ] **Step 4 : Tests verts.**
- [ ] **Step 5 : Commit** — `Rapport d'une image : verdict, vignettes, pastilles, points relevés et parties cochées`.

### Tâche 5 : Page — coches, points relevés, export

**Files:**
- Create: `app/interface/rapport-image.js`
- Modify: `app/interface/accordeon.js`, `app/interface/pastilles-image.js`, `app/interface/comparer.js`, `app/interface/index.html`, `design-system/components.css`, `design-system/ui-kit.html`, `design-system/README.md`, `app/src/textes.rs`
- Test: `app/tests/page_rapport.rs`

**Interfaces:**
- Consumes : commande `image_rapport_exporter`, `Accordeon`, `Vue3d`, `Graphiques`.
- Produces : `Accordeon.partie({ …, rapport: { cle, cochee_par_defaut } })` ; `window.RapportImage = { ouvrir() }` ; `PastillesImage.pointsReleves()`.

- [ ] **Step 1 : Tests page qui échouent** :
  ```rust
  #[test] fn chaque_partie_a_sa_coche_hors_du_bouton() {}       // <input type=checkbox> à droite de l'en-tête, jamais dans le <button> ; libellé « Inclure dans le rapport »
  #[test] fn les_coches_sont_retenues_sans_stockage_obligatoire() {} // localStorage dans try/catch
  #[test] fn garder_un_point_va_jusqu_a_huit() {}               // « Garder ce point » inactif au-delà de 8, avec sa raison
  #[test] fn la_liste_des_points_a_sa_place_reservee() {}       // emplacement de hauteur fixe : ne jamais bouger l'UI
  #[test] fn exporter_dessine_hors_ecran_les_parties_cochees_repliees() {} // conteneur hors écran position:fixed; left:-10000px, détruit après
  #[test] fn le_bouton_rapport_est_secondaire() {}             // « Rapport PDF… » class="btn" (pas primary) dans la barre de Comparer, inactif sans image avec sa raison
  #[test] fn la_fenetre_d_export_a_un_seul_bouton_principal() {}// « Exporter le PDF » ; champs Client, Objet, Langue ; Échap ferme ; focus rendu
  #[test] fn les_textes_du_rapport_existent_en_deux_langues() {}
  ```
- [ ] **Step 2 : Échec constaté.**
- [ ] **Step 3 : Implémenter.** La fenêtre d'export est une superposition, comme « Agrandir ». Elle contient :
  - le résumé de l'atelier (nom, logo) et « Modifier l'atelier » (tâche 6) ;
  - Client, Objet, Langue ;
  - la liste des parties cochées ;
  - « Exporter le PDF ».

  Pendant l'export, la fenêtre affiche « Rapport en cours… » à une place réservée, et rien ne bouge derrière. Une fois le PDF écrit, elle dit « Rapport enregistré : <nom> » avec « Ouvrir le dossier ». Les captures se font ainsi : les graphiques SVG sont sérialisés (`XMLSerializer`), et la vue 3D est rendue en PNG (`canvas.toDataURL("image/png")`) dans un canvas de 1 200 × 900 hors écran, à l'orientation affichée.
- [ ] **Step 4 : Tests verts**, plus un essai en Node de `rapport-image.js` avec une page simulée, comme pour #80.
- [ ] **Step 5 : Commit** — `Page : coches « Inclure dans le rapport », points relevés et export du rapport PDF`.

### Tâche 6 : Page — formulaire « Atelier »

**Files:**
- Create: `app/interface/atelier.js`
- Modify: `app/interface/index.html`, `app/src/textes.rs`, `design-system/ui-kit.html`
- Test: `app/tests/page_atelier.rs`

- [ ] **Step 1 : Tests page qui échouent** : champs nom, adresse (sur plusieurs lignes), courriel, téléphone, site ; « Choisir un logo… » et « Retirer le logo » ; un aperçu du cartouche, mis à jour à la frappe ; « Enregistrer » comme seul bouton principal ; une erreur (logo refusé, champ trop long) dite sous son champ ; textes en deux langues.
- [ ] **Step 2 : Échec constaté.**
- [ ] **Step 3 : Implémenter.** Le formulaire s'ouvre depuis « Modifier l'atelier » dans la fenêtre d'export. Au premier export, si l'atelier est vide, il s'ouvre d'abord, avec une phrase « Remplissez une fois les coordonnées de l'atelier : elles seront reprises sur chaque rapport. » L'aperçu utilise les mêmes classes de cartouche que le rapport, recopiées du style de la crate dans `components.css`.
- [ ] **Step 4 : Tests verts.**
- [ ] **Step 5 : Commit** — `Page : réglage de l'atelier, rempli une fois, avec aperçu du cartouche`.

### Tâche 7 : Documentation, essai d'écran et PR

**Files:**
- Create: `docs/adr/0008-moteur-de-rapport.md`
- Modify: `GLOSSARY.md` (rapport, cartouche, atelier, points relevés), `CLAUDE.md` (crate `rapport`, modules `atelier` et `pdf`, CI), `docs/blog/<jour>.md`

- [ ] **Step 1 : ADR 0008.** Il contient :
  - le moteur choisi et les options écartées (le tableau ci-dessus) ;
  - le HTML autonome comme source unique ;
  - le PDF par WebView2, avec ce que la pointe a confirmé ou laissé supposé ;
  - où vit l'atelier ;
  - la règle « aucun identifiant d'instrument » ;
  - le **branchement de #41** (section suivante).
- [ ] **Step 2 : Contrôles** — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --target i686-pc-windows-msvc`, `python outils/test_preparer_installateur.py`, `python outils/verifier_docs.py`.
- [ ] **Step 3 : Essai d'écran avec le mainteneur** (geste humain, par le général) :
  1. Remplir l'atelier (avec un logo PNG), puis le rouvrir : tout est gardé.
  2. Avec une photo et deux profils : cocher la coche et le gamut en volume, garder 3 points, exporter en français.
  3. Ouvrir le PDF :
     - cartouche en pied de chaque page, n° `2026-0001` ;
     - phrase de verdict, photo et surimpression, pastilles, 3 points ;
     - coupe et vue 3D lisibles ;
     - rien de coupé ;
     - aucun n° de série.
  4. Exporter de nouveau en anglais : n° `2026-0002`, aucun mot français.
  5. Pendant l'export, la vue ne bouge pas.
  6. Imprimer le PDF sur la PRO-4100 ou en noir sur la bureautique : le rapport reste lisible.
- [ ] **Step 4 : PR vers `dev`.** Elle reprend les décisions du mainteneur, la pointe (ce qui est confirmé ou supposé), le Review Focus, les licences (aucune nouvelle crate tierce) et les résultats de l'essai. Commenter #89.

---

## Branchement de #41 (après ce plan, dans la branche de #41)

#41 (`origin/ticket/41-verification`) a déjà un rapport HTML de vérification (`app/src/rapport.rs`, `rapport/donnees.rs`, `rapport/style.rs`, tests `app/tests/rapport.rs`). Quand #41 sera remise à niveau sur `dev` après ce plan :

1. **Renommer** son module `app/src/rapport.rs` en `app/src/rapport_verification.rs` (avec `donnees.rs`), pour ne pas cacher la crate `rapport`.
2. **Construire un `rapport::Document`** depuis `DonneesRapport`. Chaque titre de #41 devient une `Section`, ses couples des `Bloc::Couples`, ses tableaux de plages des `Bloc::Tableau`, et son verdict un `Bloc::Verdict` (mot et symbole ✓ ✗ ?).
3. **Supprimer** son `STYLE` et son `echapper` au profit de ceux de la crate. Le cartouche reçoit l'atelier, le profil, les tolérances (`v N`) et la date de la vérification.
4. **Garder ses tests** de contenu (deux familles, version des tolérances, une seule langue, aucun script, profil et rapport côte à côte, rapport d'une ancienne vérification identique à l'octet). Le dernier impose que le numéro de rapport d'une vérification soit **gardé avec elle**, et non repris à chaque export. Ce point est à trancher dans #41.
5. **Ajouter** l'export PDF par `ImprimantePdf` à côté de l'export HTML.

Le contrôle d'impression et les mesures d'un profil suivront le même chemin : une fonction `document_…` qui remplit un `rapport::Document`, puis le même export.
