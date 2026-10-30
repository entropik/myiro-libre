//! Une image ouverte pour la comparaison (ticket #80), vue par
//! l'application : méthodes d'`ImageOuverte` et de `BibliothequeOuverte`,
//! sans Tauri. Les images d'essai sont construites en mémoire et ne sont
//! jamais écrites ; les profils sont des profils d'essai de la crate `gamut`.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use app::colonne::{emplacement, BibliothequeOuverte};
use app::comparer::image::{Analyses, EtatAnalyse, EtatProfilImage, ImageOuverte};
use app::comparer::{FichiersOuverts, SourceComparaison};
use bibliotheque::IdCondition;
use gamut::ProfilEssai;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use tempfile::TempDir;

/// L = 25 + 75·(r+g+b)/3, a = 60·(r−g), b = 60·(g−b).
fn lineaire(rvb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rvb;
    [
        25.0 + 75.0 * (r + g + b) / 3.0,
        60.0 * (r - g),
        60.0 * (g - b),
    ]
}

/// Inverse exact de `lineaire`, borné au cube RVB.
fn lineaire_inverse(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let base = (l - 25.0) / 75.0;
    [
        base + a / 90.0 + b / 180.0,
        base - a / 180.0 + b / 180.0,
        base - a / 180.0 - b / 90.0,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

fn essai(inverse: Option<fn([f64; 3]) -> [f64; 3]>) -> Vec<u8> {
    gamut::profil_essai(&ProfilEssai {
        grille: 9,
        blanc: [95.0, -0.5, 2.5],
        cible: lineaire,
        inverse,
        description: "Essai".into(),
        mesures: None,
    })
}

/// PNG RVB de `largeur` × `hauteur` pixels, en mémoire.
fn png(largeur: u32, hauteur: u32, icc: Option<Vec<u8>>) -> Vec<u8> {
    let pixels: Vec<u8> = (0..largeur * hauteur)
        .flat_map(|i| {
            let v = (i * 37 % 256) as u8;
            [v, 255 - v, v / 2]
        })
        .collect();
    let mut octets = Vec::new();
    let mut encodeur = PngEncoder::new(Cursor::new(&mut octets));
    if let Some(icc) = icc {
        encodeur.set_icc_profile(icc).unwrap();
    }
    encodeur
        .write_image(&pixels, largeur, hauteur, ExtendedColorType::Rgb8)
        .unwrap();
    octets
}

struct Banc {
    dossier: TempDir,
    ouverte: BibliothequeOuverte,
    fichiers: FichiersOuverts,
    canon: IdCondition,
    image: ImageOuverte,
}

fn banc() -> Banc {
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let canon = ouverte
        .ajouter_papier(canon.id, "Photo Rag 308", "720 dpi")
        .unwrap()
        .id;
    Banc {
        dossier,
        ouverte,
        fichiers: FichiersOuverts::default(),
        canon,
        image: ImageOuverte::default(),
    }
}

impl Banc {
    /// Range un profil dans la bibliothèque (le fichier `.icc` passe par le
    /// dossier temporaire, comme un profil rangé depuis la page).
    fn ranger(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, icc).unwrap();
        let id = self
            .ouverte
            .ranger_profil(self.canon, Path::new(&chemin), "2026-10-28")
            .unwrap();
        std::fs::remove_file(&chemin).unwrap();
        SourceComparaison::Bibliotheque { profil: id.0 }
    }

    fn ouvrir_fichier(&self, nom: &str, icc: &[u8]) -> SourceComparaison {
        let chemin = self.dossier.path().join(nom);
        std::fs::write(&chemin, icc).unwrap();
        let resume = self.fichiers.ouvrir(&chemin).unwrap();
        std::fs::remove_file(&chemin).unwrap();
        SourceComparaison::Fichier {
            jeton: resume.jeton,
        }
    }

    fn analyser(&self, sources: &[SourceComparaison]) -> Result<Analyses, String> {
        self.ouverte
            .analyser_image(&self.image, sources, &self.fichiers, gamut::Rendu::Relatif)
    }
}

#[test]
fn sans_image_ouverte_l_analyse_est_refusee() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    assert_eq!(b.analyser(&[source]).unwrap_err(), "image.refus.aucune");
    assert_eq!(b.image.apercu().unwrap_err(), "image.refus.aucune");
}

#[test]
fn l_apercu_a_largeur_fois_hauteur_fois_4_octets() {
    let b = banc();
    let resume = b
        .image
        .ouvrir_octets("photo.png", &png(5, 3, None))
        .unwrap();
    assert_eq!((resume.largeur, resume.hauteur), (5, 3));
    assert_eq!((resume.largeur_origine, resume.hauteur_origine), (5, 3));
    assert_eq!(resume.pas, 1);
    assert!(!resume.seize_bits);
    assert_eq!(resume.nom, "photo.png");
    assert_eq!(resume.profil, EtatProfilImage::Srvb { suppose: false });
    let apercu = b.image.apercu().unwrap();
    assert_eq!(apercu.len(), 5 * 3 * 4);
    // Deuxième pixel de `png` : v = 37.
    assert_eq!(&apercu[4..8], &[37, 218, 18, 255]);
}

