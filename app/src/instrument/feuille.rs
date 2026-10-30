//! Mesure d'une feuille au FD-9 (ticket #34) : la définition de la mire
//! RGB-2520 envoyée au pont, la requête et son suivi. Rien ici ne dépend de
//! Tauri ; le rangement dans la bibliothèque est dans `mesurer_feuille`.

use std::path::Path;

use mires::{MireFd9, RGB2520_NOM};
use pont_protocole::{
    AdresseReseau, AvancementFeuille, CalageFeuille, DefinitionFeuille, ErreurPont, Geometrie,
    Mesure, Palier, PointVise, Reponse, Requete, ZoneFeuille,
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

/// Étapes annoncées pendant la mesure d'une feuille, dans l'ordre. Celles
/// entre `GlisserFeuille` et `Rangement` viennent de l'avancement écrit par
/// le pont (ticket #34) ; un pont qui n'en écrit pas passe directement de
/// l'une à l'autre. Envoyées à la page en JSON (`{"etape":"lecture"}`), le
/// calage sans les pixels de son image ([`VueCalage`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "etape", rename_all = "snake_case")]
pub enum EtapeFeuille {
    /// Le pont est relancé au plafond Feuille et se connecte au FD-9.
    Connexion,
    /// La mesure est demandée : glisser la feuille.
    GlisserFeuille,
    /// Le FD-9 a pris la feuille.
    FeuilleEngagee,
    /// Image, QR et repérage des plages.
    Lecture,
    /// Points visés et image de la feuille, avant la mesure.
    CalagePret {
        #[serde(serialize_with = "calage_pour_la_page")]
        calage: CalageFeuille,
    },
    /// `mesurees` plages mesurées sur `plages`, tel que le pont le compte.
    Mesure { mesurees: u32, plages: u32 },
    /// La mesure est reçue et rangée dans la résolution.
    Rangement,
}

impl EtapeFeuille {
    fn depuis(avancement: AvancementFeuille) -> Self {
        match avancement {
            AvancementFeuille::FeuilleEngagee {} => EtapeFeuille::FeuilleEngagee,
            AvancementFeuille::Lecture {} => EtapeFeuille::Lecture,
            AvancementFeuille::CalagePret { calage } => EtapeFeuille::CalagePret { calage },
            AvancementFeuille::Mesure { mesurees, plages } => {
                EtapeFeuille::Mesure { mesurees, plages }
            }
        }
    }
}

/// Dimensions de l'image d'un calage, sans ses pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DimensionsImage {
    pub largeur: u32,
    pub hauteur: u32,
    pub ppp: u32,
}

/// Ce que la page reçoit d'un calage : décalage, écart, points visés et
/// dimensions de l'image. Les pixels passent à part, en binaire (ADR 0003).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VueCalage<'a> {
    pub decalage: [i32; 2],
    pub ecart_max: u32,
    pub points: &'a [PointVise],
    pub image: Option<DimensionsImage>,
}

impl<'a> VueCalage<'a> {
    pub fn de(calage: &'a CalageFeuille) -> Self {
        VueCalage {
            decalage: calage.decalage,
            ecart_max: calage.ecart_max,
            points: &calage.points,
            image: calage.image.as_ref().map(|i| DimensionsImage {
                largeur: i.largeur(),
                hauteur: i.hauteur(),
                ppp: i.ppp(),
            }),
        }
    }
}

fn calage_pour_la_page<S: serde::Serializer>(
    calage: &CalageFeuille,
    s: S,
) -> Result<S::Ok, S::Error> {
    VueCalage::de(calage).serialize(s)
}

/// Feuille mesurée, telle que le pont l'a rendue.
#[derive(Clone, Debug, PartialEq)]
pub struct FeuilleMesuree {
    pub mesure: Mesure,
    /// Fermeture du pont de la feuille ; la mesure vaut quelle qu'elle soit.
    pub fermeture: Fermeture,
    /// Points visés et image : ceux de la réponse, sinon ceux annoncés par
    /// l'avancement ; aucun si le pont n'en a pas rendu.
    pub calage: Option<CalageFeuille>,
}

/// Pourquoi une feuille n'a pas été mesurée.
#[derive(Clone, Debug, PartialEq)]
pub enum ErreurFeuille {
    /// Aucun FD-9 détecté : rien n'est lancé.
    SansFd9,
    /// La mire a été lue, mais sa définition est refusée (pas une couleur
    /// par plage) : rien n'est lancé.
    Definition(String),
    /// Le pont de la feuille n'a pas pu se lancer, se connecter ou répondre.
    Instrument(Probleme),
    /// Le pont a refusé ou interrompu la feuille, en disant pourquoi : rien
    /// n'est mesuré.
    Feuille(ErreurPont),
}

