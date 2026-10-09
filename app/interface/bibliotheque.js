// Bibliothèque à l'écran : colonne de gauche (conditions d'impression → mesures, avec
// recherche), feuille du centre (la mesure ou la condition choisie) et cartouche de
// provenance à droite. Les données viennent de la bibliothèque locale (module Rust
// `colonne`) ; les textes, du catalogue (`textes`, chargé par app.js). Aucun texte en dur.
"use strict";

(() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const zone = document.querySelector("[data-bibliotheque]");
  const arbre = zone.querySelector("[data-arbre]");
  const message = zone.querySelector("[data-arbre-message]");
  const recherche = zone.querySelector("[data-recherche]");
  const nouvelle = zone.querySelector("[data-nouvelle-condition]");
  const nouvelleErreur = zone.querySelector("[data-nouvelle-erreur]");
  const centreVide = document.querySelector("[data-centre-vide]");
  const centrePhrase = document.querySelector("[data-centre-phrase]");
  const centreAjout = document.querySelector("[data-centre-ajout]");
  const centreChoix = document.querySelector("[data-centre-choix]");
  const detailsChoix = document.querySelector("[data-details-choix]");
  const detailsVide = document.querySelector("[data-details-vide]");

  let branches = [];
  let bibliothequeVide = false; // aucune condition d'impression, recherche à part
  let erreurBibliotheque = null; // clé du catalogue si la bibliothèque ne répond pas
  let version = ""; // version de l'application, pour le pied du cartouche
  let choix = null; // { type: "condition" | "mesure", id }
  let spectreChoisi = 0; // spectre 1, 2 ou 3 de chaque plage dans le tableau

  // ---- Petits outils ----
  function el(nom, classe, texte) {
    const e = document.createElement(nom);
    if (classe) e.className = classe;
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  // Date et heure telles que le pont les a écrites (heure du poste de mesure),
  // sans conversion de fuseau.
  function date(horodatage) {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(horodatage);
    if (!m) return horodatage;
    const d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]));
    return new Intl.DateTimeFormat(document.documentElement.lang, {
      day: "numeric", month: "short", year: "numeric",
      hour: "2-digit", minute: "2-digit", timeZone: "UTC",
    }).format(d);
  }

  function nombre(v) {
    return new Intl.NumberFormat(document.documentElement.lang, {
      minimumFractionDigits: 2, maximumFractionDigits: 2,
    }).format(v);
  }

  function instrument(i) {
    return `${i.modele} ${t("details.numero")} ${i.numero_serie}`;
  }

  function lecture(geometrie, plages) {
    const nom = t("lecture." + geometrie.lecture);
    if (geometrie.lecture === "ponctuelle") return nom;
    return `${nom}, ${plages} ${t(plages > 1 ? "lecture.plages" : "lecture.plage")}`;
  }

  // Donnée qualifiée (confirmée, supposée, inconnue) mise en mots.
  function qualifiee(info, enTexte, inconnue) {
    if (info.statut === "inconnue") return t(inconnue);
    const texte = enTexte(info.valeur);
    return info.statut === "supposee" ? `${texte} (${t("details.a_confirmer")})` : texte;
  }

  // Condition de mesure du spectre n° i d'une plage, telle que le pont l'a dite :
  // jamais devinée d'après la position du spectre (ADR 0005).
  function conditionSpectre(d, i) {
    const inconnue = `${i + 1} · ${t("cartouche.inconnue")}`;
    const toutes = d.conditions_mesure;
    if (toutes.statut === "inconnue") return inconnue;
    const une = toutes.valeur[i];
    if (une.statut === "inconnue") return inconnue;
    const supposee = toutes.statut === "supposee" || une.statut === "supposee";
    return supposee ? `${une.valeur} (${t("details.a_confirmer")})` : une.valeur;
  }

  function conditionsMesure(d) {
    if (d.conditions_mesure.statut === "inconnue") return t("cartouche.inconnue");
    return [0, 1, 2].map((i) => conditionSpectre(d, i)).join(", ");
  }

  function brancheDe(idCondition) {
    return branches.find((b) => b.condition.id === idCondition);
  }

  // ---- Arborescence, à gauche ----
  function ligne(numero, texte, droite, choixLigne, niveau1) {
    const li = el("li", niveau1 ? "l1" : "");
    li.tabIndex = 0;
    li.append(el("span", "num mute", numero), el("span", "", texte), el("span", "num", droite));
    li.dataset.type = choixLigne.type;
    li.dataset.id = choixLigne.id;
    if (choix && choix.type === choixLigne.type && choix.id === choixLigne.id) {
      li.classList.add("sel");
      li.setAttribute("aria-current", "true");
    }
    return li;
  }

  function dessinerArbre() {
    arbre.replaceChildren();
    branches.forEach((b, i) => {
      arbre.append(ligne(String(i + 1), b.condition.nom, String(b.mesures.length),
        { type: "condition", id: b.condition.id }, true));
      b.mesures.forEach((m, j) => {
        arbre.append(ligne(`${i + 1}.${j + 1}`, `${date(m.horodatage)} · ${lecture(m.geometrie, m.plages)} · ${m.instrument.modele}`, "",
          { type: "mesure", id: m.id }, false));
      });
    });
    message.hidden = branches.length > 0;
    if (erreurBibliotheque) message.textContent = t(erreurBibliotheque);
    else message.textContent = recherche.value.trim() ? t("bibliotheque.aucun_resultat") : t("vide.bibliotheque.phrase");
  }

  async function chargerArbre() {
    try {
      branches = await invoke("bibliotheque_arborescence", { recherche: recherche.value });
      erreurBibliotheque = null;
    } catch (cle) {
      branches = [];
      erreurBibliotheque = cle;
    }
    if (!erreurBibliotheque && !recherche.value.trim()) bibliothequeVide = branches.length === 0;
    dessinerArbre();
  }

  // ---- Cartouche de provenance, à droite ----
  function cellule(libelle, valeur, large) {
    const c = el("div", large ? "cell cell--wide" : "cell");
    c.append(el("span", "label", t(libelle)), el("span", "", valeur));
    return c;
  }

  function montrerCartouche(libelle, titre, cellules) {
    const c = el("div", "cartouche");
    const tete = el("div", "cartouche__title");
    tete.append(el("span", "label", t(libelle)), el("strong", "", titre));
    const cells = el("div", "cells");
    cells.append(...cellules);
    const pied = el("div", "cartouche__foot");
    pied.append(el("span", "", "myiro-libre"), el("span", "num", version));
    c.append(tete, cells, pied);
    detailsChoix.replaceChildren(c);
    detailsChoix.hidden = false;
    detailsVide.hidden = true;
  }

  function cacherCartouche() {
    detailsChoix.replaceChildren();
    detailsChoix.hidden = true;
    detailsVide.hidden = false;
  }

  // ---- Feuille du centre ----
  function montrerCentre(...enfants) {
    centreChoix.replaceChildren(...enfants);
    centreChoix.hidden = false;
    centreVide.hidden = true;
  }

  function centreSansChoix() {
    centreChoix.replaceChildren();
    centreChoix.hidden = true;
    centreVide.hidden = false;
    // Une bibliothèque qui ne s'ouvre pas le dit aussi au centre, sans proposer d'ajout.
    if (erreurBibliotheque) centrePhrase.textContent = t(erreurBibliotheque);
    else centrePhrase.textContent = t(bibliothequeVide ? "vide.bibliotheque.phrase" : "centre.choisir");
    centreAjout.hidden = Boolean(erreurBibliotheque) || !bibliothequeVide;
  }

  function enTete(titre, contexte) {
    const tete = el("div");
    tete.append(el("h2", "sheet-title", titre), el("p", "why", contexte));
    return tete;
  }

  function tableauLab(d) {
    const section = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("details.valeurs")));
    const seg = el("div", "seg");
    seg.setAttribute("role", "group");
    seg.setAttribute("aria-label", t("details.valeurs.libelle"));
    [0, 1, 2].forEach((i) => {
      const b = el("button", "", conditionSpectre(d, i));
      b.setAttribute("aria-pressed", String(i === spectreChoisi));
      b.addEventListener("click", () => { spectreChoisi = i; afficherChoix(); });
      seg.append(b);
    });
    tete.append(seg);

    const table = el("table", "table--l");
    const entete = el("tr");
    entete.append(el("th", "", t("details.plage")), el("th", "r lc", "L*"), el("th", "r lc", "a*"), el("th", "r lc", "b*"));
    const thead = el("thead");
    thead.append(entete);
    const corps = el("tbody");
    d.lab.forEach((plage, i) => {
      const tr = el("tr");
      tr.append(el("td", "id num", String(i + 1)));
      for (const v of plage[spectreChoisi]) tr.append(el("td", "r num", nombre(v)));
      corps.append(tr);
    });
    table.append(thead, corps);
    section.append(tete, table);
    return section;
  }

  // Emplacement réservé à la courbe de spectre, construite plus tard.
  function emplacementSpectre() {
    const section = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("details.spectre")));
    section.append(tete, el("p", "why", t("raison.bientot")));
    return section;
  }

  function choisirCondition(b) {
    const instruments = [];
    for (const m of b.mesures) {
      const nom = instrument(m.instrument);
      if (!instruments.includes(nom)) instruments.push(nom);
    }
    montrerCartouche("details.condition", b.condition.nom, [
      cellule("bibliotheque.mesures", String(b.mesures.length)),
      cellule("cartouche.reference", t("cartouche.aucune")),
      cellule("details.instruments", instruments.join(", ") || t("cartouche.aucun"), true),
    ]);

    const champ = el("div", "field");
    const libelle = el("label", "label", t("details.nom"));
    libelle.htmlFor = "nom-condition";
    const saisie = el("input", "input");
    saisie.id = "nom-condition";
    saisie.autocomplete = "off";
    saisie.value = b.condition.nom;
    const erreur = el("p", "why");
    erreur.hidden = true;
    champ.append(libelle, saisie, erreur);
    const bouton = el("button", "btn", t("details.renommer"));
    const actions = el("div", "actions");
    actions.append(bouton);
    const renommer = async () => {
      try {
        await invoke("bibliotheque_renommer_condition", { id: b.condition.id, nom: saisie.value });
        await chargerArbre();
        afficherChoix();
      } catch (cle) {
        saisie.classList.add("input--error");
        erreur.hidden = false;
        erreur.textContent = t(cle);
      }
    };
    bouton.addEventListener("click", renommer);
    saisie.addEventListener("keydown", (e) => { if (e.key === "Enter") renommer(); });

    const section = el("div", "section");
    section.append(champ, actions);
    montrerCentre(
      enTete(b.condition.nom, `${b.mesures.length} ${t(b.mesures.length > 1 ? "compte.mesures" : "compte.mesure")}`),
      section,
    );
  }

  async function choisirMesure(id) {
    let d;
    try {
      d = await invoke("bibliotheque_detail_mesure", { id });
    } catch (cle) {
      cacherCartouche();
      montrerCentre(el("p", "why", t(cle)));
      return;
    }
    if (!choix || choix.type !== "mesure" || choix.id !== id) return; // choix changé entre-temps
    const nomCondition = d.nom_condition;
    const lu = lecture(d.geometrie, d.lab.length);
    montrerCartouche("details.mesure", date(d.horodatage), [
      cellule("cartouche.instrument", instrument(d.instrument)),
      cellule("cartouche.etalonnage", qualifiee(d.etalonnage, (h) => date(h), "cartouche.inconnu")),
      cellule("cartouche.condition_mesure", conditionsMesure(d)),
      cellule("cartouche.reference", t("cartouche.aucune")),
      cellule("details.condition", nomCondition, true),
      cellule("details.lecture", lu, true),
    ]);
    montrerCentre(
      enTete(date(d.horodatage), [lu, nomCondition].filter(Boolean).join(" · ")),
      tableauLab(d),
      emplacementSpectre(),
    );
  }

  function afficherChoix() {
    dessinerArbre();
    const b = choix && choix.type === "condition" ? brancheDe(choix.id) : null;
    if (b) return choisirCondition(b);
    if (choix && choix.type === "mesure") return choisirMesure(choix.id);
    cacherCartouche();
    centreSansChoix();
  }

  function choisir(li) {
    choix = { type: li.dataset.type, id: Number(li.dataset.id) };
    afficherTache("bibliotheque"); // app.js : la feuille du centre montre le choix
    afficherChoix();
  }

  // ---- Ajout d'une condition d'impression ----
  async function creer() {
    try {
      const c = await invoke("bibliotheque_creer_condition", { nom: nouvelle.value });
      nouvelle.value = "";
      nouvelle.classList.remove("input--error");
      nouvelleErreur.hidden = true;
      recherche.value = "";
      choix = { type: "condition", id: c.id };
      await chargerArbre();
      afficherChoix();
    } catch (cle) {
      nouvelle.classList.add("input--error");
      nouvelleErreur.hidden = false;
      nouvelleErreur.textContent = t(cle);
    }
  }

  // ---- Événements ----
  arbre.addEventListener("click", (e) => {
    const li = e.target.closest("li");
    if (li) choisir(li);
  });
  arbre.addEventListener("keydown", (e) => {
    const li = e.target.closest("li");
    if (li && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      choisir(li);
      const meme = arbre.querySelector(`li[data-type="${li.dataset.type}"][data-id="${li.dataset.id}"]`);
      if (meme) meme.focus();
    }
  });

  let attente;
  recherche.addEventListener("input", () => {
    clearTimeout(attente);
    attente = setTimeout(chargerArbre, 150);
  });
  zone.querySelector("[data-creer-condition]").addEventListener("click", creer);
  nouvelle.addEventListener("keydown", (e) => { if (e.key === "Enter") creer(); });
  // L'action principale de la feuille vide mène au champ d'ajout, à gauche.
  centreAjout.addEventListener("click", () => {
    if (nouvelle.value.trim()) creer();
    else nouvelle.focus();
  });

  // La langue est appliquée au lancement puis à chaque changement : tout se redessine.
  let premiereFois = true;
  async function langueAppliquee() {
    recherche.placeholder = t("bibliotheque.recherche_aide");
    if (premiereFois) {
      premiereFois = false;
      version = await invoke("version_application");
      for (const v of document.querySelectorAll("[data-version]")) v.textContent = version;
      zone.querySelector("[data-demonstration]").hidden = !(await invoke("bibliotheque_demonstration"));
      await chargerArbre();
    }
    afficherChoix();
  }
  document.addEventListener("langue-appliquee", langueAppliquee);
  // Une mesure vient d'être rangée (mesurer.js) : l'arborescence la montre.
  document.addEventListener("bibliotheque-modifiee", chargerArbre);
  // Le catalogue a pu arriver avant le chargement de ce script.
  if (Object.keys(textes).length > 0) langueAppliquee();
})();
