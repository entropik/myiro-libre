//! Pondérations spectrales des densités par réflexion : statuts E et T,
//! densité visuelle ISO.
//!
//! Reprises d'ArgyllCMS 3.2.0, fichier `xicc/xspect.c` (tables `denT` et
//! `denE`), Copyright 2000 - 2006 Graeme W. Gill, sous licence GNU GPL
//! version 2 ou ultérieure : reprises ici sous la GPL-3.0 de myiro-libre.
//! Source lue dans le miroir non officiel
//! <https://github.com/CursedHardware/argyllcms-unofficial-mirror>, étiquette
//! `v3.2.0`, commit 69b069d61c8f36a8d992e3fc6e1dc35053da8584 (le site
//! argyllcms.com ne répondait pas le 9 octobre 2026).
//!
//! Provenance des chiffres : tables de l'ISO 5-3 **supposées**, non vérifiées
//! (ArgyllCMS ne cite pas sa source, la norme est payante). Voir
//! `docs/references/icc-densite.md`.
//!
//! Chaque table donne le logarithme décimal de la pondération, tous les
//! 10 nm à partir de `debut_nm` ; le maximum vaut 5 (pondération 100 000) et
//! −10 marque une pondération nulle en pratique. Valeurs reprises telles
//! quelles.

/// Pondération d'un filtre, en logarithme décimal, tous les 10 nm.
pub(crate) struct Ponderation {
    pub debut_nm: f64,
    pub log10: &'static [f64],
}

/// Statut T, filtre rouge (densité du cyan) : 340 à 770 nm.
pub(crate) const T_ROUGE: Ponderation = Ponderation {
    debut_nm: 340.0,
    log10: &[
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, 0.500, 1.778, //
        2.653, 4.477, 5.000, 4.929, 4.740, 4.398, //
        4.000, 3.699, 3.176, 2.699, 2.477, 2.176, //
        1.699, 1.000, 0.500, -10.0, -10.0, -10.0, //
        -10.0, -10.0,
    ],
};

/// Statut T, filtre vert (densité du magenta) : 340 à 770 nm.
pub(crate) const T_VERT: Ponderation = Ponderation {
    debut_nm: 340.0,
    log10: &[
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, 0.500, 3.000, 3.699, 4.447, 4.833, //
        4.964, 5.000, 4.944, 4.820, 4.623, 4.342, //
        3.954, 3.398, 2.845, 1.954, 1.000, 0.500, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0,
    ],
};

/// Statut T, filtre bleu (densité du jaune) : 340 à 770 nm.
pub(crate) const T_BLEU: Ponderation = Ponderation {
    debut_nm: 340.0,
    log10: &[
        0.500, 1.000, 1.301, 2.000, 2.477, 3.176, //
        3.778, 4.230, 4.602, 4.778, 4.914, 4.973, //
        5.000, 4.987, 4.929, 4.813, 4.602, 4.255, //
        3.699, 2.301, 1.602, 0.500, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0,
    ],
};

/// Densité visuelle ISO, commune aux statuts E et T : 340 à 770 nm. À
/// 440 nm, 2,881 (pondération 760) : python-colormath donne 650 ; on garde
/// la valeur d'ArgyllCMS, celle de `spotread -d`.
pub(crate) const VISUELLE_ISO: Ponderation = Ponderation {
    debut_nm: 340.0,
    log10: &[
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        0.500, 1.332, 1.914, 2.447, 2.881, 3.090, //
        3.346, 3.582, 3.818, 4.041, 4.276, 4.513, //
        4.702, 4.825, 4.905, 4.957, 4.989, 5.000, //
        4.989, 4.956, 4.902, 4.827, 4.731, 4.593, //
        4.433, 4.238, 4.013, 3.749, 3.490, 3.188, //
        2.901, 2.622, 2.334, 2.041, 1.732, 1.431, //
        1.146, 0.500,
    ],
};

/// Statut E, filtre rouge (densité du cyan) : 370 à 720 nm.
pub(crate) const E_ROUGE: Ponderation = Ponderation {
    debut_nm: 370.0,
    log10: &[
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, 0.500, 1.778, 2.653, 4.477, 5.000, //
        4.929, 4.740, 4.398, 4.000, 3.699, 3.176, //
        2.699, 2.477, 2.176, 1.699, 1.000, 0.500,
    ],
};

/// Statut E, filtre vert (densité du magenta) : 370 à 720 nm. La table
/// d'ArgyllCMS annonce 36 valeurs mais en écrit 47 : les 11 dernières
/// (toutes à −10, pondération nulle) sont laissées de côté, comme ArgyllCMS
/// le fait lui-même en ne lisant que les 36 annoncées.
pub(crate) const E_VERT: Ponderation = Ponderation {
    debut_nm: 370.0,
    log10: &[
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, 0.500, 3.000, //
        3.699, 4.447, 4.833, 4.964, 5.000, 4.944, //
        4.820, 4.623, 4.342, 3.954, 3.398, 2.845, //
        1.954, 1.000, 0.500, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0,
    ],
};

/// Statut E, filtre bleu (densité du jaune) : 370 à 720 nm.
pub(crate) const E_BLEU: Ponderation = Ponderation {
    debut_nm: 370.0,
    log10: &[
        1.000, 2.431, 3.431, 4.114, 4.477, 4.778, //
        4.914, 5.000, 4.959, 4.881, 4.672, 4.255, //
        3.778, 2.903, 1.699, 1.000, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, //
        -10.0, -10.0, -10.0, -10.0, -10.0, -10.0,
    ],
};
