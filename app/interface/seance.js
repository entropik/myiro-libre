// Séance de mesure d'un tirage FD-9 (ticket #35), dans la vue d'une résolution : la section ouverte
// par « Mesurer le tirage ». Le tirage à mesurer, la fiche d'impression à confirmer (vide et non
// confirmée pour chaque nouveau tirage), puis les feuilles rangées (importées de FD-S2w, ou de
// complément), chacune avec son état, à inspecter (seance-plages.js), à écarter avec un motif ou à
// retenir ; ce que donne la moyenne (seance-apercu.js) ; enfin la fin de la séance, incomplète ou
// écarts vus par choix explicite, après quoi un tirage restant peut se commencer. Une séance en
// cours dans la résolution est dite sous les boutons de la vue. Tout s'écrit sur place, dans les
// blocs de taille gardée de index.html, et les anciennes valeurs restent jusqu'aux nouvelles. Un
// bouton inactif dit pourquoi, juste dessous. La mesure au FD-9 depuis la séance reste inactive,
// avec sa raison, en attendant son câblage dans feuille-fd9.js : jamais l'ancienne mesure d'une
// feuille de résolution à sa place. Les actions passent par le module Rust `seance_fd9` ; les
// textes, par le catalogue. Utilisé par reglage.js.
"use strict";

