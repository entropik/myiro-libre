// Vue « Comparer » de la bibliothèque (ticket #71) : de un à quatre profils RVB, rangés ou ouverts
// depuis un fichier (jamais rangé), comparés par le module Rust `comparer`. Le premier sert de
// référence. Au centre : profils, chiffres clés, image (comparer-image.js), gamut en volume
// (vue3d.js, avec les couleurs de l'image de nuage-image.js), chroma par teinte, coupe, chemin (graphiques.js), couleurs de base et écarts
// (comparer-ecarts.js), méthode, chaque grande partie repliable (accordeon.js) ; à droite, la provenance de chaque profil et, sous « Avancé »,
// la commande d'iccgamut (comparer-details.js). Un chiffre `null` s'écrit « inconnu » (Graphiques.nombre). Les textes
// viennent du catalogue (app.js).
"use strict";

window.Comparer = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;
  const { carreSerie } = window.ComparerEcarts;
  const { texteRefus, texteLimite, texteFidelite, section, seg, chiffres, methode, provenance, avance } = window.ComparerDetails;
  const image = window.ComparerImage;

  const vue = document.querySelector("[data-vue-comparer]");
  const details = document.querySelector("[data-details-comparer]");
  const puces = vue.querySelector("[data-comparer-puces]");
  const ajouter = vue.querySelector("[data-comparer-ajouter]");
  const raisonAjout = vue.querySelector("[data-comparer-ajouter-raison]");
  const choix = vue.querySelector("[data-comparer-choix]");
  const message = vue.querySelector("[data-comparer-message]");
  const enCours = vue.querySelector("[data-comparer-en-cours]");
  const corps = vue.querySelector("[data-comparer-corps]");
  const MAX = 4;
  // Les huit couleurs de base, dans l'ordre de la crate `gamut` ; le chemin va de bleu à cyan.
  const BASES = [
    ["blanc", [255, 255, 255]], ["noir", [0, 0, 0]], ["rouge", [255, 0, 0]], ["vert", [0, 255, 0]],
    ["bleu", [0, 0, 255]], ["cyan", [0, 255, 255]], ["magenta", [255, 0, 255]], ["jaune", [255, 255, 0]],
  ];

  let ouverte = false;
  let choisis = []; // { source, nom } dans l'ordre : le premier est la référence
  let comparables = null; // profils de la bibliothèque, null tant que la liste n'est pas lue
  let comparaison = null;
  let chemins = [];
  let demande = 0; // la dernière demande l'emporte
  let vue3d = null;
  let reglages = { couleur: "serie", rendu: "opaque", forme: null, l: 50, depart: "bleu", arrivee: "cyan" };
  const masques = new Set(); // rangs des profils décochés dans la vue 3D
  // Le nuage de l'image (nuage-image.js) suit le seuil et le profil de la surimpression.
  let coupeNuage = () => {};
  window.NuageImage.suivre(() => { if (vue3d) vue3d.points(window.NuageImage.points()); coupeNuage(); });

  function annoncer(texte) {
    message.textContent = texte;
    message.hidden = !texte;
  }

  // Une erreur des commandes est une clé du catalogue.
  const texteErreur = (cle) => t(String(cle));

  // ---- Profils comparés et ajout ----
  function dessinerAjout() {
    const plein = choisis.length >= MAX;
    ajouter.disabled = comparables === null || plein;
    raisonAjout.hidden = !ajouter.disabled;
    raisonAjout.textContent = t(plein ? "comparer.ajouter.max" : "comparer.ajouter.chargement");
    if (ajouter.disabled) choix.hidden = true;
    ajouter.setAttribute("aria-expanded", String(!choix.hidden));
  }

  function dessinerPuces() {
    puces.replaceChildren(...choisis.map((c, i) => {
      const li = el("li");
      const p = comparaison && comparaison.profils[i];
      li.append(carreSerie(i + 1), el("span", "", p ? p.nom : c.nom));
      if (i === 0) li.append(el("span", "label", t("comparer.reference")));
      const retirer = el("button", "btn", t("comparer.retirer"));
      retirer.addEventListener("click", () => {
        choisis.splice(i, 1);
        masques.clear();
        analyser();
      });
      li.append(retirer);
      if (p && p.etat.etat === "refuse") li.append(el("p", "why", texteRefus(p.etat)));
      return li;
    }));
    dessinerAjout();
  }

  function ajouterSource(source, nom) {
    if (choisis.length >= MAX) return;
    choisis.push({ source, nom });
    choix.hidden = true;
    analyser();
  }

  function dessinerChoix() {
    const liste = el("div", "choix-liste");
    for (const p of comparables || []) {
      const morceaux = [`${p.imprimante} › ${p.papier} › ${p.resolution}`, window.Arbre.date(p.horodatage)];
      if (p.actif) morceaux.push(t("comparer.choix.actuel"));
      const b = el("button", "btn", morceaux.join(" · "));
      b.title = p.nom;
      b.addEventListener("click", () => ajouterSource({ type: "bibliotheque", profil: p.profil }, p.nom));
      liste.append(b);
    }
    const fichier = el("button", "btn", t("comparer.choix.fichier"));
    fichier.addEventListener("click", ouvrirFichier);
    choix.replaceChildren(el("span", "label", t("comparer.choix.titre")), liste);
    if (!(comparables || []).length) choix.append(el("p", "why", t("comparer.choix.vide")));
    const actions = el("div", "actions");
    actions.append(fichier);
    choix.append(actions);
  }

  async function ouvrirFichier() {
    annoncer("");
    try {
      const f = await invoke("comparer_ouvrir_fichier", { filtre: t("parc.filtre.icc") });
      if (f) ajouterSource({ type: "fichier", jeton: f.jeton }, f.nom); // annulé : rien ne change
    } catch (cle) {
      annoncer(texteErreur(cle));
    }
  }

  async function chargerComparables() {
    try {
      comparables = await invoke("comparer_profils_comparables");
    } catch (cle) {
      comparables = [];
      annoncer(texteErreur(cle));
    }
    dessinerAjout();
    dessinerChoix();
  }

  // ---- Calcul : analyse de tous les profils, puis courbes du chemin ----
  const rvbDe = (nom) => (BASES.find((b) => b[0] === nom) || BASES[4])[1];

  // Rend vrai si la réponse est celle de la dernière demande de chemin.
  let demandeChemin = 0;
  async function chargerChemin() {
    const n = ++demandeChemin;
    let reponse = [];
    try {
      reponse = await invoke("comparer_chemin", {
        sources: choisis.map((c) => c.source),
        depart: rvbDe(reglages.depart),
        arrivee: rvbDe(reglages.arrivee),
      });
    } catch (cle) {
      if (n === demandeChemin) annoncer(texteErreur(cle));
    }
    if (n !== demandeChemin) return false;
    chemins = reponse;
    return true;
  }

  async function analyser() {
    const n = ++demande;
    annoncer("");
    comparaison = null;
    chemins = [];
    dessinerPuces();
    if (!choisis.length) { enCours.hidden = true; dessiner(); return; }
    enCours.hidden = false;
    dessiner();
    try {
      const reponse = await invoke("comparer_analyser", { sources: choisis.map((c) => c.source) });
      // Périmée : la demande plus récente tient « en cours ».
      if (n !== demande) return;
      comparaison = reponse;
      reglages.forme = null;
      await chargerChemin();
    } catch (cle) {
      if (n === demande) annoncer(texteErreur(cle));
    }
    if (n !== demande) return;
    enCours.hidden = true;
    // L'image ouverte est analysée de nouveau face aux profils affichés.
    image.profilsChanges(choisis.map((c) => c.source));
    dessinerPuces();
    dessiner();
  }

  // Un profil de la comparaison, avec son rang, sa couleur de série (rang + 1) et, s'il est
  // analysé, son analyse, sa limite, sa fidélité et sa répétabilité (null s'il est refusé).
  function profilVu(p, i) {
    const e = p.etat.etat === "analyse" ? p.etat : {};
    return {
      ...p, rang: i, serie: i + 1, analyse: e.analyse || null, limite: e.limite || null,
      fidelite: e.fidelite || null, repetabilite: e.repetabilite || null,
    };
  }

  function analyses() {
    return comparaison ? comparaison.profils.map(profilVu).filter((p) => p.analyse) : [];
  }

  // ---- Gamut en volume ----
  function formes(liste) {
    return liste.filter((p) => !masques.has(p.rang)).map((p) => {
      const limite = reglages.forme === "limite" && p.limite.etat === "calculee";
      const m = limite ? p.limite.maillage : p.analyse.affichage;
      return { ...m, serie: p.serie, nom: p.nom, volume: limite ? p.limite.volume : p.analyse.volume_surface, blanc: p.analyse.blanc };
    });
  }

  // Partie repliable (accordeon.js) : la vue 3D n'existe que partie ouverte, et se défait à la fermeture.
  function volume3d(liste) {
    return section("comparer.volume", (zone) => remplirVolume(zone, liste), () => {
      if (vue3d) { vue3d.detruire(); vue3d = null; }
    });
  }

  function remplirVolume(zone, liste) {
    const calculees = liste.filter((p) => p.limite.etat === "calculee");
    if (reglages.forme === null) reglages.forme = calculees.length ? "limite" : "surface";
    const inactives = {};
    if (!calculees.length) {
      const sans = liste.find((p) => p.limite.etat === "indisponible");
      inactives.limite = sans ? texteLimite(sans.limite) : t("comparer.limite.sans_argyll");
    }
    const affiches = el("div", "field");
    affiches.append(el("span", "label", t("comparer.volume.affiches")));
    const cases = el("div", "choix-liste");
    for (const p of liste) {
      const rang = el("label", "choix-liste__rang");
      const c = el("input");
      c.type = "checkbox";
      c.checked = !masques.has(p.rang);
      c.addEventListener("change", () => {
        if (c.checked) masques.delete(p.rang); else masques.add(p.rang);
        vue3d.afficher(formes(liste));
      });
      rang.append(c, carreSerie(p.serie), p.nom);
      cases.append(rang);
    }
    affiches.append(cases);
    // Couleur et rendu se règlent sans refaire la vue : elle garde sa rotation.
    const regler = (cle) => (v) => { reglages[cle] = v; vue3d.regler({ [cle]: v }); };
    // Avec la limite choisie, un profil dont elle manque montre sa surface brute, et dit pourquoi.
    const manques = el("div");
    const dessinerManques = () => manques.replaceChildren(...(reglages.forme !== "limite" ? [] : liste
      .filter((x) => x.limite.etat !== "calculee")
      .map((p) => el("p", "why", t("comparer.volume.surface_montree")
        .replace("{nom}", () => p.nom).replace("{raison}", () => texteLimite(p.limite))))));
    const changerForme = (v) => { reglages.forme = v; vue3d.afficher(formes(liste)); dessinerManques(); };
    const canvas = el("canvas", "vue3d");
    // En couleurs réelles, la vue 3D habille la référence seule ; l'aide dit pourquoi les autres sont en fil de fer.
    const aideCouleur = seg("comparer.volume.couleur", ["serie", "reelle"], reglages.couleur, regler("couleur"));
    aideCouleur.append(el("p", "", t("comparer.volume.couleur.aide")));
    zone.append(affiches, aideCouleur,
      seg("comparer.volume.forme", ["limite", "surface"], reglages.forme, changerForme, inactives),
      seg("comparer.volume.rendu", ["opaque", "transparent"], reglages.rendu, regler("rendu")),
      manques, window.NuageImage.champ(), canvas, el("p", "why", t("comparer.volume.aide")));
    dessinerManques();
    vue3d = window.Vue3d.creer(canvas, { couleur: reglages.couleur, rendu: reglages.rendu });
    vue3d.afficher(formes(liste));
    vue3d.points(window.NuageImage.points());
  }

  // ---- Graphiques : teintes, coupe, chemin ----
  const svg = () => document.createElementNS("http://www.w3.org/2000/svg", "svg");

  // Choix d'une couleur de base du chemin ; `redessiner` refait le graphique du chemin.
  function selection(cle, redessiner) {
    const c = el("div", "field");
    const s = el("select", "input");
    s.id = "comparer-" + cle;
    for (const [nom] of BASES) {
      const o = el("option", "", t("comparer.primaire." + nom));
      o.value = nom;
      o.selected = reglages[cle] === nom;
      s.append(o);
    }
    s.addEventListener("change", async () => {
      reglages[cle] = s.value;
      if (await chargerChemin()) redessiner();
    });
    const l = el("label", "label", t("comparer.chemin." + cle));
    l.htmlFor = s.id;
    c.append(l, s);
    return c;
  }

  // Trois parties repliables : chaque graphique se dessine à l'ouverture, une fois dans la page (le
  // tableau « Valeurs » suit son SVG) ; replié, il ne dessine rien.
  function graphiques(liste) {
    const serie = (p, valeurs) => ({ valeurs, serie: p.serie, nom: p.nom });
    const s1 = section("comparer.teintes", (zone) => {
      const teintes = svg();
      zone.append(teintes);
      window.Graphiques.teintes(teintes, liste.map((p) => serie(p, p.analyse.chroma_par_teinte)));
    });
    const s2 = section("comparer.coupe", (zone) => {
      const coupe = svg();
      const curseur = el("input", "curseur");
      Object.assign(curseur, { type: "range", min: 5, max: 95, step: 1, value: reglages.l, id: "comparer-clarte" });
      const libelle = el("label", "label", `${t("comparer.coupe.clarte")} ${nombre(reglages.l, 0)}`);
      libelle.htmlFor = curseur.id;
      const champ = el("div", "field");
      champ.append(libelle, curseur);
      zone.append(champ, coupe);
      const dessinerCoupe = () => {
        libelle.textContent = `${t("comparer.coupe.clarte")} ${nombre(reglages.l, 0)}`;
        window.Graphiques.coupe(coupe, liste.map((p) => serie(p, p.analyse.coupes[reglages.l])), reglages.l, window.NuageImage.points());
      };
      coupeNuage = dessinerCoupe;
      curseur.addEventListener("input", () => { reglages.l = Number(curseur.value); dessinerCoupe(); });
      dessinerCoupe();
    }, () => { coupeNuage = () => {}; });
    const s3 = section("comparer.chemin", (zone) => {
      const cheminL = svg();
      const cheminC = svg();
      const dessinerChemin = () => window.Graphiques.chemin(cheminL, cheminC, liste.map((p) => ({
        ...serie(p, (chemins[p.rang] || {}).points || []),
        mesures: (chemins[p.rang] || {}).mesures || [],
      })));
      zone.append(selection("depart", dessinerChemin), selection("arrivee", dessinerChemin), cheminL, cheminC);
      dessinerChemin();
    });
    corps.append(s1, s2, s3);
  }

  // ---- Dessin de toute la vue ----
  function dessiner() {
    if (vue3d) { vue3d.detruire(); vue3d = null; }
    coupeNuage = () => {};
    corps.replaceChildren();
    details.replaceChildren();
    if (!ouverte) return;
    if (!choisis.length) { corps.append(el("h2", "empty__text", t("comparer.vide"))); return; }
    if (!comparaison) return;
    const liste = analyses();
    if (liste.length) {
      corps.append(chiffres(liste), image.section(liste), volume3d(liste));
      graphiques(liste);
      corps.append(section("comparer.primaires", window.ComparerEcarts.primaires(liste)));
    }
    corps.append(window.ComparerEcarts.ecarts(comparaison, comparaison.profils.map(profilVu), texteFidelite), methode());
    details.replaceChildren(...comparaison.profils.map(provenance), avance(comparaison.commande_limite, liste));
  }

  // ---- Ouverture et fermeture : la vue prend la place de celle d'une résolution ----
  function ouvrir(sources) {
    ouverte = true;
    choisis = sources.slice(0, MAX).map((source) => ({ source, nom: "" }));
    masques.clear();
    choix.hidden = true;
    vue.hidden = false;
    details.hidden = false;
    document.dispatchEvent(new CustomEvent("comparer-ouverte"));
    chargerComparables();
    analyser();
  }

  function fermer() {
    if (!ouverte) return;
    ouverte = false;
    demande++;
    enCours.hidden = true;
    vue.hidden = true;
    details.hidden = true;
    image.fermer();
    dessiner();
    document.dispatchEvent(new CustomEvent("comparer-fermee"));
  }

  ajouter.addEventListener("click", () => {
    choix.hidden = !choix.hidden;
    dessinerAjout();
  });
  vue.querySelector("[data-comparer-fermer]").addEventListener("click", fermer);
  document.addEventListener("langue-appliquee", () => {
    if (!ouverte) return;
    dessinerPuces();
    dessinerChoix();
    dessiner();
  });

  return { ouvrir, fermer, ouverte: () => ouverte };
})();
