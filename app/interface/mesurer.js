// Tâche Mesurer : mesure ponctuelle au centre (un seul bouton principal, les mesures de la
// séance empilées en dessous, noms modifiables), valeurs et provenance de la mesure choisie à
// droite. Mesures, valeurs et état de l'instrument viennent des modules Rust `mesurer` et
// `instrument` ; la page ne calcule rien. Les textes viennent du catalogue (chargé par app.js).
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const feuille = document.querySelector("[data-vue='mesurer']");
  const choixCondition = feuille.querySelector("[data-mesurer-condition]");
  const consigne = feuille.querySelector("[data-mesurer-consigne]");
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
  let version = "";

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
      tr.append(el("td", "id num", String(f.numero)), couleur, nomCellule, el("td", "num", date(f.horodatage)), el("td", "", range));
      tr.addEventListener("click", (e) => {
        if (e.target === nom) return;
        choisie = f.numero;
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

  // ---- Détails à droite : valeurs, puis cartouche de provenance ----
  function cellule(libelle, valeur, large) {
    const c = el("div", large ? "cell cell--wide" : "cell");
    c.append(el("span", "label", libelle), el("span", "", valeur));
    return c;
  }

  // Grand carré de la couleur mesurée, calculée en Rust (crate colorimetrie) ; inconnue : pas de carré.
  function apercu(v) {
    const bloc = el("div");
    bloc.append(el("span", "label", t("mesurer.couleur")));
    if (!v) {
      bloc.append(el("p", "", t("cartouche.inconnue")));
      return bloc;
    }
    const carre = el("span", "apercu");
    carre.style.setProperty("--c", v.ecran);
    carre.setAttribute("role", "img");
    carre.setAttribute("aria-label", t("mesurer.couleur"));
    bloc.append(carre);
    if (v.approchee) bloc.append(el("p", "why", t("mesurer.approchee")));
    return bloc;
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
      const b = el("button", "", conditionSpectre(f, i));
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
    valeurs.append(apercu(v), tete, table, el("p", "why", t("mesurer.calcul")));

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
    detail.replaceChildren(valeurs, espace);
  }

  function dessiner() {
    const pourquoi = enCours ? null : raison();
    bouton.disabled = enCours || pourquoi !== null;
    // Raison du bouton inactif, ou refus de la dernière demande (cause puis action).
    const ligne = pourquoi || (erreur && !enCours ? refus(erreur) : null);
    raisonTexte.hidden = ligne === null;
    raisonTexte.textContent = ligne || "";
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
      fiches = (await invoke("mesures_seance", { langue: langue() })).mesures;
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
    dessiner();
    try {
      const r = await invoke("mesurer", { condition: Number(choixCondition.value), langue: langue() });
      if (r.instrument) vueInstrument = r.instrument;
      if (r.mesures.length > fiches.length) {
        choisie = r.mesures[0].numero;
        document.dispatchEvent(new CustomEvent("bibliotheque-modifiee"));
      }
      fiches = r.mesures;
    } catch (cle) {
      erreur = cle;
    }
    enCours = false;
    afficherInstrument(); // app.js : barre, puis cette feuille (événement « instrument-affiche »)
  }

  async function renommer(numero, champ) {
    try {
      fiches = (await invoke("renommer_mesure", { numero, nom: champ.value, langue: langue() })).mesures;
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
      fiches = (await invoke("ranger_a_nouveau", { langue: langue() })).mesures;
      document.dispatchEvent(new CustomEvent("bibliotheque-modifiee"));
    } catch (e) {
      console.error("ranger_a_nouveau", e);
    }
    dessiner();
  });
  choixCondition.addEventListener("change", () => memoire("condition-mesure", choixCondition.value));
  document.addEventListener("instrument-affiche", dessiner);
  document.addEventListener("tache-affichee", (e) => { if (e.detail === "mesurer") chargerConditions(); });

  let premiereFois = true;
  async function langueAppliquee() {
    if (premiereFois) {
      premiereFois = false;
      version = await invoke("version_application");
      await chargerConditions();
    }
    await chargerMesures(); // valeurs réécrites dans la langue de l'écran
  }
  document.addEventListener("langue-appliquee", langueAppliquee);
  if (Object.keys(textes).length > 0) langueAppliquee();
})();