impl ErreurFeuille {
    /// Clé du catalogue : `probleme.<code>` pour un problème de l'instrument
    /// (textes `.cause` et `.action` existants), `feuille_fd9.<type>` pour
    /// un refus du pont, du nom de son type dans le protocole
    /// (`feuille_fd9.feuille_absente`…), `feuille_fd9.sans_fd9` sans FD-9
    /// détecté, `feuille_fd9.definition_refusee` pour une définition refusée.
    pub fn cle(&self) -> String {
        match self {
            ErreurFeuille::SansFd9 => "feuille_fd9.sans_fd9".into(),
            ErreurFeuille::Definition(_) => "feuille_fd9.definition_refusee".into(),
            ErreurFeuille::Instrument(probleme) => format!("probleme.{}", probleme.code()),
            ErreurFeuille::Feuille(erreur) => format!("feuille_fd9.{}", type_erreur(erreur)),
        }
    }

    /// Détail technique, montré replié.
    pub fn detail(&self) -> Option<String> {
        match self {
            ErreurFeuille::SansFd9 => None,
            ErreurFeuille::Definition(detail) => Some(detail.clone()),
            ErreurFeuille::Instrument(probleme) => probleme.detail().map(String::from),
            ErreurFeuille::Feuille(erreur) => Some(format!("{erreur:?}")),
        }
    }
}

/// Nom du type de l'erreur dans le protocole (`"type"` du JSON), sans
/// joker : une erreur ajoutée au protocole doit recevoir sa clé ici.
fn type_erreur(erreur: &ErreurPont) -> &'static str {
    match erreur {
        ErreurPont::EtatInvalide { .. } => "etat_invalide",
        ErreurPont::PalierNonAutorise { .. } => "palier_non_autorise",
        ErreurPont::InstrumentInconnu {} => "instrument_inconnu",
        ErreurPont::ParametreRefuse {} => "parametre_refuse",
        ErreurPont::EtatIncompatible {} => "etat_incompatible",
        ErreurPont::EtalonnageEchoue { .. } => "etalonnage_echoue",
        ErreurPont::NonEtalonne {} => "non_etalonne",
        ErreurPont::EtalonnageRequis {} => "etalonnage_requis",
        ErreurPont::SessionInexploitable {} => "session_inexploitable",
        ErreurPont::MesureEchouee { .. } => "mesure_echouee",
        ErreurPont::DeclenchementRefuse { .. } => "declenchement_refuse",
        ErreurPont::MesureAnnulee { .. } => "mesure_annulee",
        ErreurPont::ReponseInattendue { .. } => "reponse_inattendue",
        ErreurPont::Delai {} => "delai",
        ErreurPont::InstrumentPerdu {} => "instrument_perdu",
        ErreurPont::ReposIncertain { .. } => "repos_incertain",
        ErreurPont::DeconnexionEchouee { .. } => "deconnexion_echouee",
        ErreurPont::SessionFermee {} => "session_fermee",
        ErreurPont::MireNonReconnue { .. } => "mire_non_reconnue",
        ErreurPont::AutreMire { .. } => "autre_mire",
        ErreurPont::FeuilleAbsente {} => "feuille_absente",
        ErreurPont::InstrumentIndisponible { .. } => "instrument_indisponible",
        ErreurPont::IncidentFeuille { .. } => "incident_feuille",
        ErreurPont::ReconnaissanceDouteuse { .. } => "reconnaissance_douteuse",
        ErreurPont::MesureInvraisemblable { .. } => "mesure_invraisemblable",
        ErreurPont::PositionsPlages { .. } => "positions_plages",
        ErreurPont::ComposantQrIntrouvable { .. } => "composant_qr_introuvable",
        ErreurPont::Sdk { .. } => "sdk",
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
        // Calage annoncé pendant la feuille, gardé si la réponse n'en a pas.
        let mut annonce = None;
        let echange = pont.demander_suivi(
            &Requete::MesurerFeuille { mire },
            annulation,
            &mut |avancement| {
                if let AvancementFeuille::CalagePret { calage } = &avancement {
                    annonce = Some(calage.clone());
                }
                suivi(EtapeFeuille::depuis(avancement));
            },
        );
        annulation.terminer();
        let reponse = echange
            .reponse
            .map_err(|panne| ErreurFeuille::Instrument(panne.into()))?;
        match reponse {
            Reponse::Mesure { mesure, calage, .. }
                if mesure.provenance().geometrie == (Geometrie::Feuille {}) =>
            {
                Ok((mesure, calage.or(annonce)))
            }
            // L'installation est incomplète, pas la feuille : le problème
            // de l'instrument, avec ses textes.
            Reponse::Erreur {
                erreur: ErreurPont::ComposantQrIntrouvable { chemin },
            } => Err(ErreurFeuille::Instrument(
                Probleme::ComposantQrIntrouvable { detail: chemin },
            )),
            Reponse::Erreur { erreur } => Err(ErreurFeuille::Feuille(erreur)),
            autre => Err(ErreurFeuille::Instrument(inattendue(autre))),
        }
    });
    let fermeture = fermer_pont(pont);
    lue.map(|(mesure, calage)| FeuilleMesuree {
        mesure,
        fermeture,
        calage,
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
