// Vue d'une résolution d'impression, choisie dans l'arbre (parc.js) : au centre, le profil actif
// (verdict couleur + icône + mot), le seul bouton principal « Installer dans Windows », exporter,
// ranger un profil existant, calculer un profil depuis une mesure importée, l'historique et les
// mesures ; à droite, la provenance du profil. Les actions passent par les modules Rust `parc` et
// `profiler` ; les textes, par le catalogue (app.js).
"use strict";

window.Reglage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const vue = document.querySelector("[data-vue-reglage]");
  const details = document.querySelector("[data-details-reglage]");
  const annonce = vue.querySelector("[data-reglage-message]");
  // Ce que parc.js prête à la vue : la résolution affichée et de quoi recharger.
  let parc = { reglage: () => null, rafraichir: async () => {}, recharger: async () => {} };

  const { el, date } = window.Arbre;

  // Verdict : couleur, icône et mot, jamais la couleur seule.
  const TRACES = {
    conforme: "M5 12l4 4 10-10",
    limite: "M12 4v10M12 18v2",
    hors: "M6 6l12 12M18 6L6 18",
    aucun: "M5 12h14",
  };
  const VERDICTS = {
    accepte: ["conforme", "parc.verdict.verifie"],
    non_verifie: ["limite", "parc.verdict.non_verifie"],
    hors_tolerance: ["hors", "parc.verdict.hors"],
    incomplet: ["limite", "parc.verdict.incomplet"],
  };

  function annoncer(texte) {
    annonce.textContent = texte;
    annonce.hidden = !texte;
  }

  // Une erreur arrive en clé du catalogue, ou en objet {cle, cause} (calcul d'un profil) :
  // le texte de la clé, puis ce qu'a dit ArgyllCMS.
  function texteErreur(erreur) {
    if (!erreur || typeof erreur !== "object") return t(String(erreur));
    const texte = t(String(erreur.cle));
    return erreur.cause ? `${texte} ${t("profil.erreur.cause").replace("{cause}", erreur.cause)}` : texte;
  }

  const nombre = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v);

  // Pendant une action (fenêtre de Windows ouverte, installation, calcul), tous les boutons de
  // la vue sont inactifs et la raison « en cours » est dite juste à côté : un double clic ne
  // fait rien. Un bouton marqué data-inactif l'est aussi hors de toute action.
  const BOUTONS = "[data-installer], [data-exporter-profil], [data-ranger-profil], [data-calcul-ouvrir], [data-calcul-profil] button, [data-calcul-profil] input";
  const occupe = vue.querySelector("[data-reglage-occupe]");
  let enCours = false;

  function dessinerOccupation(sansProfil) {
    for (const b of vue.querySelectorAll(BOUTONS)) {
      b.disabled = enCours || b.hasAttribute("data-inactif") || (sansProfil && b.hasAttribute("data-installer"));
    }
    occupe.hidden = !enCours;
    dessinerRaisonCalcul();
  }

  // ---- Calcul d'un profil (ticket #40) : section ouverte par « Calculer un profil » ----
  const installer = vue.querySelector("[data-installer]");
  const ouvrir = vue.querySelector("[data-calcul-ouvrir]");
  const calcul = vue.querySelector("[data-calcul-profil]");
  const RECETTES = ["rapide", "standard", "approfondi"];
  let ouvert = false;
  let avanceOuvert = false;
  // Choix de la section, gardé tant qu'on reste dans la même résolution.
  let choix = { condition: null, mesure: null, mesureCondition: null, recette: "standard" };
  let calculEnCours = false;
  let erreurCalcul = ""; // texte de la dernière erreur, dit à côté du bouton
  let raisonCalcul = null; // <p class="why"> sous « Calculer le profil »
  let commandesBloc = null;
  let commandes = "";
  let demande = 0; // dernière demande de commandes : une réponse plus ancienne est ignorée

  function dessinerRaisonCalcul() {
    if (!raisonCalcul) return;
    const texte = calculEnCours ? t("profil.en_cours")
      : erreurCalcul || (choix.mesure === null ? t("profil.calculer.raison") : "");
    raisonCalcul.textContent = texte;
    raisonCalcul.hidden = !texte;
  }

  // La mesure proposée : celle déjà choisie si elle peut servir, sinon la plus récente qui peut
  // servir ; la condition M1 si la mesure l'a, sinon la première qu'elle a.
  function choisirParDefaut(d) {
    if (choix.condition !== d.condition.id) {
      choix = { condition: d.condition.id, mesure: null, mesureCondition: null, recette: "standard" };
    }
    const utilisables = d.sources.filter((s) => !s.raison);
    const source = utilisables.find((s) => s.mesure === choix.mesure) || utilisables[0] || null;
    choix.mesure = source ? source.mesure : null;
    const conditions = source ? source.conditions : [];
    if (!conditions.includes(choix.mesureCondition)) {
      choix.mesureCondition = conditions.includes("M1") ? "M1" : conditions[0] || null;
    }
    return source;
  }

  function changer(modification) {
    modification();
    erreurCalcul = "";
    const d = parc.reglage();
    if (d) redessinerCalcul(d);
  }

  function champ(cle, ...contenu) {
    const f = el("div", "field");
    f.append(el("span", "label", t(cle)), ...contenu);
    return f;
  }

  // Une mesure qui ne peut pas servir reste dans la liste, inactive, avec sa raison dessous.
  function champSource(d) {
    const liste = el("div", "choix-liste");
    for (const s of d.sources) {
      const ligne = el("div");
      const rang = el("label", "choix-liste__rang");
      const radio = el("input");
      radio.type = "radio";
      radio.name = "calcul-source";
      radio.checked = s.mesure === choix.mesure;
      if (s.raison) radio.dataset.inactif = "";
      radio.addEventListener("change", () => changer(() => { choix.mesure = s.mesure; }));
      const nom = el("span", "", s.fichier);
      nom.title = s.fichier;
      rang.append(radio, nom);
      ligne.append(rang);
      if (s.raison) ligne.append(el("p", "why", t(s.raison)));
      liste.append(ligne);
    }
    return champ("profil.source", liste);
  }

  function puces(valeurs, choisie, libelle, choisir) {
    const p = el("div", "puces");
    for (const v of valeurs) {
      const b = el("button", "", libelle(v));
      b.type = "button";
      b.setAttribute("aria-pressed", String(v === choisie));
      b.addEventListener("click", () => changer(() => choisir(v)));
      p.append(b);
    }
    return p;
  }

  // Avancé, replié : les commandes équivalentes, en lecture seule.
  function blocCommandes() {
    const avance = el("details", "replie");
    avance.open = avanceOuvert;
    avance.addEventListener("toggle", () => { avanceOuvert = avance.open; });
    commandesBloc = el("pre", "", commandes);
    avance.append(el("summary", "", t("profil.avance")), commandesBloc);
    return avance;
  }

  function dessinerCalcul(d) {
    ouvrir.setAttribute("aria-expanded", String(ouvert));
    calcul.hidden = !ouvert;
    // Section ouverte : « Calculer le profil » est le seul bouton principal de la vue.
    installer.classList.toggle("btn--primary", !ouvert);
    raisonCalcul = null;
    commandesBloc = null;
    if (!ouvert) return calcul.replaceChildren();
    const corps = el("div", "section section--serree");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("profil.titre")));
    corps.append(tete);
    const source = choisirParDefaut(d);
    if (!d.sources.length) {
      // Le bouton d'import existant (bibliotheque.js), qui propose cette résolution.
      const importer = el("button", "btn", t("export.importer"));
      importer.addEventListener("click", () => document.querySelector("[data-export] [data-importer]").click());
      const actions = el("div", "actions");
      actions.append(importer);
      corps.append(el("p", "prose", t("profil.sans_mesure")), actions);
      return calcul.replaceChildren(corps);
    }
    corps.append(champSource(d));
    if (source) {
      corps.append(champ("profil.condition",
        puces(source.conditions, choix.mesureCondition, (c) => c, (c) => { choix.mesureCondition = c; })));
    }
    const recette = champ("profil.recette",
      puces(RECETTES, choix.recette, (r) => t("profil.recette.nom." + r), (r) => { choix.recette = r; }));
    recette.append(el("p", "", t("profil.recette." + choix.recette)));
    corps.append(recette, blocCommandes());
    const actions = el("div", "actions");
    const calculer = el("button", "btn btn--primary", t("profil.calculer"));
    if (choix.mesure === null) calculer.dataset.inactif = "";
    calculer.addEventListener("click", () => agir(calculerProfil));
    actions.append(calculer);
    raisonCalcul = el("p", "why");
    raisonCalcul.setAttribute("role", "status");
    corps.append(actions, raisonCalcul);
    calcul.replaceChildren(corps);
  }

  function parametres(d) {
    return { condition: d.condition.id, mesure: choix.mesure, mesureCondition: choix.mesureCondition, recette: choix.recette };
  }

  // Les commandes suivent chaque changement de choix ; rien n'est calculé.
  async function chargerCommandes(d) {
    const n = ++demande;
    let texte = "";
    if (ouvert && choix.mesure !== null && choix.mesureCondition) {
      try {
        texte = (await invoke("profiler_commandes", parametres(d))).join("\n");
      } catch (erreur) {
        texte = texteErreur(erreur);
      }
    }
    if (n !== demande) return;
    commandes = texte;
    if (commandesBloc) commandesBloc.textContent = texte;
  }

  function redessinerCalcul(d) {
    dessinerCalcul(d);
    dessinerOccupation(!d.actif);
    chargerCommandes(d);
  }

  // Le calcul peut durer plusieurs minutes : la vue attend, puis se recharge sur le nouveau profil.
  async function calculerProfil(d) {
    erreurCalcul = "";
    calculEnCours = true;
    dessinerRaisonCalcul();
    try {
      const r = await invoke("profiler_calculer", parametres(d));
      ouvert = false;
      await parc.recharger();
      annoncer(t("profil.fait").replace("{moyenne}", nombre(r.precision_moyenne)).replace("{max}", nombre(r.precision_max)));
    } catch (erreur) {
      erreurCalcul = texteErreur(erreur);
    }
    calculEnCours = false;
  }

  ouvrir.addEventListener("click", () => changer(() => { ouvert = !ouvert; }));

  function remplirVue(d, lignesMesures) {
    const titre = vue.querySelector("[data-reglage-titre]");
    titre.textContent = `${d.papier} · ${d.resolution}`;
    titre.title = `${d.imprimante} · ${titre.textContent}`;
    const actif = d.actif;
    // Un état que la page ne connaît pas encore est montré comme « non vérifié », sans rien casser.
    const [classe, mot] = actif ? VERDICTS[actif.etat] || VERDICTS.non_verifie : ["aucun", "parc.verdict.aucun"];
    vue.querySelector("[data-reglage-verdict]").className = "carte carte--verdict verdict--" + classe;
    vue.querySelector("[data-verdict-trace]").setAttribute("d", TRACES[classe]);
    vue.querySelector("[data-verdict-mot]").textContent = t(mot);
    vue.querySelector("[data-verdict-contexte]").textContent = actif
      ? [t("parc.verdict.contexte"), date(actif.horodatage), t("parc.espace." + actif.espace)]
        .concat(d.ancien ? [t("parc.age.plus_d_un_an")] : []).join(" · ")
      : t("parc.verdict.aucun_contexte");

    // Un seul bouton principal ; sans profil, il est inactif et dit pourquoi, juste à côté.
    vue.querySelector("[data-installer-raison]").hidden = Boolean(actif) || enCours;
    vue.querySelector("[data-exporter-profil]").hidden = !actif;
    // Une autre résolution : la section se referme et ses choix repartent de zéro.
    if (choix.condition !== d.condition.id) {
      ouvert = false;
      erreurCalcul = "";
    }
    redessinerCalcul(d);

    const historique = vue.querySelector("[data-historique]");
    historique.replaceChildren(...d.historique.map((p) => {
      const ligne = el("div", "cle-valeur mute");
      const quand = el("span", "", `${date(p.horodatage)} · ${t(p.actif ? "parc.historique.actuel" : "parc.historique.remplace")}`);
      quand.title = p.nom;
      const etat = textes["parc.etat." + p.etat] ? "parc.etat." + p.etat : "parc.etat.non_verifie";
      ligne.append(quand, el("span", "", t(etat)));
      return ligne;
    }));
    if (!d.historique.length) historique.append(el("p", "why", t("parc.historique.vide")));

    const mesures = vue.querySelector("[data-reglage-mesures]");
    const liste = el("ul", "arbre");
    liste.setAttribute("role", "tree");
    liste.append(...lignesMesures(d));
    mesures.replaceChildren(liste);
    if (!liste.children.length) mesures.append(el("p", "why", t("parc.mesures.vide")));
  }

  function section(titre, lignes) {
    const s = el("details", "section-repliable");
    s.open = true;
    const corps = el("div", "section-repliable__corps");
    corps.append(...lignes);
    s.append(el("summary", "", t(titre)), corps);
    return s;
  }

  function cleValeur(cle, valeur) {
    const l = el("div", "cle-valeur");
    const v = el("b", "", valeur);
    v.title = valeur;
    l.append(el("span", "", t(cle)), v);
    return l;
  }

  // Instrument d'un profil calculé : celui que déclare le fichier de la mesure source.
  function instrumentDeclare(d) {
    const source = d.provenance && d.importees.find((m) => m.id === d.provenance.mesure);
    const info = source ? source.instrument : null;
    if (!info || info.statut === "inconnue") return t("parc.inconnu");
    return info.statut === "supposee" ? `${info.valeur} (${t("details.a_confirmer")})` : info.valeur;
  }

  // Mesure source, condition, recette, version d'ArgyllCMS et précision d'un profil calculé.
  // La précision est celle du calcul : le verdict « non vérifié » reste à part.
  function provenanceCalcul(p) {
    if (!p) return [];
    const condition = p.condition_supposee ? `${p.condition} (${t("profil.supposee")})` : p.condition;
    const precision = t("profil.precision.valeur")
      .replace("{moyenne}", nombre(p.precision_moyenne)).replace("{max}", nombre(p.precision_max));
    return [
      cleValeur("profil.source", p.fichier),
      cleValeur("profil.condition", condition),
      cleValeur("profil.recette", t("profil.recette.nom." + p.recette)),
      cleValeur("profil.version", p.version),
      cleValeur("profil.precision", precision),
    ];
  }

  // Provenance du profil à droite : ce qui n'est pas connu est écrit « inconnu » en toutes lettres.
  function remplirDetails(d) {
    const a = d.actif;
    const provenance = a
      ? [
        cleValeur("parc.origine", t("parc.origine." + d.origine)),
        cleValeur("parc.nom", a.nom),
        cleValeur("parc.empreinte", a.empreinte.slice(0, 12)),
        ...provenanceCalcul(d.provenance),
        cleValeur("parc.instrument", instrumentDeclare(d)),
        cleValeur("parc.mire", t("parc.inconnue")),
      ]
      : [el("p", "", t("parc.verdict.aucun"))];
    details.replaceChildren(
      section("parc.provenance", provenance),
      section("parc.fiche", [cleValeur("parc.fiche.reglages", t("parc.inconnue"))]),
      section("parc.windows", [cleValeur("parc.windows.etat",
        t(!a ? "parc.windows.sans_profil" : d.installe ? "installation.installe" : "parc.windows.non_installe"))]),
    );
  }

  // ---- Actions sur le profil : la fenêtre de Windows s'ouvre côté Rust ; annuler ne change rien ----
  async function agir(faire) {
    const d = parc.reglage();
    if (enCours || !d) return;
    enCours = true;
    dessinerOccupation(!d.actif);
    annoncer("");
    try {
      await faire(d);
    } catch (erreur) {
      annoncer(texteErreur(erreur));
    }
    enCours = false;
    dessinerOccupation(!d.actif);
    await parc.rafraichir();
  }

  vue.querySelector("[data-ranger-profil]").addEventListener("click", () => agir(async (d) => {
    const id = await invoke("parc_ranger_profil", { condition: d.condition.id, filtre: t("parc.filtre.icc") });
    if (id === null) return; // fenêtre annulée : rien ne change, aucun message
    await parc.recharger();
    annoncer(t("parc.range"));
  }));
  vue.querySelector("[data-exporter-profil]").addEventListener("click", () => agir(async (d) => {
    const nom = await invoke("parc_exporter_profil", { profil: d.actif.id, filtre: t("parc.filtre.icc") });
    if (nom) annoncer(`${t("export.fait")} ${nom}`);
  }));
  vue.querySelector("[data-installer]").addEventListener("click", () => agir(async (d) => {
    const etat = await invoke("parc_installer_profil", { profil: d.actif.id });
    annoncer(t(etat === "deja_installe" ? "parc.deja_installe" : "installation.installe"));
  }));

  return {
    brancher: (fonctions) => { parc = fonctions; },
    remplir: (d, lignesMesures) => { remplirVue(d, lignesMesures); remplirDetails(d); },
    annoncer,
  };
})();
