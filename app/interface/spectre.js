// Courbe de spectre (ticket #10) : réflectance de chaque condition de mesure (M0, M1, M2)
// selon la longueur d'onde, pour une mesure choisie et celles gardées pour comparer.
// Les spectres viennent de Rust, déjà rangés par condition (`bibliotheque::SpectresMesure`) :
// une condition absente reste « non disponible », jamais tracée à zéro. La partie
// « données » (preparer, lire, nmSous) est testée sans navigateur
// (`app/tests/interface/spectre.test.js`) ; la partie « écran » dessine en SVG avec les
// filets et les couleurs du système graphique. Aucun texte en dur : catalogue (`textes`).
"use strict";

const CourbeSpectre = (() => {
  const PAS_Y = 20; // graduations de réflectance, en %
  const PAS_X = 50; // graduations de longueur d'onde, en nm
  const GARDEES_MAX = 3; // mesures gardées en plus de la mesure choisie

  const arrondi = (v) => Math.round(v * 10) / 10;

  // ---- Données : séries → courbes placées dans le cadre ----
  // `series` : [{ donnees: SpectresMesure, plage }] ; `cadre` : largeur, hauteur et marges.
  function preparer(series, cadre) {
    const placables = [];
    const sansLongueurs = [];
    series.forEach((s, i) => {
      const lo = s.donnees.longueurs_onde;
      if (lo.statut === "inconnue" || !lo.valeur.length) sansLongueurs.push(i);
      else placables.push(i);
    });
    const tous = placables.flatMap((i) => series[i].donnees.longueurs_onde.valeur);
    const xMin = tous.length ? Math.min(...tous) : 0;
    const xMax = tous.length ? Math.max(...tous) : 0;
    let plusHaut = 0;
    for (const i of placables) {
      for (const c of series[i].donnees.plages[series[i].plage].conditions) {
        if (c) plusHaut = Math.max(plusHaut, ...c.valeurs.map((v) => v * 100));
      }
    }
    const yMax = Math.max(100, Math.ceil(plusHaut / PAS_Y) * PAS_Y);

    const large = cadre.largeur - cadre.gauche - cadre.droite;
    const haut = cadre.hauteur - cadre.haut - cadre.bas;
    const x = (nm) => arrondi(cadre.gauche + (xMax > xMin ? ((nm - xMin) / (xMax - xMin)) * large : large / 2));
    const y = (pct) => arrondi(cadre.haut + haut * (1 - pct / yMax));

    const courbes = [];
    const nonDisponibles = [];
    series.forEach((s, serie) => {
      const conditions = s.donnees.plages[s.plage].conditions;
      conditions.forEach((c, condition) => {
        if (!c) {
          nonDisponibles.push({ serie, condition });
          return;
        }
        if (!placables.includes(serie)) return;
        const nm = s.donnees.longueurs_onde.valeur;
        const points = c.valeurs.map((v, k) => [x(nm[k]), y(v * 100)]);
        const d = points.map(([px, py], k) => `${k ? "L" : "M"}${px} ${py}`).join("");
        courbes.push({ serie, condition, supposee: c.supposee, points, d });
      });
    });

    const axeX = [];
    for (let nm = Math.ceil(xMin / PAS_X) * PAS_X; nm <= xMax; nm += PAS_X) axeX.push({ nm, x: x(nm) });
    // Graduations aux multiples de 50 nm ; les bouts seulement si elles manquent.
    if (axeX.length < 2) {
      if (!axeX.length || axeX[0].nm !== xMin) axeX.unshift({ nm: xMin, x: x(xMin) });
      if (axeX[axeX.length - 1].nm !== xMax) axeX.push({ nm: xMax, x: x(xMax) });
    }
    const axeY = [];
    for (let v = 0; v <= yMax; v += PAS_Y) axeY.push({ valeur: v, y: y(v) });

    return { series, cadre, xMin, xMax, yMax, x, y, courbes, nonDisponibles, sansLongueurs, axeX, axeY };
  }

  // Longueur d'onde mesurée la plus proche de `nm`, parmi celles des séries placées.
  function plusProche(trace, nm) {
    let meilleure = null;
    for (const s of trace.series) {
      if (s.donnees.longueurs_onde.statut === "inconnue") continue;
      for (const l of s.donnees.longueurs_onde.valeur) {
        if (meilleure === null || Math.abs(l - nm) < Math.abs(meilleure - nm)) meilleure = l;
      }
    }
    return meilleure;
  }

  // Réflectance (%) de chaque série et condition à la longueur d'onde mesurée la plus
  // proche ; `null` : condition non disponible, ou pas mesurée à cette longueur d'onde.
  function lire(trace, nm) {
    const proche = plusProche(trace, nm);
    const valeurs = trace.series.map((s) => {
      const lo = s.donnees.longueurs_onde;
      const k = lo.statut === "inconnue" ? -1 : lo.valeur.indexOf(proche);
      return s.donnees.plages[s.plage].conditions.map((c) =>
        (c && k >= 0 ? arrondi(c.valeurs[k] * 100) : null));
    });
    return { nm: proche, valeurs };
  }

  // Longueur d'onde lue sous l'abscisse `px` du cadre.
  function nmSous(trace, px) {
    const { gauche, droite, largeur } = trace.cadre;
    const part = Math.min(1, Math.max(0, (px - gauche) / (largeur - gauche - droite)));
    return plusProche(trace, trace.xMin + part * (trace.xMax - trace.xMin));
  }

  // ---- Écran ----
  const NS = "http://www.w3.org/2000/svg";
  const CONDITIONS = ["M0", "M1", "M2"];
  let gardees = []; // [{ cle, titre, donnees, plage }], communes à la bibliothèque et à Mesurer

  function el(nom, classe, texte) {
    const e = document.createElement(nom);
    if (classe) e.className = classe;
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  function svg(nom, attributs, texte) {
    const e = document.createElementNS(NS, nom);
    for (const [cle, valeur] of Object.entries(attributs)) e.setAttribute(cle, String(valeur));
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  // Petit trait d'exemple d'une condition, pour l'en-tête de la légende.
  function echantillon(condition) {
    const s = svg("svg", { class: "spectre__echantillon", viewBox: "0 0 24 8", "aria-hidden": "true" });
    s.append(svg("line", { class: `spectre__courbe spectre__courbe--m${condition}`, x1: 0, y1: 4, x2: 24, y2: 4 }));
    return s;
  }

  // Section « Courbe de spectre » de la mesure `courante` ({ cle, titre, donnees, plage }).
  // `t` : catalogue ; `nombre(v)` : nombre à l'écran dans la langue de la page.
  function section(courante, t, nombre) {
    const racine = el("div", "section spectre");

    function dessiner() {
      const series = [{ ...courante, courante: true },
        ...gardees.filter((g) => g.cle !== courante.cle)];
      racine.replaceChildren();

      const tete = el("div", "section__head");
      const plages = courante.donnees.plages;
      const titre = plages.length > 1
        ? `${t("spectre.titre")} · ${t("details.plage")} ${plages[courante.plage].identifiant}`
        : t("spectre.titre");
      tete.append(el("span", "label label--ink", titre));
      const dejaGardee = gardees.some((g) => g.cle === courante.cle);
      if (!dejaGardee) {
        const garder = el("button", "btn", t("spectre.garder"));
        garder.disabled = gardees.length >= GARDEES_MAX;
        if (garder.disabled) garder.title = t("spectre.limite");
        garder.addEventListener("click", () => {
          gardees.push({ cle: courante.cle, titre: courante.titre, donnees: courante.donnees, plage: courante.plage });
          dessiner();
        });
        tete.append(garder);
      }

      const figure = el("figure", "spectre__trace");
      figure.setAttribute("role", "img");
      figure.setAttribute("aria-label", t("spectre.libelle"));
      figure.tabIndex = 0;

      // Légende et lecture au survol : une ligne par mesure, une colonne par condition.
      const table = el("table", "table--compact spectre__legende");
      const entete = el("tr");
      const enteteLecture = el("th", "lc", t("spectre.lecture"));
      entete.append(el("th", "", ""), enteteLecture);
      CONDITIONS.forEach((nom, c) => {
        const th = el("th", "r");
        th.append(echantillon(c), document.createTextNode(` ${nom}`));
        entete.append(th);
      });
      entete.append(el("th", "", ""));
      const thead = el("thead");
      thead.append(entete);
      const corps = el("tbody");
      const cellules = series.map((s, i) => {
        const tr = el("tr", s.courante ? "spectre__ligne--courante" : "");
        tr.append(el("td", "id num", String(i + 1)), el("td", "", s.titre));
        const cases = CONDITIONS.map(() => el("td", "r num"));
        tr.append(...cases);
        const action = el("td", "r");
        if (!s.courante) {
          const retirer = el("button", "spectre__retirer", t("spectre.retirer"));
          retirer.addEventListener("click", () => {
            gardees = gardees.filter((g) => g.cle !== s.cle);
            dessiner();
          });
          action.append(retirer);
        }
        tr.append(action);
        corps.append(tr);
        return cases;
      });
      table.append(thead, corps);

      let trace = null;
      let lu = null; // longueur d'onde lue, ou null hors de la courbe

      function remplirCases() {
        const lecture = trace && lu !== null ? lire(trace, lu) : null;
        enteteLecture.textContent = lecture ? t("spectre.lecture_nm").replace("{}", lecture.nm) : t("spectre.lecture");
        series.forEach((s, i) => {
          const conditions = s.donnees.plages[s.plage].conditions;
          const sans = trace && trace.sansLongueurs.includes(i);
          cellules[i].forEach((td, c) => {
            td.classList.toggle("mute", !conditions[c] || sans);
            if (!conditions[c]) td.textContent = t("spectre.non_disponible");
            else if (sans) td.textContent = t("spectre.longueurs_inconnues");
            else if (lecture) {
              const v = lecture.valeurs[i][c];
              td.textContent = v === null ? t("spectre.non_disponible") : t("spectre.valeur").replace("{}", nombre(v));
            } else td.textContent = conditions[c].supposee ? t("details.a_confirmer") : "";
          });
        });
      }

      function tracer() {
        const largeur = Math.round(figure.clientWidth);
        const hauteur = Math.round(figure.clientHeight);
        if (!largeur || !hauteur) return;
        const cadre = { largeur, hauteur, gauche: 44, droite: 20, haut: 24, bas: 36 };
        trace = preparer(series, cadre);
        const dessin = svg("svg", { viewBox: `0 0 ${largeur} ${hauteur}`, width: largeur, height: hauteur, "aria-hidden": "true" });
        const bas = hauteur - cadre.bas;
        for (const g of trace.axeY) {
          dessin.append(svg("line", { class: g.valeur === 0 ? "spectre__axe" : "spectre__grille", x1: cadre.gauche, x2: largeur - cadre.droite, y1: g.y, y2: g.y }));
          dessin.append(svg("text", { class: "spectre__graduation", x: cadre.gauche - 6, y: g.y + 4, "text-anchor": "end" }, String(g.valeur)));
        }
        for (const g of trace.axeX) {
          dessin.append(svg("line", { class: "spectre__axe", x1: g.x, x2: g.x, y1: bas, y2: bas + 4 }));
          dessin.append(svg("text", { class: "spectre__graduation", x: g.x, y: bas + 16, "text-anchor": "middle" }, String(g.nm)));
        }
        dessin.append(svg("text", { class: "spectre__titre-axe", x: cadre.gauche, y: cadre.haut - 10 }, t("spectre.axe_y")));
        dessin.append(svg("text", { class: "spectre__titre-axe", x: largeur - cadre.droite, y: hauteur - 4, "text-anchor": "end" }, t("spectre.axe_x")));
        // Les mesures gardées d'abord, la mesure choisie par-dessus.
        for (const c of [...trace.courbes].reverse()) {
          const sorte = series[c.serie].courante ? "courante" : "gardee";
          dessin.append(svg("path", { class: `spectre__courbe spectre__courbe--m${c.condition} spectre__courbe--${sorte}`, d: c.d }));
        }
        // Un seul numéro par mesure, au bout de sa courbe la plus haute.
        series.forEach((_, serie) => {
          const bouts = trace.courbes.filter((c) => c.serie === serie).map((c) => c.points[c.points.length - 1]);
          if (!bouts.length) return;
          const [px, py] = bouts.reduce((a, b) => (b[1] < a[1] ? b : a));
          dessin.append(svg("text", { class: "spectre__numero", x: px + 4, y: py + 4 }, String(serie + 1)));
        });
        const repere = svg("line", { class: "spectre__repere", x1: 0, x2: 0, y1: cadre.haut, y2: bas, visibility: "hidden" });
        dessin.append(repere);
        figure.replaceChildren(dessin);

        const placerRepere = () => {
          if (lu === null || !trace.courbes.length) {
            repere.setAttribute("visibility", "hidden");
          } else {
            const px = trace.x(lire(trace, lu).nm);
            repere.setAttribute("x1", px);
            repere.setAttribute("x2", px);
            repere.setAttribute("visibility", "visible");
          }
          remplirCases();
        };
        figure.onpointermove = (e) => {
          if (!trace.courbes.length) return;
          lu = nmSous(trace, e.clientX - figure.getBoundingClientRect().left);
          placerRepere();
        };
        figure.onpointerleave = () => { lu = null; placerRepere(); };
        // Au clavier : flèches gauche et droite d'une longueur d'onde à l'autre.
        figure.onkeydown = (e) => {
          if (!trace.courbes.length || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
          e.preventDefault();
          const pas = e.key === "ArrowLeft" ? -1 : 1;
          const ecart = trace.series.find((s) => s.donnees.longueurs_onde.statut !== "inconnue");
          const valeurs = ecart ? ecart.donnees.longueurs_onde.valeur : [];
          const k = lu === null ? (pas > 0 ? -1 : valeurs.length) : valeurs.indexOf(lire(trace, lu).nm);
          const suivant = valeurs[Math.min(valeurs.length - 1, Math.max(0, k + pas))];
          if (suivant !== undefined) { lu = suivant; placerRepere(); }
        };
        figure.onblur = () => { lu = null; placerRepere(); };
        placerRepere();
      }

      racine.append(tete, figure, table);
      // Le dessin suit la largeur de la feuille ; il est refait à chaque changement de taille.
      if (observateur) observateur.disconnect();
      observateur = new ResizeObserver(tracer);
      observateur.observe(figure);
      remplirCases();
    }

    let observateur = null;
    dessiner();
    return racine;
  }

  return { preparer, lire, nmSous, section };
})();

if (typeof module !== "undefined") module.exports = CourbeSpectre;
