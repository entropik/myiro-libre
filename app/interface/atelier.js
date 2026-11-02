// Réglage « Atelier » (ticket #89) : les coordonnées de l'atelier et son logo, remplis une fois et
// repris dans le cartouche de chaque rapport PDF. Le même formulaire vit à deux places : en page,
// dans la vue « Réglages » de la barre du haut, et en fenêtre, depuis « Modifier l'atelier » de la
// fenêtre d'export (rapport-image.js) ; au premier rapport, si l'atelier est vide, la fenêtre s'ouvre
// d'abord, avec une phrase qui dit pourquoi. Un aperçu du cartouche suit la frappe. « Enregistrer »
// est le seul bouton principal ; une erreur (logo refusé, champ trop long) s'écrit sous son champ,
// à une place gardée. Le module Rust garde le réglage à côté des données de l'application.
//
// Le logo est peint dans un canvas, jamais chargé par une adresse `blob:` ou `data:` : la politique
// de sécurité de la page (`img-src 'self'`, ADR 0003) le refuserait et l'aperçu serait cassé.
"use strict";

window.Atelier = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const CHAMPS = [["nom", 120], ["adresse", 400], ["courriel", 200], ["telephone", 200], ["site", 200]];

  // Un texte du catalogue repris sur place au changement de langue (app.js relit `data-t`).
  function txt(nom, classe, cle) {
    const e = el(nom, classe, t(cle));
    e.dataset.t = cle;
    return e;
  }

  // Le logo enregistré, décodé une fois ; `null` sans logo.
  async function decoderLogo() {
    const octets = await invoke("atelier_logo").catch(() => null);
    if (!octets || !octets.byteLength) return null;
    return createImageBitmap(new Blob([octets], { type: "image/png" })).catch(() => null);
  }

  // Le logo peint dans un canvas à sa taille ; le CSS le contient dans sa case.
  function peindreLogo(image, classe) {
    const c = el("canvas", classe);
    [c.width, c.height] = [image.width, image.height];
    c.getContext("2d")?.drawImage(image, 0, 0);
    c.setAttribute("role", "img");
    c.setAttribute("aria-label", t("rapport.logo"));
    return c;
  }

  // L'aperçu du cartouche : mêmes cases que celui du rapport (crate `rapport`), en petit.
  function apercu(zone, valeurs, logo) {
    const cartouche = el("div", "cartouche-rapport");
    const etiquette = (cle) => el("span", "cartouche-rapport__etiquette", t(cle));
    const cellule = (...enfants) => {
      const c = el("div", "cartouche-rapport__cellule");
      c.append(...enfants);
      return c;
    };
    if (logo) cartouche.append(cellule(peindreLogo(logo, "cartouche-rapport__logo")));
    const contacts = [valeurs.courriel, valeurs.telephone, valeurs.site].filter(Boolean);
    cartouche.append(
      cellule(etiquette("cartouche_rapport.atelier"), el("b", "", valeurs.nom || t("rapport.atelier_vide")),
        ...valeurs.adresse.split("\n").filter(Boolean).map((l) => el("span", "", l)), ...contacts.map((c) => el("span", "", c))),
      cellule(etiquette("cartouche_rapport.client"), el("span", "", "—"), etiquette("cartouche_rapport.objet"), el("span", "", "—")),
      cellule(etiquette("cartouche_rapport.numero"), el("b", "", t("atelier.apercu.numero"))));
    zone.replaceChildren(cartouche);
  }

  function montrerErreur(erreurs, refus) {
    for (const ligne of Object.values(erreurs)) ligne.textContent = "";
    const cle = refus && refus.cle ? refus.cle : String(refus);
    const ligne = (refus && erreurs[refus.champ]) || erreurs.statut;
    ligne.textContent = t(cle);
  }

  // Le formulaire, sans son cadre : champs, logo, aperçu, statut et « Enregistrer ». `apres` suit
  // un enregistrement réussi ; `fermer` ajoute un bouton « Fermer » (fenêtre).
  function formulaire({ premiere = false, apres = null, fermer = null, prefixe }) {
    const erreurs = {};
    const contenu = [];
    if (premiere) contenu.push(txt("p", "prose", "atelier.premiere"));

    const controles = {};
    for (const [champ, longueur] of CHAMPS) {
      const c = champ === "adresse" ? el("textarea", "input") : el("input", "input");
      if (champ === "adresse") c.rows = 3;
      else c.type = champ === "courriel" ? "email" : champ === "telephone" ? "tel" : "text";
      [c.id, c.maxLength] = [prefixe + champ, longueur];
      const bloc = el("div", "field");
      const l = txt("label", "label", "atelier.champ." + champ);
      l.htmlFor = c.id;
      erreurs[champ] = el("p", "why fenetre-atelier__erreur");
      bloc.append(l, c, erreurs[champ]);
      controles[champ] = c;
      contenu.push(bloc);
    }

    // Le logo : « Choisir un logo… » ; « Retirer le logo », inactif sans logo, dit pourquoi.
    const logo = el("div", "field");
    const choisir = txt("button", "btn", "atelier.choisir_logo");
    const retirer = txt("button", "btn", "atelier.retirer_logo");
    const raisonLogo = txt("p", "why", "atelier.logo.aide");
    erreurs.logo = el("p", "why fenetre-atelier__erreur");
    const boutonsLogo = el("div", "actions");
    boutonsLogo.append(choisir, retirer, raisonLogo);
    logo.append(txt("span", "label", "atelier.logo"), boutonsLogo, erreurs.logo);
    contenu.push(logo);

    const zoneApercu = el("div", "fenetre-atelier__apercu");
    contenu.push(txt("span", "label", "atelier.apercu"), zoneApercu);

    erreurs.statut = el("p", "why fenetre-atelier__erreur");
    erreurs.statut.setAttribute("role", "status");
    const enregistrer = txt("button", "btn btn--primary", "atelier.enregistrer");
    const actions = el("div", "actions");
    actions.append(enregistrer);
    if (fermer) {
      const fermerBtn = txt("button", "btn", "atelier.fermer");
      fermerBtn.addEventListener("click", fermer);
      actions.append(fermerBtn);
    }
    contenu.push(erreurs.statut, actions);

    let image = null; // logo décodé (ImageBitmap)
    const valeurs = () => Object.fromEntries(CHAMPS.map(([c]) => [c, controles[c].value]));
    const redessiner = () => apercu(zoneApercu, valeurs(), image);
    const poserLogo = async (present) => {
      retirer.disabled = !present;
      raisonLogo.dataset.t = present ? "atelier.logo.aide" : "atelier.retirer_logo.raison";
      raisonLogo.textContent = t(raisonLogo.dataset.t);
      const nouveau = present ? await decoderLogo() : null;
      image?.close();
      image = nouveau;
      redessiner();
    };
    for (const [c] of CHAMPS) controles[c].addEventListener("input", redessiner);

    choisir.addEventListener("click", async () => {
      try {
        const vue = await invoke("atelier_choisir_logo", { filtre: t("atelier.filtre_logo") });
        if (vue) { erreurs.logo.textContent = ""; await poserLogo(vue.logo); }
      } catch (refus) {
        montrerErreur(erreurs, refus);
      }
    });
    retirer.addEventListener("click", async () => {
      try {
        const vue = await invoke("atelier_retirer_logo");
        await poserLogo(vue.logo);
        choisir.focus();
      } catch (refus) {
        montrerErreur(erreurs, refus);
      }
    });
    enregistrer.addEventListener("click", async () => {
      try {
        await invoke("atelier_enregistrer", { saisi: valeurs() });
        for (const ligne of Object.values(erreurs)) ligne.textContent = "";
        erreurs.statut.textContent = t("atelier.enregistre");
        if (apres) apres();
      } catch (refus) {
        montrerErreur(erreurs, refus);
      }
    });

    // Relit le réglage enregistré et pose les valeurs sur place, sans rien redessiner d'autre.
    async function relire() {
      try {
        const vue = await invoke("atelier_lire");
        for (const [c] of CHAMPS) controles[c].value = vue.saisi[c] || "";
        await poserLogo(vue.logo);
      } catch (refus) {
        montrerErreur(erreurs, refus);
      }
    }

    retirer.disabled = true;
    redessiner();
    return { contenu, relire, redessiner, focus: () => controles.nom.focus(), liberer: () => image?.close() };
  }

  // ---- En fenêtre, depuis le rapport ----
  let ouvert = null; // { dialogue, formulaire }

  function ouvrir(apres, premiere = false) {
    fermer();
    const dialogue = el("dialog", "fenetre-rapport fenetre-atelier");
    dialogue.setAttribute("aria-labelledby", "fenetre-atelier-titre");
    const titre = txt("h2", "titre", "atelier.titre");
    titre.id = "fenetre-atelier-titre";
    const f = formulaire({ premiere, apres, fermer: () => dialogue.close(), prefixe: "atelier-" });
    ouvert = { dialogue, formulaire: f };
    dialogue.append(titre, ...f.contenu);
    dialogue.addEventListener("close", rendre);
    document.body.append(dialogue);
    dialogue.showModal();
    f.focus();
    f.relire();
  }

  // Au premier rapport, l'atelier vide s'ouvre d'abord à remplir ; rempli, plus jamais.
  async function premiere(apres) {
    const vue = await invoke("atelier_lire").catch(() => null);
    if (vue && !vue.saisi.nom.trim()) ouvrir(apres, true);
  }

  // La fenêtre disparaît, le focus revient à la fenêtre d'export (ou au bouton qui l'a ouverte).
  function rendre() {
    if (!ouvert) return;
    const { dialogue, formulaire: f } = ouvert;
    ouvert = null;
    f.liberer();
    const precedent = dialogue.previousElementSibling;
    dialogue.remove();
    precedent?.querySelector("button")?.focus();
  }

  function fermer() {
    if (!ouvert) return;
    const { dialogue } = ouvert;
    rendre();
    if (dialogue.open) dialogue.close();
  }

  // ---- En page, dans la vue « Réglages » ----
  // Posé une fois, à la première visite ; ensuite relu sur place à chaque retour sur la vue (la
  // fenêtre du rapport a pu l'enregistrer entre-temps). Le focus reste où l'opérateur l'a mis.
  let enPage = null;
  document.addEventListener("tache-affichee", (e) => {
    if (e.detail !== "reglages") return;
    const zone = document.querySelector("[data-reglages-atelier]");
    if (!zone) return;
    if (!enPage) {
      enPage = formulaire({ prefixe: "reglages-atelier-" });
      zone.replaceChildren(...enPage.contenu);
    }
    enPage.relire();
  });
  // L'aperçu est le seul morceau écrit sans `data-t` : il suit la langue.
  document.addEventListener("langue-appliquee", () => enPage?.redessiner());

  return { ouvrir, premiere, fermer, decoderLogo, peindreLogo };
})();
