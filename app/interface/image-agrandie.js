// Image agrandie de la section « Image » (ticket #80) : le bouton « Agrandir » montre l'image en
// grand, par-dessus toute la fenêtre, dans une fenêtre modale (<dialog>). Le cadre de l'image
// (canvas de la surimpression et repère du point cliqué, pastilles-image.js) y est déplacé tel
// quel : la surimpression, le seuil et la pipette continuent d'y marcher, et le point cliqué
// s'affiche sous l'image. Fermeture par « Fermer », Échap ou un clic hors de l'image ; le cadre
// retourne alors à sa place et le focus au bouton « Agrandir ».
"use strict";

window.ImageAgrandie = (() => {
  const t = (cle) => textes[cle] || cle;
  const { el } = window.Arbre;

  let ouverte = null; // { dialogue, cadre, place, bouton } de l'image agrandie

  // Le bouton « Agrandir » d'un cadre d'image ; `lecture()` rend ce qui s'affiche sous l'image.
  function bouton(cadre, lecture) {
    const b = el("button", "btn", t("image.agrandir"));
    b.setAttribute("aria-haspopup", "dialog");
    b.addEventListener("click", () => ouvrir(cadre, lecture, b));
    return b;
  }

  function ouvrir(cadre, lecture, bouton) {
    fermer();
    const dialogue = el("dialog", "agrandie");
    dialogue.setAttribute("aria-label", t("image.agrandie.titre"));
    const barre = el("div", "agrandie__barre");
    const fermerImage = el("button", "btn", t("image.agrandie.fermer"));
    fermerImage.addEventListener("click", () => dialogue.close());
    barre.append(el("p", "why", t("image.agrandie.aide")), fermerImage);
    // Une marque garde la place du cadre dans la section, pour l'y remettre.
    const place = document.createComment("");
    cadre.before(place);
    dialogue.append(barre, cadre, lecture());
    // Un clic hors de l'image ferme ; un clic sur l'image reste à la pipette.
    dialogue.addEventListener("click", (e) => {
      if (!e.target.closest(".surimpression, .agrandie__barre, .agrandie__lecture")) dialogue.close();
    });
    // Échap ferme aussi la fenêtre (événement « close » du <dialog>).
    dialogue.addEventListener("close", rendre);
    document.body.append(dialogue);
    ouverte = { dialogue, cadre, place, bouton };
    dialogue.showModal();
    fermerImage.focus();
  }

  // Le cadre retourne à sa place, la fenêtre disparaît, le focus revient au bouton « Agrandir ».
  function rendre() {
    if (!ouverte) return;
    const { dialogue, cadre, place, bouton } = ouverte;
    ouverte = null;
    if (place.isConnected) place.replaceWith(cadre);
    else place.remove();
    dialogue.remove();
    if (bouton.isConnected) bouton.focus();
  }

  // La section se redessine (nouvelle analyse, image retirée) : l'image agrandie se ferme.
  function fermer() {
    if (!ouverte) return;
    const { dialogue } = ouverte;
    rendre();
    if (dialogue.open) dialogue.close();
  }

  return { bouton, fermer };
})();
