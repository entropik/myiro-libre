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
let vueInstrument = null; // null : recherche en cours
let ecranChoisi = null; // écran ouvert par l'opérateur (changer l'emplacement du SDK)
const TACHES_SANS_INSTRUMENT = ["bibliotheque"];

function ecranInstrument() {
  if (ecranChoisi) return ecranChoisi;
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

// Cause et action du problème (sauf quand le titre de l'écran le dit déjà), détail replié.
function remplirEcran(section) {
  const probleme = vueInstrument && vueInstrument.probleme;
  const concerne = Boolean(probleme) && probleme.ecran === section.dataset.ecran;
  const evident = concerne && ["aucun_instrument", "sdk_non_indique"].includes(probleme.code);
  const avis = section.querySelector("[data-avis]");
  avis.hidden = !concerne || evident;
  if (concerne) {
    avis.querySelector("[data-avis-cause]").textContent = textes["probleme." + probleme.code + ".cause"];
    avis.querySelector("[data-avis-action]").textContent = textes["probleme." + probleme.code + ".action"];
  }
  const detail = concerne ? probleme.detail : null;
  section.querySelector("[data-details]").hidden = !detail;
  section.querySelector("[data-details-texte]").textContent = detail || "";
}

function afficherInstrument() {
  const barre = document.querySelector("[data-instrument]");
  const texte = barre.querySelector("[data-instrument-texte]");
  let pret = false;
  if (!vueInstrument) texte.textContent = textes["instrument.recherche"];
  else if (vueInstrument.etat === "non_detecte") texte.textContent = textes["instrument.aucun"];
  else {
    texte.textContent = vueInstrument.modele + ", " + textes["instrument.etat." + vueInstrument.etat];
    pret = vueInstrument.etat !== "etalonnage_requis";
  }
  barre.classList.toggle("state--ok", pret);
  barre.classList.toggle("state--warn", !pret);
  for (const b of document.querySelectorAll("[data-action]")) b.disabled = !vueInstrument;
  afficherFeuille();
}

async function ouvrirInstrument(commande, args) {
  vueInstrument = null;
  ecranChoisi = null;
  afficherInstrument();
  try {
    vueInstrument = await invoke(commande, args);
  } catch (erreur) {
    vueInstrument = {
      etat: "non_detecte",
      modele: null,
      probleme: { code: "pont_en_panne", ecran: "non_detecte", detail: String(erreur) },
    };
  }
  afficherInstrument();
}

async function changerSdk() {
  ecranChoisi = "emplacement_sdk";
  const champ = document.getElementById("emplacement-sdk");
  if (!champ.value) champ.value = (await invoke("emplacement_sdk")) || "";
  afficherFeuille();
  champ.focus();
}

function validerSdk() {
  ouvrirInstrument("indiquer_sdk", { chemin: document.getElementById("emplacement-sdk").value });
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
  if (cible.dataset.action === "reessayer") ouvrirInstrument("ouvrir_instrument");
  if (cible.dataset.action === "changer_sdk") changerSdk();
  if (cible.dataset.action === "valider_sdk") validerSdk();
});
document.getElementById("emplacement-sdk").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && vueInstrument) validerSdk();
});
sombreSysteme.addEventListener("change", marquerTheme);

(async () => {
  appliquerTheme(memoire("theme"));
  const demandee = await invoke("langue_demandee");
  await appliquerLangue(demandee || memoire("langue") || "fr");
  document.body.hidden = false;
  document.getElementById("emplacement-sdk").value = (await invoke("emplacement_sdk")) || "";
  await ouvrirInstrument("ouvrir_instrument");
})();
