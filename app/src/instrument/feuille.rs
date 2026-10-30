//! Mesure d'une feuille au FD-9 (ticket #34) : la définition de la mire
//! RGB-2520 envoyée au pont, la requête et son suivi. Rien ici ne dépend de
//! Tauri ; le rangement dans la bibliothèque est dans `mesurer_feuille`.

use std::path::Path;

use mires::{MireFd9, RGB2520_NOM};
use pont_protocole::{
    ecrire_reponse, AdresseReseau, DefinitionFeuille, ErreurPont, Geometrie, Mesure, Palier,
    Reponse, Requete, ZoneFeuille,
};
use serde::Serialize;

use super::choix::{fermer_pont, Fermeture};
use super::{inattendue, Etat, Instrument, Probleme};
use crate::pont::{Annulation, Panne, Pont};

/// Feuille de la RGB-2520, en 1/100 mm : largeur, hauteur (définition de
/// mire de MYIROtools, étude statique du 28 octobre 2026).
pub const FEUILLE_RGB2520: (u32, u32) = (27_940, 41_660);

/// Zone unique de la RGB-2520, en 1/100 mm : 60 rangées de 42 plages de
/// 601, jointives, la première à (1 349 ; 2 300) du coin haut gauche.
pub const ZONE_RGB2520: ZoneFeuille = ZoneFeuille {
    gauche: 1_349,
    haut: 2_300,
    largeur_plage: 601,
    hauteur_plage: 601,
    pas_x: 601,
    pas_y: 601,
    rangees: 60,
    colonnes: 42,
};

/// Définition de la RGB-2520 envoyée au pont, avec la couleur imprimée sur
/// chaque plage (RVB de 0 à 255), lue dans la mire : le FD-9 en a besoin
/// pour recaler la feuille (reconnaissance COMBINE, essai E4 ter). Refusée,
/// en le disant, si la mire n'a pas une couleur par plage.
pub fn definition_rgb2520(mire: &MireFd9) -> Result<DefinitionFeuille, String> {
    let couleurs = mire
        .valeurs
        .iter()
        // Valeurs de 0 à 100 lues dans le TIFF 8 bits (octet / 2,55) : la
        // multiplication rend l'octet d'origine, à l'arrondi près.
        .map(|v| v.map(|c| (c * 2.55).round().clamp(0.0, 255.0) as u8))
        .collect();
    let definition = DefinitionFeuille {
        nom: RGB2520_NOM.into(),
        largeur: FEUILLE_RGB2520.0,
        hauteur: FEUILLE_RGB2520.1,
        zones: vec![ZONE_RGB2520],
        couleurs: Some(couleurs),
    };
    definition.verifier().map_err(|e| e.to_string())?;
    Ok(definition)
}

/// Étapes annoncées pendant la mesure d'une feuille, dans l'ordre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtapeFeuille {
    /// Le pont est relancé au plafond Feuille et se connecte au FD-9.
    Connexion,
    /// La mesure est demandée : glisser la feuille ; le FD-9 la lit, puis
    /// mesure ses plages. Le pont ne dit rien de plus avant sa réponse.
    GlisserFeuille,
    /// La mesure est reçue et rangée dans la résolution.
    Rangement,
}

/// Feuille mesurée, telle que le pont l'a rendue.
#[derive(Clone, Debug, PartialEq)]
pub struct FeuilleMesuree {
    pub mesure: Mesure,
    /// La réponse entière du pont, en JSON, à garder telle quelle : tout ce
    /// qu'il a rendu avec la mesure y reste.
    pub reponse: String,
    /// Fermeture du pont de la feuille ; la mesure vaut quelle qu'elle soit.
    pub fermeture: Fermeture,
}

/// Pourquoi une feuille n'a pas été mesurée.
#[derive(Clone, Debug, PartialEq)]
pub enum ErreurFeuille {
    /// Aucun FD-9 détecté : rien n'est lancé.
    SansFd9,
    /// Le pont de la feuille n'a pas pu se lancer, se connecter ou répondre.
    Instrument(Probleme),
    /// Le pont a refusé ou interrompu la feuille, en disant pourquoi : rien
    /// n'est mesuré.
    Feuille(ErreurPont),
}

impl ErreurFeuille {
    /// Clé du catalogue : `probleme.<code>` pour un problème de l'instrument
    /// (textes `.cause` et `.action` existants), `feuille_fd9.<type>` pour
    /// un refus du pont (`feuille_fd9.feuille_absente`…), `feuille_fd9.sans_fd9`
    /// sans FD-9 détecté.
    pub fn cle(&self) -> String {
        match self {
            ErreurFeuille::SansFd9 => "feuille_fd9.sans_fd9".into(),
            ErreurFeuille::Instrument(probleme) => format!("probleme.{}", probleme.code()),
            ErreurFeuille::Feuille(erreur) => {
                let genre = serde_json::to_value(erreur)
                    .ok()
                    .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from))
                    .unwrap_or_else(|| "autre".into());
                format!("feuille_fd9.{genre}")
            }
        }
    }

    /// Détail technique, montré replié.
    pub fn detail(&self) -> Option<String> {
        match self {
            ErreurFeuille::SansFd9 => None,
            ErreurFeuille::Instrument(probleme) => probleme.detail().map(String::from),
            ErreurFeuille::Feuille(erreur) => Some(format!("{erreur:?}")),
        }
    }
}