#[test]
fn chaque_profil_affiche_a_son_analyse() {
    let b = banc();
    let rangee = b.ranger("rangee.icc", &essai(Some(lineaire_inverse)));
    let fichier = b.ouvrir_fichier("fichier.icc", &essai(Some(lineaire_inverse)));
    b.image
        .ouvrir_octets("photo.png", &png(6, 4, None))
        .unwrap();
    let analyses = b.analyser(&[fichier, rangee]).unwrap();
    assert_eq!(
        analyses
            .profils
            .iter()
            .map(|a| a.source)
            .collect::<Vec<_>>(),
        vec![fichier, rangee]
    );
    for (i, a) in analyses.profils.iter().enumerate() {
        let EtatAnalyse::Calculee { analyse } = &a.etat else {
            panic!("profil {i} non calculé : {:?}", a.etat);
        };
        assert_eq!(analyse.pixels_comptes, 24);
        assert_eq!(b.image.ecarts(analyses.numero, i).unwrap().len(), 24);
    }
    assert!(b.image.ecarts(analyses.numero, 2).is_err());
}

#[test]
fn un_profil_sans_b2a1_est_inconnu_sans_bloquer_les_autres() {
    let b = banc();
    let sans = b.ranger("sans.icc", &essai(None));
    let avec = b.ouvrir_fichier("avec.icc", &essai(Some(lineaire_inverse)));
    b.image
        .ouvrir_octets("photo.png", &png(4, 4, None))
        .unwrap();
    let analyses = b.analyser(&[sans, avec]).unwrap();
    assert_eq!(analyses.profils.len(), 2);
    match &analyses.profils[0].etat {
        EtatAnalyse::Inconnue { cle, detail } => {
            assert_eq!(*cle, "comparer.refus.sans_table_inverse");
            assert_eq!(*detail, None);
        }
        autre => panic!("profil sans B2A1 calculé : {autre:?}"),
    }
    assert!(matches!(
        analyses.profils[1].etat,
        EtatAnalyse::Calculee { .. }
    ));
    let n = analyses.numero;
    assert_eq!(b.image.ecarts(n, 0).unwrap_err(), "image.refus.sans_ecarts");
    assert_eq!(b.image.ecarts(n, 1).unwrap().len(), 16);
}

#[test]
fn un_profil_embarque_illisible_n_est_jamais_remplace_en_silence() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    let mut tronque = essai(None);
    tronque.truncate(300);
    let resume = b
        .image
        .ouvrir_octets("photo.png", &png(4, 4, Some(tronque)))
        .unwrap();
    let EtatProfilImage::Refuse { cle, .. } = resume.profil else {
        panic!("profil tronqué accepté : {:?}", resume.profil);
    };
    assert!(cle.starts_with("comparer.image.refus."), "{cle}");
    assert_eq!(b.analyser(&[source]).unwrap_err(), "image.refus.profil");

    let resume = b.image.traiter_comme_srvb().unwrap();
    assert_eq!(resume.profil, EtatProfilImage::Srvb { suppose: true });
    assert!(matches!(
        b.analyser(&[source]).unwrap().profils[0].etat,
        EtatAnalyse::Calculee { .. }
    ));
}

#[test]
fn un_profil_embarque_lisible_n_est_pas_remplace_par_srvb() {
    let b = banc();
    let resume = b
        .image
        .ouvrir_octets("photo.png", &png(2, 2, Some(essai(None))))
        .unwrap();
    let attendu = EtatProfilImage::Embarque {
        nom: "Essai".into(),
        ecartees: vec![],
    };
    assert_eq!(resume.profil, attendu);
    assert_eq!(b.image.traiter_comme_srvb().unwrap().profil, attendu);
}

/// Tous les fichiers sous `dossier`, avec leur taille.
fn fichiers_sous(dossier: &Path) -> Vec<(PathBuf, u64)> {
    let mut liste = Vec::new();
    for entree in std::fs::read_dir(dossier).unwrap() {
        let entree = entree.unwrap();
        let chemin = entree.path();
        if chemin.is_dir() {
            liste.extend(fichiers_sous(&chemin));
        } else {
            liste.push((chemin, entree.metadata().unwrap().len()));
        }
    }
    liste.sort();
    liste
}

