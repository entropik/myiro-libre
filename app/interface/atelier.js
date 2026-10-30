// Réglage « Atelier » (ticket #89) : les coordonnées de l'atelier et son logo, remplis une fois et
// repris dans le cartouche de chaque rapport PDF. Le formulaire s'ouvre depuis « Modifier l'atelier »
// de la fenêtre d'export (rapport-image.js) ; au premier rapport, si l'atelier est vide, il s'ouvre
// d'abord, avec une phrase qui dit pourquoi. Un aperçu du cartouche suit la frappe. « Enregistrer »
// est le seul bouton principal ; une erreur (logo refusé, champ trop long) s'écrit sous son champ,
// à une place gardée. Le module Rust garde le réglage à côté des données de l'application.
"use strict";

window.Atelier = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  const CHAMPS = [["nom", 120], ["adresse", 400], ["courriel", 200], ["telephone", 200], ["site", 200]];

  let ouvert = null; // { dialogue, logo (URL de l'aperçu), apres }
  let erreurs = {}; // champ → ligne d'erreur sous lui

  async function lireLogo() {
    const octets = await invoke("atelier_logo").catch(() => null);
    if (!octets || !octets.byteLength || !ouvert) return null;
    if (ouvert.logo) URL.revokeObjectURL(ouvert.logo);
    ouvert.logo = URL.createObjectURL(new Blob([octets], { type: "image/png" }));
    return ouvert.logo;
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
    if (logo) {
      const img = el("img", "cartouche-rapport__logo");
      [img.src, img.alt] = [logo, t("rapport.logo")];
      cartouche.append(cellule(img));
    }
    const contacts = [valeurs.courriel, valeurs.telephone, valeurs.site].filter(Boolean);
    cartouche.append(
      cellule(etiquette("cartouche_rapport.atelier"), el("b", "", valeurs.nom || t("rapport.atelier_vide")),
        ...valeurs.adresse.split("\n").filter(Boolean).map((l) => el("span", "", l)), ...contacts.map((c) => el("span", "", c))),
      cellule(etiquette("cartouche_rapport.client"), el("span", "", "—"), etiquette("cartouche_rapport.objet"), el("span", "", "—")),
      cellule(etiquette("cartouche_rapport.numero"), el("b", "", t("atelier.apercu.numero"))));
    zone.replaceChildren(cartouche);
  }

  function montrerErreur(refus) {
    for (const ligne of Object.values(erreurs)) ligne.textContent = "";
    const cle = refus && refus.cle ? refus.cle : String(refus);
    const ligne = (refus && erreurs[refus.champ]) || erreurs.statut;
    ligne.textContent = t(cle);
  }

  function ouvrir(apres, premiere = false) {
    fermer();
    const dialogue = el("dialog", "fenetre-rapport fenetre-atelier");
    dialogue.setAttribute("aria-labelledby", "fenetre-atelier-titre");
    ouvert = { dialogue, logo: null, apres };
    erreurs = {};
    const titre = el("h2", "titre", t("atelier.titre"));
    titre.id = "fenetre-atelier-titre";
    const contenu = [titre];
    if (premiere) contenu.push(el("p", "prose", t("atelier.premiere")));

    const controles = {};
    for (const [champ, longueur] of CHAMPS) {
      const c = champ === "adresse" ? el("textarea", "input") : el("input", "input");
      if (champ === "adresse") c.rows = 3;
      else c.type = champ === "courriel" ? "email" : champ === "telephone" ? "tel" : "text";
      [c.id, c.maxLength] = ["atelier-" + champ, longueur];
      const bloc = el("div", "field");
      const l = el("label", "label", t("atelier.champ." + champ));
      l.htmlFor = c.id;
      erreurs[champ] = el("p", "why fenetre-atelier__erreur");
      bloc.append(l, c, erreurs[champ]);
      controles[champ] = c;
      contenu.push(bloc);
    }

    // Le logo : « Choisir un logo… » ; « Retirer le logo », inactif sans logo, dit pourquoi.
    const logo = el("div", "field");
    const choisir = el("button", "btn", t("atelier.choisir_logo"));
    const retirer = el("button", "btn", t("atelier.retirer_logo"));
    const raisonLogo = el("p", "why", t("atelier.logo.aide"));
    erreurs.logo = el("p", "why fenetre-atelier__erreur");
    const boutonsLogo = el("div", "actions");
    boutonsLogo.append(choisir, retirer, raisonLogo);
    logo.append(el("span", "label", t("atelier.logo")), boutonsLogo, erreurs.logo);
    contenu.push(logo);

    const zoneApercu = el("div", "fenetre-atelier__apercu");
    contenu.push(el("span", "label", t("atelier.apercu")), zoneApercu);

    erreurs.statut = el("p", "why fenetre-atelier__erreur");
    erreurs.statut.setAttribute("role", "status");
    const enregistrer = el("button", "btn btn--primary", t("atelier.enregistrer"));
    const fermerBtn = el("button", "btn", t("atelier.fermer"));
    fermerBtn.addEventListener("click", () => dialogue.close());
    const actions = el("div", "actions");
    actions.append(enregistrer, fermerBtn);
    contenu.push(erreurs.statut, actions);
    dialogue.append(...contenu);

    let avecLogo = false;
    const valeurs = () => Object.fromEntries(CHAMPS.map(([c]) => [c, controles[c].value]));
    const redessiner = () => apercu(zoneApercu, valeurs(), avecLogo ? ouvert?.logo : null);
    const poserLogo = async (present) => {
      avecLogo = present;
      retirer.disabled = !present;
      raisonLogo.textContent = present ? t("atelier.logo.aide") : t("atelier.retirer_logo.raison");
      if (present) await lireLogo();
      redessiner();
    };
    for (const [c] of CHAMPS) controles[c].addEventListener("input", redessiner);

    choisir.addEventListener("click", async () => {
      try {
        const vue = await invoke("atelier_choisir_logo", { filtre: t("atelier.filtre_logo") });
        if (vue) { erreurs.logo.textContent = ""; await poserLogo(vue.logo); }
      } catch (refus) {
        montrerErreur(refus);
      }
    });
    retirer.addEventListener("click", async () => {
      try {
        const vue = await invoke("atelier_retirer_logo");
        await poserLogo(vue.logo);
        choisir.focus();
      } catch (refus) {
        montrerErreur(refus);
      }
    });
    enregistrer.addEventListener("click", async () => {
      try {
        await invoke("atelier_enregistrer", { saisi: valeurs() });
        for (const ligne of Object.values(erreurs)) ligne.textContent = "";
        erreurs.statut.textContent = t("atelier.enregistre");
        if (ouvert && ouvert.apres) ouvert.apres();
      } catch (refus) {
        montrerErreur(refus);
      }
    });

    dialogue.addEventListener("close", rendre);
    document.body.append(dialogue);
    dialogue.showModal();
    controles.nom.focus();
    invoke("atelier_lire").then((vue) => {
      if (!ouvert || ouvert.dialogue !== dialogue) return;
      for (const [c] of CHAMPS) controles[c].value = vue.saisi[c] || "";
      poserLogo(vue.logo);
    }).catch(montrerErreur);
    poserLogo(false);
  }

  // Au premier rapport, l'atelier vide s'ouvre d'abord à remplir.
  async function premiere(apres) {
    const vue = await invoke("atelier_lire").catch(() => null);
    if (vue && !vue.saisi.nom.trim()) ouvrir(apres, true);
  }

  // La fenêtre disparaît, le focus revient à la fenêtre d'export (ou au bouton qui l'a ouverte).
  function rendre() {
    if (!ouvert) return;
    const { dialogue, logo } = ouvert;
    ouvert = null;
    if (logo) URL.revokeObjectURL(logo);
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

  return { ouvrir, premiere, fermer };
})();
