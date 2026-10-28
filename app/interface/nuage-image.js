// Nuage de couleurs de l'image (ticket #80) : les couleurs de la photo ouverte dans la section
// « Image », pour le profil choisi pour la surimpression, montrées dans la vue 3D (vue3d.js) et dans
// la coupe (Graphiques.coupe). Chaque point est une cellule de 4 unités Lab, placée en colorimétrie
// absolue sur le blanc du papier de ce profil (crate `gamut`, `PointNuage`). Un point est en alerte
// selon la règle des pixels de la surimpression : son plus grand écart, en code d'écart (dixièmes de
// ΔE00 arrondis vers le bas, bornés à 254), atteint round(10 × seuil). Changer le seuil ne relance
// aucune commande. comparer-image.js donne l'analyse et le seuil, comparer.js redessine (suivre).
"use strict";

window.NuageImage = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;

  let analyse = null; // analyse calculée du profil de la surimpression, ou null
  let seuil = 2;
  let nom = ""; // nom de ce profil
  let montre = true; // bascule « Couleurs de l'image »
  let rappel = () => {};
  let bascule = null; // { caseACocher, aide } du champ affiché

  // L'image, son analyse, le profil ou le seuil a changé (comparer-image.js).
  function changer(nouvelle, nouveauSeuil, nouveauNom) {
    if (nouvelle && !analyse) montre = true; // une image vient d'être analysée : nuage montré
    analyse = nouvelle;
    seuil = nouveauSeuil;
    nom = nouveauNom;
    ecrireChamp();
    rappel();
  }

  // Points à dessiner : [{ lab, couleur, alerte }], vide si la bascule est décochée ou sans nuage.
  function points() {
    if (!montre || !analyse) return [];
    return (analyse.nuage || []).filter((p) => Array.isArray(p.lab) && p.lab.every(Number.isFinite)).map((p) => ({
      lab: p.lab,
      couleur: p.couleur,
      alerte: Number.isFinite(p.ecart_max) && Math.min(Math.floor(p.ecart_max * 10), 254) >= Math.round(seuil * 10),
    }));
  }

  // ---- Bascule « Couleurs de l'image », dans la section du gamut en volume ----
  function ecrireChamp() {
    if (!bascule) return;
    const { caseACocher, aide } = bascule;
    caseACocher.disabled = !analyse;
    caseACocher.checked = montre && Boolean(analyse);
    aide.textContent = analyse
      ? t("nuage.aide").replace("{profil}", () => nom).replace("{s}", () => nombre(seuil, 1))
      : t("nuage.indisponible");
    aide.className = analyse ? "" : "why";
  }

  function champ() {
    const bloc = el("div", "field");
    const rang = el("label", "choix-liste__rang");
    const caseACocher = el("input");
    caseACocher.type = "checkbox";
    caseACocher.addEventListener("change", () => {
      montre = caseACocher.checked;
      rappel();
    });
    rang.append(caseACocher, t("nuage.bascule"));
    const aide = el("p", "");
    bloc.append(rang, aide);
    bascule = { caseACocher, aide };
    ecrireChamp();
    return bloc;
  }

  // ---- Vue 3D ----
  const couleur = (p, jetons) => (/^#[0-9a-fA-F]{6}$/.test(String(p.couleur)) ? p.couleur : jetons.attenue);

  // Petits carrés de 3 px × devicePixelRatio, après les triangles, du plus loin au plus proche.
  // `projeter(lab)` rend [x, y, profondeur] dans le canvas. Le contour à l'encre du thème garde
  // lisible un point sombre sur fond sombre comme un point clair sur fond clair.
  function peindre(ctx, jetons, projeter, points) {
    if (!points.length) return;
    const ratio = window.devicePixelRatio || 1;
    const cote = 3 * ratio;
    const places = points.map((p) => ({ p, e: projeter(p.lab) })).sort((u, v) => v.e[2] - u.e[2]);
    ctx.globalAlpha = 1;
    ctx.lineWidth = 0.5 * ratio;
    ctx.strokeStyle = jetons.encre;
    for (const { p, e } of places) {
      ctx.fillStyle = p.alerte ? jetons.alerte : couleur(p, jetons);
      ctx.fillRect(e[0] - cote / 2, e[1] - cote / 2, cote, cote);
      ctx.strokeRect(e[0] - cote / 2, e[1] - cote / 2, cote, cote);
    }
  }

  // Phrase du texte de remplacement : nombre de points, dont en alerte ; vide sans nuage.
  function decrire(points) {
    if (!points.length) return "";
    return " " + t("nuage.aria").replace("{n}", () => nombre(points.length, 0))
      .replace("{a}", () => nombre(points.filter((p) => p.alerte).length, 0));
  }

  // comparer.js redessine la vue 3D et la coupe quand le nuage change.
  function suivre(f) {
    rappel = f;
  }

  return { changer, points, champ, peindre, decrire, suivre };
})();
