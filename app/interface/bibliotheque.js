// Colonne de gauche (conditions d'impression → mesures, avec recherche) et panneau
// de détails. Les données viennent de la bibliothèque locale (module Rust `colonne`) ;
// les textes, du catalogue (`textes`, chargé par app.js). Aucun texte en dur ici.
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
  const detailsChoix = document.querySelector("[data-details-choix]");
  const detailsVide = document.querySelector("[data-details-vide]");

  let branches = [];
  let choix = null; // { type: "condition" | "mesure", id }
  let condition_lab = 0; // M0, M1 ou M2 dans le tableau des valeurs

  // ---- Petits outils ----
  function el(nom, classe, texte) {
    const e = document.createElement(nom);
    if (classe) e.className = classe;
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  // Date et heure telles que le pont les a écrites (heure du poste de mesure),
  // sans conversion de fuseau.
  function date(horodatage, avecAnnee = true) {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(horodatage);
    if (!m) return horodatage;
    const d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]));
    return new Intl.DateTimeFormat(document.documentElement.lang, {
      day: "numeric", month: "short", year: avecAnnee ? "numeric" : undefined,
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

  // ---- Arborescence ----
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
        arbre.append(ligne(`${i + 1}.${j + 1}`, `${date(m.horodatage)} · ${lecture(m.geometrie, m.plages)}`, "",
          { type: "mesure", id: m.id }, false));
      });
    });
    const vide = branches.length === 0;
    message.hidden = !vide;
    message.textContent = recherche.value.trim() ? t("bibliotheque.aucun_resultat") : t("vide.bibliotheque.phrase");
  }

  async function chargerArbre() {
    try {
      branches = await invoke("bibliotheque_arborescence", { recherche: recherche.value });
    } catch (cle) {
      branches = [];
      dessinerArbre();
      message.hidden = false;
      message.textContent = t(cle);
      return;
    }
    dessinerArbre();
  }

  // ---- Détails ----
  function cellule(libelle, valeur, large) {
    const c = el("div", large ? "cell cell--wide" : "cell");
    c.append(el("span", "label", t(libelle)), el("span", "", valeur));
    return c;
  }

  function cartouche(libelle, titre, cellules) {
    const c = el("div", "cartouche");
    const tete = el("div", "cartouche__title");
    tete.append(el("span", "label", t(libelle)), el("strong", "", titre));
    const cells = el("div", "cells");
    cells.append(...cellules);
    const pied = el("div", "cartouche__foot");
    pied.append(el("span", "", "myiro-libre"), el("span", "num", "0.1.0"));
    c.append(tete, cells, pied);
    return c;
  }

  function montrer(...enfants) {
    detailsChoix.replaceChildren(...enfants);
    detailsChoix.hidden = false;
    detailsVide.hidden = true;
  }

  function cacherDetails() {
    detailsChoix.replaceChildren();
    detailsChoix.hidden = true;
    detailsVide.hidden = false;
  }

  function detailsCondition(b) {
    const instruments = [];
    for (const m of b.mesures) {
      const nom = instrument(m.instrument);
      if (!instruments.includes(nom)) instruments.push(nom);
    }
    const champ = el("div", "field");
    const libelle = el("label", "label", t("details.nom"));
    libelle.htmlFor = "nom-condition";
    const saisie = el("input", "input");
    saisie.id = "nom-condition";
    saisie.autocomplete = "off";
    saisie.value = b.condition.nom;
    const erreur = el("p");
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
    montrer(
      cartouche("details.condition", b.condition.nom, [
        cellule("bibliotheque.mesures", String(b.mesures.length)),
        cellule("cartouche.reference", t("cartouche.aucune")),
        cellule("details.instruments", instruments.join(", ") || t("cartouche.aucun"), true),
      ]),
      section,
    );
  }

  function tableauLab(d) {
    const section = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("details.valeurs")));
    const seg = el("div", "seg");
    seg.setAttribute("role", "group");
    seg.setAttribute("aria-label", t("details.valeurs.libelle"));
    ["M0", "M1", "M2"].forEach((nom, i) => {
      const b = el("button", "", nom);
      b.setAttribute("aria-pressed", String(i === condition_lab));
      b.addEventListener("click", () => { condition_lab = i; afficherChoix(); });
      seg.append(b);
    });
    tete.append(seg);

    const table = el("table");
    const entete = el("tr");
    entete.append(el("th", "", t("details.plage")), el("th", "r lc", "L*"), el("th", "r lc", "a*"), el("th", "r lc", "b*"));
    const thead = el("thead");
    thead.append(entete);
    const corps = el("tbody");
    d.lab.forEach((plage, i) => {
      const tr = el("tr");
      tr.append(el("td", "id num", String(i + 1)));
      for (const v of plage[condition_lab]) tr.append(el("td", "r num", nombre(v)));
      corps.append(tr);
    });
    table.append(thead, corps);
    section.append(tete, table);
    return section;
  }

  async function detailsMesure(id) {
    let d;
    try {
      d = await invoke("bibliotheque_detail_mesure", { id });
    } catch (cle) {
      montrer(el("p", "why", t(cle)));
      return;
    }
    if (!choix || choix.type !== "mesure" || choix.id !== id) return; // choix changé entre-temps
    const branche = branches.find((b) => b.condition.id === d.condition);
    const conditions = qualifiee(d.conditions_mesure,
      (liste) => liste.map((c) => qualifiee(c, (v) => v, "cartouche.inconnue")).join(", "),
      "cartouche.inconnue");
    montrer(
      cartouche("details.mesure", date(d.horodatage), [
        cellule("cartouche.instrument", instrument(d.instrument)),
        cellule("cartouche.etalonnage", qualifiee(d.etalonnage, (h) => date(h), "cartouche.inconnu")),
        cellule("cartouche.condition_mesure", conditions),
        cellule("cartouche.reference", t("cartouche.aucune")),
        cellule("details.condition", branche ? branche.condition.nom : "", true),
        cellule("details.lecture", lecture(d.geometrie, d.lab.length), true),
      ]),
      tableauLab(d),
    );
  }

  function afficherChoix() {
    dessinerArbre();
    if (!choix) return cacherDetails();
    if (choix.type === "condition") {
      const b = branches.find((x) => x.condition.id === choix.id);
      return b ? detailsCondition(b) : cacherDetails();
    }
    return detailsMesure(choix.id);
  }

  function choisir(li) {
    choix = { type: li.dataset.type, id: Number(li.dataset.id) };
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

  // La langue est appliquée au lancement puis à chaque changement : tout se redessine.
  let premiereFois = true;
  document.addEventListener("langue-appliquee", async () => {
    recherche.placeholder = t("bibliotheque.recherche_aide");
    if (premiereFois) {
      premiereFois = false;
      zone.querySelector("[data-demonstration]").hidden = !(await invoke("bibliotheque_demonstration"));
      await chargerArbre();
    }
    afficherChoix();
  });
})();
