//! Palier Connexion : rappel inscrit, connexion, identité, puis fermeture,
//! dans l'ordre des fiches `docs/abi/FD9_RegisterDeviceEventHandler.md`,
//! `FD9_Connect.md`, `FD9_GetSystemInfo.md` et `FD9_Disconnect.md`.

use crate::{Fermeture, SdkFd9, Session, NOM_APPLICATION};
use fd9_sys::{
    connexion_etablie, Appareil, InfosSysteme, NomApplication, TamponInfosSysteme,
    CODE_RECERTIFICATION_DUE,
};
use pont_protocole::{AdresseReseau, ErreurPont, Identite, Palier, RemiseAuRepos};
use std::time::Duration;

impl<S: SdkFd9> Session<S> {
    /// Connexion à l'instrument n° `instrument` de la dernière détection.
    pub fn connecter(&mut self, instrument: usize) -> Result<Identite, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Detection))?;
        let appareil = *self
            .detectes
            .get(instrument)
            .ok_or(ErreurPont::InstrumentInconnu {})?;
        self.ouvrir(&appareil)
    }

    /// Connexion directe à une adresse réseau, sans détection.
    pub fn connecter_adresse(&mut self, adresse: &AdresseReseau) -> Result<Identite, ErreurPont> {
        self.autoriser(Palier::Connexion, Some(Palier::Version))?;
        let appareil =
            Appareil::reseau(adresse.texte()).map_err(|_| ErreurPont::ParametreRefuse {})?;
        self.ouvrir(&appareil)
    }

    /// Inscrit le rappel, se connecte, lit l'identité et l'état. Un échec
    /// défait ce qui a été fait, dans l'ordre inverse.
    fn ouvrir(&mut self, appareil: &Appareil) -> Result<Identite, ErreurPont> {
        // Déjà connectée, la DLL rendrait 0 sans rien faire : succès trompeur.
        if self.connecte {
            return Err(ErreurPont::EtatIncompatible {});
        }
        let nom = NomApplication::nouveau(NOM_APPLICATION)
            .map_err(|e| inattendue(format!("nom d'application refusé : {e:?}")))?;
        self.vider_hors_requete();
        let code = self.sdk.inscrire_rappel(true);
        self.journal
            .push(format!("FD9_RegisterDeviceEventHandler : {code}"));
        if code != 0 {
            return Err(ErreurPont::Sdk { code });
        }
        self.inscrit = true;
        let code = self.sdk.connecter(appareil, &nom);
        if !connexion_etablie(code) {
            let interne = self.sdk.dernier_code();
            self.journal
                .push(format!("FD9_Connect : code {code}, code interne {interne}"));
            self.desinscrire();
            return Err(ErreurPont::Sdk { code });
        }
        self.connecte = true;
        let recertification_due = code == CODE_RECERTIFICATION_DUE;
        self.journal.push(if recertification_due {
            "FD9_Connect : 1901, connecté, recertification de l'instrument due".into()
        } else {
            format!("FD9_Connect : {code}")
        });
        match self.lire_identite(recertification_due) {
            Ok(identite) => {
                let mut statut = [0u32; 8];
                let code = self.sdk.statut(&mut statut);
                self.journal
                    .push(format!("FD9_GetStatus : code {code}, {statut:?}"));
                self.franchir(Palier::Connexion);
                Ok(identite)
            }
            Err(erreur) => {
                self.abandonner_connexion();
                Err(erreur)
            }
        }
    }

    fn lire_identite(&mut self, recertification_due: bool) -> Result<Identite, ErreurPont> {
        let mut tampon = TamponInfosSysteme::vide();
        let code = self.sdk.infos(&mut tampon);
        if code != 0 {
            let interne = self.sdk.dernier_code();
            self.journal.push(format!(
                "FD9_GetSystemInfo : code {code}, code interne {interne}"
            ));
            return Err(ErreurPont::Sdk { code });
        }
        let infos = tampon.infos;
        if !infos.est_rempli() {
            return Err(inattendue("FD9_GetSystemInfo n'a rien écrit".into()));
        }
        // 8 chiffres supposés en BCD : lus en décimal, sans rien deviner.
        let texte = infos.numero_de_serie();
        let numero_serie = Some(&texte)
            .filter(|t| !t.is_empty() && t.bytes().all(|o| o.is_ascii_digit()))
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| inattendue(format!("n° de série « {texte} » non numérique")))?;
        self.journal.push(format!(
            "FD9_GetSystemInfo : 0, micrologiciel {}, produit {}",
            infos.micrologiciel(),
            infos.produit()
        ));
        Ok(Identite {
            numero_serie,
            micrologiciel: infos.micrologiciel(),
            code_produit: infos.code_produit(),
            adresse_mac: infos.mac(),
            // Le sens des dates du FD-9 n'est pas établi (fiche FD9_GetSystemInfo).
            date_initiale: None,
            anomalie_date_initiale: false,
            brute_hex: octets(&infos)
                .iter()
                .map(|octet| format!("{octet:02x}"))
                .collect(),
            recertification_due,
        })
    }

    /// Connecté, mais l'identité est inutilisable : déconnecter, puis
    /// désinscrire. Une déconnexion refusée laisse la session à fermer.
    fn abandonner_connexion(&mut self) {
        let code = self.sdk.deconnecter();
        self.journal.push(format!("FD9_Disconnect : {code}"));
        if code == 0 {
            self.connecte = false;
            self.desinscrire();
        }
    }

    fn desinscrire(&mut self) {
        let code = self.sdk.inscrire_rappel(false);
        self.journal
            .push(format!("FD9_RegisterDeviceEventHandler (nul) : {code}"));
        if code == 0 {
            self.inscrit = false;
        }
    }

    /// Événements arrivés hors de toute requête : journalisés, sans effet.
    pub(crate) fn vider_hors_requete(&mut self) {
        if !self.inscrit {
            return;
        }
        for e in self.sdk.evenements(Duration::ZERO) {
            self.journal.push(format!(
                "événement hors requête ignoré : état {} (travail {}, page {}, compte {})",
                e.etat.code(),
                e.travail,
                e.page,
                e.compte
            ));
        }
    }

    /// Déconnecte puis désinscrit le rappel. Ensuite, plus rien n'atteint la
    /// DLL. Une déconnexion refusée n'est pas retentée.
    pub fn fermer(&mut self) -> Fermeture {
        if let Some(fermeture) = &self.fermeture {
            return fermeture.clone();
        }
        self.fermee = true;
        self.vider_hors_requete();
        let fermeture = if self.connecte {
            let code = self.sdk.deconnecter();
            self.journal.push(format!("FD9_Disconnect : {code}"));
            if code == 0 {
                self.connecte = false;
                self.desinscrire();
                Fermeture::Confirmee
            } else {
                Fermeture::Incertaine {
                    remise_au_repos: RemiseAuRepos::ArretRefuse { code },
                }
            }
        } else {
            if self.inscrit {
                self.desinscrire();
            }
            Fermeture::Confirmee
        };
        self.journal
            .push(format!("résultat de la fermeture : {fermeture:?}"));
        self.fermeture = Some(fermeture.clone());
        fermeture
    }
}

pub(crate) fn inattendue(detail: String) -> ErreurPont {
    ErreurPont::ReponseInattendue { detail }
}

/// Les 100 octets de l'identité, dans l'ordre de la DLL.
fn octets(infos: &InfosSysteme) -> Vec<u8> {
    let mots =
        |valeurs: &[u32]| -> Vec<u8> { valeurs.iter().flat_map(|v| v.to_le_bytes()).collect() };
    [
        mots(&infos.version_sdk),
        mots(&infos.version_micrologiciel),
        infos.code_produit.to_vec(),
        infos.numero_serie.to_vec(),
        infos.adresse_mac.to_vec(),
        infos.remplissage.to_vec(),
        mots(&infos.date_etalonnage_usine),
        mots(&infos.date_premiere_mesure),
        infos.nom_produit.to_vec(),
    ]
    .concat()
}
