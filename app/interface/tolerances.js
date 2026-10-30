// Éditeur d'un jeu de tolérances (ticket #41), ouvert depuis la section « Vérifier le profil »
// (verification.js) : un nom, trois seuils ΔE00 par famille (vides permis) et le seuil hors
// gamut. « Enregistrer » range une nouvelle version, sans toucher aux précédentes. Sans aucun
// jeu enregistré, il est prérempli avec les valeurs de départ, dites « proposition, à
// confirmer » : aucun seuil n'est imposé. Les textes passent par le catalogue (app.js).
"use strict";

window.Tolerances = (() => {
  const { invoke } = window.__TAURI__.core;
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  // Valeurs de départ du jeu « Atelier » (décision du mainteneur du 28 octobre 2026).
  const PROPOSITION = {
    nom: t("verif.tolerances.nom_propose"),
    criteres: {
      fidelite: { moyenne: 2.0, p95: 4.0, max: 6.0 },
      prediction: { moyenne: 1.5, p95: 3.0, max: 5.0 },
      seuil_gamut: 1.0,
    },
  };
  const FAMILLES = [["fidelite", "verif.fidelite"], ["prediction", "verif.prediction"]];
  const CRITERES = [["moyenne", "verif.critere.moyenne"], ["p95", "verif.critere.p95"], ["max", "verif.critere.max"]];

  // Dernière version de chaque nom : la liste arrive triée par nom, puis version décroissante.
  function dernieres(jeux) {
    const vus = new Set();
    return jeux.filter((j) => !vus.has(j.nom) && vus.add(j.nom));
  }

  // « Atelier v2 », par le catalogue.
  const nomVersion = (j) => t("verif.tolerances.nom_version")
    .replace("{nom}", () => j.nom).replace("{version}", () => String(j.version));

  // Un nombre saisi : vide → aucun seuil ; virgule ou point acceptés.
  function lire(champ) {
    const texte = champ.value.trim().replace(",", ".");
    if (!texte) return null;
    const v = Number(texte);
    return Number.isFinite(v) ? v : NaN;
  }

  function saisie(valeur, libelle) {
    const champ = el("input", "input");
    champ.inputMode = "decimal";
    champ.value = valeur === null || valeur === undefined ? "" : String(valeur);
    champ.setAttribute("aria-label", libelle);
    return champ;
  }

  // Construit l'éditeur dans `cible`. `depart` : le jeu à reprendre (sa dernière version), ou
  // rien. `enregistre(jeu)` est appelé avec la version rangée ; `erreur(texte)` en cas de refus.
  function ouvrir(cible, depart, { enregistre, texteErreur }) {
    const proposition = !depart;
    const base = depart ? { nom: depart.nom, criteres: JSON.parse(depart.criteres) } : PROPOSITION;
    const corps = el("div", "carte");
    corps.append(el("span", "label label--ink", t("verif.tolerances")));
    if (proposition) corps.append(el("p", "why", t("verif.tolerances.proposition")));
    const nom = el("input", "input");
    nom.value = base.nom;
    const champNom = el("div", "field");
    champNom.append(el("span", "label", t("verif.tolerances.nom")), nom);
    corps.append(champNom);
    const champs = {};
    for (const [famille, cle] of FAMILLES) {
      const f = el("div", "field");
      f.append(el("span", "label", t(cle)));
      const ligne = el("div", "criteres");
      champs[famille] = {};
      for (const [critere, cleCritere] of CRITERES) {
        const cellule = el("label", "critere");
        const champ = saisie(base.criteres[famille][critere], `${t(cle)} · ${t(cleCritere)}`);
        champs[famille][critere] = champ;
        cellule.append(el("span", "critere__nom", t(cleCritere)), champ);
        ligne.append(cellule);
      }
      f.append(ligne);
      corps.append(f);
    }
    const gamut = saisie(base.criteres.seuil_gamut, t("verif.tolerances.seuil_gamut"));
    const champGamut = el("div", "field");
    champGamut.append(el("span", "label", t("verif.tolerances.seuil_gamut")), gamut);
    corps.append(champGamut);

    const raison = el("p", "why");
    raison.setAttribute("role", "status");
    raison.hidden = true;
    const actions = el("div", "actions");
    const bouton = el("button", "btn", t("verif.tolerances.enregistrer"));
    bouton.type = "button";
    bouton.addEventListener("click", async () => {
      const criteres = { seuil_gamut: lire(gamut) };
      for (const [famille] of FAMILLES) {
        criteres[famille] = {};
        for (const [critere] of CRITERES) criteres[famille][critere] = lire(champs[famille][critere]);
      }
      // Un seuil illisible n'est jamais envoyé : il deviendrait « aucun seuil » en JSON.
      const valeurs = [criteres.seuil_gamut, ...FAMILLES.flatMap(([f]) => CRITERES.map(([c]) => criteres[f][c]))];
      if (criteres.seuil_gamut === null || valeurs.some((v) => Number.isNaN(v))) {
        raison.textContent = t("verif.tolerances.seuil_invalide");
        raison.hidden = false;
        return;
      }
      bouton.disabled = true;
      try {
        const jeu = await invoke("tolerances_enregistrer", { nom: nom.value, criteres });
        await enregistre(jeu, t("verif.tolerances.enregistre").replace("{jeu}", () => nomVersion(jeu)));
      } catch (erreur) {
        raison.textContent = texteErreur(erreur);
        raison.hidden = false;
      }
      bouton.disabled = false;
    });
    actions.append(bouton);
    corps.append(actions, raison);
    cible.replaceChildren(corps);
  }

  return { ouvrir, dernieres, nomVersion };
})();
