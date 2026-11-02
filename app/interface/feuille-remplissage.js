// Remplissage progressif de la mire pendant la mesure au FD-9 (ticket n° 103) : les plages pas encore
// mesurées sont voilées, puis reprennent leur couleur au fil du compte du FD-9, dans son ordre de
// lecture. Le FD-9 ne donne que le compte des plages mesurées, jamais lesquelles : l'ordre vient de
// ce qu'on a observé. Utilisé par feuille-vue.js, qui le dessine dans l'onglet « Mire ».
"use strict";

window.FeuilleRemplissage = (() => {
  const VOILE = 0.72; // opacité du fond posé sur une plage pas encore mesurée

  // Ordre de lecture : en serpentin, la première rangée de gauche à droite, la suivante de droite à
  // gauche, et ainsi de suite, zone après zone. Vu à l'essai E4, puis confirmé par la trace de
  // l'essai E5 (ordre des plages rendues par la DLL, RGB-2520 en une zone) ; supposé pour une autre
  // mire. Rend, pour chaque plage dans l'ordre de la mire (zone, rangée, colonne), son rang de lecture.
  function ordre(mire) {
    const rangs = [];
    if (!mire) return rangs;
    let [debut, n] = [0, 0];
    for (const z of mire.zones) {
      for (let rangee = 0; rangee < z.rangees; rangee++) {
        for (let k = 0; k < z.colonnes; k++) {
          const colonne = rangee % 2 === 0 ? k : z.colonnes - 1 - k;
          rangs[debut + rangee * z.colonnes + colonne] = n++;
        }
      }
      debut += z.rangees * z.colonnes;
    }
    return rangs;
  }

  // Voile des plages dont le rang de lecture atteint `mesurees` : un seul tracé, rempli une fois,
  // sans jour ni double voile entre deux plages jointives. `o` : origine et échelle de la vue.
  function voile(ctx, plages, rangs, mesurees, o, fond) {
    ctx.beginPath();
    plages.forEach((p, i) => {
      if (rangs[i] < mesurees) return;
      const x = o.x + (p.z.gauche + p.z.pas_x * p.colonne) * o.s;
      const y = o.y + (p.z.haut + p.z.pas_y * p.rangee) * o.s;
      ctx.rect(x, y, p.z.largeur_plage * o.s, p.z.hauteur_plage * o.s);
    });
    ctx.globalAlpha = VOILE;
    ctx.fillStyle = fond;
    ctx.fill();
    ctx.globalAlpha = 1;
  }

  return { ordre, voile };
})();
