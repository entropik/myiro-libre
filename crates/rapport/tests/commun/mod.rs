//! Document d'essai partagé par les tests de la crate.

#![allow(dead_code)]

use rapport::*;

pub fn atelier() -> Atelier {
    Atelier {
        nom: "Atelier Exemple".into(),
        adresse: "12 rue des Presses\n75011 Paris".into(),
        courriel: "contact@exemple.fr".into(),
        telephone: "01 23 45 67 89".into(),
        site: "exemple.fr".into(),
        logo_png: None,
    }
}

pub fn libelles() -> Libelles {
    Libelles {
        page: "Page {n} / {total}".into(),
        atelier: "Atelier".into(),
        client: "Client".into(),
        objet: "Objet".into(),
        numero: "N° de rapport".into(),
    }
}

pub fn document(sections: Vec<Section>) -> Document {
    Document {
        langue: "fr".into(),
        cartouche: Cartouche {
            atelier: atelier(),
            client: "Galerie Dupont".into(),
            objet: "Tirage Paysage 60 × 40".into(),
            lignes: vec![
                ("Papier".into(), "Hahnemühle Photo Rag".into()),
                ("Profil".into(), "PRO-4100 Photo Rag v2".into()),
                ("Date".into(), "30 octobre 2026".into()),
            ],
            numero: "2026-0001".into(),
            titre: "Rapport d'image".into(),
        },
        sections,
        libelles: libelles(),
    }
}

/// Un PNG valide de 1 × 1 pixel.
pub const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// Un document qui emploie chaque sorte de bloc.
pub fn document_complet() -> Document {
    document(vec![
        Section {
            titre: "Verdict".into(),
            blocs: vec![Bloc::Verdict {
                texte: "97,2 % de l'image est imprimable".into(),
                detail: "Seuil ΔE00 2,0 · colorimétrique relatif".into(),
            }],
        },
        Section {
            titre: "Image".into(),
            blocs: vec![
                Bloc::Images(vec![
                    Figure { png: PNG_1X1.to_vec(), legende: "Photo".into(), largeur_mm: 80.0 },
                    Figure { png: PNG_1X1.to_vec(), legende: "Surimpression".into(), largeur_mm: 80.0 },
                ]),
                Bloc::Paires(vec![PaireCouleurs {
                    image: "#c03020".into(),
                    imprime: "#a84030".into(),
                    titre: "Point 1".into(),
                    lignes: vec!["ΔE00 4,1".into(), "au-delà du seuil".into()],
                }]),
            ],
        },
        Section {
            titre: "Chiffres".into(),
            blocs: vec![
                Bloc::Couples(vec![("Image".into(), "photo.jpg, 2048 × 1365".into())]),
                Bloc::Tableau {
                    entetes: vec!["Mesure".into(), "Profil A".into()],
                    lignes: vec![vec!["Volume".into(), "812 000".into()]],
                    nombres: vec![false, true],
                },
                Bloc::Paragraphe("Les couleurs à l'écran sont approchées.".into()),
                Bloc::Svg {
                    svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"><rect width=\"10\" height=\"10\"/></svg>".into(),
                    legende: "Coupe".into(),
                },
                Bloc::Pre("ligne 1\nligne 2".into()),
            ],
        },
    ])
}
