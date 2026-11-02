// Inspection d'une feuille d'une séance de mesure (ticket #35) : chaque plage de la mire dans une
// grille légère, la couleur demandée à l'imprimante (approchée à l'écran), une plage manquante
// barrée, une plage incertaine bordée ; au survol ou au clavier, la plage, le demandé, le RVB du
// fichier et les Lab mesurés, sur une ligne gardée. Aucun calcul de couleur ici : les valeurs
// viennent du module Rust `seance_fd9`, et l'aperçu écran n'est pas une preuve colorimétrique
// (la page le dit). Puis les compléments manuels de la séance. Tout s'écrit sur place, dans les
// blocs de taille gardée de index.html. Utilisé par seance.js, qui prête l'état de la séance et
// de quoi le recharger.
"use strict";

window.SeancePlages = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const bloc = document.querySelector("[data-seance-plages]");
  const nomFichier = bloc.querySelector("[data-seance-plages-fichier]");
  const toile = bloc.querySelector("[data-seance-plages-toile]");
  const consigne = bloc.querySelector("[data-seance-plages-consigne]");
  const survol = bloc.querySelector("[data-seance-plages-survol]");
  const fermer = bloc.querySelector("[data-seance-plages-fermer]");
  const champPlage = bloc.querySelector("[data-seance-complement-plage]");
  const champSource = bloc.querySelector("[data-seance-complement-source]");
  const champIdentifiant = bloc.querySelector("[data-seance-complement-identifiant]");
  const ajouter = bloc.querySelector("[data-seance-complement-ajouter]");
  const liste = bloc.querySelector("[data-seance-complements]");
  const vide = bloc.querySelector("[data-seance-complements-vide]");
  const modele = bloc.querySelector("[data-seance-complement-modele]");
  const message = bloc.querySelector("[data-seance-complements-message]");

  // Ce que seance.js prête : la séance, son état (`seance_etat`) et de quoi le recharger.
  let seance = { id: () => null, etat: () => null, recharger: async () => {}, texteErreur: (e) => String(e) };
  let plages = []; // réponse de `seance_inspecter` : une entrée par plage du plan
  let choisie = -1; // plage survolée ou choisie au clavier
  let demande = 0; // la dernière inspection demandée l'emporte
  let texteMessage = "";
  let occupe = false;
  const lignes = []; // lignes de compléments, créées au besoin puis gardées

  const hexa = (c) => /^#[0-9a-fA-F]{6}$/.test(c);
  const octet = (v) => Math.max(0, Math.min(255, Math.round(v * 2.55)));
  const hex = (rvb) => `#${rvb.map((v) => octet(v).toString(16).padStart(2, "0")).join("")}`;
  const jeton = (nom) => getComputedStyle(document.documentElement).getPropertyValue(nom).trim();
  const decimal = (v) => new Intl.NumberFormat(document.documentElement.lang, {
    minimumFractionDigits: 1, maximumFractionDigits: 1,
  }).format(v);
  const EMPLACEMENTS = ["M0", "M1", "M2"];

  // ---- La grille : des cases carrées qui remplissent la toile, de taille fixe ----
  function grille() {
    const l = toile.clientWidth;
    const h = toile.clientHeight;
    const n = plages.length;
    if (!n || !l || !h) return null;
    const colonnes = Math.max(1, Math.ceil(Math.sqrt((n * l) / h)));
    const rangees = Math.ceil(n / colonnes);
    const cote = Math.max(2, Math.floor(Math.min(l / colonnes, h / rangees)));
    return { colonnes, cote };
  }

  function dessiner() {
    const ratio = window.devicePixelRatio || 1;
    toile.width = Math.round(toile.clientWidth * ratio);
    toile.height = Math.round(toile.clientHeight * ratio);
    const ctx = toile.getContext("2d");
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.fillStyle = jeton("--fond");
    ctx.fillRect(0, 0, toile.clientWidth, toile.clientHeight);
    const g = grille();
    if (!g) return;
    const plein = g.cote - 1; // un filet de fond entre deux cases
    plages.forEach((p, i) => {
      const x = (i % g.colonnes) * g.cote;
      const y = Math.floor(i / g.colonnes) * g.cote;
      const c = hex(p.rvb_attendu);
      ctx.fillStyle = hexa(c) ? c : jeton("--fond");
      ctx.fillRect(x, y, plein, plein);
      if (p.etat === "manquante") {
        ctx.strokeStyle = jeton("--attenue");
        ctx.beginPath();
        ctx.moveTo(x, y + plein);
        ctx.lineTo(x + plein, y);
        ctx.stroke();
      } else if (p.etat === "douteuse") {
        ctx.strokeStyle = jeton("--hors");
        ctx.strokeRect(x + 0.5, y + 0.5, plein - 1, plein - 1);
      }
    });
    if (choisie >= 0 && choisie < plages.length) {
      const x = (choisie % g.colonnes) * g.cote;
      const y = Math.floor(choisie / g.colonnes) * g.cote;
      ctx.strokeStyle = jeton("--fond");
      ctx.strokeRect(x - 1.5, y - 1.5, plein + 3, plein + 3);
      ctx.strokeStyle = jeton("--encre");
      ctx.strokeRect(x - 0.5, y - 0.5, plein + 1, plein + 1);
    }
  }

  // ---- La ligne du survol : la plage, le demandé, le fichier, les Lab, l'état ----
  function ecrireSurvol() {
    const p = plages[choisie];
    if (!p) {
      survol.textContent = t("seance.plages.survol.vide");
      survol.title = "";
      consigne.style.removeProperty("--c");
      return;
    }
    const c = hex(p.rvb_attendu);
    if (hexa(c)) consigne.style.setProperty("--c", c);
    const rvb = (v) => v.map((x) => String(octet(x))).join(" ");
    const morceaux = [
      t("seance.plages.plage").replace("{id}", () => p.identifiant),
      t("seance.plages.demande").replace("{rvb}", () => rvb(p.rvb_attendu)),
    ];
    if (p.rvb_fichier) morceaux.push(t("seance.plages.fichier").replace("{rvb}", () => rvb(p.rvb_fichier)));
    p.lab.forEach((lab, k) => {
      if (!lab) return;
      morceaux.push(t("seance.plages.lab").replace("{m}", () => EMPLACEMENTS[k])
        .replace("{l}", () => decimal(lab[0])).replace("{a}", () => decimal(lab[1]))
        .replace("{b}", () => decimal(lab[2])));
    });
    morceaux.push(t(`seance.plages.etat.${p.etat}`));
    survol.textContent = morceaux.join(" · ");
    survol.title = survol.textContent;
  }

  function choisir(i) {
    if (!plages.length) return;
    choisie = Math.max(0, Math.min(plages.length - 1, i));
    dessiner();
    ecrireSurvol();
  }

  function plageSous(evenement) {
    const g = grille();
    if (!g) return -1;
    const r = toile.getBoundingClientRect();
    const colonne = Math.floor((evenement.clientX - r.left) / g.cote);
    const rangee = Math.floor((evenement.clientY - r.top) / g.cote);
    const i = rangee * g.colonnes + colonne;
    return colonne >= 0 && colonne < g.colonnes && i >= 0 && i < plages.length ? i : -1;
  }

  toile.addEventListener("pointermove", (e) => {
    const i = plageSous(e);
    if (i >= 0 && i !== choisie) choisir(i);
  });
  toile.addEventListener("keydown", (e) => {
    const g = grille();
    if (!g) return;
    const pas = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: g.colonnes, ArrowUp: -g.colonnes }[e.key];
    if (pas !== undefined) choisir(Math.max(0, choisie) + pas);
    else if (e.key === "Home") choisir(0);
    else if (e.key === "End") choisir(plages.length - 1);
    else return;
    e.preventDefault();
  });

  // ---- Ouvrir et fermer l'inspection ; l'ancienne grille reste jusqu'à la nouvelle ----
  async function ouvrir(mesure, fichier) {
    bloc.hidden = false;
    nomFichier.textContent = fichier;
    nomFichier.title = fichier;
    const n = ++demande;
    try {
      const r = await invoke("seance_inspecter", { seance: seance.id(), mesure });
      if (n !== demande) return;
      plages = r;
      choisie = -1;
      texteMessage = "";
    } catch (erreur) {
      if (n !== demande) return;
      texteMessage = seance.texteErreur(erreur);
    }
    dessiner();
    ecrireSurvol();
    ecrireComplements();
  }

  fermer.addEventListener("click", () => {
    bloc.hidden = true;
  });

  // ---- Compléments manuels : une plage du plan prise à la main dans une mesure de complément ----
  function remplirSources(sources) {
    const voulues = sources.map((s) => String(s.mesure));
    const actuelles = [...champSource.options].map((o) => o.value);
    if (voulues.join() === actuelles.join()) return; // rien ne change : le choix reste
    const choisi = champSource.value;
    champSource.length = 0;
    for (const s of sources) champSource.add(new Option(s.fichier, String(s.mesure)));
    if (voulues.includes(choisi)) champSource.value = choisi;
  }

  function ligneComplement(i) {
    if (!lignes[i]) {
      const ligne = modele.content.firstElementChild.cloneNode(true);
      ligne.querySelector("[data-retirer]").addEventListener("click", () => retirer(Number(ligne.dataset.rang)));
      liste.append(ligne);
      lignes[i] = ligne;
    }
    return lignes[i];
  }

  function ecrireComplements() {
    const e = seance.etat();
    const complements = e ? e.complements : [];
    const sources = e ? e.sources.filter((s) => s.role === "complement") : [];
    const ouverte = Boolean(e && e.etat === "en_cours");
    remplirSources(sources);
    complements.forEach((c, i) => {
      const ligne = ligneComplement(i);
      const source = sources.find((s) => s.mesure === c.source);
      const texte = t("seance.complements.ligne").replace("{plage}", () => c.plage)
        .replace("{identifiant}", () => c.identifiant)
        .replace("{fichier}", () => (source ? source.fichier : String(c.source)));
      ligne.dataset.rang = String(i);
      ligne.querySelector("[data-texte]").textContent = texte;
      ligne.querySelector("[data-texte]").title = texte;
      const bouton = ligne.querySelector("[data-retirer]");
      bouton.textContent = t("seance.complements.retirer");
      bouton.disabled = occupe || !ouverte;
      ligne.hidden = false;
    });
    lignes.slice(complements.length).forEach((l) => { l.hidden = true; });
    vide.hidden = complements.length > 0;
    for (const champ of [champPlage, champSource, champIdentifiant]) champ.disabled = occupe || !ouverte;
    ajouter.disabled = occupe || !ouverte || !sources.length;
    message.textContent = texteMessage
      || (ouverte && !sources.length ? t("seance.complements.sans_source") : "");
    message.title = message.textContent;
  }

  async function changerComplements(complements, apres) {
    if (occupe) return;
    occupe = true;
    ecrireComplements();
    try {
      await invoke("seance_completer", { seance: seance.id(), complements });
      texteMessage = "";
      apres();
      await seance.recharger();
    } catch (erreur) {
      texteMessage = seance.texteErreur(erreur);
    }
    occupe = false;
    ecrireComplements();
  }

  ajouter.addEventListener("click", () => {
    const e = seance.etat();
    if (!e) return;
    const nouveau = {
      plage: champPlage.value.trim(),
      source: Number(champSource.value),
      identifiant: champIdentifiant.value.trim(),
    };
    changerComplements([...e.complements, nouveau], () => {
      champPlage.value = "";
      champIdentifiant.value = "";
    });
  });

  function retirer(rang) {
    const e = seance.etat();
    if (e) changerComplements(e.complements.filter((_, i) => i !== rang), () => {});
  }

  // La toile suit sa taille et le thème ; la langue réécrit les lignes.
  new ResizeObserver(() => dessiner()).observe(toile);
  new MutationObserver(() => dessiner())
    .observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  document.addEventListener("langue-appliquee", () => {
    ecrireSurvol();
    ecrireComplements();
  });

  return {
    brancher: (fonctions) => { seance = fonctions; },
    ouvrir,
    // Une autre séance, ou une autre résolution : l'inspection se ferme.
    fermer: () => {
      bloc.hidden = true;
      plages = [];
      choisie = -1;
      texteMessage = "";
    },
    // Après un rechargement de la séance : compléments et sources. Une mesure source ne change
    // jamais : la grille inspectée reste telle quelle.
    ecrire: ecrireComplements,
  };
})();
