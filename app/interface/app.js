// Cadre de l'application : tâches, thème, langue, instrument. Les textes viennent du catalogue
// Rust (module `textes`) : une seule langue par fenêtre, jamais de texte en dur ici.
"use strict";

const { invoke } = window.__TAURI__.core;
const racine = document.documentElement;
let textes = {};

function memoire(cle, valeur) {
  try {
    if (valeur === undefined) return localStorage.getItem(cle);
    localStorage.setItem(cle, valeur);
  } catch (_) {
    return null;
  }
  return null;
}

// ---- Langue ----
async function appliquerLangue(code) {
  textes = await invoke("catalogue", { langue: code });
  racine.lang = code;
  for (const el of document.querySelectorAll("[data-t]")) el.textContent = textes[el.dataset.t];
  for (const el of document.querySelectorAll("[data-t-aria]")) el.setAttribute("aria-label", textes[el.dataset.tAria]);
  for (const b of document.querySelectorAll("[data-langue-choix]")) {
    b.setAttribute("aria-pressed", String(b.dataset.langueChoix === code));
  }
  afficherTache(tacheCourante());
  afficherInstrument();
  memoire("langue", code);
  // Les parties dessinées par d'autres scripts (bibliotheque.js) se redessinent.
  document.dispatchEvent(new CustomEvent("langue-appliquee"));
}

// ---- Thème : celui du système tant que l'utilisateur n'a pas choisi ----
const sombreSysteme = window.matchMedia("(prefers-color-scheme: dark)");

function themeEffectif() {
  return racine.dataset.theme || (sombreSysteme.matches ? "dark" : "light");
}

function marquerTheme() {
  const theme = themeEffectif();
  for (const b of document.querySelectorAll("[data-theme-choix]")) {
    b.setAttribute("aria-pressed", String(b.dataset.themeChoix === theme));
  }
}

function appliquerTheme(theme) {
  if (theme) racine.dataset.theme = theme;
  marquerTheme();
}

// ---- Tâches ----
function tacheCourante() {
  const b = document.querySelector("[data-tache][aria-current='page']");
  return b ? b.dataset.tache : "mesurer";
}

function afficherTache(tache) {
  for (const b of document.querySelectorAll("[data-tache]")) {
    if (b.dataset.tache === tache) b.setAttribute("aria-current", "page");
    else b.removeAttribute("aria-current");
  }
  document.querySelector("[data-tache-courante]").textContent = textes["tache." + tache] || "";
  afficherFeuille();
  document.dispatchEvent(new CustomEvent("tache-affichee", { detail: tache }));
}

// ---- Instrument : l'état vient du module Rust `instrument`, la page ne fait qu'afficher ----
let vueInstrument = null; // dernier état rendu par le module `instrument`
let occupe = true; // recherche de l'instrument en cours
const TACHES_SANS_INSTRUMENT = ["bibliotheque"];

// L'écran d'étalonnage ne s'ouvre que sur demande de l'opérateur (bouton « Étalonner »).
// Un problème de mesure s'affiche sur la feuille Mesurer elle-même (mesurer.js).
function ecranInstrument() {
  const ecran = vueInstrument && vueInstrument.probleme ? vueInstrument.probleme.ecran : null;
  return ecran === "etalonnage" || ecran === "mesure" ? null : ecran;
}

// Feuille du centre : l'écran de l'instrument remplace celle des tâches qui en ont besoin.
// L'étalonnage en cours l'emporte : l'opérateur l'a demandé.
function afficherFeuille() {
  const tache = tacheCourante();
  const ecran = phaseEtalonnage
    ? "etalonnage"
    : TACHES_SANS_INSTRUMENT.includes(tache)
      ? null
      : ecranInstrument();
  for (const v of document.querySelectorAll("[data-vue]")) v.hidden = ecran !== null || v.dataset.vue !== tache;
  // Détails à droite : ceux de la mesure en cours pour la tâche Mesurer, sinon ceux de la bibliothèque.
  for (const d of document.querySelectorAll("[data-details-tache]")) {
    d.hidden = (d.dataset.detailsTache === "mesurer") !== (tache === "mesurer" && ecran === null);
  }
  for (const e of document.querySelectorAll("[data-ecran]")) {
    e.hidden = e.dataset.ecran !== ecran;
    remplirEcran(e);
  }
}

