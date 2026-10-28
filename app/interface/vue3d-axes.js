// Axes et repères de la vue 3D (tickets #71 et #80) : axes L*, a*, b* gradués, et blanc du papier de
// chaque profil sur l'axe L*. Ils sont peints dans le canvas de vue3d.js, qui leur passe ses jetons et
// sa projection `ecran(s)` (L*, a*, b* vers pixels du canvas). Les chiffres passent par
// Graphiques.nombre : virgule décimale en français, « inconnu » pour une valeur absente.
"use strict";

window.Vue3dAxes = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const rvb = (c) => `rgb(${c.map((x) => Math.round(x)).join(",")})`;

  // Axes L*, a*, b*, gradués tous les 20, dans la couleur atténuée ; a* et b* au plancher L* = 0.
  function axes(ctx, jetons, ecran, avecTextes) {
    const liste = [
      { nom: "L*", de: [0, 0, 0], a: [100, 0, 0], pas: (v) => [v, 0, 0], debut: 0, fin: 100 },
      { nom: "a*", de: [0, -120, 0], a: [0, 120, 0], pas: (v) => [0, v, 0], debut: -100, fin: 100 },
      { nom: "b*", de: [0, 0, -120], a: [0, 0, 120], pas: (v) => [0, 0, v], debut: -100, fin: 100 },
    ];
    ctx.strokeStyle = jetons.attenue;
    ctx.fillStyle = jetons.attenue;
    ctx.lineWidth = 1;
    ctx.font = `${Math.round(jetons.texte)}px ${jetons.fonte}`;
    for (const axe of liste) {
      const de = ecran(axe.de);
      const a = ecran(axe.a);
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
        const q = ecran(axe.pas(v));
        ctx.fillRect(q[0] - 1.5, q[1] - 1.5, 3, 3);
        ctx.fillText(nombre(v, 0), q[0] + jetons.ecart, q[1] - jetons.ecart);
      }
    }
  }

  // Blanc du papier de chaque forme : un trait sur l'axe L* dans sa couleur de série, et son étiquette
  // à gauche de l'axe (les graduations sont à droite), sur le fond de la carte pour rester lisible
  // devant le gamut. Une étiquette trop proche de la précédente descend, reliée à son trait par un filet.
  function blancs(ctx, jetons, ecran, formes) {
    const ratio = window.devicePixelRatio || 1;
    const hauteur = jetons.texte + jetons.ecart * 2;
    let bas = -Infinity;
    const marques = formes.filter((f) => f.blanc !== null)
      .map((f) => ({ f, q: ecran([f.blanc, 0, 0]) }))
      .sort((u, v) => u.q[1] - v.q[1]);
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    for (const { f, q } of marques) {
      const couleur = rvb(jetons.series[f.serie - 1]);
      const texte = t("comparer.vue3d.blanc").replace("{l}", () => nombre(f.blanc, 1));
      const largeur = ctx.measureText(texte).width + jetons.ecart * 2;
      const droite = q[0] - jetons.marge * 2;
      bas = Math.max(q[1], bas + hauteur);
      ctx.strokeStyle = couleur;
      ctx.lineWidth = 2 * ratio;
      ctx.beginPath();
      ctx.moveTo(q[0] - jetons.marge, q[1]);
      ctx.lineTo(q[0] + jetons.marge, q[1]);
      ctx.stroke();
      if (bas !== q[1]) {
        ctx.lineWidth = ratio;
        ctx.beginPath();
        ctx.moveTo(q[0] - jetons.marge, q[1]);
        ctx.lineTo(droite, bas);
        ctx.stroke();
      }
      ctx.fillStyle = jetons.carte;
      ctx.fillRect(droite - largeur, bas - hauteur / 2, largeur, hauteur);
      ctx.lineWidth = ratio;
      ctx.strokeRect(droite - largeur, bas - hauteur / 2, largeur, hauteur);
      ctx.fillStyle = jetons.encre;
      ctx.fillText(texte, droite - jetons.ecart, bas);
    }
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
  }

  return { axes, blancs };
})();
