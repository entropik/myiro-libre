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

  // Avancement du calcul en cours. Le module `profiler` envoie, par l'événement « profil-avancement »
  // au numéro du calcul lancé, ce que la barre montre : l'étape, les étapes faites, la position (de
  // 0 à 1, qui ne recule jamais), le pourcentage de la suite en cours écrit par ArgyllCMS, sa
  // dernière ligne, et s'il faut un reflet (rien de réel ne fait avancer la barre). La page dessine
  // cet état ; elle y ajoute le temps écoulé et la durée restante estimée d'après le dernier calcul
  // de la même recette, dite comme telle.
  const NOMS_ETAPES = {
    preparation: "profil.etape.preparation",
    profil: "profil.etape.profil",
    table: "profil.etape.table",
    gamut: "profil.etape.gamut",
    inverses: "profil.etape.inverses",
    limite: "profil.etape.limite",
    controle: "profil.etape.controle",
  };
  const ETAT_DEPART = { etape: "preparation", faites: 0, total: 7, position: 0, pourcentage: null, ligne: null, reflet: true };
  let etat = ETAT_DEPART;
  let debutCalcul = 0;
  let dernierCalcul = Date.now(); // numéro du dernier calcul lancé par cette page
  let minuterie = null;
  const DUREE_REUSSITE = 700; // ms à l'état réussi avant le résultat
  let estimation = null; // durée estimée du calcul choisi (secondes), reçue avec les commandes
  let estimationCalcul = null; // celle du calcul lancé
  // Barre d'avancement, présente pendant le calcul (et, après une erreur, jusqu'au prochain dessin).
  let barre = null;
  let piste = null;
  let remplissage = null;
  let reflet = null;
  let detail = null;
  let annonceEtape = null; // zone lue par les lecteurs d'écran : l'étape seulement

  const duree = (ms) => {
    const s = Math.max(0, Math.floor(ms / 1000));
    const m = Math.floor(s / 60);
    return m ? `${m} min ${String(s % 60).padStart(2, "0")} s` : `${s} s`;
  };
  // Une estimation se dit en minutes rondes : elle n'a pas la précision de la seconde.
  const dureeEstimee = (s) => (s >= 60 ? `${Math.round(s / 60)} min` : `${Math.ceil(s)} s`);
  const libelleEtape = () => t(NOMS_ETAPES[etat.etape] || NOMS_ETAPES.preparation);

  // La seule ligne « en cours », sous la barre : l'étape, le temps écoulé et, s'il y a un
  // historique, le temps restant estimé.
  function ligneEtape() {
    const ecoule = Date.now() - debutCalcul;
    const morceaux = [libelleEtape(), t("profil.ecoule").replace("{duree}", () => duree(ecoule))];
    if (estimationCalcul !== null) {
      const reste = estimationCalcul - ecoule / 1000;
      morceaux.push(reste > 0
        ? t("profil.estimation.restante").replace("{duree}", () => dureeEstimee(reste))
        : t("profil.estimation.depassee"));
    }
    return morceaux.join(" · ");
  }

  // Ce qu'ArgyllCMS a écrit en dernier, avec son pourcentage de la partie en cours : en discret.
  function ligneArgyll() {
    if (!etat.ligne) return "";
    return etat.pourcentage !== null
      ? t("profil.argyll.pourcentage").replace("{ligne}", () => etat.ligne)
        .replace("{pourcentage}", () => String(etat.pourcentage))
      : t("profil.argyll.ligne").replace("{ligne}", () => etat.ligne);
  }

  // Barre et détail, créés une fois puis mis à jour par des propriétés personnalisées (--position,
  // --debut, --part) : la largeur glisse (transition CSS).
  function creerBarre() {
    barre = el("div", "progression");
    piste = el("div", "progression__piste");
    piste.setAttribute("role", "progressbar");
    piste.setAttribute("aria-label", t("profil.progression"));
    piste.setAttribute("aria-valuemin", "0");
    piste.title = t("profil.en_cours");
    remplissage = el("span", "progression__remplissage");
    reflet = el("span", "progression__reflet");
    piste.append(remplissage, reflet);
    detail = el("p", "progression__detail");
    annonceEtape = el("p", "lecteur-ecran");
    annonceEtape.setAttribute("aria-live", "polite");
    barre.append(piste);
    raisonCalcul.before(barre);
    raisonCalcul.after(detail, annonceEtape);
  }

  function oublierBarre() {
    barre = piste = remplissage = reflet = detail = annonceEtape = null;
  }

  function dessinerBarre() {
    if (!calculEnCours || !raisonCalcul) return;
    if (!barre) creerBarre();
    const part = 100 / etat.total;
    piste.style.setProperty("--position", `${etat.position * 100}%`);
    // Le reflet glisse sur la part en cours quand rien de réel ne fait avancer la barre.
    reflet.hidden = !etat.reflet;
    piste.style.setProperty("--debut", `${part * etat.faites}%`);
    piste.style.setProperty("--part", `${part}%`);
    piste.setAttribute("aria-valuemax", String(etat.total));
    piste.setAttribute("aria-valuenow", String(etat.faites));
    piste.setAttribute("aria-valuetext", t("profil.progression.etape")
      .replace("{n}", () => String(etat.faites + 1)).replace("{total}", () => String(etat.total)));
    detail.textContent = ligneArgyll();
    detail.hidden = !detail.textContent;
    // Seul un changement d'étape est annoncé aux lecteurs d'écran, pas le temps qui passe.
    const nomEtape = libelleEtape();
    if (annonceEtape.textContent !== nomEtape) annonceEtape.textContent = nomEtape;
  }

  // Fin du calcul : la barre se remplit et passe à l'état réussi, ou passe à l'état erreur.
  function finirBarre(reussi) {
    if (!piste) return;
    reflet.hidden = true;
    if (reussi) {
      piste.style.setProperty("--position", "100%");
      piste.setAttribute("aria-valuenow", piste.getAttribute("aria-valuemax"));
      piste.dataset.etat = "reussi";
    } else {
      piste.dataset.etat = "erreur";
    }
    if (detail) detail.hidden = true;
  }

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
    const texte = calculEnCours ? ligneEtape()
      : erreurCalcul || erreurCommandes || (choix.mesure === null ? t("profil.calculer.raison") : "");
    raisonCalcul.textContent = texte;
    raisonCalcul.hidden = !texte;
    // Le temps écoulé change chaque seconde : pendant le calcul, la ligne n'est pas lue en continu.
    raisonCalcul.setAttribute("aria-live", calculEnCours ? "off" : "polite");
    dessinerBarre();
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
    oublierBarre();
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
    estimation = null;
    if (ouvert && choix.mesure !== null && choix.mesureCondition) {
      try {
        const reponse = await invoke("profiler_commandes", parametres(d));
        if (n === demande) {
          commandes = reponse.commandes.join("\n");
          erreurCommandes = reponse.moteur ? texteErreur(reponse.moteur) : "";
          estimation = typeof reponse.estimation === "number" ? reponse.estimation : null;
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
    const numero = ++dernierCalcul;
    etat = ETAT_DEPART;
    debutCalcul = Date.now();
    estimationCalcul = estimation;
    calculEnCours = true;
    clearInterval(minuterie);
    minuterie = setInterval(dessinerRaison, 1000);
    dessinerRaison();
    parc.occupation();
    // L'avancement de ce calcul seulement ; l'écouteur est détaché à la fin, réussie ou non.
    const detacher = await window.__TAURI__.event.listen("profil-avancement", (evenement) => {
      if (evenement.payload?.calcul !== numero) return;
      etat = evenement.payload;
      dessinerRaison();
    });
    let r;
    try {
      r = await invoke("profiler_calculer", { ...parametres(d), calcul: numero });
    } catch (erreur) {
      calculEnCours = false;
      // État erreur, sans animation de réussite ; la barre part au prochain dessin.
      finirBarre(false);
      if (ici()) erreurCalcul = texteErreur(erreur);
      return;
    } finally {
      detacher();
      clearInterval(minuterie);
      minuterie = null;
    }
    // Réussi : la barre se remplit, reste un instant à l'état réussi, puis laisse place au résultat.
    finirBarre(true);
    await new Promise((fin) => setTimeout(fin, DUREE_REUSSITE));
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
  // est supposée, recette, version d'ArgyllCMS et précision du calcul (le verdict, « à
  // vérifier », reste à part), puis, repliés, la commande et le journal, en lecture seule.
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
