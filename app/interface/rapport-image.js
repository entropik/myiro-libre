// Rapport PDF d'une image (ticket #89). Le bouton « Rapport PDF… » de la vue « Comparer » est actif
// dès qu'une image est analysée ; sinon il dit pourquoi, juste dessous, à une place gardée. Il ouvre
// une fenêtre modale : l'atelier (nom, logo, « Modifier l'atelier »), Client, Objet, Langue, la liste
// de ce qui part dans le rapport, puis « Exporter le PDF », seul bouton principal. Le module Rust
// construit le rapport depuis l'image et l'analyse gardées ; la page n'envoie que les choix et les
// graphiques des parties cochées. Ces graphiques sont redessinés hors écran, dans la langue du
// rapport et en thème clair, le temps d'un seul passage sans attente : rien ne bouge dans la vue.
// Les SVG ne gardent que la liste blanche des balises, leurs styles figés en attributs ; la vue 3D
// part en PNG. Pendant l'export, « Rapport en cours… » s'écrit à une place gardée de la fenêtre.
"use strict";

window.RapportImage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const NS = "http://www.w3.org/2000/svg";
  const BALISES = new Set(["svg", "g", "path", "line", "polyline", "polygon", "rect", "circle", "ellipse", "text",
    "tspan", "title", "desc"]);
  const GARDES = ["viewBox", "width", "height", "transform", "d", "x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "r",
    "rx", "ry", "dx", "dy", "points", "text-anchor", "dominant-baseline"];
  const FIGES = ["fill", "fill-opacity", "fill-rule", "stroke", "stroke-width", "stroke-dasharray", "stroke-opacity",
    "stroke-linecap", "stroke-linejoin", "opacity", "font-size", "font-weight", "font-family"];
  const CACHE = "fenetre-rapport__dossier--cache";

  let bouton = null; // « Rapport PDF… » de la vue Comparer
  let raison = null; // sa ligne, juste dessous
  let ouverte = null; // { dialogue, logo } de la fenêtre ouverte
  let client = "";
  let objet = "";
  let occupe = false;

  // ---- Le bouton de la vue Comparer ----
  function brancher(b, r) {
    [bouton, raison] = [b, r];
    bouton.addEventListener("click", ouvrir);
    for (const nom of ["image-dessinee", "comparer-fermee", "langue-appliquee"]) document.addEventListener(nom, etat);
    document.addEventListener("comparer-fermee", fermer);
    etat();
  }

  function etat() {
    if (!bouton) return;
    const pret = Boolean(window.ComparerImage.pourRapport());
    bouton.disabled = !pret;
    raison.textContent = t(pret ? "rapport.bouton.aide" : "rapport.bouton.raison");
  }

  // ---- Captures hors écran ----
  const echapper = (v) => String(v).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

  // Un nœud SVG en texte : balises de la liste blanche seulement, styles calculés écrits en attributs.
  function serialiser(noeud) {
    if (noeud.nodeType === Node.TEXT_NODE) return echapper(noeud.nodeValue);
    if (noeud.nodeType !== Node.ELEMENT_NODE || !BALISES.has(noeud.localName)) return "";
    const style = getComputedStyle(noeud);
    if (style.display === "none" || style.visibility === "hidden") return "";
    const attributs = noeud.localName === "svg" ? [["xmlns", NS]] : [];
    for (const a of GARDES) if (noeud.hasAttribute(a)) attributs.push([a, noeud.getAttribute(a)]);
    for (const a of FIGES) {
      const v = style.getPropertyValue(a).trim();
      if (v && !/url\(/i.test(v)) attributs.push([a, v]);
    }
    if (noeud.localName === "svg" && !noeud.hasAttribute("width")) {
      const [, , l, h] = (noeud.getAttribute("viewBox") || "").trim().split(/[\s,]+/);
      if (l && h) attributs.push(["width", l], ["height", h]);
    }
    const ouvre = noeud.localName + attributs.map(([n, v]) => ` ${n}="${echapper(v)}"`).join("");
    return `<${ouvre}>${[...noeud.childNodes].map(serialiser).join("")}</${noeud.localName}>`;
  }

  const texte = (cellule) => cellule.textContent.replace(/\s+/g, " ").trim();

  // Les tableaux d'un nœud, en textes : en-têtes, puis lignes.
  function tableaux(partie, noeud, zone) {
    zone.append(noeud);
    return [...noeud.querySelectorAll("table")].map((table) => ({
      partie,
      tableau: {
        entetes: [...(table.tHead?.rows[0]?.cells || [])].map(texte),
        lignes: [...table.tBodies].flatMap((b) => [...b.rows].map((r) => [...r.cells].map(texte))),
      },
    }));
  }

  // `n` SVG posés dans la zone, dessinés par `dessiner`, puis sérialisés.
  function svgs(partie, zone, n, dessiner) {
    const liste = [...Array(n)].map(() => zone.appendChild(document.createElementNS(NS, "svg")));
    dessiner(liste);
    return liste.map((s) => ({ partie, svg: serialiser(s) }));
  }

  const serie = (p, valeurs) => ({ valeurs, serie: p.serie, nom: p.nom });

  // Le dessin de chaque partie cochée, avec les données de la vue (comparer.js).
  const DESSINS = {
    "comparer.volume": (d, zone) => {
      const canvas = el("canvas", "vue3d");
      canvas.style.width = "1200px";
      canvas.style.height = "900px";
      zone.append(canvas);
      const vue = window.Vue3d.creer(canvas, { couleur: d.reglages.couleur, rendu: d.reglages.rendu });
      try {
        vue.afficher(d.formes);
        vue.points(window.NuageImage.points());
        return [{ partie: "comparer.volume", png_base64: vue.capturer() }];
      } finally {
        vue.detruire();
      }
    },
    "comparer.teintes": (d, zone) => svgs("comparer.teintes", zone, 1, ([s]) =>
      window.Graphiques.teintes(s, d.liste.map((p) => serie(p, p.analyse.chroma_par_teinte)))),
    "comparer.coupe": (d, zone) => svgs("comparer.coupe", zone, 1, ([s]) => window.Graphiques.coupe(s,
      d.liste.map((p) => serie(p, p.analyse.coupes[d.reglages.l])), d.reglages.l, window.NuageImage.points())),
    "comparer.chemin": (d, zone) => svgs("comparer.chemin", zone, 2, ([l, c]) => window.Graphiques.chemin(l, c,
      d.liste.map((p) => ({ ...serie(p, (d.chemins[p.rang] || {}).points || []), mesures: (d.chemins[p.rang] || {}).mesures || [] })))),
    "comparer.primaires": (d, zone) => tableaux("comparer.primaires", window.ComparerEcarts.primaires(d.liste), zone),
    "comparer.ecarts": (d, zone) => (d.comparaison ? tableaux("comparer.ecarts",
      window.ComparerEcarts.ecarts(d.comparaison, d.profils, window.ComparerDetails.texteFidelite), zone) : []),
  };

  // Les chiffres clés (toujours) et les parties cochées, redessinés hors écran dans la langue du
  // rapport et en thème clair, d'un seul passage synchrone : la vue n'est ni repeinte ni déplacée.
  function captures(catalogue, langue) {
    const racine = document.documentElement;
    const [avant, lang, theme] = [textes, racine.lang, racine.dataset.theme];
    const d = window.Comparer.donneesRapport();
    const zone = el("div", "hors-ecran");
    zone.setAttribute("aria-hidden", "true");
    const graphiques = [];
    try {
      textes = catalogue;
      racine.lang = langue;
      racine.dataset.theme = "light";
      document.body.append(zone);
      if (d.liste.length) graphiques.push(...tableaux("comparer.chiffres", window.ComparerDetails.chiffres(d.liste), zone));
      for (const cle of window.Accordeon.cochees()) {
        if (DESSINS[cle] && d.liste.length) graphiques.push(...DESSINS[cle](d, zone));
      }
    } finally {
      zone.remove();
      textes = avant;
      racine.lang = lang;
      if (theme === undefined) delete racine.dataset.theme;
      else racine.dataset.theme = theme;
    }
    return graphiques;
  }

  // ---- La fenêtre d'export ----
  function champ(libelle, controle) {
    const c = el("div", "field");
    const l = el("label", "label", libelle);
    controle.id = "rapport-" + Math.random().toString(36).slice(2);
    l.htmlFor = controle.id;
    c.append(l, controle);
    return c;
  }

  function saisie(valeur) {
    const i = el("input", "input");
    [i.type, i.value, i.maxLength] = ["text", valeur, 200];
    return i;
  }

  // L'atelier : son logo et son nom, ou la phrase qui invite à le remplir.
  async function lireAtelier(zone) {
    let vue = null;
    try {
      vue = await invoke("atelier_lire");
    } catch (cle) {
      zone.replaceChildren(el("p", "why", t(String(cle?.cle || cle))));
      return;
    }
    if (!ouverte) return;
    const nom = el("b", "", vue.saisi.nom || t("rapport.atelier_vide"));
    const contenu = [nom];
    if (vue.logo) {
      const octets = await invoke("atelier_logo").catch(() => null);
      if (octets && ouverte) {
        if (ouverte.logo) URL.revokeObjectURL(ouverte.logo);
        ouverte.logo = URL.createObjectURL(new Blob([octets], { type: "image/png" }));
        const img = el("img", "fenetre-rapport__logo");
        [img.src, img.alt] = [ouverte.logo, t("rapport.logo")];
        contenu.unshift(img);
      }
    }
    zone.replaceChildren(...contenu);
  }

  function ouvrir() {
    fermer();
    const dialogue = el("dialog", "fenetre-rapport");
    dialogue.setAttribute("aria-labelledby", "fenetre-rapport-titre");
    ouverte = { dialogue, logo: null };
    const titre = el("h2", "titre", t("rapport.titre"));
    titre.id = "fenetre-rapport-titre";

    const atelier = el("div", "fenetre-rapport__atelier");
    const resume = el("div", "fenetre-rapport__resume", t("rapport.atelier_lecture"));
    const modifier = el("button", "btn", t("rapport.modifier_atelier"));
    modifier.addEventListener("click", () => window.Atelier?.ouvrir(() => lireAtelier(resume)));
    atelier.append(resume, modifier);

    const champClient = saisie(client);
    const champObjet = saisie(objet);
    champClient.addEventListener("input", () => { client = champClient.value; });
    champObjet.addEventListener("input", () => { objet = champObjet.value; });
    const langue = el("select", "input");
    for (const code of ["fr", "en"]) {
      const o = el("option", "", t("rapport.langue." + code));
      [o.value, o.selected] = [code, code === (document.documentElement.lang || "fr")];
      langue.append(o);
    }

    const parties = el("ul", "fenetre-rapport__parties");
    for (const cle of ["rapport.section.verdict", "image.titre", "image.chiffres", "comparer.chiffres", ...window.Accordeon.cochees()]) {
      parties.append(el("li", "", t(cle)));
    }
    const points = window.PastillesImage.pointsReleves().length;
    parties.append(el("li", "", t("rapport.points.nombre").replace("{n}", () => points)));

    const statut = el("p", "why fenetre-rapport__statut");
    statut.setAttribute("role", "status");
    const dossier = el("button", "btn " + CACHE, t("rapport.ouvrir_dossier"));
    const ligneStatut = el("div", "fenetre-rapport__ligne");
    ligneStatut.append(statut, dossier);

    const exporterBtn = el("button", "btn btn--primary", t("rapport.exporter"));
    const fermerBtn = el("button", "btn", t("rapport.fermer"));
    fermerBtn.addEventListener("click", () => dialogue.close());
    const actions = el("div", "actions");
    actions.append(exporterBtn, fermerBtn);

    let chemin = "";
    dossier.addEventListener("click", () => invoke("rapport_ouvrir_dossier", { chemin }).catch((cle) => {
      statut.textContent = t(String(cle));
    }));
    exporterBtn.addEventListener("click", async () => {
      const r = await exporter(langue.value, statut, exporterBtn, dossier);
      if (r) chemin = r.chemin;
    });

    dialogue.append(titre, atelier, champ(t("rapport.champ.client"), champClient), champ(t("rapport.champ.objet"), champObjet),
      champ(t("rapport.champ.langue"), langue), el("span", "label", t("rapport.contenu")), parties, ligneStatut, actions);
    // Échap ferme aussi la fenêtre (événement « close » du <dialog>).
    dialogue.addEventListener("close", rendre);
    document.body.append(dialogue);
    dialogue.showModal();
    champClient.focus();
    lireAtelier(resume);
    // Premier rapport : l'atelier vide s'ouvre d'abord à remplir (atelier.js).
    window.Atelier?.premiere(() => lireAtelier(resume));
  }

  async function exporter(langue, statut, exporterBtn, dossier) {
    const etatImage = window.ComparerImage.pourRapport();
    if (!etatImage || occupe) return null;
    occupe = true;
    exporterBtn.disabled = true;
    dossier.classList.add(CACHE);
    statut.textContent = t("rapport.en_cours");
    try {
      const catalogue = langue === (document.documentElement.lang || "fr") ? textes : await invoke("catalogue", { langue });
      const demande = {
        langue, client: client.trim(), objet: objet.trim(), ...etatImage,
        points: window.PastillesImage.pointsReleves(), parties: window.Accordeon.cochees(),
        graphiques: captures(catalogue, langue),
      };
      const r = await invoke("image_rapport_exporter", { demande, filtre: t("rapport.filtre") });
      statut.textContent = r ? t("rapport.enregistre").replace("{nom}", () => r.nom) : "";
      if (r) dossier.classList.remove(CACHE);
      return r;
    } catch (cle) {
      statut.textContent = t(String(cle));
      return null;
    } finally {
      occupe = false;
      exporterBtn.disabled = false;
    }
  }

  // La fenêtre disparaît, le focus revient au bouton « Rapport PDF… ».
  function rendre() {
    if (!ouverte) return;
    const { dialogue, logo } = ouverte;
    ouverte = null;
    if (logo) URL.revokeObjectURL(logo);
    dialogue.remove();
    if (bouton && bouton.isConnected) bouton.focus();
  }

  function fermer() {
    if (!ouverte) return;
    const { dialogue } = ouverte;
    rendre();
    if (dialogue.open) dialogue.close();
  }

  return { brancher, fermer, captures };
})();
