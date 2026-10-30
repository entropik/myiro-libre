// Description de l'image ouverte, en tête de la section « Image » (ticket #80) : son nom, l'état
// de son profil (intégré, sRVB, supposé ou refusé, avec les tables écartées), et ce qui est
// approché (un pixel sur N, valeurs 16 bits ramenées à 8 bits), puis sa taille. Un profil refusé
// reçoit l'action « Traiter comme sRVB » préparée par comparer-image.js.
"use strict";

window.DescriptionImage = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;
  const { texteRefus } = window.ComparerDetails;

  function etatProfil(p) {
    if (p.etat === "embarque") return t("image.profil.embarque").replace("{nom}", () => p.nom || t("image.profil.sans_nom"));
    if (p.etat === "srvb") return t(p.suppose ? "image.profil.suppose" : "image.profil.srvb");
    return texteRefus(p);
  }

  // `resume` : ImageResume de l'image ; `traiterSrvb` : rangée d'actions montrée si le profil est refusé.
  function bloc(resume, traiterSrvb) {
    const bloc = el("div");
    const nom = el("p", "");
    nom.append(el("b", "", resume.nom));
    bloc.append(nom, el("p", resume.profil.etat === "refuse" ? "why" : "", etatProfil(resume.profil)));
    for (const e of resume.profil.ecartees || []) {
      bloc.append(el("p", "why", t("image.profil.ecartee").replace("{table}", () => e.table)
        .replace("{raison}", () => texteRefus(e))));
    }
    if (resume.profil.etat === "refuse") bloc.append(traiterSrvb);
    if (resume.pas > 1) bloc.append(el("p", "why", t("image.approche").replace("{n}", () => nombre(resume.pas * resume.pas, 0))));
    if (resume.seize_bits) bloc.append(el("p", "why", t("image.seize_bits")));
    bloc.append(el("p", "why", t("image.taille")
      .replace("{l}", () => nombre(resume.largeur, 0)).replace("{h}", () => nombre(resume.hauteur, 0))
      .replace("{lo}", () => nombre(resume.largeur_origine, 0)).replace("{ho}", () => nombre(resume.hauteur_origine, 0))));
    return bloc;
  }

  return { bloc };
})();
