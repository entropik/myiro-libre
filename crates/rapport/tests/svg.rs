//! Liste blanche des graphiques SVG venus de la page.

use rapport::{nettoyer_svg, RefusSvg};

const GRAPHIQUE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50" width="100" height="50" class="graphique" role="img" aria-label="Coupe"><title>Coupe</title><g transform="translate(1 2)" fill="none" stroke="#333" stroke-width="0.5"><path d="M0 0 L10 10Z" stroke-dasharray="2 1"/><line x1="0" y1="0" x2="5" y2="5"/><polyline points="0,0 1,1"/><polygon points="0,0 1,1 2,0" fill-opacity="0.4"/><rect x="1" y="1" width="2" height="2"/><circle cx="3" cy="3" r="1" opacity="0.5"/><text x="2" y="9" font-size="4" text-anchor="middle">a &amp; b<tspan dy="1">c</tspan></text></g></svg>"##;

#[test]
fn un_svg_de_graphique_passe_tel_quel() {
    assert_eq!(nettoyer_svg(GRAPHIQUE).as_deref(), Ok(GRAPHIQUE));
}

#[test]
fn un_script_ou_un_lien_dans_un_svg_est_refuse() {
    for piege in [
        r#"<svg><script>alert(1)</script></svg>"#,
        r#"<svg><rect onclick="x()"/></svg>"#,
        r#"<svg><a href="https://x"><rect/></a></svg>"#,
        r#"<svg><image href="file:///c:/x.png"/></svg>"#,
        r##"<svg><use xlink:href="#a"/></svg>"##,
        r#"<svg><foreignObject><p>x</p></foreignObject></svg>"#,
        r#"<svg><rect fill="url(https://x)"/></svg>"#,
        r##"<svg><rect style="fill:url(#a)"/></svg>"##,
        r#"<svg><!DOCTYPE x [<!ENTITY a "b">]><rect/></svg>"#,
        r#"<?xml-stylesheet href="x"?><svg/>"#,
        r#"<svg><rect x="1" y="2"</svg>"#,
        r#"<svg><rect x='1'/></svg>"#,
        r#"<svg><rect x="&ext;"/></svg>"#,
        r#"<div>x</div>"#,
        "<svg><rect/>",
        "",
    ] {
        assert!(nettoyer_svg(piege).is_err(), "accepté : {piege}");
    }
    assert!(matches!(
        nettoyer_svg("<svg><script/></svg>"),
        Err(RefusSvg::Balise(b)) if b == "script"
    ));
}

#[test]
fn un_style_sans_lien_passe() {
    let svg = r##"<svg><rect style="fill:#123456;stroke:none"/></svg>"##;
    assert_eq!(nettoyer_svg(svg).as_deref(), Ok(svg));
}