#[test]
fn une_image_ouverte_n_est_ni_rangee_ni_ecrite() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    let avant = fichiers_sous(b.dossier.path());
    let profils = b.ouverte.profils_comparables().unwrap();
    b.image
        .ouvrir_octets("photo.png", &png(8, 8, None))
        .unwrap();
    b.analyser(&[source]).unwrap();
    b.image.fermer();
    assert_eq!(fichiers_sous(b.dossier.path()), avant);
    assert_eq!(b.ouverte.profils_comparables().unwrap(), profils);
    assert_eq!(b.image.apercu().unwrap_err(), "image.refus.aucune");
}

#[test]
fn ouvrir_une_image_remplace_la_precedente() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    b.image.ouvrir_octets("une.png", &png(4, 4, None)).unwrap();
    let premiere = b.analyser(&[source]).unwrap();
    let resume = b.image.ouvrir_octets("deux.png", &png(3, 2, None)).unwrap();
    assert_eq!(resume.nom, "deux.png");
    assert_eq!(b.image.apercu().unwrap().len(), 3 * 2 * 4);
    // Les écarts de la première image ne valent pas pour la seconde.
    assert_eq!(
        b.image.ecarts(premiere.numero, 0).unwrap_err(),
        "image.refus.sans_ecarts"
    );
}

#[test]
fn un_fichier_qui_n_est_pas_une_image_est_refuse() {
    let b = banc();
    assert_eq!(
        b.image
            .ouvrir_octets("notes.png", b"pas une image")
            .unwrap_err(),
        "image.refus.illisible"
    );
    assert_eq!(
        b.image
            .ouvrir(&b.dossier.path().join("absente.png"))
            .unwrap_err(),
        "image.refus.illisible"
    );
}

#[test]
fn les_textes_de_l_image_sont_au_catalogue() {
    use app::comparer::image::CLES_IMAGE;
    use app::textes::{texte, Langue};
    use photo::RefusPhoto;
    let refus_photo = [
        RefusPhoto::Illisible(String::new()),
        RefusPhoto::FormatNonPrisEnCharge,
        RefusPhoto::Cmjn,
        RefusPhoto::TropGrande { pixels: 0 },
    ];
    let page = [
        "image.ouvrir",
        "image.filtre",
        "image.traiter_srvb",
        "image.profil.embarque",
        "image.profil.sans_nom",
        "image.profil.ecartee",
        "image.profil.srvb",
        "image.profil.suppose",
        "image.approche",
        "image.seize_bits",
    ];
    let cles = CLES_IMAGE
        .iter()
        .copied()
        .chain(refus_photo.iter().map(RefusPhoto::cle))
        .chain(page);
    for cle in cles {
        for langue in [Langue::Francais, Langue::Anglais] {
            assert_ne!(texte(langue, cle), cle, "{cle} manque");
        }
    }
    let profil = texte(Langue::Francais, "image.refus.profil");
    let bouton = texte(Langue::Francais, "image.traiter_srvb");
    assert!(
        profil.contains(&format!("«\u{202f}{bouton}\u{202f}»")),
        "{profil}"
    );
}

