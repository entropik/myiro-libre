// Graphiques d'une comparaison de profils (ticket #71), en SVG, sans bibliothèque : saturation par
// teinte, coupe à clarté constante (plan a*b*), et L* et C* le long d'un chemin du cube RVB. Chaque
// graphique a ses axes, ses unités, sa légende (le nom de chaque profil : la couleur ne parle jamais
// seule), un `aria-label`, et ses valeurs dans un tableau replié « Valeurs ». Les couleurs viennent
// des jetons `--serie-1` à `--serie-4`, par `data-serie` (components.css). Tous les chiffres passent
// par nombre() : une valeur absente (`null` dans le JSON) s'écrit « inconnu » et coupe la courbe.
"use strict";

window.Graphiques = (() => {
  const t = (cle) => textes[cle] || cle;
  const SVG = "http://www.w3.org/2000/svg";
  const LARGEUR = 640;
  const MARGE = { gauche: 52, droite: 16, haut: 16, bas: 44 };
  const LIGNE_LEGENDE = 18;

  // Chiffre écrit dans la langue de la page (virgule décimale en français), ou « inconnu ».
  function nombre(valeur, decimales) {
    if (typeof valeur !== "number" || !Number.isFinite(valeur)) return t("comparer.inconnu");
    const langue = document.documentElement.lang || "fr";
    return new Intl.NumberFormat(langue, {
      minimumFractionDigits: decimales,
      maximumFractionDigits: decimales,
    }).format(valeur);
  }

  function connue(v) {
    return typeof v === "number" && Number.isFinite(v);
  }

  function noeud(nom, attributs = {}, texte) {
    const e = document.createElementNS(SVG, nom);
    for (const [cle, valeur] of Object.entries(attributs)) e.setAttribute(cle, String(valeur));
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  function noms(series) {
    return series.map((s) => s.nom).join(", ");
  }

  // Haut d'un axe de chroma : le maximum connu, arrondi à la vingtaine au-dessus (20 au moins).
  function hautChroma(valeurs) {
    const max = valeurs.filter(connue).reduce((m, v) => Math.max(m, v), 0);
    return Math.max(20, Math.ceil(max / 20) * 20);
  }

  function serie(s) {
    return Math.min(Math.max(Number(s.serie) || 1, 1), 4);
  }

  // Vide le SVG et pose son cadre : taille, rôle d'image et texte de remplacement.
  function preparer(svg, hauteur, aria) {
    svg.replaceChildren();
    svg.setAttribute("viewBox", `0 0 ${LARGEUR} ${hauteur}`);
    svg.setAttribute("class", "graphique");
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", aria);
  }

  // Zone de tracé et passage des valeurs aux coordonnées du SVG.
  function zone(x0, x1, y0, y1, gauche, droite, haut, bas) {
    return {
      x: (v) => gauche + ((v - x0) / (x1 - x0)) * (droite - gauche),
      y: (v) => bas - ((v - y0) / (y1 - y0)) * (bas - haut),
      gauche, droite, haut, bas,
    };
  }

  // Grille, graduations et titres des axes.
  function axes(svg, z, x, y) {
    for (let v = x.min; v <= x.max + 1e-9; v += x.pas) {
      svg.append(noeud("line", { class: "graphique__grille", x1: z.x(v), x2: z.x(v), y1: z.haut, y2: z.bas }));
      svg.append(noeud("text", { class: "graphique__texte", x: z.x(v), y: z.bas + 14, "text-anchor": "middle" }, nombre(v, 0)));
    }
    for (let v = y.min; v <= y.max + 1e-9; v += y.pas) {
      svg.append(noeud("line", { class: "graphique__grille", x1: z.gauche, x2: z.droite, y1: z.y(v), y2: z.y(v) }));
      svg.append(noeud("text", { class: "graphique__texte", x: z.gauche - 6, y: z.y(v) + 4, "text-anchor": "end" }, nombre(v, 0)));
    }
    svg.append(noeud("line", { class: "graphique__axe", x1: z.gauche, x2: z.droite, y1: z.bas, y2: z.bas }));
    svg.append(noeud("line", { class: "graphique__axe", x1: z.gauche, x2: z.gauche, y1: z.haut, y2: z.bas }));
    svg.append(noeud("text", { class: "graphique__titre", x: (z.gauche + z.droite) / 2, y: z.bas + 32, "text-anchor": "middle" }, x.titre));
    const yTitre = (z.haut + z.bas) / 2;
    svg.append(noeud("text", { class: "graphique__titre", x: 14, y: yTitre, "text-anchor": "middle", transform: `rotate(-90 14 ${yTitre})` }, y.titre));
  }

  // Courbe d'une série ; un point inconnu (null) la coupe en morceaux.
  function trace(svg, points, s, fermer = false) {
    let d = "";
    let debut = true;
    let complet = true;
    for (const p of points) {
      if (!p) {
        debut = true;
        complet = false;
        continue;
      }
      d += `${debut ? "M" : "L"}${Math.round(p[0] * 100) / 100},${Math.round(p[1] * 100) / 100} `;
      debut = false;
    }
    if (fermer && complet && d) d += "Z";
    if (d) svg.append(noeud("path", { class: "graphique__trace", "data-serie": serie(s), d: d.trim() }));
  }

  // Légende sous le tracé, deux entrées par ligne : trait de la série et nom du profil.
  function legende(svg, series, y, avecMesures) {
    const entrees = series.map((s) => ({ nom: s.nom, serie: serie(s), point: false }));
    if (avecMesures) entrees.push({ nom: t("comparer.graphique.mesures"), serie: 0, point: true });
    const colonne = (LARGEUR - MARGE.gauche - MARGE.droite) / 2;
    entrees.forEach((e, i) => {
      const x = MARGE.gauche + (i % 2) * colonne;
      const ligne = y + Math.floor(i / 2) * LIGNE_LEGENDE;
      if (e.point) svg.append(noeud("rect", { class: "graphique__point", x: x + 8, y: ligne - 6, width: 6, height: 6 }));
      else svg.append(noeud("line", { class: "graphique__trace", "data-serie": e.serie, x1: x, x2: x + 22, y1: ligne - 3, y2: ligne - 3 }));
      const nom = e.nom.length > 44 ? `${e.nom.slice(0, 43)}…` : e.nom;
      const texte = noeud("text", { class: "graphique__legende", x: x + 30, y: ligne }, nom);
      texte.append(noeud("title", {}, e.nom));
      svg.append(texte);
    });
    return Math.ceil(entrees.length / 2) * LIGNE_LEGENDE;
  }

  // Tableau replié « Valeurs », juste après le graphique : on y lit les chiffres.
  function valeurs(apres, entetes, lignes) {
    let details = apres.nextElementSibling;
    if (!details || !details.hasAttribute("data-valeurs")) {
      details = document.createElement("details");
      details.className = "replie";
      details.setAttribute("data-valeurs", "");
      apres.after(details);
    }
    const resume = document.createElement("summary");
    resume.textContent = t("comparer.graphique.valeurs");
    const table = document.createElement("table");
    table.className = "table--compact";
    const tete = table.createTHead().insertRow();
    entetes.forEach((texte, i) => {
      const th = document.createElement("th");
      th.textContent = texte;
      if (i > 0) th.className = "r lc";
      tete.append(th);
    });
    const corps = table.createTBody();
    for (const ligne of lignes) {
      const tr = corps.insertRow();
      ligne.forEach((texte, i) => {
        const td = tr.insertCell();
        td.textContent = texte;
        td.className = i > 0 ? "r num" : "num";
      });
    }
    details.replaceChildren(resume, table);
  }

  // Hauteur d'un graphique cartésien : tracé, légende, marges.
  function hauteurAvecLegende(trace, entrees) {
    return MARGE.haut + trace + MARGE.bas + Math.ceil(entrees / 2) * LIGNE_LEGENDE + 8;
  }

  // Saturation maximale par teinte : 180 valeurs (une tous les 2°), teinte de 0 à 360°.
  function teintes(svg, series) {
    const TRACE = 240;
    preparer(svg, hauteurAvecLegende(TRACE, series.length), t("comparer.graphique.teintes.aria").replace("{profils}", () => noms(series)));
    const max = hautChroma(series.flatMap((s) => s.valeurs || []));
    const z = zone(0, 360, 0, max, MARGE.gauche, LARGEUR - MARGE.droite, MARGE.haut, MARGE.haut + TRACE);
    axes(svg, z, { min: 0, max: 360, pas: 60, titre: t("comparer.graphique.teinte") }, { min: 0, max, pas: 20, titre: t("comparer.graphique.chroma") });
    for (const s of series) {
      const points = (s.valeurs || []).map((c, i) => (connue(c) ? [z.x(i * 2 + 1), z.y(c)] : null));
      trace(svg, points, s);
    }
    legende(svg, series, z.bas + MARGE.bas + 12, false);
    const lignes = [];
    for (let h = 0; h < 360; h += 30) lignes.push([nombre(h, 0), ...series.map((s) => nombre((s.valeurs || [])[h / 2], 1))]);
    valeurs(svg, [t("comparer.graphique.teinte"), ...series.map((s) => s.nom)], lignes);
  }

  // Coupe à clarté constante, dans le plan a*b* (de −100 à +100, plus si un profil en sort).
  function coupe(svg, series, l) {
    const COTE = 420;
    const hauteur = MARGE.haut + COTE + MARGE.bas + Math.ceil(series.length / 2) * LIGNE_LEGENDE + 8;
    const aria = t("comparer.graphique.coupe.aria").replace("{l}", () => nombre(l, 0)).replace("{profils}", () => noms(series));
    preparer(svg, hauteur, aria);
    const borne = Math.max(100, hautChroma(series.flatMap((s) => s.valeurs || [])));
    const gauche = (LARGEUR - COTE) / 2;
    const z = zone(-borne, borne, -borne, borne, gauche, gauche + COTE, MARGE.haut, MARGE.haut + COTE);
    axes(svg, z, { min: -borne, max: borne, pas: 20, titre: "a*" }, { min: -borne, max: borne, pas: 20, titre: "b*" });
    svg.append(noeud("line", { class: "graphique__axe", x1: z.x(0), x2: z.x(0), y1: z.haut, y2: z.bas }));
    svg.append(noeud("line", { class: "graphique__axe", x1: z.gauche, x2: z.droite, y1: z.y(0), y2: z.y(0) }));
    for (const s of series) {
      const points = (s.valeurs || []).map((c, i) => {
        if (!connue(c)) return null;
        const h = ((i * 2 + 1) * Math.PI) / 180;
        return [z.x(c * Math.cos(h)), z.y(c * Math.sin(h))];
      });
      trace(svg, points, s, true);
    }
    legende(svg, series, z.bas + MARGE.bas + 12, false);
    const lignes = [];
    for (let h = 0; h < 360; h += 30) lignes.push([nombre(h, 0), ...series.map((s) => nombre((s.valeurs || [])[h / 2], 1))]);
    valeurs(svg, [t("comparer.graphique.teinte"), ...series.map((s) => `${s.nom} (C*)`)], lignes);
  }

  // L* et C* le long d'un chemin du cube RVB : 256 points (L*, C*) par profil, de 0 à 255, et les
  // plages mesurées du chemin (position, L*, C*) en points carrés.
  function chemin(svgL, svgC, series) {
    const TRACE = 200;
    const avecMesures = series.some((s) => (s.mesures || []).length > 0);
    const entrees = series.length + (avecMesures ? 1 : 0);
    const dessiner = (svg, indice, aria, titre, max) => {
      preparer(svg, hauteurAvecLegende(TRACE, entrees), t(aria).replace("{profils}", () => noms(series)));
      const z = zone(0, 255, 0, max, MARGE.gauche, LARGEUR - MARGE.droite, MARGE.haut, MARGE.haut + TRACE);
      axes(svg, z, { min: 0, max: 255, pas: 51, titre: t("comparer.graphique.position") }, { min: 0, max, pas: 20, titre });
      for (const s of series) {
        const points = (s.valeurs || []).map((p, i) => (p && connue(p[indice]) ? [z.x(i), z.y(p[indice])] : null));
        trace(svg, points, s);
        for (const m of s.mesures || []) {
          if (!connue(m[0]) || !connue(m[indice + 1])) continue;
          svg.append(noeud("rect", { class: "graphique__point", "data-serie": serie(s), x: z.x(m[0]) - 3, y: z.y(m[indice + 1]) - 3, width: 6, height: 6 }));
        }
      }
      legende(svg, series, z.bas + MARGE.bas + 12, avecMesures);
    };
    dessiner(svgL, 0, "comparer.graphique.chemin.l.aria", t("comparer.graphique.clarte"), 100);
    const chromas = series.flatMap((s) => [...(s.valeurs || []).map((p) => p && p[1]), ...(s.mesures || []).map((m) => m[2])]);
    dessiner(svgC, 1, "comparer.graphique.chemin.c.aria", t("comparer.graphique.chroma"), hautChroma(chromas));
    const lignes = [0, 32, 64, 96, 128, 160, 192, 224, 255].map((i) => [
      nombre(i, 0),
      ...series.flatMap((s) => {
        const p = (s.valeurs || [])[i];
        return [nombre(p && p[0], 1), nombre(p && p[1], 1)];
      }),
    ]);
    valeurs(svgC, [t("comparer.graphique.position"), ...series.flatMap((s) => [`${s.nom} (L*)`, `${s.nom} (C*)`])], lignes);
  }

  return { nombre, teintes, coupe, chemin };
})();