window.SeanceFd9 = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { date } = window.Arbre;
  const plages = window.SeancePlages;
  const apercu = window.SeanceApercu;

  const vue = document.querySelector("[data-vue-reglage]");
  const ouvrirBouton = vue.querySelector("[data-seance-ouvrir]");
  const reprendre = vue.querySelector("[data-seance-reprendre]");
  const racine = vue.querySelector("[data-seance]");
  const q = (selecteur) => racine.querySelector(selecteur);
  const etatLigne = q("[data-seance-etat]");
  const choixTirage = q("[data-seance-tirage]");
  const champsFiche = [...racine.querySelectorAll("[data-seance-fiche]")];
  const confirmee = q("[data-seance-confirmee]");
  const commencer = q("[data-seance-commencer]");
  const mesurer = q("[data-seance-mesurer]");
  const importer = q("[data-seance-importer]");
  const complement = q("[data-seance-complement]");
  const raison = q("[data-seance-raison]");
  const motif = q("[data-seance-motif]");
  const liste = q("[data-seance-sources]");
  const vide = q("[data-seance-sources-vide]");
  const modele = q("[data-seance-modele]");
  const incomplete = q("[data-seance-incomplete]");
  const ecartsVus = q("[data-seance-ecarts-vus]");
  const terminer = q("[data-seance-terminer]");
  const raisonFin = q("[data-seance-raison-fin]");
  const message = q("[data-seance-message]");

  // Ce que reglage.js prête : la résolution affichée, de quoi la recharger, et le texte d'une erreur.
  let parc = { reglage: () => null, recharger: async () => {}, texteErreur: (e) => String(e) };
  let condition = null; // résolution affichée
  let tirages = []; // tirages FD-9 pas encore mesurés de la résolution
  let seance = null; // numéro de la séance montrée
  let etat = null; // réponse de `seance_etat` : celle de la séance montrée, ou la précédente
  let chargement = false; // l'état d'une autre séance est demandé : l'ancien reste montré
  let occupe = false;
  let texteMessage = "";
  let demande = 0; // le dernier état demandé l'emporte
  let ficheDe = null; // séance dont la fiche figée est dans les champs
  const lignes = []; // lignes des feuilles, créées au besoin puis gardées

  const libre = () => !occupe && !chargement;
  const enCours = () => Boolean(etat && etat.id === seance && etat.etat === "en_cours");

  // ---- La fiche d'impression (mire de profilage : sans gestion des couleurs) ----
  function fiche() {
    const f = { impression: "sans_gestion", confirmee: confirmee.checked };
    for (const champ of champsFiche) f[champ.dataset.seanceFiche] = champ.value.trim();
    return f;
  }

  // Une fiche vide et non confirmée : chaque tirage se confirme pour lui-même.
  function viderFiche() {
    for (const champ of champsFiche) champ.value = "";
    confirmee.checked = false;
    ficheDe = null;
  }

  // La fiche figée de la séance montrée remplace la saisie, et ne se modifie plus.
  function ecrireFiche() {
    if (etat && etat.id === seance && ficheDe !== etat.id) {
      try {
        const figee = JSON.parse(etat.plan.fiche);
        for (const champ of champsFiche) champ.value = figee[champ.dataset.seanceFiche] || "";
        confirmee.checked = Boolean(figee.confirmee);
        ficheDe = etat.id;
      } catch (erreur) {
        console.error("fiche figée", erreur);
      }
    }
    for (const champ of champsFiche) champ.disabled = occupe || seance !== null;
    confirmee.disabled = occupe || seance !== null;
  }

  // ---- Le tirage : liste gardée, choix gardé tant qu'il existe ----
  function ecrireTirages() {
    const options = tirages.length
      ? tirages.map((r) => {
        const texte = t("seance.tirage.option").replace("{nom}", () => r.nom)
          .replace("{date}", () => date(r.horodatage));
        return { valeur: String(r.id), texte: r.seance ? `${texte} ${t("seance.tirage.en_cours")}` : texte };
      })
      : [{ valeur: "", texte: t("seance.tirage.aucun") }];
    const deLaSeance = tirages.find((r) => r.seance !== null && r.seance === seance);
    const choisi = deLaSeance ? String(deLaSeance.id) : choixTirage.value;
    const actuelles = [...choixTirage.options].map((o) => o.value + o.text).join();
    if (options.map((o) => o.valeur + o.texte).join() !== actuelles) {
      choixTirage.length = 0;
      for (const o of options) choixTirage.add(new Option(o.texte, o.valeur));
    }
    if (options.some((o) => o.valeur === choisi)) choixTirage.value = choisi;
    choixTirage.disabled = occupe || !tirages.length;
  }

  choixTirage.addEventListener("change", () => {
    const r = tirages.find((x) => String(x.id) === choixTirage.value);
    if (r && r.seance !== null) return montrerSeance(r.seance);
    viderFiche();
    return montrerSeance(null);
  });

  // ---- Boutons et raisons : un bouton inactif dit pourquoi, juste dessous ----
  function raisonCommune() {
    if (occupe) return "seance.raison.en_cours";
    if (chargement) return "seance.raison.chargement";
    if (seance === null) return "seance.raison.sans_seance";
    return enCours() ? null : "seance.raison.fermee";
  }

  function ecrireBoutons() {
    const tirage = tirages.find((x) => String(x.id) === choixTirage.value);
    const ouverte = libre() && enCours();
    commencer.disabled = !libre() || seance !== null || !tirage || !confirmee.checked;
    // En attente du câblage de la mesure dans feuille-fd9.js (raison dite ci-dessous).
    mesurer.disabled = true;
    for (const b of [importer, complement, motif, incomplete, ecartsVus]) b.disabled = !ouverte;
    terminer.disabled = !ouverte || !etat.sources.length;
    const raisons = [];
    const commune = raisonCommune();
    if (seance === null && libre()) {
      if (!tirage) raisons.push("seance.tirage.aucun");
      else if (!confirmee.checked) raisons.push("seance.raison.fiche");
    } else if (seance !== null && libre() && enCours()) {
      raisons.push("seance.raison.deja_commencee");
    }
    if (commune) raisons.push(commune);
    raisons.push("seance.raison.mesure_a_venir");
    raison.textContent = raisons.map((c) => t(c)).join(" ");
    raison.title = raison.textContent; // coupée à deux lignes, lisible en entier au survol
    const fin = commune || (etat && !etat.sources.length ? "seance.raison.aucune_feuille" : null);
    raisonFin.textContent = fin ? t(fin) : "";
    raisonFin.title = raisonFin.textContent;
  }

  // ---- Une ligne par feuille rangée ; les lignes sont gardées et réécrites ----
  function etatSource(s) {
    if (!s.retenue) {
      let pourquoi = "";
      try {
        pourquoi = JSON.parse(s.selection || "{}").motif || "";
      } catch (erreur) {
        console.error("choix de l'opérateur", erreur);
      }
      return t("seance.source.ecartee").replace("{motif}", () => pourquoi);
    }
    if (s.association.doutes.length) return t("seance.source.douteuse");
    if (s.association.manquantes.length) {
      return t("seance.source.incomplete").replace("{n}", () => String(s.association.manquantes.length));
    }
    return t("seance.source.associee");
  }

  function ligneSource(i) {
    if (!lignes[i]) {
      const ligne = modele.content.firstElementChild.cloneNode(true);
      const source = () => etat.sources.find((s) => String(s.mesure) === ligne.dataset.mesure);
      const inspecter = ligne.querySelector("[data-inspecter]");
      inspecter.addEventListener("click", () => {
        const s = source();
        if (s) plages.ouvrir(s.mesure, s.fichier, inspecter);
      });
      ligne.querySelector("[data-retenir]").addEventListener("click", () => {
        const s = source();
        if (s) retenir(s);
      });
      liste.append(ligne);
      lignes[i] = ligne;
    }
    return lignes[i];
  }

  function ecrireSources() {
    const sources = etat ? etat.sources : [];
    const pourquoi = raisonCommune();
    sources.forEach((s, i) => {
      const ligne = ligneSource(i);
      ligne.dataset.mesure = String(s.mesure);
      const origine = s.role === "complement" ? "seance.source.complement" : `seance.source.${s.origine}`;
      const texte = [s.fichier, t(origine), etatSource(s)].join(" · ");
      const libelle = ligne.querySelector("[data-texte]");
      libelle.textContent = texte;
      libelle.title = texte;
      const inspecter = ligne.querySelector("[data-inspecter]");
      inspecter.textContent = t("seance.inspecter");
      inspecter.disabled = !libre();
      inspecter.title = inspecter.disabled ? t(pourquoi) : "";
      const bouton = ligne.querySelector("[data-retenir]");
      bouton.textContent = t(s.retenue ? "seance.ecarter" : "seance.retenir");
      // Une mesure de complément ne s'écarte pas : le bouton garde sa place, invisible.
      bouton.toggleAttribute("data-vide", s.role === "complement");
      bouton.disabled = !libre() || !enCours() || s.role === "complement";
      bouton.title = bouton.disabled && pourquoi ? t(pourquoi) : "";
      ligne.hidden = false;
    });
    lignes.slice(sources.length).forEach((l) => { l.hidden = true; });
    vide.hidden = sources.length > 0;
  }

  function ecrireEtat() {
    let texte = t("seance.etat.aucune");
    if (etat && seance !== null) {
      const quand = etat.fin || etat.horodatage;
      texte = t(`seance.etat.${etat.etat}`).replace("{date}", () => date(quand))
        .replace("{n}", () => String(etat.sources.length));
    }
    etatLigne.textContent = texte;
    etatLigne.title = texte;
  }

  function ecrire() {
    ouvrirBouton.setAttribute("aria-expanded", String(!racine.hidden));
    reprendre.textContent = tirages.some((r) => r.seance !== null) ? t("seance.reprendre") : "";
    ecrireEtat();
    ecrireTirages();
    ecrireFiche();
    ecrireBoutons();
    ecrireSources();
    apercu.ecrire(seance === null ? null : etat, parc.texteErreur);
    message.textContent = texteMessage;
    message.title = texteMessage;
    plages.ecrire();
  }

  // ---- L'état de la séance, relu après chaque action ----
  async function chargerEtat() {
    if (seance === null) return;
    const n = ++demande;
    try {
      const r = await invoke("seance_etat", { seance });
      if (n === demande) etat = r;
    } catch (erreur) {
      if (n === demande) texteMessage = parc.texteErreur(erreur);
    }
    if (n === demande) chargement = false;
    ecrire();
  }

  function montrerSeance(id) {
    if (id !== seance) {
      seance = id;
      texteMessage = "";
      plages.fermer();
      if (id === null) etat = null; // rien à attendre
      else chargement = true; // l'ancien état reste montré jusqu'au nouveau
    }
    ecrire();
    return chargerEtat();
  }

  // Une action à la fois ; son message s'écrit à sa place gardée, sous « Terminer la séance ».
  async function agir(faire) {
    if (!libre()) return;
    occupe = true;
    texteMessage = "";
    ecrire();
    try {
      await faire();
    } catch (erreur) {
      texteMessage = parc.texteErreur(erreur);
    }
    occupe = false;
    await chargerEtat();
    ecrire();
  }

  commencer.addEventListener("click", () => agir(async () => {
    const r = await invoke("seance_commencer", { condition, tirage: Number(choixTirage.value), fiche: fiche() });
    seance = r.seance;
    texteMessage = t(r.reprise ? "seance.reprise" : "seance.commencee");
    await parc.recharger();
  }));

  const importerFichier = (role) => agir(async () => {
    const r = await invoke("seance_importer", { seance, role, filtre: t("seance.filtre") });
    if (r === null) return; // fenêtre annulée : rien ne change, aucun message
    texteMessage = t("seance.importee").replace("{fichier}", () => r.fichier);
    await parc.recharger();
  });
  importer.addEventListener("click", () => importerFichier("exemplaire"));
  complement.addEventListener("click", () => importerFichier("complement"));

  function retenir(s) {
    agir(async () => {
      await invoke("seance_retenir", { seance, mesure: s.mesure, retenue: !s.retenue, motif: motif.value });
      if (s.retenue) motif.value = "";
    });
  }

  // Après la fin : la séance et la fiche sont remises à vide, un tirage restant est choisi, et le
  // message gardé dit la fin avec son résultat.
  terminer.addEventListener("click", () => agir(async () => {
    const resultat = await invoke("seance_terminer", {
      seance, fin: { incomplet_assume: incomplete.checked, ecarts_vus: ecartsVus.checked },
    });
    const close = await invoke("seance_etat", { seance });
    seance = null;
    etat = null;
    viderFiche();
    incomplete.checked = false;
    ecartsVus.checked = false;
    plages.fermer();
    await parc.recharger();
    const d = parc.reglage();
    const rangee = d && d.importees.find((m) => m.id === resultat);
    const cle = close.etat === "incomplete" ? "seance.incomplete.resultat" : "seance.terminee.resultat";
    texteMessage = t(cle).replace("{fichier}", () => (rangee ? rangee.fichier : String(resultat)));
  }));

  confirmee.addEventListener("change", ecrireBoutons);
  for (const champ of champsFiche) champ.addEventListener("input", () => { confirmee.checked = false; ecrireBoutons(); });

  // La section s'ouvre et se ferme sous les boutons de la vue ; elle garde son contenu.
  ouvrirBouton.addEventListener("click", () => {
    racine.hidden = !racine.hidden;
    ecrire();
  });

  document.addEventListener("langue-appliquee", ecrire);

  plages.brancher({
    id: () => seance,
    etat: () => (seance === null ? null : etat),
    recharger: chargerEtat,
    texteErreur: (e) => parc.texteErreur(e),
  });

  // Une autre résolution : fiche vide, et sa séance en cours s'il y en a une ; la même : son état
  // relu.
  function dessiner(d) {
    tirages = d.tirages.filter((r) => r.instrument === "fd9" && !r.mesure);
    if (condition !== d.condition.id) {
      condition = d.condition.id;
      texteMessage = "";
      viderFiche();
      incomplete.checked = false;
      ecartsVus.checked = false;
      const enCoursIci = tirages.find((r) => r.seance !== null);
      return montrerSeance(enCoursIci ? enCoursIci.seance : null);
    }
    return montrerSeance(seance);
  }

  return {
    brancher: (fonctions) => { parc = fonctions; },
    dessiner,
  };
})();
