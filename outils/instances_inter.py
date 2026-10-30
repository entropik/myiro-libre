#!/usr/bin/env python3
"""Polices des rapports PDF : instances statiques d'Inter pour Typst.

Typst ne lit ni le WOFF2 ni les axes variables : ce script tire de la police
de l'application (`app/interface/fontes/InterVariable.woff2`, licence SIL OFL
1.1) deux instances TTF, Regular (400) et SemiBold (600), au corps optique 14,
réduites aux écritures latines, grecques et aux signes usuels. Elles vont dans
`crates/rapport/fontes/`, avec la licence.

Usage (à la racine du dépôt), une seule fois quand la police change :
    python outils/instances_inter.py

Demande fontTools et brotli (hors du dépôt, par exemple
`pip install --target <dossier> fonttools brotli` puis `PYTHONPATH=<dossier>`).
"""

from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

RACINE = Path(__file__).resolve().parent.parent
SOURCE = RACINE / "app" / "interface" / "fontes" / "InterVariable.woff2"
DESTINATION = RACINE / "crates" / "rapport" / "fontes"
CARACTERES = (
    "U+0020-007E,U+00A0-024F,U+0300-036F,U+0370-03FF,U+2000-206F,U+20A0-20CF,"
    "U+2100-214F,U+2190-21FF,U+2200-22FF,U+25A0-25FF,U+2713-2717"
)
GRAISSES = {"Regular": 400, "SemiBold": 600}


def main() -> None:
    for nom, graisse in GRAISSES.items():
        police = instancer.instantiateVariableFont(
            TTFont(SOURCE), {"wght": graisse, "opsz": 14}, updateFontNames=True
        )
        police.flavor = None
        options = subset.Options()
        options.layout_features = ["*"]
        options.name_IDs = ["*"]
        options.name_languages = ["*"]
        options.notdef_outline = True
        reduction = subset.Subsetter(options)
        reduction.populate(unicodes=subset.parse_unicodes(CARACTERES))
        reduction.subset(police)
        police.save(DESTINATION / f"Inter-{nom}.ttf")
        print(f"Inter-{nom}.ttf écrite")


if __name__ == "__main__":
    main()