impl<P: Pont> Instrument<P> {
    /// Mesure une feuille au FD-9 détecté. Le pont ordinaire est fermé (un
    /// seul logiciel à la fois tient l'instrument) ; un pont est lancé au
    /// plafond Feuille pour cette mesure seulement, se connecte à l'adresse
    /// de la détection, mesure `mire`, puis il est fermé. Le pont ordinaire
    /// est ensuite relancé ([`super::fd9::PLAFOND_FD9`]), quelle que soit
    /// l'issue.
    /// L'annulation ([`Instrument::annulation`]) est transmise au pont
    /// pendant l'attente et la lecture de la feuille.
    pub fn mesurer_feuille(
        &mut self,
        mire: DefinitionFeuille,
        lancer: &mut impl FnMut(&Path, &Path, Palier) -> Result<P, Panne>,
        suivi: &mut impl FnMut(EtapeFeuille),
    ) -> Result<FeuilleMesuree, ErreurFeuille> {
        let lance = match (&self.etat, &self.pont, &self.fd9) {
            (Etat::Detecte { .. }, Some(_), Some(lance)) => lance.clone(),
            _ => return Err(ErreurFeuille::SansFd9),
        };
        self.fermer();
        suivi(EtapeFeuille::Connexion);
        let resultat = match lancer(&lance.programme, &lance.dll, Palier::Feuille) {
            Ok(pont) => mesurer_avec(pont, &lance.adresse, mire, &self.annulation, suivi),
            Err(panne) => Err(ErreurFeuille::Instrument(panne.into())),
        };
        // L'annulation reste la même poignée : l'écran la garde.
        let annulation = self.annulation.clone();
        *self = Self::relancer_fd9(&lance, lancer);
        self.annulation = annulation;
        resultat
    }
}

/// Version, connexion, feuille, puis fermeture du pont de la feuille.
fn mesurer_avec<P: Pont>(
    mut pont: P,
    adresse: &str,
    mire: DefinitionFeuille,
    annulation: &Annulation,
    suivi: &mut impl FnMut(EtapeFeuille),
) -> Result<FeuilleMesuree, ErreurFeuille> {
    let lue = connecter(&mut pont, adresse).and_then(|()| {
        suivi(EtapeFeuille::GlisserFeuille);
        annulation.commencer();
        let echange = pont.demander_annulable(&Requete::MesurerFeuille { mire }, annulation);
        annulation.terminer();
        let reponse = echange
            .reponse
            .map_err(|panne| ErreurFeuille::Instrument(panne.into()))?;
        match &reponse {
            Reponse::Mesure { mesure, .. }
                if mesure.provenance().geometrie == (Geometrie::Feuille {}) =>
            {
                Ok((mesure.clone(), ecrire_reponse(&reponse)))
            }
            Reponse::Erreur { erreur } => Err(ErreurFeuille::Feuille(erreur.clone())),
            _ => Err(ErreurFeuille::Instrument(inattendue(reponse))),
        }
    });
    let fermeture = fermer_pont(pont);
    lue.map(|(mesure, reponse)| FeuilleMesuree {
        mesure,
        reponse,
        fermeture,
    })
}

/// Version de la DLL, puis connexion à l'adresse de la détection.
fn connecter(pont: &mut impl Pont, adresse: &str) -> Result<(), ErreurFeuille> {
    let probleme = ErreurFeuille::Instrument;
    match pont.demander(&Requete::Version {}) {
        Ok(Reponse::VersionDll { .. }) => {}
        Ok(Reponse::Erreur { erreur }) => {
            return Err(probleme(Probleme::LogicielInutilisable {
                detail: format!("version du SDK du FD-9 : {erreur:?}"),
            }))
        }
        Ok(autre) => return Err(probleme(inattendue(autre))),
        Err(panne) => return Err(probleme(panne.into())),
    }
    let adresse = AdresseReseau::new(adresse).map_err(|detail| {
        probleme(Probleme::ConnexionImpossible {
            detail: format!("adresse du FD-9 : {detail}"),
        })
    })?;
    match pont.demander(&Requete::ConnecterAdresse { adresse }) {
        Ok(Reponse::Connecte { .. }) => Ok(()),
        Ok(Reponse::Erreur { erreur }) => Err(probleme(Probleme::ConnexionImpossible {
            detail: format!("connexion au FD-9 : {erreur:?}"),
        })),
        Ok(autre) => Err(probleme(inattendue(autre))),
        Err(panne) => Err(probleme(panne.into())),
    }
}
