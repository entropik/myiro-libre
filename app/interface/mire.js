// Préparer une mire depuis la vue d'une résolution (ticket #33) : la section ouverte par
// « Préparer une mire » (instrument, taille et format pour le MYIRO-1, réglages avancés), la
// préparation elle-même, les fichiers à imprimer et les mires déjà préparées. Pour le FD-9, la
// mire est RGB-2520, reprise telle quelle : rien à choisir. Utilisé par reglage.js, qui prête la
// résolution affichée et le mécanisme agir(). Les préparations passent par le module Rust
// `mires` ; les textes, par le catalogue (`textes`, chargé par app.js).
"use strict";

window.PreparerMire = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el, date } = window.Arbre;

  const vue = document.querySelector("[data-vue-reglage]");
  const installer = vue.querySelector("[data-installer]");
  const ouvrirCalcul = vue.querySelector("[data-calcul-ouvrir]");
  const ouvrir = vue.querySelector("[data-mire-ouvrir]");
  const section = vue.querySelector("[data-mire]");
  const TAILLES = ["rapide", "standard", "approfondi"];
  const FORMATS = ["a4", "a3"];
  // Ce que reglage.js prête : la résolution affichée, de quoi recharger, annoncer, agir, dire
  // une erreur et redessiner l'occupation des boutons.
  let parc = {
    reglage: () => null, recharger: async () => {}, annoncer: () => {}, agir: async () => {},
    texteErreur: (e) => String(e), occupation: () => {},
  };

  let ouvert = false;
  let avanceOuvert = false;
  // Choix gardés par résolution pendant la séance, sans stockage.
  const choixParResolution = new Map();
  let choix = null;
  let condition = null;
  // Réponses de mire_apercu : la demande en cours, et chaque taille (nombre de pages).
  let apercu = null;
  let apercusTailles = {};
  let erreurApercu = "";
  let erreurPreparation = "";
  let preparee = null; // dernière mire préparée sur cette résolution
  let demandeApercu = 0;

  const CHOIX_DEPART = { instrument: "fd9", taille: "rapide", format: "a4", cote: "", nombre: "" };

  // Une saisie vide garde la valeur par défaut ; une virgule vaut un point.
  const lireNombre = (texte) => {
    const v = String(texte).trim().replace(",", ".");
    return v === "" ? null : Number(v);
  };
  // Clé de la raison qui refuse une saisie de l'avancé, ou "" : une saisie illisible (« abc »)
  // n'est jamais remplacée en silence par la valeur par défaut.
  function erreurSaisie(texte, entier) {
    const v = lireNombre(texte);
    if (v === null) return "";
    if (!Number.isFinite(v)) return entier ? "mire.saisie.entier" : "mire.saisie.nombre";
    return entier && !Number.isInteger(v) ? "mire.saisie.entier" : "";
  }
  const saisieRefusee = () => (choix.instrument === "myiro1"
    ? erreurSaisie(choix.cote, false) || erreurSaisie(choix.nombre, true) : "");
  const nombreLocal = (v) => new Intl.NumberFormat(document.documentElement.lang).format(v);
  const pages = (n) => (n === 1 ? t("mire.pages.une") : t("mire.pages.plusieurs").replace("{n}", () => String(n)));

  // La demande telle que le module `mires` l'attend (noms de champs du Rust).
  function demande(taille = choix.taille) {
    if (choix.instrument === "fd9") {
      return { instrument: "fd9", taille: null, nombre: null, format: null, cote_mm: null };
    }
    const nombre = lireNombre(choix.nombre);
    return {
      instrument: "myiro1",
      taille: nombre === null ? taille : null,
      nombre,
      format: choix.format,
      cote_mm: lireNombre(choix.cote),
    };
  }

  // Une seule section ouverte à la fois, donc un seul bouton principal dans la vue.
  const calculOuvert = () => ouvrirCalcul.getAttribute("aria-expanded") === "true";
  function dessinerPrincipal() {
    installer.classList.toggle("btn--primary", !ouvert && !calculOuvert());
  }

  function champ(cle, ...contenu) {
    const f = el("div", "field");
    f.append(el("span", "label", t(cle)), ...contenu);
    return f;
  }

  // Cartes de choix : une ligne par valeur, la valeur choisie marquée.
  function cartes(nom, valeurs, choisie, libelle, choisir) {
    const liste = el("div", "choix-liste");
    for (const v of valeurs) {
      const rang = el("label", "choix-liste__rang");
      const radio = el("input");
      radio.type = "radio";
      radio.name = nom;
      radio.checked = v === choisie;
      radio.addEventListener("change", () => changer(() => choisir(v)));
      rang.append(radio, el("span", "", libelle(v)));
      liste.append(rang);
    }
    return liste;
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

  // Vide, une saisie montre en grisé la valeur retenue ; illisible, elle est marquée et sa
  // raison est dite juste dessous.
  function saisie(valeur, retenue, erreur, modifier) {
    const i = el("input", erreur ? "input input--error" : "input");
    i.inputMode = "decimal";
    i.value = valeur;
    if (retenue !== null) i.placeholder = nombreLocal(retenue);
    i.addEventListener("change", () => changer(() => modifier(i.value)));
    const raison = el("p", "why", erreur ? t(erreur) : "");
    raison.hidden = !erreur;
    return [i, raison];
  }

  // Avancé, replié : côté et nombre libre, avec leurs limites, et la commande équivalente.
  // Fermé, il résume ses valeurs.
  function blocAvance() {
    const avance = el("details", "replie");
    avance.open = avanceOuvert;
    avance.addEventListener("toggle", () => { avanceOuvert = avance.open; });
    const resume = apercu
      ? t("mire.avance.resume").replace("{cote}", () => nombreLocal(apercu.cote_mm))
        .replace("{plages}", () => nombreLocal(apercu.plages))
      : "";
    // Le côté minimal vient du module `mires` (aperçu), jamais d'un nombre écrit ici.
    const limites = apercu
      ? t("mire.cote.limites").replace("{min}", () => nombreLocal(apercu.limites_cote[0])) : "";
    const cote = champ("mire.cote",
      ...saisie(choix.cote, apercu ? apercu.cote_mm : null, erreurSaisie(choix.cote, false), (v) => { choix.cote = v; }),
      el("p", "", limites));
    const nombre = champ("mire.nombre",
      ...saisie(choix.nombre, apercu ? apercu.plages : null, erreurSaisie(choix.nombre, true), (v) => { choix.nombre = v; }),
      el("p", "", t("mire.nombre.limites")));
    const commande = champ("mire.commande", el("pre", "", apercu ? apercu.commande : ""));
    avance.append(el("summary", "", `${t("mire.avance")} · ${resume}`), cote, nombre, commande);
    return avance;
  }

  function libelleTaille(taille) {
    const a = apercusTailles[taille];
    const nom = t("mire.taille.nom." + taille);
    return a ? `${nom} · ${t("mire.plages").replace("{n}", () => nombreLocal(a.plages))} · ${pages(a.pages)}` : nom;
  }

  function libelleInstrument(instrument) {
    const detail = instrument === "fd9" ? t("mire.fd9.carte") : t("mire.myiro1.carte");
    return `${t("mire.instrument." + instrument)} · ${detail}`;
  }

  // Raison sous « Préparer la mire » : une erreur, la mire FD-9 absente, ou rien.
  function raisonPreparation() {
    if (erreurPreparation) return erreurPreparation;
    if (saisieRefusee()) return t(saisieRefusee());
    if (erreurApercu) return erreurApercu;
    if (apercu && apercu.mire_fd9_attendue) {
      return `${t("mire.erreur.fd9_introuvable")} ${t("mire.fd9.attendu").replace("{chemin}", () => apercu.mire_fd9_attendue)}`;
    }
    return "";
  }

  function resultat() {
    if (!preparee) return [];
    const liste = el("ul", "prose");
    for (const f of preparee.fichiers) {
      const nom = String(f).split(/[\\/]/).pop();
      const ligne = el("li", "", nom);
      ligne.title = String(f);
      liste.append(ligne);
    }
    const dossier = el("button", "btn", t("mire.ouvrir_dossier"));
    dossier.addEventListener("click", async () => {
      try {
        await invoke("mire_ouvrir_dossier");
      } catch (erreur) {
        parc.annoncer(parc.texteErreur(erreur));
      }
    });
    const actions = el("div", "actions");
    actions.append(dossier);
    const notice = el("div", "notice");
    notice.append(el("p", "", t("mire.rappel")));
    return [champ("mire.fichiers", liste), actions, notice];
  }

  // Mires déjà préparées pour la résolution : date, instrument, plages, pages.
  function preparees(d) {
    const bloc = el("div", "field");
    bloc.append(el("span", "label", t("mire.preparees")));
    for (const tirage of d.tirages || []) {
      const ligne = el("div", "cle-valeur mute");
      ligne.append(el("span", "", date(tirage.horodatage)),
        el("span", "", [t("mire.instrument." + tirage.instrument),
          t("mire.plages").replace("{n}", () => nombreLocal(tirage.plages)), pages(tirage.pages)].join(" · ")));
      bloc.append(ligne);
    }
    if (!(d.tirages || []).length) bloc.append(el("p", "why", t("mire.preparees.vide")));
    return bloc;
  }

  function dessinerSection(d) {
    ouvrir.setAttribute("aria-expanded", String(ouvert));
    section.hidden = !ouvert;
    dessinerPrincipal();
    if (!ouvert) return section.replaceChildren();
    const corps = el("div", "section section--serree");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("mire.titre")));
    corps.append(tete, champ("mire.instrument",
      cartes("mire-instrument", ["fd9", "myiro1"], choix.instrument, libelleInstrument, (v) => { choix.instrument = v; })));
    if (choix.instrument === "myiro1") {
      corps.append(
        champ("mire.taille", cartes("mire-taille", TAILLES, lireNombre(choix.nombre) === null ? choix.taille : null,
          libelleTaille, (v) => { choix.taille = v; choix.nombre = ""; })),
        champ("mire.format", puces(FORMATS, choix.format, (f) => t("mire.format." + f), (f) => { choix.format = f; })),
        blocAvance(),
      );
    }
    const actions = el("div", "actions");
    const preparer = el("button", "btn btn--primary", t("mire.preparer"));
    const raison = el("p", "why");
    raison.setAttribute("role", "status");
    raison.textContent = raisonPreparation();
    raison.hidden = !raison.textContent;
    if (!apercu || erreurApercu || apercu.mire_fd9_attendue || saisieRefusee()) preparer.dataset.inactif = "";
    preparer.addEventListener("click", () => parc.agir(preparerMire));
    actions.append(preparer);
    corps.append(actions, raison, ...resultat(), preparees(d));
    section.replaceChildren(corps);
  }

  // Chaque changement de choix redemande l'aperçu ; rien n'est lancé.
  async function chargerApercu(d) {
    const n = ++demandeApercu;
    if (!ouvert) return;
    // Une saisie refusée ne demande rien au module `mires`.
    if (saisieRefusee()) return;
    let reponse = null;
    let erreur = "";
    const tailles = {};
    try {
      reponse = await invoke("mire_apercu", { demande: demande() });
      if (choix.instrument === "myiro1") {
        for (const taille of TAILLES) {
          tailles[taille] = await invoke("mire_apercu", { demande: { ...demande(taille), taille, nombre: null } });
        }
      }
    } catch (e) {
      erreur = parc.texteErreur(e);
    }
    if (n !== demandeApercu) return;
    apercu = reponse;
    apercusTailles = tailles;
    erreurApercu = erreur;
    dessinerSection(d);
    parc.occupation();
  }

  function changer(modification) {
    modification();
    erreurPreparation = "";
    const d = parc.reglage();
    if (!d) return;
    dessinerSection(d);
    parc.occupation();
    chargerApercu(d);
  }

  async function preparerMire(d) {
    const ici = () => parc.reglage()?.condition.id === d.condition.id;
    erreurPreparation = "";
    if (saisieRefusee()) return;
    try {
      // Le texte imprimé sur les pages suit la langue de la page (#79).
      const langue = document.documentElement.lang || "fr";
      const r = await invoke("mire_preparer", { condition: d.condition.id, demande: demande(), langue });
      if (ici()) preparee = r;
      await parc.recharger();
      // La même mire, pas encore mesurée, est reprise sans doublon : sa date est dite.
      if (ici()) {
        parc.annoncer(r.deja_preparee
          ? t("mire.deja_preparee").replace("{date}", () => date(r.deja_preparee))
          : t("mire.fait"));
      }
    } catch (erreur) {
      if (ici()) {
        erreurPreparation = parc.texteErreur(erreur);
        dessinerSection(parc.reglage());
      }
    }
  }

  ouvrir.addEventListener("click", () => changer(() => {
    ouvert = !ouvert;
    // La section du calcul se referme : un seul bouton principal.
    if (ouvert && calculOuvert()) ouvrirCalcul.click();
  }));
  // Le calcul s'ouvre : la section de la mire se referme.
  ouvrirCalcul.addEventListener("click", () => {
    if (calculOuvert() && ouvert) changer(() => { ouvert = false; });
  });

  // Une autre résolution : la section se referme ; ses choix sont gardés pour la séance.
  function dessiner(d) {
    if (condition !== d.condition.id) {
      condition = d.condition.id;
      ouvert = false;
      preparee = null;
      erreurPreparation = "";
      if (!choixParResolution.has(condition)) choixParResolution.set(condition, { ...CHOIX_DEPART });
      choix = choixParResolution.get(condition);
    }
    dessinerSection(d);
    chargerApercu(d);
  }

  return {
    brancher: (fonctions) => { parc = fonctions; },
    dessiner,
  };
})();
