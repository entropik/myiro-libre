// Cadre de l'application : tâches, thème, langue. Les textes viennent du catalogue
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
  for (const v of document.querySelectorAll("[data-vue]")) v.hidden = v.dataset.vue !== tache;
  document.querySelector("[data-tache-courante]").textContent = textes["tache." + tache] || "";
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
});
sombreSysteme.addEventListener("change", marquerTheme);

(async () => {
  appliquerTheme(memoire("theme"));
  const demandee = await invoke("langue_demandee");
  await appliquerLangue(demandee || memoire("langue") || "fr");
  document.body.hidden = false;
})();
