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
  let choix = null; // { type: "condition" | "mesure" | "importee" | "apercu", id }
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

  // Date courte pour la colonne de gauche : jour, mois et heure (« 9 oct., 09:15 »).
  function dateCourte(horodatage) {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(horodatage);
    if (!m) return horodatage;
    const d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]));
    return new Intl.DateTimeFormat(document.documentElement.lang, {
      day: "numeric", month: "short", hour: "2-digit", minute: "2-digit", timeZone: "UTC",
    }).format(d);
  }

  function nombre(v) {
    return new Intl.NumberFormat(document.documentElement.lang, {
      minimumFractionDigits: 2, maximumFractionDigits: 2, signDisplay: "negative",
    }).format(v); // « negative » : jamais « -0,00 »
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

  function nombreMesures(b) {
    return b.mesures.length + b.importees.length;
  }

  // Condition d'impression du choix courant : la condition elle-même, ou celle
  // de la mesure choisie.
  function conditionChoisie() {
    if (!choix) return null;
    if (choix.type === "condition") return choix.id;
    const b = branches.find((b) => b.mesures.some((m) => m.id === choix.id) || b.importees.some((m) => m.id === choix.id));
    return b ? b.condition.id : null;
  }

  // ---- Arborescence, à gauche ----
  // `texte` : une chaîne, ou [nom, détail] pour une mesure, sur deux lignes au plus.
  function ligne(numero, texte, droite, choixLigne, niveau1) {
    const li = el("li", niveau1 ? "l1" : "");
    li.tabIndex = 0;
    let corps;
    if (Array.isArray(texte)) {
      const [nom, detail] = texte;
      corps = el("span");
      const n = el("span", "index__nom", nom);
      const d = el("span", "index__meta", detail);
      n.title = nom; // le texte entier au survol, si la colonne est trop étroite
      d.title = detail;
      corps.append(n, d);
    } else {
      corps = el("span", "", texte);
    }
    li.append(el("span", "num mute", numero), corps, el("span", "num", droite));
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
      arbre.append(ligne(String(i + 1), b.condition.nom, String(nombreMesures(b)),
        { type: "condition", id: b.condition.id }, true));
      b.mesures.forEach((m, j) => {
        // Ligne 1 : le nom donné à la mesure (tâche Mesurer), ou la date ; ligne 2 : date courte · lecture.
        const nom = m.nom || date(m.horodatage);
        const detail = `${dateCourte(m.horodatage)} · ${lecture(m.geometrie, m.plages)}`;
        arbre.append(ligne(`${i + 1}.${j + 1}`, [nom, detail], "",
          { type: "mesure", id: m.id }, false));
      });
      // Les mesures importées suivent, marquées comme telles avec le nom de leur fichier.
      b.importees.forEach((m, j) => {
        arbre.append(ligne(`${i + 1}.${b.mesures.length + j + 1}`, [m.fichier, t("import.importee")], "",
          { type: "importee", id: m.id }, false));
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

  // Nom d'un spectre dans la bascule : sa condition de mesure si elle est connue,
  // sinon son rang, dit en clair (jamais « 2 · Inconnue » comme choix).
  function libelleBascule(libelle, i, connue) {
    return connue ? libelle : t("details.spectre_sans_condition").replace("{}", String(i + 1));
  }

  // Tableau des Lab, compact ; lui seul défile, l'en-tête de la feuille reste en vue.
  // `spectres` : les spectres présents dans la mesure, [{ libelle, index }] ;
  // `lab[plage][index]` vaut trois nombres, ou `null` si la valeur est inconnue.
  function tableauLab(spectres, lab) {
    if (spectres.length && !spectres.some((s) => s.index === spectreChoisi)) spectreChoisi = spectres[0].index;
    const section = el("div", "section section--defile");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("details.valeurs")));
    if (spectres.length > 1) {
      const seg = el("div", "seg");
      seg.setAttribute("role", "group");
      seg.setAttribute("aria-label", t("details.valeurs.libelle"));
      for (const s of spectres) {
        const b = el("button", "", s.libelle);
        b.setAttribute("aria-pressed", String(s.index === spectreChoisi));
        b.addEventListener("click", () => { spectreChoisi = s.index; afficherChoix(); });
        seg.append(b);
      }
      tete.append(seg);
    } else if (spectres.length === 1) {
      // Une seule condition dans la mesure : son nom, sans bascule.
      tete.append(el("span", "label", spectres[0].libelle));
    }

    const table = el("table", "table--compact");
    const entete = el("tr");
    entete.append(el("th", "", t("details.plage")), el("th", "r lc", "L*"), el("th", "r lc", "a*"), el("th", "r lc", "b*"));
    const thead = el("thead");
    thead.append(entete);
    const corps = el("tbody");
    lab.forEach((plage, i) => {
      const tr = el("tr");
      tr.append(el("td", "id num", String(i + 1)));
      const valeurs = plage[spectreChoisi];
      if (valeurs) for (const v of valeurs) tr.append(el("td", "r num", nombre(v)));
      else for (let k = 0; k < 3; k++) tr.append(el("td", "r mute", t("cartouche.inconnu")));
      corps.append(tr);
    });
    table.append(thead, corps);
    const defile = el("div", "defile");
    defile.append(table);
    section.append(tete, defile);
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
      cellule("bibliotheque.mesures", String(nombreMesures(b))),
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
      enTete(b.condition.nom, `${nombreMesures(b)} ${t(nombreMesures(b) > 1 ? "compte.mesures" : "compte.mesure")}`),
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
    // Une mesure du pont a toujours ses trois spectres ; seule leur condition peut être inconnue.
    const connue = (i) => d.conditions_mesure.statut !== "inconnue" && d.conditions_mesure.valeur[i].statut !== "inconnue";
    montrerCentre(
      enTete(date(d.horodatage), [lu, nomCondition].filter(Boolean).join(" · ")),
      tableauLab([0, 1, 2].map((i) => ({ libelle: libelleBascule(conditionSpectre(d, i), i, connue(i)), index: i })), d.lab),
      emplacementSpectre(),
    );
  }

  // Ce que montre une mesure importée, en aperçu ou rangée : les spectres
  // présents dans le fichier, les cellules du cartouche et les Lab.
  function vueImport(d) {
    const presents = [0, 1, 2].filter((e) => d.spectres[e] || d.lab.some((p) => p[e].statut !== "inconnue"));
    const nommer = (e) => libelleBascule(qualifiee(d.conditions[e], (v) => v, "cartouche.inconnue"), e,
      d.conditions[e].statut !== "inconnue");
    const contenus = presents.map((e) =>
      `${nommer(e)} (${t(d.spectres[e] ? "import.spectre_et_lab" : "import.lab_seul")})`);
    const instrumentLu = qualifiee(d.instrument, (i) =>
      d.numero_serie.statut === "inconnue" ? i : `${i} ${t("details.numero")} ${d.numero_serie.valeur}`,
    "cartouche.inconnu");
    const cellules = [
      cellule("import.origine", t("import.importee")),
      cellule("import.date", qualifiee(d.date, (h) => date(h), "cartouche.inconnue")),
      cellule("import.fichier", d.fichier, true),
      cellule("cartouche.instrument", instrumentLu, true),
      cellule("import.provenance", t(d.myiro_libre ? "import.provenance.myiro" : "import.provenance.autre"), true),
      cellule("cartouche.condition_mesure", contenus.join(", ") || t("cartouche.inconnue"), true),
    ];
    if (!d.spectres.some(Boolean)) cellules.push(cellule("import.spectres", t("import.aucun_spectre"), true));
    return {
      cellules,
      contexte: [t("import.importee"), `${d.plages} ${t(d.plages > 1 ? "lecture.plages" : "lecture.plage")}`],
      tableau: () => tableauLab(presents.map((e) => ({ libelle: nommer(e), index: e })),
        d.lab.map((p) => p.map((v) => (v.statut === "inconnue" ? null : v.valeur)))),
    };
  }

  // Une mesure importée et rangée : son cartouche dit « Importée » et le fichier d'origine.
  async function choisirImportee(id) {
    let d;
    try {
      d = await invoke("bibliotheque_detail_importee", { id });
    } catch (cle) {
      cacherCartouche();
      montrerCentre(el("p", "why", t(cle)));
      return;
    }
    if (!choix || choix.type !== "importee" || choix.id !== id) return; // choix changé entre-temps
    const v = vueImport(d);
    montrerCartouche("import.titre", d.fichier, [...v.cellules, cellule("details.condition", d.nom_condition, true)]);
    montrerCentre(enTete(d.fichier, [...v.contexte, d.nom_condition].join(" · ")), v.tableau());
  }

  // ---- Import en deux temps : aperçu du fichier lu, puis rangement sur accord ----
  let apercu = null; // contenu du fichier lu, pas encore rangé
  let destination = null; // condition d'impression qui recevra la mesure
  let avantApercu = null; // choix à retrouver si l'opérateur annule

  function montrerApercu() {
    const v = vueImport(apercu);
    montrerCartouche("import.apercu", apercu.fichier, v.cellules);

    // L'action d'abord, toujours en vue : un seul bouton principal.
    const choisie = destination === null ? null : brancheDe(destination);
    const ranger = el("button", "btn btn--primary",
      choisie ? t("import.ranger").replace("{}", choisie.condition.nom) : t("import.ranger.attente"));
    ranger.disabled = !choisie;
    ranger.addEventListener("click", () => rangerApercu(ranger));
    const annuler = el("button", "btn", t("import.annuler"));
    annuler.addEventListener("click", annulerApercu);
    const actions = el("div", "actions actions--serrees");
    actions.append(ranger, annuler);

    // Puis la condition d'impression qui recevra la mesure, au choix.
    const section = el("div", "section section--serree");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("import.destination")));
    section.append(tete);
    if (branches.length === 0) section.append(el("p", "why", t("import.aucune_condition")));
    else if (!choisie) section.append(el("p", "why", t("import.ranger.raison")));
    const liste = el("div", "choix-liste");
    for (const b of branches) {
      const rang = el("label", "choix-liste__rang");
      const radio = el("input");
      radio.type = "radio";
      radio.name = "destination";
      radio.id = `destination-${b.condition.id}`;
      radio.checked = b.condition.id === destination;
      radio.addEventListener("change", () => { destination = b.condition.id; afficherChoix(); });
      rang.append(radio, el("span", "", b.condition.nom));
      liste.append(rang);
    }
    section.append(liste);

    montrerCentre(enTete(apercu.fichier, [...v.contexte, t("import.apercu")].join(" · ")), actions, section, v.tableau());
  }

  // Le bouton est inactif pendant l'appel : un double clic ne range pas deux fois.
  async function rangerApercu(bouton) {
    const choisie = brancheDe(destination);
    bouton.disabled = true;
    try {
      const id = await invoke("bibliotheque_ranger_import", { numero: apercu.numero, condition: destination });
      apercu = null;
      choix = { type: "importee", id };
      await chargerArbre();
      afficherChoix();
      annoncer(t("import.rangee").replace("{}", choisie ? choisie.condition.nom : ""));
    } catch (cle) {
      bouton.disabled = false;
      annoncer(t(cle));
    }
  }

  async function annulerApercu() {
    apercu = null;
    choix = avantApercu;
    try { await invoke("bibliotheque_annuler_import"); } catch (_) { /* rien n'était rangé */ }
    afficherChoix();
  }

  // ---- Barre d'actions de la feuille : toujours en vue, au-dessus du tableau ----
  const exportZone = document.querySelector("[data-export]");
  const exporterBouton = exportZone.querySelector("[data-exporter]");
  const exportRaison = exportZone.querySelector("[data-export-raison]");
  const exportMessage = exportZone.querySelector("[data-export-message]");
  const restaurerAvis = exportZone.querySelector("[data-restaurer-avis]");

  function dessinerExport() {
    // Un seul export : le CGATS de la mesure choisie.
    const sansMesure = !(choix && choix.type === "mesure");
    exporterBouton.disabled = sansMesure;
    exportRaison.hidden = !sansMesure;
  }

  function annoncer(texte) {
    exportMessage.textContent = texte;
    exportMessage.hidden = false;
  }

  // Le sélecteur de fichier de Windows s'ouvre côté Rust ; `null` : l'opérateur a annulé.
  async function exporter() {
    exportMessage.hidden = true;
    try {
      const nom = await invoke("bibliotheque_exporter_cgats", { id: choix.id, filtre: t("export.filtre.cgats") });
      if (nom) annoncer(`${t("export.fait")} ${nom}`);
    } catch (cle) {
      annoncer(t(cle));
    }
  }

  async function sauvegarder() {
    exportMessage.hidden = true;
    try {
      const nom = await invoke("bibliotheque_sauvegarder", { filtre: t("export.filtre.sauvegarde") });
      if (nom) annoncer(`${t("export.fait")} ${nom}`);
    } catch (cle) {
      annoncer(t(cle));
    }
  }

  // Lit le fichier et le montre en aperçu : rien n'est rangé avant « Ranger dans … ».
  async function importer() {
    exportMessage.hidden = true;
    try {
      const contenu = await invoke("bibliotheque_apercu_cgats", { filtre: t("export.filtre.cgats") });
      if (!contenu) return;
      // Sans sélection, « Annuler » revient à la feuille vide, pas à un ancien choix.
      avantApercu = choix && choix.type === "apercu" ? avantApercu : choix;
      destination = conditionChoisie();
      apercu = contenu;
      recherche.value = "";
      await chargerArbre();
      if (destination !== null && !brancheDe(destination)) destination = null;
      choix = { type: "apercu", id: 0 };
      afficherTache("bibliotheque"); // app.js : la feuille du centre montre l'aperçu
      afficherChoix();
    } catch (cle) {
      annoncer(t(cle));
    }
  }

  async function restaurer() {
    restaurerAvis.hidden = true;
    exportMessage.hidden = true;
    try {
      const nom = await invoke("bibliotheque_restaurer", { filtre: t("export.filtre.sauvegarde"), langue: document.documentElement.lang || "fr" });
      if (!nom) return;
      choix = null;
      apercu = null;
      recherche.value = "";
      await chargerArbre();
      afficherChoix();
      annoncer(`${t("export.restaure")} ${nom}`);
      document.dispatchEvent(new CustomEvent("bibliotheque-restauree")); // mesurer.js : la référence suit
    } catch (cle) {
      annoncer(t(cle));
    }
  }

  function afficherChoix() {
    dessinerArbre();
    dessinerExport();
    if (choix && choix.type === "apercu" && apercu) return montrerApercu();
    const b = choix && choix.type === "condition" ? brancheDe(choix.id) : null;
    if (b) return choisirCondition(b);
    if (choix && choix.type === "mesure") return choisirMesure(choix.id);
    if (choix && choix.type === "importee") return choisirImportee(choix.id);
    cacherCartouche();
    centreSansChoix();
  }

  function choisir(li) {
    // Choisir ailleurs abandonne l'aperçu : rien n'avait été rangé.
    if (apercu) {
      apercu = null;
      invoke("bibliotheque_annuler_import").catch(() => {});
      annoncer(t("import.abandonne"));
    }
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
      // Pendant un aperçu, la nouvelle condition devient sa destination.
      if (apercu) destination = c.id;
      else choix = { type: "condition", id: c.id };
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

  exporterBouton.addEventListener("click", exporter);
  exportZone.querySelector("[data-sauvegarder]").addEventListener("click", sauvegarder);
  exportZone.querySelector("[data-importer]").addEventListener("click", importer);
  // Restaurer remplace toute la bibliothèque : l'accord de l'opérateur est demandé d'abord.
  exportZone.querySelector("[data-restaurer]").addEventListener("click", () => {
    exportMessage.hidden = true;
    restaurerAvis.hidden = false;
  });
  exportZone.querySelector("[data-restaurer-confirmer]").addEventListener("click", restaurer);
  exportZone.querySelector("[data-restaurer-annuler]").addEventListener("click", () => { restaurerAvis.hidden = true; });

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
