//! Liste blanche des graphiques SVG venus de la page (coupe, chroma par
//! teinte, chemins…). Le SVG vient de notre propre page, mais rien n'y est
//! cru sur parole : un petit analyseur de balises garde les éléments et les
//! attributs de dessin, et refuse tout le reste (script, lien, objet
//! étranger, `url(`, entité inconnue, déclaration, forme incorrecte). Un SVG
//! accepté est rendu tel quel, à l'octet près.

/// Pourquoi un SVG est refusé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusSvg {
    /// Élément hors de la liste blanche.
    Balise(String),
    /// Attribut hors de la liste blanche.
    Attribut(String),
    /// Valeur qui appelle une ressource (`url(`) ou entité inconnue.
    Valeur(String),
    /// Balise mal fermée, déclaration, commentaire ou racine autre que `svg`.
    Forme,
}

const BALISES: [&str; 13] = [
    "svg", "g", "path", "line", "polyline", "polygon", "rect", "circle", "ellipse", "text",
    "tspan", "title", "desc",
];

const ATTRIBUTS: [&str; 44] = [
    "xmlns",
    "viewBox",
    "width",
    "height",
    "class",
    "id",
    "role",
    "aria-label",
    "aria-hidden",
    "transform",
    "fill",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-dasharray",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-opacity",
    "opacity",
    "d",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "dx",
    "dy",
    "points",
    "font-size",
    "font-weight",
    "font-family",
    "text-anchor",
    "dominant-baseline",
    "style",
    "preserveAspectRatio",
    "vector-effect",
    "shape-rendering",
];

const ENTITES: [&str; 5] = ["amp", "lt", "gt", "quot", "apos"];

/// Un texte ou une valeur : pas de `<`, entités connues seulement, aucun
/// `url(`.
fn verifier_texte(t: &str) -> Result<(), RefusSvg> {
    if t.contains('<') {
        return Err(RefusSvg::Forme);
    }
    if t.to_ascii_lowercase().contains("url(") {
        return Err(RefusSvg::Valeur(t.into()));
    }
    let mut reste = t;
    while let Some(i) = reste.find('&') {
        let apres = &reste[i + 1..];
        let fin = apres.find(';').ok_or_else(|| RefusSvg::Valeur(t.into()))?;
        let nom = &apres[..fin];
        let numerique = nom
            .strip_prefix("#x")
            .map(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()))
            .or_else(|| {
                nom.strip_prefix('#')
                    .map(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
            })
            .unwrap_or(false);
        if !numerique && !ENTITES.contains(&nom) {
            return Err(RefusSvg::Valeur(t.into()));
        }
        reste = &apres[fin + 1..];
    }
    Ok(())
}

fn nom_valide(n: &str) -> bool {
    !n.is_empty()
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':' || c == '_')
}

/// Les attributs d'une balise ouvrante : `nom="valeur"` séparés par des
/// blancs, guillemets doubles seulement.
fn verifier_attributs(mut t: &str) -> Result<(), RefusSvg> {
    loop {
        t = t.trim_start();
        if t.is_empty() {
            return Ok(());
        }
        let egal = t.find('=').ok_or(RefusSvg::Forme)?;
        let nom = &t[..egal];
        if !nom_valide(nom) {
            return Err(RefusSvg::Forme);
        }
        if !ATTRIBUTS.contains(&nom) {
            return Err(RefusSvg::Attribut(nom.into()));
        }
        let valeur = t[egal + 1..].strip_prefix('"').ok_or(RefusSvg::Forme)?;
        let fin = valeur.find('"').ok_or(RefusSvg::Forme)?;
        verifier_texte(&valeur[..fin])?;
        t = &valeur[fin + 1..];
        if !(t.is_empty() || t.starts_with(char::is_whitespace)) {
            return Err(RefusSvg::Forme);
        }
    }
}

/// Le SVG tel quel s'il ne contient que des éléments et des attributs de
/// dessin, bien imbriqués sous une seule racine `svg`.
pub fn nettoyer_svg(svg: &str) -> Result<String, RefusSvg> {
    let mut pile: Vec<&str> = Vec::new();
    let mut racines = 0;
    let mut reste = svg.trim();
    if !reste.starts_with("<svg") {
        return Err(RefusSvg::Forme);
    }
    while !reste.is_empty() {
        let debut = reste.find('<').unwrap_or(reste.len());
        verifier_texte(&reste[..debut])?;
        if debut > 0 && pile.is_empty() {
            return Err(RefusSvg::Forme);
        }
        reste = &reste[debut..];
        if reste.is_empty() {
            break;
        }
        let fin = reste.find('>').ok_or(RefusSvg::Forme)?;
        let balise = &reste[1..fin];
        reste = &reste[fin + 1..];
        if let Some(nom) = balise.strip_prefix('/') {
            if pile.pop() != Some(nom.trim_end()) {
                return Err(RefusSvg::Forme);
            }
            continue;
        }
        let (balise, vide) = match balise.strip_suffix('/') {
            Some(b) => (b, true),
            None => (balise, false),
        };
        let nom_fin = balise.find(char::is_whitespace).unwrap_or(balise.len());
        let nom = &balise[..nom_fin];
        if !nom_valide(nom) {
            return Err(RefusSvg::Forme);
        }
        if !BALISES.contains(&nom) {
            return Err(RefusSvg::Balise(nom.into()));
        }
        if pile.is_empty() {
            racines += 1;
            if nom != "svg" || racines > 1 {
                return Err(RefusSvg::Forme);
            }
        }
        verifier_attributs(&balise[nom_fin..])?;
        if !vide {
            pile.push(nom);
        }
    }
    if !pile.is_empty() || racines != 1 {
        return Err(RefusSvg::Forme);
    }
    Ok(svg.to_string())
}
