//! Styles intégrés du rapport : noir et gris, mise en page A4, lisible
//! photocopié. Aucune ressource externe.

pub(super) const STYLE: &str = "@page{size:A4;margin:15mm}\
body{font-family:Georgia,'Times New Roman',serif;color:#000;background:#fff;font-size:10.5pt;line-height:1.35;margin:0 auto;max-width:180mm}\
h1{font-size:17pt;margin:0 0 2pt}h2{font-size:12.5pt;border-bottom:1px solid #000;margin:14pt 0 4pt}\
h3{font-size:11pt;margin:9pt 0 3pt}.sous{color:#444;margin:0 0 8pt}\
.verdict{font-size:15pt;font-weight:bold;border:2px solid #000;padding:6pt 9pt;margin:8pt 0}\
table{border-collapse:collapse;width:100%;margin:3pt 0}th,td{border:1px solid #888;padding:2pt 4pt;text-align:left;vertical-align:top}\
th{background:#e6e6e6}td.n{text-align:right;font-variant-numeric:tabular-nums}\
pre{white-space:pre-wrap;background:#f2f2f2;padding:4pt;font-size:9pt}.note{color:#444}\
.plages{font-size:8.5pt}";
