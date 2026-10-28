// Section « Image » de la vue « Comparer » (ticket #80) : une photo de client ouverte pour la
// séance (jamais rangée ni copiée), analysée face à chaque profil affiché par le module Rust
// `comparer::image`. Les pixels de l'image d'analyse et les codes d'écart arrivent en binaire et
// sont posés par putImageData. Un code d'écart est un dixième de ΔE00 (0 à 254), 255 = inconnu ou
// transparent : un pixel est au-delà du seuil si son code vaut au moins round(10 × seuil). Changer
// le seuil, le profil de la surimpression ou l'affichage ne relance aucune commande. Les pixels à
// l'écart inconnu sont dits à part, jamais comptés comme bons. Les textes viennent du catalogue.
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
  // Familles de teinte, dans l'ordre des lignes de `par_teinte` (gamut::FAMILLES_TEINTE).
  const FAMILLES = ["rouge", "jaune", "vert", "cyan", "bleu", "magenta", "gris"];
  const AFFICHAGES = ["image", "les_deux", "alerte"];
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
  let canvas = null;
  let legende = null;
  let tableauChiffres = null;
  let avisInconnu = null; // raison d'un profil de surimpression inconnu
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
      demande++;
      [resume, apercu, analyses, ecarts] = [r, null, null, []];
      window.NuageImage.oublier(); // nouvelle image : nuage libéré, case recochée
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
    analyses = null;
    ecarts = [];
    if (!resume || resume.profil.etat === "refuse" || !sources.length) {
      enCours = false;
      dessiner();
      return;
    }
    enCours = true;
    dessiner();
    try {
      const reponse = await invoke("image_analyser", { sources });
      if (n !== demande) return; // périmée : la demande plus récente tient « en cours »
      const octets = await Promise.all(reponse.profils.map((a, index) => (a.etat.etat === "calculee"
        ? invoke("image_ecarts", { analyse: reponse.numero, index }).then((b) => new Uint8Array(b))
        : null)));
      if (n !== demande) return;
      analyses = reponse.profils;
      ecarts = octets;
    } catch (cle) {
      if (n === demande) message = t(String(cle));
    }
    if (n !== demande) return;
    enCours = false;
    dessiner();
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
    [sources, profils, seuil, affichage] = [[], [], 2, "les_deux"];
    racine.replaceChildren();
    imageCalque = canvas = legende = tableauChiffres = avisInconnu = null;
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

  function dessinerChiffres() {
    if (!tableauChiffres) return;
    const k = code(seuil);
    const colonne = (p) => {
      const a = analyseDe(p.rang);
      if (!a) return [...Array(6)].map(() => t("comparer.inconnu"));
      return [pourcent(part(a, k, 255)), ...CLASSES.map(([, de, a_]) => pourcent(part(a, de, a_))),
        teintes(a), inconnus(a), nombre(a.couleurs_distinctes, 0)];
    };
    const colonnes = profils.map(colonne);
    const cles = ["image.chiffres.part", ...CLASSES.map(([nom]) => "image.classe." + nom),
      "image.chiffres.teintes", "image.chiffres.inconnus", "image.chiffres.couleurs"];
    const lignes = cles.map((cle, i) => [
      t(cle).replace("{s}", () => nombre(seuil, 1)), ...colonnes.map((c) => c[i])]);
    // Un profil inconnu dit pourquoi, dans sa colonne.
    const raisons = profils.map((p) => (analyses?.[p.rang]?.etat.etat === "inconnue" ? texteRefus(analyses[p.rang].etat) : ""));
    if (raisons.some(Boolean)) lignes.push([t("image.chiffres.raison"), ...raisons]);
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
    const p = profils.find((x) => x.rang === choisi);
    const nom = p ? p.nom : "";
    const a = analyseDe(choisi);
    canvas.setAttribute("aria-label", t("image.canvas.aria").replace("{p}", () => nombre(a ? part(a, k, 255) : null, 1))
      .replace("{s}", () => nombre(seuil, 1)).replace("{profil}", () => nom));
    if (legende) legende.lastChild.textContent = t("image.legende").replace("{s}", () => nombre(seuil, 1)).replace("{profil}", () => nom);
  }

  // Le nuage du profil de la surimpression suit dans la vue 3D et la coupe (nuage-image.js).
  const nuage = () => window.NuageImage.changer(analyseDe(choisi), seuil, (profils.find((x) => x.rang === choisi) || {}).nom || "");

  function changerSeuil(valeur) {
    seuil = valeur;
    calque();
    dessinerChiffres();
    nuage();
  }

  // ---- Réglages : profil, seuil, affichage ----
  function choixProfil() {
    const champ = el("div", "field");
    const s = el("select", "input");
    s.id = "image-profil";
    for (const p of profils) {
      const o = el("option", "", p.nom);
      o.value = String(p.rang);
      o.selected = p.rang === choisi;
      s.append(o);
    }
    // Seuls le calque et son avis changent : le choix garde le focus du clavier.
    s.addEventListener("change", () => choisirProfil(Number(s.value)));
    const l = el("label", "label", t("image.surimpression.profil"));
    l.htmlFor = s.id;
    champ.append(l, s);
    return champ;
  }

  function choisirProfil(rang) {
    choisi = rang;
    const a = analyses?.[choisi];
    if (avisInconnu) {
      avisInconnu.hidden = a?.etat.etat !== "inconnue";
      avisInconnu.textContent = avisInconnu.hidden ? "" : t("image.surimpression.inconnue").replace("{raison}", () => texteRefus(a.etat));
    }
    calque();
    nuage();
  }

  function curseurSeuil() {
    const champ = el("div", "field");
    const curseur = el("input", "curseur");
    Object.assign(curseur, { type: "range", min: 0.5, max: 10, step: 0.5, value: seuil, id: "image-seuil" });
    const libelle = el("label", "label");
    const ecrire = () => { libelle.textContent = t("image.seuil.valeur").replace("{s}", () => nombre(seuil, 1)); };
    libelle.htmlFor = curseur.id;
    ecrire();
    curseur.addEventListener("input", () => { changerSeuil(Number(curseur.value)); ecrire(); });
    champ.append(libelle, curseur);
    return champ;
  }

  function choixAffichage() {
    const champ = el("div", "field");
    const libelle = el("span", "label", t("image.affichage"));
    libelle.id = "image-affichage";
    const groupe = el("div", "seg");
    groupe.setAttribute("role", "group");
    groupe.setAttribute("aria-labelledby", libelle.id);
    for (const v of AFFICHAGES) {
      const b = el("button", "", t("image.affichage." + v));
      b.setAttribute("aria-pressed", String(v === affichage));
      b.addEventListener("click", () => {
        affichage = v;
        for (const autre of groupe.children) autre.setAttribute("aria-pressed", String(autre === b));
        calque();
      });
      groupe.append(b);
    }
    champ.append(libelle, groupe);
    return champ;
  }

  // ---- Dessin de la section ----
  function etatProfil() {
    const p = resume.profil;
    if (p.etat === "embarque") return t("image.profil.embarque").replace("{nom}", () => p.nom || t("image.profil.sans_nom"));
    if (p.etat === "srvb") return t(p.suppose ? "image.profil.suppose" : "image.profil.srvb");
    return texteRefus(p);
  }

  function description() {
    const bloc = el("div");
    const nom = el("p", "");
    nom.append(el("b", "", resume.nom));
    bloc.append(nom, el("p", resume.profil.etat === "refuse" ? "why" : "", etatProfil()));
    for (const e of resume.profil.ecartees || []) {
      bloc.append(el("p", "why", t("image.profil.ecartee").replace("{table}", () => e.table)
        .replace("{raison}", () => texteRefus(e))));
    }
    if (resume.profil.etat === "refuse") bloc.append(actions(["image.traiter_srvb", traiterCommeSrvb]));
    if (resume.pas > 1) bloc.append(el("p", "why", t("image.approche").replace("{n}", () => nombre(resume.pas * resume.pas, 0))));
    if (resume.seize_bits) bloc.append(el("p", "why", t("image.seize_bits")));
    bloc.append(el("p", "why", t("image.taille")
      .replace("{l}", () => nombre(resume.largeur, 0)).replace("{h}", () => nombre(resume.hauteur, 0))
      .replace("{lo}", () => nombre(resume.largeur_origine, 0)).replace("{ho}", () => nombre(resume.hauteur_origine, 0))));
    return bloc;
  }

  function surimpression() {
    const bloc = el("div");
    if (!profils.some((p) => p.rang === choisi)) choisi = profils.length ? profils[0].rang : 0;
    avisInconnu = el("p", "why");
    bloc.append(choixProfil(), curseurSeuil(), choixAffichage(), avisInconnu);
    legende = el("p", "legende-alerte");
    const carre = el("span", "swatch");
    carre.style.setProperty("--c", "var(--couleur-hors-gamut)");
    carre.setAttribute("aria-hidden", "true");
    legende.append(carre, el("span", ""));
    canvas = el("canvas", "surimpression");
    Object.assign(canvas, { width: resume.largeur, height: resume.hauteur, tabIndex: 0 });
    canvas.setAttribute("role", "img");
    bloc.append(legende, canvas);
    return bloc;
  }

  function dessiner() {
    canvas = legende = tableauChiffres = avisInconnu = null;
    const contenu = [];
    if (!resume) contenu.push(el("p", "prose", t("image.aide")));
    contenu.push(resume ? actions(["image.remplacer", ouvrir], ["image.retirer", retirer]) : actions(["image.ouvrir", ouvrir]));
    if (resume) contenu.push(description());
    if (message) contenu.push(statut(message));
    // Pendant le calcul, « Annuler » ; après une annulation ou une erreur, « Relancer l'analyse ».
    if (enCours) contenu.push(statut(t("image.en_cours")), actions(["image.annuler", annuler]));
    else if (resume && analyses === null && resume.profil.etat !== "refuse" && sources.length) {
      contenu.push(actions(["image.relancer", analyser]));
    }
    if (resume && apercu && profils.length) contenu.push(surimpression());
    if (resume && analyses && profils.length) {
      tableauChiffres = el("div");
      contenu.push(el("span", "label", t("image.chiffres")), tableauChiffres);
    }
    racine.replaceChildren(section("image.titre", ...contenu));
    choisirProfil(choisi); // calque, avis d'un profil inconnu et nuage
    dessinerChiffres();
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
