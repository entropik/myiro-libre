// Vue d'une résolution d'impression, choisie dans l'arbre (parc.js) : au centre, le profil actif
// (verdict couleur + icône + mot), le seul bouton principal « Installer dans Windows », exporter,
// ranger un profil existant, l'historique et les mesures ; à droite, la provenance du profil.
// Les actions passent par le module Rust `parc` ; les textes, par le catalogue (app.js).
"use strict";

window.Reglage = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;

  const vue = document.querySelector("[data-vue-reglage]");
  const details = document.querySelector("[data-details-reglage]");
  const annonce = vue.querySelector("[data-reglage-message]");
  // Ce que parc.js prête à la vue : la résolution affichée et de quoi recharger.
  let parc = { reglage: () => null, rafraichir: async () => {}, recharger: async () => {} };

  const { el, date } = window.Arbre;

  // Verdict : couleur, icône et mot, jamais la couleur seule.
  const TRACES = {
    conforme: "M5 12l4 4 10-10",
    limite: "M12 4v10M12 18v2",
    hors: "M6 6l12 12M18 6L6 18",
    aucun: "M5 12h14",
  };
  const VERDICTS = {
    accepte: ["conforme", "parc.verdict.verifie"],
    non_verifie: ["limite", "parc.verdict.non_verifie"],
    hors_tolerance: ["hors", "parc.verdict.hors"],
    incomplet: ["limite", "parc.verdict.incomplet"],
  };

  function annoncer(texte) {
    annonce.textContent = texte;
    annonce.hidden = !texte;
  }

  function remplirVue(d, lignesMesures) {
    const titre = vue.querySelector("[data-reglage-titre]");
    titre.textContent = `${d.papier} · ${d.resolution}`;
    titre.title = `${d.imprimante} · ${titre.textContent}`;
    const actif = d.actif;
    const [classe, mot] = actif ? VERDICTS[actif.etat] : ["aucun", "parc.verdict.aucun"];
    vue.querySelector("[data-reglage-verdict]").className = "carte carte--verdict verdict--" + classe;
    vue.querySelector("[data-verdict-trace]").setAttribute("d", TRACES[classe]);
    vue.querySelector("[data-verdict-mot]").textContent = t(mot);
    vue.querySelector("[data-verdict-contexte]").textContent = actif
      ? [t("parc.verdict.contexte"), date(actif.horodatage), t("parc.espace." + actif.espace)]
        .concat(d.ancien ? [t("parc.age.plus_d_un_an")] : []).join(" · ")
      : t("parc.verdict.aucun_contexte");

    // Un seul bouton principal ; sans profil, il est inactif et dit pourquoi, juste à côté.
    vue.querySelector("[data-installer]").disabled = !actif;
    vue.querySelector("[data-installer-raison]").hidden = Boolean(actif);
    vue.querySelector("[data-exporter-profil]").hidden = !actif;

    const historique = vue.querySelector("[data-historique]");
    historique.replaceChildren(...d.historique.map((p) => {
      const ligne = el("div", "cle-valeur mute");
      const quand = el("span", "", `${date(p.horodatage)} · ${t(p.actif ? "parc.historique.actuel" : "parc.historique.remplace")}`);
      quand.title = p.nom;
      ligne.append(quand, el("span", "", t("parc.etat." + p.etat)));
      return ligne;
    }));
    if (!d.historique.length) historique.append(el("p", "why", t("parc.historique.vide")));

    const mesures = vue.querySelector("[data-reglage-mesures]");
    const liste = el("ul", "arbre");
    liste.setAttribute("role", "tree");
    liste.append(...lignesMesures(d));
    mesures.replaceChildren(liste);
    if (!liste.children.length) mesures.append(el("p", "why", t("parc.mesures.vide")));
  }

  function section(titre, lignes) {
    const s = el("details", "section-repliable");
    s.open = true;
    const corps = el("div", "section-repliable__corps");
    corps.append(...lignes);
    s.append(el("summary", "", t(titre)), corps);
    return s;
  }

  function cleValeur(cle, valeur) {
    const l = el("div", "cle-valeur");
    const v = el("b", "", valeur);
    v.title = valeur;
    l.append(el("span", "", t(cle)), v);
    return l;
  }

  // Provenance du profil à droite : ce qui n'est pas connu est écrit « inconnu » en toutes lettres.
  function remplirDetails(d) {
    const a = d.actif;
    const provenance = a
      ? [
        cleValeur("parc.origine", t("parc.origine." + d.origine)),
        cleValeur("parc.nom", a.nom),
        cleValeur("parc.empreinte", a.empreinte.slice(0, 12)),
        cleValeur("parc.instrument", t("parc.inconnu")),
        cleValeur("parc.mire", t("parc.inconnue")),
      ]
      : [el("p", "", t("parc.verdict.aucun"))];
    details.replaceChildren(
      section("parc.provenance", provenance),
      section("parc.fiche", [cleValeur("parc.fiche.reglages", t("parc.inconnue"))]),
      section("parc.windows", [cleValeur("parc.windows.etat",
        t(!a ? "parc.windows.sans_profil" : d.installe ? "installation.installe" : "parc.windows.non_installe"))]),
    );
  }

  // ---- Actions sur le profil : la fenêtre de Windows s'ouvre côté Rust ; annuler ne change rien ----
  async function agir(bouton, faire) {
    bouton.disabled = true;
    annoncer("");
    try {
      await faire(parc.reglage());
    } catch (cle) {
      annoncer(t(String(cle)));
    }
    bouton.disabled = false;
    await parc.rafraichir();
  }

  vue.querySelector("[data-ranger-profil]").addEventListener("click", (e) => agir(e.currentTarget, async (d) => {
    const id = await invoke("parc_ranger_profil", { condition: d.condition.id, filtre: t("parc.filtre.icc") });
    if (id === null) return; // fenêtre annulée : rien ne change, aucun message
    await parc.recharger();
    annoncer(t("parc.range"));
  }));
  vue.querySelector("[data-exporter-profil]").addEventListener("click", (e) => agir(e.currentTarget, async (d) => {
    const nom = await invoke("parc_exporter_profil", { profil: d.actif.id, filtre: t("parc.filtre.icc") });
    if (nom) annoncer(`${t("export.fait")} ${nom}`);
  }));
  vue.querySelector("[data-installer]").addEventListener("click", (e) => agir(e.currentTarget, async (d) => {
    const etat = await invoke("parc_installer_profil", { profil: d.actif.id });
    annoncer(t(etat === "deja_installe" ? "parc.deja_installe" : "installation.installe"));
  }));

  return {
    brancher: (fonctions) => { parc = fonctions; },
    remplir: (d, lignesMesures) => { remplirVue(d, lignesMesures); remplirDetails(d); },
    annoncer,
  };
})();
