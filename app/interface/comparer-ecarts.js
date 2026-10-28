// Tableaux de la vue « Comparer » (ticket #71) : couleurs de base de chaque profil, et écarts de
// chaque profil au profil de référence (le premier). Les chiffres viennent de la crate `gamut`, par
// le module Rust `comparer` ; ils passent tous par Graphiques.nombre : un chiffre `null` (NaN ou
// infini côté Rust) s'écrit « inconnu », jamais « 0,00 ». Les textes viennent du catalogue.
"use strict";

window.ComparerEcarts = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;

  function lab(v) {
    const [l, a, b] = v || [];
    return t("comparer.valeur.lab")
      .replace("{l}", () => nombre(l, 2))
      .replace("{a}", () => nombre(a, 2))
      .replace("{b}", () => nombre(b, 2));
  }

  // Carré de la couleur de série d'un profil (jeton `--serie-n`).
  function carreSerie(serie) {
    const c = el("span", "swatch");
    c.style.setProperty("--c", `var(--serie-${serie})`);
    c.setAttribute("aria-hidden", "true");
    return c;
  }

  // Tableau : en-têtes, puis lignes ; une cellule est un texte ou un élément. Les colonnes après
  // la première sont des chiffres, alignés à droite.
  function tableau(entetes, lignes, chiffres = true) {
    const table = el("table", "table--compact");
    const tete = table.createTHead().insertRow();
    entetes.forEach((entete, i) => {
      const th = el("th", i > 0 && chiffres ? "r lc" : "");
      th.append(entete instanceof Node ? entete : String(entete));
      tete.append(th);
    });
    const corps = table.createTBody();
    for (const ligne of lignes) {
      const tr = corps.insertRow();
      ligne.forEach((cellule, i) => {
        const td = tr.insertCell();
        td.className = i > 0 && chiffres ? "r num" : "num";
        td.append(cellule instanceof Node ? cellule : String(cellule));
      });
    }
    return table;
  }

  function titre(cle) {
    return el("span", "label label--ink", t(cle));
  }

  function sousTitre(texte) {
    const p = el("p", "label", texte);
    return p;
  }

  // Couleurs de base : une ligne par couleur et par profil analysé. Un gris n'a pas de teinte
  // (« — ») ; une pastille ramenée dans l'écran est marquée « approchée ».
  function primaires(analyses) {
    const lignes = [];
    const premiers = (analyses[0] || { analyse: { primaires: [] } }).analyse.primaires;
    premiers.forEach((base, rang) => {
      for (const { analyse, nom, serie } of analyses) {
        const p = analyse.primaires[rang];
        if (!p) continue;
        const pastille = el("span", "");
        const carre = el("span", "swatch");
        carre.style.setProperty("--c", p.ecran);
        carre.title = p.ecran;
        pastille.append(carre);
        if (p.ramenee) pastille.append(" ", t("comparer.primaires.approchee"));
        const profil = el("span", "");
        profil.append(carreSerie(serie), " ", nom);
        lignes.push([
          t("comparer.primaire." + base.nom),
          profil,
          nombre(p.lab[0], 2),
          nombre(p.lab[1], 2),
          nombre(p.lab[2], 2),
          nombre(p.c, 2),
          p.h === null || p.h === undefined ? t("comparer.primaires.sans_teinte") : nombre(p.h, 1),
          pastille,
        ]);
      }
    });
    return tableau(
      [t("comparer.primaires.couleur"), t("comparer.primaires.profil"), "L*", "a*", "b*", "C*", "h (°)", t("comparer.primaires.pastille")],
      lignes,
    );
  }

  function part(v) {
    return t("comparer.ecarts.part").replace("{part}", () => nombre(v, 1));
  }

  // Écarts d'un profil au profil de référence : grille, zones, dix plus forts, couleurs de base.
  function blocEcarts(e, reference) {
    const bloc = el("div", "");
    const g = e.grille;
    bloc.append(
      sousTitre(t("comparer.ecarts.grille")),
      tableau(
        ["", t("comparer.ecarts.moyenne"), t("comparer.ecarts.mediane"), t("comparer.ecarts.p95"), t("comparer.ecarts.max"),
          t("comparer.ecarts.sup_1"), t("comparer.ecarts.sup_2")],
        [[t("comparer.ecarts.ecart"), nombre(g.moyenne, 2), nombre(g.mediane, 2), nombre(g.p95, 2), nombre(g.max, 2),
          part(g.part_sup_1), part(g.part_sup_2)]],
      ),
    );
    if (e.zones.length) {
      bloc.append(
        sousTitre(t("comparer.ecarts.zones")),
        tableau(
          [t("comparer.ecarts.zone"), t("comparer.ecarts.nombre"), t("comparer.ecarts.moyenne"), t("comparer.ecarts.p95"), t("comparer.ecarts.max")],
          e.zones.map((z) => [t(cleZone(z.zone)), nombre(z.nombre, 0), nombre(z.moyenne, 2), nombre(z.p95, 2), nombre(z.max, 2)]),
        ),
      );
    }
    if (e.pires.length && !e.identiques) {
      bloc.append(
        sousTitre(t("comparer.ecarts.pires")),
        tableau(
          [t("comparer.ecarts.rvb"), t("comparer.ecarts.lab_reference"), t("comparer.ecarts.lab_compare"), t("comparer.ecarts.ecart")],
          e.pires.map((p) => [p.rvb.join(" · "), lab(p.reference), lab(p.compare), nombre(p.ecart, 2)]),
        ),
      );
    }
    const noms = reference.analyse.primaires.map((p) => p.nom);
    bloc.append(
      sousTitre(t("comparer.ecarts.primaires")),
      tableau(
        [t("comparer.primaires.couleur"), t("comparer.ecarts.ecart")],
        e.primaires.map((v, i) => [t("comparer.primaire." + (noms[i] || "")), nombre(v, 2)]),
      ),
    );
    return bloc;
  }

  // Zone sérialisée par serde : "Gris", ou { "Saturee": "Rouge" } ; clé `comparer.zone.…`.
  function cleZone(zone) {
    if (typeof zone === "string") return "comparer.zone." + zone.toLowerCase();
    const [genre, famille] = Object.entries(zone || {})[0] || ["", ""];
    return `comparer.zone.${genre.toLowerCase()}.${String(famille).toLowerCase()}`;
  }

  // Fidélités des deux profils côte à côte, quand ils embarquent les mêmes mesures.
  function fidelites(paires, texteFidelite) {
    const liste = el("div", "");
    for (const { nom, serie, fidelite } of paires) {
      const ligne = el("div", "cle-valeur");
      const qui = el("span", "");
      qui.append(carreSerie(serie), " ", nom);
      ligne.append(qui, el("b", "", texteFidelite(fidelite)));
      liste.append(ligne);
    }
    return liste;
  }

  // Section des écarts : à partir de deux profils ; « profils identiques » et « mêmes mesures »
  // dits en clair.
  function ecarts(comparaison, profils, texteFidelite) {
    const section = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(titre("comparer.ecarts"));
    section.append(tete);
    if (profils.length < 2) {
      section.append(el("p", "why", t("comparer.ecarts.deux")));
      return section;
    }
    const reference = profils[0];
    profils.slice(1).forEach((p, k) => {
      const i = k + 1;
      const e = comparaison.ecarts[i];
      const entete = el("p", "prose");
      entete.append(carreSerie(p.serie), " ", t("comparer.ecarts.titre")
        .replace("{nom}", () => p.nom)
        .replace("{reference}", () => reference.nom));
      section.append(entete);
      if (!e || !reference.analyse || !p.analyse) {
        section.append(el("p", "why", t("comparer.ecarts.refuse")));
        return;
      }
      if (e.identiques) section.append(el("div", "notice", t("comparer.ecarts.identiques")));
      if (comparaison.memes_mesures[i]) {
        const avis = el("div", "notice");
        avis.append(el("p", "", t("comparer.ecarts.memes_mesures")), sousTitre(t("comparer.ecarts.fidelites")),
          fidelites([reference, p], texteFidelite));
        section.append(avis);
      }
      section.append(blocEcarts(e, reference));
    });
    return section;
  }

  return { primaires, ecarts, carreSerie, tableau, lab, cleZone };
})();
