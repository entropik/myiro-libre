// Bibliothèque en arbre (zone gauche) : imprimante → papier → résolution, avec l'âge du profil
// actif, la recherche, l'ajout en ligne et le groupe « Non classées ». Le choix d'une résolution
// ouvre sa vue au centre et à droite (reglage.js) ; celui d'une mesure, la feuille
// (bibliotheque.js). Les données viennent du module Rust `parc` ; les textes, du catalogue (app.js).
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const zone = document.querySelector("[data-zone='gauche']");
  const recherche = zone.querySelector("[data-recherche]");
  const arbre = zone.querySelector("[data-arbre]");
  const message = zone.querySelector("[data-arbre-message]");
  const ajout = zone.querySelector("[data-ajout]");
  const ajoutActions = zone.querySelector("[data-ajout-actions]");
  const nonClasseesBloc = zone.querySelector("[data-non-classees-bloc]");
  const nonClassees = zone.querySelector("[data-non-classees]");
  const vue = document.querySelector("[data-vue-reglage]");
  const feuille = document.querySelector("[data-feuille]");
  const details = document.querySelector("[data-details-reglage]");
  const ariane = document.querySelector("[data-ariane]");

  let parc = { imprimantes: [], non_classees: [] };
  let branches = []; // conditions et leurs mesures (pour « Non classées »)
  let erreur = null; // clé du catalogue si la bibliothèque ne répond pas
  let vide = false; // aucune imprimante, recherche à part
  let contexte = { imprimante: null, papier: null, condition: null }; // nœud choisi
  let reglage = null; // détail de la résolution choisie (module `parc`)
  let mesureChoisie = null; // "mesure-3", "importee-5" : mesure ouverte dans la feuille
  let formulaire = null; // ajout en cours : "imprimante", "papier" ou "resolution"
  let saisie = { nom: "", resolution: "" };
  let erreurAjout = null;
  const replies = new Set(); // imprimantes ("i3") et conditions ("c2") repliées

  // Pièces de l'arbre, sans état (arbre.js).
  const { el, noeud, age } = window.Arbre;
  const estChoisie = (cle) => mesureChoisie === cle;
  const lignesMesures = (b) => window.Arbre.lignesMesures(b, estChoisie);

  function repliable(li, cle) {
    const replie = replies.has(cle);
    window.Arbre.repliable(li, cle, replie);
    return replie;
  }

  function dessinerArbre() {
    const cherche = recherche.value.trim() !== "";
    arbre.replaceChildren();
    for (const bi of parc.imprimantes) {
      const i = bi.imprimante.id;
      const choisie = contexte.imprimante === i && contexte.papier === null && contexte.condition === null;
      const li = noeud("imprimante", bi.imprimante.nom, null, { imprimante: i }, choisie);
      const replie = repliable(li, "i" + i) && !cherche;
      if (replie) {
        const n = bi.papiers.length;
        li.append(el("span", "noeud__compte", `${n} ${t(n > 1 ? "parc.compte.papiers" : "parc.compte.papier")}`));
      }
      arbre.append(li);
      if (replie) continue;
      for (const bp of bi.papiers) {
        // Un papier à une seule résolution s'affiche sans ce niveau.
        const seul = bp.reglages.length === 1 ? bp.reglages[0] : null;
        const premier = bp.reglages[0];
        const donnees = { imprimante: i, papier: bp.papier.id };
        if (premier) donnees.condition = premier.condition.id;
        const choisi = seul ? contexte.condition === seul.condition.id
          : contexte.papier === bp.papier.id && contexte.condition === null;
        arbre.append(noeud("papier", bp.papier.nom, seul ? age(seul) : null, donnees, choisi));
        if (seul) continue;
        for (const r of bp.reglages) {
          arbre.append(noeud("resolution", r.resolution, age(r),
            { imprimante: i, papier: bp.papier.id, condition: r.condition.id }, contexte.condition === r.condition.id));
        }
      }
    }
    message.hidden = !erreur && parc.imprimantes.length > 0;
    if (erreur) message.textContent = t(erreur);
    else message.textContent = cherche ? t("bibliotheque.aucun_resultat") : t("parc.vide");

    // Conditions d'avant le parc, avec leurs mesures, à part.
    nonClassees.replaceChildren();
    for (const c of parc.non_classees) {
      const b = branches.find((x) => x.condition.id === c.id);
      const n = b ? b.mesures.length + b.importees.length : 0;
      const li = noeud("imprimante", c.nom, el("span", "noeud__compte", String(n)), { nonClassee: c.id },
        mesureChoisie === `condition-${c.id}`);
      const replie = repliable(li, "c" + c.id);
      nonClassees.append(li);
      if (!replie && b) nonClassees.append(...lignesMesures(b));
    }
    nonClasseesBloc.hidden = parc.non_classees.length === 0;
  }

  // ---- Ajout en ligne, sans fenêtre ----
  function trouver(condition) {
    for (const bi of parc.imprimantes) {
      for (const bp of bi.papiers) {
        if (bp.reglages.some((r) => r.condition.id === condition)) return { bi, bp };
      }
    }
    return null;
  }

  function nomImprimante(id) {
    const bi = parc.imprimantes.find((b) => b.imprimante.id === id);
    return bi ? bi.imprimante.nom : "";
  }

  function nomPapier(id) {
    for (const bi of parc.imprimantes) {
      const bp = bi.papiers.find((p) => p.papier.id === id);
      if (bp) return bp.papier.nom;
    }
    return "";
  }

  // Dernière résolution ajoutée sur cette imprimante, pour pré-remplir un nouveau papier.
  function derniereResolution(imprimante) {
    const bi = parc.imprimantes.find((b) => b.imprimante.id === imprimante);
    const reglages = bi ? bi.papiers.flatMap((p) => p.reglages) : [];
    reglages.sort((a, b) => b.condition.id - a.condition.id);
    return reglages.length ? reglages[0].resolution : "";
  }

  function ouvrirAjout(mode) {
    formulaire = mode;
    erreurAjout = null;
    saisie = { nom: "", resolution: mode === "papier" ? derniereResolution(contexte.imprimante) : "" };
    dessinerAjout();
  }

  function champ(cle, libelle) {
    const bloc = el("div", "field");
    const l = el("label", "label", t(libelle));
    l.htmlFor = "ajout-" + cle;
    const entree = el("input", erreurAjout ? "input input--error" : "input");
    entree.id = "ajout-" + cle;
    entree.autocomplete = "off";
    entree.value = saisie[cle];
    entree.addEventListener("input", () => { saisie[cle] = entree.value; });
    entree.addEventListener("keydown", (e) => {
      if (e.key === "Enter") valider();
      if (e.key === "Escape") { formulaire = null; dessinerAjout(); }
    });
    bloc.append(l, entree);
    return bloc;
  }

  function dessinerAjout() {
    for (const b of ajoutActions.querySelectorAll("[data-ajout-contexte]")) b.remove();
    if (contexte.imprimante !== null) {
      const b = el("button", "btn", t("parc.ajouter_papier"));
      b.dataset.ajoutContexte = "papier";
      ajoutActions.append(b);
    }
    if (contexte.papier !== null) {
      const b = el("button", "btn", t("parc.ajouter_resolution"));
      b.dataset.ajoutContexte = "resolution";
      ajoutActions.append(b);
    }
    ajout.replaceChildren();
    if (!formulaire) return;
    const bloc = el("div", "ajout");
    const pour = formulaire === "papier" ? nomImprimante(contexte.imprimante) : nomPapier(contexte.papier);
    bloc.append(el("span", "label label--ink", t("parc.ajout." + formulaire).replace("{}", pour)));
    if (formulaire !== "resolution") bloc.append(champ("nom", "parc.champ." + formulaire));
    if (formulaire !== "imprimante") bloc.append(champ("resolution", "parc.champ.resolution"));
    if (erreurAjout) bloc.append(el("p", "why", t(erreurAjout)));
    const actions = el("div", "actions");
    const valide = el("button", "btn", t("parc.ajouter"));
    valide.addEventListener("click", valider);
    const annule = el("button", "btn", t("parc.annuler"));
    annule.addEventListener("click", () => { formulaire = null; dessinerAjout(); });
    actions.append(valide, annule);
    bloc.append(actions);
    ajout.append(bloc);
    const premier = bloc.querySelector("input");
    if (premier) premier.focus();
  }

  async function valider() {
    try {
      if (formulaire === "imprimante") {
        const i = await invoke("parc_ajouter_imprimante", { nom: saisie.nom });
        // La nouvelle imprimante est choisie : on enchaîne avec son premier papier.
        contexte = { imprimante: i.id, papier: null, condition: null };
        reglage = null;
        recherche.value = "";
        await charger();
        ouvrirAjout("papier");
        return;
      }
      const condition = formulaire === "papier"
        ? await invoke("parc_ajouter_papier", { imprimante: contexte.imprimante, nom: saisie.nom, resolution: saisie.resolution })
        : await invoke("parc_ajouter_resolution", { papier: contexte.papier, resolution: saisie.resolution });
      formulaire = null;
      erreurAjout = null;
      recherche.value = "";
      await charger();
      await choisirReglage(condition.id);
      // Mesurer et l'import proposent aussi cette nouvelle condition d'impression ;
      // marqué « parc » pour que ce script ne recharge pas une seconde fois.
      document.dispatchEvent(new CustomEvent("bibliotheque-modifiee", { detail: "parc" }));
    } catch (cle) {
      erreurAjout = String(cle);
      dessinerAjout();
    }
  }

  function dessinerAriane() {
    if (tacheCourante() !== "bibliotheque") return;
    const racine = el("button", "", t("tache.bibliotheque"));
    racine.dataset.arianeRacine = "";
    const morceaux = [racine];
    if (contexte.imprimante !== null) morceaux.push(" / ", nomImprimante(contexte.imprimante));
    if (contexte.papier !== null) morceaux.push(" / ", el("b", "", nomPapier(contexte.papier)));
    ariane.replaceChildren(...morceaux);
    ariane.title = ariane.textContent;
  }

  function dessinerCentre() {
    // La comparaison (comparer.js) prend la place de la vue d'une résolution et de la feuille.
    if (window.Comparer.ouverte()) {
      vue.hidden = true;
      feuille.hidden = true;
      details.hidden = true;
      document.querySelector("[data-details-choix]").hidden = true;
      document.querySelector("[data-details-vide]").hidden = true;
      dessinerAriane();
      if (tacheCourante() === "bibliotheque") ariane.append(" / ", el("b", "", t("comparer.titre")));
      return;
    }
    const montre = Boolean(reglage) && reglage.condition.id === contexte.condition;
    vue.hidden = !montre;
    feuille.hidden = montre;
    details.hidden = !montre;
    if (montre) {
      window.Reglage.remplir(reglage, lignesMesures);
      document.querySelector("[data-details-choix]").hidden = true;
      document.querySelector("[data-details-vide]").hidden = true;
    } else if (window.Bibliotheque) {
      window.Bibliotheque.redessiner();
    }
    dessinerAriane();
  }

  function dessiner() {
    dessinerArbre();
    dessinerAjout();
    dessinerCentre();
  }

  async function charger() {
    try {
      parc = await invoke("parc", { recherche: recherche.value });
      branches = await invoke("bibliotheque_arborescence", { recherche: "" });
      erreur = null;
    } catch (cle) {
      parc = { imprimantes: [], non_classees: [] };
      branches = [];
      erreur = String(cle);
    }
    if (!erreur && !recherche.value.trim()) vide = parc.imprimantes.length === 0;
    dessiner();
  }

  async function rafraichir() {
    const condition = contexte.condition;
    if (condition === null) return;
    let d = null;
    try {
      d = await invoke("parc_detail_reglage", { condition });
    } catch (cle) {
      window.Reglage.annoncer(t(cle));
    }
    // Un autre choix fait entre-temps l'emporte : sa réponse arrivera aussi.
    if (condition !== contexte.condition) return;
    reglage = d;
    dessinerCentre();
  }

  async function choisirReglage(condition) {
    const place = trouver(condition);
    contexte = { imprimante: place ? place.bi.imprimante.id : null, papier: place ? place.bp.papier.id : null, condition };
    mesureChoisie = null;
    window.Reglage.annoncer("");
    afficherTache("bibliotheque"); // app.js : la vue de la bibliothèque, au centre
    await rafraichir();
    dessinerArbre();
    dessinerAjout();
  }

  // Quitte la résolution : la feuille des mesures reprend le centre.
  function quitterReglage() {
    window.Comparer.fermer();
    contexte = { imprimante: null, papier: null, condition: null };
    reglage = null;
    mesureChoisie = null;
    formulaire = null;
    dessiner();
  }

  // ---- Événements ----
  function choisirNoeud(li, cible) {
    const d = li.dataset;
    if (cible.dataset.replier) {
      const cle = cible.dataset.replier;
      if (replies.has(cle)) replies.delete(cle);
      else replies.add(cle);
      dessinerArbre();
      return;
    }
    // Un choix dans l'arbre ferme la comparaison : ce qui est choisi reprend le centre.
    window.Comparer.fermer();
    if (d.mesure || d.importee) {
      contexte = { imprimante: null, papier: null, condition: null };
      reglage = null;
      mesureChoisie = d.mesure ? `mesure-${d.mesure}` : `importee-${d.importee}`;
      dessiner();
      if (d.mesure) window.Bibliotheque.afficherMesure(Number(d.mesure));
      else window.Bibliotheque.afficherImportee(Number(d.importee));
      dessinerArbre();
      return;
    }
    if (d.nonClassee) {
      // Une condition d'avant le parc : sa feuille (nom, mesures), sans classement.
      contexte = { imprimante: null, papier: null, condition: null };
      reglage = null;
      mesureChoisie = `condition-${d.nonClassee}`;
      dessiner();
      window.Bibliotheque.afficherCondition(Number(d.nonClassee));
      return;
    }
    if (d.condition) {
      choisirReglage(Number(d.condition));
      return;
    }
    // Une imprimante (ou un papier sans résolution) : le contexte des ajouts, déplié.
    replies.delete("i" + d.imprimante);
    contexte = { imprimante: Number(d.imprimante), papier: d.papier ? Number(d.papier) : null, condition: null };
    reglage = null;
    mesureChoisie = null;
    afficherTache("bibliotheque");
    dessiner();
  }

  for (const liste of [arbre, nonClassees, vue.querySelector("[data-reglage-mesures]")]) {
    liste.addEventListener("click", (e) => {
      const li = e.target.closest("li");
      if (li) choisirNoeud(li, e.target);
    });
    liste.addEventListener("keydown", (e) => {
      const li = e.target.closest("li");
      if (li && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        choisirNoeud(li, li);
      }
    });
  }
  ariane.addEventListener("click", (e) => {
    if (e.target.closest("[data-ariane-racine]")) quitterReglage();
  });
  ajoutActions.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    if (b.hasAttribute("data-ajouter-imprimante")) ouvrirAjout("imprimante");
    else if (b.dataset.ajoutContexte) ouvrirAjout(b.dataset.ajoutContexte);
  });
  let attente;
  recherche.addEventListener("input", () => {
    clearTimeout(attente);
    attente = setTimeout(charger, 150);
  });
  document.addEventListener("tache-affichee", (e) => { if (e.detail === "bibliotheque") dessinerCentre(); });
  for (const evenement of ["comparer-ouverte", "comparer-fermee"]) document.addEventListener(evenement, dessinerCentre);
  // Une mesure rangée (Mesurer), importée ou restaurée : l'arbre et la vue suivent.
  for (const evenement of ["bibliotheque-modifiee", "parc-modifie"]) {
    document.addEventListener(evenement, async (e) => {
      if (e.detail === "parc") return; // déjà rechargé ici
      await charger();
      await rafraichir();
    });
  }

  let premiereFois = true;
  async function langueAppliquee() {
    recherche.placeholder = t("bibliotheque.recherche_aide");
    if (premiereFois) {
      premiereFois = false;
      await charger();
      return;
    }
    dessiner();
  }
  document.addEventListener("langue-appliquee", langueAppliquee);

  // La vue d'une résolution (reglage.js) agit sur la résolution choisie ici.
  window.Reglage.brancher({ reglage: () => reglage, rafraichir, recharger: charger });

  window.Parc = {
    recharger: charger,
    choisirReglage,
    quitterReglage,
    conditionChoisie: () => contexte.condition,
    estVide: () => vide && !erreur,
    ouvrirAjoutImprimante: () => ouvrirAjout("imprimante"),
  };
  // Le catalogue a pu arriver avant le chargement de ce script.
  if (Object.keys(textes).length > 0) langueAppliquee();
})();
