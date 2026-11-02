// Séance de mesure d'un tirage FD-9 (ticket #35), dans la vue d'une résolution : la section ouverte
// par « Mesurer le tirage ». Le tirage à mesurer, la fiche d'impression à confirmer, puis les
// feuilles rangées (importées de FD-S2w, ou de complément), chacune avec son état, à inspecter
// (seance-plages.js), à écarter avec un motif ou à retenir ; ce que donnerait la moyenne ; enfin
// la fin de la séance, incomplète ou écarts vus par choix explicite. Une séance en cours dans la
// résolution est dite sous les boutons de la vue. Tout s'écrit sur place, dans les blocs de taille
// gardée de index.html : une action ne redessine pas la section et ne fait pas bouger la page.
// La mesure au FD-9 depuis la séance reste inactive, avec sa raison, en attendant son câblage dans
// feuille-fd9.js : jamais l'ancienne mesure d'une feuille de résolution à sa place. Les actions
// passent par le module Rust `seance_fd9` ; les textes, par le catalogue. Utilisé par reglage.js.
"use strict";

window.SeanceFd9 = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { date } = window.Arbre;
  const plages = window.SeancePlages;

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
  const apercu = (nom) => q(`[data-seance-apercu="${nom}"]`);
  const refus = q("[data-seance-refus]");
  const incomplete = q("[data-seance-incomplete]");
  const ecartsVus = q("[data-seance-ecarts-vus]");
  const terminer = q("[data-seance-terminer]");
  const message = q("[data-seance-message]");

  // Ce que reglage.js prête : la résolution affichée, de quoi la recharger, et le texte d'une erreur.
  let parc = { reglage: () => null, recharger: async () => {}, texteErreur: (e) => String(e) };
  let condition = null; // résolution affichée
  let tirages = []; // tirages FD-9 pas encore mesurés de la résolution
  let seance = null; // numéro de la séance montrée
  let etat = null; // réponse de `seance_etat`
  let occupe = false;
  let texteMessage = "";
  let demande = 0; // le dernier état demandé l'emporte
  const lignes = []; // lignes des feuilles, créées au besoin puis gardées

  const enCours = () => Boolean(etat && etat.etat === "en_cours");
  const decimal = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v);

  // ---- Lire et écrire la fiche d'impression (mire de profilage : sans gestion des couleurs) ----
  function fiche() {
    const f = { impression: "sans_gestion", confirmee: confirmee.checked };
    for (const champ of champsFiche) f[champ.dataset.seanceFiche] = champ.value.trim();
    return f;
  }

  // La fiche figée de la séance montrée remplace la saisie, et ne se modifie plus.
  function ecrireFiche() {
    let figee = null;
    try {
      figee = etat ? JSON.parse(etat.plan.fiche) : null;
    } catch (erreur) {
      console.error("fiche figée", erreur);
    }
    for (const champ of champsFiche) {
      if (figee) champ.value = figee[champ.dataset.seanceFiche] || "";
      champ.disabled = occupe || seance !== null;
    }
    if (figee) confirmee.checked = Boolean(figee.confirmee);
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
    const choisi = etat ? String(etat.tirage) : choixTirage.value;
    if (options.map((o) => o.valeur + o.texte).join() !== [...choixTirage.options].map((o) => o.value + o.text).join()) {
      choixTirage.length = 0;
      for (const o of options) choixTirage.add(new Option(o.texte, o.valeur));
    }
    if (options.some((o) => o.valeur === choisi)) choixTirage.value = choisi;
    choixTirage.disabled = occupe || !tirages.length;
  }

  choixTirage.addEventListener("change", () => {
    const r = tirages.find((x) => String(x.id) === choixTirage.value);
    montrerSeance(r && r.seance !== null ? r.seance : null);
  });

  // ---- Boutons et raison : un bouton inactif dit pourquoi, juste dessous ----
  function ecrireBoutons() {
    const tirage = tirages.find((x) => String(x.id) === choixTirage.value);
    commencer.disabled = occupe || seance !== null || !tirage || !confirmee.checked;
    // En attente du câblage de la mesure dans feuille-fd9.js (raison dite ci-dessous).
    mesurer.disabled = true;
    importer.disabled = occupe || !enCours();
    complement.disabled = occupe || !enCours();
    motif.disabled = occupe || !enCours();
    incomplete.disabled = occupe || !enCours();
    ecartsVus.disabled = occupe || !enCours();
    terminer.disabled = occupe || !enCours() || !etat.sources.length;
    const raisons = [];
    if (occupe) raisons.push("seance.raison.en_cours");
    else if (seance === null && !tirage) raisons.push("seance.tirage.aucun");
    else if (seance === null && !confirmee.checked) raisons.push("seance.raison.fiche");
    else if (seance !== null && !enCours()) raisons.push("seance.raison.fermee");
    raisons.push("seance.raison.mesure_a_venir");
    raison.textContent = raisons.map((c) => t(c)).join(" ");
    raison.title = raison.textContent; // coupée à deux lignes, lisible en entier au survol
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
      ligne.querySelector("[data-inspecter]").addEventListener("click", () => {
        const s = source();
        if (s) plages.ouvrir(s.mesure, s.fichier);
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
      inspecter.disabled = occupe;
      const bouton = ligne.querySelector("[data-retenir]");
      bouton.textContent = t(s.retenue ? "seance.ecarter" : "seance.retenir");
      // Une mesure de complément ne s'écarte pas : le bouton garde sa place, invisible.
      bouton.toggleAttribute("data-vide", s.role === "complement");
      bouton.disabled = occupe || !enCours() || s.role === "complement";
      ligne.hidden = false;
    });
    lignes.slice(sources.length).forEach((l) => { l.hidden = true; });
    vide.hidden = sources.length > 0;
  }

  // ---- Ce que donnerait la moyenne, ou ce qu'elle a donné ----
  function ecrireApercu() {
    const a = etat && (etat.apercu || etat.composition);
    const valeur = (nom, texte) => {
      apercu(nom).textContent = texte;
      apercu(nom).title = texte;
    };
    if (!a) {
      for (const nom of ["plages", "manquantes", "conditions", "ecarts"]) valeur(nom, "");
      refus.textContent = etat && etat.refus ? parc.texteErreur(etat.refus) : t("seance.apercu.vide");
      return;
    }
    const total = etat.plan.plages.length;
    const moyennees = etat.apercu ? a.plages : a.composition.length;
    valeur("plages", `${moyennees} / ${total}`);
    valeur("manquantes", a.manquantes.length
      ? `${a.manquantes.length} (${a.manquantes.slice(0, 8).join(", ")}${a.manquantes.length > 8 ? "…" : ""})`
      : t("seance.apercu.aucune"));
    const conditions = [...new Set(a.conditions_ecartees.map((c) => c.emplacement.toUpperCase()))];
    valeur("conditions", conditions.length ? conditions.join(", ") : t("seance.apercu.aucune"));
    const signales = a.ecarts.filter((e) => e.signale);
    const max = Math.max(0, ...a.ecarts.map((e) => e.de00 || 0));
    valeur("ecarts", t("seance.apercu.ecarts.valeur").replace("{n}", () => String(signales.length))
      .replace("{max}", () => decimal(max)));
    refus.textContent = "";
  }

  function ecrireEtat() {
    let texte = t("seance.etat.aucune");
    if (etat) {
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
    ecrireApercu();
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
    ecrire();
  }

  function montrerSeance(id) {
    if (id !== seance) {
      seance = id;
      etat = null;
      texteMessage = "";
      plages.fermer();
    }
    ecrire();
    return chargerEtat();
  }

  // Une action à la fois ; son message s'écrit à sa place gardée, sous « Terminer la séance ».
  async function agir(faire) {
    if (occupe) return;
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
    const r = await invoke("seance_commencer", {
      condition, tirage: Number(choixTirage.value), fiche: fiche(),
    });
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

  terminer.addEventListener("click", () => agir(async () => {
    await invoke("seance_terminer", {
      seance, fin: { incomplet_assume: incomplete.checked, ecarts_vus: ecartsVus.checked },
    });
    texteMessage = t("seance.terminee");
    await parc.recharger();
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
    etat: () => etat,
    recharger: chargerEtat,
    texteErreur: (e) => parc.texteErreur(e),
  });

  // Une autre résolution : sa séance en cours, s'il y en a une ; la même : son état relu.
  function dessiner(d) {
    tirages = d.tirages.filter((r) => r.instrument === "fd9" && !r.mesure);
    if (condition !== d.condition.id) {
      condition = d.condition.id;
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
