// Courbe de spectre (ticket #10), vue par ses données : ce que le module de la page
// trace, sans navigateur. Lancé par `cargo test -p app --test page` (node --test).
"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const Courbe = require("../../interface/spectre.js");

// Spectres comme la commande Rust les rend : 380, 390, 400 nm ; M0 et M1 présents,
// M2 absent de la mesure.
function mesure(m0, m1, statut = "confirmee") {
  return {
    longueurs_onde: { statut, valeur: [380, 390, 400] },
    plages: [{
      identifiant: "1",
      conditions: [{ valeurs: m0, supposee: false }, { valeurs: m1, supposee: true }, null],
    }],
  };
}

const cadre = { largeur: 300, hauteur: 200, gauche: 40, droite: 10, haut: 10, bas: 30 };

test("chaque condition présente devient une courbe, en pourcentage de réflectance", () => {
  const t = Courbe.preparer([{ donnees: mesure([0.25, 0.5, 0.75], [0.3, 0.6, 0.9]), plage: 0 }], cadre);
  assert.equal(t.courbes.length, 2);
  const m0 = t.courbes.find((c) => c.condition === 0);
  // x : 380 nm au bord gauche, 400 nm au bord droit ; y : 0 % en bas, 100 % en haut.
  assert.deepEqual(m0.points, [[40, 130], [165, 90], [290, 50]]);
  assert.equal(m0.d, "M40 130L165 90L290 50");
  assert.equal(t.courbes.find((c) => c.condition === 1).supposee, true);
});

test("une condition absente est « non disponible », jamais tracée à zéro", () => {
  const t = Courbe.preparer([{ donnees: mesure([0.25, 0.5, 0.75], [0.3, 0.6, 0.9]), plage: 0 }], cadre);
  assert.ok(t.courbes.every((c) => c.condition !== 2));
  assert.deepEqual(t.nonDisponibles, [{ serie: 0, condition: 2 }]);
  const lu = Courbe.lire(t, 390);
  assert.equal(lu.nm, 390);
  assert.deepEqual(lu.valeurs, [[50, 60, null]]);
});

test("au-delà de 100 % (papier azuré), l'axe monte au pas suivant", () => {
  const t = Courbe.preparer([{ donnees: mesure([0.9, 1.08, 0.95], [0.9, 0.9, 0.9]), plage: 0 }], cadre);
  assert.equal(t.yMax, 120);
  assert.deepEqual(t.axeY.map((g) => g.valeur), [0, 20, 40, 60, 80, 100, 120]);
  assert.deepEqual(t.axeX.map((g) => g.nm), [380, 400]);
});

test("plusieurs mesures se superposent, chacune avec son numéro", () => {
  const a = { donnees: mesure([0.2, 0.2, 0.2], [0.3, 0.3, 0.3]), plage: 0 };
  const b = { donnees: mesure([0.6, 0.6, 0.6], [0.7, 0.7, 0.7]), plage: 0 };
  const t = Courbe.preparer([a, b], cadre);
  assert.deepEqual(t.courbes.map((c) => [c.serie, c.condition]), [[0, 0], [0, 1], [1, 0], [1, 1]]);
  assert.deepEqual(Courbe.lire(t, 384).valeurs, [[20, 30, null], [60, 70, null]]);
  assert.equal(Courbe.lire(t, 384).nm, 380, "la longueur d'onde mesurée la plus proche");
});

test("sans longueurs d'onde connues, rien n'est placé au hasard", () => {
  const t = Courbe.preparer([{ donnees: mesure([0.2, 0.2, 0.2], [0.3, 0.3, 0.3], "inconnue"), plage: 0 }], cadre);
  assert.deepEqual(t.courbes, []);
  assert.deepEqual(t.sansLongueurs, [0]);
});

test("une plage choisie trace ses propres spectres", () => {
  const d = mesure([0.1, 0.1, 0.1], [0.1, 0.1, 0.1]);
  d.plages.push({ identifiant: "2", conditions: [null, null, { valeurs: [0.5, 0.5, 0.5], supposee: false }] });
  const t = Courbe.preparer([{ donnees: d, plage: 1 }], cadre);
  assert.deepEqual(t.courbes.map((c) => c.condition), [2]);
  assert.deepEqual(t.nonDisponibles.map((n) => n.condition), [0, 1]);
});

test("la position du pointeur donne la longueur d'onde lue", () => {
  const t = Courbe.preparer([{ donnees: mesure([0.2, 0.2, 0.2], [0.3, 0.3, 0.3]), plage: 0 }], cadre);
  assert.equal(Courbe.nmSous(t, 160), 390);
  assert.equal(Courbe.nmSous(t, 0), 380);
  assert.equal(Courbe.nmSous(t, 999), 400);
});
