// Vue d'une feuille au FD-9 (ticket #34), deux onglets comme le logiciel du fabricant :
// « Mire », la mise en page théorique avec les plages dans leurs couleurs de consigne, et
// « Points mesurés », l'image de la feuille prise par le FD-9 avec un point rouge au centre de chaque
// plage trouvée. Une seule toile de taille fixe : le zoom et le déplacement changent seulement ce
// qu'elle dessine, rien ne bouge autour. Survol ou flèches du clavier : n° de la plage (« 1A1 »,
// page, colonne en lettres, rangée) et sa couleur de consigne, sur une ligne gardée sous la toile.
// Pas de correction des points. Utilisé par feuille-fd9.js ; textes par le catalogue (`textes`).
"use strict";

window.FeuilleVue = (() => {
  const t = (cle) => textes[cle] || cle;
  const racine = document.querySelector("[data-feuille-fd9]");
  const vue = racine.querySelector("[data-feuille-vue]");
  const toile = vue.querySelector("[data-feuille-toile]");
  const onglets = [...vue.querySelectorAll("[role='tab']")];
  const panneau = vue.querySelector("[data-feuille-panneau]");
  const niveau = vue.querySelector("[data-zoom-niveau]");
  const consigne = vue.querySelector("[data-feuille-consigne]");
  const survolTexte = vue.querySelector("[data-feuille-survol-texte]");
  const legende = vue.querySelector("[data-feuille-legende]");

  const ZOOMS = [1, 1.5, 2, 3, 4, 6, 8];
  const PX_PAR_MM = 4; // résolution de la mise en page dessinée (onglet « Mire »)
  const hexa = (c) => /^#[0-9a-fA-F]{6}$/.test(c);
  const nombre = (v) => new Intl.NumberFormat(document.documentElement.lang, { maximumFractionDigits: 1 }).format(v);

  let onglet = "mire";
  let mire = null; // définition de la feuille (module `mesurer_feuille`), ou null
  let dessinMire = null; // toile hors écran : la mise en page en couleurs de consigne
  let calage = null; // { decalage, ecart_max, points, image } sans les pixels
  let photo = null; // toile hors écran : l'image de la feuille, en gris
  let derniere = false; // le calage est celui de la dernière feuille rangée, pas d'une feuille en cours
  let zoom = 0; // rang dans ZOOMS
  let centre = null; // centre de la vue, en 1/100 mm sur la feuille ; null : feuille entière
  let choisie = -1; // plage survolée ou choisie au clavier, rang dans l'ordre de la mire

  // ---- Plages de la mire : rang, nom et couleur de consigne ----
  function plages() {
    const liste = [];
    if (!mire) return liste;
    mire.zones.forEach((z, zone) => {
      for (let rangee = 0; rangee < z.rangees; rangee++) {
        for (let colonne = 0; colonne < z.colonnes; colonne++) liste.push({ zone, rangee, colonne, z });
      }
    });
    return liste;
  }
  let liste = [];

  // Colonne en lettres comme le logiciel du fabricant : A … Z, puis AA, AB…
  function lettres(n) {
    let texte = "";
    for (let k = n + 1; k > 0; k = Math.floor((k - 1) / 26)) texte = String.fromCharCode(65 + ((k - 1) % 26)) + texte;
    return texte;
  }
  const nomPlage = (p) => `${p.zone + 1}${lettres(p.colonne)}${p.rangee + 1}`;
  const rangDe = (zone, rangee, colonne) => liste.findIndex((p) => p.zone === zone && p.rangee === rangee && p.colonne === colonne);
  const couleur = (i) => (mire && mire.couleurs ? mire.couleurs[i] : null);

  // ---- Toiles hors écran ----
  function dessinerMire() {
    if (!mire || !mire.couleurs) return null;
    const l = Math.round((mire.largeur / 100) * PX_PAR_MM);
    const h = Math.round((mire.hauteur / 100) * PX_PAR_MM);
    const donnees = new ImageData(l, h);
    donnees.data.fill(255); // le papier : RVB 255 de la consigne
    const px = (centiemes) => Math.round((centiemes / 100) * PX_PAR_MM);
    liste.forEach((p, i) => {
      const [r, v, b] = mire.couleurs[i];
      // Bords arrondis chacun depuis la feuille : deux plages jointives ne laissent aucun jour.
      const [gauche, haut] = [p.z.gauche + p.z.pas_x * p.colonne, p.z.haut + p.z.pas_y * p.rangee];
      const [x0, y0] = [px(gauche), px(haut)];
      const x1 = Math.min(l, px(gauche + p.z.largeur_plage));
      const y1 = Math.min(h, px(haut + p.z.hauteur_plage));
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          const k = (y * l + x) * 4;
          donnees.data[k] = r;
          donnees.data[k + 1] = v;
          donnees.data[k + 2] = b;
        }
      }
    });
    const c = document.createElement("canvas");
    [c.width, c.height] = [l, h];
    c.getContext("2d").putImageData(donnees, 0, 0);
    return c;
  }

  // Image reçue en binaire : largeur, hauteur, ppp (u32 petit-boutiste), puis le gris.
  function lirePhoto(tampon) {
    if (!tampon || tampon.byteLength < 12) return null;
    const vueOctets = new DataView(tampon);
    const [l, h, ppp] = [0, 4, 8].map((k) => vueOctets.getUint32(k, true));
    const gris = new Uint8Array(tampon, 12);
    if (!l || !h || !ppp || gris.length !== l * h) return null;
    const donnees = new ImageData(l, h);
    for (let i = 0; i < gris.length; i++) {
      donnees.data[i * 4] = donnees.data[i * 4 + 1] = donnees.data[i * 4 + 2] = gris[i];
      donnees.data[i * 4 + 3] = 255;
    }
    const c = document.createElement("canvas");
    [c.width, c.height] = [l, h];
    c.getContext("2d").putImageData(donnees, 0, 0);
    return { toile: c, largeur: (l / ppp) * 2540, hauteur: (h / ppp) * 2540 };
  }

  // ---- Géométrie de la vue ----
  function etendue() {
    if (onglet === "points" && photo) return { l: photo.largeur, h: photo.hauteur };
    return mire ? { l: mire.largeur, h: mire.hauteur } : { l: 27940, h: 41660 };
  }
  function echelle() {
    const e = etendue();
    return Math.min(toile.clientWidth / e.l, toile.clientHeight / e.h) * ZOOMS[zoom];
  }
  function origine() {
    const e = etendue();
    const s = echelle();
    const c = centre || { x: e.l / 2, y: e.h / 2 };
    return { x: toile.clientWidth / 2 - c.x * s, y: toile.clientHeight / 2 - c.y * s, s };
  }
  const versFeuille = (cx, cy) => {
    const o = origine();
    return { x: (cx - o.x) / o.s, y: (cy - o.y) / o.s };
  };

  // ---- Dessin ----
  const jeton = (nom) => getComputedStyle(document.documentElement).getPropertyValue(nom).trim();

  function dessiner() {
    const ratio = window.devicePixelRatio || 1;
    const [l, h] = [Math.round(toile.clientWidth * ratio), Math.round(toile.clientHeight * ratio)];
    if (toile.width !== l || toile.height !== h) [toile.width, toile.height] = [l, h];
    const ctx = toile.getContext("2d");
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.fillStyle = jeton("--fond");
    ctx.fillRect(0, 0, toile.clientWidth, toile.clientHeight);
    const o = origine();
    const e = etendue();
    const source = onglet === "points" ? photo && photo.toile : dessinMire;
    ctx.imageSmoothingEnabled = ZOOMS[zoom] < 3;
    if (source) ctx.drawImage(source, o.x, o.y, e.l * o.s, e.h * o.s);
    if (onglet === "points" && calage) {
      const rayon = Math.max(2, 60 * o.s);
      ctx.fillStyle = jeton("--point-vise");
      ctx.strokeStyle = jeton("--point-vise-bord");
      ctx.lineWidth = 1;
      for (const p of calage.points) {
        ctx.beginPath();
        ctx.arc(o.x + p.x * o.s, o.y + p.y * o.s, rayon, 0, 2 * Math.PI);
        ctx.fill();
        ctx.stroke();
      }
    }
    dessinerChoix(ctx, o);
  }

  // Repère de la plage choisie : carré à l'encre bordé du fond, lisible sur clair comme sur sombre.
  function dessinerChoix(ctx, o) {
    const p = liste[choisie];
    if (!p) return;
    let x = o.x + (p.z.gauche + p.z.pas_x * p.colonne) * o.s;
    let y = o.y + (p.z.haut + p.z.pas_y * p.rangee) * o.s;
    const point = onglet === "points" && calage && calage.points.find((q) => q.zone === p.zone && q.rangee === p.rangee && q.colonne === p.colonne);
    if (point) {
      x = o.x + (point.x - p.z.largeur_plage / 2) * o.s;
      y = o.y + (point.y - p.z.hauteur_plage / 2) * o.s;
    }
    const [l, h] = [p.z.largeur_plage * o.s, p.z.hauteur_plage * o.s];
    ctx.lineWidth = 3;
    ctx.strokeStyle = jeton("--fond");
    ctx.strokeRect(x, y, l, h);
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = jeton("--encre");
    ctx.strokeRect(x, y, l, h);
  }

  // ---- Textes gardés à leur place : niveau, survol, légende ----
  function ecrire() {
    niveau.textContent = t("feuille_fd9.zoom.niveau").replace("{n}", () => String(Math.round(ZOOMS[zoom] * 100)));
    toile.setAttribute("aria-label", t(onglet === "points" ? "feuille_fd9.points.alt" : "feuille_fd9.mire.alt"));
    const p = liste[choisie];
    const c = p ? couleur(choisie) : null;
    if (p && c) {
      survolTexte.textContent = t("feuille_fd9.survol").replace("{nom}", () => nomPlage(p))
        .replace("{r}", () => String(c[0])).replace("{v}", () => String(c[1])).replace("{b}", () => String(c[2]));
      const valeur = "#" + c.map((o) => o.toString(16).padStart(2, "0")).join("");
      if (hexa(valeur)) consigne.style.setProperty("--c", valeur);
      consigne.setAttribute("aria-label", t("feuille_fd9.consigne").replace("{nom}", () => nomPlage(p)));
    } else {
      survolTexte.textContent = t("feuille_fd9.survol.vide");
      consigne.style.removeProperty("--c");
      consigne.removeAttribute("aria-label");
    }
    legende.textContent = texteLegende();
  }

  function texteLegende() {
    if (onglet === "mire") return t(mire ? "feuille_fd9.mire.legende" : "feuille_fd9.mire.absente");
    if (!calage) return t("feuille_fd9.points.vide");
    const mm = (v) => nombre(v / 100);
    const parties = [t("feuille_fd9.points.legende").replace("{dx}", () => mm(calage.decalage[0]))
      .replace("{dy}", () => mm(calage.decalage[1])).replace("{ecart}", () => mm(calage.ecart_max))];
    if (!photo) parties.push(t("feuille_fd9.points.sans_image"));
    if (derniere) parties.unshift(t("feuille_fd9.points.derniere"));
    return parties.join(" ");
  }

  function rafraichir() {
    ecrire();
    dessiner();
  }

  // ---- Onglets : clic, flèches, Début et Fin ----
  function choisirOnglet(nom, focus) {
    onglet = nom;
    for (const b of onglets) {
      const actif = b.dataset.onglet === nom;
      b.setAttribute("aria-selected", String(actif));
      b.tabIndex = actif ? 0 : -1;
      if (actif) {
        panneau.setAttribute("aria-labelledby", b.id);
        if (focus) b.focus();
      }
    }
    rafraichir();
  }
  for (const b of onglets) {
    b.addEventListener("click", () => choisirOnglet(b.dataset.onglet, false));
    b.addEventListener("keydown", (e) => {
      const i = onglets.indexOf(b);
      const cible = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: onglets.length - 1 }[e.key];
      if (cible === undefined) return;
      e.preventDefault();
      choisirOnglet(onglets[(cible + onglets.length) % onglets.length].dataset.onglet, true);
    });
  }

  // ---- Zoom, déplacement, survol, clavier ----
  function zoomer(rang, autour) {
    const avant = rang === zoom ? null : autour && versFeuille(autour.x, autour.y);
    zoom = Math.max(0, Math.min(ZOOMS.length - 1, rang));
    if (zoom === 0) centre = null;
    else if (avant) {
      // Le point sous le pointeur y reste.
      const s = echelle();
      centre = { x: avant.x - (autour.x - toile.clientWidth / 2) / s, y: avant.y - (autour.y - toile.clientHeight / 2) / s };
    }
    rafraichir();
  }
  vue.querySelector("[data-zoom='moins']").addEventListener("click", () => zoomer(zoom - 1));
  vue.querySelector("[data-zoom='plus']").addEventListener("click", () => zoomer(zoom + 1));
  vue.querySelector("[data-zoom='ajuster']").addEventListener("click", () => zoomer(0));
  toile.addEventListener("wheel", (e) => {
    if (!e.ctrlKey) return; // la roue seule fait défiler la page
    e.preventDefault();
    const r = toile.getBoundingClientRect();
    zoomer(zoom + (e.deltaY < 0 ? 1 : -1), { x: e.clientX - r.left, y: e.clientY - r.top });
  }, { passive: false });

  // Plage sous le pointeur : dans la grille de la mire, ou le point visé le plus proche.
  function plageSous(x, y) {
    const f = versFeuille(x, y);
    if (onglet === "points" && calage) {
      let [meilleur, distance] = [-1, Infinity];
      for (const p of calage.points) {
        const d = Math.hypot(p.x - f.x, p.y - f.y);
        if (d < distance) [meilleur, distance] = [rangDe(p.zone, p.rangee, p.colonne), d];
      }
      return distance <= 450 ? meilleur : -1;
    }
    return liste.findIndex((p) => {
      const x0 = p.z.gauche + p.z.pas_x * p.colonne;
      const y0 = p.z.haut + p.z.pas_y * p.rangee;
      return f.x >= x0 && f.x < x0 + p.z.pas_x && f.y >= y0 && f.y < y0 + p.z.pas_y;
    });
  }

  let glisse = null;
  toile.addEventListener("pointerdown", (e) => {
    if (zoom === 0) return;
    glisse = { x: e.clientX, y: e.clientY, centre: centre || { x: etendue().l / 2, y: etendue().h / 2 } };
    toile.setPointerCapture(e.pointerId);
  });
  toile.addEventListener("pointerup", () => { glisse = null; });
  toile.addEventListener("pointermove", (e) => {
    if (glisse) {
      const s = echelle();
      centre = { x: glisse.centre.x - (e.clientX - glisse.x) / s, y: glisse.centre.y - (e.clientY - glisse.y) / s };
      return dessiner();
    }
    const r = toile.getBoundingClientRect();
    const i = plageSous(e.clientX - r.left, e.clientY - r.top);
    if (i !== choisie) {
      choisie = i;
      rafraichir();
    }
  });
  toile.addEventListener("keydown", (e) => {
    if (e.key === "+" || e.key === "=") return zoomer(zoom + 1);
    if (e.key === "-") return zoomer(zoom - 1);
    if (e.key === "0") return zoomer(0);
    const p = liste[choisie] || liste[0];
    if (!p) return;
    const pas = { ArrowRight: [0, 1], ArrowLeft: [0, -1], ArrowDown: [1, 0], ArrowUp: [-1, 0] }[e.key];
    if (!pas) return;
    e.preventDefault();
    const rangee = Math.max(0, Math.min(p.z.rangees - 1, p.rangee + pas[0]));
    const colonne = Math.max(0, Math.min(p.z.colonnes - 1, p.colonne + pas[1]));
    choisie = liste[choisie] ? rangDe(p.zone, rangee, colonne) : 0;
    rafraichir();
  });

  // La toile garde sa taille ; seule la fenêtre redimensionnée change ce qu'elle dessine.
  new ResizeObserver(() => dessiner()).observe(toile);
  // Le thème change les jetons : la toile se redessine.
  new MutationObserver(() => dessiner()).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

  return {
    // La définition de la mire, lue une fois (onglet « Mire »).
    mire: (definition) => {
      if (JSON.stringify(definition) === JSON.stringify(mire)) return;
      mire = definition;
      liste = plages();
      dessinMire = dessinerMire();
      choisie = -1;
      rafraichir();
    },
    // Un calage et son image (ArrayBuffer), ou rien ; `ancien` : celui de la dernière feuille rangée.
    calage: (nouveau, image, ancien) => {
      calage = nouveau;
      photo = nouveau ? lirePhoto(image) : null;
      derniere = Boolean(nouveau && ancien);
      rafraichir();
    },
    montrer: (nom) => choisirOnglet(nom, false),
    ecrire: rafraichir,
  };
})();
