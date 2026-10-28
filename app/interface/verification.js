// Vérifier un profil sur une seconde mire (ticket #41), depuis la vue d'une résolution : la
// section ouverte par « Vérifier le profil » (profil, mesure de la seconde mire, condition de
// mesure, jeu de tolérances, fiche d'impression à confirmer, commandes xicclu), le résultat (grand
// verdict, fidélité, hors gamut, prédiction, contrôles, pires plages) et, à droite, la dernière
// vérification et l'historique. Utilisé par reglage.js, qui prête la résolution affichée et
// agir() ; l'éditeur des tolérances est dans tolerances.js. Les calculs passent par le module Rust
// `verifier` ; les textes, par le catalogue (`textes`, chargé par app.js).
"use strict";

window.Verification = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el, date } = window.Arbre;

  const vue = document.querySelector("[data-vue-reglage]");
  const installer = vue.querySelector("[data-installer]");
  const ouvrir = vue.querySelector("[data-verif-ouvrir]");
  const raisonOuvrir = vue.querySelector("[data-verif-raison]");
  const section = vue.querySelector("[data-verification]");
  let parc = { reglage: () => null, recharger: async () => {}, annoncer: () => {}, agir: async () => {}, occupation: () => {}, texteErreur: String };

  const FICHE_VIDE = { impression: "a_travers_le_profil", logiciel: "", support_pilote: "", qualite: "", encres: "", remarques: "", confirmee: false };
  let ouvert = false;
  let choix = { condition: null, profil: null, mesure: null, mesureCondition: null, tolerances: null, fiche: { ...FICHE_VIDE } };
  let donnees = { sources: [], jeux: [], commandes: "" };
  let resultat = null; // dernier ResultatVerification de cette résolution
  let resultatJeu = ""; // nom et version du jeu de ce résultat : « Atelier v2 »
  let editeur = false;
  let erreur = "";

  const { motEtat, nomVersion } = window.ResultatVerification;

  function nomJeu(id) {
    const jeu = donnees.jeux.find((j) => j.id === id);
    return jeu ? nomVersion(jeu.nom, jeu.version) : "";
  }

  // Profils RVB de la résolution : l'actif d'abord, puis l'historique.
  const profilsRvb = (d) => d.historique.filter((p) => p.espace === "rvb")
    .sort((a, b) => Number(b.actif) - Number(a.actif));

  function champ(cle, ...contenu) {
    const f = el("div", "field");
    f.append(el("span", "label", t(cle)), ...contenu);
    return f;
  }

  function changer(modification) {
    modification();
    erreur = "";
    const d = parc.reglage();
    if (d) dessinerSection(d);
  }

  // Une liste de choix ; une ligne inactive reste visible, avec sa raison dessous.
  function liste(nom, lignes, choisi, choisir) {
    const l = el("div", "choix-liste");
    for (const { valeur, libelle, raison } of lignes) {
      const ligne = el("div");
      const rang = el("label", "choix-liste__rang");
      const radio = el("input");
      radio.type = "radio";
      radio.name = nom;
      radio.checked = valeur === choisi;
      if (raison) radio.dataset.inactif = "";
      radio.addEventListener("change", () => { if (!raison) changer(() => choisir(valeur)); });
      const texte = el("span", "", libelle);
      texte.title = libelle;
      rang.append(radio, texte);
      ligne.append(rang);
      if (raison) ligne.append(el("p", "why", t(raison)));
      l.append(ligne);
    }
    return l;
  }

  function saisie(cle) {
    const champTexte = el("input", "input");
    champTexte.value = choix.fiche[cle];
    // Changer un réglage oblige à confirmer de nouveau.
    champTexte.addEventListener("change", () => changer(() => { choix.fiche[cle] = champTexte.value; choix.fiche.confirmee = false; }));
    return champ("verif.fiche." + cle, champTexte);
  }

  function blocFiche() {
    const corps = el("div");
    corps.append(champ("verif.fiche.impression", liste("verif-impression",
      ["a_travers_le_profil", "sans_gestion"].map((i) => ({ valeur: i, libelle: t("verif.fiche.impression." + i) })),
      choix.fiche.impression, (i) => { choix.fiche.impression = i; choix.fiche.confirmee = false; chargerCommandes(); })));
    for (const cle of ["logiciel", "support_pilote", "qualite", "encres", "remarques"]) corps.append(saisie(cle));
    const confirmer = el("label", "choix-liste__rang");
    const caseConfirmer = el("input");
    caseConfirmer.type = "checkbox";
    caseConfirmer.checked = choix.fiche.confirmee;
    caseConfirmer.addEventListener("change", () => changer(() => { choix.fiche.confirmee = caseConfirmer.checked; }));
    confirmer.append(caseConfirmer, el("span", "", t("verif.fiche.confirmer")));
    corps.append(confirmer);
    return champ("verif.fiche", corps);
  }

  function blocTolerances() {
    const jeux = window.Tolerances.dernieres(donnees.jeux);
    const contenu = el("div");
    if (jeux.length) {
      contenu.append(liste("verif-tolerances", jeux.map((j) => ({ valeur: j.id, libelle: nomVersion(j.nom, j.version) })),
        choix.tolerances, (id) => { choix.tolerances = id; }));
    } else {
      contenu.append(el("p", "why", t("verif.tolerances.aucun")));
    }
    const modifier = el("button", "btn", t("verif.tolerances"));
    modifier.type = "button";
    modifier.setAttribute("aria-expanded", String(editeur));
    modifier.addEventListener("click", () => changer(() => { editeur = !editeur; }));
    const actions = el("div", "actions actions--serrees");
    actions.append(modifier);
    contenu.append(actions);
    if (editeur) {
      const cible = el("div");
      const depart = donnees.jeux.find((j) => j.id === choix.tolerances) || jeux[0] || null;
      window.Tolerances.ouvrir(cible, depart, {
        texteErreur: parc.texteErreur,
        enregistre: async (jeu, message) => {
          editeur = false;
          choix.tolerances = jeu.id;
          await charger(parc.reglage());
          parc.annoncer(message);
        },
      });
      contenu.append(cible);
    }
    return champ("verif.tolerances", contenu);
  }

  // Pourquoi « Vérifier » ne peut pas encore partir ; rien s'il peut.
  function raisonLancer() {
    if (choix.mesure === null) return "verif.raison.mesure";
    if (choix.tolerances === null) return "verif.tolerances.aucun";
    if (!choix.fiche.confirmee) return "verif.refus.fiche";
    return null;
  }

  function blocResultat() {
    const { verification } = resultat;
    const corps = window.ResultatVerification.dessiner(resultat, resultatJeu);
    const actions = el("div", "actions");
    const recalculer = el("button", "btn", t("verif.recalculer"));
    recalculer.addEventListener("click", () => parc.agir(async () => {
      resultat = await invoke("verifier_recalculer", { verification, tolerances: choix.tolerances });
      resultatJeu = nomJeu(choix.tolerances);
      await parc.recharger();
    }));
    const rapport = el("button", "btn", t("verif.exporter_rapport"));
    rapport.addEventListener("click", () => parc.agir(async () => exporter("verifier_exporter_rapport", verification, "verif.filtre.html")));
    const profil = el("button", "btn", t("verif.exporter_profil"));
    profil.addEventListener("click", () => parc.agir(async () => exporter("verifier_exporter_profil", verification, "parc.filtre.icc")));
    actions.append(recalculer, rapport, profil);
    corps.append(actions);
    return corps;
  }

  async function exporter(commande, verification, filtre) {
    const nom = await invoke(commande, { verification, langue: document.documentElement.lang, filtre: t(filtre) });
    if (nom) parc.annoncer(`${t("export.fait")} ${nom}`);
  }

  function dessinerSection(d) {
    const disponible = profilsRvb(d).length > 0;
    if (disponible) delete ouvrir.dataset.inactif;
    else ouvrir.dataset.inactif = "";
    raisonOuvrir.hidden = disponible;
    ouvrir.setAttribute("aria-expanded", String(ouvert && disponible));
    section.hidden = !(ouvert && disponible);
    // Une section ouverte porte le seul bouton principal de la vue.
    installer.classList.toggle("btn--primary", section.hidden && !window.CalculProfil.ouvert());
    if (section.hidden) return section.replaceChildren();
    const corps = el("div", "section section--serree");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t("verif.titre")));
    corps.append(tete, champ("verif.profil", liste("verif-profil", profilsRvb(d).map((p) => ({
      valeur: p.id, libelle: `${date(p.horodatage)} · ${t(p.libelle)}`,
    })), choix.profil, (id) => { choix.profil = id; choix.mesure = null; resultat = null; charger(parc.reglage()); })));
    // La mire de vérification FD-9, créée une fois, s'ouvre dans FD-S2w pour être imprimée.
    const mire = el("button", "btn", t("verif.mire_fd9"));
    mire.type = "button";
    mire.addEventListener("click", () => parc.agir(async () => {
      const nom = await invoke("verifier_mire_fd9", { filtre: t("verif.filtre.xml") });
      if (nom) parc.annoncer(`${t("verif.mire_fd9.faite")} ${nom}`);
    }));
    const actionsMire = el("div", "actions actions--serrees");
    actionsMire.append(mire);
    corps.append(champ("verif.mesure", liste("verif-mesure", donnees.sources.map((s) => ({
      valeur: s.mesure, libelle: s.fichier, raison: s.raison,
    })), choix.mesure, (m) => { choix.mesure = m; }), actionsMire, el("p", "", t("verif.mire_fd9.aide"))));
    const source = donnees.sources.find((s) => s.mesure === choix.mesure);
    if (source) {
      const puces = el("div", "puces");
      for (const c of source.conditions) {
        const b = el("button", "", c);
        b.type = "button";
        b.setAttribute("aria-pressed", String(c === choix.mesureCondition));
        b.addEventListener("click", () => changer(() => { choix.mesureCondition = c; }));
        puces.append(b);
      }
      corps.append(champ("verif.condition", puces));
    }
    corps.append(blocTolerances(), blocFiche());
    const avance = el("details", "replie");
    avance.append(el("summary", "", t("verif.avance")), el("pre", "", donnees.commandes));
    corps.append(avance);
    const actions = el("div", "actions");
    const lancer = el("button", "btn btn--primary", t("verif.lancer"));
    const raison = raisonLancer();
    if (raison) lancer.dataset.inactif = "";
    lancer.addEventListener("click", () => { if (!raisonLancer()) parc.agir(verifier); });
    actions.append(lancer);
    const pourquoi = el("p", "why", erreur || (raison ? t(raison) : ""));
    pourquoi.setAttribute("role", "status");
    pourquoi.hidden = !pourquoi.textContent;
    corps.append(actions, pourquoi);
    if (resultat) corps.append(blocResultat());
    section.replaceChildren(corps);
    parc.occupation();
  }

  async function verifier(d) {
    const condition = d.condition.id;
    try {
      resultatJeu = nomJeu(choix.tolerances);
      resultat = await invoke("verifier_lancer", { demande: {
        profil: choix.profil, mesure: choix.mesure, mesure_condition: choix.mesureCondition,
        tolerances: choix.tolerances, fiche: choix.fiche,
      } });
    } catch (e) {
      if (parc.reglage()?.condition.id === condition) erreur = parc.texteErreur(e);
      return dessinerSection(d);
    }
    await parc.recharger();
  }

  async function chargerCommandes() {
    try {
      const r = await invoke("verifier_commandes", { impression: choix.fiche.impression });
      donnees.commandes = r.commandes.join("\n");
      erreur = r.moteur ? parc.texteErreur(r.moteur) : erreur;
    } catch (e) {
      erreur = parc.texteErreur(e);
    }
  }

  // Sources du profil choisi, jeux de tolérances, fiche reprise (à confirmer) et commandes.
  async function charger(d) {
    if (!d || choix.profil === null) return;
    try {
      const [sources, jeux] = await Promise.all([
        invoke("verifier_sources", { profil: choix.profil }), invoke("tolerances_lister"),
      ]);
      donnees.sources = sources;
      donnees.jeux = jeux;
      const utilisables = sources.filter((s) => !s.raison);
      if (!utilisables.some((s) => s.mesure === choix.mesure)) choix.mesure = utilisables[0]?.mesure ?? null;
      const conditions = utilisables.find((s) => s.mesure === choix.mesure)?.conditions || [];
      if (!conditions.includes(choix.mesureCondition)) choix.mesureCondition = conditions.includes("M1") ? "M1" : conditions[0] || null;
      const dernieres = window.Tolerances.dernieres(jeux);
      if (!dernieres.some((j) => j.id === choix.tolerances)) choix.tolerances = dernieres[0]?.id ?? null;
      await chargerCommandes();
    } catch (e) {
      erreur = parc.texteErreur(e);
    }
    if (parc.reglage()?.condition.id === d.condition.id) dessinerSection(parc.reglage());
  }

  ouvrir.addEventListener("click", async () => {
    const d = parc.reglage();
    if (!d || ouvrir.dataset.inactif !== undefined) return;
    ouvert = !ouvert;
    if (ouvert && choix.profil === null) choix.profil = profilsRvb(d)[0]?.id ?? null;
    if (ouvert) {
      const fiche = await invoke("verifier_fiche", { condition: d.condition.id }).catch(() => null);
      if (fiche) choix.fiche = fiche;
      await charger(d);
    }
    dessinerSection(d);
  });

  // Une autre résolution : la section se referme et ses choix repartent de zéro.
  function dessiner(d) {
    if (choix.condition !== d.condition.id) {
      ouvert = false;
      editeur = false;
      resultat = null;
      erreur = "";
      choix = { condition: d.condition.id, profil: null, mesure: null, mesureCondition: null, tolerances: null, fiche: { ...FICHE_VIDE } };
    }
    dessinerSection(d);
  }

  // À droite : la dernière vérification du profil actif, puis l'historique de ses vérifications.
  function provenance(d, cleValeur) {
    const v = d.verification;
    if (!v) return [cleValeur("verif.derniere", t("verif.etat.non_verifie"))];
    const lignes = [cleValeur("verif.derniere", `${date(v.horodatage)} · ${motEtat(v.etat, nomVersion(v.tolerances_nom, v.tolerances_version))}`)];
    const historique = el("div");
    invoke("verifier_historique", { profil: d.actif.id }).then((liste) => {
      historique.replaceChildren(el("span", "label", t("verif.historique")), ...liste.map((h) => {
        const ligne = el("div", "cle-valeur mute");
        const jeu = nomVersion(h.tolerances_nom, h.tolerances_version);
        ligne.append(el("span", "", date(h.horodatage)), el("span", "", `${jeu} · ${motEtat(h.etat, jeu)}`));
        return ligne;
      }));
    }).catch((e) => historique.replaceChildren(el("p", "why", parc.texteErreur(e))));
    lignes.push(historique);
    return lignes;
  }

  return {
    brancher: (fonctions) => { parc = fonctions; },
    dessiner,
    provenance,
    ouverte: () => ouvert,
  };
})();
