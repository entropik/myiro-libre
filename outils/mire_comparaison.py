#!/usr/bin/env python3
"""Mire de comparaison MYIRO-1 / FD-9 : un PDF CMJN à imprimer et la liste de ses plages.

Sans dépendance externe (Python 3 seul). Usage :
    python outils/mire_comparaison.py [dossier]      (par défaut : .scratch/mires/)

La mire sert à mesurer les mêmes plages avec les deux instruments et à comparer les
résultats. La mise en page respecte les contraintes des deux manuels :
- FD-9 (mire libre de FD-S2w) : plages de 6 à 30 mm, marges ≥ 23 mm en tête et ≥ 33 mm
  en pied, ≥ 4 mm sur les côtés, ≥ 8 mm de blanc entre zones de plages, pas de texte à
  moins de 2 mm d'une zone, contraste suffisant entre plages voisines ;
- MYIRO-1 : ouverture de 3,5 mm (mesure ponctuelle) ; en bande, plages ≥ 7 mm,
  ΔE*ab > 10 entre voisines, 10 mm de blanc papier aux deux bouts, 257 mm au plus.
Chaque rangée est donc aussi une bande lisible par le MYIRO-1.
"""
import pathlib
import sys

MM = 72 / 25.4
LARGEUR, HAUTEUR = 297 * MM, 210 * MM  # A4 paysage
COTE = 20  # mm, côté d'une plage
ECART_RANGEES = 15  # mm de blanc entre deux rangées

# (nom, C, M, J, N) en %. Voisines choisies très différentes, pour le FD-9 comme pour la bande.
# Pas de plage de papier nu, et des plages soutenues aux deux bouts de chaque rangée : une
# plage trop claire contre le blanc qui entoure la mire ne serait pas reconnue comme plage.
# Le blanc papier se mesure à part, en mesure ponctuelle dans la marge.
RANGEES = [
    [("C100", 100, 0, 0, 0), ("J100", 0, 0, 100, 0), ("M100", 0, 100, 0, 0),
     ("N100", 0, 0, 0, 100), ("CMJ30", 30, 30, 30, 0), ("R", 0, 100, 100, 0),
     ("V", 100, 0, 100, 0), ("B", 100, 100, 0, 0), ("C50", 50, 0, 0, 0),
     ("J50", 0, 0, 50, 0), ("M50", 0, 50, 0, 0), ("N50", 0, 0, 0, 50)],
    [("N55", 0, 0, 0, 55), ("N5", 0, 0, 0, 5), ("N65", 0, 0, 0, 65),
     ("N15", 0, 0, 0, 15), ("N75", 0, 0, 0, 75), ("N25", 0, 0, 0, 25),
     ("N85", 0, 0, 0, 85), ("N35", 0, 0, 0, 35), ("N95", 0, 0, 0, 95),
     ("N45", 0, 0, 0, 45), ("N10", 0, 0, 0, 10), ("N100", 0, 0, 0, 100)],
    [("M70J70", 0, 70, 70, 0), ("C20", 20, 0, 0, 0), ("J30", 0, 0, 30, 0),
     ("C70M50", 70, 50, 0, 0), ("M30", 0, 30, 0, 0), ("C40J80", 40, 0, 80, 0),
     ("N15", 0, 0, 0, 15), ("CMJN60", 60, 60, 60, 60), ("J80M10", 0, 10, 80, 0),
     ("C10M40", 10, 40, 0, 0), ("M10J15", 0, 10, 15, 0), ("C100M70N20", 100, 70, 0, 20)],
]


def contenu_page():
    largeur_rangee = len(RANGEES[0]) * COTE
    hauteur_bloc = len(RANGEES) * COTE + (len(RANGEES) - 1) * ECART_RANGEES
    x0 = (297 - largeur_rangee) / 2
    y_haut = (210 + hauteur_bloc) / 2  # bloc centré : marges de 60 mm en tête et en pied
    ops = []
    for r, rangee in enumerate(RANGEES):
        y = y_haut - COTE - r * (COTE + ECART_RANGEES)
        for c, (_, cy, ma, ja, no) in enumerate(rangee):
            x = x0 + c * COTE
            ops.append(f"{cy/100:.2f} {ma/100:.2f} {ja/100:.2f} {no/100:.2f} k "
                       f"{x*MM:.2f} {y*MM:.2f} {COTE*MM:.2f} {COTE*MM:.2f} re f")
        # Numéro de rangée dans la marge gauche, à plus de 2 mm des plages.
        ops.append(f"0 0 0 1 k BT /F1 9 Tf {(x0-9)*MM:.2f} {(y+COTE/2-1.5)*MM:.2f} Td ({r+1}) Tj ET")
    ops.append(f"0 0 0 1 k BT /F1 8 Tf {x0*MM:.2f} {200*MM:.2f} Td "
               "(myiro-libre - mire de comparaison MYIRO-1 / FD-9 - v1 - imprimer a 100 %, sans "
               "mise a l'echelle - plages de 20 mm) Tj ET")
    return "\n".join(ops).encode("ascii")


def ecrire_pdf(chemin):
    flux = contenu_page()
    objets = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {LARGEUR:.2f} {HAUTEUR:.2f}] "
         "/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>").encode("ascii"),
        b"<< /Length " + str(len(flux)).encode() + b" >>\nstream\n" + flux + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    sortie = bytearray(b"%PDF-1.4\n")
    positions = []
    for numero, objet in enumerate(objets, 1):
        positions.append(len(sortie))
        sortie += f"{numero} 0 obj\n".encode() + objet + b"\nendobj\n"
    debut_xref = len(sortie)
    sortie += f"xref\n0 {len(objets)+1}\n0000000000 65535 f \n".encode()
    for p in positions:
        sortie += f"{p:010d} 00000 n \n".encode()
    sortie += f"trailer\n<< /Size {len(objets)+1} /Root 1 0 R >>\nstartxref\n{debut_xref}\n%%EOF\n".encode()
    chemin.write_bytes(bytes(sortie))


def ecrire_liste(chemin):
    lignes = ["rangee;colonne;nom;C;M;J;N"]
    for r, rangee in enumerate(RANGEES, 1):
        for c, (nom, cy, ma, ja, no) in enumerate(rangee, 1):
            lignes.append(f"{r};{c};{nom};{cy};{ma};{ja};{no}")
    chemin.write_text("\n".join(lignes) + "\n", encoding="utf-8")


def main():
    racine = pathlib.Path(__file__).resolve().parent.parent
    dossier = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else racine / ".scratch" / "mires"
    dossier.mkdir(parents=True, exist_ok=True)
    ecrire_pdf(dossier / "mire-comparaison-v1.pdf")
    ecrire_liste(dossier / "mire-comparaison-v1.csv")
    print(f"Mire écrite dans {dossier}")


if __name__ == "__main__":
    main()
