// Pièces de l'arbre de la bibliothèque, sans état : nœud (nom entier au survol), marque de repli,
// âge du profil actif, lignes de mesure. Utilisées par parc.js et reglage.js. Les textes
// viennent du catalogue (`textes`, chargé par app.js).
"use strict";

window.Arbre = (() => {
  const t = (cle) => textes[cle] || cle;

  function el(nom, classe, texte) {
    const e = document.createElement(nom);
    if (classe) e.className = classe;
    if (texte !== undefined) e.textContent = texte;
    return e;
  }

  // Date et heure telles qu'elles ont été écrites, sans conversion de fuseau.
  function formater(horodatage, options) {
    const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(horodatage);
    if (!m) return horodatage;
    return new Intl.DateTimeFormat(document.documentElement.lang, { ...options, timeZone: "UTC" })
      .format(new Date(Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5])));
  }

  const date = (h) => formater(h, { day: "numeric", month: "short", year: "numeric" });
  const moisAnnee = (h) => formater(h, { month: "short", year: "numeric" });
  const dateCourte = (h) => formater(h, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
  const dateHeure = (h) => formater(h, { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });

  // Ce qui distingue un profil d'un autre du même jour (liste « Profil à vérifier » et
  // historique) : date et heure, puis sa recette s'il a été calculé.
  function reperesProfil(p) {
    const reperes = [dateHeure(p.horodatage)];
    if (p.recette) reperes.push(t("profil.recette.nom." + p.recette));
    return reperes;
  }

  function lecture(geometrie, plages) {
    const nom = t("lecture." + geometrie.lecture);
    if (geometrie.lecture === "ponctuelle") return nom;
    return `${nom}, ${plages} ${t(plages > 1 ? "lecture.plages" : "lecture.plage")}`;
  }

  // Un nœud : son nom (des points s'il est trop long, le nom entier au survol), puis à droite
  // l'âge du profil ou un compte.
  function noeud(niveau, nom, droite, donnees, choisi) {
    const li = el("li", "noeud noeud--" + niveau);
    li.setAttribute("role", "treeitem");
    li.tabIndex = 0;
    Object.assign(li.dataset, donnees);
    if (choisi) li.setAttribute("aria-selected", "true");
    const corps = el("span", "noeud__corps");
    const n = el("span", "noeud__nom", nom);
    n.title = nom;
    corps.append(n);
    li.append(corps);
    if (droite) li.append(droite);
    return li;
  }

  // Marque de repli devant le nom ; un clic sur elle replie ou déplie (`data-replier`).
  function repliable(li, cle, replie) {
    li.setAttribute("aria-expanded", String(!replie));
    const marque = el("span", "noeud__repli", replie ? "▸" : "▾");
    marque.setAttribute("aria-hidden", "true");
    marque.dataset.replier = cle;
    li.prepend(marque);
  }

  // Âge du profil actif : mois et année, signalé s'il a plus d'un an, « aucun » sinon.
  function age(r) {
    if (!r.profil) return el("span", "age age--aucun", t("parc.age.aucun"));
    const texte = moisAnnee(r.profil.horodatage);
    const a = el("span", "age " + (r.profil_ancien ? "age--ancien" : "age--recent"),
      r.profil_ancien ? t("parc.age.ancien").replace("{}", texte) : texte);
    a.title = date(r.profil.horodatage);
    return a;
  }

  // Une mesure sur deux lignes : son nom ou sa lecture, puis la date courte.
  function ligneMesure(nom, detail, donnees, choisie) {
    const li = noeud("papier", nom, null, donnees, choisie);
    const meta = el("span", "noeud__meta", detail);
    meta.title = detail;
    li.querySelector(".noeud__corps").append(meta);
    return li;
  }

  // Les mesures d'une condition, puis ses mesures importées. `choisie(cle)` : la mesure
  // ouverte dans la feuille (« mesure-3 », « importee-5 »).
  function lignesMesures(b, choisie) {
    const lignes = b.mesures.map((m) => {
      // Ligne 1 : le nom donné à la mesure (tâche Mesurer), sinon la lecture ; ligne 2 : la date
      // courte, suivie de la lecture si elle n'est pas déjà en ligne 1. Jamais la date deux fois.
      const nom = m.nom || lecture(m.geometrie, m.plages);
      const detail = m.nom ? `${dateCourte(m.horodatage)} · ${lecture(m.geometrie, m.plages)}` : dateCourte(m.horodatage);
      return ligneMesure(nom, detail, { mesure: m.id }, choisie(`mesure-${m.id}`));
    });
    for (const m of b.importees) {
      lignes.push(ligneMesure(m.fichier, t("import.importee"), { importee: m.id }, choisie(`importee-${m.id}`)));
    }
    return lignes;
  }

  return { el, date, dateHeure, reperesProfil, noeud, repliable, age, lignesMesures };
})();
