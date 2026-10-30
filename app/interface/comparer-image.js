// Section « Image » de la vue « Comparer » (ticket #80) : une photo de client ouverte pour la
// séance (jamais rangée ni copiée), analysée face à chaque profil affiché par le module Rust
// `comparer::image`. Les pixels de l'image d'analyse et les codes d'écart arrivent en binaire et
// sont posés par putImageData. Un code d'écart est un dixième de ΔE00 (0 à 254), 255 = inconnu ou
// transparent : un pixel est au-delà du seuil si son code vaut au moins round(10 × seuil). Changer
// le seuil, le profil de la surimpression ou l'affichage ne relance aucune commande. Les pixels à
// l'écart inconnu sont dits à part, jamais comptés comme bons. Changer la compensation du point
// noir relance l'analyse sur place, sans redessiner la section. Deux colonnes : l'image à gauche,
// à droite les réglages (reglages-image.js), le point cliqué et les pastilles (pastilles-image.js)
// et les chiffres ; « Agrandir » montre l'image en grand (image-agrandie.js) ; la description de
// l'image vient de description-image.js. Les textes viennent du catalogue.
"use strict";

window.ComparerImage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;
  const { carreSerie, tableau } = window.ComparerEcarts;
  const { section, texteRefus } = window.ComparerDetails;

  const INCONNU = 255;
  // Classes d'écart du mainteneur, en codes d'écart : [nom, début inclus, fin exclue].
  const CLASSES = [["leger", 20, 50], ["moyen", 50, 100], ["fort", 100, 255]];
  // Lignes de texte réservées à chaque ligne des chiffres : part, trois classes, teintes, inconnus,
  // couleurs distinctes, raison.
  const LIGNES = [3, 2, 2, 2, 3, 3, 2, 3];
  // Familles de teinte, dans l'ordre des lignes de `par_teinte` (gamut::FAMILLES_TEINTE).
  const FAMILLES = ["rouge", "jaune", "vert", "cyan", "bleu", "magenta", "gris"];
  const code = (s) => Math.round(s * 10);

  // La section reste la même d'un dessin de la vue à l'autre ; seul son contenu change.
  const racine = el("div");
  let resume = null; // ImageResume de l'image ouverte
  let apercu = null; // pixels RVBA de l'image d'analyse
  let analyses = null; // [{ source, etat }] de la dernière analyse, dans l'ordre des profils
  let ecarts = []; // codes d'écart par rang de profil (null pour un profil inconnu)
  let sources = [];
  let profils = []; // profils analysés de la comparaison : { rang, serie, nom }
  let enCours = false;
  let message = "";
  let demande = 0; // la dernière demande l'emporte
  let seuil = 2;
  let choisi = 0; // rang du profil de la surimpression
  let affichage = "les_deux";
  let pointNoir = true; // compensation du point noir, cochée par défaut
  let compense = true; // réglage de l'analyse affichée (pointNoir peut l'avoir devancé)
  let etatCalcul = null; // « Analyse en cours… » du panneau, pour une analyse relancée sur place
  let rendu = null; // ligne qui rappelle le réglage au-dessus des chiffres
  let canvas = null;
  let legende = null;
  let tableauChiffres = null;
  let imageCalque = null; // ImageData du calque, gardée d'un mouvement du curseur à l'autre

  // Une rangée d'actions : un bouton par [clé, action].
  function actions(...boutons) {
    const rangee = el("div", "actions");
    for (const [cle, action] of boutons) {
      const b = el("button", "btn", t(cle));
      b.addEventListener("click", action);
      rangee.append(b);
    }
    return rangee;
  }

  const statut = (texte) => {
    const p = el("p", "why", texte);
    p.setAttribute("role", "status");
    return p;
  };

  // Un jeton de couleur du thème courant, en [r, v, b] ; null s'il ne se lit pas.
  function jeton(nom) {
    const h = getComputedStyle(document.documentElement).getPropertyValue(nom).trim().replace("#", "");
    return /^[0-9a-fA-F]{6}$/.test(h) ? [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) : null;
  }

  // ---- Ouvrir, analyser, annuler, fermer ----
  async function ouvrir() {
    message = "";
    try {
      const r = await invoke("image_ouvrir", { filtre: t("image.filtre") });
      if (!r) return; // annulé : rien ne change
      window.Accordeon.ouvrir("image.titre");
      demande++;
      [resume, apercu, analyses, ecarts] = [r, null, null, []];
      window.NuageImage.oublier(); // nouvelle image : nuage libéré, case recochée
      window.PastillesImage.oublier(); // et le point cliqué de l'ancienne image oublié
      apercu = new Uint8ClampedArray(await invoke("image_apercu"));
    } catch (cle) {
      message = t(String(cle));
      dessiner();
      return;
    }
    await analyser();
  }

  async function traiterCommeSrvb() {
    message = "";
    try {
      resume = await invoke("image_traiter_comme_srvb");
    } catch (cle) {
      message = t(String(cle));
      dessiner();
      return;
    }
    await analyser();
  }

  async function analyser() {
    message = ""; // un ancien « annulée » ou une ancienne erreur ne reste pas au-dessus de chiffres justes
    const n = ++demande;
    [analyses, ecarts] = [null, []];
    window.PastillesImage.reprendre(); // le point cliqué est redemandé avec les nouveaux réglages
    if (!resume || resume.profil.etat === "refuse" || !sources.length) {
      enCours = false;
      dessiner();
      return;
    }
    enCours = true;
    dessiner();
    try {
      const reponse = await invoke("image_analyser", { sources, compensation: pointNoir });
      if (n !== demande) return; // périmée : la demande plus récente tient « en cours »
      const octets = await lireEcarts(reponse);
      if (n !== demande) return;
      [analyses, ecarts, compense] = [reponse.profils, octets, pointNoir];
    } catch (cle) {
      if (n === demande) message = t(String(cle));
    }
    if (n !== demande) return;
    enCours = false;
    dessiner();
  }

  // Codes d'écart d'une analyse, en binaire, profil par profil (null pour un profil inconnu).
  const lireEcarts = (reponse) => Promise.all(reponse.profils.map((a, index) => (a.etat.etat === "calculee"
    ? invoke("image_ecarts", { analyse: reponse.numero, index }).then((b) => new Uint8Array(b)) : null)));

  // La compensation du point noir relance l'analyse sans redessiner la section : mise en page, image,
  // défilement et focus restent en place ; les anciennes valeurs, marquées, restent visibles jusqu'aux
  // nouvelles, qui les remplacent sur place. Sans valeurs affichées, c'est l'analyse ordinaire.
  async function recalculer() {
    if (!analyses || !etatCalcul) return analyser();
    const n = ++demande;
    message = "";
    etatCalcul.etat("calcul");
    window.VerdictImage.perime(legende, true);
    try {
      const reponse = await invoke("image_analyser", { sources, compensation: pointNoir });
      const octets = n === demande ? await lireEcarts(reponse) : null;
      if (n !== demande) return;
      [analyses, ecarts, compense] = [reponse.profils, octets, pointNoir];
    } catch (cle) {
      if (n !== demande) return;
      // Annulée ou en erreur : les valeurs s'effacent sur place, la raison et « Relancer » s'affichent.
      [analyses, ecarts, message] = [null, [], t(String(cle))];
    }
    etatCalcul.etat(analyses ? "repos" : "relancer", message);
    window.VerdictImage.perime(legende, false);
    if (rendu) rendu.textContent = t(compense ? "image.rendu.point_noir" : "image.rendu.relatif");
    choisirProfil(choisi); // calque, légende, nuage et pastilles, sur place
    dessinerChiffres();
    if (analyses) window.PastillesImage.reprendre();
  }

  // Une erreur des commandes est une clé du catalogue.
  const erreur = (cle) => { message = t(String(cle)); dessiner(); };
  const annuler = () => invoke("image_annuler").catch(erreur);

  function oublier() {
    demande++;
    resume = apercu = analyses = null;
    [ecarts, enCours, message] = [[], false, ""];
  }

  const retirer = () => { oublier(); dessiner(); invoke("image_fermer").catch(erreur); };

  // « Fermer la comparaison » ferme aussi l'image, et le seuil revient à 2,0.
  function fermer() {
    const ouverte = resume !== null;
    oublier();
    [sources, profils, seuil, affichage, pointNoir, compense] = [[], [], 2, "les_deux", true, true];
    racine.replaceChildren();
    imageCalque = etatCalcul = rendu = canvas = legende = tableauChiffres = null;
    window.NuageImage.oublier();
    if (ouverte) invoke("image_fermer").catch(() => {});
  }

  // Les profils de la comparaison ont changé : l'image est analysée de nouveau face à eux.
  function profilsChanges(nouvelles) {
    sources = nouvelles;
    if (resume) analyser();
  }

  // ---- Chiffres : tout se recalcule depuis l'histogramme, sans commande ----
  const somme = (h, de, a_) => h.slice(de, a_).reduce((s, v) => s + v, 0);

  // Part de l'image, en %, des pixels dont le code va de `de` (inclus) à `a_` (exclu), sur les
  // pixels dont l'écart est connu. Sans aucun pixel connu, la part est inconnue (null).
  function part(a, de, a_, h = a.histogramme) {
    const connus = a.pixels_comptes - a.pixels_inconnus;
    if (!connus) return null;
    return (somme(h, de, a_) / connus) * 100;
  }

  const pourcent = (p) => t("image.valeur.pourcent").replace("{p}", () => nombre(p, 1));
  const analyseDe = (rang) => (analyses?.[rang]?.etat.etat === "calculee" ? analyses[rang].etat.analyse : null);

  function teintes(a) {
    const k = code(seuil);
    const touchees = FAMILLES.map((f, s) => [f, part(a, k, 255, a.par_teinte[s] || [])])
      .filter(([, p]) => p > 0)
      .sort((x, y) => y[1] - x[1])
      .slice(0, 3);
    if (!touchees.length) return t("image.teintes.aucune");
    const liste = el("div");
    for (const [f, p] of touchees) {
      liste.append(el("div", "", t("image.teinte.valeur").replace("{nom}", () => t("image.teinte." + f)).replace("{p}", () => nombre(p, 1))));
    }
    return liste;
  }

  function inconnus(a) {
    if (!a.pixels_inconnus) return t("image.inconnus.aucun");
    return t("image.inconnus.valeur").replace("{n}", () => nombre(a.pixels_inconnus, 0))
      .replace("{p}", () => nombre((a.pixels_inconnus / a.pixels_comptes) * 100, 1));
  }

  // Une cellule des chiffres, haute de `n` lignes quelle que soit sa valeur ; le texte entier reste
  // lisible dans son titre.
  function cellule(v, n) {
    const c = el("div", "chiffres-image__cellule");
    c.style.setProperty("--lignes", String(n));
    c.append(v instanceof Node ? v : String(v));
    c.title = c.textContent;
    return c;
  }

  function dessinerChiffres() {
    if (!tableauChiffres) return;
    const k = code(seuil);
    const colonne = (p) => {
      const a = analyseDe(p.rang);
      if (!a) return [...Array(7)].map(() => t("comparer.inconnu"));
      return [pourcent(part(a, k, 255)), ...CLASSES.map(([, de, a_]) => pourcent(part(a, de, a_))),
        teintes(a), inconnus(a), nombre(a.couleurs_distinctes, 0)];
    };
    const colonnes = profils.map(colonne);
    const cles = ["image.chiffres.part", ...CLASSES.map(([nom]) => "image.classe." + nom),
      "image.chiffres.teintes", "image.chiffres.inconnus", "image.chiffres.couleurs"];
    // Un profil inconnu dit pourquoi, dans sa colonne ; la ligne est toujours là.
    const raisons = profils.map((p) => (analyses?.[p.rang]?.etat.etat === "inconnue" ? texteRefus(analyses[p.rang].etat) : ""));
    // Chaque ligne a sa hauteur fixe (LIGNES) : aucune valeur ne change la taille du tableau.
    const lignes = [...cles, "image.chiffres.raison"].map((cle, i) => [t(cle).replace("{s}", () => nombre(seuil, 1)),
      ...profils.map((_, j) => (i < cles.length ? colonnes[j][i] : raisons[j]))].map((v) => cellule(v, LIGNES[i])));
    const entetes = profils.map((p) => {
      const nom = el("span", "");
      nom.append(carreSerie(p.serie), " ", p.nom);
      return nom;
    });
    tableauChiffres.replaceChildren(tableau(["", ...entetes], lignes, false));
  }

  // ---- Surimpression : l'image, puis chaque pixel au-delà du seuil en vert fluo ----
  function calque() {
    if (!canvas || !apercu || !resume) return;
    const ctx = canvas.getContext("2d");
    const [w, h] = [resume.largeur, resume.hauteur];
    if (!imageCalque || imageCalque.width !== w || imageCalque.height !== h) imageCalque = ctx.createImageData(w, h);
    const d = imageCalque.data;
    const k = code(seuil);
    // Vert fluo sur la photo dans les deux thèmes ; la légende garde --couleur-hors-gamut.
    const alerte = jeton("--couleur-hors-gamut-calque");
    const fond = jeton("--couleur-hors-gamut-fond");
    let octets = affichage === "image" ? null : ecarts[choisi];
    let seule = affichage === "alerte";
    if (!alerte || !fond) [octets, seule] = [null, false]; // jeton illisible : l'image seule, sans marque
    if (!seule) d.set(apercu.subarray(0, d.length));
    const n = w * h;
    for (let i = 0; i < n; i++) {
      const e = octets ? octets[i] : INCONNU;
      const c = e !== INCONNU && e >= k ? alerte : seule ? fond : null;
      if (!c) continue;
      d.set(c, 4 * i);
      d[4 * i + 3] = 255;
    }
    ctx.putImageData(imageCalque, 0, 0);
    const nom = (profils.find((x) => x.rang === choisi) || {}).nom || "";
    const a = analyseDe(choisi);
    canvas.setAttribute("aria-label", t("image.canvas.aria").replace("{p}", () => nombre(a ? part(a, k, 255) : null, 1))
      .replace("{s}", () => nombre(seuil, 1)).replace("{profil}", () => nom));
    window.VerdictImage.ecrire(legende, a, seuil, nom, compense); // part imprimable, seuil, profil, rendu
  }

  // Le nuage (nuage-image.js) et les pastilles sous l'image (pastilles-image.js) suivent la surimpression.
  function nuage() {
    const nom = (profils.find((x) => x.rang === choisi) || {}).nom || "";
    const a = analyses?.[choisi];
    const avis = a?.etat.etat === "inconnue" ? t("image.surimpression.inconnue").replace("{raison}", () => texteRefus(a.etat)) : "";
    window.NuageImage.changer(analyseDe(choisi), seuil, nom);
    window.PastillesImage.changer(analyseDe(choisi), seuil, choisi, nom, avis);
  }

  function changerSeuil(valeur) {
    seuil = valeur;
    calque();
    dessinerChiffres();
    nuage();
  }

  // ---- Réglages (reglages-image.js) ----
  // La compensation du point noir change le calcul : l'analyse est relancée, sur place.
  function changerPointNoir(coche) {
    pointNoir = coche;
    recalculer();
  }

  // Un profil inconnu le dit à la place des pastilles (nuage()), sans rien insérer.
  function choisirProfil(rang) {
    choisi = rang;
    calque();
    nuage();
  }

  // ---- Dessin de la section ----
  // Deux colonnes : à gauche l'image (légende, « Agrandir », surimpression) ; à droite, collé en haut
  // au défilement, les réglages, le point cliqué, les pastilles et les chiffres. Empilées si étroit.
  function surimpression() {
    if (!profils.some((p) => p.rang === choisi)) choisi = profils.length ? profils[0].rang : 0;
    const R = window.ReglagesImage;
    const panneau = el("div", "image-comparee__panneau defile-gauche");
    etatCalcul = R.etatCalcul(panneau, annuler, analyser);
    // Les résultats (point cliqué, pastilles, chiffres), marqués pendant une analyse relancée sur place.
    const resultats = el("div", "image-comparee__resultats");
    resultats.append(window.PastillesImage.section());
    panneau.append(R.profil(profils, choisi, choisirProfil), R.seuil(seuil, changerSeuil),
      R.affichage(affichage, (v) => { affichage = v; calque(); }), R.pointNoir(pointNoir, changerPointNoir),
      etatCalcul.bloc, resultats);
    // Les chiffres sont toujours là, à lignes fixes, « inconnu » tant qu'il n'y a pas d'analyse.
    tableauChiffres = el("div", "chiffres-image");
    rendu = el("p", "why image-comparee__rendu", t(compense ? "image.rendu.point_noir" : "image.rendu.relatif"));
    resultats.append(el("span", "label", t("image.chiffres")), rendu, tableauChiffres);
    legende = window.VerdictImage.creer(); // verdict et légende de la surimpression (verdict-image.js)
    canvas = el("canvas", "surimpression");
    Object.assign(canvas, { width: resume.largeur, height: resume.hauteur, tabIndex: 0 });
    canvas.setAttribute("role", "img");
    const cadre = window.PastillesImage.cadre(canvas, () => ({ sources, compensation: pointNoir }));
    const barre = el("div", "image-comparee__barre");
    barre.append(legende, window.ImageAgrandie.bouton(cadre, window.PastillesImage.lecture));
    const image = el("div", "image-comparee__image");
    image.append(barre, cadre);
    const colonnes = el("div", "image-comparee__colonnes");
    colonnes.style.setProperty("--ratio", String(resume.largeur / Math.max(resume.hauteur, 1)));
    colonnes.append(image, panneau);
    const bloc = el("div", "image-comparee");
    bloc.append(colonnes);
    return bloc;
  }

  function dessiner() {
    // L'image agrandie se ferme (son cadre est remplacé) ; le champ qui avait le focus le retrouve.
    window.ImageAgrandie.fermer();
    const focus = racine.contains(document.activeElement) ? document.activeElement.id : "";
    etatCalcul = rendu = canvas = legende = tableauChiffres = null;
    const contenu = [];
    const avecPanneau = resume && apercu && profils.length;
    if (!resume) contenu.push(el("p", "prose", t("image.aide")));
    contenu.push(resume ? actions(["image.remplacer", ouvrir], ["image.retirer", retirer]) : actions(["image.ouvrir", ouvrir]));
    if (resume) contenu.push(window.DescriptionImage.bloc(resume, actions(["image.traiter_srvb", traiterCommeSrvb])));
    // Sans panneau, l'état s'écrit ici ; avec lui, à sa place gardée dans le panneau (rien ne bouge).
    if (message && !avecPanneau) contenu.push(statut(message));
    if (enCours && !avecPanneau) contenu.push(statut(t("image.en_cours")), actions(["image.annuler", annuler]));
    if (avecPanneau) contenu.push(surimpression());
    racine.replaceChildren(section("image.titre", ...contenu));
    // Pendant le calcul, « Annuler » ; après une annulation ou une erreur, « Relancer l'analyse ».
    const relancer = resume && analyses === null && resume.profil.etat !== "refuse" && sources.length;
    if (etatCalcul) etatCalcul.etat(enCours ? "calcul" : relancer ? "relancer" : "repos", message);
    choisirProfil(choisi); // calque, avis d'un profil inconnu et nuage
    dessinerChiffres();
    if (focus) document.getElementById(focus)?.focus();
  }

  // La vue « Comparer » range la section sous les chiffres clés, avec ses profils analysés.
  function section_(liste) {
    profils = liste.map((p) => ({ rang: p.rang, serie: p.serie, nom: p.nom }));
    dessiner();
    return racine;
  }

  // Le thème change les jetons : le calque est refait dans les nouvelles couleurs.
  new MutationObserver(calque).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

  return { section: section_, profilsChanges, fermer };
})();
