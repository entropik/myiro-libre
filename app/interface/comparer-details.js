// Textes et détails de la vue « Comparer » (ticket #71) : chiffres clés de chaque profil, méthode,
// provenance de chaque profil à droite et, sous « Avancé », la commande et le journal d'iccgamut.
// Ce qui n'est pas connu s'écrit « inconnu » (Graphiques.nombre) ou dit sa raison (clé du
// catalogue), jamais 0. Les textes viennent du catalogue (app.js).
"use strict";

window.ComparerDetails = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;
  const { carreSerie, lab, tableau } = window.ComparerEcarts;

  // Refus d'un profil : le texte de sa clé, avec la structure, l'espace ou la balise en cause.
  function texteRefus(etat) {
    return t(etat.cle).replace("{detail}", () => etat.detail || "");
  }

  function texteLimite(limite) {
    return t(limite.cle).replace("{version}", () => limite.cause || "");
  }

  function texteFidelite(f) {
    if (!f || f.etat !== "connue") return t(f ? f.cle : "comparer.fidelite.sans_mesures");
    const s = f.statistiques;
    return t("comparer.chiffres.fidelite.valeur")
      .replace("{moyenne}", () => nombre(s.moyenne, 2)).replace("{p95}", () => nombre(s.p95, 2))
      .replace("{max}", () => nombre(s.max, 2)).replace("{plages}", () => nombre(f.plages, 0));
  }

  function texteRepetabilite(r) {
    if (!r) return t("comparer.chiffres.repetabilite.inconnue");
    return t("comparer.chiffres.repetabilite.valeur")
      .replace("{moyenne}", () => nombre(r.moyenne, 2)).replace("{max}", () => nombre(r.max, 2))
      .replace("{couleurs}", () => nombre(r.couleurs, 0)).replace("{paires}", () => nombre(r.paires, 0));
  }

  // Un volume inconnu (iccgamut ne l'a pas dit) s'écrit « inconnu », sans unité.
  const volume = (v) => (Number.isFinite(v)
    ? t("comparer.valeur.volume").replace("{volume}", () => nombre(v, 0))
    : t("comparer.inconnu"));

  function cleValeur(cle, valeur) {
    const l = el("div", "cle-valeur");
    const v = el("b", "", valeur);
    v.title = valeur;
    l.append(el("span", "", t(cle)), v);
    return l;
  }

  function section(cle, ...contenu) {
    const s = el("div", "section");
    const tete = el("div", "section__head");
    tete.append(el("span", "label label--ink", t(cle)));
    s.append(tete, ...contenu);
    return s;
  }

  // ---- Chiffres clés : une colonne par profil, pour les lire côte à côte ----
  function chiffres(liste) {
    const colonne = (p) => {
      const a = p.analyse;
      return [
        lab(a.blanc),
        t("comparer.chiffres.noir.valeur")
          .replace("{l}", () => nombre(a.noir[0], 2)).replace("{d}", () => nombre(a.dmax_approche, 2)),
        volume(a.volume_surface),
        p.limite.etat === "calculee" ? volume(p.limite.volume) : t("comparer.chiffres.limite_indisponible"),
        texteFidelite(p.fidelite),
        texteRepetabilite(p.repetabilite),
      ];
    };
    const colonnes = liste.map(colonne);
    const lignes = ["blanc", "noir", "volume_surface", "volume_limite", "fidelite", "repetabilite"]
      .map((cle, i) => [t("comparer.chiffres." + cle), ...colonnes.map((c) => c[i])]);
    const entetes = liste.map((p) => {
      const nom = el("span", "");
      nom.append(carreSerie(p.serie), " ", p.nom);
      return nom;
    });
    return section("comparer.chiffres", tableau(["", ...entetes], lignes, false));
  }

  // ---- Méthode : calculé, approché, limite de la fidélité ----
  function methode() {
    const d = el("details", "replie");
    d.append(el("summary", "", t("comparer.methode.resume")),
      el("span", "label", t("comparer.methode.calcule.titre")), el("p", "prose", t("comparer.methode.calcule")),
      el("span", "label", t("comparer.methode.approche.titre")), el("p", "prose", t("comparer.methode.approche")),
      el("p", "prose", t("comparer.methode.volume")), el("p", "prose", t("comparer.methode.fidelite")));
    return d;
  }

  // ---- Zone de droite : provenance de chaque profil, puis « Avancé » ----
  function origine(p) {
    if (p.source.type === "fichier") return t("comparer.origine.fichier").replace("{nom}", () => p.origine);
    return t("comparer.origine.bibliotheque").replace("{chemin}", () => p.origine)
      .replace("{date}", () => p.date || t("comparer.inconnu"));
  }

  function provenance(p, i) {
    const s = el("details", "section-repliable");
    s.open = true;
    const contenu = el("div", "section-repliable__corps");
    const resume = el("summary", "");
    resume.append(carreSerie(i + 1), " ", p.nom);
    contenu.append(cleValeur("comparer.provenance.origine", origine(p)));
    if (p.etat.etat === "refuse") contenu.append(el("p", "", texteRefus(p.etat)));
    else {
      const a = p.etat.analyse;
      const f = p.etat.fidelite;
      contenu.append(cleValeur("comparer.provenance.version", a.version),
        cleValeur("comparer.provenance.description", a.description || t("comparer.provenance.aucune")),
        cleValeur("comparer.provenance.grille", t("comparer.provenance.grille.valeur").replace("{n}", () => nombre(a.grille, 0))));
      if (f.etat === "connue") {
        const condition = f.condition || t("comparer.provenance.condition.inconnue");
        contenu.append(cleValeur("comparer.provenance.mesures", t("comparer.provenance.mesures.valeur").replace("{n}", () => nombre(f.plages, 0))),
          cleValeur("comparer.provenance.condition", f.condition_supposee ? `${condition} (${t("comparer.provenance.supposee")})` : condition),
          cleValeur("comparer.provenance.lab", t(f.lab_calcule ? "comparer.provenance.lab_calcule" : "comparer.provenance.lab_lu")));
        if (f.rvb_supposes) contenu.append(cleValeur("comparer.provenance.rvb", t("comparer.provenance.rvb_supposes")));
      } else contenu.append(cleValeur("comparer.provenance.mesures", texteFidelite(f)));
    }
    s.append(resume, contenu);
    return s;
  }

  // Commande d'iccgamut en lecture seule, puis le journal (ou le message d'ArgyllCMS) de chaque
  // profil analysé, comme pour le calcul d'un profil.
  function avance(commande, liste) {
    const d = el("details", "replie");
    d.append(el("summary", "", t("comparer.avance")), el("span", "label", t("comparer.avance.commande")),
      el("pre", "", commande || t("comparer.avance.sans_commande")));
    for (const p of liste) {
      const l = p.limite;
      const texte = l.etat === "calculee" ? l.journal : l.cause;
      if (!texte) continue;
      const cle = l.etat === "calculee" ? "comparer.avance.journal" : "comparer.avance.cause";
      d.append(el("span", "label", t(cle).replace("{nom}", () => p.nom)), el("pre", "", texte));
    }
    return d;
  }

  return { texteRefus, texteLimite, texteFidelite, section, chiffres, methode, provenance, avance };
})();
