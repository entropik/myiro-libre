// Résultat d'une vérification de profil (ticket #41), dessiné dans la section « Vérifier le profil »
// (verification.js) : le grand verdict (couleur, icône et mot ; le rouge seulement pour « hors
// tolérance »), la fidélité aux couleurs demandées, les couleurs hors gamut, l'accord avec la
// prédiction, les contrôles un par un et les pires plages des trois familles. Les valeurs
// demandées, calculées, prédites et mesurées ont chacune leur colonne. Textes par le catalogue.
"use strict";

window.ResultatVerification = (() => {
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const TRACES = { conforme: "M5 12l4 4 10-10", limite: "M12 4v10M12 18v2", hors: "M6 6l12 12M18 6L6 18" };
  const CLASSES = { accepte: "conforme", hors_tolerance: "hors", incomplet: "limite", non_verifie: "limite" };

  const nombre = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 2, maximumFractionDigits: 2,
  }).format(v);
  const inconnu = (v, f) => (v === null || v === undefined ? t("verif.inconnu") : f(v));
  const lab = (c) => inconnu(c, (x) => [x.l, x.a, x.b].map((v) => v.toFixed(1)).join(" "));
  const octets = (v) => v.map((x) => Math.round(x * 255)).join(" ");

  // « Atelier v2 » : nom et version d'un jeu de tolérances, par le catalogue.
  const nomVersion = (nom, version) => t("verif.tolerances.nom_version")
    .replace("{nom}", () => nom).replace("{version}", () => String(version));

  function motEtat(etat, tolerances) {
    return t("verif.etat." + etat).replace("{tolerances}", () => tolerances || "");
  }

  function carteFamille(cle, stats, inconnue) {
    const carte = el("div", "carte");
    carte.append(el("span", "label label--ink", t(cle)));
    if (!stats) {
      carte.append(el("p", "why", t(inconnue)));
      return carte;
    }
    const grille = el("div", "criteres");
    for (const [nom, valeur] of [["verif.critere.moyenne", nombre(stats.moyenne)], ["verif.critere.p95", nombre(stats.p95)],
      ["verif.critere.max", nombre(stats.max)], ["verif.plages", String(stats.plages)]]) {
      const c = el("div", "critere");
      c.append(el("span", "critere__nom", t(nom)), el("span", "critere__valeur", valeur));
      grille.append(c);
    }
    carte.append(grille);
    return carte;
  }

  // Les pires plages des trois familles (fidélité, hors gamut, prédiction), sans doublon.
  function tableauPires(r) {
    const pires = [r.fidelite, r.hors_gamut, r.prediction].flatMap((s) => s?.pires || []);
    const ids = [...new Set(pires.map((p) => p.id))];
    const table = el("table", "table--compact");
    const tete = el("tr");
    for (const cle of ["rapport.plage", "verif.demande", "verif.appareil", "verif.predite", "verif.mesuree",
      "rapport.ecart_fidelite", "rapport.ecart_prediction", "rapport.ecart_gamut"]) {
      tete.append(el("th", "", t(cle)));
    }
    table.append(tete);
    for (const id of ids) {
      const p = r.plages.find((x) => x.id === id);
      if (!p) continue;
      const ligne = el("tr");
      for (const v of [p.id, lab(p.demande), octets(p.appareil), lab(p.prediction), lab(p.mesure),
        inconnu(p.ecart_fidelite, nombre), nombre(p.ecart_prediction), inconnu(p.ecart_gamut, nombre)]) {
        ligne.append(el("td", "", v));
      }
      table.append(ligne);
    }
    return table;
  }

  function controles(liste) {
    const carte = el("div", "carte");
    carte.append(el("span", "label label--ink", t("rapport.controles")));
    for (const c of liste) {
      const l = el("div", "cle-valeur");
      const respect = c.respecte === null ? t("verif.inconnu") : t(c.respecte ? "verif.respecte" : "verif.depasse");
      l.append(el("span", "", `${t("verif." + c.famille)} · ${t("verif.critere." + c.critere)} ≤ ${nombre(c.limite)}`),
        el("b", "", `${inconnu(c.valeur, nombre)} · ${respect}`));
      carte.append(l);
    }
    return carte;
  }

  // Le résultat complet ; `jeu` : « Atelier v2 », le jeu avec lequel il a été calculé.
  function dessiner({ etat, calcul }, jeu) {
    const classe = CLASSES[etat] || "limite";
    const verdict = el("div", "carte carte--verdict verdict--" + classe);
    verdict.innerHTML = `<svg class="verdict__icone" viewBox="0 0 24 24" aria-hidden="true"><path d="${TRACES[classe]}"/></svg>`;
    const texte = el("div", "verdict__texte");
    texte.append(el("span", "verdict__contexte", t("verif.titre")), el("div", "verdict__mot", motEtat(etat, jeu)));
    verdict.append(texte);
    const sansDemande = calcul.evaluation.demande === null;
    const r = calcul.resultats;
    const corps = el("div");
    corps.append(verdict, el("p", "why", t("verif.regle")));
    if (calcul.memes_couleurs_que_le_profil) corps.append(el("p", "why", t("verif.memes_couleurs")));
    if (calcul.rvb_supposes) corps.append(el("p", "why", t("verif.rvb_supposes")));
    corps.append(
      carteFamille("verif.fidelite", r.fidelite, sansDemande ? "verif.inconnu.sans_demande" : "verif.inconnu.aucune_plage_dans_le_gamut"),
      carteFamille("verif.hors_gamut", r.hors_gamut, sansDemande ? "verif.inconnu.sans_demande" : "verif.inconnu.aucune_plage"),
      carteFamille("verif.prediction", r.prediction, "verif.inconnu"),
      controles(calcul.controles),
    );
    const pires = el("div", "field");
    pires.append(el("span", "label", t("verif.pires")), tableauPires(r));
    corps.append(pires, el("p", "why", t("verif.appareil.calculee")));
    return corps;
  }

  return { dessiner, motEtat, nomVersion };
})();