// Cause et action du problème (sauf quand le titre de l'écran le dit déjà),
// étapes de câblage si elles servent, détail replié, raison d'un bouton inactif.
function remplirEcran(section) {
  const probleme = vueInstrument && vueInstrument.probleme;
  const concerne = Boolean(probleme) && probleme.ecran === section.dataset.ecran;
  const evident = concerne && ["aucun_instrument", "logiciel_absent"].includes(probleme.code);
  const avis = section.querySelector("[data-avis]");
  avis.hidden = !concerne || evident;
  if (concerne) {
    avis.querySelector("[data-avis-cause]").textContent = textes["probleme." + probleme.code + ".cause"];
    avis.querySelector("[data-avis-action]").textContent = textes["probleme." + probleme.code + ".action"];
  }
  const titre = section.querySelector("[data-titre-choix]");
  if (titre) {
    const cas = concerne && probleme.code === "logiciel_inutilisable" ? probleme.code : "logiciel_absent";
    titre.textContent = textes["ecran.choix.titre." + cas];
  }
  const guide = section.querySelector("[data-guide-cablage]");
  if (guide) guide.hidden = !(concerne && probleme.guide_cablage);
  // Pare-feu fermé pour le FD-9 : son bouton passe devant « Réessayer ».
  const autoriser = section.querySelector("[data-action='autoriser_pare_feu']");
  if (autoriser) {
    const montre = concerne && probleme.autoriser_pare_feu;
    autoriser.hidden = !montre;
    section.querySelector("[data-action='reessayer']").classList.toggle("btn--primary", !montre);
  }
  if (section.dataset.ecran === "etalonnage") remplirEtalonnage(section);
  const detail = concerne ? probleme.detail : null;
  section.querySelector("[data-details]").hidden = !detail;
  section.querySelector("[data-details-texte]").textContent = detail || "";
  section.querySelector("[data-raison-recherche]").hidden = !occupe;
}

function afficherInstrument() {
  const barre = document.querySelector("[data-instrument]");
  const texte = barre.querySelector("[data-instrument-texte]");
  const vue = occupe ? null : vueInstrument;
  const code = vue && vue.probleme ? vue.probleme.code : null;
  if (!vue) texte.textContent = textes["instrument.recherche"];
  else if (code === "logiciel_absent") texte.textContent = textes["instrument.logiciel_absent"];
  else if (!vue.modele) texte.textContent = textes["instrument.aucun"];
  else {
    texte.textContent = textes["instrument.barre"]
      .replace("{modele}", vue.modele)
      .replace("{etat}", textes["instrument.etat." + vue.etat]);
  }
  // « Prêt » est décidé par le module `instrument`, jamais ici.
  const pret = Boolean(vue && vue.pret);
  barre.classList.toggle("state--ok", pret);
  barre.classList.toggle("state--warn", !pret);
  for (const b of document.querySelectorAll("[data-action]")) b.disabled = occupe;
  // « Étalonner » à côté de l'état, tant que l'étalonnage est requis et pas déjà ouvert.
  const etalonner = document.querySelector("[data-etalonner]");
  etalonner.hidden = !(vue && vue.etat === "etalonnage_requis") || phaseEtalonnage !== null;
  // Rappel avant toute mesure : la raison des boutons de mesure inactifs le dit.
  const raison = vue && vue.etat === "etalonnage_requis" ? "raison.etalonnage" : "raison.instrument";
  for (const p of document.querySelectorAll("[data-t='raison.instrument']")) p.textContent = textes[raison];
  afficherFeuille();
  // La feuille Mesurer (mesurer.js) suit l'état de l'instrument.
  document.dispatchEvent(new CustomEvent("instrument-affiche"));
}

// ---- Étalonnage guidé : le module `instrument` demande le geste (événement « geste »),
// l'écran le montre et rend la réponse de l'opérateur (commande `repondre_geste`). ----
let phaseEtalonnage = null; // null, "geste", "en_cours" ou "reussi"
let appelEnCours = false; // la commande `etalonner` n'a pas encore rendu sa vue
let gesteEnAttente = false; // le module attend la réponse de l'opérateur
let gesteDejaFait = false; // geste confirmé sur l'écran d'échec, avant d'être redemandé
const ETAPES = { geste: "blanc", en_cours: "etalonnage", reussi: "mesure" };

function remplirEtalonnage(section) {
  const phase = phaseEtalonnage || "geste";
  const courante = ETAPES[phase];
  const ordre = Object.values(ETAPES);
  const etapes = section.querySelector("[data-etapes]");
  etapes.style.setProperty("--n", String(ordre.length));
  for (const b of etapes.querySelectorAll("[data-etape]")) {
    const rang = ordre.indexOf(b.dataset.etape);
    b.classList.toggle("done", rang < ordre.indexOf(courante));
    if (b.dataset.etape === courante) b.setAttribute("aria-current", "step");
    else b.removeAttribute("aria-current");
  }
  for (const el of section.querySelectorAll("[data-phase]")) {
    el.hidden = !el.dataset.phase.split(" ").includes(phase);
  }
  // Boutons actifs quand le module attend la réponse, ou sur l'écran d'échec (plus d'appel en cours).
  const repondable = phase === "geste" && (gesteEnAttente || !appelEnCours);
  for (const b of section.querySelectorAll("[data-geste]")) b.disabled = !repondable;
  const avis = section.querySelector("[data-avis]");
  if (phase !== "geste") avis.hidden = true;
  // Après un échec, l'avis donne déjà le geste à refaire : pas de redite, et
  // l'explication déjà lue laisse la place à l'avis.
  section.querySelector("[data-geste-texte]").hidden = !avis.hidden;
  section.querySelector("[data-pourquoi]").hidden = !avis.hidden;
}

