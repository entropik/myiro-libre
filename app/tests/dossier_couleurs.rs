//! Installer un profil dans le dossier des couleurs de Windows, par le trait
//! `InstallationProfils`. Ces tests passent tous par `InstallationSimulee`
//! (un dossier temporaire) : aucun ne touche le vrai Windows.
//!
//! Le fichier ne s'appelle pas `installation.rs` : Windows prend tout
//! programme 32 bits dont le nom contient « install » pour un installateur
//! et exige alors des droits d'administrateur pour le lancer.

use std::path::Path;

use app::colonne::{emplacement, BibliothequeOuverte};
use app::installation::{
    est_installe, installer_profil, ErreurInstallation, EtatInstallation, InstallationSimulee,
};
use tempfile::TempDir;

/// En-tête ICC minimal de 132 octets, avec un octet de différence possible.
fn icc(marque: u8) -> Vec<u8> {
    let mut octets = vec![0u8; 132];
    octets[0..4].copy_from_slice(&132u32.to_be_bytes());
    octets[16..20].copy_from_slice(b"RGB ");
    octets[36..40].copy_from_slice(b"acsp");
    octets[100] = marque;
    octets
}

/// Un dossier des couleurs simulé et un dossier temporaire, vides.
fn systeme(refuse: bool) -> (TempDir, InstallationSimulee, std::path::PathBuf) {
    let racine = tempfile::tempdir().unwrap();
    let couleurs = racine.path().join("color");
    let temporaire = racine.path().join("temporaire");
    std::fs::create_dir_all(&couleurs).unwrap();
    std::fs::create_dir_all(&temporaire).unwrap();
    let systeme = InstallationSimulee {
        dossier: couleurs,
        refuse,
    };
    (racine, systeme, temporaire)
}

const NOM: &str = "Canon-PRO-4100_HMPhoto-Luster-290_1200-ppp_2026-10-22.icc";

#[test]
fn un_profil_neuf_est_copie_dans_le_dossier_des_couleurs() {
    let (_racine, systeme, temporaire) = systeme(false);
    assert!(!est_installe(&systeme, NOM, &icc(1)));

    let etat = installer_profil(&systeme, NOM, &icc(1), &temporaire);
    assert_eq!(etat, Ok(EtatInstallation::Installe));
    assert_eq!(std::fs::read(systeme.dossier.join(NOM)).unwrap(), icc(1));
    assert!(est_installe(&systeme, NOM, &icc(1)));
}

#[test]
fn un_profil_deja_installe_a_l_identique_est_accepte() {
    // Un Windows qui refuserait tout appel : l'installation n'en fait aucun.
    let (_racine, systeme, temporaire) = systeme(true);
    std::fs::write(systeme.dossier.join(NOM), icc(1)).unwrap();

    let etat = installer_profil(&systeme, NOM, &icc(1), &temporaire);
    assert_eq!(etat, Ok(EtatInstallation::DejaInstalle));
}

#[test]
fn un_profil_different_du_meme_nom_n_est_pas_ecrase() {
    let (_racine, systeme, temporaire) = systeme(false);
    std::fs::write(systeme.dossier.join(NOM), icc(2)).unwrap();

    let etat = installer_profil(&systeme, NOM, &icc(1), &temporaire);
    assert_eq!(etat, Err(ErreurInstallation::DejaPresentDifferent));
    assert_eq!(std::fs::read(systeme.dossier.join(NOM)).unwrap(), icc(2));
    assert!(!est_installe(&systeme, NOM, &icc(1)));
}

#[test]
fn un_refus_de_windows_est_rendu_tel_quel() {
    let (_racine, systeme, temporaire) = systeme(true);
    let etat = installer_profil(&systeme, NOM, &icc(1), &temporaire);
    assert_eq!(etat, Err(ErreurInstallation::Refusee));
    assert!(!systeme.dossier.join(NOM).exists());
}

/// Ce que la page reçoit : l'état d'installation du profil actif, et des
/// clés du catalogue pour chaque refus.
#[test]
fn le_detail_dit_si_le_profil_actif_est_installe() {
    let (_racine, systeme, temporaire) = systeme(false);
    let dossier = tempfile::tempdir().unwrap();
    let ouverte = BibliothequeOuverte::ouvrir(&emplacement(dossier.path()));
    let canon = ouverte.ajouter_imprimante("Canon PRO-4100").unwrap();
    let condition = ouverte
        .ajouter_papier(canon.id, "HMPhoto Luster 290", "1200 ppp")
        .unwrap()
        .id;
    let source = dossier.path().join("atelier.icc");
    std::fs::write(&source, icc(1)).unwrap();

    // Sans profil : rien à installer.
    let detail = ouverte
        .detail_reglage_installe(condition, &systeme)
        .unwrap();
    assert!(!detail.installe);

    let profil = ouverte
        .ranger_profil(condition, &source, "2026-10-22")
        .unwrap();
    assert!(
        !ouverte
            .detail_reglage_installe(condition, &systeme)
            .unwrap()
            .installe
    );

    assert_eq!(
        ouverte.installer_profil(profil, &systeme, &temporaire),
        Ok(EtatInstallation::Installe)
    );
    assert!(
        ouverte
            .detail_reglage_installe(condition, &systeme)
            .unwrap()
            .installe
    );
    assert_eq!(
        std::fs::read(systeme.dossier.join(NOM)).unwrap(),
        icc(1),
        "installé sous son nom proposé"
    );

    // Un autre profil du même nom, déjà installé, n'est pas écrasé.
    let (_autre, autre_systeme, autre_temporaire) = self::systeme(false);
    std::fs::write(autre_systeme.dossier.join(NOM), icc(9)).unwrap();
    let cle = ouverte
        .installer_profil(profil, &autre_systeme, &autre_temporaire)
        .unwrap_err();
    assert_eq!(cle, "installation.deja_present");
    let refuse = InstallationSimulee {
        dossier: dossier.path().join("vide"),
        refuse: true,
    };
    std::fs::create_dir_all(&refuse.dossier).unwrap();
    let cle_refus = ouverte
        .installer_profil(profil, &refuse, Path::new(&autre_temporaire))
        .unwrap_err();
    assert_eq!(cle_refus, "installation.refusee");
    for cle in [
        cle,
        cle_refus,
        "installation.installe".to_string(),
        "installation.echec".to_string(),
    ] {
        assert!(app::textes::cles().any(|c| c == cle), "{cle}");
    }
}
