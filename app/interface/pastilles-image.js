// Pastilles des couleurs hors gamut, sous l'image de la section « Image » (ticket #80). Chaque paire
// montre à gauche une couleur de l'image (couleur demandée) et à droite sa couleur imprimée
// (couleur reproduite) par le profil choisi pour la surimpression, avec leur écart en ΔE00 et la part
// de l'image. Les paires viennent de l'analyse (`gamut::Pastille` : une cellule du nuage, représentée
// par sa couleur RVB la plus fréquente) ; la page en montre au plus 12, celles au-delà du seuil
// courant, les plus présentes d'abord. Changer le seuil ou le profil ne relance aucune commande.
// Pipette : un clic sur l'image demande au module Rust (`image_pixel`) la couleur du point touché et
// sa couleur imprimée par chaque profil ; la paire s'affiche en tête de la section (et sous l'image
// agrandie) et suit le profil et le seuil sans nouveau clic. Avec la compensation du point noir, la
// pastille « Image » reste la couleur de l'image, « Imprimé » la couleur reproduite de la couleur
// visée, et la paire du point cliqué dit la couleur visée. Les couleurs à l'écran sont approchées :
// la page le dit.
"use strict";

window.PastillesImage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;
  const { texteRefus } = window.ComparerDetails;

  const AFFICHEES = 12;
  const code = (s) => Math.round(s * 10);
  const hexa = (c) => /^#[0-9a-fA-F]{6}$/.test(String(c));

  let analyse = null; // analyse calculée du profil de la surimpression, ou null
  let seuil = 2;
  let rang = 0; // rang de ce profil parmi les profils analysés
  let nom = ""; // nom de ce profil
  let avis = ""; // raison d'un profil inconnu, posée à la place des pastilles
  let pixel = null; // dernier point cliqué : PixelImage, avec la taille de l'image d'analyse
  let dernier = null; // { point, largeur, hauteur } du dernier clic, pour le redemander
  let demandeur = () => ({}); // sources et compensation de l'analyse affichée (comparer-image.js)
  let demande = 0; // la dernière demande de la pipette l'emporte
  let racine = null; // section affichée
  let zoneAgrandie = null; // le point cliqué, sous l'image agrandie
  let repere = null; // repère du point cliqué sur l'image affichée

  // L'analyse, le seuil ou le profil de la surimpression a changé (comparer-image.js).
  function changer(nouvelle, nouveauSeuil, nouveauRang, nouveauNom, nouvelAvis = "") {
    [analyse, seuil, rang, nom, avis] = [nouvelle, nouveauSeuil, nouveauRang, nouveauNom, nouvelAvis];
    ecrire();
  }

  // Une autre image : le point cliqué est oublié.
  function oublier() {
    demande++;
    [pixel, dernier] = [null, null];
    placer();
    ecrire();
  }

  // Une nouvelle analyse (autres profils, compensation changée) : le point cliqué est redemandé ;
  // l'ancienne paire reste en place jusqu'à la nouvelle, pour que rien ne bouge.
  function reprendre() {
    if (!dernier) return oublier();
    demanderPixel(dernier);
  }

  // ---- Pipette : le cadre de l'image, son repère et le clic ----
  // `demandeurPixel()` donne les sources et la compensation de l'analyse affichée. Le cadre garde
  // les proportions de l'image (--ratio) : sa hauteur suit la largeur, bornée par la mise en page.
  function cadre(canvas, demandeurPixel) {
    demandeur = demandeurPixel;
    const bloc = el("div", "pipette");
    bloc.style.setProperty("--ratio", String(canvas.width / Math.max(canvas.height, 1)));
    repere = el("span", "pipette__repere");
    repere.setAttribute("aria-hidden", "true");
    canvas.addEventListener("click", (e) => cliquer(e, canvas));
    bloc.append(canvas, repere);
    placer();
    return bloc;
  }

  // Le point cliqué, pour l'image agrandie (image-agrandie.js).
  function lecture() {
    zoneAgrandie = el("div", "agrandie__lecture");
    ecrire();
    return zoneAgrandie;
  }

  // Le repère se place au centre du pixel cliqué, en fraction de l'image affichée.
  function placer() {
    if (!repere) return;
    repere.hidden = !pixel;
    if (!pixel) return;
    repere.style.setProperty("--x", String((pixel.x + 0.5) / pixel.largeur));
    repere.style.setProperty("--y", String((pixel.y + 0.5) / pixel.hauteur));
  }

  // Coordonnées du clic dans la boîte de l'image (sans son filet) ; le module Rust les ramène au
  // pixel de l'image d'analyse, image centrée dans sa boîte comprise (object-fit: contain). En dehors
  // de l'image ou sur un pixel transparent : refus, rien d'affiché.
  function cliquer(e, canvas) {
    const point = { x: e.offsetX, y: e.offsetY, largeur: canvas.clientWidth, hauteur: canvas.clientHeight };
    demanderPixel({ point, largeur: canvas.width, hauteur: canvas.height });
  }

  async function demanderPixel(clic) {
    const n = ++demande;
    let r = null;
    try {
      r = await invoke("image_pixel", { ...demandeur(), point: clic.point });
    } catch {
      r = null;
    }
    if (n !== demande) return;
    [pixel, dernier] = [r && { ...r, largeur: clic.largeur, hauteur: clic.hauteur }, r ? clic : null];
    placer();
    ecrire();
  }

  // ---- Pastilles ----
  // Une pastille : la couleur écran, et pour les lecteurs d'écran son rôle et ses L*a*b* arrondis.
  function pastille(lab, couleur, role) {
    const s = el("span", "pastille-couleur");
    if (hexa(couleur)) s.style.setProperty("--c", couleur);
    s.setAttribute("role", "img");
    s.setAttribute("aria-label", t("pastilles.aria." + role).replace("{l}", () => nombre(lab[0], 0))
      .replace("{a}", () => nombre(lab[1], 0)).replace("{b}", () => nombre(lab[2], 0)));
    return s;
  }

  const lab = (v) => t("pastilles.lab").replace("{l}", () => nombre(v[0], 0))
    .replace("{a}", () => nombre(v[1], 0)).replace("{b}", () => nombre(v[2], 0));
  const ecart = (p) => t("pastilles.ecart").replace("{e}", () => nombre(p.ecart, 1));

  // Une paire : couleur de l'image à gauche, couleur imprimée à droite, puis les lignes données.
  function paire(p, lignes, classe = "") {
    const bloc = el("div", ("paire " + classe).trim());
    const couleurs = el("div", "paire__couleurs");
    couleurs.append(pastille(p.demandee, p.couleur_demandee, "image"),
      pastille(p.reproduite, p.couleur_reproduite, "imprimee"));
    bloc.append(couleurs);
    for (const ligne of lignes) bloc.append(el("span", "paire__texte", ligne));
    return bloc;
  }

  // Part de l'image dans une cellule, en %, sur les pixels dont l'écart est connu.
  function part(p) {
    const connus = analyse.pixels_comptes - analyse.pixels_inconnus;
    return connus ? (p.pixels / connus) * 100 : null;
  }

  // Les 12 emplacements sont toujours là (vides s'il y a moins de couleurs) ; la phrase « aucune », ou
  // la raison d'un profil inconnu, se pose par-dessus : la grille ne change jamais de taille.
  function grille() {
    const choisies = analyse ? (analyse.pastilles || []).filter((p) => p.code >= code(seuil)).slice(0, AFFICHEES) : [];
    const liste = el("div", "pastilles");
    for (let i = 0; i < AFFICHEES; i++) {
      const p = choisies[i];
      liste.append(p ? paire(p, [ecart(p), t("pastilles.part").replace("{p}", () => nombre(part(p), 1))]) : vide());
    }
    const message = el("p", "pastilles__message");
    if (analyse && !choisies.length) {
      message.textContent = t("pastilles.aucune").replace("{s}", () => nombre(seuil, 1)).replace("{profil}", () => nom);
    } else if (!analyse) message.textContent = avis;
    liste.append(message);
    return liste;
  }

  // Un emplacement vide : deux carrés à la couleur de la carte, sans texte.
  function vide() {
    const bloc = el("div", "paire paire--vide");
    bloc.setAttribute("aria-hidden", "true");
    const couleurs = el("div", "paire__couleurs");
    couleurs.append(el("span", "pastille-couleur"), el("span", "pastille-couleur"));
    bloc.append(couleurs);
    return bloc;
  }

  // ---- Le point cliqué, pour le profil de la surimpression ----
  // Bloc de hauteur fixe, présent même vide : « Cliquez dans l'image… » tant qu'aucun point n'est choisi.
  function pipette() {
    const etat = pixel ? pixel.profils[rang] : null;
    const bloc = el("div", "pastilles-pipette");
    bloc.append(el("span", "label", t("pastilles.pipette.titre")));
    if (!etat) {
      bloc.append(el("p", "why", t("pastilles.pipette.aide")));
      return bloc;
    }
    if (etat.etat !== "calcule") {
      bloc.append(el("p", "why", t("pastilles.pipette.inconnu").replace("{profil}", () => nom)
        .replace("{raison}", () => texteRefus(etat))));
      return bloc;
    }
    const p = etat.paire;
    const cle = p.code >= code(seuil) ? "pastilles.pipette.au_dela" : "pastilles.pipette.en_deca";
    // Sous chaque pastille, son nom (« Image », « Imprimé ») et ses L*a*b* arrondis.
    const noms = el("div", "paire__noms");
    for (const [role, v] of [["pastilles.image", p.demandee], ["pastilles.imprimee", p.reproduite]]) {
      const nomCouleur = el("span", "");
      nomCouleur.append(el("b", "", t(role)), el("span", "", lab(v)));
      noms.append(nomCouleur);
    }
    const resultat = paire(p, [ecart(p), t(cle).replace("{s}", () => nombre(seuil, 1))], "paire--pipette");
    resultat.insertBefore(noms, resultat.children[1] || null);
    bloc.append(resultat);
    if (p.visee) bloc.append(visee(p));
    return bloc;
  }

  // Avec la compensation du point noir : la couleur visée, dont l'écart à l'imprimé est mesuré, ou
  // une ligne qui dit qu'elle est celle de l'image.
  function visee(p) {
    if (!(p.ecart_visee >= 0.1)) return el("p", "why", t("pastilles.pipette.visee_identique"));
    const ligne = el("p", "paire__visee");
    ligne.append(pastille(p.visee, p.couleur_visee, "visee"), el("span", "", t("pastilles.pipette.visee")
      .replace("{lab}", () => lab(p.visee)).replace("{e}", () => nombre(p.ecart_visee, 1))));
    return ligne;
  }

  // ---- La section, sous l'image ----
  function section() {
    racine = el("div", "pastilles-section");
    ecrire();
    return racine;
  }

  function ecrire() {
    if (zoneAgrandie) zoneAgrandie.replaceChildren(pipette());
    if (!racine) return;
    const contenu = [el("span", "label", t("pastilles.titre")), pipette(),
      el("p", "pastilles-lecture", t("pastilles.lecture")), grille(), el("p", "why", t("pastilles.precaution"))];
    racine.replaceChildren(...contenu);
  }

  return { changer, oublier, reprendre, cadre, lecture, section };
})();
