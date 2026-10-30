// Vue d'une résolution d'impression, choisie dans l'arbre (parc.js) : au centre, le profil actif
// (verdict couleur + icône + mot), le seul bouton principal « Installer dans Windows », exporter,
// ranger un profil existant, calculer un profil (calcul-profil.js), l'historique et les mesures ;
// à droite, la provenance du profil. Les actions passent par le module Rust `parc` ; les textes,
// par le catalogue (app.js).
"use strict";

window.Reglage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const vue = document.querySelector("[data-vue-reglage]");
  const details = document.querySelector("[data-details-reglage]");
  const annonce = vue.querySelector("[data-reglage-message]");
  // Ce que parc.js prête à la vue : la résolution affichée et de quoi recharger.
  let parc = { reglage: () => null, rafraichir: async () => {}, recharger: async () => {} };

  const { el, date, reperesProfil } = window.Arbre;

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

  // Calcul d'un profil : section, calcul et provenance (calcul-profil.js).
  const calcul = window.CalculProfil;
  const { texteErreur } = calcul;
  // Préparer une mire : section et préparation (mire.js).
  const mire = window.PreparerMire;
  // Mesurer une feuille au FD-9 : bouton, état de la feuille, mire et points mesurés (feuille-fd9.js).
  const feuilleFd9 = window.FeuilleFd9;

  // Comparaison de profils (comparer.js) : elle prend la place de cette vue.
  const comparer = window.Comparer;
  const source = (profil) => ({ type: "bibliotheque", profil });

  // Pendant une action (fenêtre de Windows ouverte, installation, calcul), tous les boutons de
  // la vue sont inactifs et la raison « en cours » est dite juste à côté : un double clic ne
  // fait rien. Pendant un calcul, seule sa raison à lui est dite, sous son bouton. Un bouton
  // marqué data-inactif l'est aussi hors de toute action. « Comparer » et « Comparer à
  // l'actuel » en font partie, comme la vérification.
  const BOUTONS = "[data-installer], [data-exporter-profil], [data-ranger-profil], [data-calcul-ouvrir], [data-comparer-ouvrir], [data-comparer-actuel], [data-calcul-profil] button, [data-calcul-profil] input, [data-verif-ouvrir], [data-verification] button, [data-verification] input";
  const BOUTONS_MIRE = "[data-mire-ouvrir], [data-mire] button, [data-mire] input";
  // Vérification d'un profil : section, résultat et provenance (verification.js).
  const verification = window.Verification;
  const occupe = vue.querySelector("[data-reglage-occupe]");
  let enCours = false;

  function dessinerOccupation(sansProfil) {
    for (const b of vue.querySelectorAll(`${BOUTONS}, ${BOUTONS_MIRE}`)) {
      b.disabled = enCours || b.hasAttribute("data-inactif") || (sansProfil && b.hasAttribute("data-installer"));
    }
    occupe.hidden = !enCours || calcul.raisonVisible();
    calcul.dessinerRaison();
  }

  function remplirVue(d, lignesMesures) {
    const titre = vue.querySelector("[data-reglage-titre]");
    titre.textContent = `${d.papier} · ${d.resolution}`;
    titre.title = `${d.imprimante} · ${titre.textContent}`;
    const actif = d.actif;
    // Un état que la page ne connaît pas encore est montré comme « à vérifier », sans rien casser.
    // Le mot vient du module `parc` : un profil à vérifier dit s'il est calculé ou importé.
    const [classe, repli] = actif ? VERDICTS[actif.etat] || VERDICTS.non_verifie : ["aucun", "parc.verdict.aucun"];
    const mot = textes[d.verdict] ? d.verdict : repli;
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
    vue.querySelector("[data-comparer-ouvrir]").hidden = !actif;
    calcul.dessiner(d);
    verification.dessiner(d);
    mire.dessiner(d);
    feuilleFd9.dessiner(d);
    dessinerOccupation(!actif);

    const historique = vue.querySelector("[data-historique]");
    historique.replaceChildren(...d.historique.map((p) => {
      const ligne = el("div", "cle-valeur mute");
      // Même repères que la liste « Profil à vérifier » : date et heure, recette, puis actuel ou
      // remplacé ; l'état reste à droite.
      const quand = el("span", "", [...reperesProfil(p), t(p.actif ? "parc.historique.actuel" : "parc.historique.remplace")].join(" · "));
      quand.title = p.nom;
      const etat = textes[p.libelle] ? p.libelle : "parc.etat.non_verifie";
      const droite = el("span", "", t(etat));
      // Un profil remplacé se compare à l'actuel : avant / après, l'actuel en référence.
      if (!p.actif && actif) {
        const b = el("button", "btn", t("comparer.ouvrir.actuel"));
        b.dataset.comparerActuel = "";
        b.disabled = enCours;
        b.addEventListener("click", () => {
          comparer.ouvrir([source(d.actif.id), source(p.id)]);
        });
        droite.append(" ", b);
      }
      ligne.append(quand, droite);
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

  // Instrument d'un profil calculé : le modèle que déclare le fichier de la mesure source,
  // déjà réduit par Rust (« FD-9 »), jamais son numéro de série (ticket #84).
  function instrumentDeclare(d) {
    const source = d.provenance && d.importees.find((m) => m.id === d.provenance.mesure);
    const info = source ? source.instrument : null;
    if (!info || info.statut === "inconnue") return t("parc.inconnu");
    return info.statut === "supposee" ? `${info.valeur} (${t("details.a_confirmer")})` : info.valeur;
  }

  // Provenance du profil à droite : ce qui n'est pas connu est écrit « inconnu » en toutes lettres.
  function remplirDetails(d) {
    const a = d.actif;
    const provenance = a
      ? [
        cleValeur("parc.origine", t("parc.origine." + d.origine)),
        cleValeur("parc.nom", a.nom),
        cleValeur("parc.empreinte", a.empreinte.slice(0, 12)),
        cleValeur("parc.instrument", instrumentDeclare(d)),
        cleValeur("parc.mire", t("parc.inconnue")),
        ...calcul.provenance(d, cleValeur),
      ]
      : [el("p", "", t("parc.verdict.aucun"))];
    details.replaceChildren(
      section("parc.provenance", provenance),
      section("verif.titre", a ? verification.provenance(d, cleValeur) : [el("p", "", t("parc.verdict.aucun"))]),
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

  vue.querySelector("[data-comparer-ouvrir]").addEventListener("click", () => {
    const d = parc.reglage();
    if (d && d.actif) comparer.ouvrir([source(d.actif.id)]);
  });

  // Le calcul agit sur la résolution affichée, avec le même mécanisme que les autres actions.
  calcul.brancher({
    reglage: () => parc.reglage(),
    recharger: () => parc.recharger(),
    annoncer,
    agir,
    occupation: () => dessinerOccupation(!(parc.reglage() || {}).actif),
  });
  verification.brancher({
    reglage: () => parc.reglage(),
    recharger: () => parc.recharger(),
    annoncer,
    agir,
    occupation: () => dessinerOccupation(!(parc.reglage() || {}).actif),
    texteErreur,
  });

  // La préparation d'une mire agit sur la résolution affichée, avec le même mécanisme.
  mire.brancher({
    reglage: () => parc.reglage(),
    recharger: () => parc.recharger(),
    annoncer,
    agir,
    texteErreur,
    occupation: () => dessinerOccupation(!(parc.reglage() || {}).actif),
  });

  // La feuille agit sur la résolution affichée ; elle ne bloque pas les autres boutons de la vue.
  feuilleFd9.brancher({
    reglage: () => parc.reglage(),
    recharger: () => parc.recharger(),
  });

  return {
    brancher: (fonctions) => { parc = fonctions; },
    remplir: (d, lignesMesures) => { remplirVue(d, lignesMesures); remplirDetails(d); },
    annoncer,
  };
})();
