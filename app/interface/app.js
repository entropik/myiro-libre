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
}

// ---- Instrument : l'état vient du module Rust `instrument`, la page ne fait qu'afficher ----
let vueInstrument = null; // dernier état rendu par le module `instrument`
let occupe = true; // recherche de l'instrument en cours
const TACHES_SANS_INSTRUMENT = ["bibliotheque"];

function ecranInstrument() {
  return vueInstrument && vueInstrument.probleme ? vueInstrument.probleme.ecran : null;
}

// Feuille du centre : l'écran de l'instrument remplace celle des tâches qui en ont besoin.
function afficherFeuille() {
  const tache = tacheCourante();
  const ecran = TACHES_SANS_INSTRUMENT.includes(tache) ? null : ecranInstrument();
  for (const v of document.querySelectorAll("[data-vue]")) v.hidden = ecran !== null || v.dataset.vue !== tache;
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
  const guide = section.querySelector("[data-guide-cablage]");
  if (guide) guide.hidden = !(concerne && probleme.guide_cablage);
  const detail = concerne ? probleme.detail : null;
  section.querySelector("[data-details]").hidden = !detail;
  section.querySelector("[data-details-texte]").textContent = detail || "";
  section.querySelector("[data-raison-recherche]").hidden = !occupe;
}

function afficherInstrument() {
  const barre = document.querySelector("[data-instrument]");
  const texte = barre.querySelector("[data-instrument-texte]");
  let pret = false;
  if (occupe || !vueInstrument) texte.textContent = textes["instrument.recherche"];
  else if (ecranInstrument() === "choix_dossier") texte.textContent = textes["instrument.logiciel_absent"];
  else if (vueInstrument.etat === "non_detecte") texte.textContent = textes["instrument.aucun"];
  else {
    texte.textContent = textes["instrument.barre"]
      .replace("{modele}", vueInstrument.modele)
      .replace("{etat}", textes["instrument.etat." + vueInstrument.etat]);
    pret = vueInstrument.etat !== "etalonnage_requis";
  }
  barre.classList.toggle("state--ok", pret);
  barre.classList.toggle("state--warn", !pret);
  for (const b of document.querySelectorAll("[data-action]")) b.disabled = occupe;
  afficherFeuille();
}

// Appelle une commande du module instrument. Elle rend la nouvelle vue, ou
// `null` si l'opérateur a annulé (la vue précédente reste).
async function interrogerInstrument(commande) {
  occupe = true;
  afficherInstrument();
  try {
    const vue = await invoke(commande);
    if (vue) vueInstrument = vue;
  } catch (erreur) {
    vueInstrument = {
      etat: "non_detecte",
      modele: null,
      probleme: { code: "pont_en_panne", ecran: "non_detecte", guide_cablage: false, detail: String(erreur) },
    };
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
  if (cible.dataset.action === "choisir_dossier") interrogerInstrument("choisir_dossier");
});
sombreSysteme.addEventListener("change", marquerTheme);

(async () => {
  appliquerTheme(memoire("theme"));
  const demandee = await invoke("langue_demandee");
  await appliquerLangue(demandee || memoire("langue") || "fr");
  document.body.hidden = false;
  await interrogerInstrument("ouvrir_instrument");
})();