async function lancerEtalonnage(dejaFait) {
  if (appelEnCours) return;
  appelEnCours = true;
  gesteDejaFait = dejaFait;
  phaseEtalonnage = dejaFait ? "en_cours" : "geste";
  afficherInstrument();
  try {
    const vue = await invoke("etalonner");
    if (vue) vueInstrument = vue;
  } catch (erreur) {
    console.error("etalonner", erreur);
  }
  appelEnCours = false;
  gesteEnAttente = false;
  gesteDejaFait = false;
  const etalonne = vueInstrument && vueInstrument.etat === "etalonne";
  const aRefaire = Boolean(vueInstrument && vueInstrument.probleme && vueInstrument.probleme.ecran === "etalonnage");
  // Réussite : étape suivante. Échec : l'écran reste, avec l'avis et une action.
  // Annulation : l'écran se ferme. Perte de l'instrument : l'écran du problème prend le relais.
  if (etalonne && phaseEtalonnage === "en_cours") phaseEtalonnage = "reussi";
  else if (aRefaire && phaseEtalonnage !== null) phaseEtalonnage = "geste";
  else phaseEtalonnage = null;
  afficherInstrument();
}

function repondreGeste(fait) {
  if (gesteEnAttente) {
    gesteEnAttente = false;
    phaseEtalonnage = fait ? "en_cours" : null;
    invoke("repondre_geste", { fait }).catch((erreur) => console.error("repondre_geste", erreur));
  } else if (!appelEnCours) {
    // Écran d'échec : « Lancer l'étalonnage » repart ; « Annuler » ferme l'écran.
    if (fait) {
      lancerEtalonnage(true);
      return;
    }
    phaseEtalonnage = null;
  }
  afficherInstrument();
}

window.__TAURI__.event.listen("geste", (evenement) => {
  if (evenement.payload !== "poser_sur_blanc") return;
  gesteEnAttente = true;
  if (gesteDejaFait) {
    gesteDejaFait = false;
    repondreGeste(true);
    return;
  }
  phaseEtalonnage = "geste";
  afficherInstrument();
});

// Appelle une commande du module instrument. Elle rend la nouvelle vue, ou
// `null` si l'opérateur a annulé : la vue précédente reste. La page ne
// fabrique jamais d'état elle-même ; une erreur d'appel est seulement notée.
async function interrogerInstrument(commande) {
  occupe = true;
  afficherInstrument();
  try {
    const vue = await invoke(commande);
    if (vue) vueInstrument = vue;
  } catch (erreur) {
    console.error(commande, erreur);
  }
  occupe = false;
  afficherInstrument();
}

// ---- Lancement ----
document.addEventListener("click", (e) => {
  const cible = e.target.closest("button");
  if (!cible) return;
  if (cible.dataset.tache) afficherTache(cible.dataset.tache);
  if (cible.dataset.themeChoix) {
    appliquerTheme(cible.dataset.themeChoix);
    memoire("theme", cible.dataset.themeChoix);
  }
  if (cible.dataset.langueChoix) appliquerLangue(cible.dataset.langueChoix);
  if (cible.dataset.action === "reessayer") interrogerInstrument("ouvrir_instrument");
  if (cible.dataset.action === "autoriser_pare_feu") interrogerInstrument("autoriser_pare_feu_fd9");
  if (cible.dataset.action === "choisir_dossier") interrogerInstrument("choisir_dossier");
  if (cible.hasAttribute("data-etalonner")) lancerEtalonnage(false);
  if (cible.dataset.geste) repondreGeste(cible.dataset.geste === "fait");
  if (cible.hasAttribute("data-fin-etalonnage")) {
    phaseEtalonnage = null;
    afficherInstrument();
  }
});
sombreSysteme.addEventListener("change", marquerTheme);

(async () => {
  appliquerTheme(memoire("theme"));
  const demandee = await invoke("langue_demandee");
  await appliquerLangue(demandee || memoire("langue") || "fr");
  document.body.hidden = false;
  // Page rechargée pendant un geste : le module ne l'attend plus.
  await invoke("repondre_geste", { fait: false });
  await interrogerInstrument("ouvrir_instrument");
})();