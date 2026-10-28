// Vue 3D des gamuts (ticket #71) : maison, en Canvas 2D, sans bibliothèque. Les sommets sont en
// L*, a*, b* ; la projection est orthographique, centrée sur L* 50. À chaque image, les triangles de
// toutes les formes sont triés par profondeur et peints du plus loin au plus proche (algorithme du
// peintre) : c'est ce tri qui évite les fausses facettes en transparence. Ombrage plat. Les couleurs
// de série sont des jetons (`--serie-1` à `--serie-4`), relus au changement de thème. Les chiffres
// passent par Graphiques.nombre : un chiffre inconnu s'écrit « inconnu ».
"use strict";

window.Vue3d = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);

  // Vue de départ : un peu de biais, un peu d'au-dessus.
  const DEPART = { lacet: -35, tangage: 20, zoom: 1 };
  const PAS_ANGLE = 10; // degrés par appui sur une flèche
  const PAS_ZOOM = 1.2;
  const OPACITE_TRANSPARENT = 0.35;
  // Lumière fixe dans le repère de la vue (vers l'observateur, un peu d'en haut à gauche).
  const LUMIERE = normer([-0.35, 0.55, -0.75]);
  const ETENDUE = 270; // unités L*a*b* visibles dans la plus petite dimension, au zoom 1
  const RAYON_BULLE = 12; // px : distance au sommet le plus proche pour la bulle

  function normer(v) {
    const n = Math.hypot(v[0], v[1], v[2]) || 1;
    return [v[0] / n, v[1] / n, v[2] / n];
  }

  function hexVersRvb(hex) {
    const h = String(hex).trim().replace("#", "");
    if (!/^[0-9a-fA-F]{6}$/.test(h)) return [128, 128, 128];
    return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  }

  // Jetons du thème courant : couleurs, fonte, et tailles en pixels du canvas (rem × corps de base).
  function lireJetons() {
    const style = getComputedStyle(document.documentElement);
    const jeton = (nom) => style.getPropertyValue(nom).trim();
    const base = parseFloat(style.fontSize) || 16;
    const taille = (nom) => (parseFloat(jeton(nom)) || 0) * (jeton(nom).endsWith("rem") ? base : 1) * (window.devicePixelRatio || 1);
    return {
      texte: taille("--text-xs"), ecart: taille("--space-1"), marge: taille("--space-2"), bulle: taille("--space-5"),
      series: [1, 2, 3, 4].map((n) => hexVersRvb(jeton(`--serie-${n}`))),
      attenue: jeton("--attenue"),
      encre: jeton("--encre"),
      carte: jeton("--carte"),
      filet: jeton("--filet"),
      fonte: jeton("--font-body"),
    };
  }

  function creer(canvas, options = {}) {
    let reglages = { couleur: "serie", rendu: "opaque", ...options };
    let vue = { ...DEPART };
    let jetons = lireJetons();
    let formes = []; // formes préparées : sommets, triangles, couleurs moyennes
    let projetes = []; // par forme : Float64Array (x, y, z) par sommet, dans le repère de la vue
    let image = 0; // demande d'image en attente
    let survol = null; // { x, y } de la souris, hors glisser
    let glisser = null; // { x, y } du dernier point pendant un glisser

    canvas.tabIndex = 0;
    canvas.setAttribute("role", "img");

    function preparer(f) {
      const sommets = f.sommets || [];
      const triangles = f.triangles || [];
      const n = triangles.length;
      const indices = new Uint32Array(n * 3);
      const reelles = new Float64Array(n * 3);
      const rvbSommets = (f.couleurs || []).map(hexVersRvb);
      let gardes = 0;
      for (const tri of triangles) {
        if (!tri.every((i) => Number.isInteger(i) && i >= 0 && i < sommets.length)) continue;
        indices.set(tri, gardes * 3);
        if (rvbSommets.length === sommets.length) {
          for (let c = 0; c < 3; c++) {
            reelles[gardes * 3 + c] = (rvbSommets[tri[0]][c] + rvbSommets[tri[1]][c] + rvbSommets[tri[2]][c]) / 3;
          }
        }
        gardes++;
      }
      return {
        nom: f.nom || "",
        volume: f.volume ?? null,
        serie: Math.min(Math.max(Number(f.serie) || 1, 1), 4),
        sommets,
        indices: indices.subarray(0, gardes * 3),
        reelles: rvbSommets.length === sommets.length ? reelles : null,
        nombre: gardes,
      };
    }

    function rotation() {
      const l = (vue.lacet * Math.PI) / 180;
      // Tangage positif : le haut (L* 100) vient vers l'observateur, on voit le dessus.
      const p = (-vue.tangage * Math.PI) / 180;
      return { cl: Math.cos(l), sl: Math.sin(l), cp: Math.cos(p), sp: Math.sin(p) };
    }

    // (a*, b*, L*) vers le repère de la vue : x à droite, y en haut, z qui s'éloigne.
    function tourner(s, r) {
      const x = s[1];
      const y = s[0] - 50;
      const z = s[2];
      const x1 = x * r.cl - z * r.sl;
      const z1 = x * r.sl + z * r.cl;
      return [x1, y * r.cp - z1 * r.sp, y * r.sp + z1 * r.cp];
    }

    function echelle() {
      return (vue.zoom * Math.min(canvas.width, canvas.height)) / ETENDUE;
    }

    function versEcran(p, k) {
      return [canvas.width / 2 + p[0] * k, canvas.height / 2 - p[1] * k];
    }

    function projeter(r) {
      projetes = formes.map((f) => {
        const sortie = new Float64Array(f.sommets.length * 3);
        f.sommets.forEach((s, i) => sortie.set(tourner(s, r), i * 3));
        return sortie;
      });
    }

    function dessinerTriangles(ctx, k) {
      const liste = [];
      formes.forEach((f, numero) => {
        for (let i = 0; i < f.nombre; i++) liste.push(numero, i);
      });
      const total = liste.length / 2;
      const profondeurs = new Float64Array(total);
      const ordre = new Uint32Array(total);
      for (let j = 0; j < total; j++) {
        const f = formes[liste[j * 2]];
        const p = projetes[liste[j * 2]];
        const i = liste[j * 2 + 1] * 3;
        profondeurs[j] = p[f.indices[i] * 3 + 2] + p[f.indices[i + 1] * 3 + 2] + p[f.indices[i + 2] * 3 + 2];
        ordre[j] = j;
      }
      // Du plus loin (z grand) au plus proche.
      ordre.sort((u, v) => profondeurs[v] - profondeurs[u]);

      const opaque = reglages.rendu !== "transparent";
      ctx.globalAlpha = opaque ? 1 : OPACITE_TRANSPARENT;
      ctx.lineWidth = 0.6;
      ctx.lineJoin = "round";
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      for (const j of ordre) {
        const f = formes[liste[j * 2]];
        const p = projetes[liste[j * 2]];
        const i = liste[j * 2 + 1];
        const a = f.indices[i * 3] * 3;
        const b = f.indices[i * 3 + 1] * 3;
        const c = f.indices[i * 3 + 2] * 3;
        const u = [p[b] - p[a], p[b + 1] - p[a + 1], p[b + 2] - p[a + 2]];
        const v = [p[c] - p[a], p[c + 1] - p[a + 1], p[c + 2] - p[a + 2]];
        const n = normer([u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]);
        // Plancher 0,8 : la face la plus sombre des séries 1 à 3 garde 3 : 1 au moins sur --carte.
        const lumiere = 0.8 + 0.2 * Math.abs(n[0] * LUMIERE[0] + n[1] * LUMIERE[1] + n[2] * LUMIERE[2]);
        const base =
          reglages.couleur === "reelle" && f.reelles
            ? [f.reelles[i * 3], f.reelles[i * 3 + 1], f.reelles[i * 3 + 2]]
            : jetons.series[f.serie - 1];
        const teinte = `rgb(${base.map((x) => Math.round(x * lumiere)).join(",")})`;
        ctx.beginPath();
        ctx.moveTo(cx + p[a] * k, cy - p[a + 1] * k);
        ctx.lineTo(cx + p[b] * k, cy - p[b + 1] * k);
        ctx.lineTo(cx + p[c] * k, cy - p[c + 1] * k);
        ctx.closePath();
        ctx.fillStyle = teinte;
        ctx.fill();
        // En opaque, un trait de la même couleur ferme les joints entre triangles voisins.
        if (opaque) {
          ctx.strokeStyle = teinte;
          ctx.stroke();
        }
      }
      ctx.globalAlpha = 1;
    }

    // Axes L*, a*, b*, gradués tous les 20, dans la couleur atténuée ; a* et b* au plancher L* = 0.
    function dessinerAxes(ctx, r, k, avecTextes) {
      const axes = [
        { nom: "L*", de: [0, 0, 0], a: [100, 0, 0], pas: (v) => [v, 0, 0], debut: 0, fin: 100 },
        { nom: "a*", de: [0, -120, 0], a: [0, 120, 0], pas: (v) => [0, v, 0], debut: -100, fin: 100 },
        { nom: "b*", de: [0, 0, -120], a: [0, 0, 120], pas: (v) => [0, 0, v], debut: -100, fin: 100 },
      ];
      ctx.strokeStyle = jetons.attenue;
      ctx.fillStyle = jetons.attenue;
      ctx.lineWidth = 1;
      ctx.font = `${Math.round(jetons.texte)}px ${jetons.fonte}`;
      for (const axe of axes) {
        const de = versEcran(tourner(axe.de, r), k);
        const a = versEcran(tourner(axe.a, r), k);
        if (!avecTextes) {
          ctx.beginPath();
          ctx.moveTo(de[0], de[1]);
          ctx.lineTo(a[0], a[1]);
          ctx.stroke();
          continue;
        }
        ctx.fillText(axe.nom, a[0] + jetons.ecart, a[1]);
        for (let v = axe.debut; v <= axe.fin; v += 20) {
          if (v === 0 && axe.nom !== "L*") continue;
          const q = versEcran(tourner(axe.pas(v), r), k);
          ctx.fillRect(q[0] - 1.5, q[1] - 1.5, 3, 3);
          ctx.fillText(nombre(v, 0), q[0] + jetons.ecart, q[1] - jetons.ecart);
        }
      }
    }

    // Sommet le plus proche de la souris, toutes formes confondues.
    function plusProche(k) {
      if (!survol) return null;
      let meilleur = null;
      formes.forEach((f, numero) => {
        const p = projetes[numero];
        for (let i = 0; i < f.sommets.length; i++) {
          const e = versEcran([p[i * 3], p[i * 3 + 1]], k);
          const d = Math.hypot(e[0] - survol.x, e[1] - survol.y);
          if (d <= RAYON_BULLE * (window.devicePixelRatio || 1) && (!meilleur || d < meilleur.d)) {
            meilleur = { d, e, nom: f.nom, s: f.sommets[i] };
          }
        }
      });
      return meilleur;
    }

    function dessinerBulle(ctx, k) {
      const proche = plusProche(k);
      if (!proche) return;
      const texte = t("comparer.vue3d.bulle")
        .replace("{nom}", () => proche.nom)
        .replace("{l}", () => nombre(proche.s[0], 1))
        .replace("{a}", () => nombre(proche.s[1], 1))
        .replace("{b}", () => nombre(proche.s[2], 1));
      const { marge, bulle: hauteur } = jetons;
      const largeur = ctx.measureText(texte).width + marge * 2;
      const x = Math.min(proche.e[0] + marge, canvas.width - largeur);
      const y = Math.max(proche.e[1] - hauteur - marge, 0);
      ctx.fillStyle = jetons.carte;
      ctx.strokeStyle = jetons.filet;
      ctx.fillRect(x, y, largeur, hauteur);
      ctx.strokeRect(x, y, largeur, hauteur);
      ctx.fillStyle = jetons.encre;
      ctx.textBaseline = "middle";
      ctx.fillText(texte, x + marge, y + hauteur / 2);
      ctx.textBaseline = "alphabetic";
      ctx.strokeStyle = jetons.encre;
      ctx.strokeRect(proche.e[0] - 3, proche.e[1] - 3, 6, 6);
    }

    function dessiner() {
      image = 0;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const r = rotation();
      const k = echelle();
      projeter(r);
      dessinerAxes(ctx, r, k, false);
      dessinerTriangles(ctx, k);
      dessinerAxes(ctx, r, k, true);
      dessinerBulle(ctx, k);
    }

    function redessiner() {
      if (!image) image = requestAnimationFrame(dessiner);
    }

    // Texte de remplacement : les profils affichés et leurs volumes.
    function decrire() {
      if (formes.length === 0) {
        canvas.setAttribute("aria-label", t("comparer.vue3d.aria.vide"));
        return;
      }
      const profils = formes
        .map((f) => t("comparer.vue3d.aria.profil").replace("{nom}", () => f.nom).replace("{volume}", () => nombre(f.volume, 0)))
        .join(" ; ");
      canvas.setAttribute("aria-label", t("comparer.vue3d.aria").replace("{profils}", () => profils));
    }

    // ---- Taille, thème, langue ----
    function ajusterTaille() {
      const ratio = window.devicePixelRatio || 1;
      const largeur = Math.max(1, Math.round(canvas.clientWidth * ratio));
      const hauteur = Math.max(1, Math.round(canvas.clientHeight * ratio));
      if (canvas.width !== largeur || canvas.height !== hauteur) {
        canvas.width = largeur;
        canvas.height = hauteur;
      }
      redessiner();
    }
    const taille = new ResizeObserver(ajusterTaille);
    taille.observe(canvas);
    const theme = new MutationObserver(() => {
      jetons = lireJetons();
      redessiner();
    });
    theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    const langue = () => {
      decrire();
      redessiner();
    };
    document.addEventListener("langue-appliquee", langue);

    // ---- Souris et clavier ----
    function position(evenement) {
      const cadre = canvas.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      return { x: (evenement.clientX - cadre.left) * ratio, y: (evenement.clientY - cadre.top) * ratio };
    }
    function tourne(dLacet, dTangage) {
      vue.lacet = (vue.lacet + dLacet) % 360;
      vue.tangage = Math.min(90, Math.max(-90, vue.tangage + dTangage));
      redessiner();
    }
    function zoome(facteur) {
      vue.zoom = Math.min(8, Math.max(0.25, vue.zoom * facteur));
      redessiner();
    }
    const ecouteurs = {
      pointerdown: (e) => {
        glisser = position(e);
        survol = null;
        canvas.setPointerCapture(e.pointerId);
      },
      pointermove: (e) => {
        const p = position(e);
        if (glisser) {
          const ratio = window.devicePixelRatio || 1;
          tourne(((p.x - glisser.x) / ratio) * 0.5, ((p.y - glisser.y) / ratio) * 0.5);
          glisser = p;
        } else {
          survol = p;
          redessiner();
        }
      },
      pointerup: (e) => {
        glisser = null;
        if (canvas.hasPointerCapture(e.pointerId)) canvas.releasePointerCapture(e.pointerId);
      },
      pointerleave: () => {
        survol = null;
        redessiner();
      },
      wheel: (e) => {
        e.preventDefault();
        zoome(e.deltaY < 0 ? PAS_ZOOM : 1 / PAS_ZOOM);
      },
    };
    for (const [nom, f] of Object.entries(ecouteurs)) {
      canvas.addEventListener(nom, f, nom === "wheel" ? { passive: false } : undefined);
    }
    // Clavier : flèches pour tourner, + et − pour zoomer, 0 pour revenir à la vue de départ.
    function clavier(e) {
      switch (e.key) {
        case "ArrowLeft": tourne(-PAS_ANGLE, 0); break;
        case "ArrowRight": tourne(PAS_ANGLE, 0); break;
        case "ArrowUp": tourne(0, -PAS_ANGLE); break;
        case "ArrowDown": tourne(0, PAS_ANGLE); break;
        case "+": case "=": zoome(PAS_ZOOM); break;
        case "-": case "−": zoome(1 / PAS_ZOOM); break;
        case "0": vue = { ...DEPART }; redessiner(); break;
        default: return;
      }
      e.preventDefault();
    }
    canvas.addEventListener("keydown", clavier);
    decrire();

    return {
      afficher(nouvelles) {
        formes = (nouvelles || []).map(preparer);
        decrire();
        redessiner();
      },
      regler(nouveaux) {
        reglages = { ...reglages, ...nouveaux };
        redessiner();
      },
      detruire() {
        if (image) cancelAnimationFrame(image);
        image = 0;
        taille.disconnect();
        theme.disconnect();
        document.removeEventListener("langue-appliquee", langue);
        for (const [nom, f] of Object.entries(ecouteurs)) canvas.removeEventListener(nom, f);
        canvas.removeEventListener("keydown", clavier);
        formes = [];
        projetes = [];
      },
    };
  }

  return { creer };
})();
