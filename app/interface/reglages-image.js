// Réglages de la section « Image » de la vue « Comparer » (ticket #80), dans la colonne de droite :
// profil de la surimpression, seuil d'alerte, affichage et compensation du point noir. Chaque
// fonction rend un champ et appelle la fonction donnée quand l'opérateur change le réglage ;
// comparer-image.js garde les valeurs. Le seuil, le profil et l'affichage ne relancent aucune
// commande ; la compensation du point noir relance l'analyse (annulable).
"use strict";

window.ReglagesImage = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;

  const AFFICHAGES = ["image", "les_deux", "alerte"];

  // Profil de la surimpression, parmi les profils analysés. Seuls le calque et son avis changent :
  // le choix garde le focus du clavier.
  function profil(profils, choisi, choisir) {
    const champ = el("div", "field");
    const s = el("select", "input");
    s.id = "image-profil";
    for (const p of profils) {
      const o = el("option", "", p.nom);
      o.value = String(p.rang);
      o.selected = p.rang === choisi;
      s.append(o);
    }
    s.addEventListener("change", () => choisir(Number(s.value)));
    const l = Object.assign(el("label", "label", t("image.surimpression.profil")), { htmlFor: s.id });
    champ.append(l, s);
    return champ;
  }

  // Seuil d'alerte, de 0,5 à 10 par pas de 0,5 ; la valeur s'écrit dans le libellé.
  function seuil(valeur, changer) {
    const champ = el("div", "field");
    const curseur = el("input", "curseur");
    Object.assign(curseur, { type: "range", min: 0.5, max: 10, step: 0.5, value: valeur, id: "image-seuil" });
    const libelle = Object.assign(el("label", "label"), { htmlFor: curseur.id });
    const ecrire = (v) => { libelle.textContent = t("image.seuil.valeur").replace("{s}", () => nombre(v, 1)); };
    ecrire(valeur);
    curseur.addEventListener("input", () => { changer(Number(curseur.value)); ecrire(Number(curseur.value)); });
    champ.append(libelle, curseur);
    return champ;
  }

  // Affichage : l'image, l'image et l'alerte, l'alerte seule.
  function affichage(valeur, changer) {
    const champ = el("div", "field");
    const libelle = el("span", "label", t("image.affichage"));
    libelle.id = "image-affichage";
    const groupe = el("div", "seg");
    groupe.setAttribute("role", "group");
    groupe.setAttribute("aria-labelledby", libelle.id);
    for (const v of AFFICHAGES) {
      const b = el("button", "", t("image.affichage." + v));
      b.setAttribute("aria-pressed", String(v === valeur));
      b.addEventListener("click", () => {
        for (const autre of groupe.children) autre.setAttribute("aria-pressed", String(autre === b));
        changer(v);
      });
      groupe.append(b);
    }
    champ.append(libelle, groupe);
    return champ;
  }

  // Compensation du point noir, cochée par défaut ; la changer relance l'analyse.
  function pointNoir(coche, changer) {
    const champ = el("div", "field");
    const rang = el("label", "choix-liste__rang");
    const caseACocher = el("input");
    Object.assign(caseACocher, { type: "checkbox", checked: coche, id: "image-point-noir" });
    caseACocher.addEventListener("change", () => changer(caseACocher.checked));
    rang.append(caseACocher, t("image.point_noir"));
    champ.append(rang, el("p", "", t("image.point_noir.aide")));
    return champ;
  }

  return { profil, seuil, affichage, pointNoir };
})();
