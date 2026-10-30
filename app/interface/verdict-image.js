// Verdict de la section « Image » (ticket #80, demande du mainteneur du 30 octobre 2026), au-dessus
// de la photo, à la place gardée de la légende : en grand, « 97,2 % de l'image est imprimable sur
// ce papier avec ce profil » ; dessous, en petit, avec le carré vert de la surimpression, le seuil,
// le profil et le rendu. La part imprimable est celle des pixels à l'écart connu en deçà du seuil
// (code d'écart < round(10 × seuil), la règle des chiffres), sur les pixels à l'écart connu ; les
// pixels à l'écart inconnu sont dits à part, jamais comptés comme bons. Elle s'écrit à 0,1 % près,
// jamais « 100 » si un seul pixel connu dépasse le seuil, ni « 0 » si un seul est bon. Pendant une
// analyse relancée sur place, l'ancien verdict reste, atténué. La taille du bloc ne change jamais.
"use strict";

window.VerdictImage = (() => {
  const t = (cle) => textes[cle] || cle;
  const nombre = (v, decimales) => window.Graphiques.nombre(v, decimales);
  const { el } = window.Arbre;

  // Part de `n` pixels sur `total`, en % à 0,1 près : « plus de 99,9 » s'il en manque au moins un,
  // « moins de 0,1 » s'il y en a au moins un.
  function pourcent(n, total) {
    const p = (n / total) * 100;
    if (n < total && p >= 99.95) return t("image.verdict.plus_de").replace("{p}", () => nombre(99.9, 1));
    if (n > 0 && p < 0.05) return t("image.verdict.moins_de").replace("{p}", () => nombre(0.1, 1));
    return nombre(p, 1);
  }

  // Pixels imprimables (à l'écart connu, en deçà du seuil) et pixels à l'écart connu ; null si aucun.
  function imprimables(a, seuil) {
    const connus = a.pixels_comptes - a.pixels_inconnus;
    if (!connus) return null;
    const au_dela = a.histogramme.slice(Math.round(seuil * 10)).reduce((s, v) => s + v, 0);
    return { bons: connus - au_dela, connus };
  }

  // Le bloc : la phrase du verdict, puis le carré vert et la ligne du seuil, du profil et du rendu.
  function creer() {
    const bloc = el("div", "verdict");
    const part = el("p", "verdict__part");
    part.setAttribute("aria-live", "polite");
    const detail = el("p", "legende-alerte verdict__detail");
    const carre = el("span", "swatch");
    carre.style.setProperty("--c", "var(--couleur-hors-gamut)");
    carre.setAttribute("aria-hidden", "true");
    detail.append(carre, el("span", ""));
    bloc.append(part, detail);
    return bloc;
  }

  // `a` : analyse du profil de la surimpression (null si inconnue) ; `compense` : le rendu de
  // l'analyse affichée.
  function ecrire(bloc, a, seuil, nom, compense) {
    if (!bloc) return;
    const [part, detail] = bloc.children;
    const n = a ? imprimables(a, seuil) : null;
    let phrase = n ? t("image.verdict").replace("{p}", () => pourcent(n.bons, n.connus)) : t("image.verdict.inconnu");
    if (a && a.pixels_inconnus) {
      phrase += t("image.verdict.inconnus").replace("{p}", () => pourcent(a.pixels_inconnus, a.pixels_comptes));
    }
    part.textContent = phrase;
    part.title = phrase;
    detail.lastChild.textContent = t(compense ? "image.legende.point_noir" : "image.legende")
      .replace(/\{s\}/g, () => nombre(seuil, 1)).replace("{profil}", () => nom);
  }

  // Pendant une analyse relancée sur place, l'ancien verdict reste, atténué.
  function perime(bloc, oui) {
    if (bloc) bloc.classList.toggle("verdict--perime", oui);
  }

  return { creer, ecrire, perime, pourcent, imprimables };
})();
