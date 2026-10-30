// « Mesurer une feuille au FD-9 » dans la vue d'une résolution (ticket #34, tâche 9) : le bouton,
// actif quand un FD-9 est prêt et la mire RGB-2520 trouvée, sinon inactif avec sa raison juste
// dessous ; l'état de la feuille en clair (glissez la feuille, lecture, mesure n / 2 520, rangée, ou
// l'erreur avec sa cause et l'action à faire) ; « Annuler » pendant l'attente et la lecture. Tout
// est mis à jour sur place dans des blocs de taille gardée (index.html) : rien ne bouge, les
// anciennes valeurs restent jusqu'aux nouvelles. L'avancement vient du module Rust par l'événement
// `feuille_fd9` ; la mire et les points mesurés sont montrés par feuille-vue.js. Utilisé par
// reglage.js, qui prête la résolution affichée et de quoi la recharger.
"use strict";

window.FeuilleFd9 = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const vueFeuille = window.FeuilleVue;

  const racine = document.querySelector("[data-feuille-fd9]");
  const mesurer = racine.querySelector("[data-feuille-mesurer]");
  const annuler = racine.querySelector("[data-feuille-annuler]");
  const voir = racine.querySelector("[data-feuille-voir]");
  const raison = racine.querySelector("[data-feuille-raison]");
  const demonstration = racine.querySelector("[data-feuille-demonstration]");
  const pas = [...racine.querySelectorAll("[data-pas]")];
  const cours = racine.querySelector("[data-feuille-cours]");
  const piste = racine.querySelector("[data-feuille-piste]");
  const message = racine.querySelector("[data-feuille-message]");
  const echecBloc = racine.querySelector("[data-feuille-echec]");
  const details = racine.querySelector("[data-feuille-details]");
  const vue = racine.querySelector("[data-feuille-vue]");

  // Étape annoncée → pas montré (feuille, lecture, mesure, rangement).
  const PAS = {
    connexion: "feuille", glisser_feuille: "feuille", feuille_engagee: "feuille",
    lecture: "lecture", calage_pret: "lecture", mesure: "mesure", rangement: "rangement",
  };
  // Pendant l'attente et la lecture, « Annuler » arrête la feuille sans rien mesurer.
  const ANNULABLES = ["connexion", "glisser_feuille", "feuille_engagee", "lecture"];
  const ORDRE = ["feuille", "lecture", "mesure", "rangement"];
  const nombre = (v) => new Intl.NumberFormat(document.documentElement.lang).format(v);

  // Ce que reglage.js prête : la résolution affichée et de quoi la recharger.
  let parc = { reglage: () => null, recharger: async () => {} };
  let condition = null; // résolution affichée
  let etat = null; // réponse de `feuille_fd9_etat`
  // La feuille en cours ou la dernière finie dans cette séance : sa résolution, l'étape annoncée,
  // le compte, puis le résultat. null : rien à montrer (repos).
  let feuille = null;
  let demandeCalage = 0; // le dernier calage demandé l'emporte

  const enCours = () => Boolean(feuille && !feuille.fini && !feuille.echec);

  // ---- Bouton, « Annuler » et raison : la raison dit pourquoi un bouton est inactif ----
  function texteRaison() {
    if (enCours()) {
      if (feuille.condition !== condition) return t("feuille_fd9.raison.ailleurs");
      return t(ANNULABLES.includes(feuille.etape) ? "feuille_fd9.raison.en_cours" : "feuille_fd9.raison.mesure_en_cours");
    }
    if (!etat) return t("feuille_fd9.raison.chargement");
    return etat.raisons.map((c) => t(c)).join(" ");
  }

  function ecrireBoutons() {
    mesurer.disabled = enCours() || !etat || !etat.disponible || condition === null;
    annuler.toggleAttribute("data-vide", !enCours());
    annuler.disabled = !enCours() || !ANNULABLES.includes(feuille.etape);
    raison.textContent = texteRaison();
    demonstration.textContent = etat && etat.demonstration ? t("feuille_fd9.demonstration") : "";
  }

  // ---- État de la feuille : pas, barre, message ou erreur, à leur place ----
  function ecrireEtat() {
    const courant = feuille ? (feuille.fini ? "fin" : PAS[feuille.etape] || "feuille") : null;
    const rang = courant === "fin" ? ORDRE.length : ORDRE.indexOf(courant);
    for (const p of pas) {
      const i = ORDRE.indexOf(p.dataset.pas);
      p.classList.toggle("etape--faite", feuille !== null && i < rang && !(feuille.echec && i === rang));
      if (i === rang && !feuille.fini) p.setAttribute("aria-current", "step");
      else p.removeAttribute("aria-current");
    }
    // Barre : la part des plages mesurées ; pleine une fois la mesure finie.
    const part = !feuille ? 0 : feuille.fini || ["rangement"].includes(feuille.etape)
      ? 1 : feuille.plages ? feuille.mesurees / feuille.plages : 0;
    piste.style.setProperty("--position", `${(part * 100).toFixed(2)}%`);
    piste.setAttribute("aria-valuenow", String(Math.round(part * 100)));
    if (!feuille) piste.dataset.etat = "repos";
    else if (feuille.echec) piste.dataset.etat = "erreur";
    else if (feuille.fini) piste.dataset.etat = "reussi";
    else delete piste.dataset.etat;
    cours.hidden = Boolean(feuille && feuille.echec);
    echecBloc.hidden = !(feuille && feuille.echec);
    message.textContent = texteMessage();
    if (feuille && feuille.echec) ecrireEchec(feuille.echec);
  }

  function texteMessage() {
    if (!feuille) return t("feuille_fd9.repos");
    if (feuille.fini) {
      const fait = t("feuille_fd9.fait").replace("{fichier}", () => feuille.fini.fichier)
        .replace("{plages}", () => nombre(feuille.fini.plages));
      return feuille.fini.fermeture_confirmee ? fait : `${fait} ${t("feuille_fd9.fait.incertaine")}`;
    }
    if (feuille.etape === "mesure") {
      return t("feuille_fd9.etape.mesure").replace("{n}", () => nombre(feuille.mesurees))
        .replace("{total}", () => nombre(feuille.plages));
    }
    return t(`feuille_fd9.etape.${feuille.etape}`);
  }

  // Cause et action ; une clé inconnue de la page prend le texte général.
  function ecrireEchec(echec) {
    const cle = textes[`${echec.cle}.cause`] ? echec.cle : "feuille_fd9.autre";
    racine.querySelector("[data-feuille-cause]").textContent = t(`${cle}.cause`);
    racine.querySelector("[data-feuille-action]").textContent = t(`${cle}.action`);
    details.hidden = !echec.detail;
    racine.querySelector("[data-feuille-details-texte]").textContent = echec.detail || "";
  }

  function ecrire() {
    ecrireBoutons();
    ecrireEtat();
  }

  // ---- Mire et points mesurés, repliés tant qu'on ne les demande pas ----
  function ouvrirVue(ouverte) {
    vue.hidden = !ouverte;
    voir.setAttribute("aria-expanded", String(ouverte));
    if (ouverte) vueFeuille.ecrire();
  }
  voir.addEventListener("click", () => ouvrirVue(vue.hidden));

  async function montrerCalage(numero, calage, ancien) {
    const n = ++demandeCalage;
    let image = null;
    try {
      image = await invoke("feuille_fd9_image", { numero });
    } catch (erreur) {
      console.error("feuille_fd9_image", erreur);
    }
    if (n === demandeCalage) vueFeuille.calage(calage, image, ancien);
  }

  // Dernier calage gardé dans la résolution : l'onglet « Points mesurés » le montre.
  async function chargerDernierCalage(id) {
    const n = ++demandeCalage;
    let dernier = null;
    try {
      dernier = await invoke("feuille_fd9_dernier_calage", { condition: id });
    } catch (erreur) {
      console.error("feuille_fd9_dernier_calage", erreur);
    }
    if (n !== demandeCalage || condition !== id) return;
    if (dernier) await montrerCalage(dernier.numero, dernier.calage, true);
    else vueFeuille.calage(null, null, false);
  }

  async function chargerEtat() {
    try {
      etat = await invoke("feuille_fd9_etat");
    } catch (erreur) {
      console.error("feuille_fd9_etat", erreur);
      etat = { disponible: false, raisons: ["feuille_fd9.raison.occupe"], mire: null, demonstration: false };
    }
    if (etat.mire) vueFeuille.mire(etat.mire);
    ecrireBoutons();
  }

  // ---- La mesure : la commande dure le temps de la feuille, l'événement en dit l'avancement ----
  async function lancer() {
    const d = parc.reglage();
    if (!d || mesurer.disabled) return;
    feuille = { condition: d.condition.id, etape: "connexion", mesurees: 0, plages: 0, fini: null, echec: null };
    ouvrirVue(true);
    vueFeuille.montrer("mire");
    ecrire();
    annuler.focus();
    try {
      const r = await invoke("mesurer_feuille_fd9", { condition: feuille.condition });
      if (r.instrument && typeof recevoir === "function") {
        recevoir(r.instrument); // app.js : instrument actif et liste du poste
        afficherInstrument();
      }
      if (r.rangee) feuille.fini = r.rangee;
      else feuille.echec = r.echec || { cle: "feuille_fd9.autre", detail: null };
    } catch (cle) {
      // Refusée avant de commencer : une autre opération tient l'instrument.
      feuille.echec = { cle: "feuille_fd9.autre", detail: t(String(cle)) };
    }
    ecrire();
    mesurer.focus();
    await chargerEtat();
    if (feuille.fini && feuille.condition === condition) await parc.recharger();
  }
  mesurer.addEventListener("click", lancer);

  annuler.addEventListener("click", async () => {
    try {
      await invoke("annuler_feuille_fd9");
    } catch (erreur) {
      console.error("annuler_feuille_fd9", erreur);
    }
  });

  window.__TAURI__.event.listen("feuille_fd9", (evenement) => {
    const e = evenement.payload;
    if (!enCours() || e.condition !== feuille.condition) return;
    feuille.etape = e.etape;
    if (e.etape === "mesure") [feuille.mesurees, feuille.plages] = [e.mesurees, e.plages];
    if (e.etape === "calage_pret") {
      // Les points visés, montrés avant la mesure.
      ouvrirVue(true);
      vueFeuille.montrer("points");
      montrerCalage(e.numero, e.calage, false);
    }
    ecrire();
  });

  document.addEventListener("langue-appliquee", () => {
    ecrire();
    vueFeuille.ecrire();
  });

  // Une autre résolution : l'état de sa dernière feuille ; la feuille en cours garde le sien.
  function dessiner(d) {
    if (condition !== d.condition.id) {
      condition = d.condition.id;
      if (!enCours()) {
        feuille = null;
        chargerDernierCalage(condition);
      }
    }
    ecrire();
    chargerEtat();
  }

  return {
    brancher: (fonctions) => { parc = fonctions; },
    dessiner,
  };
})();
