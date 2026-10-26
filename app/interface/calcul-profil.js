// Calcul d'un profil depuis la vue d'une résolution (ticket #40) : la section ouverte par
// « Calculer un profil » (mesure importée, condition de mesure, recette, commande équivalente),
// le calcul lui-même et, à droite, la provenance d'un profil calculé. Utilisé par reglage.js,
// qui prête la résolution affichée et le mécanisme agir(). Les calculs passent par le module
// Rust `profiler` ; les textes, par le catalogue (`textes`, chargé par app.js).
"use strict";

window.CalculProfil = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const vue = document.querySelector("[data-vue-reglage]");
  const installer = vue.querySelector("[data-installer]");
  const ouvrir = vue.querySelector("[data-calcul-ouvrir]");
  const calcul = vue.querySelector("[data-calcul-profil]");
  const RECETTES = ["rapide", "standard", "approfondi"];
  // Ce que reglage.js prête : la résolution affichée, de quoi recharger, annoncer, agir et
  // redessiner l'occupation des boutons.
  let parc = {
    reglage: () => null, recharger: async () => {}, annoncer: () => {}, agir: async () => {}, occupation: () => {},
  };

  let ouvert = false;
  let avanceOuvert = false;
  // Choix de la section, gardé tant qu'on reste dans la même résolution.
  let choix = { condition: null, mesure: null, mesureCondition: null, recette: "standard" };
  let calculEnCours = false;
  let erreurCalcul = ""; // dernière erreur du calcul, dite sous le bouton
  let erreurCommandes = ""; // erreur des commandes (ArgyllCMS introuvable), dite sous le bouton
  let raisonCalcul = null; // <p class="why"> sous « Calculer le profil »
  let commandesBloc = null;
  let commandes = "";
  let demande = 0; // dernière demande de commandes : une réponse plus ancienne est ignorée

  // Avancement du calcul en cours, reçu du module `profiler` (événement « profil-avancement ») :
  // l'étape, puis la dernière ligne écrite par ArgyllCMS et le pourcentage qu'il a écrit lui-même
  // pour la partie en cours. Rien n'est estimé. Le temps écoulé est redessiné chaque seconde.
  const ETAPES = {
    preparation: "profil.etape.preparation",
    profil: "profil.etape.profil",
    controle: "profil.etape.controle",
  };
  let avancement = { etape: "preparation", ligne: "", pourcentage: null, debut: 0 };
  let minuterie = null;

  const duree = (ms) => {
    const s = Math.max(0, Math.floor(ms / 1000));
    const m = Math.floor(s / 60);
    return m ? `${m} min ${String(s % 60).padStart(2, "0")} s` : `${s} s`;
  };

  // Le seul message « en cours » : la phrase d'attente, l'étape et le temps écoulé, puis ce
  // qu'ArgyllCMS a écrit en dernier.
  function texteEnCours() {
    const ecoule = t("profil.ecoule").replace("{duree}", () => duree(Date.now() - avancement.debut));
    const lignes = [t("profil.en_cours"), `${t(ETAPES[avancement.etape])} · ${ecoule}`];
    if (avancement.ligne) {
      lignes.push(avancement.pourcentage === null
        ? t("profil.argyll.ligne").replace("{ligne}", () => avancement.ligne)
        : t("profil.argyll.pourcentage").replace("{ligne}", () => avancement.ligne)
          .replace("{pourcentage}", () => String(avancement.pourcentage)));
    }
    return lignes.join("\n");
  }

  window.__TAURI__.event.listen("profil-avancement", (evenement) => {
    if (!calculEnCours) return;
    const a = evenement.payload || {};
    if (a.etape in ETAPES) {
      avancement = { ...avancement, etape: a.etape, ligne: "", pourcentage: null };
    } else if (typeof a.ligne === "string") {
      avancement = { ...avancement, ligne: a.ligne, pourcentage: null };
    } else if (typeof a.pourcentage === "number") {
      avancement = { ...avancement, pourcentage: a.pourcentage };
    }
    dessinerRaison();
  });

  // Une erreur arrive en clé du catalogue, ou en objet {cle, cause, version} (calcul d'un
  // profil) : le texte de la clé, avec la version refusée à sa place, puis ce qu'a dit ArgyllCMS,
  // inséré tel quel. Tout le reste (objet sans clé, exception de la page) est une erreur
  // inattendue.
  function texteErreur(erreur) {
    if (typeof erreur === "string") return t(erreur);
    if (!erreur || typeof erreur.cle !== "string") {
      console.error(erreur);
      return t("parc.erreur.inattendue");
    }
    let texte = t(erreur.cle);
    if (erreur.version) texte = texte.replace("{version}", () => erreur.version);
    return erreur.cause ? `${texte} ${t("profil.erreur.cause").replace("{cause}", () => erreur.cause)}` : texte;
  }

  const nombre = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v);

  function dessinerRaison() {
    if (!raisonCalcul) return;
    const texte = calculEnCours ? texteEnCours()
      : erreurCalcul || erreurCommandes || (choix.mesure === null ? t("profil.calculer.raison") : "");
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
    if (d) redessiner(d);
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

  function dessinerSection(d) {
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
      importer.addEventListener("click", () => {
        const bouton = document.querySelector("[data-export] [data-importer]");
        if (bouton) bouton.click();
      });
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
    calculer.addEventListener("click", () => parc.agir(calculerProfil));
    actions.append(calculer);
    raisonCalcul = el("p", "why");
    raisonCalcul.setAttribute("role", "status");
    corps.append(actions, raisonCalcul);
    calcul.replaceChildren(corps);
  }

  function parametres(d) {
    return { condition: d.condition.id, mesure: choix.mesure, mesureCondition: choix.mesureCondition, recette: choix.recette };
  }

  // Les commandes suivent chaque changement de choix ; rien n'est calculé. Elles se montrent
  // même sans ArgyllCMS ; la raison qui empêchera le calcul, ou une erreur, se lit sous le
  // bouton avant le clic, pas dans le bloc replié.
  async function chargerCommandes(d) {
    const n = ++demande;
    commandes = "";
    erreurCommandes = "";
    if (ouvert && choix.mesure !== null && choix.mesureCondition) {
      try {
        const reponse = await invoke("profiler_commandes", parametres(d));
        if (n === demande) {
          commandes = reponse.commandes.join("\n");
          erreurCommandes = reponse.moteur ? texteErreur(reponse.moteur) : "";
        }
      } catch (erreur) {
        if (n === demande) erreurCommandes = texteErreur(erreur);
      }
    }
    if (n !== demande) return;
    if (commandesBloc) commandesBloc.textContent = commandes;
    dessinerRaison();
  }

  function redessiner(d) {
    dessinerSection(d);
    parc.occupation();
    chargerCommandes(d);
  }

  // Le calcul peut durer plusieurs minutes : la vue attend, puis se recharge sur le nouveau
  // profil. Si l'opérateur a changé de résolution entre-temps, rien ne s'annonce sur l'autre vue.
  async function calculerProfil(d) {
    const condition = d.condition.id;
    const ici = () => parc.reglage()?.condition.id === condition;
    erreurCalcul = "";
    avancement = { etape: "preparation", ligne: "", pourcentage: null, debut: Date.now() };
    calculEnCours = true;
    clearInterval(minuterie);
    minuterie = setInterval(dessinerRaison, 1000);
    dessinerRaison();
    parc.occupation();
    let r;
    try {
      r = await invoke("profiler_calculer", parametres(d));
    } catch (erreur) {
      calculEnCours = false;
      if (ici()) erreurCalcul = texteErreur(erreur);
      return;
    } finally {
      clearInterval(minuterie);
      minuterie = null;
    }
    calculEnCours = false;
    if (ici()) ouvert = false;
    // Le profil est rangé : un rechargement qui échoue ne fait pas du calcul un échec.
    try {
      await parc.recharger();
    } catch (erreur) {
      console.error(erreur);
    }
    if (ici()) parc.annoncer(t("profil.fait").replace("{moyenne}", nombre(r.precision_moyenne)).replace("{max}", nombre(r.precision_max)));
  }

  ouvrir.addEventListener("click", () => changer(() => { ouvert = !ouvert; }));

  // Une autre résolution : la section se referme et ses choix repartent de zéro.
  function dessiner(d) {
    if (choix.condition !== d.condition.id) {
      ouvert = false;
      erreurCalcul = "";
    }
    dessinerSection(d);
    chargerCommandes(d);
  }

  // Provenance d'un profil calculé, à droite : mesure source, condition, échelle RVB si elle
  // est supposée, recette, version d'ArgyllCMS et précision du calcul (le verdict « non
  // vérifié » reste à part), puis, repliés, la commande et le journal, en lecture seule.
  function provenance(d, cleValeur) {
    const p = d.provenance;
    if (!p) return [];
    const condition = p.condition_supposee ? `${p.condition} (${t("profil.supposee")})` : p.condition;
    const precision = t("profil.precision.valeur")
      .replace("{moyenne}", nombre(p.precision_moyenne)).replace("{max}", nombre(p.precision_max));
    const avance = el("details", "replie");
    avance.append(el("summary", "", t("profil.avance.provenance")), el("pre", "", p.commandes.join("\n")),
      el("span", "label", t("profil.journal")), el("pre", "", p.journal));
    const lignes = [cleValeur("profil.source", p.fichier), cleValeur("profil.condition", condition)];
    if (p.rvb_supposes) lignes.push(cleValeur("profil.rvb", t("profil.rvb.supposee")));
    lignes.push(
      cleValeur("profil.recette", t("profil.recette.nom." + p.recette)),
      cleValeur("profil.version", p.version),
      cleValeur("profil.precision", precision),
      avance,
    );
    return lignes;
  }

  return {
    brancher: (fonctions) => { parc = fonctions; },
    dessiner,
    dessinerRaison,
    // La raison « en cours » du calcul est à l'écran, sous son bouton : le message général se tait.
    raisonVisible: () => calculEnCours && raisonCalcul !== null,
    texteErreur,
    provenance,
  };
})();
