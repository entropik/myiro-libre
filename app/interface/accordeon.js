// Accordéon des grandes parties d'une vue (ticket #80, demande du mainteneur du 30 octobre 2026) :
// chaque partie a un en-tête cliquable (un bouton, Entrée ou Espace, aria-expanded et
// aria-controls, chevron) qui déploie ou replie son contenu, juste dessous : les autres en-têtes ne
// bougent que de la place du contenu ouvert, et la page ne saute pas. Par défaut, seuls les
// chiffres clés sont ouverts. L'état de chaque partie est retenu dans le navigateur (localStorage,
// dans un try/catch : la page marche sans). Une partie lourde (vue 3D, graphiques) reçoit une
// fonction `remplir(corps)` appelée à l'ouverture, dans la page, et `vider()` à la fermeture :
// repliée, elle ne calcule ni ne dessine rien. Rapport PDF (ticket #89) : à droite de l'en-tête, hors
// du bouton, une coche « Inclure dans le rapport » (retenue comme l'état, décochée par défaut) ; les
// chiffres clés et l'image y sont toujours, leur coche est cochée et inactive, avec sa raison.
"use strict";

window.Accordeon = (() => {
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const CLE = "myiro-libre.accordeon";
  const OUVERTES_PAR_DEFAUT = new Set(["comparer.chiffres"]);
  let numero = 0;
  let etats = lire();

  function lire() {
    try {
      const e = JSON.parse(localStorage.getItem(CLE) || "{}");
      return e && typeof e === "object" ? e : {};
    } catch {
      return {};
    }
  }

  function garder() {
    try {
      localStorage.setItem(CLE, JSON.stringify(etats));
    } catch {
      // Sans stockage, l'état vaut pour la séance seulement.
    }
  }

  const ouverte = (cle) => (cle in etats ? Boolean(etats[cle]) : OUVERTES_PAR_DEFAUT.has(cle));

  // ---- Coches du rapport PDF ----
  const CLE_RAPPORT = "myiro-libre.rapport";
  const RAPPORT_TOUJOURS = new Set(["comparer.chiffres", "image.titre"]);
  const RAPPORT_COCHABLES = new Set(["comparer.volume", "comparer.teintes", "comparer.coupe", "comparer.chemin",
    "comparer.primaires", "comparer.ecarts"]);
  let coches = lireCoches();

  function lireCoches() {
    try {
      const e = JSON.parse(localStorage.getItem(CLE_RAPPORT) || "{}");
      return e && typeof e === "object" ? e : {};
    } catch {
      return {};
    }
  }

  function garderCoches() {
    try {
      localStorage.setItem(CLE_RAPPORT, JSON.stringify(coches));
    } catch {
      // Sans stockage, les coches valent pour la séance seulement.
    }
  }

  // La coche d'une partie, posée à côté de son bouton ; rien pour une partie hors du rapport.
  function coche(cle) {
    if (!RAPPORT_TOUJOURS.has(cle) && !RAPPORT_COCHABLES.has(cle)) return document.createDocumentFragment();
    const l = el("label", "accordeon__rapport");
    const c = el("input");
    c.type = "checkbox";
    if (RAPPORT_TOUJOURS.has(cle)) {
      c.checked = true;
      c.disabled = true;
      l.title = t("rapport.toujours"); // une coche inactive dit pourquoi
    } else {
      c.checked = Boolean(coches[cle]);
      c.addEventListener("change", () => { coches[cle] = c.checked; garderCoches(); });
    }
    l.append(c, el("span", "", t("rapport.inclure")));
    return l;
  }

  // Les parties cochées pour le rapport, dans l'ordre de la vue.
  const cochees = () => [...RAPPORT_COCHABLES].filter((cle) => coches[cle]);

  // Une partie : `contenu` (liste de nœuds) est posé tout de suite ; ou bien `remplir(corps)` le
  // pose à l'ouverture, une fois le corps dans la page, et `vider()` défait ce qu'il a créé.
  function partie(cle, contenu = [], remplir = null, vider = null) {
    const id = `partie-${++numero}`;
    const bloc = el("div", "section accordeon");
    bloc.dataset.partie = cle;
    const titre = el("h2", "section__head accordeon__titre");
    const bouton = el("button", "accordeon__bouton");
    Object.assign(bouton, { type: "button", id: id + "-titre" });
    bouton.setAttribute("aria-controls", id);
    bouton.append(el("span", "label label--ink", t(cle)), el("span", "accordeon__chevron"));
    bouton.lastChild.setAttribute("aria-hidden", "true");
    titre.append(bouton);
    titre.append(coche(cle));
    const corps = el("div", "accordeon__corps");
    corps.id = id;
    corps.setAttribute("role", "region");
    corps.setAttribute("aria-labelledby", bouton.id);
    corps.append(...contenu);
    bloc.append(titre, corps);
    let rempli = false;
    function appliquer(ouvrir) {
      bouton.setAttribute("aria-expanded", String(ouvrir));
      corps.hidden = !ouvrir;
      if (!remplir) return;
      if (ouvrir && !rempli && corps.isConnected) {
        rempli = true;
        remplir(corps);
      } else if (!ouvrir && rempli) {
        rempli = false;
        if (vider) vider();
        corps.replaceChildren();
      }
    }
    bouton.addEventListener("click", () => {
      etats[cle] = !ouverte(cle);
      garder();
      appliquer(etats[cle]);
    });
    bloc.appliquer = appliquer;
    appliquer(ouverte(cle));
    // Une partie lourde ouverte d'office se remplit dès qu'elle est posée dans la page.
    if (remplir && ouverte(cle)) queueMicrotask(() => appliquer(ouverte(cle)));
    return bloc;
  }

  // Ouvre une partie (par exemple « Image » quand une image s'ouvre), affichée ou à venir.
  function ouvrir(cle) {
    etats[cle] = true;
    garder();
    for (const bloc of document.querySelectorAll(".accordeon")) {
      if (bloc.dataset.partie === cle) bloc.appliquer(true);
    }
  }

  return { partie, ouvrir, ouverte, cochees };
})();
