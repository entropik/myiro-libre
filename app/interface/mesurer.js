// Tâche Mesurer : mesure ponctuelle au centre (un seul bouton principal, les mesures de la
// séance empilées en dessous, noms modifiables), valeurs et provenance de la mesure choisie à
// droite. Une mesure peut être la couleur de référence : les autres montrent leur écart et leur
// verdict, calculés en Rust (module `mesurer`, crate `colorimetrie`). Mesures, valeurs et état de l'instrument viennent des modules Rust `mesurer` et
// `instrument` ; la page ne calcule rien. Les textes viennent du catalogue (chargé par app.js).
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const feuille = document.querySelector("[data-vue='mesurer']");
  const choixCondition = feuille.querySelector("[data-mesurer-condition]");
  const consigne = feuille.querySelector("[data-mesurer-consigne]");
  const choixDeclenchement = feuille.querySelector("[data-mesurer-declenchement]");
  const avis = feuille.querySelector("[data-mesurer-avis]");
  const bouton = feuille.querySelector("[data-mesurer]");
  const reessayer = feuille.querySelector("[data-mesurer-reessayer]");
  const raisonTexte = feuille.querySelector("[data-mesurer-raison]");
  const enCoursTexte = feuille.querySelector("[data-mesurer-en-cours]");
  const detailsTechniques = feuille.querySelector("[data-mesurer-details-techniques]");
  const vide = feuille.querySelector("[data-mesurer-vide]");
  const liste = feuille.querySelector("[data-mesurer-liste]");
  const lignes = feuille.querySelector("[data-mesurer-lignes]");
  const erreurListe = feuille.querySelector("[data-mesurer-erreur]");
  const rangerActions = feuille.querySelector("[data-mesurer-ranger-actions]");
  const rangerBouton = feuille.querySelector("[data-mesurer-ranger]");
  const detail = document.querySelector("[data-mesurer-detail]");

  let conditions = []; // conditions d'impression de la bibliothèque
  let fiches = []; // mesures de la séance, la plus récente d'abord
  let choisie = null; // numéro de la mesure montrée à droite
  let spectreChoisi = 0; // spectre 1, 2 ou 3 de la plage
  let enCours = false; // la commande `mesurer` n'a pas encore répondu
  let erreur = null; // clé du catalogue d'un refus de la commande
  let reference = null; // couleur de référence de la séance : numéro, nom, seuil écrit
  let erreurReference = null; // clé du catalogue d'un refus sur la référence ou son seuil
  let version = "";
  // « Mesure : automatique / manuelle », retenu par le module Rust `mesurer` d'une fois sur l'autre.
  let declenchement = "automatique";

  const langue = () => document.documentElement.lang || "fr";

  function el(nom, classe, texte) {
    const e = document.createElement(nom);
    if (classe) e.className = classe;
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  // Date et heure telles que le pont les a écrites, sans conversion de fuseau.
  function date(horodatage) {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(horodatage);
    if (!m) return horodatage;
    const d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]));
    return new Intl.DateTimeFormat(langue(), {
      day: "numeric", month: "short", year: "numeric",
      hour: "2-digit", minute: "2-digit", timeZone: "UTC",
    }).format(d);
  }

  // Condition de mesure du spectre n° i, telle que le pont l'a dite : jamais devinée.
  function conditionSpectre(fiche, i) {
    const info = fiche.spectres[i].condition;
    if (info.statut === "inconnue") return `${i + 1} · ${t("cartouche.inconnue")}`;
    return info.statut === "supposee" ? `${info.valeur} (${t("details.a_confirmer")})` : info.valeur;
  }

  function etalonnage(info) {
    if (info.statut === "inconnue") return t("cartouche.inconnu");
    return info.statut === "supposee" ? `${date(info.valeur)} (${t("details.a_confirmer")})` : date(info.valeur);
  }

  // Écran rendu par une commande du module `mesurer` : mesures et couleur de référence.
  function recevoirEcran(r) {
    fiches = r.mesures;
    reference = r.reference;
  }

  // Refus d'une commande : un code `mesurer.erreur.*` a sa cause et son action.
  function refus(cle) {
    const cause = textes[cle + ".cause"];
    return cause ? `${cause} ${textes[cle + ".action"]}` : t(cle);
  }

  // ---- Raison d'un bouton « Mesurer » inactif ; null s'il est actif ----
  function raison() {
    const v = vueInstrument;
    if (occupe) return t("raison.recherche");
    if (!v || !v.modele) return t("raison.instrument");
    if (v.etat === "etalonnage_requis") return t("raison.etalonnage");
    // FD-9 détecté : son pont s'arrête à la détection, la mesure viendra plus tard.
    if (v.etat === "detecte") return t("raison.bientot");
    if (!v.mesurable) {
      return v.probleme && v.probleme.code === "repos_incertain" ? t("mesurer.raison.repos") : t("raison.instrument");
    }
    if (conditions.length === 0) return t("mesurer.raison.aucune_condition");
    return null;
  }

  // ---- Feuille du centre ----
  function dessinerAvis() {
    const probleme = vueInstrument && vueInstrument.probleme;
    const concerne = Boolean(probleme) && probleme.ecran === "mesure" && !enCours;
    avis.hidden = !concerne;
    reessayer.hidden = !(concerne && probleme.code === "repos_incertain");
    detailsTechniques.hidden = !(concerne && probleme.detail);
    if (concerne) {
      avis.querySelector("[data-avis-cause]").textContent = textes["probleme." + probleme.code + ".cause"];
      avis.querySelector("[data-avis-action]").textContent = textes["probleme." + probleme.code + ".action"];
      detailsTechniques.querySelector("[data-details-texte]").textContent = probleme.detail || "";
    }
  }

  function dessinerListe() {
    vide.hidden = fiches.length > 0;
    liste.hidden = fiches.length === 0;
    lignes.replaceChildren();
    for (const f of fiches) {
      const tr = el("tr", f.numero === choisie ? "hl" : "");
      const nom = el("input", "input");
      nom.value = f.nom;
      nom.title = f.nom; // nom entier au survol, s'il finit par des points
      nom.autocomplete = "off";
      nom.setAttribute("aria-label", t("details.nom"));
      nom.addEventListener("change", () => renommer(f.numero, nom));
      nom.addEventListener("keydown", (e) => { if (e.key === "Enter") nom.blur(); });
      const nomCellule = el("td");
      nomCellule.append(nom);
      const range = f.erreur_rangement ? `${f.condition_impression} · ${t("mesurer.non_rangee")}` : f.condition_impression;
      // Petit carré de la couleur, devant le nom (même spectre que le détail).
      const v = f.spectres[spectreChoisi].valeurs;
      const couleur = el("td", "id num");
      if (v) {
        const carre = el("span", "swatch");
        carre.style.setProperty("--c", v.ecran);
        couleur.append(carre);
      }
      const [ecartCellule, verdictCellule] = ecartEnListe(f);
      tr.append(el("td", "id num", String(f.numero)), couleur, nomCellule, ecartCellule, verdictCellule, el("td", "num", date(f.horodatage)), el("td", "", range));
      tr.addEventListener("click", (e) => {
        if (e.target === nom) return;
        choisie = f.numero;
        erreurReference = null;
        dessiner();
      });
      lignes.append(tr);
    }
    const nonRangee = fiches.find((f) => f.erreur_rangement);
    erreurListe.hidden = !nonRangee;
    erreurListe.textContent = nonRangee ? textes[nonRangee.erreur_rangement] : "";
    rangerActions.hidden = !nonRangee;
    rangerBouton.disabled = enCours;
  }

  // ---- Couleur de référence et écart ----
  const CLASSES_VERDICT = { conforme: "ok", proche_de_la_limite: "warn", hors_tolerance: "bad" };
  const SYMBOLES = { ok: "M4 12.5l5 5L20 6.5", warn: "M12 5v9M12 17.5v2", bad: "M6 6l12 12M18 6L6 18" };

  // Petit état (carré et mot) : jamais la couleur seule.
  function etat(e) {
    if (e.verdict === "seuil_non_fixe") return el("span", "state state--none", t("mesurer.reference.seuil_non_fixe"));
    const classe = CLASSES_VERDICT[e.verdict];
    if (!classe) return el("span", "why", textes["verdict." + e.verdict]);
    return el("span", "state state--" + classe, textes["verdict." + e.verdict]);
  }
  function avecCarre(span) {
    if (span.classList.contains("state")) span.prepend(el("i"));
    return span;
  }

  function ecartEnListe(f) {
    const ecartCellule = el("td", "r num");
    const verdictCellule = el("td");
    if (f.reference) {
      verdictCellule.append(el("span", "label label--ink", t("mesurer.reference.marque")));
    } else {
      const e = f.spectres[spectreChoisi].ecart;
      if (e) {
        if (e.verdict !== "non_comparable") ecartCellule.textContent = e.delta_e00 || t("cartouche.inconnu");
        verdictCellule.append(avecCarre(etat(e)));
      }
    }
    return [ecartCellule, verdictCellule];
  }

  async function commandeReference(commande, arguments_) {
    try {
      recevoirEcran(await invoke(commande, { ...arguments_, langue: langue() }));
      erreurReference = null;
      document.dispatchEvent(new CustomEvent("bibliotheque-modifiee"));
    } catch (cle) {
      erreurReference = cle;
    }
    dessiner();
  }

  // Le verdict en grand, doublé d'un carré à symbole, sur une seule ligne ; sans seuil, aucun
  // verdict inventé.
  function blocVerdict(e) {
    const bloc = el("div", "verdict");
    const classe = CLASSES_VERDICT[e.verdict];
    if (!classe) {
      const p = el("p", "prose");
      p.append(avecCarre(etat(e)));
      bloc.append(p);
      // Conditions différentes : l'avis qui suit dit pourquoi, sans chiffre ni verdict.
      if (e.verdict === "seuil_non_fixe") bloc.append(el("p", "why", t("mesurer.reference.seuil_non_fixe_texte")));
      else if (e.verdict === "inconnu") bloc.append(el("p", "why", t("mesurer.verdict.inconnu")));
      return bloc;
    }
    bloc.classList.add("verdict--" + classe);
    const ligne = el("div", "verdict__row");
    const marque = el("span", classe === "ok" ? "mark" : "mark mark--" + classe);
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    const trace = document.createElementNS("http://www.w3.org/2000/svg", "path");
    trace.setAttribute("d", SYMBOLES[classe]);
    svg.append(trace);
    marque.append(svg);
    ligne.append(marque, el("h2", "verdict__word verdict__word--m", textes["verdict." + e.verdict]));
    bloc.append(ligne);
    return bloc;
  }

  // En tête des détails, compact et titré : l'écart de la mesure choisie à la référence. Rien
  // pour la référence elle-même, ni sans référence.
  function blocEcart(f) {
    const e = f.spectres[spectreChoisi].ecart;
    if (!reference || f.reference || !e) return [];
    const bloc = el("div");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("mesurer.ecart.titre")), el("span", "label", reference.nom));
    bloc.append(tete, blocVerdict(e));
    if (e.comparaison !== "meme_condition") {
      const avis = el("div", "notice notice--warn");
      avis.append(el("p", "", t(e.comparaison === "condition_differente" ? "mesurer.ecart.condition_differente" : "mesurer.ecart.non_confirmee")));
      bloc.append(avis);
    }
    // Sans écart (conditions différentes), pas de tableau de chiffres vides.
    if (e.verdict !== "non_comparable") {
      const inconnu = t("cartouche.inconnu");
      const corps = el("tbody");
      for (const [libelle, valeur, fort] of [
        [t("mesurer.ecart.de00"), e.delta_e00 || inconnu, true],
        [t("mesurer.ecart.dc"), e.delta_c || inconnu],
        [t("mesurer.ecart.dh"), e.delta_h || inconnu],
        [t("mesurer.ecart.accepte"), reference.seuil || t("mesurer.reference.seuil_non_fixe")],
      ]) {
        const tr = el("tr");
        const td = el("td", "r num");
        td.append(fort ? el("b", "", valeur) : document.createTextNode(valeur));
        tr.append(el("td", "", libelle), td);
        corps.append(tr);
      }
      const table = el("table");
      table.append(corps);
      bloc.append(table);
    }
    return [bloc];
  }

  // Après la provenance, titré : désigner, régler ou retirer la couleur de référence.
  function blocReference(f) {
    const bloc = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("mesurer.reference.titre")));
    bloc.append(tete);
    const actions = el("div", "actions");
    if (f.reference) {
      const champ = el("div", "field");
      const libelle = el("label", "label", t("mesurer.reference.seuil"));
      libelle.htmlFor = "seuil-reference";
      const saisie = el("input", erreurReference ? "input num input--error" : "input num");
      saisie.id = "seuil-reference";
      saisie.inputMode = "decimal";
      saisie.autocomplete = "off";
      saisie.value = reference.seuil || "";
      saisie.addEventListener("change", () => commandeReference("regler_seuil", { seuil: saisie.value }));
      saisie.addEventListener("keydown", (e) => { if (e.key === "Enter") saisie.blur(); });
      champ.append(libelle, saisie, el("p", "", t("mesurer.reference.seuil_aide")));
      const retirer = el("button", "btn", t("mesurer.reference.retirer"));
      retirer.addEventListener("click", () => commandeReference("retirer_reference", {}));
      actions.append(retirer);
      bloc.append(el("p", "why", t("mesurer.reference.est")), champ, actions);
    } else {
      const designer = el("button", "btn", t("mesurer.reference.designer"));
      designer.addEventListener("click", () => commandeReference("designer_reference", { numero: f.numero }));
      actions.append(designer);
      const phrase = reference
        ? t("mesurer.reference.actuelle").replace("{nom}", reference.nom)
        : t("mesurer.reference.aucune_texte");
      bloc.append(el("p", "why", phrase), actions);
    }
    if (erreurReference) bloc.append(el("p", "why", t(erreurReference)));
    if (reference) {
      const replie = el("details", "replie");
      replie.append(el("summary", "", t("mesurer.ecart.calcul_titre")), el("pre", "", t("mesurer.ecart.calcul")));
      bloc.append(replie);
    }
    return bloc;
  }

  // ---- Détails à droite : écart, couleur et valeurs, cartouche, puis la référence ----
  function cellule(libelle, valeur, large) {
    const c = el("div", large ? "cell cell--wide" : "cell");
    c.append(el("span", "label", libelle), el("span", "", valeur));
    return c;
  }

  // Grand carré de la couleur mesurée, calculée en Rust (crate colorimetrie) ; inconnue : pas de
  // carré. Comparée à la référence : un seul carré partagé, la référence en haut.
  function apercu(f, v) {
    const bloc = el("div");
    const ref = reference && !f.reference ? fiches.find((m) => m.numero === reference.numero) : null;
    const vRef = ref && ref.spectres[spectreChoisi].valeurs;
    bloc.append(el("span", "label", t(vRef && v ? "mesurer.ecart.comparaison" : "mesurer.couleur")));
    if (!v) {
      bloc.append(el("p", "", t("cartouche.inconnue")));
      return bloc;
    }
    const carre = el("span", vRef ? "apercu apercu--comparaison" : "apercu");
    if (vRef) {
      carre.style.setProperty("--ref", vRef.ecran);
      carre.style.setProperty("--mes", v.ecran);
    } else {
      carre.style.setProperty("--c", v.ecran);
    }
    carre.setAttribute("role", "img");
    carre.setAttribute("aria-label", t(vRef ? "mesurer.ecart.comparaison" : "mesurer.couleur"));
    bloc.append(carre);
    if (v.approchee) bloc.append(el("p", "why", t("mesurer.approchee")));
    return bloc;
  }

  // Libellé court d'un bouton de spectre : « M0 », ou son rang si la condition est inconnue.
  function libelleCourt(f, i) {
    const info = f.spectres[i].condition;
    return info.statut === "inconnue" ? String(i + 1) : info.valeur;
  }

  // « À confirmer » ou « inconnues », dit une seule fois à côté des boutons.
  function mentionConditions(f) {
    const statuts = f.spectres.map((s) => s.condition.statut);
    if (statuts.includes("inconnue")) return t("mesurer.conditions.inconnues");
    if (statuts.includes("supposee")) return t("mesurer.conditions.a_confirmer");
    return null;
  }

  function dessinerDetail() {
    const f = fiches.find((m) => m.numero === choisie);
    if (!f) {
      detail.replaceChildren();
      return;
    }
    const valeurs = el("div");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("mesurer.valeurs")));
    const seg = el("div", "seg");
    seg.setAttribute("role", "group");
    seg.setAttribute("aria-label", t("details.valeurs.libelle"));
    [0, 1, 2].forEach((i) => {
      const b = el("button", "", libelleCourt(f, i));
      b.setAttribute("aria-pressed", String(i === spectreChoisi));
      b.addEventListener("click", () => { spectreChoisi = i; dessiner(); }); // la liste suit le spectre choisi
      seg.append(b);
    });
    tete.append(seg);
    const v = f.spectres[spectreChoisi].valeurs;
    const inconnu = t("cartouche.inconnu");
    const rangees = [
      [t("mesurer.lab"), v ? [v.l, v.a, v.b] : [inconnu, inconnu, inconnu]],
      [t("mesurer.lch"), v ? [v.l, v.c, v.h || t("cartouche.inconnue")] : [inconnu, inconnu, inconnu]],
      [t("mesurer.xyz"), v ? [v.x, v.y, v.z] : [inconnu, inconnu, inconnu]],
    ];
    const corps = el("tbody");
    for (const [libelle, nombres] of rangees) {
      const tr = el("tr");
      tr.append(el("td", "", libelle));
      for (const n of nombres) tr.append(el("td", "r num", n));
      corps.append(tr);
    }
    const table = el("table");
    table.append(corps);
    valeurs.append(apercu(f, v), tete);
    const mention = mentionConditions(f);
    if (mention) valeurs.append(el("p", "why", mention));
    valeurs.append(table, el("p", "why", t("mesurer.calcul")));

    const cartouche = el("div", "cartouche");
    const titre = el("div", "cartouche__title");
    titre.append(el("span", "label", t("cartouche.libelle")), el("strong", "", f.nom));
    const cells = el("div", "cells");
    cells.append(
      // Une donnée que le pont n'a pas rendue s'écrit « inconnu », jamais une case vide.
      cellule(t("cartouche.instrument"), `${f.modele || t("cartouche.inconnu")} ${t("details.numero")} ${f.numero_serie}`),
      cellule(t("cartouche.micrologiciel"), f.micrologiciel || t("cartouche.inconnu")),
      cellule(t("cartouche.etalonnage"), etalonnage(f.etalonnage)),
      cellule(t("cartouche.condition_mesure"), conditionSpectre(f, spectreChoisi)),
      cellule(t("cartouche.date"), date(f.horodatage), true),
      cellule(t("details.condition"), f.condition_impression, true),
    );
    const pied = el("div", "cartouche__foot");
    pied.append(el("span", "", "myiro-libre"), el("span", "num", version));
    cartouche.append(titre, cells, pied);
    const espace = el("div", "section");
    espace.append(cartouche);
    // L'écart, s'il y en a un, précède la couleur : les valeurs prennent alors un intervalle.
    const ecart = blocEcart(f);
    const sectionValeurs = el("div", ecart.length ? "section section--serree" : "");
    sectionValeurs.append(valeurs);
    detail.replaceChildren(...ecart, sectionValeurs, espace, blocReference(f));
  }

  function dessiner() {
    const pourquoi = enCours ? null : raison();
    bouton.disabled = enCours || pourquoi !== null;
    // Raison du bouton inactif, ou refus de la dernière demande (cause puis action).
    const ligne = pourquoi || (erreur && !enCours ? refus(erreur) : null);
    raisonTexte.hidden = ligne === null;
    raisonTexte.textContent = ligne || "";
    // La consigne suit le choix « Mesure » : en automatique, pas de bouton à presser sur l'instrument.
    consigne.dataset.t = `mesurer.consigne.${declenchement}`;
    consigne.textContent = t(consigne.dataset.t);
    enCoursTexte.dataset.t = `mesurer.en_cours.${declenchement}`;
    enCoursTexte.textContent = t(enCoursTexte.dataset.t);
    // Choix propre au MYIRO-1 : un FD-9 actif ne mesure pas encore.
    choixDeclenchement.closest(".field").hidden = Boolean(vueInstrument && vueInstrument.etat === "detecte");
    for (const b of choixDeclenchement.querySelectorAll("[data-declenchement]")) {
      b.setAttribute("aria-pressed", String(b.dataset.declenchement === declenchement));
      b.disabled = enCours;
    }
    consigne.hidden = enCours;
    enCoursTexte.hidden = !enCours;
    choixCondition.disabled = enCours;
    dessinerAvis();
    if (choisie === null && fiches.length > 0) choisie = fiches[0].numero;
    dessinerListe();
    dessinerDetail();
  }

  // ---- Conditions d'impression : celle choisie est retenue d'une fois sur l'autre ----
  async function chargerConditions() {
    try {
      const branches = await invoke("bibliotheque_arborescence", { recherche: "" });
      conditions = branches.map((b) => b.condition);
    } catch (_) {
      conditions = [];
    }
    const retenue = choixCondition.value || memoire("condition-mesure");
    choixCondition.replaceChildren(...conditions.map((c) => {
      const o = el("option", "", c.nom);
      o.value = String(c.id);
      return o;
    }));
    if (conditions.some((c) => String(c.id) === retenue)) choixCondition.value = retenue;
    dessiner();
  }

  async function chargerMesures() {
    try {
      recevoirEcran(await invoke("mesures_seance", { langue: langue() }));
    } catch (e) {
      console.error("mesures_seance", e);
    }
    dessiner();
  }

  // ---- Mesurer : le module demande le geste, l'opérateur l'a déjà lu et a cliqué ----
  async function mesurer() {
    if (enCours || bouton.disabled) return;
    enCours = true;
    erreur = null;
    operationEnCours = "mesure"; // app.js : la barre ne s'ouvre pas pendant la mesure
    afficherInstrument();
    try {
      const r = await invoke("mesurer", { condition: Number(choixCondition.value), langue: langue() });
      if (r.instrument) recevoir(r.instrument); // app.js : instrument actif et liste du poste
      if (r.mesures.length > fiches.length) {
        choisie = r.mesures[0].numero;
        document.dispatchEvent(new CustomEvent("bibliotheque-modifiee"));
      }
      recevoirEcran(r);
    } catch (cle) {
      erreur = cle;
    }
    enCours = false;
    operationEnCours = null;
    afficherInstrument(); // app.js : barre, puis cette feuille (événement « instrument-affiche »)
  }

  async function renommer(numero, champ) {
    try {
      recevoirEcran(await invoke("renommer_mesure", { numero, nom: champ.value, langue: langue() }));
      erreur = null;
      dessiner();
      document.dispatchEvent(new CustomEvent("bibliotheque-modifiee")); // le nom, à gauche
    } catch (cle) {
      champ.classList.add("input--error");
      erreurListe.hidden = false;
      erreurListe.textContent = t(cle);
    }
  }

  window.__TAURI__.event.listen("geste", (evenement) => {
    if (evenement.payload !== "poser_sur_couleur") return;
    // La consigne était affichée au-dessus du bouton : le clic sur « Mesurer » vaut accord.
    invoke("repondre_geste", { fait: enCours }).catch((e) => console.error("repondre_geste", e));
  });

  bouton.addEventListener("click", mesurer);
  rangerBouton.addEventListener("click", async () => {
    try {
      recevoirEcran(await invoke("ranger_a_nouveau", { langue: langue() }));
      document.dispatchEvent(new CustomEvent("bibliotheque-modifiee"));
    } catch (e) {
      console.error("ranger_a_nouveau", e);
    }
    dessiner();
  });
  choixCondition.addEventListener("change", () => memoire("condition-mesure", choixCondition.value));
  choixDeclenchement.addEventListener("click", async (e) => {
    const b = e.target.closest("[data-declenchement]");
    if (!b || enCours) return;
    const nonRetenu = feuille.querySelector("[data-mesurer-declenchement-non-retenu]");
    try {
      const choix = await invoke("choisir_mode_mesure", { mode: b.dataset.declenchement });
      declenchement = choix.mode;
      // Écriture impossible : le choix vaut pour la séance, et on le dit.
      nonRetenu.hidden = choix.retenu;
    } catch (err) {
      console.error("choisir_mode_mesure", err);
      nonRetenu.hidden = false;
    }
    dessiner();
  });
  document.addEventListener("instrument-affiche", dessiner);
  document.addEventListener("bibliotheque-restauree", () => { erreurReference = null; chargerMesures(); });
  document.addEventListener("tache-affichee", (e) => { if (e.detail === "mesurer") chargerConditions(); });

  let premiereFois = true;
  async function langueAppliquee() {
    if (premiereFois) {
      premiereFois = false;
      version = await invoke("version_application");
      try {
        declenchement = await invoke("mode_mesure");
      } catch (e) {
        console.error("mode_mesure", e);
      }
      await chargerConditions();
    }
    await chargerMesures(); // valeurs réécrites dans la langue de l'écran
  }
  document.addEventListener("langue-appliquee", langueAppliquee);
  if (Object.keys(textes).length > 0) langueAppliquee();
})();
