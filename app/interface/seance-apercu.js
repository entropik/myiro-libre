// Ce que donne la moyenne d'une séance de mesure (ticket #35), dans la section de seance.js : la
// mire du plan, les plages moyennées et manquantes, les conditions laissées de côté, les écarts
// entre feuilles (chiffres clés, puis la liste des écarts signalés : plage, feuille, ΔE00), et un
// refus de moyennage avec la feuille en cause. Les valeurs viennent du module Rust `seance_fd9`
// (`seance_etat`) ; rien n'est recalculé ici. Tout s'écrit sur place dans les blocs de taille
// gardée de index.html ; une ligne d'écart vient d'un modèle et reste ensuite.
"use strict";

window.SeanceApercu = (() => {
  const t = (cle) => textes[cle] || cle;
  const racine = document.querySelector("[data-seance]");
  const q = (selecteur) => racine.querySelector(selecteur);
  const mire = q("[data-seance-mire]");
  const valeurs = Object.fromEntries(
    ["plages", "manquantes", "conditions", "ecarts"].map((nom) => [nom, q(`[data-seance-apercu="${nom}"]`)]),
  );
  const ligneRefus = q("[data-seance-refus]");
  const liste = q("[data-seance-ecarts]");
  const vide = q("[data-seance-ecarts-vide]");
  const modele = q("[data-seance-ecart-modele]");
  const lignes = []; // lignes d'écarts, créées au besoin puis gardées
  const LIGNES_MAX = 200; // au-delà, les écarts sont comptés

  const decimal = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v);
  const entier = (v) => new Intl.NumberFormat(document.documentElement.lang).format(v);

  function ecrireValeur(nom, texte) {
    valeurs[nom].textContent = texte;
    valeurs[nom].title = texte;
  }

  const fichierDe = (etat, mesure) => {
    const s = etat.sources.find((x) => x.mesure === mesure);
    return s ? s.fichier : String(mesure);
  };

  // Le texte d'un refus, puis la feuille ou la plage en cause quand le détail la donne.
  function texteRefus(etat, refus, texteErreur) {
    const morceaux = [texteErreur(refus)];
    if (refus.incompatible && typeof refus.incompatible.source === "number") {
      morceaux.push(t("seance.refus.feuille").replace("{fichier}", () => fichierDe(etat, refus.incompatible.source)));
    } else if (refus.incompatible && refus.incompatible.plage) {
      morceaux.push(t("seance.refus.plage").replace("{plage}", () => refus.incompatible.plage));
    }
    return morceaux.join(" ");
  }

  function ligneEcart(i) {
    if (!lignes[i]) {
      lignes[i] = modele.content.firstElementChild.cloneNode(true);
      liste.append(lignes[i]);
    }
    return lignes[i];
  }

  // Les écarts signalés, du plus grand au plus petit.
  function ecrireListe(etat, ecarts) {
    const signales = ecarts.filter((e) => e.signale).sort((a, b) => (b.de00 || 0) - (a.de00 || 0));
    const montres = signales.slice(0, LIGNES_MAX);
    montres.forEach((e, i) => {
      const ligne = ligneEcart(i);
      const texte = t("seance.ecarts.ligne").replace("{plage}", () => e.plage)
        .replace("{fichier}", () => fichierDe(etat, e.source));
      const libelle = ligne.querySelector("[data-texte]");
      libelle.textContent = texte;
      libelle.title = texte;
      ligne.querySelector("[data-valeur]").textContent = t("seance.ecarts.valeur")
        .replace("{de00}", () => decimal(e.de00 || 0));
      ligne.hidden = false;
    });
    lignes.slice(montres.length).forEach((l) => { l.hidden = true; });
    vide.textContent = !signales.length ? t("seance.ecarts.vide")
      : signales.length > montres.length
        ? t("seance.ecarts.autres").replace("{n}", () => entier(signales.length - montres.length)) : "";
  }

  // `etat` : la réponse de `seance_etat`, ou rien sans séance.
  function ecrire(etat, texteErreur) {
    mire.textContent = etat ? t("seance.mire").replace("{nom}", () => etat.plan.mire.nom)
      .replace("{empreinte}", () => etat.plan.mire.empreinte.slice(0, 12)) : "";
    mire.title = etat ? etat.plan.mire.empreinte : "";
    const a = etat && (etat.apercu || etat.composition);
    if (!a) {
      for (const nom of Object.keys(valeurs)) ecrireValeur(nom, "");
      ligneRefus.textContent = etat && etat.refus ? texteRefus(etat, etat.refus, texteErreur) : t("seance.apercu.vide");
      ecrireListe(etat, []);
      vide.textContent = "";
      return;
    }
    const moyennees = etat.apercu ? a.plages : a.composition.length;
    ecrireValeur("plages", t("seance.apercu.plages.valeur").replace("{n}", () => entier(moyennees))
      .replace("{total}", () => entier(etat.plan.plages.length)));
    ecrireValeur("manquantes", a.manquantes.length
      ? t("seance.apercu.manquantes.valeur").replace("{n}", () => entier(a.manquantes.length))
        .replace("{liste}", () => a.manquantes.slice(0, 8).join(", ") + (a.manquantes.length > 8 ? "…" : ""))
      : t("seance.apercu.aucune"));
    const conditions = [...new Set(a.conditions_ecartees.map((c) => c.emplacement))]
      .map((e) => t(`seance.emplacement.${e}`));
    ecrireValeur("conditions", conditions.length ? conditions.join(", ") : t("seance.apercu.aucune"));
    const signales = a.ecarts.filter((e) => e.signale).length;
    const max = Math.max(0, ...a.ecarts.map((e) => e.de00 || 0));
    ecrireValeur("ecarts", t("seance.apercu.ecarts.valeur").replace("{n}", () => entier(signales))
      .replace("{max}", () => decimal(max)));
    ligneRefus.textContent = "";
    ecrireListe(etat, a.ecarts);
  }

  return { ecrire };
})();