/// CRC-32 d'un bloc PNG (type puis données).
fn crc_png(octets: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &octet in octets {
        crc ^= u32::from(octet);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Le PNG de `png`, avec un bloc iCCP dont le profil n'est pas du zlib : le
/// décodeur ne rend aucun profil, la crate `photo` le dit illisible.
fn png_au_profil_abime(largeur: u32, hauteur: u32) -> Vec<u8> {
    let png = png(largeur, hauteur, None);
    let mut corps = b"iCCP".to_vec();
    corps.extend(b"essai\0\0pas du zlib");
    let mut bloc = ((corps.len() - 4) as u32).to_be_bytes().to_vec();
    bloc.extend(&corps);
    bloc.extend(crc_png(&corps).to_be_bytes());
    let apres_ihdr = 8 + 25;
    let mut sortie = png[..apres_ihdr].to_vec();
    sortie.extend(bloc);
    sortie.extend(&png[apres_ihdr..]);
    sortie
}

#[test]
fn un_profil_embarque_que_le_decodeur_ne_lit_pas_est_refuse_et_non_pris_pour_srvb() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    let resume = b
        .image
        .ouvrir_octets("photo.png", &png_au_profil_abime(4, 3))
        .unwrap();
    let EtatProfilImage::Refuse { cle, detail } = &resume.profil else {
        panic!("profil abîmé pris pour sRVB : {:?}", resume.profil);
    };
    assert_eq!(*cle, "comparer.image.refus.illisible");
    assert!(detail.is_some());
    assert_eq!(b.analyser(&[source]).unwrap_err(), "image.refus.profil");

    let resume = b.image.traiter_comme_srvb().unwrap();
    assert_eq!(resume.profil, EtatProfilImage::Srvb { suppose: true });
    assert!(matches!(
        b.analyser(&[source]).unwrap().profils[0].etat,
        EtatAnalyse::Calculee { .. }
    ));
}

/// PNG de `cote` × `cote` pixels tous de couleurs différentes : l'analyse
/// passe plusieurs fois par le contrôle d'annulation (toutes les 65 536
/// couleurs distinctes) et dure assez pour être interrompue.
fn png_varie(cote: u32) -> Vec<u8> {
    let pixels: Vec<u8> = (0..cote * cote)
        .flat_map(|i| [(i & 0xff) as u8, (i >> 8 & 0xff) as u8, (i >> 16) as u8])
        .collect();
    let mut octets = Vec::new();
    PngEncoder::new(Cursor::new(&mut octets))
        .write_image(&pixels, cote, cote, ExtendedColorType::Rgb8)
        .unwrap();
    octets
}

/// Lance `analyse` dans un autre fil et répète `pendant` tant qu'elle
/// tourne : au moins une répétition tombe pendant le calcul.
fn pendant_l_analyse<T: Send>(analyse: impl FnOnce() -> T + Send, pendant: impl Fn()) -> T {
    std::thread::scope(|s| {
        let fil = s.spawn(analyse);
        while !fil.is_finished() {
            pendant();
            std::thread::yield_now();
        }
        fil.join().unwrap()
    })
}

#[test]
fn annuler_pendant_le_calcul_arrete_l_analyse_et_le_dit() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    b.image
        .ouvrir_octets("grande.png", &png_varie(512))
        .unwrap();
    let resultat = pendant_l_analyse(|| b.analyser(&[source]), || b.image.annuler_analyse());
    assert_eq!(resultat.unwrap_err(), "image.annulee");
    // L'annulation ne valait que pour cette analyse-là.
    let suivante = b.analyser(&[source]).unwrap();
    assert_eq!(b.image.ecarts(suivante.numero, 0).unwrap().len(), 512 * 512);
}

#[test]
fn une_nouvelle_analyse_annule_la_precedente() {
    let b = banc();
    let a = b.ranger("a.icc", &essai(Some(lineaire_inverse)));
    let autre = b.ouvrir_fichier("autre.icc", &essai(Some(lineaire_inverse)));
    b.image
        .ouvrir_octets("grande.png", &png_varie(512))
        .unwrap();
    let derniere = std::sync::Mutex::new(None);
    let premiere = pendant_l_analyse(
        || b.analyser(&[a]),
        || {
            if let Ok(r) = b.analyser(&[a, autre]) {
                *derniere.lock().unwrap() = Some(r);
            }
        },
    );
    assert_eq!(premiere.unwrap_err(), "image.annulee");
    let derniere = derniere
        .into_inner()
        .unwrap()
        .expect("une analyse menée à terme");
    assert_eq!(derniere.profils.len(), 2);
    // Les écarts rangés sont ceux de la dernière analyse, et d'elle seule.
    assert_eq!(b.image.ecarts(derniere.numero, 1).unwrap().len(), 512 * 512);
    assert_eq!(
        b.image.ecarts(derniere.numero + 1, 0).unwrap_err(),
        "image.refus.sans_ecarts"
    );
}

#[test]
fn une_nouvelle_image_pendant_l_analyse_la_rend_caduque() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    let grande = png_varie(512);
    b.image.ouvrir_octets("une.png", &grande).unwrap();
    let resultat = pendant_l_analyse(
        || b.analyser(&[source]),
        || {
            b.image.ouvrir_octets("deux.png", &grande).unwrap();
        },
    );
    assert_eq!(resultat.unwrap_err(), "image.annulee");
}

#[test]
fn fermer_pendant_l_analyse_la_rend_caduque() {
    let b = banc();
    let source = b.ranger("avec.icc", &essai(Some(lineaire_inverse)));
    let grande = png_varie(512);
    b.image.ouvrir_octets("une.png", &grande).unwrap();
    let resultat = pendant_l_analyse(|| b.analyser(&[source]), || b.image.fermer());
    assert!(["image.annulee", "image.refus.aucune"].contains(&resultat.unwrap_err().as_str()),);
}

#[test]
fn un_fichier_de_plus_de_1_gio_est_refuse_sans_etre_lu() {
    use app::comparer::image::OCTETS_MAX_IMAGE;
    let b = banc();
    let chemin = b.dossier.path().join("enorme.tif");
    // Taille posée sans rien écrire : le fichier n'a pas de contenu.
    std::fs::File::create(&chemin)
        .unwrap()
        .set_len(OCTETS_MAX_IMAGE + 1)
        .unwrap();
    assert_eq!(
        b.image.ouvrir(&chemin).unwrap_err(),
        "image.refus.trop_grande"
    );
    std::fs::remove_file(&chemin).unwrap();
}
