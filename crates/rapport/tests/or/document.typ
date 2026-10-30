// Gabarit des rapports de myiro-libre (crate rapport, ADR 0008).
// Sobre : noir et gris, filets fins, ni arrondi ni ombre. Les textes du
// document arrivent toujours en chaînes : ils ne sont jamais lus comme du
// balisage. Les seules couleurs posées sont celles des paires de pastilles.

#let gris = luma(38%)
#let filet-fin = 0.5pt + luma(55%)
#let filet = 0.75pt + black

#set text(font: "Inter", size: 9pt)
#set par(leading: 0.55em, spacing: 0.9em)

// Un texte dont les sauts de ligne sont gardés.
#let multiligne(t) = t.split("\n").map(l => [#l]).join(linebreak())

#let etiquette(t) = text(size: 5.5pt, weight: 600, tracking: 0.1em, fill: gris, upper(t))

// Le cartouche d'architecte : hauteur fixe, contenu rogné, jamais sur le
// contenu de la page.
#let cartouche(a, client, objet, details, numero, titre, l) = block(width: 100%, height: 36mm, clip: true, breakable: false, {
  set text(size: 7pt)
  set par(leading: 0.45em, spacing: 0.6em)
  let numero-page = l.page.replace("{n}", str(counter(page).get().first())).replace("{total}", str(counter(page).final().first()))
  let cellules = ()
  if a.logo != none {
    cellules.push(align(center + horizon, image(a.logo, width: 24mm, height: 28mm, fit: "contain")))
  }
  let contacts = (a.courriel, a.telephone, a.site).filter(c => c != "")
  cellules.push({
    etiquette(l.atelier)
    parbreak()
    text(size: 8pt, weight: 600, a.nom)
    if a.adresse != "" { linebreak(); multiligne(a.adresse) }
    for c in contacts { linebreak(); c }
  })
  cellules.push({
    etiquette(l.client)
    parbreak()
    text(weight: 600, client)
    v(2mm)
    etiquette(l.objet)
    parbreak()
    objet
  })
  cellules.push(grid(columns: (auto, 1fr), column-gutter: 2mm, row-gutter: 1.4mm,
    ..details.map(((k, v)) => (etiquette(k), v)).flatten()))
  cellules.push({
    etiquette(l.numero)
    parbreak()
    text(size: 11pt, weight: 600, numero)
    parbreak()
    titre
    place(bottom + left, numero-page)
  })
  let colonnes = (1.25fr, 1fr, 1.15fr, 34mm)
  if a.logo != none { colonnes.insert(0, 30mm) }
  table(columns: colonnes, rows: (36mm,), inset: 2.2mm, stroke: filet, ..cellules)
})

#let titre-document(t) = block(below: 6mm, text(size: 15pt, weight: 600, t))

#let section(t) = block(above: 7mm, below: 3mm, sticky: true, width: 100%, inset: (bottom: 1.5mm), stroke: (bottom: filet-fin),
  text(size: 7.5pt, weight: 600, tracking: 0.12em, upper(t)))

#let verdict(t, detail) = block(breakable: false, width: 100%, below: 4mm, {
  text(size: 16pt, weight: 600, t)
  if detail != "" { parbreak(); text(size: 8pt, fill: gris, detail) }
})

#let paragraphe(t) = block(width: 100%, multiligne(t))

#let couples(liste) = block(breakable: false, width: 100%,
  grid(columns: (38%, 1fr), column-gutter: 3mm, row-gutter: 1.6mm,
    ..liste.map(((k, v)) => (text(fill: gris, k), multiligne(v))).flatten()))

#let tableau(entetes, lignes, nombres) = {
  set text(size: 8pt, number-width: "tabular")
  table(columns: entetes.len(), inset: (x: 2mm, y: 1.6mm),
    stroke: (x, y) => (bottom: if y == 0 { filet } else { filet-fin }),
    align: (x, y) => if nombres.at(x, default: false) { right } else { left },
    table.header(..entetes.map(e => text(weight: 600, e))),
    ..lignes.flatten())
}

#let legende(t) = text(size: 7.5pt, fill: gris, t)

#let images(liste) = block(breakable: false, width: 100%,
  grid(columns: liste.len(), column-gutter: 5mm,
    ..liste.map(((chemin, largeur, l)) => stack(spacing: 1.5mm, image(chemin, width: largeur), legende(l)))))

#let carre(c) = box(width: 9mm, height: 9mm, stroke: filet-fin, fill: if c == none { none } else { rgb(c) })

#let paires(liste) = block(breakable: false, width: 100%, {
  set text(size: 7.5pt)
  grid(columns: (1fr, 1fr, 1fr), column-gutter: 4mm, row-gutter: 3.5mm,
    ..liste.map(((c-image, c-imprime, titre, lignes)) => grid(columns: (auto, 1fr), column-gutter: 2.5mm,
      stack(dir: ltr, carre(c-image), carre(c-imprime)),
      { text(weight: 600, titre); for l in lignes { linebreak(); l } })))
})

#let graphique(chemin, l) = block(breakable: false, width: 100%, {
  layout(taille => {
    let img = image(chemin, width: taille.width)
    if measure(img).height > 110mm { image(chemin, height: 110mm) } else { img }
  })
  v(1.5mm)
  legende(l)
})

#let pre(t) = block(width: 100%, inset: 2mm, stroke: filet-fin, text(size: 7.5pt, multiligne(t)))

// ---- document ----
#set document(title: "Rapport d'image 2026-0001")
#set text(lang: "fr")
#set page(paper: "a4", margin: (top: 15mm, x: 15mm, bottom: 56mm), footer-descent: 6mm, footer: context cartouche(
  (nom: "Atelier Exemple", adresse: "12 rue des Presses\n75011 Paris", courriel: "contact@exemple.fr", telephone: "01 23 45 67 89", site: "exemple.fr", logo: none),
  "Galerie Dupont", "Tirage Paysage 60 × 40", (("Papier", "Hahnemühle Photo Rag"), ("Profil", "PRO-4100 Photo Rag v2"), ("Date", "30 octobre 2026"),), "2026-0001", "Rapport d'image",
  (page: "Page {n} / {total}", atelier: "Atelier", client: "Client", objet: "Objet", numero: "N° de rapport"),
))
#titre-document("Rapport d'image")
#section("Verdict")
#verdict("97,2 % de l'image est imprimable", "Seuil ΔE00 2,0 · colorimétrique relatif")
#section("Image")
#images((("/images/0.png", 80.0mm, "Photo"), ("/images/1.png", 80.0mm, "Surimpression"),))
#paires((("#c03020", "#a84030", "Point 1", ("ΔE00 4,1", "au-delà du seuil",)),))
#section("Chiffres")
#couples((("Image", "photo.jpg, 2048 × 1365"),))
#tableau(("Mesure", "Profil A",), (("Volume", "812 000",),), (false, true,))
#paragraphe("Les couleurs à l'écran sont approchées.")
#graphique("/images/2.svg", "Coupe")
#pre("ligne 1\nligne 2")
